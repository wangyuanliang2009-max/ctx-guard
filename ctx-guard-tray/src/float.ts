import { listen } from '@tauri-apps/api/event';
import { Window } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';

const win = Window.getCurrent();

document.getElementById('app')!.addEventListener('mousedown', (e) => {
  const target = e.target as HTMLElement;
  if (target.id === 'close') return;
  win.startDragging();
});
document.getElementById('close')!.addEventListener('click', () => win.hide());

const PLAN_NAMES: Record<string,string> = {
  '1048576':'MAX 1M','524288':'MAX 500K','204800':'PRO 200K'
};

const canvas = document.getElementById('hg') as HTMLCanvasElement;
const ctx = canvas.getContext('2d')!;
const W = 64, H = 128;
const pctEl  = document.getElementById('pct')!;
const sizeEl = document.getElementById('size-lbl')!;
const planEl = document.getElementById('plan-lbl')!;

let maxBytes = 1048576;
let currentPct = 1;
let animFrame = 0;
let isFlashing = false;
let grains: Grain[] = [];

// 启动时读取已保存套餐
(async () => {
  try {
    const saved = await invoke<number>('get_max_bytes');
    if (saved > 0) { maxBytes = saved; planEl.textContent = PLAN_NAMES[String(maxBytes)] || 'CUSTOM'; }
  } catch(e) { console.error(e); }
})();

// ── 沙漏绘制
function getSandColor(pct: number): [number,number,number] {
  if (pct >= 0.5) {
    const t = (pct-0.5)/0.5;
    return [Math.round(234+(34-234)*t), Math.round(179+(197-179)*t), Math.round(8+(94-8)*t)];
  } else {
    const t = pct/0.5;
    return [220, Math.round(20*t), Math.round(20*t)];
  }
}
function c(rgb:[number,number,number], a=1){ return `rgba(${rgb[0]},${rgb[1]},${rgb[2]},${a})`; }
function lt(rgb:[number,number,number],n:number):[number,number,number]{ return [Math.min(255,rgb[0]+n),Math.min(255,rgb[1]+n),Math.min(255,rgb[2]+n)]; }
function dk(rgb:[number,number,number],n:number):[number,number,number]{ return [Math.max(0,rgb[0]-n),Math.max(0,rgb[1]-n),Math.max(0,rgb[2]-n)]; }

class Grain {
  x:number; y:number; vx:number; vy:number; r:number; life:number;
  constructor(cx:number, ny:number) {
    this.x=cx+(Math.random()-.5)*2.5; this.y=ny;
    this.vx=(Math.random()-.5)*.3; this.vy=.15+Math.random()*.15;
    this.r=Math.random()*.8+.4; this.life=1;
  }
  update(){ this.vy=Math.min(this.vy+.04,1.2); this.x+=this.vx; this.y+=this.vy; this.life-=.008; }
}

function drawHourglass(pct: number, t: number) {
  ctx.clearRect(0,0,W,H);
  const cx=W/2, padY=8, topY=padY, botY=H-padY, midY=H/2;
  const topW=W/2-6, neckW=2.5, cpR=0.82;

  function makeHgPath(){
    ctx.beginPath();
    ctx.moveTo(cx-topW,topY); ctx.lineTo(cx+topW,topY);
    ctx.bezierCurveTo(cx+topW,topY+(midY-topY)*cpR,cx+neckW,midY-(midY-topY)*(1-cpR),cx+neckW,midY);
    ctx.bezierCurveTo(cx+neckW,midY+(botY-midY)*(1-cpR),cx+topW,botY-(botY-midY)*cpR,cx+topW,botY);
    ctx.lineTo(cx-topW,botY);
    ctx.bezierCurveTo(cx-topW,botY-(botY-midY)*cpR,cx-neckW,midY+(botY-midY)*(1-cpR),cx-neckW,midY);
    ctx.bezierCurveTo(cx-neckW,midY-(midY-topY)*(1-cpR),cx-topW,topY+(midY-topY)*cpR,cx-topW,topY);
    ctx.closePath();
  }
  function makeTopHalf(){
    ctx.beginPath();
    ctx.moveTo(cx-topW,topY); ctx.lineTo(cx+topW,topY);
    ctx.bezierCurveTo(cx+topW,topY+(midY-topY)*cpR,cx+neckW,midY-(midY-topY)*(1-cpR),cx+neckW,midY);
    ctx.lineTo(cx-neckW,midY);
    ctx.bezierCurveTo(cx-neckW,midY-(midY-topY)*(1-cpR),cx-topW,topY+(midY-topY)*cpR,cx-topW,topY);
    ctx.closePath();
  }
  function makeBotHalf(){
    ctx.beginPath();
    ctx.moveTo(cx+neckW,midY);
    ctx.bezierCurveTo(cx+neckW,midY+(botY-midY)*(1-cpR),cx+topW,botY-(botY-midY)*cpR,cx+topW,botY);
    ctx.lineTo(cx-topW,botY);
    ctx.bezierCurveTo(cx-topW,botY-(botY-midY)*cpR,cx-neckW,midY+(botY-midY)*(1-cpR),cx-neckW,midY);
    ctx.closePath();
  }

  const sc = getSandColor(pct);

  // 顶底玻璃横条
  [[topY-5,5],[botY,5]].forEach(([y,h])=>{
    const bg=ctx.createLinearGradient(cx-topW,0,cx+topW,0);
    bg.addColorStop(0,'rgba(180,210,255,0.07)');
    bg.addColorStop(.5,'rgba(220,235,255,0.20)');
    bg.addColorStop(1,'rgba(180,210,255,0.07)');
    ctx.fillStyle=bg;
    ctx.beginPath(); ctx.roundRect(cx-topW,y,topW*2,h,2); ctx.fill();
    ctx.fillStyle='rgba(255,255,255,0.26)';
    ctx.beginPath(); ctx.roundRect(cx-topW+2,y+0.5,topW*2-4,1.5,1); ctx.fill();
  });

  // 玻璃背景
  ctx.save(); makeHgPath();
  const bg=ctx.createLinearGradient(0,0,W,0);
  bg.addColorStop(0,'rgba(40,80,140,0.18)'); bg.addColorStop(.35,'rgba(140,190,240,0.06)');
  bg.addColorStop(.65,'rgba(140,190,240,0.06)'); bg.addColorStop(1,'rgba(40,80,140,0.18)');
  ctx.fillStyle=bg; ctx.fill(); ctx.restore();

  // 上半沙子
  if(pct>0.005){
    ctx.save(); makeTopHalf(); ctx.clip();
    const aH=midY-topY, sY=midY-aH*pct, wa=1.2;
    const sg=ctx.createLinearGradient(0,sY,0,midY);
    sg.addColorStop(0,c(lt(sc,70),.70));
    sg.addColorStop(.2,c(lt(sc,30),.88));
    sg.addColorStop(.6,c(sc,.95));
    sg.addColorStop(1,c(dk(sc,70),.99));
    ctx.fillStyle=sg;
    ctx.beginPath();
    ctx.moveTo(0,midY); ctx.lineTo(W,midY);
    ctx.lineTo(W,sY+Math.sin(t*.04+1)*wa);
    ctx.bezierCurveTo(cx+topW*.4,sY+Math.sin(t*.05)*wa,cx-topW*.4,sY+Math.sin(t*.05+2)*wa,0,sY+Math.sin(t*.04+3)*wa);
    ctx.closePath(); ctx.fill();
    for(let i=0;i<22;i++){
      const gx=6+Math.random()*(W-12), gy=sY+3+Math.random()*(midY-sY-5), gs=Math.random()*1.3+.4;
      ctx.fillStyle=Math.random()>.5?'rgba(255,255,255,0.22)':'rgba(0,0,0,0.18)';
      ctx.beginPath();ctx.arc(gx,gy,gs,0,Math.PI*2);ctx.fill();
    }
    ctx.restore();
  }

  // 下半沙子
  if(pct<0.995){
    ctx.save(); makeBotHalf(); ctx.clip();
    const aH=botY-midY, sS=botY-aH*(1-pct);
    const sg2=ctx.createLinearGradient(0,sS,0,botY);
    sg2.addColorStop(0,c(lt(sc,50),.68));
    sg2.addColorStop(.25,c(lt(sc,20),.85));
    sg2.addColorStop(.65,c(sc,.93));
    sg2.addColorStop(1,c(dk(sc,75),.99));
    ctx.fillStyle=sg2; ctx.fillRect(0,sS,W,botY-sS);
    for(let i=0;i<18;i++){
      const gx=6+Math.random()*(W-12), gy=sS+3+Math.random()*(botY-sS-5), gs=Math.random()*1.2+.35;
      ctx.fillStyle=Math.random()>.5?'rgba(255,255,255,0.20)':'rgba(0,0,0,0.16)';
      ctx.beginPath();ctx.arc(gx,gy,gs,0,Math.PI*2);ctx.fill();
    }
    ctx.restore();
  }

  // 流动粒子
  ctx.save();
  grains.forEach(g=>{
    ctx.globalAlpha=Math.max(0,g.life)*.85;
    ctx.fillStyle=c(sc,1);
    ctx.beginPath();ctx.arc(g.x,g.y,g.r,0,Math.PI*2);ctx.fill();
  });
  ctx.globalAlpha=1; ctx.restore();

  // 玻璃描边
  ctx.save(); makeHgPath();
  const st=ctx.createLinearGradient(0,0,W,0);
  st.addColorStop(0,'rgba(120,170,230,0.35)'); st.addColorStop(.45,'rgba(220,240,255,0.80)');
  st.addColorStop(.55,'rgba(220,240,255,0.80)'); st.addColorStop(1,'rgba(120,170,230,0.35)');
  ctx.strokeStyle=st; ctx.lineWidth=1.3; ctx.stroke(); ctx.restore();

  // 左侧高光
  ctx.save(); makeHgPath(); ctx.clip();
  const hl=ctx.createLinearGradient(0,0,W*.42,0);
  hl.addColorStop(0,'rgba(255,255,255,0.55)');
  hl.addColorStop(.2,'rgba(255,255,255,0.25)');
  hl.addColorStop(.5,'rgba(255,255,255,0.06)');
  hl.addColorStop(1,'rgba(255,255,255,0)');
  ctx.fillStyle=hl; ctx.fillRect(0,0,W*.42,H); ctx.restore();

  // 右侧阴影
  ctx.save(); makeHgPath(); ctx.clip();
  const rl=ctx.createLinearGradient(W*.55,0,W,0);
  rl.addColorStop(0,'rgba(0,0,0,0)');
  rl.addColorStop(.6,'rgba(0,0,0,0.18)');
  rl.addColorStop(1,'rgba(0,0,0,0.38)');
  ctx.fillStyle=rl; ctx.fillRect(W*.55,0,W*.45,H); ctx.restore();

  // 顶部球形高光
  ctx.save(); makeHgPath(); ctx.clip();
  const th=ctx.createRadialGradient(cx-topW*.2,topY+4,0,cx,topY+12,topW*.9);
  th.addColorStop(0,'rgba(255,255,255,0.40)');
  th.addColorStop(.4,'rgba(255,255,255,0.08)');
  th.addColorStop(1,'rgba(255,255,255,0)');
  ctx.fillStyle=th; ctx.fillRect(0,topY,W,midY-topY); ctx.restore();

  // 底部内阴影
  ctx.save(); makeHgPath(); ctx.clip();
  const bs=ctx.createRadialGradient(cx,botY-4,2,cx,botY-8,topW*.85);
  bs.addColorStop(0,'rgba(0,0,0,0)');
  bs.addColorStop(.5,'rgba(0,0,0,0.08)');
  bs.addColorStop(1,'rgba(0,0,0,0.28)');
  ctx.fillStyle=bs; ctx.fillRect(0,midY,W,botY-midY); ctx.restore();

  // 30%以下红色外发光
  if(pct<0.3){
    const glow=(0.3-pct)/0.3*.7*(.5+.5*Math.sin(t*.07));
    ctx.save(); ctx.shadowColor='#ef4444'; ctx.shadowBlur=24;
    makeHgPath();
    ctx.strokeStyle=`rgba(220,20,20,${glow})`; ctx.lineWidth=2.5; ctx.stroke();
    ctx.restore();
  }

  // 10%以下整体闪烁
  if(isFlashing){
    const fa = 0.18*(0.5+0.5*Math.sin(t*.18));
    ctx.save(); makeHgPath();
    ctx.fillStyle=`rgba(239,68,68,${fa})`; ctx.fill(); ctx.restore();
  }
}

function animate(){
  animFrame++;
  const rate = currentPct<0.1?2:currentPct<0.3?3:5;
  if(currentPct>0.01 && animFrame%rate===0)
    grains.push(new Grain(W/2, H/2));
  grains = grains.filter(g=>g.life>0&&g.y<H);
  grains.forEach(g=>g.update());
  drawHourglass(currentPct, animFrame);
  requestAnimationFrame(animate);
}
animate();

// ── 事件监听
listen('float-update', (e:any)=>{
  const { sizeStr, sizeBytes, maxBytes: max } = e.payload;
  if(max) maxBytes = max;
  currentPct = Math.max(0, Math.min(1, 1 - sizeBytes/maxBytes));
  isFlashing = currentPct < 0.1;
  pctEl.textContent = (currentPct*100).toFixed(1)+'%';
  pctEl.style.color = currentPct<0.1?'#ef4444':currentPct<0.3?'#eab308':'#fff';
  sizeEl.textContent = sizeStr;
  planEl.textContent = PLAN_NAMES[String(maxBytes)]||'CUSTOM';
});

listen('plan-changed', (e:any)=>{
  maxBytes = e.payload;
  planEl.textContent = PLAN_NAMES[String(maxBytes)]||'CUSTOM';
});

listen('alert', ()=>{
  currentPct = 0; isFlashing = true;
  pctEl.textContent = '0%'; pctEl.style.color = '#ef4444';
  sizeEl.textContent = '⚠️ 已触发！';
});
