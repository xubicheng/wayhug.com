// ============================================================
// wayhug.com · Rust 单二进制入口
//
// 五层架构：
//   内核层 story     TXT → 场景
//   引擎层 animate   JSON / CSS / HTML5
//   服务层 mail       SMTP / IMAP
//   业务层 store     作品 / 申请 / IP 体验权
//   交付层 pages     公众页面 + studio + A4 PDF
// ============================================================

mod animate;
mod certificate;
mod config;
mod fontpack;
mod guard;
mod mail;
mod model;
mod pages;
mod sketch;
mod store;
mod story;
mod web;
mod worldview;

use config::Config;
use std::path::Path;
use web::AppState;

fn selftest(cfg: &Config) {
    println!("== Wayhug selftest ==");
    let parsed = story::parse_story(story::demo_text(), Some("小狐与月亮邮局"), Some("demo"))
        .expect("演示故事解析失败");
    println!(
        "story: {} 幕 / {} 字符 / {} 角色 / 指纹 {}",
        parsed.stats.scenes, parsed.stats.chars, parsed.stats.cast, parsed.fingerprint
    );
    let html = animate::build_html(&parsed, animate::HtmlOptions::default());
    let css = animate::build_css(&parsed);
    println!("bundle: html {} bytes / css {} bytes", html.len(), css.len());
    assert!(html.contains("<!DOCTYPE html>") && css.contains("@keyframes"));
    let req = model::Request {
        id: String::from("WH-19700101-TEST"),
        name: String::from("测试持有人"),
        email: String::from("test@example.com"),
        story_id: parsed.id.clone(),
        story_title: parsed.title.clone(),
        cast_names: parsed.cast.iter().map(|c| c.name.clone()).collect(),
        scenes: parsed.stats.scenes,
        chars: parsed.stats.chars,
        fingerprint: parsed.fingerprint.clone(),
        status: String::from("pending"),
        source: String::from("selftest"),
        created_at: store::now_iso(),
        expires_at: store::now_iso(),
        ..Default::default()
    };
    match certificate::certificate_pdf(cfg, &req, Some(&parsed)) {
        Ok(bytes) => println!("certificate: {} bytes / PDF header {:?}", bytes.len(), &bytes[..5]),
        Err(e) => panic!("证书生成失败: {}", e),
    }
    println!("worldview: {} 象限", worldview::public_view()["quadrants"].as_array().map(|a| a.len()).unwrap_or(0));
    println!("ok");
}

fn usage() {
    println!(
        "wayhug — 故事到动物的 IP 实验场\n\n\
         用法:\n  wayhug                      启动服务\n  wayhug selftest             解析 / 动画 / PDF 自检\n  wayhug ingest-stories <文件或目录>  导入 TXT 文稿（第一行可作标题）\n  wayhug ingest-sketch <file> [标题] [四相] [四属性]\n  wayhug ingest-dir <目录>     批量喂养素描原画\n\n\
         四相: dawn|day|dusk|night   四属性: wind|fire|water|earth\n\
         内容边界: 不收录宗教、政治内容\n\
         环境变量: WAYHUG_PORT, WAYHUG_ADMIN_TOKEN, WAYHUG_DATA, WAYHUG_SKETCH_DIR,\n\
                   WAYHUG_SMTP_*, WAYHUG_IMAP_*, WAYHUG_AUTO_REPLY, WAYHUG_SITE_URL"
    );
}

/// 导入 TXT 文稿（单文件或目录）：第一行可作标题，自动分幕、判四相
fn ingest_stories(cfg: &Config, path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("路径不存在: {}", path.display()));
    }
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    if path.is_dir() {
        for entry in std::fs::read_dir(path).map_err(|e| format!("读取目录失败: {}", e))? {
            let p = entry.map_err(|e| format!("读取目录失败: {}", e))?.path();
            if p.is_file() && p.extension().map(|x| x.eq_ignore_ascii_case("txt")).unwrap_or(false) {
                files.push(p);
            }
        }
        files.sort();
    } else {
        files.push(path.to_path_buf());
    }
    if files.is_empty() {
        return Err("没有找到 .txt 文稿".into());
    }

    let mut db = store::load(cfg);
    store::ensure_demo(cfg, &mut db);
    let (mut ok, mut skipped) = (0usize, 0usize);
    for f in &files {
        let text = match std::fs::read_to_string(f) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("跳过 {}：读取失败（{}）", f.display(), e);
                skipped += 1;
                continue;
            }
        };
        if let Err(e) = guard::check(&text) {
            eprintln!("跳过 {}：{}", f.display(), e);
            skipped += 1;
            continue;
        }
        let mut parsed = match story::parse_story(&text, None, None) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("跳过 {}：{}", f.display(), e);
                skipped += 1;
                continue;
            }
        };
        parsed.source = "manuscript".into();
        println!("收录 {} · {} 幕 · 《{}》", parsed.id, parsed.stats.scenes, parsed.title);
        db.put_story(parsed);
        ok += 1;
    }
    store::save(cfg, &db)?;
    println!("文稿导入完成：{} 篇收录，{} 篇跳过（共 {} 个文件）", ok, skipped, files.len());
    Ok(())
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let cfg = Config::load();
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "selftest" => {
                selftest(&cfg);
                return;
            }
            "ingest-stories" => {
                if args.len() < 3 {
                    eprintln!("用法: wayhug ingest-stories <TXT 文件或目录>");
                    std::process::exit(2);
                }
                if let Err(e) = ingest_stories(&cfg, Path::new(&args[2])) {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
                return;
            }
            "ingest-sketch" => {
                if args.len() < 3 {
                    eprintln!("用法: wayhug ingest-sketch <图片路径> [标题] [四相] [四属性]");
                    std::process::exit(2);
                }
                let title = args.get(3).map(|s| s.as_str()).unwrap_or("");
                let phase = args.get(4).map(|s| s.as_str()).unwrap_or("");
                let element = args.get(5).map(|s| s.as_str()).unwrap_or("");
                if let Err(e) = sketch::cli_ingest(&cfg, Path::new(&args[2]), title, phase, element) {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
                return;
            }
            "ingest-dir" => {
                if args.len() < 3 {
                    eprintln!("用法: wayhug ingest-dir <目录>");
                    std::process::exit(2);
                }
                if let Err(e) = sketch::cli_ingest_dir(&cfg, Path::new(&args[2])) {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
                return;
            }
            "-h" | "--help" | "help" => {
                usage();
                return;
            }
            other => {
                eprintln!("未知命令: {}（用 --help 查看用法）", other);
                std::process::exit(2);
            }
        }
    }

    if let Some(dir) = cfg.data_path.parent() {
        if let Err(e) = std::fs::create_dir_all(dir) {
            eprintln!("[boot] 数据目录创建失败: {}", e);
        }
    }
    let mut db = store::load(&cfg);
    store::ensure_demo(&cfg, &mut db);

    let state = AppState::new(cfg.clone(), db);
    tokio::spawn(mail::run(cfg.clone(), state.db.clone()));

    let app = web::router(state);
    let addr = format!("0.0.0.0:{}", cfg.port);
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[boot] 端口 {} 绑定失败: {}", cfg.port, e);
            std::process::exit(1);
        }
    };

    println!("wayhug.com · Rust IP Lab");
    println!("  本地   http://127.0.0.1:{}", cfg.port);
    println!("  站点   {}", cfg.site_url);
    println!("  数据   {}", cfg.data_path.display());
    println!("  原画   {}", cfg.sketch_dir.display());
    println!(
        "  能力   SMTP {} / IMAP {} / 自动回信 {} / 后台令牌 {}",
        if cfg.smtp_ready() { "on" } else { "off" },
        if cfg.imap_ready() { "on" } else { "off" },
        if cfg.auto_reply { "on" } else { "off" },
        if cfg.admin_token.is_empty() { "未设置" } else { "已设置" }
    );

    let shutdown = async {
        let _ = tokio::signal::ctrl_c().await;
        println!("[boot] 收到退出信号，正在关闭…");
    };
    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
    {
        eprintln!("[boot] 服务异常退出: {}", e);
    }
}
