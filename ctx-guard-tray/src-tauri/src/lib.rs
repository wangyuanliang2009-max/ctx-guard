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

/// 读取文件末尾最多 max_tail 字节（用于启动时获取最近的token数）
fn read_file_tail(path: &PathBuf, max_tail: u64) -> Option<String> {
    let mut file = OpenOptions::new().read(true).open(path).ok()?;
    let len = file.metadata().ok()?.len();
    let start = len.saturating_sub(max_tail);
    file.seek(SeekFrom::Start(start)).ok()?;
    let mut buf = String::new();
    file.read_to_string(&mut buf).ok()?;
    Some(buf)
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

fn generate_handoff(keyword: &str, audit_path: &str) -> String {
    let desktop = dirs::desktop_dir().unwrap_or_else(|| PathBuf::from("."));
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
    let out_path = desktop.join(format!("handoff_{}.md", now));
    let content = format!(
        "# ctx-guard 自动生成交接文档\n\n触发信号：`{}`\n来源文件：{}\n\n---\n\n## 项目状态（请手动补充）\n\n- **当前任务**：\n- **完成进度**：\n- **下一步**：\n\n*由 ctx-guard 自动生成*\n",
        keyword, audit_path
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

            // 文件有新增：只读增量部分，从中提取最新token数和关键词
            if current_size > prev_size {
                let mut chunk = String::new();
                if let Ok(mut file) = OpenOptions::new().read(true).open(&audit_path) {
                    let _ = file.seek(SeekFrom::Start(prev_size));
                    let _ = file.read_to_string(&mut chunk);
                }
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
