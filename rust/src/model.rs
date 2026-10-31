// ============================================================
// 业务层 · 数据模型（JSON 即协议：网页 / 邮件 / PDF 共用同一份事实）
// ============================================================

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub chars: usize,
    pub sentences: usize,
    pub scenes: usize,
    pub cast: usize,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Palette {
    pub bg: String,
    pub ink: String,
    pub accent: String,
    pub animal_color: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CastMember {
    pub id: String,
    pub name: String,
    pub color: String,
    pub count: usize,
    #[serde(default)]
    pub element: String,
    #[serde(default)]
    pub phase: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scene {
    pub index: usize,
    pub text: String,
    pub animal: String,
    pub mood: String,
    /// 四相：dawn / day / dusk / night
    #[serde(default)]
    pub phase: String,
    /// 四属性：wind / fire / water / earth
    #[serde(default)]
    pub element: String,
    pub palette: Palette,
    #[serde(default)]
    pub sentences: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Story {
    pub id: String,
    pub title: String,
    pub fingerprint: String,
    pub engine_version: u32,
    pub created_at: String,
    pub stats: Stats,
    pub cast: Vec<CastMember>,
    pub scenes: Vec<Scene>,
    pub mood: String,
    #[serde(default)]
    pub mood_count: std::collections::BTreeMap<String, usize>,
    pub text: String,
    #[serde(default)]
    pub source: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailResult {
    /// smtp / manual / skipped
    pub mode: String,
    pub ok: bool,
    pub reason: String,
    #[serde(default)]
    pub at: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub id: String,
    pub name: String,
    pub email: String,
    pub story_id: String,
    pub story_title: String,
    #[serde(default)]
    pub cast_names: Vec<String>,
    pub scenes: usize,
    pub chars: usize,
    pub fingerprint: String,
    /// pending / replied
    pub status: String,
    /// web / email / studio
    pub source: String,
    pub created_at: String,
    pub expires_at: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub reply_text: String,
    #[serde(default)]
    pub replied_at: String,
    #[serde(default)]
    pub mail: MailResult,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sketch {
    pub id: String,
    pub title: String,
    /// 原画可标注的四相；空表示由模块自动建议
    #[serde(default)]
    pub phase: String,
    #[serde(default)]
    pub element: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub note: String,
    pub file: String,
    pub mime: String,
    pub bytes: usize,
    pub sha256: String,
    pub width: u32,
    pub height: u32,
    pub palette: Vec<String>,
    pub suggested_phase: String,
    pub suggested_element: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Db {
    #[serde(default)]
    pub stories: Vec<Story>,
    #[serde(default)]
    pub requests: Vec<Request>,
    #[serde(default)]
    pub sketches: Vec<Sketch>,
    /// 已处理的邮件 Message-ID（去重）
    #[serde(default)]
    pub seen: Vec<String>,
    #[serde(default)]
    pub counter: u64,
}

impl Db {
    pub fn find_story(&self, id: &str) -> Option<&Story> {
        self.stories.iter().find(|s| s.id == id)
    }

    pub fn find_request(&self, id: &str) -> Option<&Request> {
        self.requests.iter().find(|r| r.id == id)
    }

    pub fn find_sketch(&self, id: &str) -> Option<&Sketch> {
        self.sketches.iter().find(|s| s.id == id)
    }

    pub fn put_story(&mut self, story: Story) {
        if let Some(slot) = self.stories.iter_mut().find(|s| s.id == story.id) {
            *slot = story;
        } else {
            self.stories.push(story);
        }
        if self.stories.len() > 200 {
            self.stories.truncate(200);
        }
    }

    pub fn put_request(&mut self, req: Request) {
        if let Some(slot) = self.requests.iter_mut().find(|r| r.id == req.id) {
            *slot = req;
        } else {
            self.requests.insert(0, req);
        }
        if self.requests.len() > 500 {
            self.requests.truncate(500);
        }
    }

    pub fn put_sketch(&mut self, s: Sketch) {
        if let Some(slot) = self.sketches.iter_mut().find(|x| x.id == s.id) {
            *slot = s;
        } else {
            self.sketches.insert(0, s);
        }
        if self.sketches.len() > 400 {
            self.sketches.truncate(400);
        }
    }

    pub fn pending(&self) -> usize {
        self.requests.iter().filter(|r| r.status != "replied").count()
    }

    pub fn mark_seen(&mut self, key: &str) {
        if !key.is_empty() && !self.seen.iter().any(|k| k == key) {
            self.seen.insert(0, key.to_string());
            if self.seen.len() > 2000 {
                self.seen.truncate(2000);
            }
        }
    }

    pub fn seen(&self, key: &str) -> bool {
        self.seen.iter().any(|k| k == key)
    }
}
