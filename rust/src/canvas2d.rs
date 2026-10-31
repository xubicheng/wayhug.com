// SPDX-License-Identifier: MIT
// Copyright (c) 2026 王元会 (wayhug.com) <bc.xu@hotmail.com>. All rights reserved.
// 本文件为作者独立编写的原创代码，不含任何第三方（含 Blender）代码或资产。
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
//   · 每一幕的四相（原/仙/虫/人）决定招牌动物与主色
//   · 布景只画「该幕文稿里真的出现过」的物象（props），不凭空加景
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
    let (sky, ground, decor, shade, shadow, light) = match realm {
        Realm::Yuan => ("#ccccff", "#99cc66", "#669933", "#996633", "#669933", "#ffcc99"),
        Realm::Xian => ("#ccffff", "#99cccc", "#66cccc", "#339999", "#339999", "#ffffff"),
        Realm::Chong => ("#cccccc", "#999999", "#666666", "#333333", "#666666", "#cccccc"),
        Realm::Ren => ("#ffcc99", "#cc9966", "#996633", "#663333", "#996633", "#ffcc99"),
    };
    json!({
        "sky": palette::snap_hex(sky),
        "ground": palette::snap_hex(ground),
        "decor": palette::snap_hex(decor),
        "shade": palette::snap_hex(shade),
        "shadow": palette::snap_hex(shadow),
        "light": palette::snap_hex(light),
        "sun": palette::snap_hex("#ffcc66"),
        "ink": palette::snap_hex("#333333"),
        "accent": realm.accent(),
    })
}

/// 文稿 → 物象：只有文本里出现过的词，才允许出现在画面上
const PROP_WORDS: &[(&str, &[&str])] = &[
    ("sun", &["太阳", "日头", "阳光", "晒", "日出", "天亮", "晨光", "黄昏"]),
    ("moon", &["月亮", "月", "星"]),
    ("cloud", &["云", "雾", "烟", "霞"]),
    ("tree", &["树", "林", "森", "枝", "木"]),
    ("grass", &["草", "野", "原野", "甸"]),
    ("stone", &["石", "岩", "山", "坡", "崖"]),
    ("leaf", &["叶"]),
    ("field", &["田", "禾", "苗", "稻", "麦", "耕", "收成", "庄稼", "垄"]),
    ("house", &["屋", "房", "家", "门", "窗", "邮局", "站台"]),
    ("river", &["河", "水", "溪", "流", "雨", "海", "湖", "泉"]),
    ("flower", &["花", "香"]),
];

/// 每幕最多 4 种物象，保持画面简练
pub fn props_for(text: &str) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for (key, words) in PROP_WORDS {
        if words.iter().any(|w| text.contains(w)) {
            out.push(*key);
        }
        if out.len() >= 4 {
            break;
        }
    }
    out
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
            "animalName": "狐狸",
            "realmName": "原",
            "caption": short(&story.title, 40),
            "theme": theme(Realm::Yuan),
            "props": props_for(&story.title),
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
                    "animalName": realm.animal_name(),
                    "realmName": realm.name(),
                    "caption": short(&sc.text, 44),
                    "beat": crate::worldview::Phase::from_key(&sc.phase)
                        .unwrap_or(crate::worldview::Phase::Dawn)
                        .beat(),
                    "theme": theme(realm),
                    "props": props_for(&sc.text)
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
<meta name="viewport" content="width=device-width, initial-scale=1, maximum-scale=2, user-scalable=yes">
<title>__TITLE__ · 画布版</title>
<style>
  html,body{margin:0;padding:0;background:#fff;color:#222;box-sizing:border-box;
    overflow-x:hidden;
    font:12px/1.6 ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
  .shell{width:800px;max-width:100%;margin:0 auto;padding:14px 0 22px;box-sizing:border-box}
  .top{display:flex;justify-content:space-between;align-items:baseline;
    border-bottom:1px solid #ddd;padding:0 0 6px;margin-bottom:10px}
  .top b{font-size:13px;letter-spacing:.02em}
  .top span{color:#888}
  canvas{display:block;width:100%;aspect-ratio:4/3;image-rendering:pixelated;
    box-sizing:border-box;background:#eee;border:1px solid #ddd}
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
  'use strict';
  var D = __DATA__;
  var cv = document.getElementById('c');
  var g = cv.getContext('2d');
  var W = D.w, H = D.h, HZ = Math.round(H * 0.62), BY = HZ + 100;
  var q = new URLSearchParams(location.search);
  var still = q.has('still');
  var focus = Math.max(0, Math.min(D.scenes.length - 1, parseInt(q.get('scene') || '0', 10) || 0));
  var cap = document.getElementById('cap');
  var idx = document.getElementById('idx');
  var FONT = 'ui-monospace, SFMono-Regular, Menlo, Consolas, monospace';
  var SHIRT = '#3399cc';   // 人相农夫的布衣（256 色立方整档）

  // ---- 图元：只允许圆、矩形、多边形、线段 ----
  function rect(c, x, y, w, h){ g.fillStyle = c; g.fillRect(Math.round(x), Math.round(y), Math.round(w), Math.round(h)); }
  function circle(c, x, y, r){ g.fillStyle = c; g.beginPath(); g.arc(Math.round(x), Math.round(y), r, 0, 6.2832); g.fill(); }
  function poly(c, pts){ g.fillStyle = c; g.beginPath(); g.moveTo(pts[0][0], pts[0][1]);
    for (var i = 1; i < pts.length; i++) g.lineTo(pts[i][0], pts[i][1]);
    g.closePath(); g.fill(); }
  function line(c, x1, y1, x2, y2, w){ g.strokeStyle = c; g.lineWidth = w || 3;
    g.beginPath(); g.moveTo(Math.round(x1), Math.round(y1)); g.lineTo(Math.round(x2), Math.round(y2)); g.stroke(); }
  function at(x, y, fn){ g.save(); g.translate(x, y); fn(); g.restore(); }
  function turn(x, y, a, fn){ g.save(); g.translate(x, y); g.rotate(a); fn(); g.restore(); }
  function has(sc, k){ return (sc.props || []).indexOf(k) >= 0; }

  // ---- 布景：只画文稿里出现过的物象 ----
  function scenery(sc, t){
    var th = sc.theme, s = t / 1000;
    rect(th.sky, 0, 0, W, HZ);
    rect(th.ground, 0, HZ, W, H - HZ);
    rect(th.ink, 0, HZ - 3, W, 3);

    if (has(sc, 'sun')) circle(th.sun, 664, 96, 32);
    if (has(sc, 'moon')) { circle(th.light, 664, 92, 30); circle(th.sky, 680, 82, 24); }
    if (has(sc, 'cloud')) {
      circle(th.light, 170, 112, 28); circle(th.light, 204, 120, 38);
      circle(th.light, 240, 110, 24);
      circle(th.light, 600, 150, 22); circle(th.light, 632, 158, 30);
    }
    if (has(sc, 'river')) {
      rect(th.shade, 0, HZ + 150, W, 24);
      for (var r = 0; r < 6; r++) {
        var rx = (r * 150 + Math.floor(s * 22)) % W;
        line(th.light, rx, HZ + 158, rx + 46, HZ + 158, 2);
      }
    }
    if (has(sc, 'house')) {
      rect(th.decor, 610, HZ - 118, 84, 118);
      poly(th.shade, [[596, HZ - 118], [652, HZ - 158], [708, HZ - 118]]);
      rect(th.ink, 642, HZ - 48, 20, 48);
    }
    if (has(sc, 'tree')) {
      var trees = [[130, 0.92], [560, 0.8]];
      for (var k = 0; k < trees.length; k++) {
        var x = trees[k][0], sc2 = trees[k][1];
        rect(th.shade, x - 6, BY - 76 * sc2, 12, 76 * sc2);
        circle(th.decor, x, BY - 92 * sc2, Math.round(36 * sc2));
      }
    }
    if (has(sc, 'stone')) {
      for (var b = 0; b < 4; b++) {
        var bx = 80 + b * 200, sr = 20 + b * 4, gy = HZ + 40;
        poly(th.decor, [[bx - sr, gy], [bx - sr + 8, gy - sr], [bx + 4, gy - sr - 8], [bx + sr, gy]]);
      }
    }
    if (has(sc, 'field')) {
      for (var f = HZ + 28; f < H; f += 24) rect(th.shade, 0, f, W, 4);
      for (var c = 0; c < 12; c++) {
        var cx2 = 40 + c * 64, sway = Math.sin(s * 1.6 + c) * 3, gy2 = HZ + 46;
        line(th.decor, cx2, gy2, cx2 + sway, gy2 - 24, 2);
        line(th.decor, cx2 + sway, gy2 - 16, cx2 + sway - 7, gy2 - 22, 2);
        line(th.decor, cx2 + sway, gy2 - 16, cx2 + sway + 7, gy2 - 22, 2);
      }
    }
    if (has(sc, 'grass')) {
      for (var x2 = 20; x2 < W; x2 += 46) {
        var s2 = 7 + (x2 % 3), gy3 = HZ + 104;
        line(th.decor, x2, gy3, x2, gy3 - s2 * 3, 2);
        line(th.decor, x2, gy3, x2 - s2, gy3 - s2 * 2, 2);
        line(th.decor, x2, gy3, x2 + s2, gy3 - s2 * 2, 2);
      }
    }
    if (has(sc, 'flower')) {
      for (var fl = 0; fl < 4; fl++) {
        var fx2 = 180 + fl * 140, fy2 = HZ + 78;
        line(th.decor, fx2, fy2 + 18, fx2, fy2, 2);
        circle(th.light, fx2, fy2 - 4, 6);
        circle(th.sun, fx2, fy2 - 4, 2);
      }
    }
    if (has(sc, 'leaf')) {
      for (var lf = 0; lf < 4; lf++) {
        var lx = 110 + lf * 180 + Math.sin(s * 0.6 + lf) * 16;
        var ly = HZ - ((s * 26 + lf * 130) % 280);
        poly(th.shade, [[lx, ly], [lx + 9, ly - 5], [lx + 4, ly + 6]]);
      }
    }
  }

  // ---- 招牌动物：动作取自四相性格 ----

  // 原 · 狐狸（橘，容易害怕：发抖、耳朵抽动、尾巴乱摆、眨眼）
  function fox(sc, t){
    var th = sc.theme, s = t / 1000, bx = 400, by = BY;
    var shake = Math.round(Math.sin(s * 23));
    var wag = Math.sin(s * 3.1) * 0.22;
    var ear = Math.sin(s * 5.3) > 0.92 ? 0.22 : 0;
    var blink = (s % 1.7) > 1.6;
    rect(th.shadow, bx - 34, by - 3, 68, 4);
    at(bx + shake, by, function(){
      turn(-14, -34, wag, function(){
        poly(th.accent, [[0, 0], [-46, -16], [-54, 4], [-14, 12]]);
        poly(th.light, [[-46, -16], [-54, 4], [-46, 10]]);
      });
      circle(th.accent, -2, -34, 24);
      circle(th.accent, 16, -42, 18);
      rect(th.shade, -16, -16, 7, 16);
      rect(th.shade, 10, -16, 7, 16);
      circle(th.accent, 36, -58, 17);
      poly(th.light, [[46, -52], [62, -58], [46, -42]]);
      circle(th.ink, 60, -56, 3);
      if (blink) line(th.ink, 36, -62, 44, -62, 2);
      else circle(th.ink, 40, -62, 2.5);
      turn(30, -70, ear, function(){ poly(th.accent, [[-10, 3], [-5, -16], [4, -2]]); });
      turn(46, -72, -ear * 0.8, function(){ poly(th.accent, [[-4, 2], [4, -15], [8, 2]]); });
    });
  }

  // 仙 · 小妖怪（青，喜欢笑着：漂浮、弯眼笑嘴、摆手、灵气环绕）
  function monster(sc, t){
    var th = sc.theme, s = t / 1000, by = BY, fy = by - 96 + Math.sin(s * 1.7) * 9;
    rect(th.shadow, 370, by - 3, 60, 4);
    at(400, fy, function(){
      circle(th.accent, 0, 0, 24);
      circle(th.decor, 0, -26, 20);
      poly(th.accent, [[-15, -40], [-22, -56], [-8, -46]]);
      poly(th.accent, [[15, -40], [22, -56], [8, -46]]);
      g.strokeStyle = th.ink; g.lineWidth = 2;
      g.beginPath(); g.arc(-9, -30, 5, 3.34, 6.08); g.stroke();
      g.beginPath(); g.arc(9, -30, 5, 3.34, 6.08); g.stroke();
      g.lineWidth = 2.5;
      g.beginPath(); g.arc(0, -26, 9, 0.35, 2.79); g.stroke();
      var a = Math.sin(s * 3.4) * 0.5;
      turn(-20, -2, a, function(){ line(th.decor, 0, 0, -22, 16, 4); circle(th.light, -22, 16, 4); });
      turn(20, -2, -a, function(){ line(th.decor, 0, 0, 22, 16, 4); circle(th.light, 22, 16, 4); });
      line(th.shade, -8, 22, -8, 30, 4);
      line(th.shade, 8, 22, 8, 30, 4);
    });
    for (var k = 0; k < 4; k++) {
      var ang = s * 0.5 + k * 1.57;
      var sx = 400 + Math.cos(ang) * 76, sy = fy + Math.sin(ang) * 34;
      poly(th.light, [[sx, sy - 5], [sx + 5, sy], [sx, sy + 5], [sx - 5, sy]]);
    }
  }

  // 虫 · 四足小鳄虫（黑灰，会走路：四腿交替、壳纹随动、往返）
  function bug(sc, t){
    var th = sc.theme, s = t / 1000, by = BY + 2;
    var cx = 400 + Math.sin(s * 0.85) * 46;
    var bob = Math.round(Math.sin(s * 6));
    rect(th.shadow, cx - 40, by - 3, 80, 4);
    at(cx, by + bob, function(){
      var phase = [0, 3.1416, 3.1416, 0], xs = [-26, -10, 10, 26];
      for (var i = 0; i < 4; i++) {
        var a = Math.sin(s * 6 + phase[i]) * 0.45;
        turn(xs[i], -28, a, function(){
          line(th.shade, 0, 0, -4, 15, 5);
          line(th.shade, -4, 15, 0, 28, 4);
        });
      }
      rect(th.decor, -40, -50, 80, 28);
      circle(th.decor, -40, -36, 14);
      circle(th.decor, 40, -36, 14);
      for (var k = -24; k <= 24; k += 16) line(th.shade, k, -48, k, -24, 2);
      circle(th.decor, 54, -42, 14);
      poly(th.decor, [[50, -30], [72, -32], [50, -24]]);
      circle(th.ink, 58, -46, 3);
      line(th.ink, 62, -30, 70, -31, 2);
      poly(th.decor, [[-54, -40], [-72, -34], [-54, -26]]);
    });
  }

  // 人 · 农夫（黄，耕田有收成：举锄、落锄、起身）
  function farmer(sc, t){
    var th = sc.theme, s = t / 1000, by = BY, bx = 400;
    var hoe = -1.05 + Math.sin(s * 1.5) * 0.55;
    rect(th.shadow, bx - 34, by - 3, 68, 4);
    at(bx, by, function(){
      rect(th.ink, -16, -26, 12, 26);
      rect(th.ink, 4, -26, 12, 26);
      rect(SHIRT, -20, -78, 40, 54);
      circle(th.light, 0, -94, 15);
      circle(th.ink, -5, -96, 2);
      circle(th.ink, 5, -96, 2);
      line(th.ink, -5, -88, 5, -88, 2);
      poly(th.decor, [[-26, -100], [26, -100], [20, -108], [-20, -108]]);
      poly(th.shade, [[-18, -108], [18, -108], [12, -122], [-12, -122]]);
      turn(-6, -72, hoe, function(){
        line(th.light, 0, 0, 0, 66, 4);
        line(th.shade, 0, 66, 34, 84, 5);
        poly(th.decor, [[34, 84], [46, 78], [50, 92], [36, 96]]);
        line(th.light, 0, 0, 18, 30, 4);
      });
      line(th.light, 8, -72, 26, -52, 4);
    });
  }

  var actors = { fox: fox, monster: monster, bug: bug, farmer: farmer };

  // ---- 画面装配：四相标签 + 进度 ----
  function hud(sc, local){
    var th = sc.theme;
    rect(th.accent, 24, 28, 6, 26);
    g.fillStyle = th.ink;
    g.font = 'bold 20px ' + FONT;
    g.fillText(sc.realmName, 40, 50);
    g.font = '12px ' + FONT;
    g.fillText(sc.animalName + ' · ' + sc.beat, 74, 47);

    g.globalAlpha = 1;

    g.globalAlpha = 0.16;
    g.fillStyle = th.ink;
    g.fillRect(24, H - 20, W - 48, 4);
    g.globalAlpha = 1;
    rect(th.accent, 24, H - 20, Math.round((W - 48) * Math.min(1, local / D.dwell)), 4);
  }

  function draw(){
    var i = still ? focus : Math.floor(elapsed / D.dwell) % D.scenes.length;
    var local = still ? 1200 : elapsed % D.dwell;
    var sc = D.scenes[i];
    scenery(sc, local);
    (actors[sc.animal] || fox)(sc, local);
    hud(sc, local);
    cap.textContent = sc.caption;
    idx.textContent = (i + 1) + ' / ' + D.scenes.length;
  }

  var elapsed = still ? focus * D.dwell + 1200 : 0;
  var playing = !still, last = -1;

  function loop(ts){
    if (last >= 0 && playing) elapsed += ts - last;
    last = ts;
    draw();
    if (playing) requestAnimationFrame(loop);
  }

  function toggle(){
    playing = !playing;
    if (playing) { last = -1; requestAnimationFrame(loop); }
  }

  draw();
  if (!still) requestAnimationFrame(loop);
  cv.addEventListener('click', toggle);
  document.addEventListener('keydown', function(e){
    if (e.code === 'Space') { e.preventDefault(); toggle(); }
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

    /// 布景必须来自文稿：只有文本里出现过的物象才会进入 props
    #[test]
    fn props_come_from_text() {
        let story = demo_story();
        let expected: Vec<Vec<&'static str>> =
            story.scenes.iter().map(|sc| props_for(&sc.text)).collect();
        let data = scene_data(&story);
        let scenes = data["scenes"].as_array().unwrap();
        for (i, exp) in expected.iter().enumerate() {
            let drawn = scenes[i]["props"].as_array().unwrap();
            assert_eq!(drawn.len(), exp.len());
            for p in exp {
                assert!(drawn.iter().any(|v| v.as_str() == Some(*p)));
            }
        }
        assert_eq!(props_for("小狐在清晨的邮局门口等信"), vec!["house"]);
        assert_eq!(props_for("四足小鳄虫爬过石头"), vec!["stone"]);
        assert!(props_for("农夫在田里想着收成").contains(&"field"));
        assert!(props_for("小妖怪笑着飘过云层").contains(&"cloud"));
        assert!(props_for("").is_empty());
    }

    /// 主题色必须全部落在 256 色表上
    #[test]
    fn theme_colors_are_palette_exact() {
        for realm in crate::worldview::REALMS {
            let t = theme(realm);
            for key in ["sky", "ground", "decor", "shade", "shadow", "light", "sun", "ink", "accent"] {
                let v = t[key].as_str().unwrap();
                assert_eq!(v, crate::palette::snap_hex(v), "{} 未贴色", key);
            }
        }
    }
}
