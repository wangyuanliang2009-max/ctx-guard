# ctx-guard

> Never lose your Cowork context again. ctx-guard watches your Claude session and automatically saves a handoff document before you hit the 1M token limit.

---

## The problem

You're deep in a Cowork session. Claude suddenly stops with:

```
API Error: Usage credits required for 1M context
```

Everything is gone. You have to start over and re-explain everything from scratch.

## The solution

ctx-guard runs silently in your system tray. It monitors your Cowork session in real time. The moment it detects the 1M limit signal, it **automatically saves a handoff document to your Desktop** — before you even see the error.

Open a new conversation, paste the handoff document, and continue exactly where you left off.

---

## Handoff document

The handoff document (`handoff_XXXXXX.md`) is saved automatically to your **Desktop** when:
- ctx-guard detects the 1M limit signal in your session
- You click "立即生成交接文档" in the tray menu

It contains:
- The trigger signal and source file
- A template to fill in your current task, progress, and next steps
- A prompt to paste at the start of your new conversation

**Paste it at the start of your next conversation and Claude will pick up right where you left off.**

You can also generate one manually at any time from the tray icon — useful before starting a long session.

---

## Installation

Download `ctx-guard-tray_0.1.0_x64-setup.exe` and run it. No dependencies needed. **Only 2MB** — built with Tauri, not Electron.

After installing, ctx-guard starts automatically and appears as a small hourglass icon in your system tray (bottom-right corner).

---

## How it works

The hourglass shows your remaining context capacity:

| Color | Meaning |
|-------|---------|
| 🟢 Green | Plenty of context remaining |
| 🟡 Yellow | Context is filling up — keep an eye on it |
| 🔴 Red (pulsing) | Critical — handoff document saved automatically |

When you hover over the tray icon, you'll see the exact session file size and status.

**Right-click the tray icon** to:
- Generate a handoff document manually
- Switch your Claude plan (Pro 200K / Max 500K / Max 1M)
- Show or hide the floating hourglass widget
- Quit ctx-guard

---

## Supported plans

| Plan | Context limit |
|------|--------------|
| Pro | 200K tokens |
| Max (standard) | 200K tokens |
| Max + 1M context | 1M tokens |

Set your plan by right-clicking the tray icon → 切换套餐.

---

## Requirements

- Windows 10/11
- Claude Cowork (desktop app)
- Claude Max subscription with 1M context enabled

---

## For developers

ctx-guard monitors the `audit.jsonl` file written by Cowork during every session:

```
%LOCALAPPDATA%\Packages\Claude_pzs8sxrjxfjjc\LocalCache\Roaming\Claude\local-agent-mode-sessions\
```

It watches for the error signal `Usage credits required for 1M context` and triggers handoff document generation the moment it appears — faster than you can read the error message.

Built with Tauri + Rust. Source: `ctx-guard-tray/src-tauri/src/lib.rs`

---

## Roadmap

- [ ] macOS support
- [ ] Auto-fill handoff document from session context
- [ ] Support for other Claude-powered tools (Cursor, Codex, etc.)
- [ ] Usage statistics and session history

---

*Built for Claude Pro / Max users who refuse to lose their work.*
