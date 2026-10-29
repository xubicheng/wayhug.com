// ============================================================
// 服务层 · HTTP 路由
//
// 公众动线只有四个动作：看 / 写 / 领 / 寄；
// 后台工作台 /studio 需要令牌。
// ============================================================

use crate::animate::{build_css, build_html, HtmlOptions};
use crate::canvas2d;
use crate::certificate::certificate_pdf;
use crate::config::Config;
use crate::mail;
use crate::model::{Db, Story};
use crate::pages;
use crate::sketch;
use crate::store::{self, create_request};
use crate::story::{demo_text, parse_story, story_to_json};
use crate::worldview;
use axum::body::{Body, Bytes};
use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<Config>,
    pub db: Arc<Mutex<Db>>,
    pub rate: Arc<Mutex<HashMap<String, (Instant, u32)>>>,
    pub analytics: Arc<crate::analytics::Analytics>,
}

impl AppState {
    pub fn new(cfg: Config, db: Db, analytics: crate::analytics::Analytics) -> Self {
        Self {
            cfg: Arc::new(cfg),
            db: Arc::new(Mutex::new(db)),
            rate: Arc::new(Mutex::new(HashMap::new())),
            analytics: Arc::new(analytics),
        }
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/watch/{id}", get(watch))
        .route("/watch2d/{id}", get(watch2d))
        .route("/assets/realm/{key}", get(realm_asset))
        .route("/dl/{id}/{kind}", get(download))
        .route("/cert/{id}", get(cert_pdf))
        .route("/verify/{id}", get(verify_page))
        .route("/license", get(license_page))
        .route("/studio", get(studio_page))
        .route("/sketch/{id}", get(sketch_file))
        .route("/api/info", get(api_info))
        .route("/api/stories", get(api_stories))
        .route("/api/story/{id}", get(api_story))
        .route("/api/demo.txt", get(api_demo))
        .route("/api/worldview", get(api_worldview))
        .route("/api/corpus", get(api_corpus))
        .route("/api/verify/{id}", get(api_verify))
        .route("/api/generate", post(api_generate))
        .route("/api/experience", post(api_experience))
        .route("/api/admin/requests", get(api_admin_requests))
        .route("/api/admin/reply", post(api_admin_reply))
        .route("/api/admin/poll", post(api_admin_poll))
        .route("/api/admin/sketches", get(api_admin_sketches).post(api_admin_sketch_upload))
        .route("/lab", get(redirect_home))
        .route("/lab/", get(redirect_home))
        .route("/lab/s1", get(redirect_home))
        .route("/lab/s2", get(redirect_mail))
        .route("/lab/s3", get(redirect_create))
        .route("/lab/s4", get(redirect_experience))
        .route("/lab/story/{id}", get(redirect_watch))
        .route("/lab/cert/{id}", get(redirect_cert))
        .route("/lab/verify/{id}", get(redirect_verify))
        .route("/favicon.ico", get(favicon))
        .route("/brand.png", get(brand_logo))
        .route("/favicon.svg", get(favicon))
        .route("/robots.txt", get(robots_txt))
        .route("/sitemap.xml", get(sitemap_xml))
        .layer(DefaultBodyLimit::max(13 * 1024 * 1024))
        .route("/api/track", post(api_track))
        .route("/api/stats", get(api_stats))
        .route("/api/maintenance", get(api_maintenance))
        .route("/stats", get(stats_html))
        .fallback(not_found)
        .with_state(state.clone())
        .layer(axum::middleware::from_fn_with_state(state, track_hit))
}

fn json_err(code: StatusCode, msg: &str) -> Response {
    (code, Json(json!({ "ok": false, "error": msg }))).into_response()
}

fn bearer_ok(state: &AppState, headers: &HeaderMap, query_token: Option<&str>) -> bool {
    let cfg = &state.cfg;
    if cfg.admin_token.is_empty() {
        return false;
    }
    let header_token = headers
        .get("x-wayhug-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    header_token == cfg.admin_token || query_token.map(|t| t == cfg.admin_token).unwrap_or(false)
}

pub fn extract_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| headers.get("x-real-ip").and_then(|v| v.to_str().ok()).map(|s| s.trim().to_string()))
        .unwrap_or_else(|| "local".into())
}

fn cookie_value(headers: &HeaderMap, name: &str) -> String {
    headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|raw| {
            raw.split(';')
                .filter_map(|kv| kv.trim().split_once('='))
                .find(|(k, _)| k.trim() == name)
                .map(|(_, v)| v.trim().to_string())
        })
        .unwrap_or_default()
}

/// 公开页保留「写」入口；真正生成需要写作口令（或后台令牌）
/// 口令可来自请求头，也可来自解锁后写入的 cookie，页面刷新/新开标签仍有效
fn write_ok(state: &AppState, headers: &HeaderMap) -> bool {
    let header_token = headers
        .get("x-wayhug-write")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let token = if header_token.trim().is_empty() {
        cookie_value(headers, "wayhug_write")
    } else {
        header_token.trim().to_string()
    };
    state.cfg.write_ok(&token) || bearer_ok(state, headers, None)
}

/// HTML 页面统一不缓存，避免浏览器拿着旧的内联脚本继续用
pub fn html_page(html: String) -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        html,
    )
        .into_response()
}

/// 简单的写入限流：同一 IP 10 分钟内最多 30 次
async fn allow_write(state: &AppState, headers: &HeaderMap) -> bool {
    let key = extract_ip(headers);
    let mut map = state.rate.lock().await;
    let now = Instant::now();
    let entry = map.entry(key).or_insert((now, 0));
    if now.duration_since(entry.0) > Duration::from_secs(600) {
        *entry = (now, 0);
    }
    entry.1 += 1;
    entry.1 <= 30
}

fn redirect(to: &str) -> Response {
    Response::builder()
        .status(StatusCode::FOUND)
        .header(header::LOCATION, to)
        .body(Body::empty())
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

async fn redirect_home() -> Response {
    redirect("/")
}

/// 浏览器标签页图标（王元会定制 PNG），响应 /favicon.ico
async fn favicon() -> Response {
    ([(header::CONTENT_TYPE, "image/png")], crate::pages::FAVICON_PNG).into_response()
}

/// 页头品牌 logo（Wyh 圆形 PNG），响应 /brand.png
async fn brand_logo() -> Response {
    ([(header::CONTENT_TYPE, "image/png")], crate::pages::BRAND_LOGO_PNG).into_response()
}

/// robots.txt：允许全部爬虫，指定 sitemap
async fn robots_txt() -> Response {
    let body = "User-agent: *\nAllow: /\nSitemap: https://wayhug.com/sitemap.xml\n";
    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], body).into_response()
}

/// sitemap.xml：列出核心页面，供搜索引擎与 AI 爬虫索引
async fn sitemap_xml() -> Response {
    let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url><loc>https://wayhug.com/</loc><changefreq>daily</changefreq><priority>1.0</priority></url>
  <url><loc>https://wayhug.com/#watch</loc><changefreq>weekly</changefreq><priority>0.8</priority></url>
  <url><loc>https://wayhug.com/#create</loc><changefreq>weekly</changefreq><priority>0.8</priority></url>
  <url><loc>https://wayhug.com/#world</loc><changefreq>monthly</changefreq><priority>0.6</priority></url>
  <url><loc>https://wayhug.com/license</loc><changefreq>yearly</changefreq><priority>0.3</priority></url>
</urlset>"#;
    ([(header::CONTENT_TYPE, "application/xml; charset=utf-8")], body).into_response()
}
async fn redirect_mail() -> Response {
    redirect("/#mail")
}
async fn redirect_create() -> Response {
    redirect("/#create")
}
async fn redirect_experience() -> Response {
    redirect("/#experience")
}
async fn redirect_watch(Path(id): Path<String>) -> Response {
    redirect(&format!("/watch/{}", id))
}
async fn redirect_cert(Path(id): Path<String>) -> Response {
    redirect(&format!("/cert/{}.pdf", id.replace(".pdf", "")))
}
async fn redirect_verify(Path(id): Path<String>) -> Response {
    redirect(&format!("/verify/{}", id))
}

async fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        html_page(pages::error_page(404, "页面不存在")),
    )
        .into_response()
}

// ---------------- 页面 ----------------

async fn root(State(state): State<AppState>) -> Response {
    let db = state.db.lock().await;
    let Some(demo) = db.find_story("demo").or_else(|| db.stories.first()) else {
        return (StatusCode::INTERNAL_SERVER_ERROR, html_page(pages::error_page(500, "演示作品尚未生成"))).into_response();
    };
    let mut latest: Vec<crate::model::Story> = db.stories.clone();
    latest.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    latest.truncate(6);
    let corpus = worldview::corpus_view(&db.stories);
    let html = pages::landing(pages::Landing {
        cfg: &state.cfg,
        demo,
        latest: &latest,
        corpus: &corpus,
        stories: db.stories.len(),
        requests: db.requests.len(),
        sketches: db.sketches.len(),
    });
    html_page(html)
}

#[derive(Deserialize, Default)]
struct WatchQuery {
    still: Option<String>,
    scene: Option<usize>,
}

async fn watch(State(state): State<AppState>, Path(id): Path<String>, Query(q): Query<WatchQuery>) -> Response {
    let db = state.db.lock().await;
    let Some(story) = db.find_story(&id) else {
        return (StatusCode::NOT_FOUND, html_page(pages::error_page(404, "作品不存在"))).into_response();
    };
    let html = build_html(
        story,
        HtmlOptions {
            cta_url: Some("/#experience"),
            still: q.still.as_deref() == Some("1") || q.still.as_deref() == Some("true"),
            scene_index: q.scene.unwrap_or(0),
        },
    );
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=300"),
        ],
        html,
    )
        .into_response()
}

async fn watch2d(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let db = state.db.lock().await;
    let Some(story) = db.find_story(&id) else {
        return (
            StatusCode::NOT_FOUND,
            html_page(pages::error_page(404, "作品不存在")),
        )
            .into_response();
    };
    html_page(canvas2d::render_html(story))
}

/// 四相招牌动物的原创 SVG 资产（编译期内嵌，无第三方素材）
const REALM_SVGS: &[(&str, &str)] = &[
    ("yuan", include_str!("../assets/realm/yuan.svg")),
    ("xian", include_str!("../assets/realm/xian.svg")),
    ("chong", include_str!("../assets/realm/chong.svg")),
    ("ren", include_str!("../assets/realm/ren.svg")),
];

async fn realm_asset(Path(key): Path<String>) -> Response {
    let key = key.trim_end_matches(".svg");
    match REALM_SVGS.iter().find(|(k, _)| *k == key) {
        Some((_, body)) => (
            [
                (header::CONTENT_TYPE, "image/svg+xml; charset=utf-8"),
                (header::CACHE_CONTROL, "public, max-age=604800"),
            ],
            *body,
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "unknown realm asset").into_response(),
    }
}

async fn download(State(state): State<AppState>, Path((id, kind)): Path<(String, String)>) -> Response {
    let db = state.db.lock().await;
    let Some(story) = db.find_story(&id) else {
        return (StatusCode::NOT_FOUND, html_page(pages::error_page(404, "作品不存在"))).into_response();
    };
    let (body, mime, ext) = match kind.as_str() {
        "html" => (build_html(story, HtmlOptions { cta_url: Some("/#experience"), ..Default::default() }), "text/html; charset=utf-8", "html"),
        "css" => (build_css(story), "text/css; charset=utf-8", "css"),
        "json" => (serde_json::to_string_pretty(&story_to_json(story)).unwrap_or_default(), "application/json; charset=utf-8", "json"),
        "canvas" => (canvas2d::render_html(story), "text/html; charset=utf-8", "html"),
        _ => return (StatusCode::NOT_FOUND, "unknown kind").into_response(),
    };
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"wayhug-{}.{}\"", safe_id(&id), ext),
        )
        .body(Body::from(body))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

fn safe_id(id: &str) -> String {
    id.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

async fn cert_pdf(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let id = id.strip_suffix(".pdf").unwrap_or(&id).to_string();
    let (record, story) = {
        let db = state.db.lock().await;
        let Some(req) = db.find_request(&id).cloned() else {
            return (StatusCode::NOT_FOUND, html_page(pages::error_page(404, "证书不存在"))).into_response();
        };
        let story = db.find_story(&req.story_id).cloned();
        (req, story)
    };
    match certificate_pdf(&state.cfg, &record, story.as_ref()) {
        Ok(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/pdf")
            .header(
                header::CONTENT_DISPOSITION,
                format!("inline; filename=\"Wayhug-IP-{}.pdf\"", safe_id(&id)),
            )
            .body(Body::from(bytes))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response()),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "ok": false, "error": e }))).into_response(),
    }
}

async fn verify_page(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let db = state.db.lock().await;
    let Some(req) = db.find_request(&id) else {
        return (StatusCode::NOT_FOUND, html_page(pages::error_page(404, "未找到该证书编号"))).into_response();
    };
    let exists = db.find_story(&req.story_id).is_some();
    html_page(pages::verify_page(&state.cfg, req, exists))
}

async fn license_page() -> Response {
    html_page(pages::license_page())
}

async fn studio_page(State(state): State<AppState>) -> Response {
    html_page(pages::studio_page(&state.cfg))
}

async fn sketch_file(State(state): State<AppState>, Path(id): Path<String>, Query(q): Query<TokenQuery>, headers: HeaderMap) -> Response {
    if !bearer_ok(&state, &headers, q.token.as_deref()) {
        return json_err(StatusCode::UNAUTHORIZED, "需要管理令牌");
    }
    let (file, mime) = {
        let db = state.db.lock().await;
        match db.find_sketch(&id) {
            Some(s) => (s.file.clone(), s.mime.clone()),
            None => return (StatusCode::NOT_FOUND, "not found").into_response(),
        }
    };
    let path = state.cfg.sketch_dir.join(file);
    match tokio::fs::read(&path).await {
        Ok(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime)
            .header(header::CACHE_CONTROL, "private, max-age=600")
            .body(Body::from(bytes))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response()),
        Err(_) => (StatusCode::NOT_FOUND, "image missing").into_response(),
    }
}

// ---------------- 公开 API ----------------

async fn api_info(State(state): State<AppState>) -> Response {
    let db = state.db.lock().await;
    let demo = db.find_story("demo");
    Json(json!({
        "ok": true,
        "site": state.cfg.site_url,
        "engine": "wayhug/2.0-rust",
        "license": { "type": "MIT", "freeUntil": state.cfg.free_until, "url": "/license" },
        "capability": {
            "smtp": state.cfg.smtp_ready(),
            "imap": state.cfg.imap_ready(),
            "autoReply": state.cfg.auto_reply,
            "admin": !state.cfg.admin_token.is_empty(),
            "storyEmail": state.cfg.story_email,
            "autoReplyMode": if state.cfg.auto_reply && state.cfg.smtp_ready() { "automatic" } else { "manual" }
        },
        "stats": {
            "stories": db.stories.len(),
            "requests": db.requests.len(),
            "pending": db.pending(),
            "sketches": db.sketches.len()
        },
        "demo": demo.map(|s| json!({
            "id": s.id, "title": s.title, "scenes": s.stats.scenes,
            "chars": s.stats.chars, "cast": s.cast, "fingerprint": s.fingerprint
        })),
        "worldview": "worldview/0.1"
    }))
    .into_response()
}

async fn api_stories(State(state): State<AppState>) -> Response {
    let db = state.db.lock().await;
    let list: Vec<Value> = db
        .stories
        .iter()
        .map(|s| {
            json!({
                "id": s.id, "title": s.title, "scenes": s.stats.scenes,
                "chars": s.stats.chars, "cast": s.cast.iter().map(|c| &c.name).collect::<Vec<_>>(),
                "createdAt": s.created_at, "source": s.source
            })
        })
        .collect();
    Json(json!({ "ok": true, "stories": list })).into_response()
}

async fn api_story(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let db = state.db.lock().await;
    match db.find_story(&id) {
        Some(s) => Json(json!({ "ok": true, "story": s, "sceneGraph": story_to_json(s) })).into_response(),
        None => json_err(StatusCode::NOT_FOUND, "作品不存在"),
    }
}

async fn api_demo() -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        demo_text(),
    )
        .into_response()
}

async fn api_worldview() -> Response {
    Json(json!({ "ok": true, "worldview": worldview::public_view() })).into_response()
}

async fn api_corpus(State(state): State<AppState>) -> Response {
    let db = state.db.lock().await;
    Json(json!({ "ok": true, "corpus": worldview::corpus_view(&db.stories) })).into_response()
}

async fn api_verify(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let db = state.db.lock().await;
    match db.find_request(&id) {
        Some(r) => Json(json!({
            "ok": true, "valid": true,
            "request": r,
            "storyExists": db.find_story(&r.story_id).is_some()
        }))
        .into_response(),
        None => Json(json!({ "ok": true, "valid": false, "error": "未找到该证书编号" })).into_response(),
    }
}

#[derive(Deserialize)]
struct GenerateBody {
    #[serde(default)]
    title: String,
    text: String,
}

async fn api_generate(State(state): State<AppState>, headers: HeaderMap, Json(body): Json<GenerateBody>) -> Response {
    if !write_ok(&state, &headers) {
        return json_err(StatusCode::UNAUTHORIZED, "暂时无法生成，请稍后再试");
    }
    if !allow_write(&state, &headers).await {
        return json_err(StatusCode::TOO_MANY_REQUESTS, "提交太频繁，请稍后再试");
    }
    let text = body.text.trim();
    if text.chars().count() < 8 {
        return json_err(StatusCode::BAD_REQUEST, "故事太短，至少写一句完整的话");
    }
    if text.chars().count() > 20000 {
        return json_err(StatusCode::BAD_REQUEST, "故事过长（上限 20000 字符）");
    }
    if let Err(e) = crate::guard::check(&format!("{}\n{}", body.title, text)) {
        return json_err(StatusCode::BAD_REQUEST, &e);
    }
    let title = if body.title.trim().is_empty() { None } else { Some(body.title.trim()) };
    let parsed = match parse_story(text, title, None) {
        Ok(s) => s,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, &e),
    };
    {
        let mut db = state.db.lock().await;
        let mut story = parsed;
        story.source = "web".into();
        db.put_story(story.clone());
        if let Err(e) = store::save(&state.cfg, &db) {
            eprintln!("[web] 保存作品失败: {}", e);
        }
        Json(json!({
            "ok": true,
            "story": {
                "id": story.id, "title": story.title, "fingerprint": story.fingerprint,
                "stats": story.stats, "cast": story.cast
            },
            "links": {
                "watch": format!("/watch/{}", story.id),
                "html": format!("/dl/{}/html", story.id),
                "css": format!("/dl/{}/css", story.id),
                "json": format!("/dl/{}/json", story.id)
            }
        }))
        .into_response()
    }
}

#[derive(Deserialize)]
struct ExperienceBody {
    name: String,
    email: String,
    #[serde(default)]
    #[serde(rename = "storyId")]
    story_id: String,
    #[serde(default)]
    message: String,
}

async fn api_experience(State(state): State<AppState>, headers: HeaderMap, Json(body): Json<ExperienceBody>) -> Response {
    if !allow_write(&state, &headers).await {
        return json_err(StatusCode::TOO_MANY_REQUESTS, "提交太频繁，请稍后再试");
    }
    let name = body.name.trim();
    let email = body.email.trim().to_ascii_lowercase();
    if name.is_empty() || name.chars().count() > 40 {
        return json_err(StatusCode::BAD_REQUEST, "请填写有效的称呼");
    }
    if !email.contains('@') || !email.contains('.') || email.len() > 120 {
        return json_err(StatusCode::BAD_REQUEST, "请填写有效邮箱");
    }
    let story_id = if body.story_id.trim().is_empty() { "demo".to_string() } else { body.story_id.trim().to_string() };

    let story = {
        let db = state.db.lock().await;
        db.find_story(&story_id)
            .or_else(|| db.find_story("demo"))
            .cloned()
    };
    let Some(story) = story else {
        return json_err(StatusCode::BAD_REQUEST, "作品不存在");
    };

    let mut req = {
        let mut db = state.db.lock().await;
        let req = create_request(
            &mut db,
            Some(&story),
            name,
            &email,
            &story.id,
            body.message.trim(),
            "web",
            &state.cfg.free_until,
        );
        if let Err(e) = store::save(&state.cfg, &db) {
            eprintln!("[web] 保存申请失败: {}", e);
        }
        req
    };

    let mail_result = if state.cfg.auto_reply {
        mail::send_certificate(&state.cfg, &req, None).await
    } else {
        crate::model::MailResult {
            mode: "manual".into(),
            ok: false,
            reason: "自动回信未启用，已转人工答复".into(),
            at: store::now_iso(),
        }
    };
    req.mail = mail_result.clone();

    {
        let mut db = state.db.lock().await;
        db.put_request(req.clone());
        if let Err(e) = store::save(&state.cfg, &db) {
            eprintln!("[web] 更新申请失败: {}", e);
        }
    }

    Json(json!({
        "ok": true,
        "request": {
            "id": req.id, "name": req.name, "email": req.email,
            "storyId": req.story_id, "storyTitle": req.story_title,
            "status": req.status, "expiresAt": req.expires_at,
            "certUrl": format!("/cert/{}.pdf", req.id),
            "verifyUrl": format!("/verify/{}", req.id)
        },
        "certUrl": format!("/cert/{}.pdf", req.id),
        "verifyUrl": format!("/verify/{}", req.id),
        "mail": req.mail
    }))
    .into_response()
}

// ---------------- 后台 API ----------------

#[derive(Deserialize, Default)]
struct TokenQuery {
    token: Option<String>,
}

async fn api_admin_requests(State(state): State<AppState>, headers: HeaderMap, Query(q): Query<TokenQuery>) -> Response {
    if !bearer_ok(&state, &headers, q.token.as_deref()) {
        return json_err(StatusCode::UNAUTHORIZED, "令牌无效");
    }
    let db = state.db.lock().await;
    let list: Vec<&crate::model::Request> = db.requests.iter().take(200).collect();
    Json(json!({
        "ok": true,
        "pending": db.pending(),
        "requests": list
    }))
    .into_response()
}

#[derive(Deserialize)]
struct ReplyBody {
    id: String,
    text: String,
}

async fn api_admin_reply(State(state): State<AppState>, headers: HeaderMap, Query(q): Query<TokenQuery>, Json(body): Json<ReplyBody>) -> Response {
    if !bearer_ok(&state, &headers, q.token.as_deref()) {
        return json_err(StatusCode::UNAUTHORIZED, "令牌无效");
    }
    if body.text.trim().is_empty() {
        return json_err(StatusCode::BAD_REQUEST, "答复内容不能为空");
    }
    let mut req = {
        let db = state.db.lock().await;
        match db.find_request(&body.id).cloned() {
            Some(r) => r,
            None => return json_err(StatusCode::NOT_FOUND, "申请不存在"),
        }
    };
    req.reply_text = body.text.trim().chars().take(4000).collect();
    req.replied_at = store::now_iso();
    let mail_result = mail::send_certificate(&state.cfg, &req, Some(&req.reply_text)).await;
    req.mail = mail_result.clone();
    req.status = if mail_result.ok { "replied".into() } else { "pending".into() };

    {
        let mut db = state.db.lock().await;
        db.put_request(req.clone());
        if let Err(e) = store::save(&state.cfg, &db) {
            return json_err(StatusCode::INTERNAL_SERVER_ERROR, &e);
        }
    }
    Json(json!({ "ok": true, "request": req, "mail": mail_result })).into_response()
}

async fn api_admin_poll(State(state): State<AppState>, headers: HeaderMap, Query(q): Query<TokenQuery>) -> Response {
    if !bearer_ok(&state, &headers, q.token.as_deref()) {
        return json_err(StatusCode::UNAUTHORIZED, "令牌无效");
    }
    match mail::poll_now(&state.cfg, state.db.clone()).await {
        Ok(summary) => Json(json!({ "ok": true, "summary": summary })).into_response(),
        Err(e) => json_err(StatusCode::BAD_REQUEST, &e),
    }
}

async fn api_admin_sketches(State(state): State<AppState>, headers: HeaderMap, Query(q): Query<TokenQuery>) -> Response {
    if !bearer_ok(&state, &headers, q.token.as_deref()) {
        return json_err(StatusCode::UNAUTHORIZED, "令牌无效");
    }
    let db = state.db.lock().await;
    Json(json!({ "ok": true, "sketches": db.sketches })).into_response()
}

async fn api_admin_sketch_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<TokenQuery>,
    mut multipart: Multipart,
) -> Response {
    if !bearer_ok(&state, &headers, q.token.as_deref()) {
        return json_err(StatusCode::UNAUTHORIZED, "令牌无效");
    }
    let mut bytes: Option<Bytes> = None;
    let (mut title, mut phase, mut element, mut tags, mut note) = (String::new(), String::new(), String::new(), String::new(), String::new());
    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => {
                let name = field.name().unwrap_or("").to_string();
                if name == "file" {
                    match field.bytes().await {
                        Ok(b) => bytes = Some(b),
                        Err(e) => return json_err(StatusCode::BAD_REQUEST, &format!("读取图片失败: {}", e)),
                    }
                } else {
                    let v = field.text().await.unwrap_or_default();
                    match name.as_str() {
                        "title" => title = v,
                        "phase" => phase = v,
                        "element" => element = v,
                        "tags" => tags = v,
                        "note" => note = v,
                        _ => {}
                    }
                }
            }
            Ok(None) => break,
            Err(e) => return json_err(StatusCode::BAD_REQUEST, &format!("表单读取失败: {}", e)),
        }
    }
    let Some(bytes) = bytes else {
        return json_err(StatusCode::BAD_REQUEST, "缺少图片文件");
    };

    let result = {
        let mut db = state.db.lock().await;
        sketch::ingest_bytes(&state.cfg, &mut db, &bytes, &title, &phase, &element, &tags, &note)
    };
    match result {
        Ok(s) => Json(json!({ "ok": true, "sketch": s })).into_response(),
        Err(e) => json_err(StatusCode::BAD_REQUEST, &e),
    }
}

// 让 `Story` 类型在本模块被使用，避免仅测试路径引用
#[allow(dead_code)]
fn _story_type_hint(_: &Story) {}

// ---------- analytics middleware + handlers ----------
async fn track_hit(
    State(state): State<AppState>,
    method: axum::http::Method,
    uri: axum::http::Uri,
    headers: HeaderMap,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Response {
    let start = std::time::Instant::now();
    let path = uri.path().to_string();
    let ip = crate::web::extract_ip(&headers);
    let ua = headers.get("user-agent").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    let referer = headers.get("referer").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    let mut resp = next.run(req).await;
    let status = resp.status().as_u16();
    let latency = start.elapsed().as_millis() as u64;
    let a = state.analytics.clone();
    tokio::spawn(async move {
        a.record_hit(&ip, method.as_str(), &path, status, latency, &ua, &referer).await;
    });
    resp
}

async fn api_track(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::Json(body): axum::Json<serde_json::Value>,
) -> Response {
    let ip = crate::web::extract_ip(&headers);
    let ua = headers.get("user-agent").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    let event = body.get("event").and_then(|v| v.as_str())
        .or_else(|| body.get("kind").and_then(|v| v.as_str()))
        .unwrap_or("event").to_string();
    let extra = body.get("extra").cloned();
    state.analytics.record_event(&ip, &event, &ua, extra).await;
    Json(json!({ "ok": true })).into_response()
}

async fn api_stats(State(state): State<AppState>) -> Response {
    let s = state.analytics.summary(7);
    Json(json!({"ok": true, "window": s.window_days, "today": s.today, "total": s.total,
        "paths": s.recent_paths, "events": s.recent_events,
        "dir_bytes": s.dir_bytes, "disk_usage_pct": s.disk_usage_pct})).into_response()
}

async fn api_maintenance(State(state): State<AppState>) -> Response {
    let r = state.analytics.daily_maintenance(7, 90);
    Json(json!({"ok": true, "compressed": r.compressed, "deleted": r.deleted,
        "freed_bytes": r.freed_bytes, "total_bytes": r.total_bytes,
        "disk_usage_pct": r.disk_usage_pct})).into_response()
}

async fn stats_html(State(state): State<AppState>) -> Response {
    let s = state.analytics.summary(7);
    crate::web::html_page(crate::analytics::render_stats_html(&s))
}
