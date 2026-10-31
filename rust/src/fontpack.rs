// ============================================================
// 交付层 · 字体准备
//
// printpdf 的字体子集化（allsorts）只认识单字体 sfnt，
// 遇到 TTC 字体集合会在解析阶段失败并静默回退为「整字体嵌入」，
// 使 A4 证书从几十 KB 膨胀到近 20 MB（无法作为邮件附件）。
//
// 这里在生成证书前把 TTC 的第 0 号字体抽取成独立 TTF 并缓存，
// 于是子集化可以真正生效。纯 std + sha2，无额外依赖。
// ============================================================

use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const TTC_TAG: &[u8; 4] = b"ttcf";

fn be_u16(b: &[u8], at: usize) -> Option<u16> {
    b.get(at..at + 2).map(|s| u16::from_be_bytes([s[0], s[1]]))
}

fn be_u32(b: &[u8], at: usize) -> Option<u32> {
    b.get(at..at + 4)
        .map(|s| u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
}

/// 从 TTC 中抽取第 `face` 号字体，重新组装为一个独立 TTF。
/// 失败返回 None（调用方回退为原字体，仍可渲染，只是体积大）。
pub fn ttc_to_ttf(bytes: &[u8], face: usize) -> Option<Vec<u8>> {
    if bytes.len() < 16 || &bytes[0..4] != TTC_TAG {
        return None;
    }
    let num_fonts = be_u32(bytes, 8)? as usize;
    if face >= num_fonts {
        return None;
    }
    let dir_off = be_u32(bytes, 12 + face * 4)? as usize;
    let sfnt_version = be_u32(bytes, dir_off)?;
    let num_tables = be_u16(bytes, dir_off + 4)? as usize;
    if num_tables == 0 || num_tables > 512 {
        return None;
    }

    struct Table {
        tag: [u8; 4],
        checksum: u32,
        offset: usize,
        len: usize,
    }

    let mut tables = Vec::with_capacity(num_tables);
    for i in 0..num_tables {
        let rec = dir_off + 12 + i * 16;
        let tag = bytes.get(rec..rec + 4)?;
        let checksum = be_u32(bytes, rec + 4)?;
        let offset = be_u32(bytes, rec + 8)? as usize;
        let len = be_u32(bytes, rec + 12)? as usize;
        let end = offset.checked_add(len)?;
        if end > bytes.len() {
            return None;
        }
        tables.push(Table {
            tag: [tag[0], tag[1], tag[2], tag[3]],
            checksum,
            offset,
            len,
        });
    }

    // sfnt 目录：12 字节头 + 16 字节/表，表数据从其后按 4 字节对齐
    let header_len = 12 + num_tables * 16;
    let data_off = (header_len + 3) & !3;
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len().min(24 * 1024 * 1024));
    out.extend_from_slice(&sfnt_version.to_be_bytes());
    out.extend_from_slice(&(num_tables as u16).to_be_bytes());

    let mut pow = 1usize;
    let mut entry_selector = 0u16;
    while pow * 2 <= num_tables {
        pow *= 2;
        entry_selector += 1;
    }
    let search_range = (pow * 16) as u16;
    out.extend_from_slice(&search_range.to_be_bytes());
    out.extend_from_slice(&entry_selector.to_be_bytes());
    out.extend_from_slice(&((num_tables * 16 - search_range as usize) as u16).to_be_bytes());

    let mut cursor = data_off;
    let mut offsets = Vec::with_capacity(num_tables);
    for t in &tables {
        out.extend_from_slice(&t.tag);
        out.extend_from_slice(&t.checksum.to_be_bytes());
        out.extend_from_slice(&(cursor as u32).to_be_bytes());
        out.extend_from_slice(&(t.len as u32).to_be_bytes());
        offsets.push(cursor);
        cursor += (t.len + 3) & !3;
    }

    for (t, off) in tables.iter().zip(offsets.iter()) {
        // 用 0 填充对齐空隙
        while out.len() < *off {
            out.push(0);
        }
        out.extend_from_slice(&bytes[t.offset..t.offset + t.len]);
        while out.len() % 4 != 0 {
            out.push(0);
        }
    }
    Some(out)
}

fn cache_key(src: &Path, len: usize) -> String {
    let mut h = Sha256::new();
    h.update(src.to_string_lossy().as_bytes());
    h.update(len.to_le_bytes());
    let digest = h.finalize();
    digest.iter().take(8).map(|b| format!("{:02x}", b)).collect()
}

/// 返回可直接交给 printpdf 子集化的单字体 TTF 路径。
/// 普通 TTF 原样返回；TTC 抽取后写入 `cache_dir` 复用。
pub fn prepare_ttf(src: &Path, cache_dir: &Path) -> Option<PathBuf> {
    let bytes = fs::read(src).ok()?;
    if bytes.len() < 4 || &bytes[0..4] != TTC_TAG {
        return Some(src.to_path_buf());
    }
    let cached = cache_dir.join(format!("cert-{}.ttf", cache_key(src, bytes.len())));
    if cached.exists() {
        return Some(cached);
    }
    let ttf = ttc_to_ttf(&bytes, 0)?;
    fs::create_dir_all(cache_dir).ok()?;
    let tmp = cache_dir.join(format!(
        "cert-{}.tmp",
        cache_key(src, bytes.len())
    ));
    fs::write(&tmp, &ttf).ok()?;
    fs::rename(&tmp, &cached).ok()?;
    Some(cached)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_ttf_passes_through() {
        let dir = std::env::temp_dir();
        let f = dir.join("wayhug-fontpack-test.ttf");
        fs::write(&f, b"\x00\x01\x00\x00abcdefgh").unwrap();
        assert_eq!(prepare_ttf(&f, &dir).unwrap(), f);
        let _ = fs::remove_file(f);
    }

    #[test]
    fn rejects_garbage() {
        assert!(ttc_to_ttf(b"not-a-font", 0).is_none());
        assert!(ttc_to_ttf(b"ttcf\x00\x01\x00\x00\x00\x00\x00\x02", 5).is_none());
    }

    #[test]
    fn extracts_first_face_from_windows_ttc() {
        let src = Path::new("C:/Windows/Fonts/msyh.ttc");
        if !src.exists() {
            return;
        }
        let out = prepare_ttf(src, &std::env::temp_dir());
        assert!(out.is_some(), "TTC 抽取失败");
        let p = out.unwrap();
        let bytes = fs::read(&p).unwrap();
        assert_eq!(&bytes[0..4], &[0x00, 0x01, 0x00, 0x00]);
        assert!(bytes.len() > 1024);
    }
}
