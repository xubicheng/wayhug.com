// ============================================================
// 数据喂养层 · 素描原画导入
//
// 目标：把「素描原画」组织成可检索、可对齐四相世界观的参考数据。
// 能力：校验图片、算指纹、取主色、给出四相建议、落盘并登记元数据。
// 入口：网页后台上传（/api/admin/sketches）或命令行 `wayhug ingest-sketch`。
// ============================================================

use crate::config::Config;
use crate::model::{Db, Sketch};
use crate::store::{now_iso, save};
use crate::worldview::{Element, Phase};
use image::GenericImageView;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const MAX_BYTES: usize = 12 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub mime: &'static str,
    pub ext: &'static str,
    pub palette: Vec<String>,
    pub suggested_phase: String,
    pub suggested_element: String,
}

fn sniff_mime(bytes: &[u8]) -> Option<(&'static str, &'static str)> {
    if bytes.len() >= 8 && &bytes[0..8] == b"\x89PNG\r\n\x1a\n" {
        return Some(("image/png", "png"));
    }
    if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        return Some(("image/jpeg", "jpg"));
    }
    None
}

fn hex(r: u8, g: u8, b: u8) -> String {
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

fn dist(a: (u8, u8, u8), b: (u8, u8, u8)) -> i32 {
    let dr = a.0 as i32 - b.0 as i32;
    let dg = a.1 as i32 - b.1 as i32;
    let db = a.2 as i32 - b.2 as i32;
    dr * dr + dg * dg + db * db
}

pub fn analyze(bytes: &[u8]) -> Result<ImageInfo, String> {
    if bytes.len() > MAX_BYTES {
        return Err(format!("图片超过 {}MB 限制", MAX_BYTES / 1024 / 1024));
    }
    let Some((mime, ext)) = sniff_mime(bytes) else {
        return Err("只接受 PNG / JPEG 图片".into());
    };
    let img = image::load_from_memory(bytes).map_err(|e| format!("图片解码失败: {}", e))?;
    let (width, height) = img.dimensions();
    if width == 0 || height == 0 {
        return Err("图片尺寸无效".into());
    }
    if width > 12000 || height > 12000 {
        return Err("图片尺寸过大（上限 12000px）".into());
    }

    let small = img.resize(48, 48, image::imageops::FilterType::Triangle).to_rgb8();
    let mut buckets: HashMap<(u8, u8, u8), usize> = HashMap::new();
    let (mut sr, mut sg, mut sb, mut n) = (0u64, 0u64, 0u64, 0u64);
    for px in small.pixels() {
        let (r, g, b) = (px[0], px[1], px[2]);
        sr += r as u64;
        sg += g as u64;
        sb += b as u64;
        n += 1;
        // 量化到 5 bit/通道，聚合相近颜色
        let q = ((r >> 3) << 3, (g >> 3) << 3, (b >> 3) << 3);
        *buckets.entry(q).or_insert(0) += 1;
    }
    let mut ranked: Vec<((u8, u8, u8), usize)> = buckets.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1));

    let mut palette: Vec<String> = Vec::new();
    for (c, _) in ranked {
        if palette.len() >= 4 {
            break;
        }
        if palette.iter().all(|p| {
            let h = p.trim_start_matches('#');
            let pr = u8::from_str_radix(&h[0..2], 16).unwrap_or(0);
            let pg = u8::from_str_radix(&h[2..4], 16).unwrap_or(0);
            let pb = u8::from_str_radix(&h[4..6], 16).unwrap_or(0);
            dist(c, (pr, pg, pb)) > 1600
        }) {
            palette.push(hex(c.0, c.1, c.2));
        }
    }
    if palette.is_empty() {
        palette.push("#f5f5f7".into());
    }

    let (ar, ag, ab) = (
        (sr / n.max(1)) as u8,
        (sg / n.max(1)) as u8,
        (sb / n.max(1)) as u8,
    );
    let lum = 0.299 * ar as f32 + 0.587 * ag as f32 + 0.114 * ab as f32;
    let warm = ar as f32 - ab as f32;
    let (phase, element) = if lum >= 140.0 {
        if warm >= 0.0 {
            (Phase::Day, Element::Fire)
        } else {
            (Phase::Dawn, Element::Wind)
        }
    } else if warm >= 0.0 {
        (Phase::Dusk, Element::Water)
    } else {
        (Phase::Night, Element::Earth)
    };

    Ok(ImageInfo {
        width,
        height,
        mime,
        ext,
        palette,
        suggested_phase: phase.key().to_string(),
        suggested_element: element.key().to_string(),
    })
}

fn sha_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

fn norm_phase(s: &str) -> Option<String> {
    Phase::from_key(s.trim()).map(|p| p.key().to_string())
}

fn norm_element(s: &str) -> Option<String> {
    Element::from_key(s.trim()).map(|e| e.key().to_string())
}

/// 统一入口：网页上传与命令行都走这里
#[allow(clippy::too_many_arguments)]
pub fn ingest_bytes(
    cfg: &Config,
    db: &mut Db,
    bytes: &[u8],
    title: &str,
    phase: &str,
    element: &str,
    tags: &str,
    note: &str,
) -> Result<Sketch, String> {
    let info = analyze(bytes)?;
    let hash = sha_hex(bytes);
    let id = format!("sk_{}", &hash[..10]);
    if let Some(existing) = db.find_sketch(&id) {
        return Ok(existing.clone());
    }

    fs::create_dir_all(&cfg.sketch_dir).map_err(|e| format!("创建原画目录失败: {}", e))?;
    let file_name = format!("{}.{}", id, info.ext);
    let file_path = cfg.sketch_dir.join(&file_name);
    fs::write(&file_path, bytes).map_err(|e| format!("写入原画失败: {}", e))?;

    let phase = norm_phase(phase).unwrap_or_else(|| info.suggested_phase.clone());
    let element = norm_element(element).unwrap_or_else(|| info.suggested_element.clone());

    let sketch = Sketch {
        id,
        title: if title.trim().is_empty() {
            format!("素描原画 {}", &hash[..6])
        } else {
            title.trim().chars().take(60).collect()
        },
        phase,
        element,
        tags: tags
            .split(&[',', '，', ' '][..])
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        note: note.trim().chars().take(300).collect(),
        file: file_name,
        mime: info.mime.to_string(),
        bytes: bytes.len(),
        sha256: hash,
        width: info.width,
        height: info.height,
        palette: info.palette,
        suggested_phase: info.suggested_phase,
        suggested_element: info.suggested_element,
        created_at: now_iso(),
    };
    db.put_sketch(sketch.clone());
    save(cfg, db)?;
    Ok(sketch)
}

/// 命令行：把单张图喂进原画库
pub fn cli_ingest(cfg: &Config, path: &Path, title: &str, phase: &str, element: &str) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|e| format!("读取失败 {}: {}", path.display(), e))?;
    let mut db = crate::store::load(cfg);
    let sketch = ingest_bytes(cfg, &mut db, &bytes, title, phase, element, "", "")?;
    println!(
        "已导入：{}  {}x{}  {}/{}  主色 {}",
        sketch.id,
        sketch.width,
        sketch.height,
        Phase::from_key(&sketch.phase).map(|p| p.name()).unwrap_or("晨"),
        Element::from_key(&sketch.element).map(|e| e.name()).unwrap_or("风"),
        sketch.palette.join(" ")
    );
    Ok(())
}

/// 命令行：批量喂养一个目录（PNG / JPEG），按文件名排序
pub fn cli_ingest_dir(cfg: &Config, dir: &Path) -> Result<(), String> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(|e| format!("目录不可读 {}: {}", dir.display(), e))?
        .filter_map(|e| e.ok().map(|x| x.path()))
        .filter(|p| {
            p.extension()
                .and_then(|x| x.to_str())
                .map(|x| matches!(x.to_ascii_lowercase().as_str(), "png" | "jpg" | "jpeg"))
                .unwrap_or(false)
        })
        .collect();
    files.sort();
    println!("发现 {} 张原画，开始喂养…", files.len());
    let mut ok = 0;
    for p in &files {
        let title = p.file_stem().and_then(|s| s.to_str()).unwrap_or("素描原画");
        match cli_ingest(cfg, p, title, "", "") {
            Ok(()) => ok += 1,
            Err(e) => eprintln!("跳过 {}: {}", p.display(), e),
        }
    }
    println!("完成：成功 {} / 共 {}", ok, files.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_image() {
        assert!(analyze(b"not an image").is_err());
    }

    #[test]
    fn detects_png_magic() {
        assert_eq!(sniff_mime(b"\x89PNG\r\n\x1a\nxxxx").unwrap().1, "png");
    }
}
