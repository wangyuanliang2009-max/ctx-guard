<div align="center">

# ctx-guard

**Never lose your Cowork context again.**

监控 Claude Cowork 的真实 context 用量，在撞上限制前自动生成交接文档。

[功能](#-功能) · [安装](#-安装) · [工作原理](#-工作原理) · [常见问题](#-常见问题)
<img src="screenshot.png" width="180" alt="ctx-guard floating widget">
</div>

---

## 为什么需要它

用 Claude Cowork 做长任务时，最痛的时刻是：对话进行到一半，突然提示 context 已满，被迫开新会话——然后花半小时向"新的 Claude"重新解释项目背景。

ctx-guard 做两件事：

1. **桌面悬浮窗实时显示真实 token 用量**——数据来自 API 真实上报值，不是估算
2. **即将撞限时，自动生成交接文档到桌面**——新会话直接粘贴，无缝接力

## ✨ 功能

- 🎯 **真实 token 计数**：直接解析 Cowork 日志中 API 上报的 usage 数据（input + cache tokens），精确到个位
- 🪟 **极简悬浮窗**：88×118 深色圆角小窗，绿/黄/红三色状态，可拖动、可关闭、置顶显示
- 📝 **自动交接文档**：检测到 context 限制信号时，自动在桌面生成 handoff 文档
- 🔔 **系统通知**：撞限前 Windows 原生通知提醒
- 📊 **托盘常驻**：托盘图标随用量变色，悬停查看详情
- ⚙️ **套餐适配**：支持 1M / 500K / 200K context 三档（对应 Max 1M、Max、Pro 套餐）
- 🚀 **开机自启**：装完就忘，需要时它一直在

## 📥 安装

### 方式一：下载安装包（推荐）

从 [Releases](../../releases) 下载最新的 `.msi` 安装包，双击安装。

### 方式二：从源码构建

需要 [Rust](https://rustup.rs/) 和 [Node.js](https://nodejs.org/)：

```bash
git clone https://github.com/wangyuanliang2009-max/ctx-guard.git
cd ctx-guard/ctx-guard-tray
npm install
npm run tauri build
```

构建产物在 `src-tauri/target/release/bundle/` 下。

## 🔍 工作原理

Claude Cowork 在本地写入 `audit.jsonl` 日志，其中每条 AI 回复都包含 API 真实上报的 token 用量：

```
当前 context = input_tokens + cache_creation_input_tokens + cache_read_input_tokens
```

ctx-guard 每 2 秒检查日志增量（只读新增部分，几乎零开销），解析最新的 usage 数据并更新悬浮窗。当日志中出现 context 限制关键词时，自动生成交接文档。

**数据说明（诚实声明）**：

- token 数是 API 真实值，但仅在每次 AI 回复后更新，AI 思考/打字过程中不会实时变化
- 显示的是「当前对话累积 context」占所选套餐上限的比例
- 一切数据仅在本地读取和计算，不上传任何内容

## ❓ 常见问题

**Q: 支持 Mac 吗？**
A: 暂不支持。欢迎 Mac 用户提 issue 告知 audit.jsonl 的路径，会尽快适配。

**Q: 支持 Claude Code / Cursor / Codex 吗？**
A: 在路线图上。原理相同，只需适配各工具的日志路径和格式。

**Q: 悬浮窗挡住其他窗口了怎么办？**
A: 直接拖到屏幕角落，或点 × 关闭（托盘右键可随时重新打开）。

**Q: Cowork 更新后插件失效了？**
A: 本工具依赖 Cowork 的内部日志格式（无官方兼容性承诺），如失效请提 issue，通常是小适配。

## 🗺️ 路线图

- [x] 真实 token 计数（v2.2）
- [ ] 交接文档自动填充（从日志提取最近任务、文件、对话摘要）
- [ ] Mac 支持
- [ ] Claude Code / Cursor 支持
- [ ] 5小时/7天配额状态提示（日志中已有 rate_limit 数据）

## ☕ Buy me a coffee

如果这个工具帮到了你，可以请我喝杯咖啡：

[![PayPal](https://img.shields.io/badge/PayPal-打赏支持-blue?logo=paypal)](https://paypal.me/wangyuanliang2009)

## 📄 License

MIT
