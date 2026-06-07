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
fn get_max_bytes() -> u64 {
    read_max_bytes()
}

#[tauri::command]
fn set_max_bytes(val: u64) {
    write_max_bytes(val);
}

const KEYWORDS: &[&str] = &[
    "Usage credits required for 1M context",
    "turn on usage credits",
    "switch to standard context",
];

const SIZE_YELLOW: u64 = 1 * 1024 * 1024;
const SIZE_RED: u64 = 1024 * 1024 + 512 * 1024;

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

// ─── 配置文件 ─────────────────────────────────────────────────────────────────

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ctx-guard")
        .join("config.json")
}

fn read_max_bytes() -> u64 {
    let path = config_path();
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(val) = content.trim().parse::<u64>() {
            return val;
        }
    }
    1048576 // 默认 Max 1M
}

fn write_max_bytes(val: u64) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&path, val.to_string());
}

// ─── audit.jsonl 搜索 ─────────────────────────────────────────────────────────

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
                if let Ok(mtime) = meta.modified() {
                    results.push((path, mtime));
                }
            }
        }
    }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 { format!("{} B", bytes) }
    else if bytes < 1024 * 1024 { format!("{:.1} KB", bytes as f64 / 1024.0) }
    else { format!("{:.2} MB", bytes as f64 / 1024.0 / 1024.0) }
}

fn get_icon_path(size: u64, max: u64, app: &AppHandle) -> PathBuf {
    let pct = size as f64 / max as f64;
    let name = if pct >= 0.85 { "icon-red.png" }
               else if pct >= 0.6 { "icon-yellow.png" }
               else { "icon-green.png" };
    app.path().resource_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("icons").join(name)
}

fn generate_handoff(keyword: &str, audit_path: &str) -> String {
    let desktop = dirs::desktop_dir().unwrap_or_else(|| PathBuf::from("."));
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default().as_secs();
    let out_path = desktop.join(format!("handoff_{}.md", now));
    let content = format!(
        "# ctx-guard 自动生成交接文档\n\n触发信号：`{}`\n来源文件：{}\n\n---\n\n## 触发原因\n\nctx-guard 监控到 Cowork 即将达到上下文限制。\n请立即开启新对话，并将此文档粘贴到新对话开头。\n\n---\n\n## 项目状态（请手动补充）\n\n- **当前任务**：\n- **完成进度**：\n- **下一步**：\n- **重要文件**：\n\n---\n\n*由 ctx-guard Tauri 托盘版自动生成*\n",
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

fn emit_float(app: &AppHandle, size: u64, status: &str, max: u64) {
    let pct = size as f64 / max as f64;
    let color = if pct >= 0.85 { "red" }
                else if pct >= 0.6 { "yellow" }
                else { "green" }.to_string();
    let _ = app.emit("float-update", FloatUpdate {
        status: status.to_string(),
        size_str: format_size(size),
        color,
        size_bytes: size,
        max_bytes: max,
    });
}

// ─── 后台监控 ─────────────────────────────────────────────────────────────────

fn start_watcher(app: AppHandle) {
    thread::spawn(move || {
        let last_size: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));
        let last_path: Arc<Mutex<Option<PathBuf>>> = Arc::new(Mutex::new(None));
        let mut last_color = "green";

        loop {
            thread::sleep(Duration::from_secs(2));
            let max = read_max_bytes();

            let Some(audit_path) = find_latest_audit() else {
                if let Some(t) = app.tray_by_id("main") {
                    let _ = t.set_tooltip(Some("ctx-guard: 等待 Cowork 启动..."));
                }
                emit_float(&app, 0, "等待 Cowork...", max);
                continue;
            };

            {
                let mut lp = last_path.lock().unwrap();
                if *lp != Some(audit_path.clone()) {
                    *lp = Some(audit_path.clone());
                    let size = fs::metadata(&audit_path).map(|m| m.len()).unwrap_or(0);
                    *last_size.lock().unwrap() = size;
                    emit_float(&app, size, "新 session", max);
                    continue;
                }
            }

            let current_size = fs::metadata(&audit_path).map(|m| m.len()).unwrap_or(0);
            let prev_size = *last_size.lock().unwrap();
            let pct = current_size as f64 / max as f64;

            let new_color = if pct >= 0.85 { "red" } else if pct >= 0.6 { "yellow" } else { "green" };
            if new_color != last_color {
                last_color = new_color;
                let icon_path = get_icon_path(current_size, max, &app);
                if let Ok(icon) = Image::from_path(&icon_path) {
                    if let Some(t) = app.tray_by_id("main") { let _ = t.set_icon(Some(icon)); }
                }
            }

            let status = if pct >= 0.85 { "ctx-guard [危险]" }
                         else if pct >= 0.6 { "ctx-guard [注意]" }
                         else { "ctx-guard 监控中" };

            emit_float(&app, current_size, status, max);

            if let Some(t) = app.tray_by_id("main") {
                let _ = t.set_tooltip(Some(&format!(
                    "{}\naudit.jsonl: {} / {}",
                    status, format_size(current_size), format_size(max)
                )));
            }

            if current_size <= prev_size { continue; }

            let mut file = match OpenOptions::new().read(true).open(&audit_path) {
                Ok(f) => f, Err(_) => continue,
            };
            let _ = file.seek(SeekFrom::Start(prev_size));
            let mut chunk = String::new();
            let _ = file.read_to_string(&mut chunk);
            *last_size.lock().unwrap() = current_size;

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
    });
}

// ─── Tauri 入口 ───────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec![])))
        .invoke_handler(tauri::generate_handler![get_max_bytes, set_max_bytes])
        .setup(|app| {
            // 开机自启
            use tauri_plugin_autostart::ManagerExt;
            let _ = app.autolaunch().enable();

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }

            let handle = app.handle().clone();

            // 套餐子菜单
            let plan_1m  = MenuItem::with_id(app, "plan_1m",   "Max + 1M context", true, None::<&str>)?;
            let plan_500k = MenuItem::with_id(app, "plan_500k", "Max + 500K context", true, None::<&str>)?;
            let plan_200k = MenuItem::with_id(app, "plan_200k", "Pro / Max 200K", true, None::<&str>)?;
            let plan_menu = Submenu::with_items(app, "切换套餐", true, &[&plan_1m, &plan_500k, &plan_200k])?;

            let quit    = MenuItem::with_id(app, "quit",    "退出 ctx-guard", true, None::<&str>)?;
            let status  = MenuItem::with_id(app, "status",  "✅ 监控中...", false, None::<&str>)?;
            let handoff = MenuItem::with_id(app, "handoff", "📝 立即生成交接文档", true, None::<&str>)?;
            let show_f  = MenuItem::with_id(app, "show_float", "🪟 显示悬浮窗", true, None::<&str>)?;
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
                    "plan_1m"   => { write_max_bytes(1048576); let _ = app.emit("plan-changed", 1048576u64); }
                    "plan_500k" => { write_max_bytes(524288);  let _ = app.emit("plan-changed", 524288u64); }
                    "plan_200k" => { write_max_bytes(204800);  let _ = app.emit("plan-changed", 204800u64); }
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
