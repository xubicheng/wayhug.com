// ============================================================
// 引擎层 · 动画生成器（纯逻辑三件套：JSON / CSS / HTML5）
//
// 扁平化原则：纯色块、无渐变、无位图、只用几何形。
// 每一幕带四相坐标，颜色遵守「四相底色 + 情绪重音色」。
// ============================================================

use crate::model::Story;
use crate::story::mood;
use crate::worldview::{Element, Phase};

pub const SCENE_SEC: f32 = 4.2;

fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
    let h = hex.trim_start_matches('#');
    let full = if h.len() == 3 {
        h.chars().flat_map(|c| [c, c]).collect::<String>()
    } else {
        h.to_string()
    };
    let r = u8::from_str_radix(&full[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&full[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&full[4..6], 16).unwrap_or(0);
    (r, g, b)
}

fn shade(hex: &str, ratio: f32) -> String {
    let (r, g, b) = hex_to_rgb(hex);
    let f = |v: u8| ((v as f32 * ratio).round().clamp(0.0, 255.0)) as u8;
    format!("#{:02x}{:02x}{:02x}", f(r), f(g), f(b))
}

pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// 扁平化动物头像（纯 SVG 几何形）
pub fn animal_svg(key: &str, color: &str, name: &str) -> String {
    let dark = shade(color, 0.72);
    let (ears, face, extras) = match key {
        "fox" => (
            format!(
                r##"<polygon points="34,50 42,8 70,40" fill="{c}"/>
                   <polygon points="106,50 98,8 70,40" fill="{c}"/>
                   <polygon points="42,44 46,20 62,40" fill="{d}" opacity=".45"/>
                   <polygon points="98,44 94,20 78,40" fill="{d}" opacity=".45"/>"##,
                c = color,
                d = dark
            ),
            r##"<ellipse cx="70" cy="88" rx="21" ry="13" fill="#fffdf8"/>
               <ellipse cx="70" cy="82" rx="4" ry="3" fill="#2b2b2b"/>"##
                .to_string(),
            String::new(),
        ),
        "owl" => (
            format!(
                r##"<polygon points="38,46 50,12 64,38" fill="{c}"/>
                   <polygon points="102,46 90,12 76,38" fill="{c}"/>"##,
                c = color
            ),
            r##"<polygon points="70,88 62,100 78,100" fill="#ffb703"/>"##.to_string(),
            r##"<circle cx="54" cy="68" r="12" fill="#fffdf8"/><circle cx="86" cy="68" r="12" fill="#fffdf8"/>
               <circle cx="54" cy="68" r="5" fill="#20232a"/><circle cx="86" cy="68" r="5" fill="#20232a"/>"##
                .to_string(),
        ),
        "rabbit" => (
            format!(
                r##"<ellipse cx="52" cy="20" rx="9" ry="27" transform="rotate(-10 52 47)" fill="{c}"/>
                   <ellipse cx="88" cy="20" rx="9" ry="27" transform="rotate(10 88 47)" fill="{c}"/>
                   <ellipse cx="52" cy="22" rx="4" ry="17" transform="rotate(-10 52 47)" fill="#ffd7e0"/>
                   <ellipse cx="88" cy="22" rx="4" ry="17" transform="rotate(10 88 47)" fill="#ffd7e0"/>"##,
                c = color
            ),
            String::new(),
            String::new(),
        ),
        "bear" => (
            format!(
                r##"<circle cx="40" cy="44" r="14" fill="{c}"/><circle cx="100" cy="44" r="14" fill="{c}"/>
                   <circle cx="40" cy="44" r="7" fill="{d}" opacity=".35"/>
                   <circle cx="100" cy="44" r="7" fill="{d}" opacity=".35"/>"##,
                c = color,
                d = dark
            ),
            r##"<ellipse cx="70" cy="90" rx="24" ry="15" fill="#f3e6d3"/>"##.to_string(),
            String::new(),
        ),
        _ => (
            format!(
                r##"<polygon points="36,52 44,14 72,40" fill="{c}"/><polygon points="104,52 96,14 68,40" fill="{c}"/>
                   <polygon points="44,46 48,24 64,40" fill="#ffc2d1"/>
                   <polygon points="96,46 92,24 76,40" fill="#ffc2d1"/>"##,
                c = color
            ),
            String::new(),
            format!(
                r##"<line x1="34" y1="86" x2="14" y2="80" stroke="{d}" stroke-width="2.4" stroke-linecap="round"/>
                   <line x1="34" y1="92" x2="14" y2="94" stroke="{d}" stroke-width="2.4" stroke-linecap="round"/>
                   <line x1="106" y1="86" x2="126" y2="80" stroke="{d}" stroke-width="2.4" stroke-linecap="round"/>
                   <line x1="106" y1="92" x2="126" y2="94" stroke="{d}" stroke-width="2.4" stroke-linecap="round"/>"##,
                d = dark
            ),
        ),
    };

    let eyes = if key == "owl" {
        String::new()
    } else {
        r##"<g class="eyes">
             <circle cx="55" cy="66" r="4.6" fill="#20232a"/>
             <circle cx="85" cy="66" r="4.6" fill="#20232a"/>
           </g>"##
            .to_string()
    };
    let blush = if key == "owl" {
        String::new()
    } else {
        r##"<ellipse cx="42" cy="86" rx="7" ry="4" fill="#ff8fa3" opacity=".45"/>
           <ellipse cx="98" cy="86" rx="7" ry="4" fill="#ff8fa3" opacity=".45"/>"##
            .to_string()
    };

    format!(
        r##"<svg class="animal" viewBox="0 0 140 150" role="img" aria-label="{name}">
  <ellipse cx="70" cy="136" rx="42" ry="10" fill="{dark}" opacity=".14"/>
  <ellipse cx="70" cy="120" rx="31" ry="20" fill="{color}"/>
  {ears}
  <circle cx="70" cy="72" r="38" fill="{color}"/>
  {face}
  {eyes}
  {blush}
  {extras}
</svg>"##,
        name = esc(name),
        dark = dark,
        color = color,
        ears = ears,
        face = face,
        eyes = eyes,
        blush = blush,
        extras = extras
    )
}

/// 生成 CSS（扁平化样式 + 逐幕关键帧）
pub fn build_css(story: &Story) -> String {
    let n = story.scenes.len().max(1) as f32;
    let total = (n * SCENE_SEC).to_string();
    let frames = (0..story.scenes.len())
        .map(|i| {
            let start = (i as f32 / n) * 100.0;
            let end = ((i + 1) as f32 / n) * 100.0;
            let span = end - start;
            let fade = span * 0.16;
            let p = |v: f32| v.clamp(0.0, 100.0);
            format!(
                "@keyframes sc{i}{{
  0%, {a}% {{ opacity: 0; transform: translateY(16px) scale(.985); }}
  {b}% {{ opacity: 1; transform: translateY(0) scale(1); }}
  {c}% {{ opacity: 1; transform: translateY(0) scale(1); }}
  {d}%, 100% {{ opacity: 0; transform: translateY(-14px) scale(.985); }}
}}",
                i = i,
                a = format!("{:.2}", p((start - 0.01).max(0.0))),
                b = format!("{:.2}", p(start + fade)),
                c = format!("{:.2}", p(end - fade)),
                d = format!("{:.2}", p(end)),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r##":root {{
  --total: {total}s;
  --ink: #2c3038;
  --paper: #ffffff;
  --radius: 18px;
  --font: "HarmonyOS Sans SC", "PingFang SC", "Microsoft YaHei", "Noto Sans SC", system-ui, sans-serif;
}}
* {{ box-sizing: border-box; }}
html, body {{ height: 100%; }}
body {{
  margin: 0;
  font-family: var(--font);
  color: var(--ink);
  background: #f5f5f7;
  -webkit-font-smoothing: antialiased;
}}
.stage {{
  position: relative;
  width: 100%;
  height: 100%;
  min-height: 420px;
  overflow: hidden;
  background: #f5f5f7;
}}
.scene {{
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
}}
.deco {{
  position: absolute;
  border-radius: 50%;
  background: var(--accent);
  opacity: .12;
  z-index: 0;
  animation: float 7s ease-in-out infinite;
}}
.deco:nth-child(1) {{ width: 220px; height: 220px; top: -70px; left: -60px; }}
.deco:nth-child(2) {{ width: 140px; height: 140px; bottom: 40px; right: -40px; animation-delay: -2.4s; }}
.deco:nth-child(3) {{ width: 56px; height: 56px; border-radius: 12px; transform: rotate(18deg); top: 26%; right: 12%; background: var(--ink); opacity: .06; animation: none; }}
.animal-wrap {{ position: relative; z-index: 2; animation: bob 2.8s ease-in-out infinite; }}
.animal {{ width: 190px; height: auto; display: block; }}
.animal .eyes {{ transform-box: fill-box; transform-origin: center; animation: blink 5.4s infinite; }}
.bubble {{
  position: relative;
  z-index: 2;
  max-width: 720px;
  padding: 26px 30px 24px;
  background: var(--paper);
  border-left: 8px solid var(--accent);
  border-radius: 14px;
  box-shadow: 12px 12px 0 0 rgba(0, 0, 0, .06);
}}
.bubble::before {{
  content: "";
  position: absolute;
  left: 50%;
  top: -12px;
  width: 24px;
  height: 24px;
  background: var(--paper);
  border-radius: 4px;
  transform: translateX(-50%) rotate(45deg);
}}
.bubble p {{
  margin: 0;
  font-size: 21px;
  line-height: 1.75;
  letter-spacing: .02em;
  word-break: break-word;
}}
.say {{
  display: block;
  margin-top: 12px;
  font-size: 13px;
  letter-spacing: .12em;
  color: var(--accent);
  font-weight: 600;
}}
.hud {{
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
}}
.chip {{
  padding: 7px 13px;
  border-radius: 999px;
  background: rgba(255, 255, 255, .72);
  border: 1.5px solid rgba(0, 0, 0, .1);
  font-weight: 600;
  color: #2c3038;
}}
.cta {{ pointer-events: auto; text-decoration: none; }}
.caption {{
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
}}
.progress {{
  position: absolute;
  left: 0;
  right: 0;
  bottom: 0;
  height: 5px;
  background: rgba(0, 0, 0, .06);
  z-index: 6;
}}
.progress span {{
  display: block;
  height: 100%;
  width: 0;
  animation: progress var(--total) linear infinite;
}}
.badge {{
  position: absolute;
  right: 28px;
  bottom: 64px;
  font-size: 12px;
  letter-spacing: .16em;
  color: rgba(0, 0, 0, .5);
  z-index: 5;
}}
@keyframes bob {{ 0%, 100% {{ transform: translateY(0); }} 50% {{ transform: translateY(-10px); }} }}
@keyframes blink {{ 0%, 92%, 100% {{ transform: scaleY(1); }} 95% {{ transform: scaleY(.12); }} }}
@keyframes float {{ 0%, 100% {{ transform: translateY(0); }} 50% {{ transform: translateY(-14px); }} }}
@keyframes progress {{ from {{ width: 0; }} to {{ width: 100%; }} }}
{frames}
@media (max-width: 640px) {{
  .animal {{ width: 136px; }}
  .bubble {{ padding: 20px 20px 18px; max-width: 100%; }}
  .bubble p {{ font-size: 17px; }}
  .scene {{ gap: 18px; padding: 64px 20px 84px; }}
  .hud {{ padding: 14px 16px; }}
  .chip {{ font-size: 11px; }}
  .badge {{ right: 16px; bottom: 56px; }}
}}
@media (prefers-reduced-motion: reduce) {{
  .animal-wrap, .deco, .animal .eyes {{ animation: none; }}
}}
"##,
        total = total,
        frames = frames
    )
}

pub struct HtmlOptions<'a> {
    pub cta_url: Option<&'a str>,
    pub still: bool,
    pub scene_index: usize,
}

impl<'a> Default for HtmlOptions<'a> {
    fn default() -> Self {
        Self { cta_url: None, still: false, scene_index: 0 }
    }
}

/// 生成自包含 HTML5 动画页
pub fn build_html(story: &Story, opt: HtmlOptions<'_>) -> String {
    let css = build_css(story);
    let n = story.scenes.len();
    let still_css = if opt.still {
        format!(
            ".scene{{animation:none!important;opacity:0!important}}.scene[data-scene=\"{}\"]{{opacity:1!important}}.progress span{{animation:none!important;width:100%!important}}",
            opt.scene_index.min(n.saturating_sub(1))
        )
    } else {
        String::new()
    };

    let scenes = story
        .scenes
        .iter()
        .map(|s| {
            let a = story.cast.iter().find(|c| c.id == s.animal).unwrap_or(&story.cast[0]);
            let pname = Phase::from_key(&s.phase).map(|p| p.name()).unwrap_or("晨");
            let ename = Element::from_key(&s.element).map(|e| e.name()).unwrap_or("风");
            let mood_name = mood(&s.mood).name;
            format!(
                r##"<section class="scene" data-scene="{i}" data-phase="{phase}" data-element="{element}" style="--bg:{bg};--ink:{ink};--accent:{accent};--animal:{animal_color};animation-name:sc{i}">
  <div class="deco"></div>
  <div class="deco"></div>
  <div class="deco"></div>
  <div class="animal-wrap">{svg}</div>
  <div class="bubble">
    <p>{text}</p>
    <span class="say">— {aname} · {mood_name} · {pname}/{ename}</span>
  </div>
  <div class="badge">{idx} / {total}</div>
</section>"##,
                i = s.index,
                phase = esc(&s.phase),
                element = esc(&s.element),
                bg = esc(&s.palette.bg),
                ink = esc(&s.palette.ink),
                accent = esc(&s.palette.accent),
                animal_color = esc(&s.palette.animal_color),
                svg = animal_svg(&a.id, &a.color, &a.name),
                text = esc(&s.text),
                aname = esc(&a.name),
                mood_name = esc(mood_name),
                pname = pname,
                ename = ename,
                idx = format!("{:02}", s.index + 1),
                total = format!("{:02}", n),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let cta = match opt.cta_url {
        Some(url) => format!(
            r##"<a class="chip cta" href="{}" target="_top">领取 IP 体验权 →</a>"##,
            esc(url)
        ),
        None => r##"<span class="chip">TXT · JSON · CSS · HTML5</span>"##.to_string(),
    };

    let accent = story.scenes.first().map(|s| s.palette.accent.as_str()).unwrap_or("#708090");

    format!(
        r##"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title} · Wayhug 动画</title>
<style>
{css}
{still}
</style>
</head>
<body>
<div class="stage">
  <div class="hud">
    <span class="chip">WAYHUG · TXT → 动画</span>
    {cta}
  </div>
  {scenes}
  <div class="caption">《{title}》 · {n} 幕 · 四相四属性 · 指纹 {fp}</div>
  <div class="progress"><span style="background:{accent}"></span></div>
</div>
</body>
</html>
"##,
        title = esc(&story.title),
        css = css,
        still = still_css,
        cta = cta,
        scenes = scenes,
        n = n,
        fp = esc(&story.fingerprint[..8.min(story.fingerprint.len())]),
        accent = esc(accent)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::story::{demo_text, parse_story};

    #[test]
    fn bundle_is_flat_and_complete() {
        let s = parse_story(demo_text(), None, Some("demo")).unwrap();
        let css = build_css(&s);
        let html = build_html(&s, HtmlOptions::default());
        assert!(css.contains("@keyframes sc0"));
        assert!(!css.contains("gradient"));
        assert!(html.contains("<!DOCTYPE html>"));
        assert_eq!(html.matches("class=\"scene\"").count(), 8);
        // 只有动画：不允许音频元素或自动播放
        assert!(!html.contains("<audio"));
        assert!(!html.contains("autoplay"));
    }
}
