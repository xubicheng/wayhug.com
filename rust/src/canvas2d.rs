// ============================================================
// 交付层 · 2D 画布工具（HTML5 Canvas，800×600，256 色）
//
// 这是「我自己用的」生成工具：TXT → 自包含 HTML5 页面（一张画布）。
// 它借鉴 Blender 的机制（场景/图层/时间轴/关键帧/色彩管理），
// 但代码完全自研，不链接、不移植 Blender 的任何 GPL 代码。
//
// 约定：
//   · 分辨率固定 800×600（4:3），画布像素就是设计像素
//   · 颜色只允许来自 palette.rs 的 256 色表，禁止渐变与位图
//   · 文件只输出 HTML/CSS/JS/SVG（HTML5 类别），不含音频
//   · 每一幕的四相（原/仙/虫/人）决定布景、招牌动物与主色
// ============================================================

use crate::model::Story;
use crate::palette;
use crate::worldview::{realm_of_scene, Realm};
use serde_json::json;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const WIDTH: u32 = 800;
pub const HEIGHT: u32 = 600;
/// 每幕停留（毫秒）
pub const DWELL_MS: u32 = 4200;

/// 每一幕的配色（全部贴到 256 色表）
fn theme(realm: Realm) -> serde_json::Value {
    // 只取 256 色立方里的整数档（00 / 33 / 66 / 99 / cc / ff），保证贴色零误差
    let (sky, ground, decor, shade) = match realm {
        Realm::Yuan => ("#ccccff", "#99cc66", "#669933", "#996633"),
        Realm::Xian => ("#ccffff", "#99cccc", "#66cccc", "#339999"),
        Realm::Chong => ("#cccccc", "#999999", "#666666", "#333333"),
        Realm::Ren => ("#ffcc99", "#cc9966", "#996633", "#663333"),
    };
    json!({
        "sky": palette::snap_hex(sky),
        "ground": palette::snap_hex(ground),
        "decor": palette::snap_hex(decor),
        "shade": palette::snap_hex(shade),
        "ink": palette::snap_hex("#333333"),
        "accent": realm.accent(),
    })
}

fn short(text: &str, max: usize) -> String {
    let clean = text.replace('\n', " ").trim().to_string();
    if clean.chars().count() <= max {
        return clean;
    }
    let cut: String = clean.chars().take(max.saturating_sub(1)).collect();
    format!("{}…", cut.trim_end())
}

/// 把一篇 Story 编译成画布数据（场景 / 图层 / 时间轴）
pub fn scene_data(story: &Story) -> serde_json::Value {
    let scenes: Vec<serde_json::Value> = if story.scenes.is_empty() {
        vec![json!({
            "realm": "yuan",
            "animal": "fox",
            "realmName": "原",
            "caption": short(&story.title, 40),
            "theme": theme(Realm::Yuan),
            "beat": "起 · 相遇"
        })]
    } else {
        story
            .scenes
            .iter()
            .map(|sc| {
                let realm = realm_of_scene(sc);
                json!({
                    "realm": realm.key(),
                    "animal": realm.animal_key(),
                    "realmName": realm.name(),
                    "caption": short(&sc.text, 44),
                    "beat": crate::worldview::Phase::from_key(&sc.phase)
                        .unwrap_or(crate::worldview::Phase::Dawn)
                        .beat(),
                    "theme": theme(realm)
                })
            })
            .collect()
    };
    json!({
        "title": story.title,
        "w": WIDTH,
        "h": HEIGHT,
        "dwell": DWELL_MS,
        "palette": "256",
        "scenes": scenes
    })
}

/// 生成自包含的 HTML5 页面
pub fn render_html(story: &Story) -> String {
    let data = scene_data(story).to_string().replace("</", "<\\/");
    TEMPLATE
        .replace("__TITLE__", &crate::animate::esc(&story.title))
        .replace("__DATA__", &data)
}

/// 工具入口：把动画写到磁盘（CLI 用）
pub fn write_file(story: &Story, dir: &Path) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let path = dir.join(format!("{}.html", story.id));
    fs::write(&path, render_html(story))?;
    Ok(path)
}

const TEMPLATE: &str = r##"<!doctype html>
<html lang="zh">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>__TITLE__ · 画布版</title>
<style>
  html,body{margin:0;padding:0;background:#fff;color:#222;
    font:12px/1.6 ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
  .shell{width:800px;max-width:100%;margin:0 auto;padding:14px 0 22px}
  .top{display:flex;justify-content:space-between;align-items:baseline;
    border-bottom:1px solid #ddd;padding:0 0 6px;margin-bottom:10px}
  .top b{font-size:13px;letter-spacing:.02em}
  .top span{color:#888}
  canvas{display:block;width:100%;aspect-ratio:4/3;image-rendering:pixelated;
    background:#eee;border:1px solid #ddd}
  .meta{display:flex;justify-content:space-between;gap:12px;margin-top:8px;color:#666}
  .cap{min-height:20px;color:#333}
  .foot{margin-top:14px;color:#999;display:flex;justify-content:space-between}
  a{color:#555}
</style>
</head>
<body>
<div class="shell">
  <div class="top"><b>__TITLE__</b><span>256 色 · 800×600 · HTML5 Canvas</span></div>
  <canvas id="c" width="800" height="600"></canvas>
  <div class="meta">
    <div class="cap" id="cap"></div>
    <div id="idx"></div>
  </div>
  <div class="foot"><span>点击画面暂停 / 继续 · 空格同效</span><a href="https://wayhug.com/">wayhug.com</a></div>
</div>
<script>
(function(){
  var D = __DATA__;
  var cv = document.getElementById('c');
  var g = cv.getContext('2d');
  var W = D.w, H = D.h, HZ = Math.round(H * 0.62);
  var q = new URLSearchParams(location.search);
  var still = q.has('still');
  var focus = Math.max(0, Math.min(D.scenes.length - 1, parseInt(q.get('scene') || '0', 10) || 0));
  var cap = document.getElementById('cap');
  var idx = document.getElementById('idx');
  var t0 = 0, playing = !still, last = -1;

  function ink(t){ g.fillStyle = t.ink; }
  function rect(t, x, y, w, h){ g.fillStyle = t; g.fillRect(Math.round(x), Math.round(y), w, h); }
  function circle(c, x, y, r){ g.fillStyle = c; g.beginPath(); g.arc(Math.round(x), Math.round(y), r, 0, 6.2832); g.fill(); }
  function poly(c, pts){ g.fillStyle = c; g.beginPath(); g.moveTo(pts[0][0], pts[0][1]);
    for (var i = 1; i < pts.length; i++) g.lineTo(pts[i][0], pts[i][1]);
    g.closePath(); g.fill(); }
  function line(c, x1, y1, x2, y2, w){ g.strokeStyle = c; g.lineWidth = w || 3;
    g.beginPath(); g.moveTo(Math.round(x1), Math.round(y1)); g.lineTo(Math.round(x2), Math.round(y2)); g.stroke(); }

  // ---- 布景（按四相） ----
  function scenery(sc, t, i) {
    var th = sc.theme;
    g.fillStyle = th.sky; g.fillRect(0, 0, W, HZ);
    g.fillStyle = th.ground; g.fillRect(0, HZ, W, H - HZ);
    rect(th.ink, 0, HZ - 3, W, 3);

    if (sc.realm === 'yuan') {
      circle(th.accent, 668, 108, 34);
      var trees = [[120, 0.95], [252, 0.78], [598, 0.9]];
      for (var k = 0; k < trees.length; k++) {
        var x = trees[k][0], s = trees[k][1];
        rect(th.shade, x - 5, HZ - 92 * s, 10, 92 * s);
        circle(th.decor, x, HZ - 104 * s, Math.round(38 * s));
      }
      g.fillStyle = th.decor;
      for (var i2 = 0; i2 < 24; i2++) {
        var gx = (i2 * 41 + Math.floor(t / 90)) % W;
        g.fillRect(gx, HZ + 34 + (i2 % 4) * 18, 3, 9);
      }
    } else if (sc.realm === 'xian') {
      for (var d = 0; d < 7; d++) {
        var dx = 92 + d * 104, dy = 148 + Math.sin(t / 720 + d) * 18;
        var c = d % 2 ? th.accent : th.decor;
        poly(c, [[dx, dy - 16], [dx + 14, dy], [dx, dy + 16], [dx - 14, dy]]);
      }
      circle(th.decor, 196, 250, 30);
      circle(th.decor, 232, 244, 38);
      circle(th.decor, 268, 250, 30);
    } else if (sc.realm === 'chong') {
      for (var b = 0; b < 5; b++) {
        var bx = 70 + b * 160, by = HZ - 8 - (b % 2) * 10;
        poly(th.decor, [[bx, by - 46], [bx + 16, by], [bx, by + 8], [bx - 16, by]]);
        rect(th.shade, bx - 2, by + 6, 4, 26);
      }
      g.fillStyle = th.shade;
      for (var p = 0; p < 9; p++) g.fillRect(20 + p * 88, HZ + 30 + (p % 3) * 22, 34, 8);
    } else {
      circle(th.accent, 668, 96, 30);
      g.fillStyle = th.shade;
      for (var f = 0; f < 9; f++) g.fillRect(0, HZ + 26 + f * 20, W, 4);
      rect(th.shade, 96, HZ - 96, 118, 96);
      poly(th.decor, [[84, HZ - 96], [155, HZ - 146], [226, HZ - 96]]);
      rect(th.ink, 128, HZ - 60, 34, 60);
    }
  }

  // ---- 招牌动物（扁平几何） ----
  function fox(sc, t, x, y) {
    var th = sc.theme, bob = Math.sin(t / 320) * 3;
    var by = Math.round(y - 22 + bob);
    poly(th.accent, [[x - 40, by - 4], [x - 86, by - 26 + bob], [x - 88, by + 6], [x - 44, by + 10]]);
    g.fillStyle = th.accent;
    g.beginPath(); g.ellipse(x, by, 42, 24, 0, 0, 6.2832); g.fill();
    circle(th.accent, x + 36, by - 34, 22);
    poly(th.accent, [[x + 20, by - 46], [x + 26, by - 72], [x + 36, by - 50]]);
    poly(th.accent, [[x + 42, by - 48], [x + 52, by - 74], [x + 58, by - 44]]);
    rect(th.ink, x + 40, by - 40, 4, 4);
    rect(th.ink, x + 54, by - 38, 4, 4);
    rect(th.ink, x + 56, by - 28, 6, 3);
    line(th.ink, x - 26, by + 22, x - 26, by + 34, 4);
    line(th.ink, x + 16, by + 22, x + 16, by + 34, 4);
  }

  function monster(sc, t, x, y) {
    var th = sc.theme, bob = Math.sin(t / 360) * 4;
    var by = Math.round(y - 52 + bob);
    circle(th.accent, x, by, 44);
    poly(th.accent, [[x - 30, by - 34], [x - 38, by - 62], [x - 12, by - 44]]);
    poly(th.accent, [[x + 30, by - 34], [x + 38, by - 62], [x + 12, by - 44]]);
    circle('#ffffff', x - 14, by - 8, 8);
    circle('#ffffff', x + 14, by - 8, 8);
    circle(th.ink, x - 12, by - 7, 4);
    circle(th.ink, x + 16, by - 7, 4);
    line(th.ink, x - 12, by + 14, x + 12, by + 14, 4);
    line(th.ink, x - 16, by + 14, x - 20, by + 9, 3);
    line(th.ink, x + 16, by + 14, x + 20, by + 9, 3);
    rect(th.shade, x - 16, by + 40, 10, 12);
    rect(th.shade, x + 6, by + 40, 10, 12);
  }

  function bug(sc, t, x, y) {
    var th = sc.theme, bob = Math.sin(t / 240) * 2;
    var by = Math.round(y - 26 + bob);
    poly(th.accent, [[x - 52, by - 8], [x - 84, by - 2], [x - 52, by + 8]]);
    g.fillStyle = th.accent;
    g.beginPath(); g.ellipse(x - 6, by, 48, 20, 0, 0, 6.2832); g.fill();
    circle(th.accent, x + 44, by - 6, 17);
    g.fillStyle = th.shade;
    for (var s = 0; s < 4; s++) g.fillRect(x - 40 + s * 20, by - 16, 4, 32);
    rect(th.ink, x + 50, by - 12, 4, 4);
    var swing = Math.sin(t / 200) * 6;
    line(th.shade, x - 24, by + 14, x - 30 + swing, by + 34, 4);
    line(th.shade, x - 4, by + 14, x + 2 - swing, by + 34, 4);
    line(th.shade, x + 18, by + 14, x + 24 + swing, by + 34, 4);
    line(th.shade, x + 38, by + 14, x + 44 - swing, by + 34, 4);
  }

  function farmer(sc, t, x, y) {
    var th = sc.theme, bob = Math.sin(t / 420) * 2;
    var by = Math.round(y + bob);
    line(th.ink, x - 14, by - 10, x - 16, by + 26, 5);
    line(th.ink, x + 10, by - 10, x + 12, by + 26, 5);
    rect(th.decor, x - 22, by - 62, 44, 56);
    line(th.decor, x - 22, by - 50, x - 44, by - 22, 7);
    line(th.decor, x + 22, by - 50, x + 42, by - 30, 7);
    circle(th.accent, x, by - 82, 16);
    poly(th.shade, [[x - 34, by - 92], [x, by - 116], [x + 34, by - 92]]);
    rect(th.shade, x - 34, by - 94, 68, 5);
    line(th.shade, x + 42, by - 30, x + 64, by - 74, 4);
    poly(th.accent, [[x + 64, by - 74], [x + 78, by - 80], [x + 76, by - 66]]);
    rect(th.ink, x - 6, by - 86, 3, 3);
    rect(th.ink, x + 4, by - 86, 3, 3);
  }

  var actors = { fox: fox, monster: monster, bug: bug, farmer: farmer };

  function draw(sc, t, i) {
    scenery(sc, t, i);
    var fn = actors[sc.animal] || fox;
    fn(sc, t, 400, HZ);
    var th = sc.theme;
    g.fillStyle = th.ink;
    g.font = '14px ui-monospace, Menlo, Consolas, monospace';
    g.fillText(sc.realmName + ' · ' + sc.beat, 24, 38);
    g.fillStyle = th.accent;
    g.fillRect(24, 50, 44, 4);
    // 进度条
    g.fillStyle = th.ink;
    g.globalAlpha = 0.16;
    g.fillRect(0, H - 8, W, 8);
    g.globalAlpha = 1;
    g.fillStyle = th.accent;
    var total = D.scenes.length;
    g.fillRect(Math.round((i / total) * W), H - 8, Math.round(W / total), 8);
    cap.textContent = sc.caption;
    idx.textContent = (i + 1) + ' / ' + total;
  }

  function frame(ts) {
    if (!t0) t0 = ts;
    var now = ts - t0;
    var i = Math.floor(now / D.dwell) % D.scenes.length;
    var local = now % D.dwell;
    if (i !== last) { last = i; }
    draw(D.scenes[i], local, i);
    if (playing) requestAnimationFrame(frame);
  }

  draw(D.scenes[focus], 0, focus);
  if (!still) requestAnimationFrame(frame);

  cv.addEventListener('click', function () {
    if (still) return;
    playing = !playing;
    if (playing) { t0 = 0; requestAnimationFrame(frame); }
  });
  document.addEventListener('keydown', function (e) {
    if (e.code === 'Space') { e.preventDefault(); cv.click(); }
  });
})();
</script>
</body>
</html>
"##;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Scene;

    fn demo_story() -> Story {
        let mut s = Story {
            id: "st_test".into(),
            title: "画布测试".into(),
            ..Default::default()
        };
        s.scenes = vec![
            Scene { index: 0, text: "小狐在清晨的邮局门口等信".into(), animal: "fox".into(), phase: "dawn".into(), ..Default::default() },
            Scene { index: 1, text: "小妖怪笑着飘过云层".into(), animal: "monster".into(), phase: "day".into(), ..Default::default() },
            Scene { index: 2, text: "四足小鳄虫爬过石头".into(), animal: "bug".into(), phase: "dusk".into(), ..Default::default() },
            Scene { index: 3, text: "农夫在田里想着收成".into(), animal: "farmer".into(), phase: "night".into(), ..Default::default() },
        ];
        s
    }

    #[test]
    fn canvas_is_800x600_html5_and_silent() {
        let html = render_html(&demo_story());
        assert!(html.contains("width=\"800\" height=\"600\""));
        assert!(html.contains("canvas"));
        assert!(!html.contains("<audio"));
        assert!(!html.contains("autoplay"));
        assert!(!html.contains("data:image"));
        assert!(!html.contains(".png"));
        assert!(!html.contains(".jpg"));
    }

    #[test]
    fn scenes_follow_four_realms() {
        let data = scene_data(&demo_story());
        let scenes = data["scenes"].as_array().unwrap();
        assert_eq!(scenes.len(), 4);
        assert_eq!(scenes[0]["realm"], "yuan");
        assert_eq!(scenes[1]["realm"], "xian");
        assert_eq!(scenes[2]["realm"], "chong");
        assert_eq!(scenes[3]["realm"], "ren");
        for sc in scenes {
            let accent = sc["theme"]["accent"].as_str().unwrap();
            assert_eq!(accent, crate::palette::snap_hex(accent));
        }
    }
}
