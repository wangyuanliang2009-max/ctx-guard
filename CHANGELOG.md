# Changelog

## v2.2.0 (2026-06-10)

### 🎯 核心升级：真实 token 计数

- **从"文件大小估算"升级为"API 真实上报值"**：直接解析 audit.jsonl 中每条 assistant 消息的 `message.usage` 字段（input_tokens + cache_creation_input_tokens + cache_read_input_tokens），悬浮窗显示的百分比首次成为精确数字
- 套餐档位从字节数（1MB/512KB/200KB）改为真实 token 数（1,000,000 / 500,000 / 200,000）
- 托盘 tooltip 显示格式更新为 token 计数（如 `58.7K tok / 1.00M tok`）
- 增量读取优化：每次只解析日志新增部分，启动时只读文件末尾 256KB

### 🪟 悬浮窗视觉重构

- 修复 Windows 10 下四角白色直角问题（transparent + CSS border-radius 方案，移除全部 WebView2 vtable hack 代码）
- 修复底部内容被系统边框裁切的问题（Tauri #12285 已知 bug 的规避：`height: calc(100vh - 8px)`）
- 窗口比例调整为 88×118 纵向长方形
- 边缘抗锯齿优化（GPU 合成层 + 半透明描边过渡）
- 百分比字号、套餐标签亮度等细节对齐设计稿

### 🧹 代码清理

- 删除 `inject_transparent_background`（约 60 行 unsafe vtable 调用，已被 CSS 方案取代）
- `tauri.conf.json` 增加 `"shadow": false` 消除系统边框干扰

### 已知限制（诚实声明）

- token 数仅在每次 AI 回复后更新，非逐字实时
- 依赖 Cowork 内部日志格式，官方更新可能需要适配
- 仅支持 Windows（Mac 路线图中）

## v2.1.0

- audit.jsonl 实时监控 + 关键词检测
- 自动生成交接文档到桌面
- 系统托盘常驻，图标三色变化
- 套餐选择与记忆
- 悬浮窗拖动/关闭
- 开机自启
