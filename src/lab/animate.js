/**
 * 引擎层 · 动画生成器
 * 输入 story 对象，输出「纯逻辑」三件套：
 *   JSON 场景图 + CSS 关键帧 + HTML5 页面（自包含、无外部依赖、可离线打开）
 * 设计原则：扁平化 / 平面化 —— 纯色块、无渐变、无阴影堆叠、只用几何形。
 */

import { MOODS } from './story.js';

const SCENE_SEC = 4.2;

function hexToRgb(hex) {
  const h = hex.replace('#', '');
  const full = h.length === 3 ? h.split('').map((c) => c + c).join('') : h;
  return [parseInt(full.slice(0, 2), 16), parseInt(full.slice(2, 4), 16), parseInt(full.slice(4, 6), 16)];
}

function shade(hex, ratio) {
  const [r, g, b] = hexToRgb(hex).map((v) => Math.round(Math.min(255, v * ratio)));
  return `#${[r, g, b].map((v) => v.toString(16).padStart(2, '0')).join('')}`;
}

function esc(s) {
  return String(s)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

/** 扁平化动物头像（纯 SVG 几何形，无插图素材） */
function animalSVG(key, color, name) {
  const dark = shade(color, 0.72);
  let ears = '';
  let face = '';
  let extras = '';

  if (key === 'fox') {
    ears = `<polygon points="34,50 42,8 70,40" fill="${color}"/>
            <polygon points="106,50 98,8 70,40" fill="${color}"/>
            <polygon points="42,44 46,20 62,40" fill="${dark}" opacity=".45"/>
            <polygon points="98,44 94,20 78,40" fill="${dark}" opacity=".45"/>`;
    face = `<ellipse cx="70" cy="88" rx="21" ry="13" fill="#fffdf8"/>
            <ellipse cx="70" cy="82" rx="4" ry="3" fill="#2b2b2b"/>`;
  } else if (key === 'owl') {
    ears = `<polygon points="38,46 50,12 64,38" fill="${color}"/>
            <polygon points="102,46 90,12 76,38" fill="${color}"/>`;
    face = `<polygon points="70,88 62,100 78,100" fill="#ffb703"/>`;
    extras = `<circle cx="54" cy="68" r="12" fill="#fffdf8"/><circle cx="86" cy="68" r="12" fill="#fffdf8"/>
              <circle cx="54" cy="68" r="5" fill="#20232a"/><circle cx="86" cy="68" r="5" fill="#20232a"/>`;
  } else if (key === 'rabbit') {
    ears = `<ellipse cx="52" cy="20" rx="9" ry="27" transform="rotate(-10 52 47)" fill="${color}"/>
            <ellipse cx="88" cy="20" rx="9" ry="27" transform="rotate(10 88 47)" fill="${color}"/>
            <ellipse cx="52" cy="22" rx="4" ry="17" transform="rotate(-10 52 47)" fill="#ffd7e0"/>
            <ellipse cx="88" cy="22" rx="4" ry="17" transform="rotate(10 88 47)" fill="#ffd7e0"/>`;
  } else if (key === 'bear') {
    ears = `<circle cx="40" cy="44" r="14" fill="${color}"/><circle cx="100" cy="44" r="14" fill="${color}"/>
            <circle cx="40" cy="44" r="7" fill="${dark}" opacity=".35"/>
            <circle cx="100" cy="44" r="7" fill="${dark}" opacity=".35"/>`;
    face = `<ellipse cx="70" cy="90" rx="24" ry="15" fill="#f3e6d3"/>`;
  } else {
    ears = `<polygon points="36,52 44,14 72,40" fill="${color}"/><polygon points="104,52 96,14 68,40" fill="${color}"/>
            <polygon points="44,46 48,24 64,40" fill="#ffc2d1"/>
            <polygon points="96,46 92,24 76,40" fill="#ffc2d1"/>`;
    extras = `<line x1="34" y1="86" x2="14" y2="80" stroke="${dark}" stroke-width="2.4" stroke-linecap="round"/>
              <line x1="34" y1="92" x2="14" y2="94" stroke="${dark}" stroke-width="2.4" stroke-linecap="round"/>
              <line x1="106" y1="86" x2="126" y2="80" stroke="${dark}" stroke-width="2.4" stroke-linecap="round"/>
              <line x1="106" y1="92" x2="126" y2="94" stroke="${dark}" stroke-width="2.4" stroke-linecap="round"/>`;
  }

  const eyes =
    key === 'owl'
      ? ''
      : `<g class="eyes">
           <circle cx="55" cy="66" r="4.6" fill="#20232a"/>
           <circle cx="85" cy="66" r="4.6" fill="#20232a"/>
         </g>`;

  const blush =
    key === 'owl'
      ? ''
      : `<ellipse cx="42" cy="86" rx="7" ry="4" fill="#ff8fa3" opacity=".45"/>
         <ellipse cx="98" cy="86" rx="7" ry="4" fill="#ff8fa3" opacity=".45"/>`;

  return `<svg class="animal" viewBox="0 0 140 150" role="img" aria-label="${esc(name)}">
    <ellipse cx="70" cy="136" rx="42" ry="10" fill="${dark}" opacity=".14"/>
    <ellipse cx="70" cy="120" rx="31" ry="20" fill="${color}"/>
    ${ears}
    <circle cx="70" cy="72" r="38" fill="${color}"/>
    ${face}
    ${eyes}
    ${blush}
    ${extras}
  </svg>`;
}

/** 生成 CSS（扁平化样式 + 逐幕关键帧） */
export function buildAnimationCSS(story) {
  const n = story.scenes.length;
  const total = (n * SCENE_SEC).toFixed(1);
  const frames = story.scenes
    .map((_, i) => {
      const start = (i / n) * 100;
      const end = ((i + 1) / n) * 100;
      const span = end - start;
      const fade = span * 0.16;
      const p = (v) => Math.max(0, Math.min(100, v)).toFixed(2);
      return `@keyframes sc${i}{
  0%, ${p(start <= 0 ? 0 : start - 0.01)}% { opacity: 0; transform: translateY(16px) scale(.985); }
  ${p(start + fade)}% { opacity: 1; transform: translateY(0) scale(1); }
  ${p(end - fade)}% { opacity: 1; transform: translateY(0) scale(1); }
  ${p(end)}%, 100% { opacity: 0; transform: translateY(-14px) scale(.985); }
}`;
    })
    .join('\n');

  return `:root {
  --total: ${total}s;
  --ink: #2c3038;
  --paper: #ffffff;
  --radius: 18px;
  --font: "HarmonyOS Sans SC", "PingFang SC", "Microsoft YaHei", "Noto Sans SC", system-ui, sans-serif;
}
* { box-sizing: border-box; }
html, body { height: 100%; }
body {
  margin: 0;
  font-family: var(--font);
  color: var(--ink);
  background: #f5f5f7;
  -webkit-font-smoothing: antialiased;
}
.stage {
  position: relative;
  width: 100%;
  height: 100%;
  min-height: 420px;
  overflow: hidden;
  background: #f5f5f7;
}
.scene {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 26px;
  padding: 76px 8vw 92px;
  opacity: 0;
  background: var(--bg);
  color: var(--ink);
  animation-duration: var(--total);
  animation-timing-function: linear;
  animation-iteration-count: infinite;
  animation-fill-mode: both;
}
.deco {
  position: absolute;
  border-radius: 50%;
  background: var(--accent);
  opacity: .12;
  z-index: 0;
  animation: float 7s ease-in-out infinite;
}
.deco:nth-child(1) { width: 220px; height: 220px; top: -70px; left: -60px; }
.deco:nth-child(2) { width: 140px; height: 140px; bottom: 40px; right: -40px; animation-delay: -2.4s; }
.deco:nth-child(3) { width: 56px; height: 56px; border-radius: 12px; transform: rotate(18deg); top: 26%; right: 12%; background: var(--ink); opacity: .06; animation: none; }
.animal-wrap { position: relative; z-index: 2; animation: bob 2.8s ease-in-out infinite; }
.animal { width: 190px; height: auto; display: block; }
.animal .eyes { transform-box: fill-box; transform-origin: center; animation: blink 5.4s infinite; }
.bubble {
  position: relative;
  z-index: 2;
  max-width: 720px;
  padding: 26px 30px 24px;
  background: var(--paper);
  border-left: 8px solid var(--accent);
  border-radius: 14px;
  box-shadow: 12px 12px 0 0 rgba(0, 0, 0, .06);
}
.bubble::before {
  content: "";
  position: absolute;
  left: 50%;
  top: -12px;
  width: 24px;
  height: 24px;
  background: var(--paper);
  border-radius: 4px;
  transform: translateX(-50%) rotate(45deg);
}
.bubble p {
  margin: 0;
  font-size: 21px;
  line-height: 1.75;
  letter-spacing: .02em;
  word-break: break-word;
}
.say {
  display: block;
  margin-top: 12px;
  font-size: 13px;
  letter-spacing: .12em;
  color: var(--accent);
  font-weight: 600;
}
.hud {
  position: absolute;
  left: 0;
  right: 0;
  top: 0;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 20px 28px;
  font-size: 12px;
  letter-spacing: .12em;
  z-index: 5;
  pointer-events: none;
  mix-blend-mode: normal;
}
.chip {
  padding: 7px 13px;
  border-radius: 999px;
  background: rgba(255, 255, 255, .72);
  border: 1.5px solid rgba(0, 0, 0, .1);
  font-weight: 600;
  color: #2c3038;
}
.caption {
  position: absolute;
  left: 0;
  right: 0;
  bottom: 26px;
  text-align: center;
  font-size: 12px;
  letter-spacing: .1em;
  color: rgba(0, 0, 0, .45);
  z-index: 5;
  pointer-events: none;
}
.progress {
  position: absolute;
  left: 0;
  right: 0;
  bottom: 0;
  height: 5px;
  background: rgba(0, 0, 0, .06);
  z-index: 6;
}
.progress span {
  display: block;
  height: 100%;
  width: 0;
  animation: progress var(--total) linear infinite;
}
.badge {
  position: absolute;
  right: 28px;
  bottom: 64px;
  font-size: 12px;
  letter-spacing: .16em;
  color: rgba(0, 0, 0, .5);
  z-index: 5;
}
@keyframes bob { 0%, 100% { transform: translateY(0); } 50% { transform: translateY(-10px); } }
@keyframes blink { 0%, 92%, 100% { transform: scaleY(1); } 95% { transform: scaleY(.12); } }
@keyframes float { 0%, 100% { transform: translateY(0); } 50% { transform: translateY(-14px); } }
@keyframes progress { from { width: 0; } to { width: 100%; } }
${frames}
@media (max-width: 640px) {
  .animal { width: 136px; }
  .bubble { padding: 20px 20px 18px; }
  .bubble p { font-size: 17px; }
  .scene { gap: 18px; padding: 64px 20px 84px; }
  .hud { padding: 14px 16px; }
  .chip { font-size: 11px; }
  .badge { right: 16px; bottom: 56px; }
}
@media (prefers-reduced-motion: reduce) {
  .scene { animation-duration: var(--total); }
  .animal-wrap, .deco, .animal .eyes { animation: none; }
}
`;
}

/** 生成自包含 HTML5 动画页 */
export function buildAnimationHTML(story, options = {}) {
  const { cta = false, baseUrl = '', still = false, sceneIndex = 0 } = options;
  const css = buildAnimationCSS(story);
  const n = story.scenes.length;
  // 静态定格：用于截图、打印或无障碍阅读
  const stillCSS = still
    ? `.scene{animation:none!important;opacity:0!important}.scene[data-scene="${Math.max(0, Math.min(n - 1, sceneIndex))}"]{opacity:1!important}.progress span{animation:none!important;width:100%!important}`
    : '';

  const scenes = story.scenes
    .map((scene, i) => {
      const animal = story.cast.find((c) => c.id === scene.animal) || story.cast[0];
      return `<section class="scene" data-scene="${i}" style="--bg:${scene.palette.bg};--ink:${scene.palette.ink};--accent:${scene.palette.accent};--animal:${scene.palette.animalColor};animation-name:sc${i}">
  <div class="deco"></div>
  <div class="deco"></div>
  <div class="deco"></div>
  <div class="animal-wrap">${animalSVG(animal.id, animal.color, animal.name)}</div>
  <div class="bubble">
    <p>${esc(scene.text)}</p>
    <span class="say">— ${esc(animal.name)} · ${esc(MOODS[scene.mood]?.name || '平静')}</span>
  </div>
  <div class="badge">${String(i + 1).padStart(2, '0')} / ${String(n).padStart(2, '0')}</div>
</section>`;
    })
    .join('\n');

  const ctaHTML = cta
    ? `<a class="chip cta" href="${esc(baseUrl)}/s4" target="_top" style="pointer-events:auto;text-decoration:none">申请 IP 体验权证书 →</a>`
    : '<span class="chip">TXT · JSON · CSS · HTML5</span>';

  return `<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${esc(story.title)} · Wayhug 动画</title>
<style>
${css}
${stillCSS}
</style>
</head>
<body>
<div class="stage">
  <div class="hud">
    <span class="chip">WAYHUG · TXT → 动画</span>
    ${ctaHTML}
  </div>
  ${scenes}
  <div class="caption">《${esc(story.title)}》 · ${n} 幕 · 指纹 ${esc(story.fingerprint.slice(0, 8))}</div>
  <div class="progress"><span style="background:${esc(story.scenes[0]?.palette.accent || '#708090')}"></span></div>
</div>
</body>
</html>
`;
}

/** 三件套打包 */
export function buildBundle(story, options = {}) {
  return {
    html: buildAnimationHTML(story, options),
    css: buildAnimationCSS(story),
    scenes: story.scenes.length,
    duration: Number((story.scenes.length * SCENE_SEC).toFixed(1)),
  };
}
