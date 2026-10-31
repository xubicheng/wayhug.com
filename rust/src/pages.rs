// ============================================================
// 交付层 · 页面（干净、简单的公众界面；复杂度留在后台）
//
// 公网只有一页：看 → 写 → 领 → 寄。
// 四个方案（展厅 / 邮件 / 生成器 / 体验权）合并为同一条动线，
// 后台工作台单独在 /studio，不进入公众导航。
// ============================================================

use crate::animate::esc;
use crate::config::Config;
use crate::model::{Request, Story};
use crate::worldview;

const CSS: &str = include_str!("../assets/site.css");
const APP_JS: &str = include_str!("../assets/app.js");
const STUDIO_JS: &str = include_str!("../assets/studio.js");

/// 浏览器标签页图标：王元会定制 PNG（编译期内嵌为 data URI）
pub const FAVICON_PNG: &[u8] = include_bytes!("../assets/favicon.png");
pub const BRAND_LOGO_PNG: &[u8] = include_bytes!("../assets/brand_logo.png");

/// 公安备案徽章（PNG，编译期内嵌）
const BEIAN_BADGE_PNG: &[u8] = include_bytes!("../assets/beian_badge.png");
/// 王元会个人商标（PNG，编译期内嵌）
const TRADEMARK_PNG: &[u8] = include_bytes!("../assets/trademark.png");

/// 简易 base64 编码（用于把内嵌 PNG 转成 data URI，避免引入额外依赖）
fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).map(|&b| b as u32).unwrap_or(0);
        let b2 = chunk.get(2).map(|&b| b as u32).unwrap_or(0);
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3f) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((n >> 6) & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(n & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn shell(title: &str, description: &str, body: &str, js: &str, extra_js: &str) -> String {
    format!(
        r##"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, maximum-scale=2, user-scalable=yes">
<title>{title}</title>
<meta name="description" content="{desc}">
<meta name="keywords" content="王元会, 动画生成, TXT转动画, 故事动画, 文字动画, wayhug, 动物动画, IP创作">
<meta name="author" content="王元会">
<meta name="robots" content="index, follow, max-image-preview:large">
<meta name="theme-color" content="#5d8aa8">
<link rel="canonical" href="https://wayhug.com/">
<link rel="icon" type="image/png" href="{favicon}">
<link rel="apple-touch-icon" href="{favicon}">
<meta property="og:type" content="website">
<meta property="og:site_name" content="王元会">
<meta property="og:title" content="{title}">
<meta property="og:description" content="{desc}">
<meta property="og:url" content="https://wayhug.com/">
<meta property="og:locale" content="zh_CN">
<meta name="twitter:card" content="summary_large_image">
<meta name="twitter:title" content="{title}">
<meta name="twitter:description" content="{desc}">
<script type="application/ld+json">{jsonld}</script>
<style>{css}</style>
</head>
<body>
{body}
<script>{js}</script>
{extra_js}
</body>
</html>
"##,
        title = esc(title),
        desc = esc(description),
        favicon = format!("data:image/png;base64,{}", base64_encode(FAVICON_PNG)),
        jsonld = r#"{"@context":"https://schema.org","@type":"WebSite","name":"王元会","alternateName":"WAYHUG","url":"https://wayhug.com/","description":"把一段文字变成一部会呼吸的动画。王元会的文字到动画 IP 实验场。","inLanguage":"zh-CN","author":{"@type":"Person","name":"王元会"}}"#,
        css = CSS,
        body = body,
        js = js,
        extra_js = extra_js
    )
}

fn header() -> &'static str {
    r##"<header class="top">
  <div class="wrap">
    <a class="brand" href="/"><img class="brand-logo" src="/brand.png" alt="WYH" width="26" height="26">王元会</a>
    <nav class="nav">
      <a href="#watch">看</a>
      <a href="#create">写</a>
      <a href="#experience">领</a>
      <a href="#mail">寄</a>
      <a href="#world">四相</a>
    </nav>
  </div>
</header>
<nav class="nav-mobile" aria-label="移动端导航">
  <ul>
    <li><a href="#watch"><span class="tab-num">01</span>看</a></li>
    <li><a href="#create"><span class="tab-num">02</span>写</a></li>
    <li><a href="#experience"><span class="tab-num">03</span>领</a></li>
    <li><a href="#mail"><span class="tab-num">04</span>寄</a></li>
    <li><a href="#world"><span class="tab-num">05</span>四相</a></li>
  </ul>
</nav>
<div class="orient-tip" id="orient-tip">横屏观看动画效果更佳</div>"##
}

fn quadrant_cards() -> String {
    let realms = worldview::public_realms()["realms"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    realms
        .iter()
        .map(|r| {
            let n = r["n"].as_u64().unwrap_or(1);
            let accent = esc(r["color"].as_str().unwrap_or("#ff6633"));
            format!(
                r##"<article class="quad">
  <div class="corner">{n:02} / 04</div>
  <div class="bar" style="background:{accent}"></div>
  <div class="realm-head"><img class="realm-art" src="/assets/realm/{key}.svg" width="56" height="56" alt="{animal}" loading="lazy"><h3>{name} · {subtitle}</h3></div>
  <div class="attr"><i class="swatch" style="background:{accent}"></i>{animal}</div>
  <p>{character}；主色 {accent}。</p>
</article>"##,
                n = n,
                key = esc(r["realm"].as_str().unwrap_or("yuan")),
                name = esc(r["realmName"].as_str().unwrap_or("原")),
                subtitle = esc(r["subtitle"].as_str().unwrap_or("自然界")),
                animal = esc(r["animalName"].as_str().unwrap_or("狐狸")),
                character = esc(r["character"].as_str().unwrap_or("容易害怕")),
                accent = accent
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 文稿校准条：四相占比完全来自已收录 TXT 的逐幕统计
fn corpus_block(c: &serde_json::Value) -> String {
    let phases = c["realms"]
        .as_array()
        .or_else(|| c["phases"].as_array())
        .cloned()
        .unwrap_or_default();
    if phases.is_empty() {
        return String::new();
    }
    let segs = phases
        .iter()
        .map(|p| {
            let share = p["share"].as_u64().unwrap_or(0);
            let empty = share == 0;
            format!(
                r##"<i style="flex:{flex};background:{accent}" title="{beat}"></i>"##,
                flex = share.max(4),
                accent = if empty { "#dcdfe3".to_string() } else { esc(p["color"].as_str().or_else(|| p["accent"].as_str()).unwrap_or("#e8a33d")) },
                beat = esc(p["animalName"].as_str().or_else(|| p["beat"].as_str()).unwrap_or(""))
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let legend = phases
        .iter()
        .map(|p| {
            let share = p["share"].as_u64().unwrap_or(0);
            format!(
                r##"<span><i style="background:{accent}"></i>{name} {n} 幕 · {share}%</span>"##,
                accent = if share == 0 { "#dcdfe3".to_string() } else { esc(p["color"].as_str().or_else(|| p["accent"].as_str()).unwrap_or("#e8a33d")) },
                name = esc(p["realmName"].as_str().or_else(|| p["phaseName"].as_str()).unwrap_or("")),
                n = p["scenes"].as_u64().unwrap_or(0),
                share = share
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let latest = c["latest"].as_str().unwrap_or("");
    let latest_part = if latest.is_empty() {
        String::new()
    } else {
        format!(" · 最新《{}》", esc(latest))
    };
    format!(
        r##"<div class="corpus">
  <div class="corpus-head">
    <span class="latest-label">文稿校准</span>
    <span class="corpus-meta">{manuscripts} 篇 · {scenes} 幕 · {chars} 字符{latest}</span>
  </div>
  <div class="corpus-bars">{segs}</div>
  <div class="corpus-legend">{legend}</div>
  <p class="corpus-note">四相占比由 TXT 文稿逐幕统计；导入新文稿后自动重算，不写死。</p>
</div>"##,
        manuscripts = c["manuscripts"].as_u64().unwrap_or(0),
        scenes = c["scenes"].as_u64().unwrap_or(0),
        chars = c["chars"].as_u64().unwrap_or(0),
        latest = latest_part,
        segs = segs,
        legend = legend
    )
}

pub struct Landing<'a> {
    pub cfg: &'a Config,
    pub demo: &'a Story,
    pub latest: &'a [Story],
    pub corpus: &'a serde_json::Value,
    pub stories: usize,
    pub requests: usize,
    pub sketches: usize,
}

pub fn landing(l: Landing<'_>) -> String {
    let cfg = l.cfg;
    let demo = l.demo;
    let cast_chips = demo
        .cast
        .iter()
        .map(|c| format!(r##"<span class="chip"><strong>{}</strong> {} 次</span>"##, esc(&c.name), c.count))
        .collect::<Vec<_>>()
        .join("");
    let latest_block = if l.latest.is_empty() {
        String::new()
    } else {
        let items = l
            .latest
            .iter()
            .map(|s| {
                format!(
                    r##"<li><a href="/watch/{id}">《{title}》</a><span>{scenes} 幕 · {chars} 字</span></li>"##,
                    id = esc(&s.id),
                    title = esc(&s.title),
                    scenes = s.stats.scenes,
                    chars = s.stats.chars
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            r##"<div class="latest">
  <span class="latest-label">文稿集合</span>
  <ul>{items}</ul>
</div>"##
        )
    };
    let body = format!(
        r##"{header}
<main>
  <section class="hero">
    <div class="wrap hero-grid">
      <div>
        <p class="kicker">wayhug.com · Rust IP Lab</p>
        <h1>把一段文字，<br>变成一部<em>会呼吸</em>的动画。</h1>
        <p class="slogan">可以！</p>
        <p class="lede">看一段 TXT 长成扁平的 CSS / HTML5 动画，下载一张可打印、可校验的 A4 纪念证书。{free_until} 前按 MIT 协议免费授权；其余事宜直接写信给作者。</p>
        <div class="hero-actions">
          <a class="btn primary" href="#create">写一个故事</a>
          <a class="btn" href="#watch">看演示动画</a>
        </div>
        <ul class="mini-stats">
          <li><strong>{scenes} 幕</strong>演示作品</li>
          <li><strong>{cast} 个</strong>几何动物</li>
          <li><strong>5 层</strong>同一事实源</li>
          <li><strong>{stories} 部</strong>已收录作品</li>
        </ul>
      </div>
      <div>
        <div class="stage-frame">
          <iframe src="/watch/demo" title="《{demo_title}》动画演示" loading="lazy"></iframe>
        </div>
        <div class="stage-meta">
          <span>《{demo_title}》 · {demo_chars} 字符 · 指纹 {fp}</span>
          <a href="/watch/demo" target="_blank" rel="noopener">全屏观看 →</a>
        </div>
      </div>
    </div>
  </section>

  <section class="section" id="watch">
    <div class="wrap">
      <div class="section-head">
        <div>
          <span class="index">01 / 看</span>
          <h2>一部从 TXT 直接长出来的动画</h2>
          <p>没有视频、没有位图。每一幕都由 JSON 场景图驱动，CSS 关键帧播放，HTML5 自包含。你可以把它当作展厅，也可以把它拆开研究。</p>
        </div>
        <div class="downloads">
          <a class="chip accent" href="/dl/demo/html">下载 HTML5</a>
          <a class="chip" href="/dl/demo/css">下载 CSS</a>
          <a class="chip" href="/dl/demo/json">下载 JSON</a>
        </div>
      </div>
      <div class="grid-2">
        <div class="panel flat">
          <h3>三件套</h3>
          <p>JSON 是事实源，CSS 是时间轴，HTML5 是舞台。三份文件可以单独带走去二次创作。</p>
          <div class="downloads">{cast_chips}</div>
        </div>
        <div class="panel flat">
          <h3>四相坐标</h3>
          <p>四相是原（自然界）、仙（灵气）、虫（色界生命）、人（欲界的六道）；每一幕都带「四相 × 四属性」坐标，颜色、运动与节奏都由坐标推导，而不是随意挑选。</p>
          <div class="downloads"><a class="chip accent" href="#world">看田字四相 →</a></div>
        </div>
      </div>
      {latest_block}
    </div>
  </section>

  <section class="section" id="create">
    <div class="wrap">
      <div class="section-head">
        <div>
          <span class="index">02 / 写</span>
          <h2>提交 TXT，生成动画</h2>
          <p>第一行可以是标题；空行分段；每段自动切幕。留下的文字越具体，动物、情绪与四相坐标就越准确。</p>
        </div>
        <button class="btn ghost small" type="button" id="example-btn">填入示例故事</button>
      </div>
      <div class="grid-2">
        <form class="form" id="create-form">
          <div class="field">
            <label for="story-title">标题（可留空，自动提取）</label>
            <input class="input" id="story-title" name="title" maxlength="60" inputmode="text" placeholder="例如：小狐与月亮邮局">
          </div>
          <div class="field">
            <label for="story-text">故事正文</label>
            <textarea class="textarea" id="story-text" name="text" placeholder="清晨，小狐在森林里捡到一封没有地址的信……" required></textarea>
            <small>建议 100–1200 字。支持中文标点；空行分段。</small>
          </div>
          <div class="form-foot">
            <button class="btn primary" id="create-btn" type="submit">生成动画</button>
            <span class="status" id="create-status">生成后可预览、下载，并直接进入证书步骤。</span>
          </div>
        </form>
        <div class="panel flat">
          <h3>它是怎么工作的</h3>
          <p>内核层解析 TXT：切句、分幕、识别角色与情绪；引擎层判定四相与四属性；服务层把场景图变成 JSON；交付层输出 CSS 关键帧、HTML5 动画与 A4 证书。</p>
          <div class="downloads" style="margin-top:18px">
            <span class="chip"><strong>内核</strong> TXT → 场景</span>
            <span class="chip"><strong>引擎</strong> 四相 → 色板</span>
            <span class="chip"><strong>交付</strong> CSS / PDF</span>
          </div>
        </div>
      </div>
      <div class="result" id="create-result">
        <div class="result-card">
          <h3 id="create-heading">《{demo_title}》</h3>
          <p id="create-meta"></p>
          <div class="result-links">
            <a class="solid" id="dl-html" href="/dl/demo/html">下载 HTML5</a>
            <a id="dl-css" href="/dl/demo/css">下载 CSS</a>
            <a id="dl-json" href="/dl/demo/json">下载 JSON</a>
            <a id="create-cert" href="#experience">领取纪念证书 →</a>
          </div>
          <div class="stage-frame inline">
            <iframe id="create-preview" title="生成结果预览" loading="lazy"></iframe>
          </div>
        </div>
      </div>
    </div>
  </section>

  <section class="section" id="experience">
    <div class="wrap">
      <div class="section-head">
        <div>
          <span class="index">03 / 领</span>
          <h2>A4 纪念证书 · 免费授权打印页</h2>
          <p>填写称呼与邮箱，即时生成带编号的 A4 证书，可自行下载打印。{free_until} 前按 MIT 协议免费授权；需要人工答复请直接写信。</p>
        </div>
      </div>
      <div class="grid-2">
        <form class="form" id="exp-form">
          <div class="form-row">
            <div class="field">
              <label for="exp-name">称呼</label>
              <input class="input" id="exp-name" name="name" maxlength="40" placeholder="怎么称呼你" required>
            </div>
            <div class="field">
              <label for="exp-email">电子邮箱</label>
              <input class="input" id="exp-email" name="email" type="email" maxlength="120" placeholder="you@example.com" required>
            </div>
          </div>
          <div class="field">
            <label for="exp-story">选择作品</label>
            <select class="select" id="exp-story" name="storyId">
              <option value="demo">《{demo_title}》· 演示作品</option>
            </select>
          </div>
          <div class="field">
            <label for="exp-message">给作者的话（可选）</label>
            <textarea class="input" id="exp-message" name="message" rows="3" maxlength="500" placeholder="你想用这段故事做什么？"></textarea>
          </div>
          <label class="check">
            <input type="checkbox" id="exp-consent" required>
            <span>我理解：{free_until} 前，作品、动画与这张证书按 MIT 协议免费授权，可自由使用、复制、修改与分发，须保留作者署名与许可声明。</span>
          </label>
          <div class="form-foot">
            <button class="btn primary" id="exp-btn" type="submit">生成并领取证书</button>
            <span class="status" id="exp-status">证书编号形如 WH-20261027-72A2。</span>
          </div>
        </form>
        <div class="panel flat">
          <h3>这张证书包含什么</h3>
          <p>作品名、角色、幕数、字符数、内容指纹、持有人、免费授权截止日、MIT 许可摘要、在线校验地址，以及作者署名与扁平印章。</p>
          <div class="downloads" style="margin-top:18px">
            <span class="chip"><strong>A4</strong> 矢量 PDF</span>
            <span class="chip"><strong>编号</strong> 可校验</span>
            <span class="chip"><strong>MIT</strong> 免费授权</span>
          </div>
        </div>
      </div>
      <div class="result" id="exp-result">
        <div class="result-card">
          <h3>证书 <span id="exp-id" class="mono"></span></h3>
          <p>持有人 <strong id="exp-name-out"></strong> · 作品《<span id="exp-title-out"></span>》 · 免费授权至 {free_until}</p>
          <div class="result-links">
            <a class="solid" id="cert-link" href="#">下载 A4 证书 PDF</a>
            <a id="verify-link" href="#">在线校验</a>
            <a id="watch-link" href="/watch/demo">观看动画</a>
          </div>
        </div>
      </div>
    </div>
  </section>

  <section class="section" id="mail">
    <div class="wrap">
      <div class="section-head">
        <div>
          <span class="index">04 / 寄</span>
          <h2>直接写信给作者</h2>
          <p>授权、定制、合作，或只是聊聊故事，都可以发邮件。作者本人查看并回复，一般 1–2 天；本站不自动收信，也不会公开你的邮箱。</p>
        </div>
      </div>
      <div class="mailbox">
        <div class="panel">
          <h3>联系邮箱</h3>
          <a class="mail-addr" href="mailto:{contact_email}">{contact_email}</a>
          <p>邮件里可附上作品名或证书编号，方便核对。</p>
        </div>
        <div class="panel">
          <h3>关于授权</h3>
          <p>{free_until} 前，作品、动画与纪念证书按 MIT 协议免费授权；之后的使用、商用与定制，请来信商谈。</p>
          <p class="mono">完整许可：wayhug.com/license</p>
        </div>
      </div>
    </div>
  </section>

  <section class="section" id="world">
    <div class="wrap">
      <div class="section-head">
        <div>
          <span class="index">05 / 四相</span>
          <h2>田字四分法的世界观</h2>
          <p>四相是四个世界层：原（自然界，狐狸 · 橘）、仙（灵气，小妖怪 · 青）、虫（色界生命，四足小鳄虫 · 黑灰）、人（欲界的六道，农夫 · 黄）。四属性描述动力与材质：风、火、水、土。</p>
        </div>
        <div class="downloads"><a class="chip" href="/api/worldview">JSON 视图</a></div>
      </div>
      <div class="world-grid">
        {quads}
      </div>
      <div class="world-note">
        <span><strong>四相</strong> 原 · 仙 · 虫 · 人</span>
        <span><strong>扁平规则</strong> 只用 256 色、800×600、无渐变、无位图、无音频</span>
        <span><strong>扩展方式</strong> 新文稿按招牌动物与关键词自动归档到四相</span>
      </div>
      {corpus}
    </div>
  </section>
</main>

<footer class="footer">
  <div class="wrap">
    <div class="footer-grid">
      <div>
        <h4>王元会 · IP LAB</h4>
        <p>从 TXT 到动物动画，再到一张可打印、可校验的纪念证书。</p>
        <p>Rust 单二进制 · 扁平化交付 · 五层架构。</p>
      </div>
      <div>
        <h4>联系</h4>
        <p><a href="mailto:{contact_email}">{contact_email}</a></p>
        <p>公众号：旷野馨诗</p>
        <p>人工回复 · 一般 1–2 天</p>
      </div>
      <div>
        <h4>数据</h4>
        <p>作品 {stories} · 申请 {requests} · 原画 {sketches}</p>
        <p>演示作品：{demo_scenes} 幕 / {demo_chars} 字符</p>
        <p class="mono">engine {engine}</p>
      </div>
    </div>
    <div class="footer-bottom">
      <span>© 2026 wayhug.com · 王元会</span>
      <span>{free_until} 前按 MIT 协议免费授权 · <a href="/license">许可全文</a></span>
    </div>
    <div class="footer-beian">
      <p class="beian-tm"><img src="data:image/png;base64,{tm_b64}" alt="王元会商标" width="120"></p>
      <p class="beian-line">
        <a href="https://beian.mps.gov.cn/#/query/webSearch?code=32010502011359" rel="noreferrer" target="_blank">
          <img src="data:image/png;base64,{badge_b64}" alt="" width="14" height="14" class="beian-icon">苏公网安备32010502011359号
        </a>
      </p>
      <p class="beian-line"><a href="http://beian.miit.gov.cn/" rel="noreferrer" target="_blank">苏ICP备17031763号-6</a></p>
    </div>
  </div>
</footer>"##,
        header = header(),
        scenes = demo.stats.scenes,
        cast = demo.cast.len(),
        stories = l.stories,
        requests = l.requests,
        sketches = l.sketches,
        demo_title = esc(&demo.title),
        demo_chars = demo.stats.chars,
        demo_scenes = demo.stats.scenes,
        fp = esc(&demo.fingerprint[..8.min(demo.fingerprint.len())]),
        cast_chips = cast_chips,
        quads = quadrant_cards(),
        latest_block = latest_block,
        corpus = corpus_block(l.corpus),
        free_until = esc(&cfg.free_until),
        contact_email = esc(&cfg.contact_email),
        engine = "wayhug/2.0-rust",
        tm_b64 = base64_encode(TRADEMARK_PNG),
        badge_b64 = base64_encode(BEIAN_BADGE_PNG),
    );

    shell(
        "Wayhug · 可以！",
        &format!(
            "从 TXT 生成扁平化 CSS / HTML5 动物动画，下载可打印的 A4 纪念证书；{} 前按 MIT 协议免费授权。",
            cfg.free_until
        ),
        &body,
        APP_JS,
        "<script src=\"/js/autosave.js\" defer></script>",
    )
}

pub fn license_page() -> String {
    let body = format!(
        r##"{header}
<main class="wrap">
  <div class="panel verify-card">
    <span class="index">LICENSE</span>
    <h1>MIT 许可</h1>
    <p>适用对象：故事 TXT、由其生成的动画 JSON / CSS / HTML，以及 A4 纪念证书 PDF。</p>
    <p>免费授权期至 <strong>2026-12-31</strong>（含当日）。期内可自由使用、复制、修改、合并、发布、分发、再许可与商业使用，须在各副本中保留下列版权与许可声明。</p>
    <pre class="license-text">MIT License

Copyright (c) 2026 王元会 (wayhug.com)

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.</pre>
    <div class="downloads" style="margin-top:22px">
      <a class="btn small" href="/">返回首页</a>
      <a class="btn small" href="mailto:bc.xu@hotmail.com">写信给作者</a>
    </div>
  </div>
</main>
<footer class="footer"><div class="wrap footer-bottom"><span>© 2026 wayhug.com</span><span>许可全文 · MIT</span></div></footer>"##,
        header = header(),
    );
    shell("MIT 许可 · Wayhug", "Wayhug 故事、动画与纪念证书的 MIT 免费授权说明", &body, "", "<script src=\"/js/autosave.js\" defer></script>")
}

pub fn verify_page(cfg: &Config, req: &Request, story_exists: bool) -> String {
    let body = format!(
        r##"{header}
<main class="wrap">
  <div class="panel verify-card">
    <span class="index">CERTIFICATE VERIFY</span>
    <h1>证书校验</h1>
    <p class="mono" style="color:#6b7078">编号 {id}</p>
    <dl class="kv">
      <dt>状态</dt><dd>{status}</dd>
      <dt>持有人</dt><dd>{name}</dd>
      <dt>作品</dt><dd>《{title}》</dd>
      <dt>角色</dt><dd>{cast}</dd>
      <dt>规模</dt><dd>{scenes} 幕 · {chars} 字符</dd>
      <dt>内容指纹</dt><dd class="mono">{fp}</dd>
      <dt>签发时间</dt><dd>{created}</dd>
      <dt>免费授权至</dt><dd>{free_until}</dd>
      <dt>许可</dt><dd><a href="/license">MIT（{free_until} 前）</a></dd>
      <dt>来源</dt><dd>{source}</dd>
      <dt>证书 PDF</dt><dd><a href="/cert/{id}.pdf">/cert/{id}.pdf</a></dd>
      <dt>动画</dt><dd><a href="/watch/{story_id}">/watch/{story_id}</a>{missing}</dd>
    </dl>
    <div class="downloads" style="margin-top:22px">
      <a class="btn small" href="/">返回首页</a>
      <a class="btn small primary" href="/cert/{id}.pdf">下载证书</a>
    </div>
  </div>
</main>
<footer class="footer"><div class="wrap footer-bottom"><span>© 2026 wayhug.com</span><span>校验结果由服务器数据实时生成</span></div></footer>"##,
        header = header(),
        id = esc(&req.id),
        status = if req.status == "replied" { "已人工答复" } else { "已签发（待人工答复）" },
        name = esc(&req.name),
        title = esc(&req.story_title),
        cast = esc(&req.cast_names.join(" / ")),
        scenes = req.scenes,
        chars = req.chars,
        fp = esc(&req.fingerprint),
        created = esc(&req.created_at.chars().take(19).collect::<String>()),
        free_until = esc(&cfg.free_until),
        source = esc(&req.source),
        story_id = esc(&req.story_id),
        missing = if story_exists { "" } else { "（作品已下架）" },
    );
    shell("证书校验 · Wayhug", "校验 Wayhug 纪念证书编号与免费授权期", &body, "", "<script src=\"/js/autosave.js\" defer></script>")
}

pub fn studio_page(cfg: &Config) -> String {
    let body = format!(
        r##"{header}
<main class="wrap studio">
  <div class="studio-head">
    <div>
      <span class="index">STUDIO / 后台</span>
      <h1>工作台</h1>
      <p style="margin:6px 0 0;color:#6b7078">申请、作品、原画喂养。公众页面不显示这里的任何入口。</p>
    </div>
    <div class="token-row">
      <input class="input" id="token" placeholder="管理令牌" style="max-width:260px" autocomplete="off">
      <button class="btn small" id="save-token" type="button">保存</button>
    </div>
  </div>

  <section class="panel" style="margin-bottom:22px">
    <h3>待办申请</h3>
    <p class="status" id="admin-status">输入令牌后自动加载。</p>
    <div style="overflow:auto" id="requests-wrap">
      <table class="table" id="requests">
        <thead><tr><th>编号</th><th>访客</th><th>作品</th><th>状态</th><th>人工答复</th></tr></thead>
        <tbody></tbody>
      </table>
    </div>
  </section>

  <section class="panel" style="margin-bottom:22px">
    <h3>原画喂养</h3>
    <p>上传素描原画，模块会计算指纹、提取主色，并给出四相 / 四属性建议。文件只存服务器，不进入公众页面。</p>
    <form id="sketch-form" class="form" style="margin-top:16px">
      <div class="form-row">
        <div class="field"><label>标题</label><input class="input" id="sk-title" maxlength="60" placeholder="例如：小狐初稿 01"></div>
        <div class="field"><label>四相（留空自动）</label>
          <select class="select" id="sk-phase">
            <option value="">自动建议</option>
            <option value="dawn">晨</option><option value="day">昼</option>
            <option value="dusk">暮</option><option value="night">夜</option>
          </select>
        </div>
      </div>
      <div class="form-row">
        <div class="field"><label>四属性（留空自动）</label>
          <select class="select" id="sk-element">
            <option value="">自动建议</option>
            <option value="wind">风</option><option value="fire">火</option>
            <option value="water">水</option><option value="earth">土</option>
          </select>
        </div>
        <div class="field"><label>标签（逗号分隔）</label><input class="input" id="sk-tags" placeholder="小狐, 线稿, 第1集"></div>
      </div>
      <div class="field"><label>备注</label><input class="input" id="sk-note" maxlength="300" placeholder="这张原画希望保留的线条 / 构图 / 情绪"></div>
      <div class="field"><label>图片（PNG / JPEG，≤12MB）</label><input class="input" id="sk-file" type="file" accept="image/png,image/jpeg" required></div>
      <div class="form-foot">
        <button class="btn primary" type="submit">喂养入库</button>
        <span class="status" id="sk-status"></span>
      </div>
    </form>
    <div class="gallery" id="gallery"></div>
  </section>

  <section class="panel">
    <h3>邮件</h3>
    <p>IMAP 只读轮询：不写 \Seen、不删除，与 jshnp.com 共用邮箱互不干扰。也可手动触发一次。</p>
    <div class="form-foot" style="margin-top:14px">
      <button class="btn small" id="poll-btn" type="button">立即轮询 IMAP</button>
      <span class="status" id="poll-status">SMTP：{smtp} · IMAP：{imap} · 自动回信：{auto}</span>
    </div>
  </section>
</main>
<footer class="footer"><div class="wrap footer-bottom"><span>© 2026 wayhug.com</span><span>工作台仅限持有令牌的作者使用</span></div></footer>"##,
        header = header(),
        smtp = if cfg.smtp_ready() { "已配置" } else { "未配置" },
        imap = if cfg.imap_ready() { "已启用" } else { "未启用" },
        auto = if cfg.auto_reply { "开" } else { "关" },
    );
    shell(
        "Studio · Wayhug",
        "Wayhug 后台工作台",
        &body,
        "",
        &format!("<script src=\"/js/autosave.js\" defer></script><script>{}</script>", STUDIO_JS),
    )
}

pub fn error_page(code: u16, message: &str) -> String {
    let body = format!(
        r##"{header}
<main class="wrap"><div class="panel verify-card">
  <span class="index">ERROR {code}</span>
  <h1>{message}</h1>
  <p>请返回首页，或使用邮件联系作者。</p>
  <div class="downloads"><a class="btn small" href="/">返回首页</a></div>
</div></main>"##,
        header = header(),
        code = code,
        message = esc(message)
    );
    shell(&format!("{} · Wayhug", message), message, &body, "", "<script src=\"/js/autosave.js\" defer></script>")
}
