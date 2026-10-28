// ============================================================
// 服务层 · 邮件（SMTP 发信 + IMAP 只读收信）
//
// 与 jshnp.com 共用邮箱时的安全约定：
//   1. 只读拉取：不写 \Seen、不删除、不 expunge；
//   2. 主题/正文必须包含 [WAYHUG] 标记，与博客邮件隔离；
//   3. Message-ID 去重，重复轮询不会重复建单。
// ============================================================

use crate::certificate::certificate_pdf;
use crate::config::Config;
use crate::model::{Db, MailResult, Request};
use crate::store::{create_request, now_iso, save};
use crate::story::parse_story;
use async_native_tls::TlsConnector;
use futures_util::io::{AsyncRead as FAsyncRead, AsyncWrite as FAsyncWrite};
use futures_util::StreamExt;
use lettre::message::header::ContentType;
use lettre::message::{Attachment, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use mailparse::MailHeaderMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_util::compat::TokioAsyncReadCompatExt;

fn transport(cfg: &Config) -> Result<AsyncSmtpTransport<Tokio1Executor>, String> {
    let creds = Credentials::new(cfg.smtp.user.clone(), cfg.smtp.pass.clone());
    if cfg.smtp.secure {
        let tls = TlsParameters::new(cfg.smtp.host.clone()).map_err(|e| format!("TLS 参数失败: {}", e))?;
        Ok(AsyncSmtpTransport::<Tokio1Executor>::relay(&cfg.smtp.host)
            .map_err(|e| format!("SMTP 配置失败: {}", e))?
            .port(cfg.smtp.port)
            .tls(Tls::Wrapper(tls))
            .credentials(creds)
            .build())
    } else {
        Ok(AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&cfg.smtp.host)
            .map_err(|e| format!("SMTP 配置失败: {}", e))?
            .port(cfg.smtp.port)
            .credentials(creds)
            .build())
    }
}

fn manual(reason: impl Into<String>) -> MailResult {
    MailResult { mode: "manual".into(), ok: false, reason: reason.into(), at: now_iso() }
}

fn sent() -> MailResult {
    MailResult { mode: "smtp".into(), ok: true, reason: String::new(), at: now_iso() }
}

async fn send_message(cfg: &Config, msg: Message) -> Result<(), String> {
    transport(cfg)?.send(msg).await.map_err(|e| format!("SMTP 发送失败: {}", e))?;
    Ok(())
}

fn from_mailbox(cfg: &Config) -> Result<Mailbox, String> {
    format!("Wayhug IP Lab <{}>", cfg.smtp.user)
        .parse()
        .map_err(|e| format!("发件地址无效: {}", e))
}

fn reply_to(cfg: &Config) -> Option<Mailbox> {
    if cfg.owner_email.is_empty() {
        None
    } else {
        cfg.owner_email.parse().ok()
    }
}

pub async fn send_text(cfg: &Config, to: &str, subject: &str, body: &str) -> Result<(), String> {
    if !cfg.smtp_ready() {
        return Err("SMTP 未配置".into());
    }
    let mut builder = Message::builder()
        .from(from_mailbox(cfg)?)
        .to(to.parse().map_err(|e| format!("收件地址无效: {}", e))?)
        .subject(subject);
    if let Some(rt) = reply_to(cfg) {
        builder = builder.reply_to(rt);
    }
    send_message(cfg, builder.body(body.to_string()).map_err(|e| format!("构建邮件失败: {}", e))?).await
}

/// 发送证书（自动回复）。SMTP 未配置时降级为 manual。
pub async fn send_certificate(cfg: &Config, req: &Request, extra_text: Option<&str>) -> MailResult {
    if !cfg.smtp_ready() {
        return manual("SMTP 未配置，已转为人工答复");
    }
    let pdf = match certificate_pdf(cfg, req, None) {
        Ok(b) => b,
        Err(e) => return manual(format!("证书生成失败: {}", e)),
    };
    let verify = format!("{}/verify/{}", cfg.site_url, req.id);
    let watch = format!("{}/watch/{}", cfg.site_url, req.story_id);
    let mut body = format!(
        "你好，{}：\n\n这是你的 A4 纪念证书，也是《{}》的免费授权凭证，可直接下载打印。\n\n证书编号：{}\n在线校验：{}\n观看动画：{}\n免费授权至：{}\n\n授权方式：MIT。期内可自由使用、复制、修改、分发，也可商用；分发时请保留作者署名与许可声明。完整文本：{}/license\n\n如需人工答复，直接回复本邮件即可。\n\n—— Wayhug IP Lab · {}\n",
        req.name,
        req.story_title,
        req.id,
        verify,
        watch,
        cfg.free_until,
        cfg.site_url.trim_end_matches('/'),
        cfg.site_url
    );
    if let Some(extra) = extra_text {
        if !extra.trim().is_empty() {
            body.push_str("\n\n—— 作者答复 ——\n");
            body.push_str(extra.trim());
            body.push('\n');
        }
    }

    let mut builder = Message::builder()
        .from(match from_mailbox(cfg) {
            Ok(m) => m,
            Err(e) => return manual(e),
        })
        .to(match req.email.parse() {
            Ok(m) => m,
            Err(e) => return manual(format!("访客邮箱无效: {}", e)),
        })
        .subject(format!("[WAYHUG] 你的纪念证书 · {}", req.id));
    if let Some(rt) = reply_to(cfg) {
        builder = builder.reply_to(rt);
    }
    if let Ok(owner) = cfg.owner_email.parse::<Mailbox>() {
        builder = builder.bcc(owner);
    }

    let filename = format!("Wayhug-IP-{}.pdf", req.id);
    let attachment = Attachment::new(filename).body(pdf, ContentType::parse("application/pdf").unwrap());
    let msg = builder
        .multipart(
            MultiPart::mixed()
                .singlepart(SinglePart::plain(body))
                .singlepart(attachment),
        );
    let msg = match msg {
        Ok(m) => m,
        Err(e) => return manual(format!("构建邮件失败: {}", e)),
    };
    match send_message(cfg, msg).await {
        Ok(()) => sent(),
        Err(e) => manual(e),
    }
}

// ---------------- IMAP 只读收信 ----------------

trait Io: FAsyncRead + FAsyncWrite + Unpin + Send + Debug {}
impl<T: FAsyncRead + FAsyncWrite + Unpin + Send + Debug> Io for T {}
type BoxIo = Box<dyn Io>;

#[derive(Default, Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PollSummary {
    pub fetched: usize,
    pub ingested: usize,
    pub replied: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug)]
struct Incoming {
    from_email: String,
    from_name: String,
    subject: String,
    text: String,
    message_id: String,
}

fn find_part<'a>(mail: &'a mailparse::ParsedMail<'a>, mime: &str) -> Option<&'a mailparse::ParsedMail<'a>> {
    if mail.ctype.mimetype.eq_ignore_ascii_case(mime) {
        return Some(mail);
    }
    mail.subparts.iter().find_map(|p| find_part(p, mime))
}

fn parse_source(source: &[u8]) -> Option<Incoming> {
    let parsed = mailparse::parse_mail(source).ok()?;
    let subject = parsed.headers.get_first_value("Subject").unwrap_or_default();
    let from_header = parsed.headers.get_first_value("From").unwrap_or_default();
    let addrs = mailparse::addrparse(&from_header).ok()?;
    let (from_name, from_email) = match addrs.iter().next()? {
        mailparse::MailAddr::Single(info) => (info.display_name.clone().unwrap_or_default(), info.addr.clone()),
        mailparse::MailAddr::Group(g) => {
            let first = g.addrs.first()?;
            (first.display_name.clone().unwrap_or_default(), first.addr.clone())
        }
    };
    let text = find_part(&parsed, "text/plain")
        .and_then(|p| p.get_body().ok())
        .unwrap_or_default();
    let message_id = parsed.headers.get_first_value("Message-ID").unwrap_or_default();
    Some(Incoming {
        from_email: from_email.to_ascii_lowercase(),
        from_name: if from_name.trim().is_empty() {
            from_email.split('@').next().unwrap_or("访客").to_string()
        } else {
            from_name.trim().to_string()
        },
        subject,
        text,
        message_id,
    })
}

fn has_marker(cfg: &Config, subject: &str, text: &str) -> bool {
    let p = cfg.mail_prefix.to_ascii_lowercase();
    subject.to_ascii_lowercase().contains(&p)
        || text.to_ascii_lowercase().contains(&p)
        || text.to_ascii_lowercase().contains("#wayhug")
}

/// 过滤本站自动发出的邮件（自动回信 / 待办列表 / 编号错误提示），
/// 避免它们被 IMAP 再当成「投稿」收录，形成自回环。
/// 收信侧过滤，不改动发信链路。
fn is_own_auto_mail(cfg: &Config, inc: &Incoming) -> bool {
    let subject = inc.subject.to_ascii_lowercase();
    let from_self =
        !cfg.smtp.user.is_empty() && inc.from_email.eq_ignore_ascii_case(&cfg.smtp.user);
    from_self
        && (subject.contains("纪念证书")
            || subject.contains("ip 体验权证书")
            || subject.contains("待办列表")
            || subject.contains("未找到申请"))
}

fn strip_marker(text: &str) -> String {
    text.lines()
        .filter(|l| !l.to_ascii_lowercase().contains("#wayhug"))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn parse_reply_command(cfg: &Config, inc: &Incoming) -> Option<(String, String)> {
    if !cfg.is_owner(&inc.from_email) {
        return None;
    }
    let blob = format!("{}\n{}", inc.subject, inc.text);
    let idx = blob.find("WH-")?;
    let id: String = blob[idx..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();
    if id.len() < 10 {
        return None;
    }
    let rest = blob[idx + id.len()..].trim();
    let rest = rest.trim_start_matches([':', '：', '\n', ' ']).trim();
    Some((id, rest.to_string()))
}

fn list_text(db: &Db) -> String {
    let mut out = String::from("Wayhug 待办（未人工答复）：\n\n");
    let mut n = 0;
    for r in db.requests.iter().filter(|r| r.status != "replied") {
        n += 1;
        out.push_str(&format!(
            "{}   {}   作品《{}》   来源 {}\n",
            r.id, r.email, r.story_title, r.source
        ));
    }
    if n == 0 {
        out.push_str("（暂无待办）\n");
    }
    out.push_str("\n回复指令：主题写 [WAYHUG] 回复 WH-... 你的答复正文\n");
    out
}

async fn poll_once(cfg: &Config, db: Arc<Mutex<Db>>) -> Result<PollSummary, String> {
    let ic = &cfg.imap;
    let tcp = TcpStream::connect((ic.host.as_str(), ic.port))
        .await
        .map_err(|e| format!("连接失败: {}", e))?;
    let stream: BoxIo = if ic.secure {
        let tls = TlsConnector::new();
        Box::new(
            tls.connect(ic.host.as_str(), tcp.compat())
                .await
                .map_err(|e| format!("TLS 握手失败: {}", e))?,
        )
    } else {
        Box::new(tcp.compat())
    };
    let client = async_imap::Client::new(stream);
    let mut session = client
        .login(&ic.user, &ic.pass)
        .await
        .map_err(|(e, _)| format!("IMAP 登录失败: {}", e))?;

    let mut summary = PollSummary::default();
    let result: Result<(), String> = async {
        session.select("INBOX").await.map_err(|e| e.to_string())?;
        let since = (chrono::Utc::now() - chrono::Duration::days(7)).format("%d-%b-%Y").to_string();
        let query = format!("SINCE {} TEXT \"{}\"", since, cfg.mail_prefix);
        let uids = session.uid_search(query).await.map_err(|e| e.to_string())?;
        let mut uids: Vec<u32> = uids.into_iter().collect();
        uids.sort_unstable();
        summary.fetched = uids.len();

        for uid in uids {
            let mut stream = match session.uid_fetch(uid.to_string(), "(RFC822)").await {
                Ok(s) => s,
                Err(e) => {
                    summary.errors.push(format!("UID {} 拉取失败: {}", uid, e));
                    continue;
                }
            };
            let mut source: Option<Vec<u8>> = None;
            while let Some(item) = stream.next().await {
                match item {
                    Ok(msg) => {
                        if let Some(b) = msg.body() {
                            source = Some(b.to_vec());
                            break;
                        }
                    }
                    Err(e) => {
                        summary.errors.push(format!("UID {} 读取失败: {}", uid, e));
                        break;
                    }
                }
            }
            drop(stream);
            let Some(source) = source else { continue };
            let Some(inc) = parse_source(&source) else {
                summary.skipped += 1;
                continue;
            };
            if !has_marker(cfg, &inc.subject, &inc.text) {
                summary.skipped += 1;
                continue;
            }
            if is_own_auto_mail(cfg, &inc) {
                summary.skipped += 1;
                continue;
            }
            if !ic.senders.is_empty() && !ic.senders.contains(&inc.from_email) {
                summary.skipped += 1;
                continue;
            }

            // 去重
            {
                let guard = db.lock().await;
                if !inc.message_id.is_empty() && guard.seen(&inc.message_id) {
                    summary.skipped += 1;
                    continue;
                }
            }

            // 站长指令：列表 / 回复
            if let Some((id, reply)) = parse_reply_command(cfg, &inc) {
                let target = {
                    let guard = db.lock().await;
                    guard.find_request(&id).cloned()
                };
                if let Some(mut req) = target {
                    req.reply_text = reply.chars().take(2000).collect();
                    req.replied_at = now_iso();
                    req.status = "replied".into();
                    let mail = send_certificate(cfg, &req, Some(&req.reply_text)).await;
                    req.mail = mail;
                    {
                        let mut guard = db.lock().await;
                        guard.put_request(req);
                        guard.mark_seen(&inc.message_id);
                        if let Err(e) = save(cfg, &guard) {
                            summary.errors.push(e);
                        }
                    }
                    summary.replied += 1;
                } else {
                    let _ = send_text(
                        cfg,
                        &inc.from_email,
                        &format!("{} 未找到申请 {}", cfg.mail_prefix, id),
                        "请核对编号后再试。",
                    )
                    .await;
                    summary.skipped += 1;
                }
                continue;
            }

            if cfg.is_owner(&inc.from_email) && (inc.subject.contains("列表") || inc.text.trim().starts_with("列表")) {
                let body = {
                    let guard = db.lock().await;
                    list_text(&guard)
                };
                let _ = send_text(cfg, &inc.from_email, &format!("{} 待办列表", cfg.mail_prefix), &body).await;
                let mut guard = db.lock().await;
                guard.mark_seen(&inc.message_id);
                let _ = save(cfg, &guard);
                summary.skipped += 1;
                continue;
            }

            // 普通投稿：正文即故事
            let story_text = strip_marker(&inc.text);
            if story_text.is_empty() {
                summary.skipped += 1;
                continue;
            }
            if let Err(reason) = crate::guard::check(&story_text) {
                println!("[mail] UID {} 跳过：{}", uid, reason);
                summary.skipped += 1;
                continue;
            }
            let subject_title = inc
                .subject
                .replace(&cfg.mail_prefix, "")
                .trim_start_matches(|c: char| c.is_whitespace() || c == '·' || c == '-' || c == ':')
                .trim()
                .to_string();
            let parsed = parse_story(
                &story_text,
                if subject_title.is_empty() { None } else { Some(&subject_title) },
                None,
            );
            let Ok(story) = parsed else {
                summary.skipped += 1;
                continue;
            };

            let mut req = {
                let mut guard = db.lock().await;
                guard.put_story(story.clone());
                let req = create_request(
                    &mut guard,
                    Some(&story),
                    &inc.from_name,
                    &inc.from_email,
                    &story.id,
                    "通过邮件投稿",
                    "email",
                    &cfg.free_until,
                );
                guard.mark_seen(&inc.message_id);
                if let Err(e) = save(cfg, &guard) {
                    summary.errors.push(e);
                }
                req
            };
            summary.ingested += 1;

            if cfg.auto_reply {
                let mail = send_certificate(cfg, &req, None).await;
                req.mail = mail;
                let mut guard = db.lock().await;
                guard.put_request(req);
                if let Err(e) = save(cfg, &guard) {
                    summary.errors.push(e);
                }
                summary.replied += 1;
            }
        }
        Ok(())
    }
    .await;

    let _ = session.logout().await;
    result?;
    Ok(summary)
}

pub async fn poll_now(cfg: &Config, db: Arc<Mutex<Db>>) -> Result<PollSummary, String> {
    if !cfg.imap_ready() {
        return Err("IMAP 未启用或未配置".into());
    }
    tokio::time::timeout(Duration::from_secs(150), poll_once(cfg, db))
        .await
        .map_err(|_| "IMAP 轮询超时".to_string())?
}

pub async fn run(cfg: Config, db: Arc<Mutex<Db>>) {
    if !cfg.imap_ready() {
        println!("[mail] IMAP 未启用：仅网页与 SMTP 发信可用");
        return;
    }
    println!("[mail] IMAP 只读轮询已启动，每 {} 秒一次（不写 \\Seen）", cfg.imap.interval_secs);
    loop {
        match poll_now(&cfg, db.clone()).await {
            Ok(s) => {
                if s.fetched > 0 {
                    println!(
                        "[mail] 轮询：拉取 {}，建单 {}，回复 {}，跳过 {}",
                        s.fetched, s.ingested, s.replied, s.skipped
                    );
                }
                for e in s.errors {
                    eprintln!("[mail] {}", e);
                }
            }
            Err(e) => eprintln!("[mail] 轮询失败: {}", e),
        }
        tokio::time::sleep(Duration::from_secs(cfg.imap.interval_secs)).await;
    }
}
