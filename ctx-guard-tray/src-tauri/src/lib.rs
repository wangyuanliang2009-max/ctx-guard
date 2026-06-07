use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};

const KEYWORDS: &[&str] = &[
    "Usage credits required for 1M context",
    "turn on usage credits",
    "switch to standard context",
];

// 文件大小阈值（字节）
const SIZE_YELLOW: u64 = 1 * 1024 * 1024;       // 1 MB → 黄色
const SIZE_RED: u64 = 1024 * 1024 + 512 * 1024; // 1.5 MB → 红色

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

fn get_icon_path(size: u64, app: &AppHandle) -> PathBuf {
    let name = if size >= SIZE_RED { "icon-red.png" }
               else if size >= SIZE_YELLOW { "icon-yellow.png" }
               else { "icon-green.png" };
    app.path().resource_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("icons")
        .join(name)
}

fn generate_handoff(keyword: &str, audit_path: &str) -> String {
    let desktop = dirs::desktop_dir().unwrap_or_else(|| PathBuf::from("."));
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let out_path = desktop.join(format!("handoff_{}.md", now));
    let content = format!(
        "# ctx-guard 自动生成交接文档\n\n触发信号：`{}`\n来源文件：{}\n\n---\n\n## 触发原因\n\nctx-guard 监控到 Cowork 即将达到 1M token 上下文限制。\n请立即开启新对话，并将此文档粘贴到新对话开头。\n\n---\n\n## 项目状态（请手动补充）\n\n- **当前任务**：\n- **完成进度**：\n- **下一步**：\n- **重要文件**：\n\n---\n\n*由 ctx-guard Tauri 托盘版自动生成*\n",
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
            )])
            .output();
    });
}

fn start_watcher(app: AppHandle) {
    thread::spawn(move || {
        let last_size: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));
        let last_path: Arc<Mutex<Option<PathBuf>>> = Arc::new(Mutex::new(None));
        let mut last_color = "green";

        loop {
            thread::sleep(Duration::from_secs(2));

            let Some(audit_path) = find_latest_audit() else {
                if let Some(t) = app.tray_by_id("main") {
                    let _ = t.set_tooltip(Some("ctx-guard: 等待 Cowork 启动..."));
                }
                continue;
            };

            {
                let mut lp = last_path.lock().unwrap();
                if *lp != Some(audit_path.clone()) {
                    *lp = Some(audit_path.clone());
                    let size = fs::metadata(&audit_path).map(|m| m.len()).unwrap_or(0);
                    *last_size.lock().unwrap() = size;
                    if let Some(t) = app.tray_by_id("main") {
                        let _ = t.set_tooltip(Some("ctx-guard: 检测到新 session，开始监控"));
                    }
                    continue;
                }
            }

            let current_size = fs::metadata(&audit_path).map(|m| m.len()).unwrap_or(0);
            let prev_size = *last_size.lock().unwrap();

            // 更新图标颜色
            let new_color = if current_size >= SIZE_RED { "red" }
                            else if current_size >= SIZE_YELLOW { "yellow" }
                            else { "green" };

            if new_color != last_color {
                last_color = new_color;
                let icon_path = get_icon_path(current_size, &app);
                if let Ok(icon) = Image::from_path(&icon_path) {
                    if let Some(t) = app.tray_by_id("main") {
                        let _ = t.set_icon(Some(icon));
                    }
                }
            }

            // 更新 tooltip
            if let Some(t) = app.tray_by_id("main") {
                let status = if current_size >= SIZE_RED { "危险" }
                             else if current_size >= SIZE_YELLOW { "注意" }
                             else { "正常" };
                let _ = t.set_tooltip(Some(&format!(
                    "ctx-guard 监控中 [{}]\naudit.jsonl: {}",
                    status, format_size(current_size)
                )));
            }

            if current_size <= prev_size { continue; }

            let mut file = match OpenOptions::new().read(true).open(&audit_path) {
                Ok(f) => f,
                Err(_) => continue,
            };
            let _ = file.seek(SeekFrom::Start(prev_size));
            let mut chunk = String::new();
            let _ = file.read_to_string(&mut chunk);
            *last_size.lock().unwrap() = current_size;

            for kw in KEYWORDS {
                if chunk.contains(kw) {
                    let path_str = audit_path.to_string_lossy().to_string();
                    let out = generate_handoff(kw, &path_str);
                    show_notification("ctx-guard 警告", "即将触发 1M 限制！交接文档已保存到桌面。");
                    let _ = app.emit("alert", &out);
                    if let Some(t) = app.tray_by_id("main") {
                        let _ = t.set_tooltip(Some("ctx-guard: 已触发警告！交接文档已保存到桌面"));
                    }
                    break;
                }
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }

            let handle = app.handle().clone();

            let quit = MenuItem::with_id(app, "quit", "退出 ctx-guard", true, None::<&str>)?;
            let status = MenuItem::with_id(app, "status", "✅ 监控中...", false, None::<&str>)?;
            let handoff = MenuItem::with_id(app, "handoff", "📝 立即生成交接文档", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&status, &handoff, &quit])?;

            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(true)
                .tooltip("ctx-guard: 启动中...")
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
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
