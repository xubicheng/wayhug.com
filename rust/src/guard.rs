// ============================================================
// 内容边界 · 宗教 / 政治
//
// 本站只做故事、动物与动画：文稿可以写情绪、关系、四季与试炼，
// 但不收录涉及宗教与政治的内容。这里是确定性的关键词闸门，
// 网页生成、邮件投稿、命令行文稿导入共用同一套规则。
// ============================================================

const RELIGION: &[&str] = &[
    "宗教", "佛教", "佛经", "佛陀", "菩萨", "观音", "寺庙", "寺院", "和尚", "僧人",
    "方丈", "住持", "禅宗", "道教", "道士", "道观", "太上老君", "基督教", "基督",
    "耶稣", "圣经", "教堂", "神父", "牧师", "弥撒", "洗礼", "伊斯兰", "清真",
    "古兰经", "穆斯林", "真主", "安拉", "印度教", "轮回", "转世", "祈祷", "祷告",
    "礼拜", "香火", "祭祀", "朝圣", "斋戒", "传教", "布道", "邪教", "法轮",
    "天主教", "东正教", "犹太教", "宗教仪式", "信仰",
    "religion", "religious", "church", "christian", "jesus", "bible", "buddha",
    "buddhism", "temple", "monk", "mosque", "islam", "muslim", "quran", "allah",
    "pope", "priest", "pray", "prayer", "worship",
];

const POLITICS: &[&str] = &[
    "政治", "政府", "政党", "共产党", "国民党", "选举", "总统", "主席", "首相",
    "总理", "议会", "人大", "政协", "国务院", "政治局", "中南海", "领导人",
    "政权", "政变", "革命", "示威", "游行", "罢工", "外交", "政策", "民主运动",
    "专制", "独裁", "民运", "台独", "港独", "疆独", "藏独", "六四", "文革",
    "法轮功", "国家主席", "台湾独立", "香港独立", "政治宣传",
    "politics", "political", "government", "communist", "election", "president",
    "parliament", "senator", "dictator", "protest", "propaganda",
];

/// 命中返回 Some((类别, 关键词))；未命中返回 None
pub fn blocked(text: &str) -> Option<(&'static str, &'static str)> {
    let lower = text.to_lowercase();
    if let Some(w) = RELIGION.iter().find(|w| lower.contains(&w.to_lowercase())) {
        return Some(("宗教", w));
    }
    if let Some(w) = POLITICS.iter().find(|w| lower.contains(&w.to_lowercase())) {
        return Some(("政治", w));
    }
    None
}

/// 统一的内容闸门：通过返回 Ok，命中返回可直接展示给访客的说明
pub fn check(text: &str) -> Result<(), String> {
    match blocked(text) {
        Some((cat, word)) => Err(format!(
            "内容规范：暂不收录涉及{}的文本（命中「{}」）。本站只做故事、动物与动画。",
            cat, word
        )),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_plain_story() {
        assert!(check("清晨，小狐在森林里捡到一封没有地址的信。").is_ok());
    }

    #[test]
    fn blocks_religion() {
        assert!(check("他在寺庙里为所有人祈祷。").is_err());
    }

    #[test]
    fn blocks_politics() {
        assert!(check("小镇举行了一场选举。").is_err());
    }
}
