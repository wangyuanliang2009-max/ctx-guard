<div align="center">

# ⚡ ctx-guard

**Claude Code context usage monitor**

Know before you hit the 1M limit.

[![npm version](https://img.shields.io/npm/v/ctx-guard?color=7F77DD&labelColor=EEEDFE)](https://www.npmjs.com/package/ctx-guard)
[![License: MIT](https://img.shields.io/badge/License-MIT-teal.svg)](LICENSE)
[![Node.js ≥ 16](https://img.shields.io/badge/node-%3E%3D16-brightgreen)](https://nodejs.org)
[![CI](https://github.com/wangyuanliang2009-max/ctx-guard/actions/workflows/ci.yml/badge.svg)](https://github.com/wangyuanliang2009-max/ctx-guard/actions)

[English](#english) · [中文](#中文)

</div>

---

## English

### The Problem

When using Claude Code or Cowork, you get this error with no warning:

```
Usage credits required for 1M context
```

By then it's too late. Your session stalls, your flow breaks.

**ctx-guard monitors your context usage in real time so you can act before hitting the limit.**

### Demo

```
⚡ Claude Code Context Monitor          15:32:04

  ████████████████████░░░░░░░░░░  57.0%

  Used   :  570.0K / 1.00M
  Remaining:  430.0K

  Model  : claude-sonnet-4-6
  Session: ~/.claude/projects/my-app/abc123.jsonl
```

With `--hourglass`:

```
⚡ Claude Code Context Monitor          15:32:04

   █████████████████
     █████████████
       █████████
         █████
          ███
            ▓
          ███
         █░░░█
       █░░░░░░░█
     █░░░░░░░░░░░█
   █░░░░░░░░░░░░░░░█

  ████████████████████░░░░░░░░░░  57.0%
```

### Install

```bash
npm install -g ctx-guard
```

Requires Node.js ≥ 16. Zero external dependencies.

### Usage

```bash
ctx-guard                     # One-time check of your latest session
ctx-guard --watch             # Live watch, refreshes every 5 seconds
ctx-guard --hourglass         # Add ASCII hourglass visualisation
ctx-guard --all               # Overview of all sessions
ctx-guard --demo              # Try it without real data
ctx-guard --demo --pct 0.92   # Simulate 92% usage
ctx-guard --fix-1m            # Generate a .claude/settings.json to avoid accidental 1M triggers
ctx-guard --lang zh           # Force Chinese (auto-detected by default)
ctx-guard --watch --interval 10  # Custom refresh interval (seconds)
ctx-guard --help
```

### How It Works

Claude Code writes every API response to:

```
~/.claude/projects/<project-hash>/<session-id>.jsonl
```

Each line contains the exact `usage` object from the Anthropic API:

```json
{
  "usage": {
    "input_tokens": 1234,
    "output_tokens": 567,
    "cache_read_input_tokens": 890,
    "cache_creation_input_tokens": 100
  }
}
```

ctx-guard reads these files directly — **no estimation, no guessing, exact numbers from the API.**

### Alert Levels

| Colour | Threshold | Behaviour |
|--------|-----------|-----------|
| 🟢 Green | 0 – 70% | Normal |
| 🟡 Yellow | 70 – 85% | Warning message |
| 🔴 Red | 85 – 95% | Strong warning |
| 🚨 Critical | 95%+ | Flashing red background |

### Language Auto-Detection

ctx-guard reads your system locale (`$LANG`, `$LANGUAGE`) and picks the right language automatically. No flags needed.

Supported: 🇨🇳 Chinese · 🇺🇸 English · 🇯🇵 Japanese · 🇰🇷 Korean · 🇪🇸 Spanish · 🇫🇷 French · 🇩🇪 German

Override with `--lang zh/en/ja/ko/es/fr/de`.

### Fix: Avoid Accidental 1M Triggers

```bash
ctx-guard --fix-1m
```

Creates `.claude/settings.json` in your current project with `maxTokens: 180000`, which prevents Claude Code from accidentally entering 1M context mode when you don't need it.

### Tested Platforms

- macOS, Linux, Windows
- Node.js 16, 18, 20, 22
- CI: GitHub Actions (3 OS × 4 Node versions)

---

## 中文

### 解决什么问题

使用 Claude Code 或 Cowork 时，经常没有任何提示就触发：

```
Usage credits required for 1M context
```

等你看到这个报错，session 已经卡死，工作流全断。

**ctx-guard 实时监控你的上下文用量，让你在撞墙之前就能看到并处理。**

### 安装

```bash
npm install -g ctx-guard
```

Node.js ≥ 16，零外部依赖，一行安装。

### 使用方式

```bash
ctx-guard                   # 单次检查当前最新 session
ctx-guard --watch           # 实时监控（默认每 5 秒刷新）
ctx-guard --hourglass       # 显示 ASCII 沙漏图形
ctx-guard --all             # 所有 session 概览
ctx-guard --demo            # 演示模式（无需真实数据）
ctx-guard --fix-1m          # 在当前项目生成 .claude/settings.json，防止误触 1M
```

### 数据来源

直接读取 `~/.claude/projects/**/*.jsonl`，使用 Anthropic API 返回的精确 token 数，**不是估算**。

### 告警级别

| 颜色 | 阈值 | 行为 |
|------|------|------|
| 🟢 绿色 | 0 – 70% | 正常 |
| 🟡 黄色 | 70 – 85% | 警告提示 |
| 🔴 红色 | 85 – 95% | 强烈警告 |
| 🚨 危险 | 95%+ | 红底白字闪烁 |

### 语言自动检测

自动读取系统 `$LANG` 环境变量，无需手动指定语言。支持：中文、英文、日文、韩文、西班牙文、法文、德文。

---

## Contributing

Issues and PRs welcome. The codebase is intentionally small — `src/reader.js` handles data, `src/ui.js` handles display, `bin/ctx-guard.js` is the CLI.

```bash
git clone https://github.com/wangyuanliang2009-max/ctx-guard.git
cd ctx-guard
node test/reader.test.js   # Run tests (zero dependencies)
node bin/ctx-guard.js --demo --hourglass  # Try it
```

## License

MIT © [wangyuanliang2009-max](https://github.com/wangyuanliang2009-max)
