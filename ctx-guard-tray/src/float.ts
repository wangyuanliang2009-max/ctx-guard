import { listen } from '@tauri-apps/api/event';
import { Window } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';

const win = Window.getCurrent();

document.getElementById('app')!.addEventListener('mousedown', (e) => {
  if ((e.target as HTMLElement).id === 'close') return;
  win.startDragging();
});
document.getElementById('close')!.addEventListener('click', () => win.hide());

// 确保 DOM 加载完成后立即触发，不管时机早晚


const PLAN_NAMES: Record<string,string> = {'1048576':'MAX 1M','524288':'MAX 500K','204800':'PRO 200K'};
const ctxLabel = document.getElementById('ctx-label')!;
const pctEl    = document.getElementById('pct')!;
const bar      = document.getElementById('bar')!;
const planEl   = document.getElementById('plan-lbl')!;

let maxBytes = 1048576;

(async () => {
  try {
    const saved = await invoke<number>('get_max_bytes');
    if (saved > 0) { maxBytes=saved; planEl.textContent=PLAN_NAMES[String(maxBytes)]||'CUSTOM'; }
  } catch(e) {}
})();

function setColor(color: string) {
  const c  = color==='red'?'#ef4444':color==='yellow'?'#eab308':'#22c55e';
  const lc = color==='red'?'rgba(239,68,68,0.75)':color==='yellow'?'rgba(234,179,8,0.75)':'rgba(34,197,94,0.75)';
  pctEl.style.color=c; bar.style.background=c; ctxLabel.style.color=lc;
}

function update(sizeBytes: number, color: string, max: number) {
  const r = Math.max(0, Math.min(1, 1-sizeBytes/max));
  pctEl.textContent=(r*100).toFixed(1)+'%';
  bar.style.width=(r*100)+'%';
  setColor(color);
  planEl.textContent=PLAN_NAMES[String(max)]||'CUSTOM';
  r<0.1?pctEl.classList.add('flashing'):pctEl.classList.remove('flashing');
}

listen('float-update', (e:any) => {
  const { sizeBytes, color, maxBytes: max } = e.payload;
  if(max) maxBytes=max;
  update(sizeBytes||0, color, maxBytes);
});
listen('plan-changed', (e:any) => { maxBytes=e.payload; planEl.textContent=PLAN_NAMES[String(maxBytes)]||'CUSTOM'; });
listen('alert', () => { update(maxBytes,'red',maxBytes); pctEl.textContent='0%'; });
