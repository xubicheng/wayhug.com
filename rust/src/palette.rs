// ============================================================
// 交付层 · 256 色调色板（HTML5 画布专用）
//
// 固定 256 色，不随文稿变化：
//   0..215   6×6×6 色立方（0 / 51 / 102 / 153 / 204 / 255 六个档位）
//   216..255 40 级灰阶
//
// 所有画布颜色都先 snap 到这张表，保证「色泽简单、可预算、无渐变」。
// ============================================================

/// 六个色阶档位（web-safe 的 6 级）
pub const LEVELS: [u8; 6] = [0, 51, 102, 153, 204, 255];

/// 调色板颜色总数
pub const SIZE: usize = 256;

/// 取第 i 个颜色（0..255）的 RGB
pub fn rgb(index: u8) -> (u8, u8, u8) {
    let i = index as usize;
    if i < 216 {
        (LEVELS[i / 36], LEVELS[(i % 36) / 6], LEVELS[i % 6])
    } else {
        let level = ((i - 216) * 255 / 39) as u8;
        (level, level, level)
    }
}

/// 取第 i 个颜色的 #rrggbb
pub fn hex(index: u8) -> String {
    let (r, g, b) = rgb(index);
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

/// 找最接近的调色板索引（确定性：距离相同时取较小索引）
pub fn nearest(r: u8, g: u8, b: u8) -> u8 {
    let mut best = (0u8, u32::MAX);
    for i in 0..SIZE {
        let (pr, pg, pb) = rgb(i as u8);
        let dr = r as i32 - pr as i32;
        let dg = g as i32 - pg as i32;
        let db = b as i32 - pb as i32;
        let d = (dr * dr + dg * dg + db * db) as u32;
        if d < best.1 {
            best = (i as u8, d);
        }
    }
    best.0
}

/// 把 #rgb / #rrggbb 贴到调色板上；解析失败返回 0（黑）
pub fn snap(text: &str) -> u8 {
    let s = text.trim().trim_start_matches('#');
    let parse = |t: &str| u8::from_str_radix(t, 16).ok();
    match s.len() {
        6 => {
            let (r, g, b) = (parse(&s[0..2]), parse(&s[2..4]), parse(&s[4..6]));
            match (r, g, b) {
                (Some(r), Some(g), Some(b)) => nearest(r, g, b),
                _ => 0,
            }
        }
        3 => {
            let (r, g, b) = (parse(&s[0..1]), parse(&s[1..2]), parse(&s[2..3]));
            match (r, g, b) {
                (Some(r), Some(g), Some(b)) => nearest(r * 17, g * 17, b * 17),
                _ => 0,
            }
        }
        _ => 0,
    }
}

/// 把 #rrggbb 直接贴成调色板里的 #rrggbb 字符串
pub fn snap_hex(text: &str) -> String {
    hex(snap(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_is_256_and_deterministic() {
        assert_eq!(SIZE, 256);
        assert_eq!(rgb(0), (0, 0, 0));
        assert_eq!(rgb(215), (255, 255, 255));
        assert_eq!(hex(216), "#000000");
        assert_eq!(hex(255), "#ffffff");
        let a = nearest(255, 126, 71);
        assert_eq!(a, nearest(255, 126, 71));
        assert_eq!(hex(a), "#ff6633"); // 橘色贴到色立方
    }

    #[test]
    fn snap_accepts_shorthand() {
        assert_eq!(snap("#fff"), 215);
        assert_eq!(snap("not-a-color"), 0);
    }
}
