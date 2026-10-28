// ============================================================
// 内核层 · TXT 解析器（唯一事实源）
//
// 纯文本 → 标题 / 段落 / 幕 / 角色 / 情绪 / 四相坐标 / 扁平色板。
// JSON、CSS、HTML5、A4 证书全部由这里派生，保证四个方案同源同貌。
// ============================================================

use crate::model::{CastMember, Palette, Scene, Stats, Story};
use crate::worldview;
use chrono::{SecondsFormat, Utc};
use sha2::{Digest, Sha256};

pub const STORY_ENGINE_VERSION: u32 = 3;

pub struct Animal {
    pub key: &'static str,
    pub name: &'static str,
    pub color: &'static str,
    pub keywords: &'static [&'static str],
}

pub struct Mood {
    pub key: &'static str,
    pub name: &'static str,
    pub accent: &'static str,
    pub keywords: &'static [&'static str],
}

pub const ANIMALS: &[Animal] = &[
    Animal { key: "fox", name: "小狐", color: "#ff7e47", keywords: &["小狐", "狐狸", "狐", "狸", "fox"] },
    Animal { key: "owl", name: "墨鸮", color: "#6b5b95", keywords: &["墨鸮", "鸮", "猫头鹰", "owl"] },
    Animal { key: "rabbit", name: "棉棉", color: "#ffb6c1", keywords: &["棉棉", "兔", "rabbit", "bunny"] },
    Animal { key: "bear", name: "憨熊", color: "#8b6f47", keywords: &["憨熊", "熊", "bear"] },
    Animal { key: "cat", name: "喵先生", color: "#708090", keywords: &["喵先生", "猫", "喵", "cat"] },
];

pub const MOODS: &[Mood] = &[
    Mood { key: "joyful", name: "明快", accent: "#ff9f43", keywords: &["笑", "喜", "暖", "阳光", "欢", "跳", "唱", "甜", "春"] },
    Mood { key: "nostalgic", name: "怀旧", accent: "#6b5b95", keywords: &["曾", "旧", "回忆", "记得", "很久", "童年", "故"] },
    Mood { key: "thoughtful", name: "沉思", accent: "#2f9e77", keywords: &["想", "静", "问", "为什么", "也许", "忽然明白", "沉默"] },
    Mood { key: "surprised", name: "惊讶", accent: "#e0567a", keywords: &["突然", "忽然", "竟然", "惊", "没想到", "！"] },
    Mood { key: "melancholy", name: "淡淡忧伤", accent: "#5d8aa8", keywords: &["雨", "冷", "别离", "告别", "孤单", "泪", "风"] },
    Mood { key: "calm", name: "平静", accent: "#708090", keywords: &[] },
];

pub fn animal(key: &str) -> &'static Animal {
    ANIMALS.iter().find(|a| a.key == key).unwrap_or(&ANIMALS[0])
}

pub fn mood(key: &str) -> &'static Mood {
    MOODS.iter().find(|m| m.key == key).unwrap_or(&MOODS[5])
}

pub fn demo_text() -> &'static str {
    include_str!("../../stories/demo.txt")
}

pub fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last_space = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                out.push('\n');
                last_space = false;
            }
            '\u{00a0}' | '\t' | ' ' => {
                if !last_space {
                    out.push(' ');
                    last_space = true;
                }
            }
            '\n' => {
                out.push('\n');
                last_space = false;
            }
            _ => {
                out.push(c);
                last_space = false;
            }
        }
    }
    out.trim().to_string()
}

/// 拆句：保留句末标点，过滤空句
pub fn split_sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    for c in normalize(text).chars() {
        buf.push(c);
        if matches!(c, '。' | '！' | '？' | '!' | '?' | '；' | ';' | '\n') {
            let s = buf.trim();
            if !s.is_empty() {
                out.push(s.to_string());
            }
            buf.clear();
        }
    }
    let s = buf.trim();
    if !s.is_empty() {
        out.push(s.to_string());
    }
    out
}

/// 识别一段文本里的动物：按出现次数降序、首次出现位置升序
fn detect_cast(text: &str) -> Vec<&'static Animal> {
    let mut hits: Vec<(&'static Animal, usize, usize)> = ANIMALS
        .iter()
        .filter_map(|a| {
            let mut count = 0usize;
            let mut first = usize::MAX;
            for k in a.keywords {
                let mut from = 0usize;
                while let Some(pos) = text[from..].find(k) {
                    let at = from + pos;
                    count += 1;
                    first = first.min(at);
                    from = at + k.len();
                }
            }
            if count > 0 {
                Some((a, count, first))
            } else {
                None
            }
        })
        .collect();
    hits.sort_by(|a, b| b.1.cmp(&a.1).then(a.2.cmp(&b.2)));
    hits.into_iter().map(|(a, _, _)| a).collect()
}

fn detect_mood(text: &str) -> &'static str {
    let mut best = "calm";
    let mut best_score = 0usize;
    for m in MOODS {
        let score = m.keywords.iter().map(|k| text.matches(k).count()).sum::<usize>();
        if score > best_score {
            best = m.key;
            best_score = score;
        }
    }
    best
}

/// 把段落重排为「一幕一画面」，单幕不超过 max_chars（按字符数）
fn to_scenes(paragraphs: &[String], max_chars: usize) -> Vec<String> {
    let mut scenes = Vec::new();
    for para in paragraphs {
        if para.chars().count() <= max_chars {
            scenes.push(para.clone());
            continue;
        }
        let mut buf = String::new();
        for sentence in split_sentences(para) {
            let buf_len = buf.chars().count();
            let sen_len = sentence.chars().count();
            if !buf.is_empty() && buf_len + sen_len > max_chars {
                scenes.push(std::mem::take(&mut buf));
            }
            buf.push_str(&sentence);
            if buf.chars().count() >= max_chars {
                scenes.push(std::mem::take(&mut buf));
            }
        }
        if !buf.is_empty() {
            scenes.push(buf);
        }
    }
    scenes.truncate(24);
    scenes
}

fn paragraphs(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur: Vec<String> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            if !cur.is_empty() {
                out.push(cur.join("\n"));
                cur.clear();
            }
        } else {
            cur.push(line.to_string());
        }
    }
    if !cur.is_empty() {
        out.push(cur.join("\n"));
    }
    out
}

pub fn parse_story(txt: &str, title: Option<&str>, id: Option<&str>) -> Result<Story, String> {
    let text = normalize(txt);
    if text.is_empty() {
        return Err("故事内容为空".into());
    }
    let mut paras = paragraphs(&text);
    if paras.is_empty() {
        return Err("故事内容为空".into());
    }

    let mut title = title.unwrap_or("").trim().to_string();
    let had_title = !title.is_empty();
    let mut body = paras.clone();
    if title.is_empty() {
        let first = paras[0].clone();
        let has_end = first.chars().any(|c| matches!(c, '。' | '！' | '？' | '!' | '?'));
        if first.chars().count() <= 24 && !has_end && !first.contains('\n') {
            title = first.clone();
            body = paras.split_off(1);
        } else {
            let s = split_sentences(&first)
                .into_iter()
                .next()
                .unwrap_or_else(|| first.clone());
            let trimmed: String = s
                .trim_end_matches(|c| matches!(c, '。' | '！' | '？' | '!' | '?'))
                .chars()
                .take(16)
                .collect();
            let more = s.chars().count() > 16;
            title = format!("{}{}", trimmed, if more { "…" } else { "" });
        }
    }
    if body.is_empty() {
        body = paras;
    }
    // 显式标题与首段相同：首段只是标题行，不应算作第一幕
    if had_title && body.len() > 1 && body[0].trim() == title.trim() {
        body.remove(0);
    }
    if title.is_empty() {
        title = "未命名故事".into();
    }

    let mut cast_keys: Vec<&'static str> = Vec::new();
    for p in &body {
        for a in detect_cast(p) {
            if !cast_keys.contains(&a.key) {
                cast_keys.push(a.key);
            }
        }
    }
    if cast_keys.is_empty() {
        cast_keys.push("fox");
    }

    let scene_texts = to_scenes(&body, 96);
    if scene_texts.is_empty() {
        return Err("无法从文本中切分场景".into());
    }

    let mut last_animal = cast_keys[0];
    let mut scenes: Vec<Scene> = Vec::new();
    for (i, scene_text) in scene_texts.iter().enumerate() {
        let found = detect_cast(scene_text);
        let a = found.first().map(|x| x.key).unwrap_or(last_animal);
        last_animal = a;
        let mood_key = detect_mood(scene_text);
        let (phase, element) = worldview::classify(scene_text, a, mood_key);
        let (bg, ink) = worldview::phase_palette(phase);
        scenes.push(Scene {
            index: i,
            text: scene_text.clone(),
            animal: a.to_string(),
            mood: mood_key.to_string(),
            phase: phase.key().to_string(),
            element: element.key().to_string(),
            palette: Palette {
                bg: bg.to_string(),
                ink: ink.to_string(),
                accent: mood(mood_key).accent.to_string(),
                animal_color: animal(a).color.to_string(),
            },
            sentences: split_sentences(scene_text),
        });
    }

    let cast: Vec<CastMember> = cast_keys
        .iter()
        .map(|key| {
            let a = animal(key);
            let (ph, el) = worldview::animal_affinity(key);
            CastMember {
                id: a.key.to_string(),
                name: a.name.to_string(),
                color: a.color.to_string(),
                count: scenes.iter().filter(|s| s.animal == *key).count(),
                element: el.key().to_string(),
                phase: ph.key().to_string(),
            }
        })
        .collect();

    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let fingerprint = format!("{:x}", hasher.finalize())[..16].to_string();

    let mut mood_count = std::collections::BTreeMap::new();
    for s in &scenes {
        *mood_count.entry(s.mood.clone()).or_insert(0) += 1;
    }

    let stats = Stats {
        chars: text.chars().count(),
        sentences: split_sentences(&text).len(),
        scenes: scenes.len(),
        cast: cast.len(),
    };
    let main_mood = scenes[0].mood.clone();

    Ok(Story {
        id: id
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("st_{}", &fingerprint[..8])),
        title,
        fingerprint,
        engine_version: STORY_ENGINE_VERSION,
        created_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        stats,
        cast,
        scenes,
        mood: main_mood,
        mood_count,
        text,
        source: "txt".into(),
    })
}

/// 供 S3 下载：场景图 JSON（纯逻辑数据，不含渲染）
pub fn story_to_json(story: &Story) -> serde_json::Value {
    serde_json::json!({
        "meta": {
            "engine": "wayhug/2.0-rust",
            "title": story.title,
            "fingerprint": story.fingerprint,
            "createdAt": story.created_at,
            "license": "IP 体验权 · 非独占 · 免费试用"
        },
        "architecture": [
            { "layer": "内核层", "role": "TXT 纯文本 → 唯一事实源", "artifact": "story.json" },
            { "layer": "引擎层", "role": "四相/四属性/角色/情绪规则推理", "artifact": "scene graph" },
            { "layer": "服务层", "role": "HTTP + 邮件指令服务", "artifact": "routes / imap" },
            { "layer": "业务层", "role": "IP 体验权与交付流程", "artifact": "request / license" },
            { "layer": "交付层", "role": "CSS 动画 + A4 证书", "artifact": "html / pdf" }
        ],
        "worldview": worldview::public_view(),
        "cast": story.cast,
        "scenes": story.scenes.iter().map(|s| serde_json::json!({
            "id": s.index,
            "text": s.text,
            "animal": s.animal,
            "mood": s.mood,
            "phase": s.phase,
            "element": s.element,
            "palette": s.palette
        })).collect::<Vec<_>>(),
        "rules": [
            { "when": "文本出现「狐」", "then": "角色 = 小狐" },
            { "when": "文本出现「兔 / 棉棉」", "then": "角色 = 棉棉" },
            { "when": "文本出现「熊 / 憨熊」", "then": "角色 = 憨熊" },
            { "when": "文本出现「猫 / 喵先生」", "then": "角色 = 喵先生" },
            { "when": "文本出现「鸮 / 墨鸮」", "then": "角色 = 墨鸮" },
            { "when": "命中四相关键词最多", "then": "取该相为场景时间" },
            { "when": "命中四属性关键词最多", "then": "取该属性为场景动力" },
            { "when": "四相无命中", "then": "按情绪兜底，再按角色亲和兜底" },
            { "when": "段落超过 96 字符", "then": "按句切分为多幕（上限 24 幕）" }
        ],
        "stats": story.stats
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_parses() {
        let s = parse_story(demo_text(), Some("小狐与月亮邮局"), Some("demo")).unwrap();
        assert_eq!(s.scenes.len(), 8);
        assert_eq!(s.cast.len(), 4);
        assert_eq!(s.stats.chars, 451);
        assert!(s.cast.iter().any(|c| c.id == "rabbit"));
        assert!(s.scenes.iter().any(|x| x.phase == "night"));
    }

    #[test]
    fn sentences_keep_punctuation() {
        let v = split_sentences("他笑了。月亮很亮！你呢？");
        assert_eq!(v.len(), 3);
        assert_eq!(v[0], "他笑了。");
    }

    #[test]
    fn title_from_first_line() {
        let s = parse_story("月亮邮局\n\n小狐走进森林。", None, None).unwrap();
        assert_eq!(s.title, "月亮邮局");
        assert_eq!(s.scenes.len(), 1);
    }
}
