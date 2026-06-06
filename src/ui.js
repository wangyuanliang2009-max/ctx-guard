'use strict';

// ─── 自动语言检测 ────────────────────────────────────────────────────────────
function detectLang(override) {
  if (override) return override;
  // 读取系统环境变量
  const env = process.env.LANG || process.env.LANGUAGE || process.env.LC_ALL || process.env.LC_MESSAGES || '';
  const loc = env.toLowerCase();
  if (loc.startsWith('zh')) return 'zh';
  if (loc.startsWith('ja')) return 'ja';
  if (loc.startsWith('ko')) return 'ko';
  if (loc.startsWith('es')) return 'es';
  if (loc.startsWith('fr')) return 'fr';
  if (loc.startsWith('de')) return 'de';
  if (loc.startsWith('ru')) return 'ru';
  if (loc.startsWith('pt')) return 'pt';
  return 'en'; // 默认英文
}

// ─── 多语言包 ────────────────────────────────────────────────────────────────
const LANG = {
  zh: {
    title:   'Claude Code 上下文监控',
    used:    '已用',
    remain:  '剩余',
    model:   '模型',
    session: '会话',
    demo:    '演示模式',
    warn70:  '⚠️  注意：上下文已超 70%，建议关注',
    warn85:  '🔴  警告：上下文已超 85%，请及时处理',
    warn95:  '🚨  危险：上下文超 95%，即将触发限制！',
    noData:  '未找到 Claude Code 会话数据\n路径：~/.claude/projects/\n运行 --demo 查看演示',
    hourglass_full: '沙漏已满！',
  },
  en: {
    title:   'Claude Code Context Monitor',
    used:    'Used',
    remain:  'Remaining',
    model:   'Model',
    session: 'Session',
    demo:    'Demo Mode',
    warn70:  '⚠️  Warning: context over 70%, keep an eye on it',
    warn85:  '🔴  Alert: context over 85%, take action soon',
    warn95:  '🚨  Critical: context over 95%, limit imminent!',
    noData:  'No Claude Code session data found\nPath: ~/.claude/projects/\nRun --demo to see a demo',
    hourglass_full: 'Hourglass Full!',
  },
  ja: {
    title:   'Claude Code コンテキストモニター',
    used:    '使用済み',
    remain:  '残り',
    model:   'モデル',
    session: 'セッション',
    demo:    'デモモード',
    warn70:  '⚠️  注意：コンテキストが70%を超えました',
    warn85:  '🔴  警告：コンテキストが85%を超えました。対応してください',
    warn95:  '🚨  危険：コンテキストが95%超！制限が迫っています！',
    noData:  'Claude Codeのセッションデータが見つかりません\nパス: ~/.claude/projects/\n--demo でデモを確認できます',
    hourglass_full: '砂時計が満杯です！',
  },
  ko: {
    title:   'Claude Code 컨텍스트 모니터',
    used:    '사용됨',
    remain:  '남음',
    model:   '모델',
    session: '세션',
    demo:    '데모 모드',
    warn70:  '⚠️  주의: 컨텍스트가 70%를 초과했습니다',
    warn85:  '🔴  경고: 컨텍스트가 85%를 초과했습니다. 조치를 취하세요',
    warn95:  '🚨  위험: 컨텍스트가 95% 초과! 제한이 임박했습니다!',
    noData:  'Claude Code 세션 데이터를 찾을 수 없습니다\n경로: ~/.claude/projects/\n--demo 로 데모를 확인하세요',
    hourglass_full: '모래시계가 가득 찼습니다!',
  },
  es: {
    title:   'Monitor de Contexto Claude Code',
    used:    'Usado',
    remain:  'Restante',
    model:   'Modelo',
    session: 'Sesión',
    demo:    'Modo Demo',
    warn70:  '⚠️  Aviso: contexto supera el 70%',
    warn85:  '🔴  Alerta: contexto supera el 85%, actúa pronto',
    warn95:  '🚨  Crítico: contexto supera el 95%, ¡límite inminente!',
    noData:  'No se encontraron datos de sesión de Claude Code\nRuta: ~/.claude/projects/\nEjecuta --demo para ver una demo',
    hourglass_full: '¡Reloj de arena lleno!',
  },
  fr: {
    title:   'Moniteur de Contexte Claude Code',
    used:    'Utilisé',
    remain:  'Restant',
    model:   'Modèle',
    session: 'Session',
    demo:    'Mode Démo',
    warn70:  '⚠️  Avertissement : contexte dépasse 70%',
    warn85:  '🔴  Alerte : contexte dépasse 85%, agissez rapidement',
    warn95:  '🚨  Critique : contexte dépasse 95%, limite imminente !',
    noData:  'Aucune donnée de session Claude Code trouvée\nChemin : ~/.claude/projects/\nLancez --demo pour voir une démo',
    hourglass_full: 'Sablier plein !',
  },
  de: {
    title:   'Claude Code Kontext-Monitor',
    used:    'Genutzt',
    remain:  'Verbleibend',
    model:   'Modell',
    session: 'Sitzung',
    demo:    'Demo-Modus',
    warn70:  '⚠️  Hinweis: Kontext über 70%',
    warn85:  '🔴  Warnung: Kontext über 85%, bitte handeln',
    warn95:  '🚨  Kritisch: Kontext über 95%, Limit steht bevor!',
    noData:  'Keine Claude Code-Sitzungsdaten gefunden\nPfad: ~/.claude/projects/\n--demo für eine Demo ausführen',
    hourglass_full: 'Sanduhr voll!',
  },
};
// 未匹配语言回退到英文
LANG.ru = LANG.pt = LANG.en;

// ─── ANSI 颜色 ───────────────────────────────────────────────────────────────
const C = {
  reset:   '\x1b[0m',
  bold:    '\x1b[1m',
  blink:   '\x1b[5m',
  green:   '\x1b[32m',
  yellow:  '\x1b[33m',
  red:     '\x1b[31m',
  bgRed:   '\x1b[41m',
  white:   '\x1b[37m',
  cyan:    '\x1b[36m',
  gray:    '\x1b[90m',
  dim:     '\x1b[2m',
};

// ─── 工具函数 ────────────────────────────────────────────────────────────────
function fmtK(n) {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(2) + 'M';
  if (n >= 1_000)     return (n / 1_000).toFixed(1) + 'K';
  return String(n);
}

function levelColor(pct) {
  if (pct >= 0.95) return C.bgRed + C.white + C.bold;
  if (pct >= 0.85) return C.red   + C.bold;
  if (pct >= 0.70) return C.yellow + C.bold;
  return C.green + C.bold;
}

// ─── 进度条 ──────────────────────────────────────────────────────────────────
function renderBar(pct, width = 30) {
  const filled = Math.round(pct * width);
  const empty  = width - filled;
  const color  = levelColor(pct);
  return color + '█'.repeat(filled) + C.reset + C.dim + '░'.repeat(empty) + C.reset;
}

// ─── ASCII 沙漏 ──────────────────────────────────────────────────────────────
//  沙漏宽度固定 17 个字符（含两侧 ██）
//  上半 = 已用（从顶部往下填充）
//  下半 = 剩余（从腰部往下，用 ░ 表示空位）
function renderHourglass(pct) {
  // 沙漏每层的宽度（字符数，代表沙粒列数），从顶到腰共 6 行
  const rowWidths = [8, 6, 4, 2, 1, 0]; // 上半（顶→腰），0=腰
  // 下半对称
  const bottomWidths = [0, 1, 2, 4, 6, 8]; // 腰→底

  const TOTAL_CELLS = rowWidths.reduce((a, b) => a + b, 0) * 2; // 上下对称
  const filledCells = Math.round(pct * TOTAL_CELLS);

  const color = levelColor(pct);
  const lines = [];
  const W = 20; // 总宽度（用于居中）

  let remaining = filledCells;

  // 上半部分（顶→腰），已用 = 实心
  const upperRows = [];
  for (let i = 0; i < rowWidths.length; i++) {
    const w = rowWidths[i];
    const indent = ' '.repeat((W - w * 2 - 2) / 2); // 两侧各留 █ 边框

    if (w === 0) {
      // 腰部分隔线
      upperRows.push(' '.repeat(W / 2) + (color + '▓' + C.reset) + ' '.repeat(W / 2));
      continue;
    }

    const fill = Math.min(remaining, w);
    remaining -= fill;
    const empty = w - fill;

    const inner = (fill > 0 ? color + '█'.repeat(fill) + C.reset : '') +
                  (empty > 0 ? C.dim + '░'.repeat(empty) + C.reset : '');
    // 镜像：左右各一半
    const innerMirror = (empty > 0 ? C.dim + '░'.repeat(empty) + C.reset : '') +
                        (fill > 0 ? color + '█'.repeat(fill) + C.reset : '');
    upperRows.push(indent + C.gray + '█' + C.reset + innerMirror + inner + C.gray + '█' + C.reset);
  }

  // 下半部分（腰→底），剩余 = 空心
  const lowerRows = [];
  for (let i = 0; i < bottomWidths.length; i++) {
    const w = bottomWidths[i];
    if (w === 0) continue;
    const indent = ' '.repeat((W - w * 2 - 2) / 2);

    const fill = Math.min(remaining, w);
    remaining -= fill;
    const empty = w - fill;

    const inner = (fill > 0 ? color + '█'.repeat(fill) + C.reset : '') +
                  (empty > 0 ? C.dim + '░'.repeat(empty) + C.reset : '');
    const innerMirror = (empty > 0 ? C.dim + '░'.repeat(empty) + C.reset : '') +
                        (fill > 0 ? color + '█'.repeat(fill) + C.reset : '');
    lowerRows.push(indent + C.gray + '█' + C.reset + innerMirror + inner + C.gray + '█' + C.reset);
  }

  return [...upperRows, ...lowerRows];
}

// ─── 主渲染函数 ──────────────────────────────────────────────────────────────
function render(session, { lang: langOverride, hourglass = false } = {}) {
  const langKey = detectLang(langOverride);
  const T = LANG[langKey] || LANG.en;
  const LIMIT = require('./reader').CONTEXT_LIMIT;

  const now = new Date().toLocaleTimeString();
  const pct = Math.min(session.totalTokens / LIMIT, 1);
  const pctStr = (pct * 100).toFixed(1) + '%';
  const color = levelColor(pct);

  // 清屏
  process.stdout.write('\x1b[2J\x1b[H');

  // ── 标题栏 ──
  const demoTag = session.isDemo ? C.yellow + ` [${T.demo}]` + C.reset : '';
  console.log(C.cyan + C.bold + `⚡ ${T.title}` + C.reset + demoTag + C.gray + `  ${now}` + C.reset);
  console.log(C.gray + '─'.repeat(50) + C.reset);

  // ── 沙漏 or 进度条 ──
  if (hourglass) {
    const hgLines = renderHourglass(pct);
    const W = 20;
    console.log('');
    for (const line of hgLines) {
      console.log('  ' + line);
    }
    console.log('');
  }

  // 进度条（始终显示）
  const bar = renderBar(pct);
  console.log(`  ${bar}  ${color}${pctStr}${C.reset}`);
  console.log('');

  // ── 详细数字 ──
  console.log(`  ${color}${T.used}   :${C.reset}  ${fmtK(session.totalTokens)} / ${fmtK(LIMIT)}`);
  console.log(`  ${C.green}${T.remain}:${C.reset}  ${fmtK(LIMIT - session.totalTokens)}`);
  console.log('');
  console.log(`  ${C.dim}${T.model}  : ${session.model}${C.reset}`);
  if (session.filePath) {
    const short = session.filePath.replace(process.env.HOME || '', '~');
    console.log(`  ${C.dim}${T.session}: ${short.slice(0, 55)}${C.reset}`);
  }

  // ── 告警信息 ──
  if (pct >= 0.95) {
    console.log('');
    console.log(C.bgRed + C.white + C.blink + C.bold + `  ${T.warn95}  ` + C.reset);
  } else if (pct >= 0.85) {
    console.log('');
    console.log(C.red + C.bold + `  ${T.warn85}` + C.reset);
  } else if (pct >= 0.70) {
    console.log('');
    console.log(C.yellow + `  ${T.warn70}` + C.reset);
  }

  console.log('');
  console.log(C.gray + '─'.repeat(50) + C.reset);
}

function renderNoData(langOverride) {
  const T = LANG[detectLang(langOverride)] || LANG.en;
  console.log(C.yellow + `⚡ ctx-guard` + C.reset);
  console.log(C.gray + T.noData + C.reset);
}

module.exports = { render, renderBar, renderHourglass, renderNoData, detectLang, LANG, fmtK };
