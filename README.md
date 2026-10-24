# Wayhug AI Engine MVP

> wayhug.com 自有 AI 引擎 — Minimum Viable Product
> 服务内容：博客 / 个人空间 · 记录文化艺术学习快乐的点点滴滴

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
