// ============================================================
// 业务层 · 数据持久化（JSON 单文件，原子写入）
//
// 与 Node 版 data/lab.json 隔离：Rust 版使用 data/wayhug.json，
// 两者可并行运行、互不覆盖，便于灰度切换。
// ============================================================

use crate::config::Config;
use crate::model::{Db, Request, Story};
use crate::story::{parse_story, demo_text, STORY_ENGINE_VERSION};
use chrono::{Duration, SecondsFormat, Utc};
use std::fs;

pub fn load(cfg: &Config) -> Db {
    match fs::read_to_string(&cfg.data_path) {
        Ok(s) => serde_json::from_str::<Db>(&s).unwrap_or_else(|e| {
            eprintln!("[store] 数据文件解析失败（{}），使用空库: {}", cfg.data_path.display(), e);
            Db::default()
        }),
        Err(_) => Db::default(),
    }
}

pub fn save(cfg: &Config, db: &Db) -> Result<(), String> {
    if let Some(dir) = cfg.data_path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("创建数据目录失败: {}", e))?;
    }
    let tmp = cfg.data_path.with_extension("json.tmp");
    let body = serde_json::to_vec_pretty(db).map_err(|e| format!("序列化失败: {}", e))?;
    fs::write(&tmp, body).map_err(|e| format!("写入临时文件失败: {}", e))?;
    fs::rename(&tmp, &cfg.data_path).map_err(|e| format!("替换数据文件失败: {}", e))?;
    Ok(())
}

/// 演示作品：规则版本升级后自动重建，保证首页永远有可看的内容
pub fn ensure_demo(cfg: &Config, db: &mut Db) {
    let needs = match db.find_story("demo") {
        Some(s) => s.engine_version < STORY_ENGINE_VERSION,
        None => true,
    };
    if !needs {
        return;
    }
    match parse_story(demo_text(), Some("小狐与月亮邮局"), Some("demo")) {
        Ok(mut story) => {
            story.source = "demo".into();
            story.created_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
            db.put_story(story);
            if let Err(e) = save(cfg, db) {
                eprintln!("[store] 演示作品保存失败: {}", e);
            }
            println!("[store] 演示作品已生成（引擎 v{}）", STORY_ENGINE_VERSION);
        }
        Err(e) => eprintln!("[store] 演示作品解析失败: {}", e),
    }
}

pub fn today() -> String {
    Utc::now().format("%Y%m%d").to_string()
}

pub fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub fn new_request_id(db: &mut Db) -> String {
    db.counter += 1;
    // counter 保证同日递增唯一；时间纳秒与进程号做扰动，
    // 避免数据重置/多实例时出现重复编号（编号同时是证书校验键）。
    let mix = (db.counter.wrapping_mul(2654435761) & 0xffff) as u32;
    let nanos = Utc::now().timestamp_subsec_nanos();
    let entropy = nanos ^ std::process::id().wrapping_mul(2246822519);
    format!("WH-{}-{:04X}", today(), (mix ^ entropy) & 0xffff)
}

pub fn create_request(
    db: &mut Db,
    story: Option<&Story>,
    name: &str,
    email: &str,
    story_id: &str,
    message: &str,
    source: &str,
) -> Request {
    let story_title = story.map(|s| s.title.clone()).unwrap_or_else(|| "小狐与月亮邮局".into());
    let cast_names: Vec<String> = story
        .map(|s| s.cast.iter().map(|c| c.name.clone()).collect())
        .unwrap_or_else(|| vec!["小狐".into(), "墨鸮".into(), "棉棉".into(), "憨熊".into()]);
    let (scenes, chars, fingerprint) = story
        .map(|s| (s.scenes.len(), s.stats.chars, s.fingerprint.clone()))
        .unwrap_or_else(|| (8, 451, "demo-fingerprint".into()));
    let created = now_iso();
    let expires = (Utc::now() + Duration::days(90)).to_rfc3339_opts(SecondsFormat::Secs, true);

    let req = Request {
        id: new_request_id(db),
        name: name.trim().chars().take(40).collect(),
        email: email.trim().to_ascii_lowercase(),
        story_id: if story_id.is_empty() { "demo".into() } else { story_id.to_string() },
        story_title,
        cast_names,
        scenes,
        chars,
        fingerprint,
        status: "pending".into(),
        source: source.to_string(),
        created_at: created,
        expires_at: expires,
        message: message.chars().take(500).collect(),
        reply_text: String::new(),
        replied_at: String::new(),
        mail: Default::default(),
    };
    db.put_request(req.clone());
    req
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        let mut db = Db::default();
        let a = new_request_id(&mut db);
        let b = new_request_id(&mut db);
        assert_ne!(a, b);
        assert!(a.starts_with("WH-"));
    }
}
