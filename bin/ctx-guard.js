#!/usr/bin/env node
'use strict';

const { findLatestSession, getAllSessions, mockSession, CONTEXT_LIMIT } = require('../src/reader');
const { render, renderNoData, detectLang, fmtK } = require('../src/ui');

// ─── 参数解析 ────────────────────────────────────────────────────────────────
const args = process.argv.slice(2);
const has   = flag => args.includes(flag);
const get   = flag => { const i = args.indexOf(flag); return i !== -1 ? args[i + 1] : null; };

const isWatch     = has('--watch') || has('-w');
const isAll       = has('--all')   || has('-a');
const isDemo      = has('--demo');
const isHourglass = has('--hourglass') || has('-H');
const isFix1m     = has('--fix-1m');
const isHelp      = has('--help') || has('-h');
const langOpt     = get('--lang');
const intervalSec = parseInt(get('--interval') || '5', 10);
const demoPct     = parseFloat(get('--pct') || '0.57');

const lang = detectLang(langOpt);

// ─── 帮助 ────────────────────────────────────────────────────────────────────
if (isHelp) {
  console.log(`
ctx-guard — Claude Code context monitor

Usage:
  ctx-guard                   Single check (latest session)
  ctx-guard --watch           Live watch (refresh every 5s)
  ctx-guard --all             Show all sessions overview
  ctx-guard --hourglass       Show ASCII hourglass visualisation
  ctx-guard --demo            Demo mode (no real data needed)
  ctx-guard --demo --pct 0.9  Demo at 90% usage
  ctx-guard --fix-1m          Generate .claude/settings.json fix
  ctx-guard --lang zh/en/ja/ko/es/fr/de
                              Override language (auto-detected by default)
  ctx-guard --interval <sec>  Watch interval in seconds (default: 5)
`);
  process.exit(0);
}

// ─── --fix-1m：生成 .claude/settings.json ──────────────────────────────────
if (isFix1m) {
  const fs = require('fs');
  const path = require('path');
  const dir  = path.join(process.cwd(), '.claude');
  const file = path.join(dir, 'settings.json');
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
  const content = JSON.stringify({ model: 'claude-sonnet-4-6', maxTokens: 180000 }, null, 2);
  fs.writeFileSync(file, content);
  console.log(`✅  Created ${file}`);
  console.log('    maxTokens set to 180,000 — avoids accidental 1M context trigger.');
  process.exit(0);
}

// ─── --all：多 session 概览 ──────────────────────────────────────────────────
if (isAll) {
  const sessions = getAllSessions();
  if (!sessions.length) { renderNoData(langOpt); process.exit(0); }
  const C = { reset:'\x1b[0m', bold:'\x1b[1m', green:'\x1b[32m', yellow:'\x1b[33m', red:'\x1b[31m', cyan:'\x1b[36m', gray:'\x1b[90m' };
  console.log(C.cyan + C.bold + '⚡ ctx-guard — All Sessions' + C.reset);
  console.log(C.gray + '─'.repeat(65) + C.reset);
  for (const s of sessions.slice(0, 20)) {
    const pct = Math.min(s.totalTokens / CONTEXT_LIMIT, 1);
    const color = pct >= 0.85 ? C.red : pct >= 0.70 ? C.yellow : C.green;
    const bar = color + '█'.repeat(Math.round(pct * 20)) + C.reset + '\x1b[2m' + '░'.repeat(20 - Math.round(pct * 20)) + C.reset;
    const short = s.filePath.replace(process.env.HOME || '', '~').slice(-45).padEnd(45);
    console.log(`  ${bar}  ${color}${(pct*100).toFixed(1).padStart(5)}%${C.reset}  ${C.gray}${short}${C.reset}`);
  }
  console.log(C.gray + '─'.repeat(65) + C.reset);
  process.exit(0);
}

// ─── 单次 / watch ────────────────────────────────────────────────────────────
function run() {
  const session = isDemo ? mockSession(demoPct) : findLatestSession();
  if (!session) { renderNoData(langOpt); return; }
  render(session, { lang: langOpt, hourglass: isHourglass });
}

run();

if (isWatch) {
  const timer = setInterval(run, intervalSec * 1000);
  process.on('SIGINT', () => { clearInterval(timer); process.exit(0); });
}
