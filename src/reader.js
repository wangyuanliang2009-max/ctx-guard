'use strict';

const fs = require('fs');
const path = require('path');
const os = require('os');

const CONTEXT_LIMIT = 1_000_000;
const PROJECTS_DIR = path.join(os.homedir(), '.claude', 'projects');

// Cowork 路径（Windows）
const COWORK_BASE = path.join(
  os.homedir(), 'AppData', 'Local', 'Packages',
  'Claude_pzs8sxrjxfjjc', 'LocalCache', 'Roaming', 'Claude',
  'local-agent-mode-sessions'
);

function findJsonlFiles(dir) {
  if (!fs.existsSync(dir)) return [];
  const results = [];
  try {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, entry.name);
      try {
        if (entry.isDirectory()) {
          results.push(...findJsonlFiles(full));
        } else if (entry.isFile() && entry.name.endsWith('.jsonl') && !entry.name.includes('audit')) {
          results.push(full);
        }
      } catch (_) {}
    }
  } catch (_) {}
  return results;
}

function parseSession(filePath) {
  const lines = fs.readFileSync(filePath, 'utf8').trim().split('\n');
  let inputTokens = 0, outputTokens = 0, cacheRead = 0, cacheCreate = 0;
  let model = 'unknown', lastTimestamp = null;

  for (const line of lines) {
    if (!line.trim()) continue;
    try {
      const entry = JSON.parse(line);
      if (entry.model) model = entry.model;
      if (entry.timestamp) lastTimestamp = entry.timestamp;
      if (entry.message && entry.message.usage) {
        const u = entry.message.usage;
        inputTokens  += u.input_tokens                || 0;
        outputTokens += u.output_tokens               || 0;
        cacheRead    += u.cache_read_input_tokens     || 0;
        cacheCreate  += u.cache_creation_input_tokens || 0;
      }
      if (entry.usage) {
        const u = entry.usage;
        inputTokens  += u.input_tokens                || 0;
        outputTokens += u.output_tokens               || 0;
        cacheRead    += u.cache_read_input_tokens     || 0;
        cacheCreate  += u.cache_creation_input_tokens || 0;
      }
    } catch (_) {}
  }

  const totalTokens = inputTokens + outputTokens + cacheRead + cacheCreate;
  return { filePath, model, totalTokens, inputTokens, outputTokens, cacheRead, cacheCreate, lastTimestamp };
}

function findLatestSession() {
  const files = [
    ...findJsonlFiles(PROJECTS_DIR),
    ...findJsonlFiles(COWORK_BASE),
  ];
  if (!files.length) return null;
  const sessions = files.map(f => {
    try { const s = parseSession(f); s.mtime = fs.statSync(f).mtimeMs; return s; }
    catch (_) { return null; }
  }).filter(Boolean);
  if (!sessions.length) return null;
  return sessions.sort((a, b) => {
    const ta = a.lastTimestamp ? new Date(a.lastTimestamp).getTime() : a.mtime;
    const tb = b.lastTimestamp ? new Date(b.lastTimestamp).getTime() : b.mtime;
    return tb - ta;
  })[0];
}

function getAllSessions() {
  const files = [
    ...findJsonlFiles(PROJECTS_DIR),
    ...findJsonlFiles(COWORK_BASE),
  ];
  return files.map(f => {
    try { const s = parseSession(f); s.mtime = fs.statSync(f).mtimeMs; return s; }
    catch (_) { return null; }
  }).filter(Boolean).sort((a, b) => {
    const ta = a.lastTimestamp ? new Date(a.lastTimestamp).getTime() : a.mtime;
    const tb = b.lastTimestamp ? new Date(b.lastTimestamp).getTime() : b.mtime;
    return tb - ta;
  });
}

function mockSession(pct = 0.57) {
  const total = Math.round(CONTEXT_LIMIT * pct);
  return {
    filePath: '~/.claude/projects/demo/demo-session.jsonl',
    model: 'claude-sonnet-4-6',
    totalTokens: total,
    inputTokens: Math.round(total * 0.6),
    outputTokens: Math.round(total * 0.15),
    cacheRead:    Math.round(total * 0.2),
    cacheCreate:  Math.round(total * 0.05),
    lastTimestamp: new Date().toISOString(),
    mtime: Date.now(),
    isDemo: true,
  };
}

module.exports = { findLatestSession, getAllSessions, parseSession, mockSession, CONTEXT_LIMIT, PROJECTS_DIR };