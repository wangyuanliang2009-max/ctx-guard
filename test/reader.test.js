'use strict';
// 零依赖测试套件

const assert = require('assert');
const fs     = require('fs');
const os     = require('os');
const path   = require('path');
const { parseSession, mockSession, CONTEXT_LIMIT } = require('../src/reader');
const { detectLang, renderBar, fmtK }              = require('../src/ui');

let passed = 0, failed = 0;

function test(name, fn) {
  try { fn(); console.log(`  ✅  ${name}`); passed++; }
  catch (e) { console.error(`  ❌  ${name}\n     ${e.message}`); failed++; }
}

// ── 创建临时 .jsonl 文件 ─────────────────────────────────────────────────────
const tmpDir  = fs.mkdtempSync(path.join(os.tmpdir(), 'ctx-guard-'));
const tmpFile = path.join(tmpDir, 'session.jsonl');

const sampleLines = [
  { model: 'claude-sonnet-4-6', timestamp: '2026-06-04T10:00:00Z', usage: { input_tokens: 1000, output_tokens: 200, cache_read_input_tokens: 500, cache_creation_input_tokens: 100 } },
  { model: 'claude-sonnet-4-6', timestamp: '2026-06-04T10:05:00Z', usage: { input_tokens: 2000, output_tokens: 400, cache_read_input_tokens: 300, cache_creation_input_tokens: 50  } },
  { model: 'claude-sonnet-4-6', timestamp: '2026-06-04T10:10:00Z', usage: { input_tokens: 500,  output_tokens: 100, cache_read_input_tokens: 0,   cache_creation_input_tokens: 0   } },
];
fs.writeFileSync(tmpFile, sampleLines.map(l => JSON.stringify(l)).join('\n'));

// ── reader 测试 ──────────────────────────────────────────────────────────────
console.log('\n📋  reader.js 测试');

test('parseSession: 正确累加所有 token', () => {
  const s = parseSession(tmpFile);
  assert.strictEqual(s.inputTokens,  3500);
  assert.strictEqual(s.outputTokens,  700);
  assert.strictEqual(s.cacheRead,     800);
  assert.strictEqual(s.cacheCreate,   150);
  assert.strictEqual(s.totalTokens,  5150);
});

test('parseSession: 正确读取 model', () => {
  const s = parseSession(tmpFile);
  assert.strictEqual(s.model, 'claude-sonnet-4-6');
});

test('parseSession: 正确读取最后 timestamp', () => {
  const s = parseSession(tmpFile);
  assert.strictEqual(s.lastTimestamp, '2026-06-04T10:10:00Z');
});

test('parseSession: 含无效行不崩溃', () => {
  const f = path.join(tmpDir, 'bad.jsonl');
  fs.writeFileSync(f, 'not json\n{"usage":{"input_tokens":100}}\n\n');
  const s = parseSession(f);
  assert.strictEqual(s.totalTokens, 100);
});

test('parseSession: 空文件返回 0 token', () => {
  const f = path.join(tmpDir, 'empty.jsonl');
  fs.writeFileSync(f, '');
  const s = parseSession(f);
  assert.strictEqual(s.totalTokens, 0);
});

test('mockSession: 百分比正确', () => {
  const s = mockSession(0.75);
  const pct = s.totalTokens / CONTEXT_LIMIT;
  assert.ok(Math.abs(pct - 0.75) < 0.01, `got ${pct}`);
});

test('CONTEXT_LIMIT: 等于 1,000,000', () => {
  assert.strictEqual(CONTEXT_LIMIT, 1_000_000);
});

// ── ui 测试 ──────────────────────────────────────────────────────────────────
console.log('\n🎨  ui.js 测试');

test('detectLang: 环境变量 zh 识别', () => {
  const orig = process.env.LANG;
  process.env.LANG = 'zh_CN.UTF-8';
  assert.strictEqual(detectLang(null), 'zh');
  process.env.LANG = orig || '';
});

test('detectLang: 手动覆盖优先', () => {
  process.env.LANG = 'zh_CN.UTF-8';
  assert.strictEqual(detectLang('en'), 'en');
});

test('detectLang: 未知语言回退 en', () => {
  const orig = process.env.LANG;
  process.env.LANG = 'xx_XX.UTF-8';
  assert.strictEqual(detectLang(null), 'en');
  process.env.LANG = orig || '';
});

test('fmtK: 格式化 token 数', () => {
  assert.ok(fmtK(1234).includes('K'));
  assert.ok(fmtK(1_000_000).includes('M'));
  assert.strictEqual(fmtK(500), '500');
});

// ── 清理 ─────────────────────────────────────────────────────────────────────
fs.rmSync(tmpDir, { recursive: true });

// ── 结果 ─────────────────────────────────────────────────────────────────────
console.log(`\n${'─'.repeat(40)}`);
console.log(`结果：${passed} passed, ${failed} failed`);
if (failed > 0) process.exit(1);
