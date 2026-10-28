# Wayhug AI Engine MVP

> wayhug.com 自有 AI 引擎 — Minimum Viable Product
> 服务内容：博客 / 个人空间 · 记录文化艺术学习快乐的点点滴滴

## Rust 版（四合一，推荐）

`rust/` 是单二进制重写版：把原来 `/lab` 下的四套实验方案（静态 IP 展厅 / 邮件指令站 / TXT→动画生成器 / IP 体验权门户）合并为同一条公众动线——

```
看（展厅） → 写（生成器） → 领（体验权证书） → 寄（邮件投稿）
```

公众页面一页式、扁平化；申请、原画喂养、邮件轮询等复杂度收在 `/studio`。
世界观采用四分法（四相 × 四属性），记录在 [`rust/docs/worldview.md`](rust/docs/worldview.md)。

```bash
cd rust
cargo run --release     # http://127.0.0.1:3001
cargo test              # 19 项：解析 / 动画 / PDF / 世界观 / 文稿统计 / 原画 / 内容边界
```

部署、环境变量、API 与命令行说明见 [`rust/README.md`](rust/README.md)。
下方内容为 Node 版历史实现与 `/lab` 对照页，可继续并行运行。

## 架构设计（分层三角形）

```
┌─────────────────────────────────────────────┐
│  交付层  台词(Dialogue) · VS垂直切片 · PDF    │
├─────────────────────────────────────────────┤
│  业务层  商业(Business) · 动画(Animation)     │
├─────────────────────────────────────────────┤
│  引擎层  动物角色 · 逻辑引擎 · 数据            │
├─────────────────────────────────────────────┤
│  服务层  界面(UI) · HS (HTTP Server)          │
├─────────────────────────────────────────────┤
│  内核层  CSS · TXT (内容格式)                 │
└─────────────────────────────────────────────┘
```

## 功能特性

- 🧠 **逻辑引擎**：基于规则/模板的 AI 推理，可扩展接入真实 LLM
- 🐾 **角色系统**：拟人化动物角色，每个角色有独特性格与说话风格
- 💬 **台词生成**：根据角色性格 + 主题生成对话/独白内容
- 💾 **数据存储**：JSON 文件存储，零配置开箱即用
- 🌐 **HTTP API**：RESTful 接口，供网站前端调用
- 📄 **PDF 导出**：将生成的内容导出为 PDF
- 🎨 **Web 界面**：内置简洁的管理界面

## 快速开始

```bash
# 安装依赖
npm install

# 初始化示例数据
npm run seed

# 启动服务
npm start
```

启动后访问：http://localhost:3000

## IP 实验场（`/lab`）：四套落地方案

用一份 TXT 故事生成扁平化动物动画，并以 A4「IP 体验权」证书完成商业闭环。
四套方案共用同一套五层架构，可独立上线、独立评估。

| 方案 | 路径 | 核心能力 | 适合阶段 |
|------|------|----------|----------|
| S1 静态 IP 展厅 | `/lab/s1` | 预生成动画 + 邮件索取证书 | 品牌首发、零运维 |
| S2 邮件指令站 | `/lab/s2` | 邮件投稿 TXT → 自动回动画与 PDF / 人工答复 | 复刻 jshnp.com 习惯 |
| S3 在线生成器 | `/lab/s3` | 浏览器生成 JSON / CSS / HTML5，一键下载 | 演示、教学、获客 |
| S4 IP 体验权门户 | `/lab/s4` | 申请表 → 编号证书 → 自动/人工答复 | 商业化主场 |

### 数据流

```
TXT（唯一事实源）
  └─ 内核层 story.js：标题 / 段落 / 场景 / 角色 / 情绪 / 指纹
      └─ 引擎层 animate.js：JSON 场景图 + CSS 逐幕关键帧 + HTML5 自包含动画
          ├─ 服务层 routes.js / mail.js：HTTP + SMTP + IMAP 指令
          ├─ 业务层：体验申请 → 证书编号 → 签发 / 答复
          └─ 交付层 certificate.js：A4 矢量证书 PDF（中文字体自动探测）
```

### 新增目录

```
src/lab/
  config.js       环境变量与能力开关（未配置即降级）
  story.js        TXT 解析器（内核层）
  animate.js      JSON / CSS / HTML5 动画生成（引擎层）
  certificate.js  A4 证书 PDF（交付层）
  font.js         中文字体探测（ttc 用 PostScript 名）
  store.js        data/lab.json 存储
  mail.js         SMTP 发信 + IMAP 指令轮询
  routes.js       /lab 路由（服务层）
public/lab/       扁平化页面（总览 + S1~S4 + 后台）
stories/demo.txt  演示作品《小狐与月亮邮局》
scripts/lab-selftest.mjs  解析 / 动画 / 证书自检
assets/fonts/     可选：放入 .ttf/.ttc 即可用于中文证书
```

### 邮件指令（与 jshnp.com 隔离）

- 访客：主题 `[WAYHUG] 故事名`，正文粘贴 TXT（或附 `#wayhug`）。
- 站长（`LAB_OWNER_EMAIL`）：正文首行 `列表` 查待办；`回复 WH-... 正文` 代发答复与证书。
- 去重：messageId；白名单：`MAIL_IMAP_SENDERS`。
- 与 jshnp.com 共用同一邮箱时，只处理带 `[WAYHUG]` 标记的邮件，互不抢占。

### 配置

复制 `.env.example` 为 `.env`。关键项：`LAB_ADMIN_TOKEN`（后台令牌）、`LAB_AUTO_REPLY`、
`SMTP_*`（发信）、`LAB_IMAP_ENABLED` + `MAIL_IMAP_*`（S2 轮询）。
未配置 SMTP/IMAP 时，证书仍可生成与下载，申请进入后台待办，由站长人工答复。

### 自检

```bash
npm run lab:selftest
```

输出解析结果、三件套断言、证书体积与所用字体；证书样例写入 `logs/lab-cert-selftest.pdf`。

### 服务器部署（systemd + Apache 反代）

```ini
# /etc/systemd/system/wayhug-lab.service
[Unit]
Description=Wayhug IP Lab (Express)
After=network.target

[Service]
Type=simple
User=ubuntu
WorkingDirectory=/opt/wayhug.com
EnvironmentFile=/opt/wayhug.com/.env
ExecStart=/usr/bin/node src/server.js
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

```apache
# 站点根目录保持静态落地页，仅把 /lab 反代给 Node
ProxyPreserveHost On
ProxyPass        /lab http://127.0.0.1:3001/lab
ProxyPassReverse /lab http://127.0.0.1:3001/lab
```

中文证书：`sudo apt install -y fonts-wqy-microhei`（或把字体放入 `assets/fonts/`）。

## API 接口

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/characters` | 获取所有角色 |
| GET | `/api/characters/:id` | 获取指定角色 |
| POST | `/api/dialogue/generate` | 生成台词 |
| GET | `/api/contents` | 获取已生成内容列表 |
| GET | `/api/contents/:id/pdf` | 导出内容为 PDF |

### 生成台词示例

```bash
curl -X POST http://localhost:3000/api/dialogue/generate \
  -H "Content-Type: application/json" \
  -d '{"characterId": "fox", "topic": "秋天的落叶", "mood": "感慨"}'
```

## 技术栈

- Node.js + Express (HTTP Server)
- PDFKit (PDF 生成)
- JSON 文件存储

## 后续扩展方向

- 接入真实 LLM API（OpenAI /  Claude / 国产大模型）
- 数据库升级（SQLite → PostgreSQL）
- 动画层（CSS Animation / Canvas）
- 商业化模块（会员、付费内容）

## License

MIT © 2026 王元会
