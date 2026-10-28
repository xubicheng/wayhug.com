// ============================================================
// 服务层 · 配置（只读环境变量，未配置的能力自动降级，不写死账号）
// ============================================================

use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct SmtpCfg {
    pub host: String,
    pub port: u16,
    pub secure: bool,
    pub user: String,
    pub pass: String,
}

impl SmtpCfg {
    pub fn ready(&self) -> bool {
        !self.host.is_empty() && !self.user.is_empty() && !self.pass.is_empty()
    }
}

#[derive(Clone, Debug)]
pub struct ImapCfg {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub secure: bool,
    pub user: String,
    pub pass: String,
    pub interval_secs: u64,
    pub senders: Vec<String>,
}

impl ImapCfg {
    pub fn ready(&self) -> bool {
        self.enabled && !self.host.is_empty() && !self.user.is_empty() && !self.pass.is_empty()
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub port: u16,
    pub site_url: String,
    pub contact_email: String,
    /// 面向公众的故事投稿地址（可用 WAYHUG_STORY_EMAIL 指定专用邮箱）
    pub story_email: String,
    pub owner_email: String,
    pub owner_name: String,
    pub admin_token: String,
    /// 写作口令：公开页保留「写」入口，但只有持有口令的人能真正生成
    pub write_token: String,
    /// 免费授权截止日（MIT 口径），证书与页面统一引用
    pub free_until: String,
    pub auto_reply: bool,
    pub mail_prefix: String,
    pub data_path: PathBuf,
    pub sketch_dir: PathBuf,
    pub font_path: Option<String>,
    pub smtp: SmtpCfg,
    pub imap: ImapCfg,
}

fn env_any(keys: &[&str]) -> String {
    for k in keys {
        if let Ok(v) = std::env::var(k) {
            if !v.trim().is_empty() {
                return v.trim().to_string();
            }
        }
    }
    String::new()
}

fn env_bool(keys: &[&str], def: bool) -> bool {
    let v = env_any(keys);
    if v.is_empty() {
        return def;
    }
    matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
}

fn env_u64(keys: &[&str], def: u64) -> u64 {
    env_any(keys).parse().unwrap_or(def)
}

impl Config {
    pub fn load() -> Self {
        let port = env_u64(&["WAYHUG_PORT", "PORT"], 3001) as u16;
        let port = if port == 0 { 3001 } else { port };
        let smtp_port = env_u64(&["WAYHUG_SMTP_PORT", "SMTP_PORT"], 465) as u16;
        let imap_port = env_u64(&["MAIL_IMAP_PORT", "WAYHUG_IMAP_PORT"], 993) as u16;

        let senders = env_any(&["WAYHUG_IMAP_SENDERS", "MAIL_IMAP_SENDERS"])
            .split(',')
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .collect();

        let smtp_user = env_any(&["WAYHUG_SMTP_USER", "SMTP_USER"]);
        let contact_email = env_any(&["WAYHUG_CONTACT_EMAIL", "LAB_CONTACT_EMAIL", "CONTACT_EMAIL"])
            .into_fallback("bc.xu@hotmail.com");
        let story_email = env_any(&["WAYHUG_STORY_EMAIL"])
            .into_fallback(&if smtp_user.is_empty() { contact_email.clone() } else { smtp_user.clone() });

        Config {
            port,
            site_url: env_any(&["WAYHUG_SITE_URL", "LAB_SITE_URL", "SITE_URL"])
                .trim_end_matches('/')
                .to_string()
                .into_fallback("https://wayhug.com"),
            contact_email: env_any(&["WAYHUG_CONTACT_EMAIL", "LAB_CONTACT_EMAIL", "CONTACT_EMAIL"])
                .into_fallback("bc.xu@hotmail.com"),
            story_email,
            owner_email: env_any(&["WAYHUG_OWNER_EMAIL", "LAB_OWNER_EMAIL", "NOTIFY_EMAIL"]),
            owner_name: env_any(&["WAYHUG_OWNER_NAME"]).into_fallback("王元会"),
            admin_token: env_any(&["WAYHUG_ADMIN_TOKEN", "LAB_ADMIN_TOKEN"]),
            write_token: env_any(&["WAYHUG_WRITE_TOKEN"]),
            free_until: env_any(&["WAYHUG_FREE_UNTIL"]).into_fallback("2026-12-31"),
            auto_reply: env_bool(&["WAYHUG_AUTO_REPLY", "LAB_AUTO_REPLY"], false),
            mail_prefix: env_any(&["WAYHUG_MAIL_PREFIX", "LAB_MAIL_PREFIX"]).into_fallback("[WAYHUG]"),
            data_path: env_any(&["WAYHUG_DATA", "LAB_DATA"])
                .into_fallback("data/wayhug.json")
                .into(),
            sketch_dir: env_any(&["WAYHUG_SKETCH_DIR"])
                .into_fallback("data/sketches")
                .into(),
            font_path: {
                let p = env_any(&["WAYHUG_FONT_PATH", "LAB_FONT_PATH"]);
                if p.is_empty() { None } else { Some(p) }
            },
            smtp: SmtpCfg {
                host: env_any(&["WAYHUG_SMTP_HOST", "SMTP_HOST"]),
                port: smtp_port,
                secure: env_bool(&["WAYHUG_SMTP_SECURE", "SMTP_SECURE"], smtp_port == 465),
                user: env_any(&["WAYHUG_SMTP_USER", "SMTP_USER"]),
                pass: env_any(&["WAYHUG_SMTP_PASS", "SMTP_PASS"]),
            },
            imap: ImapCfg {
                enabled: env_bool(&["WAYHUG_IMAP_ENABLED", "LAB_IMAP_ENABLED"], false),
                host: env_any(&["WAYHUG_IMAP_HOST", "MAIL_IMAP_HOST"]),
                port: imap_port,
                secure: env_bool(&["WAYHUG_IMAP_SECURE", "MAIL_IMAP_SECURE"], true),
                user: env_any(&["WAYHUG_IMAP_USER", "MAIL_IMAP_USER"]),
                pass: env_any(&["WAYHUG_IMAP_PASS", "MAIL_IMAP_PASS"]),
                interval_secs: env_u64(&["WAYHUG_IMAP_INTERVAL_SECS", "MAIL_IMAP_INTERVAL_SECS"], 180).max(60),
                senders,
            },
        }
    }

    pub fn smtp_ready(&self) -> bool {
        self.smtp.ready()
    }

    pub fn imap_ready(&self) -> bool {
        self.imap.ready()
    }

    pub fn is_owner(&self, email: &str) -> bool {
        !self.owner_email.is_empty() && self.owner_email.eq_ignore_ascii_case(email.trim())
    }

    /// 写作口令与后台令牌都可用于生成；未配置口令时后台令牌仍可用
    pub fn write_ok(&self, token: &str) -> bool {
        if token.is_empty() {
            return false;
        }
        token == self.write_token || (!self.admin_token.is_empty() && token == self.admin_token)
    }
}

trait Fallback {
    fn into_fallback(self, def: &str) -> String;
}

impl Fallback for String {
    fn into_fallback(self, def: &str) -> String {
        if self.trim().is_empty() {
            def.to_string()
        } else {
            self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_works() {
        assert_eq!("".to_string().into_fallback("x"), "x");
        assert_eq!("a".to_string().into_fallback("x"), "a");
    }
}
