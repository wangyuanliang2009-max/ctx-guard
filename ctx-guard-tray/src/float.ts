import { listen } from '@tauri-apps/api/event';
import { Window } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import { Menu, MenuItem } from '@tauri-apps/api/menu';

const win = Window.getCurrent();

document.getElementById('app')!.addEventListener('mousedown', (e) => {
  if ((e.target as HTMLElement).id === 'close') return;
  win.startDragging();
});

document.getElementById('close')!.addEventListener('click', () => win.hide());

// v2.4.0 修复：key 与 lib.rs 写入的标准值对齐（之前用的是 1024 进制，lib.rs 用的是 10 进制整数）
const PLAN_NAMES: Record<string, string> = {
  '1000000': 'MAX 1M',
  '500000':  'MAX 500K',
  '200000':  'PRO 200K',
};

const ctxLabel = document.getElementById('ctx-label')!;
const pctEl    = document.getElementById('pct')!;
const bar      = document.getElementById('bar')!;
const planEl   = document.getElementById('plan-lbl')!;

let maxBytes = 1000000;

(async () => {
  try {
    const saved = await invoke<number>('get_max_bytes');
    if (saved > 0) {
      maxBytes = saved;
      planEl.textContent = PLAN_NAMES[String(maxBytes)] || 'CUSTOM';
    }
  } catch (e) {}
})();

// v2.4.0 新增：右键菜单（切换套餐 + 生成交接文档 + 退出）
document.getElementById('app')!.addEventListener('contextmenu', async (e) => {
  e.preventDefault();
  try {
    const menu = await Menu.new({
      items: [
        await MenuItem.new({ text: 'Max + 1M context',   action: () => invoke('set_max_bytes', { val: 1000000 }).then(() => { maxBytes=1000000; planEl.textContent='MAX 1M';    }) }),
        await MenuItem.new({ text: 'Max + 500K context', action: () => invoke('set_max_bytes', { val: 500000  }).then(() => { maxBytes=500000;  planEl.textContent='MAX 500K'; }) }),
        await MenuItem.new({ text: 'Pro / Max 200K',     action: () => invoke('set_max_bytes', { val: 200000  }).then(() => { maxBytes=200000;  planEl.textContent='PRO 200K'; }) }),
        await MenuItem.new({ text: '─────────────', enabled: false, action: () => {} }),
        await MenuItem.new({ text: '📝 立即生成交接文档', action: () => invoke('generate_handoff_manual') }),
        await MenuItem.new({ text: '─────────────', enabled: false, action: () => {} }),
        await MenuItem.new({ text: '退出 ctx-guard',     action: () => invoke('quit_app') }),
      ]
    });
    await menu.popup();
  } catch (err) {
    console.error('context menu error:', err);
  }
});

function setColor(color: string) {
  const c  = color === 'red'    ? '#ef4444' : color === 'yellow' ? '#eab308' : '#22c55e';
  const lc = color === 'red'    ? 'rgba(239,68,68,0.75)'
           : color === 'yellow' ? 'rgba(234,179,8,0.75)'
           :                      'rgba(34,197,94,0.75)';
  pctEl.style.color = c;
  bar.style.background = c;
  ctxLabel.style.color = lc;
}

function update(sizeBytes: number, color: string, max: number) {
  const r = Math.max(0, Math.min(1, 1 - sizeBytes / max));
  pctEl.textContent = (r * 100).toFixed(1) + '%';
  bar.style.width   = (r * 100) + '%';
  setColor(color);
  planEl.textContent = PLAN_NAMES[String(max)] || 'CUSTOM';
  r < 0.1 ? pctEl.classList.add('flashing') : pctEl.classList.remove('flashing');
}

listen('float-update', (e: any) => {
  const { sizeBytes, color, maxBytes: max } = e.payload;
  if (max) maxBytes = max;
  update(sizeBytes || 0, color, maxBytes);
});

listen('plan-changed', (e: any) => {
  maxBytes = e.payload;
  planEl.textContent = PLAN_NAMES[String(maxBytes)] || 'CUSTOM';
});

listen('alert', () => {
  update(maxBytes, 'red', maxBytes);
  pctEl.textContent = '0%';
});
