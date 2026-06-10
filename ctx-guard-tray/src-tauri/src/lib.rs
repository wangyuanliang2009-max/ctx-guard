use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, Submenu},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};
use serde::Serialize;

#[tauri::command]
fn get_max_bytes() -> u64 { read_max_bytes() }

#[tauri::command]
fn set_max_bytes(val: u64) { write_max_bytes(val); }

const KEYWORDS: &[&str] = &[
    "Usage credits required for 1M context",
    "turn on usage credits",
    "switch to standard context",
];

/// 生成交接文档时回读日志末尾的字节数（8 MiB，足够覆盖最近若干轮对话）
const HANDOFF_TAIL_BYTES: u64 = 8 * 1024 * 1024;
/// 交接文档中收录的最近用户消息条数
const HANDOFF_MAX_MESSAGES: usize = 5;
/// 真实用户消息在日志行中的特征：content 直接是字符串（工具返回则是数组 "content":[）
const USER_CONTENT_MARKER: &str = "\"role\":\"user\",\"content\":\"";

#[derive(Clone, Serialize)]
struct FloatUpdate {
    status: String,
    #[serde(rename = "sizeStr")]
    size_str: String,
    color: String,
    #[serde(rename = "sizeBytes")]
    size_bytes: u64,
    #[serde(rename = "maxBytes")]
    max_bytes: u64,
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ctx-guard")
        .join("config.json")
}

fn read_max_bytes() -> u64 {
    let path = config_path();
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(val) = content.trim().parse::<u64>() { return val; }
    }
    1000000
}

fn write_max_bytes(val: u64) {
    let path = config_path();
    if let Some(parent) = path.parent() { let _ = fs::create_dir_all(parent); }
    let _ = fs::write(&path, val.to_string());
}

fn find_latest_audit() -> Option<PathBuf> {
    let base = dirs::home_dir()?.join(
        "AppData/Local/Packages/Claude_pzs8sxrjxfjjc/LocalCache/Roaming/Claude/local-agent-mode-sessions"
    );
    if !base.exists() { return None; }
    let mut results: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();
    walk(&base, &mut results, 0);
    results.sort_by(|a, b| b.1.cmp(&a.1));
    results.into_iter().next().map(|(p, _)| p)
}

fn walk(dir: &PathBuf, results: &mut Vec<(PathBuf, std::time::SystemTime)>, depth: u32) {
    if depth > 4 { return; }
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, results, depth + 1);
        } else if path.file_name().map(|n| n == "audit.jsonl").unwrap_or(false) {
            if let Ok(meta) = fs::metadata(&path) {
                if let Ok(mtime) = meta.modified() { results.push((path, mtime)); }
            }
        }
    }
}

/// 从一段JSONL文本中倒序提取最后一条assistant消息的真实context token数。
/// 公式：input_tokens + cache_creation_input_tokens + cache_read_input_tokens
/// 这是API真实上报值，非估算。不依赖serde_json解析整行（行可能截断），
/// 用轻量字符串查找以保证健壮性。
fn extract_latest_context_tokens(text: &str) -> Option<u64> {
    for line in text.lines().rev() {
        if !line.contains("\"type\":\"assistant\"") { continue; }
        if !line.contains("\"usage\"") { continue; }
        let input = find_json_number(line, "\"input_tokens\":")?;
        let cache_create = find_json_number(line, "\"cache_creation_input_tokens\":").unwrap_or(0);
        let cache_read = find_json_number(line, "\"cache_read_input_tokens\":").unwrap_or(0);
        let total = input + cache_create + cache_read;
        if total > 0 { return Some(total); }
    }
    None
}

/// 在字符串中查找 key 后面的第一个非负整数
fn find_json_number(line: &str, key: &str) -> Option<u64> {
    let pos = line.find(key)?;
    let rest = &line[pos + key.len()..];
    let digits: String = rest.chars()
        .skip_while(|c| c.is_whitespace())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse::<u64>().ok()
}

/// 读取文件末尾最多 max_tail 字节。
/// v2.3.0 修复：之前用 read_to_string，如果 seek 落点正好切在一个多字节
/// UTF-8 字符（如中文）中间会整体失败。改为按字节读取后做有损转换，
/// 边界上的半个字符会被替换为占位符，不再导致整次读取失败。
fn read_file_tail(path: &PathBuf, max_tail: u64) -> Option<String> {
    let mut file = OpenOptions::new().read(true).open(path).ok()?;
    let len = file.metadata().ok()?.len();
    let start = len.saturating_sub(max_tail);
    file.seek(SeekFrom::Start(start)).ok()?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).ok()?;
    Some(String::from_utf8_lossy(&buf).into_owned())
}

/// 解码一段 JSON 字符串字面量（传入开引号之后的内容），
/// 处理 \n \t \" \\ \/ \uXXXX 等转义，遇到未转义的闭引号结束。
/// 返回 None 表示这一行被截断（没找到闭引号），调用方应跳过该行。
fn decode_json_string(s: &str) -> Option<String> {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '"' { return Some(out); }
        if c != '\\' { out.push(c); continue; }
        match chars.next()? {
            'n' => out.push('\n'),
            't' => out.push('\t'),
            'r' => {}
            'b' | 'f' => {}
            '"' => out.push('"'),
            '\\' => out.push('\\'),
            '/' => out.push('/'),
            'u' => {
                let hex: String = chars.by_ref().take(4).collect();
                if let Ok(cp) = u32::from_str_radix(&hex, 16) {
                    if (0xD800..0xDC00).contains(&cp) {
                        // UTF-16 高代理项：尝试与紧随其后的 \uXXXX 低代理项组合
                        let mut peek = chars.clone();
                        if peek.next() == Some('\\') && peek.next() == Some('u') {
                            let hex2: String = peek.by_ref().take(4).collect();
                            if let Ok(low) = u32::from_str_radix(&hex2, 16) {
                                if (0xDC00..0xE000).contains(&low) {
                                    let combined = 0x10000 + ((cp - 0xD800) << 10) + (low - 0xDC00);
                                    if let Some(ch) = char::from_u32(combined) {
                                        out.push(ch);
                                        chars = peek;
                                    }
                                }
                            }
                        }
                    } else if let Some(ch) = char::from_u32(cp) {
                        out.push(ch);
                    }
                }
            }
            other => out.push(other),
        }
    }
    None
}

/// 从一行日志中提取真实的用户输入原文。
/// 关键区分：真实用户消息 content 是字符串（"content":"...）；
/// 工具返回结果虽然 type 也是 user，但 content 是数组（"content":[），
/// 不会命中带闭引号的 MARKER，因此被自然排除。
fn extract_user_message(line: &str) -> Option<String> {
    if !line.contains("\"type\":\"user\"") { return None; }
    let pos = line.find(USER_CONTENT_MARKER)?;
    decode_json_string(&line[pos + USER_CONTENT_MARKER.len()..])
}

/// 过滤命令类噪音（如 --model xxx、/compact 等不是"任务描述"的输入）
fn is_noise_message(m: &str) -> bool {
    let t = m.trim();
    t.is_empty() || t.starts_with("--") || t.starts_with('/')
}

/// 把一条用户消息整理成单行预览：去掉 <uploaded_files> 头部噪音，
/// 多行合并为 " / " 分隔，超长按字符截断（避免切坏中文）。
fn clean_message_preview(m: &str, max_chars: usize) -> String {
    let body = match m.find("</uploaded_files>") {
        Some(p) => &m[p + "</uploaded_files>".len()..],
        None => m,
    };
    let parts: Vec<&str> = body
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    let joined = parts.join(" / ");
    if joined.chars().count() > max_chars {
        let cut: String = joined.chars().take(max_chars).collect();
        format!("{}……", cut)
    } else {
        joined
    }
}

/// 从消息中提取 Windows 盘符路径（如 D:\GITHUB\ctx-guard）。
/// 规则：盘符前必须是单词边界（排除 http:// 这类误报），
/// 冒号后必须是反斜杠，遇空白/引号/中文标点等停止。
fn extract_win_paths(m: &str) -> Vec<String> {
    let stop = " \t\n\"'<>|*?`，。；：！？（）【】";
    let chars: Vec<char> = m.chars().collect();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i + 2 < chars.len() {
        let boundary_ok = i == 0 || !chars[i - 1].is_alphanumeric();
        if boundary_ok
            && chars[i].is_ascii_alphabetic()
            && chars[i + 1] == ':'
            && chars[i + 2] == '\\'
        {
            let mut j = i;
            while j < chars.len() && !stop.contains(chars[j]) { j += 1; }
            let raw: String = chars[i..j].iter().collect();
            let p = raw
                .trim_end_matches(|c: char| c == '.' || c == '\\' || c == ',' || c == ')')
                .to_string();
            if p.chars().count() > 3 { out.push(p); }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

/// unix 秒 → 北京时间（年, 月, 日, 时, 分）。中国无夏令时，固定 UTC+8。
/// 使用标准的 civil-from-days 历法算法，零依赖。
fn beijing_time_parts(unix_secs: u64) -> (i64, i64, i64, u64, u64) {
    let secs = unix_secs + 8 * 3600;
    let days = (secs / 86400) as i64;
    let tod = secs % 86400;
    let z = days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d, tod / 3600, (tod % 3600) / 60)
}

fn format_tokens(tokens: u64) -> String {
    if tokens < 1000 { format!("{} tok", tokens) }
    else if tokens < 1000000 { format!("{:.1}K tok", tokens as f64 / 1000.0) }
    else { format!("{:.2}M tok", tokens as f64 / 1000000.0) }
}

fn get_icon_path(tokens: u64, max: u64, app: &AppHandle) -> PathBuf {
    let pct = tokens as f64 / max as f64;
    let name = if pct >= 0.85 { "icon-red.png" } else if pct >= 0.6 { "icon-yellow.png" } else { "icon-green.png" };
    app.path().resource_dir().unwrap_or_else(|_| PathBuf::from(".")).join("icons").join(name)
}

/// v2.3.0 核心功能：自动填充的交接文档。
/// 读取 audit.jsonl 末尾 8 MiB，提取最近 5 条去重后的真实用户消息、
/// 消息中出现的文件路径、以及当前 context token 数，填入交接文档。
fn generate_handoff(keyword: &str, audit_path: &str) -> String {
    let desktop = dirs::desktop_dir().unwrap_or_else(|| PathBuf::from("."));
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (y, mo, d, h, mi) = beijing_time_parts(now);
    let out_path = desktop.join(format!("handoff_{:04}{:02}{:02}_{:02}{:02}.md", y, mo, d, h, mi));

    let tail = read_file_tail(&PathBuf::from(audit_path), HANDOFF_TAIL_BYTES).unwrap_or_default();

    // 当前 context（API 真实上报值）
    let max = read_max_bytes();
    let tokens = extract_latest_context_tokens(&tail).unwrap_or(0);
    let ctx_line = if tokens > 0 {
        format!(
            "{} / {}（{:.0}%）",
            format_tokens(tokens),
            format_tokens(max),
            tokens as f64 / max as f64 * 100.0
        )
    } else {
        "未知（未能从日志解析到 token 数）".to_string()
    };

    // 倒序扫描日志，去重收集最近的真实用户消息
    let mut seen: Vec<String> = Vec::new();
    let mut messages: Vec<String> = Vec::new();
    for line in tail.lines().rev() {
        if messages.len() >= HANDOFF_MAX_MESSAGES { break; }
        let Some(m) = extract_user_message(line) else { continue };
        if is_noise_message(&m) { continue; }
        let key = m.trim().to_string();
        if seen.contains(&key) { continue; }
        seen.push(key);
        messages.push(m);
    }
    messages.reverse(); // 改为时间正序：旧 → 新

    // 从这些消息中提取文件路径
    let mut paths: Vec<String> = Vec::new();
    for m in &messages {
        for p in extract_win_paths(m) {
            if !paths.contains(&p) { paths.push(p); }
        }
    }
    paths.truncate(10);

    let msg_section = if messages.is_empty() {
        "（未能从日志中提取到用户消息——可能是全新 session）".to_string()
    } else {
        messages
            .iter()
            .enumerate()
            .map(|(i, m)| format!("{}. {}", i + 1, clean_message_preview(m, 200)))
            .collect::<Vec<String>>()
            .join("\n")
    };

    let path_section = if paths.is_empty() {
        "- （最近消息中未发现文件路径）".to_string()
    } else {
        paths
            .iter()
            .map(|p| format!("- `{}`", p))
            .collect::<Vec<String>>()
            .join("\n")
    };

    let content = format!(
        "# ctx-guard 自动交接文档\n\n\
**生成时间**：{:04}-{:02}-{:02} {:02}:{:02}（北京时间）\n\
**触发方式**：`{}`\n\
**来源日志**：{}\n\
**当前 context**：{}\n\n\
---\n\n\
## 最近在做什么（自动提取自最近的用户消息，旧 → 新）\n\n\
{}\n\n\
## 涉及的文件路径（自动提取）\n\n\
{}\n\n\
---\n\n\
## 需要你补充的两句话\n\n\
- **当前卡在哪一步**：\n\
- **下一步要做什么**：\n\n\
## 用法\n\n\
开新对话，把本文档整个粘贴或上传，并说：\"请根据这份交接文档继续。\"\n\n\
*由 ctx-guard v2.3.0 自动生成*\n",
        y, mo, d, h, mi, keyword, audit_path, ctx_line, msg_section, path_section
    );
    let _ = fs::write(&out_path, &content);
    out_path.to_string_lossy().to_string()
}

fn show_notification(title: &str, body: &str) {
    let title = title.to_string();
    let body = body.to_string();
    thread::spawn(move || {
        let _ = std::process::Command::new("powershell")
            .args(["-Command", &format!(
                r#"[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null; $template = [Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent([Windows.UI.Notifications.ToastTemplateType]::ToastText02); $textNodes = $template.GetElementsByTagName('text'); $textNodes.Item(0).AppendChild($template.CreateTextNode('{}')) | Out-Null; $textNodes.Item(1).AppendChild($template.CreateTextNode('{}')) | Out-Null; $toast = [Windows.UI.Notifications.ToastNotification]::new($template); [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('ctx-guard').Show($toast)"#,
                title, body
            )]).output();
    });
}

fn emit_float(app: &AppHandle, tokens: u64, status: &str, max: u64) {
    let pct = tokens as f64 / max as f64;
    let color = if pct >= 0.85 { "red" } else if pct >= 0.6 { "yellow" } else { "green" }.to_string();
    let _ = app.emit("float-update", FloatUpdate {
        status: status.to_string(),
        size_str: format_tokens(tokens),
        color,
        size_bytes: tokens,
        max_bytes: max,
    });
}

fn start_watcher(app: AppHandle) {
    thread::spawn(move || {
        let last_file_size: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));
        let last_path: Arc<Mutex<Option<PathBuf>>> = Arc::new(Mutex::new(None));
        let last_tokens: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));
        let mut last_color = "green";

        loop {
            thread::sleep(Duration::from_secs(2));
            let max = read_max_bytes();

            let Some(audit_path) = find_latest_audit() else {
                emit_float(&app, 0, "等待 Cowork...", max);
                continue;
            };

            // 检测到新session：读文件尾部获取最近token数
            {
                let mut lp = last_path.lock().unwrap();
                if *lp != Some(audit_path.clone()) {
                    *lp = Some(audit_path.clone());
                    let size = fs::metadata(&audit_path).map(|m| m.len()).unwrap_or(0);
                    *last_file_size.lock().unwrap() = size;
                    let tokens = read_file_tail(&audit_path, 262144)
                        .and_then(|t| extract_latest_context_tokens(&t))
                        .unwrap_or(0);
                    *last_tokens.lock().unwrap() = tokens;
                    emit_float(&app, tokens, "新 session", max);
                    continue;
                }
            }

            let current_size = fs::metadata(&audit_path).map(|m| m.len()).unwrap_or(0);
            let prev_size = *last_file_size.lock().unwrap();

            // 文件有新增：只读增量部分，从中提取最新token数和关键词。
            // v2.3.0 修复：增量起点可能切在多字节字符中间，改为字节读取+有损转换。
            if current_size > prev_size {
                let mut chunk_bytes: Vec<u8> = Vec::new();
                if let Ok(mut file) = OpenOptions::new().read(true).open(&audit_path) {
                    let _ = file.seek(SeekFrom::Start(prev_size));
                    let _ = file.read_to_end(&mut chunk_bytes);
                }
                let chunk = String::from_utf8_lossy(&chunk_bytes).into_owned();
                *last_file_size.lock().unwrap() = current_size;

                if let Some(tokens) = extract_latest_context_tokens(&chunk) {
                    *last_tokens.lock().unwrap() = tokens;
                }

                for kw in KEYWORDS {
                    if chunk.contains(kw) {
                        let path_str = audit_path.to_string_lossy().to_string();
                        let out = generate_handoff(kw, &path_str);
                        show_notification("ctx-guard 警告", "即将触发上下文限制！交接文档已保存到桌面。");
                        let _ = app.emit("alert", &out);
                        break;
                    }
                }
            }

            let tokens = *last_tokens.lock().unwrap();
            let pct = tokens as f64 / max as f64;

            let new_color = if pct >= 0.85 { "red" } else if pct >= 0.6 { "yellow" } else { "green" };
            if new_color != last_color {
                last_color = new_color;
                let icon_path = get_icon_path(tokens, max, &app);
                if let Ok(icon) = Image::from_path(&icon_path) {
                    if let Some(t) = app.tray_by_id("main") { let _ = t.set_icon(Some(icon)); }
                }
            }

            let status = if pct >= 0.85 { "ctx-guard [危险]" } else if pct >= 0.6 { "ctx-guard [注意]" } else { "ctx-guard 监控中" };
            emit_float(&app, tokens, status, max);

            if let Some(t) = app.tray_by_id("main") {
                let _ = t.set_tooltip(Some(&format!("{}\ncontext: {} / {}", status, format_tokens(tokens), format_tokens(max))));
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec![])))
        .invoke_handler(tauri::generate_handler![get_max_bytes, set_max_bytes])
        .setup(|app| {
            use tauri_plugin_autostart::ManagerExt;
            let _ = app.autolaunch().enable();

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }

            if let Some(float) = app.get_webview_window("float") {
                let _ = float.set_size(tauri::Size::Logical(tauri::LogicalSize {
                    width: 88.0,
                    height: 118.0,
                }));
                let _ = float.show();
            }

            let handle = app.handle().clone();

            let plan_1m   = MenuItem::with_id(app, "plan_1m",   "Max + 1M context", true, None::<&str>)?;
            let plan_500k = MenuItem::with_id(app, "plan_500k", "Max + 500K context", true, None::<&str>)?;
            let plan_200k = MenuItem::with_id(app, "plan_200k", "Pro / Max 200K", true, None::<&str>)?;
            let plan_menu = Submenu::with_items(app, "切换套餐", true, &[&plan_1m, &plan_500k, &plan_200k])?;
            let quit      = MenuItem::with_id(app, "quit",       "退出 ctx-guard", true, None::<&str>)?;
            let status    = MenuItem::with_id(app, "status",     "✅ 监控中...", false, None::<&str>)?;
            let handoff   = MenuItem::with_id(app, "handoff",    "📝 立即生成交接文档", true, None::<&str>)?;
            let show_f    = MenuItem::with_id(app, "show_float", "🪟 显示悬浮窗", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&status, &handoff, &show_f, &plan_menu, &quit])?;

            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(true)
                .tooltip("ctx-guard: 启动中...")
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    "show_float" => {
                        if let Some(w) = app.get_webview_window("float") {
                            let _ = w.show(); let _ = w.set_focus();
                        }
                    }
                    "plan_1m"   => { write_max_bytes(1000000); let _ = app.emit("plan-changed", 1000000u64); }
                    "plan_500k" => { write_max_bytes(500000);  let _ = app.emit("plan-changed", 500000u64); }
                    "plan_200k" => { write_max_bytes(200000);  let _ = app.emit("plan-changed", 200000u64); }
                    "handoff" => {
                        if let Some(p) = find_latest_audit() {
                            let out = generate_handoff("手动触发", &p.to_string_lossy());
                            show_notification("ctx-guard", "交接文档已保存到桌面！");
                            let _ = app.emit("alert", &out);
                        } else {
                            show_notification("ctx-guard", "未找到 audit.jsonl，请先启动 Cowork。");
                        }
                    }
                    _ => {}
                })
                .build(app)?;

            start_watcher(handle);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
