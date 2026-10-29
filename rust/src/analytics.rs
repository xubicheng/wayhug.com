//! 轻量追踪（零新依赖）
//!
//! JSONL 每日滚动 · >7 天 gzip · >90 天删除 · 磁盘监控

use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use tokio::sync::Mutex;

// ---------- Analytics ----------
#[derive(Debug)]
pub struct Analytics { dir: PathBuf, inner: Mutex<Inner> }
#[derive(Debug, Default)] struct Inner { date: String }
impl Clone for Analytics { fn clone(&self) -> Self {
    Self { dir: self.dir.clone(), inner: Mutex::new(Inner::default()) }
}}

impl Analytics {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        if let Err(e) = std::fs::create_dir_all(&dir) {
            eprintln!("[analytics] mkdir {}: {}", dir.display(), e);
        }
        Self { dir, inner: Mutex::new(Inner::default()) }
    }
    pub fn dir(&self) -> &Path { &self.dir }

    async fn append(&self, line: &str) {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let path = self.dir.join(format!("analytics-{}.jsonl", today));
        match OpenOptions::new().create(true).append(true).open(&path) {
            Ok(mut f) => { let _ = f.write_all(line.as_bytes()); let _ = f.write_all(b"\n"); }
            Err(e) => eprintln!("[analytics] append {}: {}", path.display(), e),
        }
    }

    pub async fn record_hit(
        &self, ip: &str, method: &str, path: &str, status: u16,
        latency_ms: u64, ua: &str, referer: &str,
    ) {
        let rec = HitRecord {
            ts: Local::now().format("%Y-%m-%dT%H:%M:%S%:z").to_string(),
            ip: ip.to_string(), method: method.to_string(), path: path.to_string(),
            status, latency_ms,
            ua: truncate(ua, 200), referer: truncate(referer, 300),
            kind: "hit".into(),
        };
        if let Ok(s) = serde_json::to_string(&rec) { self.append(&s).await; }
    }

    pub async fn record_event(&self, ip: &str, event: &str, ua: &str, extra: Option<serde_json::Value>) {
        let rec = EventRecord {
            ts: Local::now().format("%Y-%m-%dT%H:%M:%S%:z").to_string(),
            ip: ip.to_string(), kind: "event".into(),
            event: event.to_string(), ua: truncate(ua, 200), extra,
        };
        if let Ok(s) = serde_json::to_string(&rec) { self.append(&s).await; }
    }

    /// 每日维护：gzip >7天原始 → 删除 >90天原始
    pub fn daily_maintenance(&self, compress_days: u32, delete_days: u32) -> MaintenanceReport {
        let mut r = MaintenanceReport::default();
        let today = Local::now().date_naive();
        if let Ok(entries) = std::fs::read_dir(&self.dir) {
            for e in entries.flatten() {
                let path = e.path();
                let f = path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
                if !f.starts_with("analytics-") || !f.ends_with(".jsonl") { continue; }
                let dp = f["analytics-".len()..f.len()-".jsonl".len()].to_string();
                if dp == today.format("%Y-%m-%d").to_string() { continue; }
                let fd = match chrono::NaiveDate::parse_from_str(&dp, "%Y-%m-%d") { Ok(d) => d, Err(_) => continue };
                let age = (today - fd).num_days();
                if age > delete_days as i64 {
                    if let Ok(sz) = std::fs::metadata(&path) { r.freed_bytes += sz.len(); }
                    let _ = std::fs::remove_file(&path); r.deleted += 1; continue;
                }
                if age > compress_days as i64 {
                    let gz = path.with_extension("jsonl.gz");
                    if !gz.exists() {
                        if let Ok(data) = std::fs::read(&path) {
                            if std::fs::write(&gz, &data).is_ok() {
                                if std::process::Command::new("gzip").arg("-f").arg(&gz).status().is_ok() {
                                    let _ = std::fs::remove_file(&path); r.compressed += 1;
                                } else { let _ = std::fs::remove_file(&gz); }
                            }
                        }
                    }
                }
            }
        }
        r.total_bytes = dir_size(&self.dir);
        r.disk_usage_pct = disk_usage_pct();
        if r.total_bytes > 100 * 1024 * 1024 {
            eprintln!("[analytics] ⚠️ >100MB ({:.1} MB)", r.total_bytes as f64 / 1048576.0);
        }
        r
    }

    pub fn summary(&self, days: u32) -> StatsSummary {
        let mut today = DailyStats::default();
        let mut total = DailyStats::default();
        let mut today_ips: HashSet<String> = HashSet::new();
        let mut total_ips: HashSet<String> = HashSet::new();
        let mut paths: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
        let mut events: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
        let today_str = Local::now().format("%Y-%m-%d").to_string();

        if let Ok(entries) = std::fs::read_dir(&self.dir) {
            for e in entries.flatten() {
                let path = e.path();
                let f = path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
                if !f.starts_with("analytics-") { continue; }
                let is_gz = f.ends_with(".gz");
                let ext = if is_gz { ".jsonl.gz" } else { ".jsonl" };
                let ext_len = ext.len();
                let dp = f["analytics-".len()..f.len()-ext_len].to_string();
                let fd = match chrono::NaiveDate::parse_from_str(&dp, "%Y-%m-%d") { Ok(d) => d, Err(_) => continue };
                let age = (Local::now().date_naive() - fd).num_days();
                if age > days as i64 { continue; }
                if is_gz { continue; } // gz 不解压省内存
                let content = match std::fs::read_to_string(&path) { Ok(s) => s, Err(_) => continue };
                let is_today = dp == today_str;
                for line in content.lines() {
                    if line.is_empty() { continue; }
                    if let Ok(h) = serde_json::from_str::<HitRecord>(line) {
                        update_daily(&mut total, &h); total_ips.insert(h.ip.clone());
                        if is_today { update_daily(&mut today, &h); today_ips.insert(h.ip); }
                        *paths.entry(h.path).or_insert(0) += 1;
                    } else if let Ok(ev) = serde_json::from_str::<EventRecord>(line) {
                        let is_pv = ev.event == "pageview";
                        let is_click = ev.event == "click";
                        let is_dl = ev.event.starts_with("download");
                        if is_pv { total.events_pageview += 1; if is_today { today.events_pageview += 1; } }
                        if is_click { total.events_click += 1; if is_today { today.events_click += 1; } }
                        if is_dl { total.events_download += 1; if is_today { today.events_download += 1; } }
                        *events.entry(ev.event).or_insert(0) += 1;
                    }
                }
            }
        }
        today.uv = today_ips.len() as u64;
        total.uv = total_ips.len() as u64;
        let mut paths_v: Vec<_> = paths.into_iter().map(|(p, h)| PathHit { path: p, hits: h }).collect();
        paths_v.sort_by(|a, b| b.hits.cmp(&a.hits)); paths_v.truncate(12);
        let mut events_v: Vec<_> = events.into_iter().map(|(e, c)| EventHit { event: e, count: c }).collect();
        events_v.sort_by(|a, b| b.count.cmp(&a.count)); events_v.truncate(12);
        StatsSummary {
            today, total, recent_paths: paths_v, recent_events: events_v, window_days: days,
            dir_bytes: dir_size(&self.dir), disk_usage_pct: disk_usage_pct(),
        }
    }
}

fn truncate(s: &str, max: usize) -> String { if s.chars().count() <= max { s.to_string() } else { s.chars().take(max).collect() } }
fn update_daily(st: &mut DailyStats, h: &HitRecord) {
    st.pv += 1;
    match h.status { 200..=299 => st.hits_2xx += 1, 300..=399 => st.hits_3xx += 1,
                     400..=499 => st.hits_4xx += 1, _ => st.hits_5xx += 1 }
    if h.path == "/api/random-audio" || h.path == "/api/track_download" { st.api_download += 1; }
    if h.path == "/api/track" { st.api_track += 1; }
}
fn dir_size(p: &Path) -> u64 {
    let mut t = 0;
    if let Ok(e) = std::fs::read_dir(p) { for x in e.flatten() {
        if let Ok(m) = x.metadata() { if m.is_file() { t += m.len(); } }
    }}
    t
}
fn disk_usage_pct() -> f64 {
    match std::process::Command::new("df").arg("-P").arg("/").output() {
        Ok(o) => {
            for line in String::from_utf8_lossy(&o.stdout).lines().skip(1) {
                if let Some(c) = line.split_whitespace().nth(4) {
                    return c.trim_end_matches('%').parse::<f64>().unwrap_or(0.0);
                }
            } 0.0
        } Err(_) => 0.0,
    }
}

// ---------- 数据结构 ----------
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HitRecord {
    #[serde(rename = "ts")] pub ts: String,
    #[serde(rename = "ip")] pub ip: String,
    #[serde(rename = "m")] pub method: String,
    #[serde(rename = "p")] pub path: String,
    #[serde(rename = "s")] pub status: u16,
    #[serde(rename = "l")] pub latency_ms: u64,
    #[serde(default, skip_serializing_if = "String::is_empty", rename = "ua")] pub ua: String,
    #[serde(default, skip_serializing_if = "String::is_empty", rename = "ref")] pub referer: String,
    #[serde(rename = "kind")] pub kind: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EventRecord {
    #[serde(rename = "ts")] pub ts: String,
    #[serde(rename = "ip")] pub ip: String,
    #[serde(rename = "kind")] pub kind: String,
    #[serde(rename = "ev")] pub event: String,
    #[serde(default, skip_serializing_if = "String::is_empty", rename = "ua")] pub ua: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "extra")] pub extra: Option<serde_json::Value>,
}
#[derive(Debug, Serialize, Default)] pub struct DailyStats {
    pub pv: u64, pub uv: u64,
    pub hits_2xx: u64, pub hits_3xx: u64, pub hits_4xx: u64, pub hits_5xx: u64,
    pub api_track: u64, pub api_download: u64,
    pub events_pageview: u64, pub events_click: u64, pub events_download: u64,
}
#[derive(Debug, Serialize, Default)] pub struct MaintenanceReport {
    pub deleted: u32, pub compressed: u32, pub freed_bytes: u64,
    pub total_bytes: u64, pub disk_usage_pct: f64,
}
#[derive(Debug, Serialize)] pub struct PathHit { pub path: String, pub hits: u64 }
#[derive(Debug, Serialize)] pub struct EventHit { pub event: String, pub count: u64 }
#[derive(Debug, Serialize)] pub struct StatsSummary {
    pub today: DailyStats, pub total: DailyStats,
    pub recent_paths: Vec<PathHit>, pub recent_events: Vec<EventHit>,
    pub window_days: u32, pub dir_bytes: u64, pub disk_usage_pct: f64,
}

// ---------- /stats HTML ----------
pub fn render_stats_html(s: &StatsSummary) -> String {
    let fc = |label: &str, val: u64| format!(
        "<div class=\"card\"><div class=\"label\">{}</div><div class=\"val\">{}</div></div>", label, val);

    let mut ph = String::new();
    for p in &s.recent_paths {
        ph.push_str(&format!("<tr><td class=\"mono\">{}</td><td>{}</td></tr>", p.path.replace('<', "&lt;"), p.hits));
    }
    let mut eh = String::new();
    for e in &s.recent_events {
        eh.push_str(&format!("<tr><td class=\"mono\">{}</td><td>{}</td></tr>", e.event.replace('<', "&lt;"), e.count));
    }
    let kb = s.dir_bytes as f64 / 1024.0;
    let dir_disp = if kb > 1024.0 { format!("{:.1} MB", kb/1024.0) } else { format!("{:.0} KB", kb) };

    let tc = format!("{}{}{}{}", fc("今日 PV", s.today.pv), fc("今日 UV", s.today.uv),
                     fc("下载次数", s.today.api_download), fc("5xx 错误", s.today.hits_5xx));
    let tlc = format!("{}{}{}{}", fc(&format!("{}天 PV", s.window_days), s.total.pv),
                      fc(&format!("{}天 UV", s.window_days), s.total.uv),
                      fc("总下载", s.total.api_download),
                      fc("4xx+5xx", s.total.hits_4xx + s.total.hits_5xx));

    format!(r##"<!doctype html>
<html lang="zh-cn"><head><meta charset="utf-8"><title>追踪面板</title>
<style>
body{{font-family:-apple-system,"Segoe UI",system-ui,sans-serif;max-width:720px;margin:32px auto;padding:0 20px;background:#fafafa;color:#2a2018}}
h1{{margin:0 0 4px;font-size:22px;color:#a66a3a;letter-spacing:4px}}
.sub{{font-size:12px;color:#7a6a52;margin-bottom:20px}}
h2{{margin:28px 0 12px;font-size:13px;color:#7a6a52;letter-spacing:.12em;text-transform:uppercase;border-bottom:1px solid #d8cdb8;padding-bottom:6px}}
.grid{{display:grid;grid-template-columns:repeat(4,1fr);gap:12px}}
.card{{background:#fff;border:1px solid #d8cdb8;border-radius:8px;padding:14px}}
.label{{font-size:11px;color:#7a6a52;margin-bottom:4px;letter-spacing:.08em;text-transform:uppercase}}
.val{{font-size:22px;font-weight:700;color:#2a2018}}
.mono{{font-family:"SF Mono",Consolas,monospace;font-size:12px;color:#7a6a52}}
table{{width:100%;border-collapse:collapse;background:#fff;border:1px solid #d8cdb8;border-radius:8px;overflow:hidden}}
td{{padding:8px 12px;border-bottom:1px solid #d8cdb8;font-size:13px}}
tr:last-child td{{border-bottom:0}}
td:last-child{{text-align:right;font-variant-numeric:tabular-nums;color:#7a6a52}}
.foot{{margin-top:32px;text-align:center;font-size:11px;color:#a08968;letter-spacing:.06em;padding-top:16px;border-top:1px dashed #d8cdb8}}
</style></head><body>
<h1>追踪面板</h1>
<p class="sub">analytics-YYYY-MM-DD.jsonl · 保留 90 天 · 磁盘 {dpct:.0}%</p>
<h2>今日</h2><div class="grid">{tc}</div>
<h2>{win} 天累计</h2><div class="grid">{tlc}</div>
<h2>热门路径</h2><table>{ph}</table>
<h2>事件分布</h2><table>{eh}</table>
<p class="foot">analytics {dir} · {tb} bytes · {dpct:.0}% 磁盘</p>
</body></html>"##,
        tc=tc, tlc=tlc, ph=ph, eh=eh,
        win=s.window_days, dir=dir_disp, tb=s.dir_bytes, dpct=s.disk_usage_pct,
    )
}
