# wayhug.com · Rust 版

一个二进制、五个层、一条公众动线。

原来的四套实验方案（S1 静态 IP 展厅、S2 邮件指令站、S3 在线 TXT → 动画生成器、S4 IP 体验权门户）已经合并为**同一个网站的同一条动线**：

```
看（展厅） → 写（生成器） → 领（体验权） → 寄（邮件）
```

公众页面只有这一条线；申请、原画、邮件轮询等复杂度全部收在 `/studio`。

---

## 1. 快速开始

```bash
cd rust
cargo run --release
# 打开 http://127.0.0.1:3001
```

自检（不启动服务，验证解析 / 动画 / 证书 / 世界观）：

```bash
cargo run --release -- selftest
```

测试：

```bash
cargo test
```

> 证书中的中文字体自动探测：优先单字体 TTF（`simhei.ttf` / 自备 TTF），TTC（`msyh.ttc`、`wqy-microhei.ttc`）会先抽取第 0 号字体再子集化。
> 子集化生效时 A4 证书约 45 KB；失败会回退整字体嵌入（约 20 MB），并在日志里告警。

---

## 2. 五层架构

| 层 | 文件 | 职责 | 产物 |
|----|------|------|------|
| 内核层 | `src/story.rs` | TXT → 标题 / 段落 / 幕 / 角色 / 情绪 / 四相坐标 | 场景图（唯一事实源） |
| 引擎层 | `src/worldview.rs`、`src/animate.rs` | 四相判定、色板推导、CSS 关键帧与 HTML5 舞台 | JSON / CSS / HTML5 |
| 服务层 | `src/mail.rs`、`src/web.rs` | HTTP 路由、SMTP 发信、IMAP 只读轮询 | 页面 / API / 邮件 |
| 业务层 | `src/store.rs`、`src/model.rs` | 作品、申请、证书编号、90 天体验期 | `data/wayhug.json` |
| 交付层 | `src/pages.rs`、`src/certificate.rs`、`src/fontpack.rs` | 公众页面、后台、A4 PDF、字体子集化 | HTML / PDF |

数据喂养层 `src/sketch.rs` 独立于五层之外，面向作者，不进入公众页面。

设计约束见 [`docs/worldview.md`](docs/worldview.md)：四相 × 四属性、叙事四拍、扁平化不变式。

---

## 3. 配置（环境变量）

所有能力都可缺省：未配置的能力自动降级，不影响网站其它部分。

| 变量 | 说明 | 默认 |
|------|------|------|
| `WAYHUG_PORT` | 监听端口 | `3001` |
| `WAYHUG_SITE_URL` | 对外站点地址（写入证书校验链接） | `https://wayhug.com` |
| `WAYHUG_CONTACT_EMAIL` | 公开联系邮箱 | `bc.xu@hotmail.com` |
| `WAYHUG_STORY_EMAIL` | 投稿/自动回信地址 | 发件邮箱或联系邮箱 |
| `WAYHUG_OWNER_NAME` | 证书授权人署名 | `王元会` |
| `WAYHUG_ADMIN_TOKEN` | `/studio` 与后台 API 令牌 | 空（后台不可用） |
| `WAYHUG_WRITE_TOKEN` | 公开页「写」的写作口令；访问 `/?wkey=...`（或 `/#wkey=...`）即解锁 | 空（仅后台令牌可用） |
| `WAYHUG_FREE_UNTIL` | MIT 免费授权截止日 | `2026-12-31` |
| `WAYHUG_AUTO_REPLY` | 是否自动回信 | `false`（转人工） |
| `WAYHUG_MAIL_PREFIX` | 邮件主题前缀 | `[WAYHUG]` |
| `WAYHUG_DATA` | 数据文件 | `data/wayhug.json` |
| `WAYHUG_SKETCH_DIR` | 原画目录 | `data/sketches` |
| `WAYHUG_FONT_PATH` | 指定证书字体 | 自动探测 |
| `WAYHUG_SMTP_HOST/PORT/SECURE/USER/PASS` | 发信 | — |
| `WAYHUG_IMAP_ENABLED/HOST/PORT/SECURE/USER/PASS` | 收信 | `false` / 993 / TLS |
| `WAYHUG_IMAP_SENDERS` | 允许的投稿发件人（逗号分隔） | 空 |
| `WAYHUG_IMAP_INTERVAL_SECS` | 轮询间隔（≥60） | `180` |

为兼容旧部署，同时接受 `LAB_*`、`SMTP_*`、`MAIL_IMAP_*` 前缀。

---

## 4. 命令行

```bash
wayhug                              # 启动服务
wayhug selftest                     # 解析 / 动画 / PDF / 世界观自检
wayhug ingest-stories <文件或目录>    # 导入 TXT 文稿（首页「文稿集合」自动更新）
wayhug ingest-sketch <图> [标题] [四相] [四属性]
wayhug ingest-dir <目录>             # 批量喂养素描原画
wayhug render2d <编号|demo|all> [目录] # 导出 256 色 800×600 画布版 HTML
```

文稿的放置与迭代约定见仓库根目录 [`manuscripts/README.md`](../manuscripts/README.md)：

```bash
cd rust
cargo run --release -- ingest-stories ../manuscripts
```

内容边界：不收录宗教、政治内容（`src/guard.rs`，网页 / 邮件 / 命令行共用）。
网站只有动画，没有音乐、没有音效。

四相（世界层）：`yuan|xian|chong|ren`（原/仙/虫/人）
光时：`dawn|day|dusk|night`（晨/昼/暮/夜，只影响节奏与拍子）
四属性：`wind|fire|water|earth`（风/火/水/土）

| 四相 | 界 | 招牌动物 | 性格 | 主色 |
|------|----|----------|------|------|
| 原 | 自然界 | 狐狸 | 容易害怕 | `#ff6633` |
| 仙 | 灵气 | 小妖怪 | 喜欢笑着 | `#009999` |
| 虫 | 色界生命 | 四足小鳄虫 | 会走路 | `#4d4d4d` |
| 人 | 欲界的六道 | 农夫 | 耕田、收成、养家 | `#cc9966` |

对外颜色一律经 256 色调色板贴色（`src/palette.rs`），画面规格见 [`docs/canvas2d.md`](docs/canvas2d.md)。

例：

```bash
wayhug ingest-sketch sketches/fox-01.png "小狐初稿 01" yuan wind
wayhug ingest-dir sketches/
wayhug render2d demo data/render2d
```

原画会：校验格式 → 计算 sha256 → 取主色 → 给出四相建议 → 落盘 `data/sketches/` → 登记到 `data/wayhug.json`。
同一张图重复喂养不会产生重复记录。

---

## 5. 公众动线

| 地址 | 作用 |
|------|------|
| `/` | 一页式公众界面：看 / 写 / 领 / 寄 / 四相 |
| `/watch/{id}` | 播放动画（`?still=1&scene=N` 定格） |
| `/watch2d/{id}` | 256 色画布版（800×600，纯 HTML5 Canvas，无音频） |
| `/assets/realm/{key}.svg` | 四相招牌动物原创 SVG（yuan/xian/chong/ren） |
| `/dl/{id}/html|css|json` | 下载三件套 |
| `/dl/{id}/canvas` | 下载画布版单文件 HTML |
| `/verify/{id}` | 证书在线校验 |
| `/cert/{id}.pdf` | A4 证书 PDF |
| `/studio` | 后台工作台（令牌） |

## 6. API

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/info` | 站点能力与统计 |
| GET | `/api/stories` | 作品列表 |
| GET | `/api/story/{id}` | 作品 + 场景图 |
| GET | `/api/demo.txt` | 演示故事原文 |
| GET | `/api/worldview` | 四相世界观 JSON |
| GET | `/api/corpus` | 文稿集合统计（四相占比、最新文稿；首页「文稿校准」同源） |
| GET | `/api/verify/{id}` | 证书校验 |
| POST | `/api/generate` | `{title?, text}` → 生成动画（需写作口令或后台令牌） |
| POST | `/api/experience` | `{name,email,storyId?,message?}` → 领取证书 |
| GET | `/license` | MIT 许可全文（免费期至 `WAYHUG_FREE_UNTIL`） |
| GET | `/api/admin/requests` | 申请列表（令牌） |
| POST | `/api/admin/reply` | `{id,text}` 人工答复（令牌） |
| POST | `/api/admin/poll` | 立即轮询 IMAP（令牌） |
| GET/POST | `/api/admin/sketches` | 原画列表 / 上传（令牌） |

令牌可放在请求头 `x-wayhug-token` 或查询参数 `?token=`；写作口令放在请求头 `x-wayhug-write`，也可用 `wayhug_write` cookie（访问 `/?wkey=口令` 后由页面写入，作用域 `.wayhug.com`，有效期一年）。
页面 HTML 一律 `Cache-Control: no-store`，避免浏览器缓存旧的内联脚本导致口令失效。

写入限流：同一 IP 10 分钟最多 30 次（仍需写作口令）。

---

## 7. 邮件

- **发信**：SMTP；证书 PDF 作为附件；未配置或失败时自动降级为「人工答复」，不丢单。生产默认 `WAYHUG_AUTO_REPLY=false`，访客在网页即时下载，人工联系走邮箱。
- **收信**：IMAP 只读轮询；不改 `\Seen`、不删除，可与 jshnp.com 共用同一邮箱；按 `Message-ID` 去重。当前生产关闭，站长人工看信。
- **投稿**：默认由站长通过写作口令生成或 `ingest-stories` 导入；邮件投稿链路保留，启用 IMAP 后可用。
- **人工**：站长在 `/studio` 填写答复，系统回信并把状态改为「已人工答复」。

---

## 8. 数据与部署

```
data/
  wayhug.json          作品 / 申请 / 原画元数据（原子写入）
  wayhug.json.tmp      写入中间文件
  sketches/            原画文件
  fonts/               TTC 抽取出的证书字体缓存
```

- 与 Node 版 `data/lab.json` 完全隔离，可并行灰度、随时回滚。
- 单二进制部署：`cargo build --release` 后复制 `target/release/wayhug`，用 systemd 常驻，Apache/Nginx 反代到本端口。
- 文稿导入后需重启常驻服务（`systemctl restart wayhug-rust`），让内存中的数据库重新加载 JSON。
- 若本机无法直连 GitHub：先在服务器保留镜像 `~/wayhug-mirror.git`，本地 `git remote add cloud ssh://ubuntu@129.211.170.72/home/ubuntu/wayhug-mirror.git`，`git push cloud master`，再在服务器执行 `git -C ~/wayhug-mirror.git push origin master`。
- 建议生产环境设置 `WAYHUG_ADMIN_TOKEN`、`WAYHUG_SITE_URL` 与邮件账号；其余保持默认即可。

---

## 9. 与 Node 版的关系

Node 版（仓库根目录）仍保留 `/lab/*` 四套独立实验页，作为历史与对照。
Rust 版把四套方案合并成一条动线，并保留旧链接重定向：

```
/lab        → /
/lab/s1     → /#watch
/lab/s2     → /#mail
/lab/s3     → /#create
/lab/s4     → /#experience
/lab/story/{id}  → /watch/{id}
/lab/cert/{id}   → /cert/{id}.pdf
/lab/verify/{id} → /verify/{id}
```
