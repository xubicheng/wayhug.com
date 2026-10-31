/**
 * Wayhug AI Engine - HTTP 服务器入口 (HS)
 * 分层架构：界面层 → 服务层(API) → 引擎层 → 数据层
 */

import 'dotenv/config';
import express from 'express';
import { fileURLToPath } from 'node:url';
import { join, dirname } from 'node:path';
import apiRouter from './routes/api.js';
import webRouter from './routes/web.js';
import labRouter from './lab/routes.js';
import { startMailService } from './lab/mail.js';
import { ensureDemo } from './lab/store.js';
import { streamContentPDF } from './render/pdf.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const app = express();
const PORT = process.env.PORT || 3000;

// 中间件
app.use(express.json({ limit: '1mb' }));
app.use(express.urlencoded({ extended: true }));
app.use(express.static(join(__dirname, '..', 'public')));

// 视图模板（HTML 直接返回）
app.set('views', join(__dirname, '..', 'views'));

// API 路由
app.use('/api', apiRouter);

// 实验场：四套 IP 落地方案（S1~S4）
app.use('/lab', labRouter);

// PDF 导出路由（交付层）
app.get('/contents/:id/pdf', (req, res) => {
  streamContentPDF(req.params.id, res);
});

// Web 界面路由
app.use('/', webRouter);

// 健康检查
app.get('/health', (req, res) => {
  res.json({ status: 'ok', service: 'wayhug-ai-engine', version: '0.1.0' });
});

// 404
app.use((req, res) => {
  res.status(404).sendFile(join(__dirname, '..', 'views', '404.html'));
});

// 错误处理
app.use((err, req, res, next) => {
  // JSON 解析等客户端输入错误返回 400，而非笼统的 500
  if (err.type === 'entity.parse.failed' || err.status === 400) {
    return res.status(400).json({ code: 400, message: '请求体不是合法的 JSON' });
  }
  console.error('[server] 未捕获错误:', err);
  res.status(500).json({ code: 500, message: '服务器内部错误' });
});

app.listen(PORT, () => {
  try {
    ensureDemo();
  } catch (err) {
    console.warn('[lab] 演示作品初始化失败:', err.message);
  }
  startMailService();
  console.log('');
  console.log('  ╔══════════════════════════════════════════╗');
  console.log('  ║      Wayhug AI Engine MVP 已启动         ║');
  console.log('  ║      MIT License © 王元会                ║');
  console.log('  ╠══════════════════════════════════════════╣');
  console.log(`  ║  本地:   http://localhost:${PORT}            ║`);
  console.log(`  ║  实验场: http://localhost:${PORT}/lab        ║`);
  console.log(`  ║  API:    http://localhost:${PORT}/api        ║`);
  console.log(`  ║  健康:   http://localhost:${PORT}/health     ║`);
  console.log('  ╚══════════════════════════════════════════╝');
  console.log('');
});

export default app;
