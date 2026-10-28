// ============================================================
// 交付层 · A4 纪念证书（IP 体验权）
//
// 纯矢量扁平排版：色带、色块、细线，无渐变、无位图。
// 中文字体自动探测（Windows / Linux / 自带 assets/fonts），
// 找不到时降级为英文排版，保证 PDF 永远可用。
// ============================================================

use crate::config::Config;
use crate::model::{Request, Story};
use printpdf::*;
use std::fs::File;
use std::path::Path;

const BANDS: [&str; 6] = ["#ff7e47", "#ffb6c1", "#6b5b95", "#2f9e77", "#5d8aa8", "#ffb703"];
const ACCENT: &str = "#ff7e47";
const INK: &str = "#2c3038";
const MUTED: &str = "#8a8a95";
const LINE: &str = "#e6e6ee";
const PAPER: &str = "#f7f7fa";
const GREEN: &str = "#2f9e77";

/// 探测可用的中文字体；优先单字体 TTF（子集化可靠），TTC 会被抽取第 0 号字体
pub fn resolve_font(cfg: &Config) -> Option<String> {
    let mut candidates: Vec<String> = Vec::new();
    if let Some(p) = &cfg.font_path {
        candidates.push(p.clone());
    }
    for p in [
        "assets/fonts/NotoSansSC.ttf",
        "assets/fonts/SourceHanSansSC-Regular.ttf",
        "assets/fonts/wqy-microhei.ttc",
    ] {
        candidates.push(p.to_string());
    }
    for p in [
        "C:/Windows/Fonts/simhei.ttf",
        "C:/Windows/Fonts/Deng.ttf",
        "C:/Windows/Fonts/msyh.ttc",
    ] {
        candidates.push(p.to_string());
    }
    for p in [
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/arphic/uming.ttc",
    ] {
        candidates.push(p.to_string());
    }
    candidates.into_iter().find(|p| Path::new(p).exists())
}

fn rgb(hex: &str) -> Color {
    let h = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(0) as f32 / 255.0;
    let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(0) as f32 / 255.0;
    let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(0) as f32 / 255.0;
    Color::Rgb(Rgb::new(r, g, b, None))
}

/// 字符视觉宽度：中文 1.0，ASCII 约 0.55
fn visual_width(s: &str) -> f32 {
    s.chars()
        .map(|c| if c.is_ascii() { 0.55 } else { 1.0 })
        .sum()
}

fn wrap(text: &str, max_em: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    let mut w = 0.0;
    for c in text.chars() {
        let cw = if c.is_ascii() { 0.55 } else { 1.0 };
        if w + cw > max_em && !cur.is_empty() {
            lines.push(std::mem::take(&mut cur));
            w = 0.0;
        }
        cur.push(c);
        w += cw;
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

fn y(top_mm: f32) -> Mm {
    Mm(297.0 - top_mm)
}

fn text(layer: &PdfLayerReference, font: &IndirectFontRef, s: &str, size: f32, x: f32, top: f32, color: &str) {
    layer.set_fill_color(rgb(color));
    // use_text 的 y 为基线；按字号近似换算，使 top 看起来是文本顶部
    layer.use_text(s, size, Mm(x), y(top + size * 0.26), font);
}

fn text_center(
    layer: &PdfLayerReference,
    font: &IndirectFontRef,
    s: &str,
    size: f32,
    top: f32,
    color: &str,
) {
    let w_mm = visual_width(s) * size * 0.3528;
    text(layer, font, s, size, (210.0 - w_mm) / 2.0, top, color);
}

fn text_right(
    layer: &PdfLayerReference,
    font: &IndirectFontRef,
    s: &str,
    size: f32,
    right_x: f32,
    top: f32,
    color: &str,
) {
    let w_mm = visual_width(s) * size * 0.3528;
    text(layer, font, s, size, right_x - w_mm, top, color);
}

fn block(
    layer: &PdfLayerReference,
    font: &IndirectFontRef,
    s: &str,
    size: f32,
    x: f32,
    top: f32,
    max_em: f32,
    line_mm: f32,
    color: &str,
) {
    for (i, line) in wrap(s, max_em).iter().enumerate() {
        text(layer, font, line, size, x, top + i as f32 * line_mm, color);
    }
}

fn fmt_date(iso: &str) -> String {
    iso.chars().take(10).collect::<String>()
}

pub fn certificate_pdf(cfg: &Config, record: &Request, _story: Option<&Story>) -> Result<Vec<u8>, String> {
    let (doc, page1, layer1) = PdfDocument::new(
        &format!("IP Experience Certificate {}", record.id),
        Mm(210.0),
        Mm(297.0),
        "certificate",
    );
    let layer = doc.get_page(page1).get_layer(layer1);

    let font_path = resolve_font(cfg);
    let cjk = font_path.is_some();
    // TTC → TTF：让 printpdf 的字体子集化真正生效，证书从 ~20MB 降到几十 KB
    let cache_dir = cfg
        .data_path
        .parent()
        .map(|p| p.join("fonts"))
        .unwrap_or_else(|| std::path::PathBuf::from("data/fonts"));
    let ttf_path = font_path
        .as_ref()
        .map(|p| crate::fontpack::prepare_ttf(Path::new(p), &cache_dir));
    let font = match &ttf_path {
        Some(Some(p)) => doc
            .add_external_font_with_subsetting(
                File::open(p).map_err(|e| format!("字体打开失败: {}", e))?,
                true,
            )
            .map_err(|e| format!("字体载入失败: {}", e))?,
        _ => doc
            .add_builtin_font(BuiltinFont::Helvetica)
            .map_err(|e| format!("内置字体载入失败: {}", e))?,
    };
    let cjk = cjk && ttf_path.as_ref().map(|p| p.is_some()).unwrap_or(false);

    let (title, subtitle, holder_l, email_l, scope_l, verify_l, fp_l, expiry_l, footer, signer) = if cjk {
        (
            "IP 体验权证书",
            "从 TXT 文本到动物动画 · 非独占 · 免费体验",
            "持有人 / HOLDER",
            "电子邮箱",
            "授权范围 / SCOPE",
            "在线校验",
            "内容指纹",
            "体验期至",
            "本证书由 wayhug.com 自动生成 · 编号可在线校验 · 不可转让",
            format!("授权人：{}（wayhug.com）", cfg.owner_name),
        )
    } else {
        (
            "IP EXPERIENCE CERTIFICATE",
            "From TXT story to animal animation · Non-exclusive · Free trial",
            "HOLDER",
            "Email",
            "SCOPE OF LICENSE",
            "Verify",
            "Fingerprint",
            "Free trial until",
            "Generated by wayhug.com · verifiable by certificate number · non-transferable",
            format!("Licensor: {} (wayhug.com)", cfg.owner_name),
        )
    };

    // 顶部 / 底部六色带
    let bw = 210.0 / BANDS.len() as f32;
    for (i, c) in BANDS.iter().enumerate() {
        layer.set_fill_color(rgb(c));
        layer.add_rect(Rect::new(Mm(i as f32 * bw), Mm(292.0), Mm((i + 1) as f32 * bw), Mm(297.0)));
    }

    // 双线边框
    layer.set_fill_color(Color::Rgb(Rgb::new(1.0, 1.0, 1.0, None)));
    layer.set_outline_color(rgb("#e2e2ea"));
    layer.set_outline_thickness(0.6);
    layer.add_rect(Rect::new(Mm(8.0), Mm(10.0), Mm(202.0), Mm(289.0)));
    layer.set_outline_color(rgb("#f0f0f4"));
    layer.set_outline_thickness(0.3);
    layer.add_rect(Rect::new(Mm(10.0), Mm(12.0), Mm(200.0), Mm(287.0)));

    // 头部
    text_center(&layer, &font, "WAYHUG.COM · IP EXPERIENCE CERTIFICATE", 7.5, 18.0, MUTED);
    text_center(&layer, &font, title, if cjk { 27.0 } else { 23.0 }, 25.0, INK);
    text_center(&layer, &font, subtitle, 9.5, 38.0, "#6d6d78");
    layer.set_fill_color(rgb(ACCENT));
    layer.add_rect(Rect::new(Mm(90.0), Mm(292.0 - 45.0 - 1.4), Mm(120.0), Mm(292.0 - 45.0)));

    // 编号 / 日期
    text(&layer, &font, &format!("证书编号  {}", record.id), 8.5, 14.0, 50.0, MUTED);
    text_right(
        &layer,
        &font,
        &format!("签发  {}    SOURCE  {}", fmt_date(&record.created_at), record.source.to_uppercase()),
        8.5,
        196.0,
        50.0,
        MUTED,
    );

    // 作品卡
    layer.set_fill_color(rgb(PAPER));
    layer.add_rect(Rect::new(Mm(14.0), Mm(292.0 - 104.0), Mm(196.0), Mm(292.0 - 57.0)));
    layer.set_fill_color(rgb(ACCENT));
    layer.add_rect(Rect::new(Mm(14.0), Mm(292.0 - 104.0), Mm(17.0), Mm(292.0 - 57.0)));
    text(&layer, &font, if cjk { "作品" } else { "WORK" }, 7.5, 22.0, 61.0, MUTED);
    block(&layer, &font, &record.story_title, 17.0, 22.0, 66.0, 34.0, 7.0, INK);
    let cast = if record.cast_names.is_empty() {
        "-".to_string()
    } else {
        record.cast_names.join(" · ")
    };
    let meta = if cjk {
        format!("角色 {}    |    共 {} 幕    |    {} 字符", cast, record.scenes, record.chars)
    } else {
        format!("Cast {}    |    {} scenes    |    {} chars", cast, record.scenes, record.chars)
    };
    text(&layer, &font, &meta, 9.0, 22.0, 90.0, "#5f5f6a");
    text(
        &layer,
        &font,
        &format!("sha256 {} · wayhug/2.0-rust", &record.fingerprint[..16.min(record.fingerprint.len())]),
        7.5,
        22.0,
        97.0,
        "#a0a0aa",
    );

    // 持有人
    text(&layer, &font, holder_l, 7.5, 14.0, 111.0, MUTED);
    block(&layer, &font, &record.name, 15.0, 14.0, 116.0, 28.0, 6.0, INK);
    block(&layer, &font, &format!("{}  {}", email_l, record.email), 9.0, 14.0, 128.0, 52.0, 5.0, "#5f5f6a");
    text_right(&layer, &font, expiry_l, 7.5, 196.0, 111.0, MUTED);
    text_right(&layer, &font, &fmt_date(&record.expires_at), 12.0, 196.0, 116.0, INK);

    // 授权范围
    layer.set_fill_color(Color::Rgb(Rgb::new(1.0, 1.0, 1.0, None)));
    layer.set_outline_color(rgb(LINE));
    layer.set_outline_thickness(0.4);
    layer.add_rect(Rect::new(Mm(14.0), Mm(292.0 - 184.0), Mm(196.0), Mm(292.0 - 138.0)));
    layer.set_fill_color(rgb(GREEN));
    layer.add_rect(Rect::new(Mm(14.0), Mm(292.0 - 184.0), Mm(17.0), Mm(292.0 - 138.0)));
    text(&layer, &font, scope_l, 8.5, 22.0, 142.0, GREEN);

    let bullets = if cjk {
        vec![
            "在线观看、演示由本作品 TXT 生成的 CSS / HTML5 动画；",
            "下载动画页面用于个人学习、作品集与教学展示；",
            "本证书为免费体验授权，不构成著作权转让，不得转售或再授权；",
            "商业用途须与作者另行签署 IP 授权协议。",
        ]
    } else {
        vec![
            "View and present the CSS / HTML5 animation generated from this TXT story;",
            "Download the animation page for personal study, portfolio and teaching;",
            "This free trial license is not a copyright transfer; resale is forbidden;",
            "Commercial use requires a separate IP license agreement with the author.",
        ]
    };
    for (i, line) in bullets.iter().enumerate() {
        let top = 150.0 + i as f32 * 7.2;
        layer.set_fill_color(rgb(ACCENT));
        layer.add_rect(Rect::new(
            Mm(23.0),
            Mm(292.0 - top - 1.2),
            Mm(24.4),
            Mm(292.0 - top - 0.2),
        ));
        block(&layer, &font, line, 9.5, 27.0, top - 0.6, 48.0, 5.0, "#3c3c46");
    }

    // 校验信息
    text(&layer, &font, verify_l, 7.5, 14.0, 190.0, MUTED);
    block(
        &layer,
        &font,
        &format!("{}/verify/{}", cfg.site_url, record.id),
        9.0,
        14.0,
        195.0,
        70.0,
        5.0,
        "#3c3c46",
    );
    text(&layer, &font, fp_l, 7.5, 14.0, 203.0, MUTED);
    text(&layer, &font, &format!("sha256:{}", record.fingerprint), 8.0, 14.0, 208.0, "#3c3c46");

    // 签名
    layer.set_outline_color(rgb("#c9c9d4"));
    layer.set_outline_thickness(0.4);
    layer.add_line(Line {
        points: vec![
            (Point::new(Mm(14.0), Mm(292.0 - 217.0)), false),
            (Point::new(Mm(84.0), Mm(292.0 - 217.0)), false),
        ],
        is_closed: false,
    });
    text(&layer, &font, &signer, 9.5, 14.0, 220.0, "#3c3c46");

    // 扁平印章
    layer.set_outline_color(rgb(ACCENT));
    layer.set_outline_thickness(1.2);
    layer.set_fill_color(Color::Rgb(Rgb::new(1.0, 1.0, 1.0, None)));
    layer.add_rect(Rect::new(Mm(158.0), Mm(292.0 - 240.0), Mm(194.0), Mm(292.0 - 204.0)));
    text_center(&layer, &font, "IP", 24.0, 210.0, ACCENT);
    text_center(&layer, &font, "WAYHUG · LAB", 6.5, 225.0, MUTED);
    text_center(&layer, &font, &fmt_date(&record.created_at), 6.5, 231.0, "#b0b0ba");

    // 页脚
    text_center(&layer, &font, footer, 7.5, 252.0, "#a8a8b2");

    let bytes = doc.save_to_bytes().map_err(|e| format!("PDF 保存失败: {}", e))?;
    if bytes.len() > 3 * 1024 * 1024 {
        eprintln!(
            "[cert] 警告：证书 {} 体积 {} KB，字体子集化未生效（字体 {}）",
            record.id,
            bytes.len() / 1024,
            font_path.as_deref().unwrap_or("builtin")
        );
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn wrap_never_empty() {
        let v = wrap("从 TXT 文本到动物动画", 6.0);
        assert!(!v.is_empty());
        assert!(v.iter().all(|l| !l.is_empty()));
    }

    #[test]
    fn certificate_bytes_start_with_pdf() {
        let cfg = Config::load();
        let rec = Request {
            id: "WH-20261027-TEST".into(),
            name: "测试持有人".into(),
            email: "test@example.com".into(),
            story_id: "demo".into(),
            story_title: "小狐与月亮邮局".into(),
            cast_names: vec!["小狐".into(), "墨鸮".into()],
            scenes: 8,
            chars: 451,
            fingerprint: "0123456789abcdef".into(),
            status: "pending".into(),
            source: "unit".into(),
            created_at: "2026-10-27T00:00:00Z".into(),
            expires_at: "2027-01-25T00:00:00Z".into(),
            ..Default::default()
        };
        let bytes = certificate_pdf(&cfg, &rec, None).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
    }
}
