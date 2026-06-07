/**
 * ctx-guard / watcher.js
 * 实时监控 audit.jsonl，检测到 1M 报错信号后自动生成交接文档
 * 用法：node watcher.js [audit.jsonl路径]
 */

const fs = require('fs');
const path = require('path');
const os = require('os');

// ─── 配置 ───────────────────────────────────────────────────────────────────

const KEYWORDS = [
  'Usage credits required for 1M context',
  'turn on usage credits',
  'switch to standard context',
  'context window',        // 备用：捕获其他上下文相关错误
];

// audit.jsonl 默认路径（Windows）
// 支持通配符 session-id，watcher 会自动扫描最新的 session
const AUDIT_BASE = path.join(
  os.homedir(),
  'AppData/Local/Packages/Claude_pzs8sxrjxfjjc/LocalCache/Roaming/Claude/local-agent-mode-sessions'
);

// 交接文档输出目录（桌面）
const OUTPUT_DIR = path.join(os.homedir(), 'Desktop');

// 轮询间隔（毫秒）
const POLL_INTERVAL = 2000;

// ─── 工具函数 ────────────────────────────────────────────────────────────────

function log(msg) {
  const ts = new Date().toLocaleTimeString('zh-CN');
  console.log(`[${ts}] ${msg}`);
}

function findLatestAuditFile() {
  try {
    if (!fs.existsSync(AUDIT_BASE)) return null;

    // 递归搜索所有 audit.jsonl 文件，取最新修改的
    const results = [];

    function walk(dir, depth = 0) {
      if (depth > 4) return;
      const entries = fs.readdirSync(dir, { withFileTypes: true });
      for (const entry of entries) {
        const full = path.join(dir, entry.name);
        if (entry.isDirectory()) {
          walk(full, depth + 1);
        } else if (entry.name === 'audit.jsonl') {
          const stat = fs.statSync(full);
          results.push({ path: full, mtime: stat.mtimeMs });
        }
      }
    }

    walk(AUDIT_BASE);
    if (!results.length) return null;

    results.sort((a, b) => b.mtime - a.mtime);
    return results[0].path;
  } catch (e) {
    return null;
  }
}

function containsKeyword(text) {
  for (const kw of KEYWORDS) {
    if (text.includes(kw)) return kw;
  }
  return null;
}

function generateHandoff(triggerKeyword, auditPath) {
  const now = new Date();
  const dateStr = now.toISOString().slice(0, 10);
  const timeStr = now.toLocaleTimeString('zh-CN');

  const content = `# ctx-guard 自动生成交接文档

生成时间：${dateStr} ${timeStr}
触发信号：\`${triggerKeyword}\`
来源文件：${auditPath}

---

## ⚠️ 触发原因

ctx-guard 监控到 Cowork 即将达到 1M token 上下文限制。
请立即开启新对话，并将此文档粘贴到新对话开头。

---

## 项目状态（请手动补充）

- **当前任务**：
- **完成进度**：
- **下一步**：
- **重要文件**：

---

## 新对话开始时说这句话

> 我在开发 ctx-guard 开源工具，请读取这份交接文档继续开发。
> 当前任务：[在此填写]

---

*由 ctx-guard watcher.js 自动生成*
`;

  const filename = `handoff_${dateStr.replace(/-/g, '')}_${now.getHours()}${String(now.getMinutes()).padStart(2, '0')}.md`;
  const outPath = path.join(OUTPUT_DIR, filename);
  fs.writeFileSync(outPath, content, 'utf8');
  return outPath;
}

// ─── 主监控逻辑 ───────────────────────────────────────────────────────────────

let lastSize = 0;
let lastAuditPath = null;
let triggered = false;

function watch(auditPath) {
  if (!fs.existsSync(auditPath)) {
    log(`文件不存在：${auditPath}`);
    return;
  }

  const stat = fs.statSync(auditPath);
  const currentSize = stat.size;

  if (currentSize === lastSize) return; // 没有新内容

  // 只读取新增内容，避免重复扫描
  const stream = fs.createReadStream(auditPath, {
    start: lastSize,
    end: currentSize,
    encoding: 'utf8',
  });

  let chunk = '';
  stream.on('data', d => (chunk += d));
  stream.on('end', () => {
    lastSize = currentSize;
    const kw = containsKeyword(chunk);
    if (kw && !triggered) {
      triggered = true;
      log(`🚨 检测到关键词：${kw}`);
      log('📝 正在生成交接文档...');
      const outPath = generateHandoff(kw, auditPath);
      log(`✅ 交接文档已保存到桌面：${path.basename(outPath)}`);
      log('💡 请立即开启新对话，粘贴交接文档继续开发');

      // 10秒后重置，允许再次触发（保留 lastSize，不重读旧内容）
      setTimeout(() => {
        triggered = false;
        log('🔄 监控重置，继续监听...');
      }, 10000);
    }
  });
}

function main() {
  // 支持命令行传入路径
  const argPath = process.argv[2];
  const targetPath = argPath || null;

  log('🛡️  ctx-guard watcher 启动');

  if (targetPath) {
    log(`📂 监控文件：${targetPath}`);
    lastAuditPath = targetPath;
    setInterval(() => watch(targetPath), POLL_INTERVAL);
  } else {
    log(`📂 自动搜索 audit.jsonl（每 ${POLL_INTERVAL / 1000}s 扫描一次）`);
    setInterval(() => {
      const found = findLatestAuditFile();
      if (!found) {
        log('⏳ 未找到 audit.jsonl，等待 Cowork 启动...');
        return;
      }
      if (found !== lastAuditPath) {
        lastAuditPath = found;
        // 跳过已有内容，只监听新增部分
        try { lastSize = fs.statSync(found).size; } catch(e) { lastSize = 0; }
        log(`📂 检测到新 session，从当前位置开始监听（跳过历史内容）`);
      }
      watch(found);
    }, POLL_INTERVAL);
  }

  log('👀 监控中... 按 Ctrl+C 退出');
}

main();
