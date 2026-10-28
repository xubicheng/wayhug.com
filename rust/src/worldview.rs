// ============================================================
// 引擎层 · 四相世界观（田字四分法）
//
// 田字四象限 = 四相 × 四属性：
//   左上 晨 · 风   （相遇 / 轻快 / 漂移）
//   右上 昼 · 火   （同行 / 明亮 / 上升）
//   左下 暮 · 水   （试炼 / 低回 / 下落）
//   右下 夜 · 土   （归处 / 安静 / 静止）
//
// 四相描述「光与时间」，四属性描述「动力与材质」。
// 本模块只做确定性分类，不依赖模型；后续可替换为更强的语义引擎。
// ============================================================

use crate::model::Story;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Dawn,
    Day,
    Dusk,
    Night,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Element {
    Wind,
    Fire,
    Water,
    Earth,
}

impl Phase {
    pub fn key(&self) -> &'static str {
        match self {
            Phase::Dawn => "dawn",
            Phase::Day => "day",
            Phase::Dusk => "dusk",
            Phase::Night => "night",
        }
    }
    pub fn name(&self) -> &'static str {
        match self {
            Phase::Dawn => "晨",
            Phase::Day => "昼",
            Phase::Dusk => "暮",
            Phase::Night => "夜",
        }
    }
    pub fn from_key(k: &str) -> Option<Self> {
        match k {
            "dawn" | "晨" => Some(Phase::Dawn),
            "day" | "昼" => Some(Phase::Day),
            "dusk" | "暮" => Some(Phase::Dusk),
            "night" | "夜" => Some(Phase::Night),
            _ => None,
        }
    }
    /// 四相与叙事的对应：起、承、转、合
    pub fn beat(&self) -> &'static str {
        match self {
            Phase::Dawn => "起 · 相遇",
            Phase::Day => "承 · 同行",
            Phase::Dusk => "转 · 试炼",
            Phase::Night => "合 · 归处",
        }
    }
    pub fn color(&self) -> &'static str {
        match self {
            Phase::Dawn => "#e8a33d",
            Phase::Day => "#ff7e47",
            Phase::Dusk => "#5d8aa8",
            Phase::Night => "#6b5b95",
        }
    }
}

impl Element {
    pub fn key(&self) -> &'static str {
        match self {
            Element::Wind => "wind",
            Element::Fire => "fire",
            Element::Water => "water",
            Element::Earth => "earth",
        }
    }
    pub fn name(&self) -> &'static str {
        match self {
            Element::Wind => "风",
            Element::Fire => "火",
            Element::Water => "水",
            Element::Earth => "土",
        }
    }
    pub fn from_key(k: &str) -> Option<Self> {
        match k {
            "wind" | "风" => Some(Element::Wind),
            "fire" | "火" => Some(Element::Fire),
            "water" | "水" => Some(Element::Water),
            "earth" | "土" => Some(Element::Earth),
            _ => None,
        }
    }
    pub fn motion(&self) -> &'static str {
        match self {
            Element::Wind => "漂移",
            Element::Fire => "上升",
            Element::Water => "下落",
            Element::Earth => "静止",
        }
    }
    pub fn color(&self) -> &'static str {
        match self {
            Element::Wind => "#ffb703",
            Element::Fire => "#ff7e47",
            Element::Water => "#5d8aa8",
            Element::Earth => "#8b6f47",
        }
    }
}

/// 四相与四属性的固定配对（田字）：晨风、昼火、暮水、夜土
pub fn paired_element(phase: Phase) -> Element {
    match phase {
        Phase::Dawn => Element::Wind,
        Phase::Day => Element::Fire,
        Phase::Dusk => Element::Water,
        Phase::Night => Element::Earth,
    }
}

pub fn phase_palette(phase: Phase) -> (&'static str, &'static str) {
    match phase {
        Phase::Dawn => ("#fff7e6", "#2d2a26"),
        Phase::Day => ("#fff0e6", "#33251f"),
        Phase::Dusk => ("#eaf2f7", "#22313d"),
        Phase::Night => ("#edeefa", "#232538"),
    }
}

/// 角色与四相的天然亲和（用于没有时间线索时的兜底）
pub fn animal_affinity(animal: &str) -> (Phase, Element) {
    match animal {
        "fox" => (Phase::Day, Element::Fire),
        "owl" => (Phase::Night, Element::Wind),
        "rabbit" => (Phase::Dawn, Element::Earth),
        "bear" => (Phase::Dusk, Element::Earth),
        "cat" => (Phase::Night, Element::Water),
        _ => (Phase::Dawn, Element::Wind),
    }
}

const PHASE_WORDS: &[(Phase, &[&str])] = &[
    (
        Phase::Dawn,
        &["晨", "清晨", "拂晓", "曙光", "日出", "醒来", "早晨", "曦"],
    ),
    (
        Phase::Day,
        &["正午", "中午", "白昼", "阳光", "太阳", "晴", "午", "日头"],
    ),
    (
        Phase::Dusk,
        &["黄昏", "傍晚", "暮", "夕", "晚霞", "日落", "斜阳"],
    ),
    (
        Phase::Night,
        &["夜", "月", "星", "深夜", "梦", "灯", "黑暗", "午夜"],
    ),
];

const ELEMENT_WORDS: &[(Element, &[&str])] = &[
    (Element::Wind, &["风", "云", "飞", "飘", "呼吸", "轻", "翼"]),
    (Element::Fire, &["火", "暖", "光", "热", "燃", "灯", "亮"]),
    (Element::Water, &["水", "雨", "河", "海", "泪", "雪", "溪", "湖"]),
    (Element::Earth, &["土", "山", "石", "树", "林", "根", "屋", "路"]),
];

fn score_words(text: &str, words: &[&str]) -> usize {
    words.iter().map(|w| text.matches(w).count()).sum()
}

/// 四相判定：关键词得分最高者；无命中返回 None
pub fn detect_phase(text: &str) -> Option<Phase> {
    let mut best: Option<(Phase, usize)> = None;
    for (phase, words) in PHASE_WORDS {
        let score = score_words(text, words);
        if score > 0 && best.map(|(_, b)| score > b).unwrap_or(true) {
            best = Some((*phase, score));
        }
    }
    best.map(|(p, _)| p)
}

/// 四属性判定：关键词得分最高者；无命中返回 None
pub fn detect_element(text: &str) -> Option<Element> {
    let mut best: Option<(Element, usize)> = None;
    for (el, words) in ELEMENT_WORDS {
        let score = score_words(text, words);
        if score > 0 && best.map(|(_, b)| score > b).unwrap_or(true) {
            best = Some((*el, score));
        }
    }
    best.map(|(e, _)| e)
}

/// 情绪 → 四相的兜底映射（保证每一幕都有世界观坐标）
pub fn phase_from_mood(mood: &str) -> Phase {
    match mood {
        "joyful" => Phase::Day,
        "surprised" => Phase::Day,
        "nostalgic" => Phase::Dusk,
        "thoughtful" => Phase::Dusk,
        "melancholy" => Phase::Night,
        _ => Phase::Dawn,
    }
}

/// 综合判定：优先文本线索，其次情绪/角色亲和，最后默认晨·风
pub fn classify(text: &str, animal: &str, mood: &str) -> (Phase, Element) {
    let phase = detect_phase(text)
        .or_else(|| {
            // 文本没有时间线索时，看角色亲和是否有更强的时间意图
            let (ap, _) = animal_affinity(animal);
            if detect_element(text).is_some() {
                Some(ap)
            } else {
                None
            }
        })
        .unwrap_or_else(|| phase_from_mood(mood));
    let element = detect_element(text).unwrap_or_else(|| paired_element(phase));
    (phase, element)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuadrantInfo {
    pub phase: &'static str,
    pub phase_name: &'static str,
    pub element: &'static str,
    pub element_name: &'static str,
    pub beat: &'static str,
    pub motion: &'static str,
    pub bg: &'static str,
    pub ink: &'static str,
    pub accent: &'static str,
    pub element_color: &'static str,
    pub animals: &'static [&'static str],
}

/// 供页面与 API 使用的四相公开视图
pub fn public_view() -> serde_json::Value {
    const ANIMALS: [&[&str]; 4] = [&["rabbit"], &["fox"], &["bear"], &["owl", "cat"]];
    let quads = [
        Phase::Dawn,
        Phase::Day,
        Phase::Dusk,
        Phase::Night,
    ]
    .iter()
    .enumerate()
    .map(|(i, phase)| {
        let element = paired_element(*phase);
        let (bg, ink) = phase_palette(*phase);
        QuadrantInfo {
            phase: phase.key(),
            phase_name: phase.name(),
            element: element.key(),
            element_name: element.name(),
            beat: phase.beat(),
            motion: element.motion(),
            bg,
            ink,
            accent: phase.color(),
            element_color: element.color(),
            animals: ANIMALS[i],
        }
    })
    .collect::<Vec<_>>();
    serde_json::json!({
        "version": "worldview/0.1",
        "layout": "田字四象限",
        "axes": { "phase": "四相 · 光与时间", "element": "四属性 · 动力与材质" },
        "quadrants": quads,
        "rules": [
            "每一幕先判定四相，再判定四属性；四属性缺省时取四相的固定配对",
            "四相同样构成叙事四拍：起（晨）· 承（昼）· 转（暮）· 合（夜）",
            "扁平化色板只使用四相底色与情绪重音色，禁止渐变与位图"
        ]
    })
}

pub const PHASES: [Phase; 4] = [Phase::Dawn, Phase::Day, Phase::Dusk, Phase::Night];
pub const ELEMENTS: [Element; 4] = [Element::Wind, Element::Fire, Element::Water, Element::Earth];

/// 文稿集合 → 世界观校准数据
///
/// 四相卡片的占比不是写死的，而是每次都由已收录文稿逐幕统计得出：
/// 新文稿导入后，首页与 /api/corpus 会立即反映新的分布。
pub fn corpus_view(stories: &[Story]) -> serde_json::Value {
    let manuscripts: Vec<&Story> = stories.iter().filter(|s| s.source == "manuscript").collect();
    let has_manuscripts = !manuscripts.is_empty();
    let pool: Vec<&Story> = if has_manuscripts {
        manuscripts
    } else {
        stories.iter().collect()
    };

    let mut phase_scenes: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut element_scenes: BTreeMap<&'static str, usize> = BTreeMap::new();
    let (mut scenes, mut chars, mut cast) = (0usize, 0usize, 0usize);
    for s in &pool {
        scenes += s.stats.scenes;
        chars += s.stats.chars;
        cast += s.cast.len();
        for sc in &s.scenes {
            let key = Phase::from_key(&sc.phase).unwrap_or(Phase::Dawn).key();
            *phase_scenes.entry(key).or_insert(0) += 1;
            let el = Element::from_key(&sc.element)
                .unwrap_or_else(|| paired_element(Phase::from_key(&sc.phase).unwrap_or(Phase::Dawn)))
                .key();
            *element_scenes.entry(el).or_insert(0) += 1;
        }
    }

    let share = |n: usize| -> u32 {
        if scenes == 0 {
            0
        } else {
            ((n as f64 / scenes as f64) * 100.0).round() as u32
        }
    };
    let phases: Vec<serde_json::Value> = PHASES
        .iter()
        .map(|p| {
            let n = *phase_scenes.get(p.key()).unwrap_or(&0);
            serde_json::json!({
                "phase": p.key(),
                "phaseName": p.name(),
                "beat": p.beat(),
                "accent": p.color(),
                "scenes": n,
                "share": share(n)
            })
        })
        .collect();
    let elements: Vec<serde_json::Value> = ELEMENTS
        .iter()
        .map(|e| {
            let n = *element_scenes.get(e.key()).unwrap_or(&0);
            serde_json::json!({
                "element": e.key(),
                "elementName": e.name(),
                "motion": e.motion(),
                "color": e.color(),
                "scenes": n,
                "share": share(n)
            })
        })
        .collect();
    let latest = pool
        .iter()
        .max_by(|a, b| a.created_at.cmp(&b.created_at))
        .map(|s| s.title.clone())
        .unwrap_or_default();

    serde_json::json!({
        "source": if has_manuscripts { "manuscript" } else { "all" },
        "manuscripts": pool.len(),
        "scenes": scenes,
        "chars": chars,
        "cast": cast,
        "latest": latest,
        "phases": phases,
        "elements": elements
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_and_element() {
        assert_eq!(detect_phase("夜里，月亮很亮"), Some(Phase::Night));
        assert_eq!(detect_phase("清晨的风吹过"), Some(Phase::Dawn));
        assert_eq!(detect_element("雨落在河面上"), Some(Element::Water));
        let (p, e) = classify("夜里，墨鸮落在屋檐上", "owl", "calm");
        assert_eq!(p, Phase::Night);
        assert_eq!(e, Element::Earth); // 无元素词时取夜·土
    }

    #[test]
    fn mood_fallback() {
        assert_eq!(phase_from_mood("joyful"), Phase::Day);
        assert_eq!(phase_from_mood("melancholy"), Phase::Night);
        assert_eq!(paired_element(Phase::Dawn), Element::Wind);
    }

    #[test]
    fn corpus_counts_scenes_by_phase() {
        let mut a = Story {
            id: "st_a".into(),
            source: "manuscript".into(),
            created_at: "2026-10-28T01:00:00Z".into(),
            ..Default::default()
        };
        a.stats = crate::model::Stats { scenes: 2, chars: 40, ..Default::default() };
        a.scenes = vec![
            crate::model::Scene { index: 0, phase: "dawn".into(), element: "wind".into(), ..Default::default() },
            crate::model::Scene { index: 1, phase: "night".into(), element: "".into(), ..Default::default() },
        ];
        let mut b = Story { id: "st_b".into(), source: "web".into(), ..Default::default() };
        b.stats = crate::model::Stats { scenes: 1, chars: 10, ..Default::default() };
        b.scenes = vec![crate::model::Scene { index: 0, phase: "day".into(), element: "fire".into(), ..Default::default() }];

        let v = corpus_view(&[a, b]);
        assert_eq!(v["source"], "manuscript");
        assert_eq!(v["manuscripts"], 1);
        assert_eq!(v["scenes"], 2);
        let phases = v["phases"].as_array().unwrap();
        assert_eq!(phases[0]["scenes"], 1); // 晨
        assert_eq!(phases[3]["scenes"], 1); // 夜
        let elements = v["elements"].as_array().unwrap();
        assert_eq!(elements[3]["scenes"], 1); // 夜无属性词 → 土
        assert_eq!(v["phases"][0]["share"], 50);
    }
}
