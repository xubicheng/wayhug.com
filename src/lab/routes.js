/**
 * 服务层 · 实验场路由（/lab）
 * S1 静态 IP 展厅 / S2 邮件指令站 / S3 在线生成器 / S4 体验权门户
 * 四套方案共享：story 解析、动画生成、证书、存储、邮件。
 */

import { Router } from 'express';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { readFileSync } from 'node:fs';
import { labConfig, capability } from './config.js';
import { parseStory, storyToJSON, MOODS } from './story.js';
import { buildBundle } from './animate.js';
import { streamCertificate, certificateBuffer } from './certificate.js';
import { sendMail, pollOnce } from './mail.js';
import * as store from './store.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, '..', '..');
const PAGE = (name) => join(root, 'public', 'lab', name);
const router = Router();

// —— 轻量限流：同一 IP 10 分钟内最多 30 次写操作 ——
const hits = new Map();
function limited(ip) {
  const now = Date.now();
  const win = 10 * 60 * 1000;
  const arr = (hits.get(ip) || []).filter((t) => now - t < win);
  arr.push(now);
  hits.set(ip, arr);
  return arr.length > 30;
}

function publicStory(story) {
  return {
    id: story.id,
    title: story.title,
    createdAt: story.createdAt,
    fingerprint: story.fingerprint,
    stats: story.stats,
    cast: story.cast,
    moodCount: story.moodCount,
  };
}

function adminOk(req) {
  const token = req.query.token || req.get('x-lab-token') || req.body?.token;
  return Boolean(labConfig.adminToken) && token === labConfig.adminToken;
}

function requireAdmin(req, res, next) {
  if (!labConfig.adminToken) return res.status(503).json({ code: 503, message: '后台未启用：请设置 LAB_ADMIN_TOKEN' });
  if (!adminOk(req)) return res.status(401).json({ code: 401, message: '令牌无效' });
  next();
}

// ———————————————— 页面 ————————————————
router.get('/', (req, res) => res.sendFile(PAGE('index.html')));
['s1', 's2', 's3', 's4', 'admin'].forEach((name) => {
  router.get(`/${name}`, (req, res) => res.sendFile(PAGE(`${name}.html`)));
});

// ———————————————— 基础信息 ————————————————
router.get('/api/info', (req, res) => {
  const demo = store.ensureDemo();
  res.json({
    code: 0,
    data: {
      name: 'Wayhug IP Lab',
      siteUrl: labConfig.siteUrl,
      basePath: labConfig.basePath,
      contactEmail: labConfig.contactEmail,
      capability: capability(),
      stats: store.stats(),
      demo: demo ? publicStory(demo) : null,
      solutions: [
        { id: 's1', name: '静态 IP 展厅', tagline: '预生成动画 + 邮件索取证书' },
        { id: 's2', name: '邮件指令站', tagline: '发一封 TXT 邮件，自动回动画与 PDF' },
        { id: 's3', name: '在线 TXT→动画生成器', tagline: '浏览器内生成 JSON / CSS / HTML5' },
        { id: 's4', name: 'IP 体验权门户', tagline: '申请表 → 编号证书 → 自动/人工答复' },
      ],
    },
  });
});

router.get('/api/stories', (req, res) => {
  res.json({ code: 0, data: store.listStories(Number(req.query.limit) || 20) });
});

router.get('/api/story/:id', (req, res) => {
  const story = store.getStory(req.params.id);
  if (!story) return res.status(404).json({ code: 404, message: '作品不存在' });
  res.json({ code: 0, data: { ...publicStory(story), json: storyToJSON(story) } });
});

// ———————————————— S1/S3：动画页 ————————————————
router.get('/story/:id', (req, res) => {
  const story = store.getStory(req.params.id);
  if (!story) return res.status(404).send('作品不存在');
  const bundle = buildBundle(story, {
    cta: true,
    baseUrl: `${labConfig.basePath}`,
    still: req.query.still === '1',
    sceneIndex: Number(req.query.scene || 0),
  });
  res.setHeader('Content-Type', 'text/html; charset=utf-8');
  res.setHeader('Cache-Control', 'public, max-age=300');
  res.send(bundle.html);
});

// S1 可直接下载演示动画三件套
router.get('/story/:id/:file(html|css|json)', (req, res) => {
  const story = store.getStory(req.params.id);
  if (!story) return res.status(404).json({ code: 404, message: '作品不存在' });
  const bundle = buildBundle(story, { cta: true, baseUrl: labConfig.basePath });
  const base = `wayhug-${story.id}`;
  if (req.params.file === 'css') {
    res.setHeader('Content-Type', 'text/css; charset=utf-8');
    res.setHeader('Content-Disposition', `attachment; filename="${base}.css"`);
    return res.send(bundle.css);
  }
  if (req.params.file === 'json') {
    res.setHeader('Content-Type', 'application/json; charset=utf-8');
    res.setHeader('Content-Disposition', `attachment; filename="${base}.json"`);
    return res.send(JSON.stringify(storyToJSON(story), null, 2));
  }
  res.setHeader('Content-Type', 'text/html; charset=utf-8');
  res.setHeader('Content-Disposition', `attachment; filename="${base}.html"`);
  res.send(bundle.html);
});

// ———————————————— S3：在线生成 ————————————————
router.post('/api/generate', (req, res) => {
  if (limited(req.ip)) return res.status(429).json({ code: 429, message: '操作过于频繁，请稍后再试' });
  const { txt, title } = req.body || {};
  if (!txt || String(txt).trim().length < 10) {
    return res.status(400).json({ code: 400, message: '请粘贴至少 10 个字符的故事文本' });
  }
  if (String(txt).length > 20000) return res.status(400).json({ code: 400, message: '文本过长（上限 20000 字符）' });

  let story;
  try {
    story = parseStory(String(txt), { title: String(title || '') });
  } catch (err) {
    return res.status(400).json({ code: 400, message: err.message });
  }
  const saved = store.saveStory(story, { source: 'web' });
  const bundle = buildBundle(saved, { cta: true, baseUrl: `${labConfig.basePath}` });
  res.json({
    code: 0,
    data: {
      story: publicStory(saved),
      html: bundle.html,
      css: bundle.css,
      json: storyToJSON(saved),
      duration: bundle.duration,
    },
  });
});

// ———————————————— S2/S4：体验权申请 ————————————————
router.post('/api/experience', async (req, res) => {
  if (limited(req.ip)) return res.status(429).json({ code: 429, message: '操作过于频繁，请稍后再试' });
  const { name, email, storyId, txt, title, consent, note, source } = req.body || {};
  if (!name || !String(name).trim()) return res.status(400).json({ code: 400, message: '请填写称呼' });
  if (!/^[^\s@]+@[^\s@]+\.[^\s@]{2,}$/.test(String(email || ''))) {
    return res.status(400).json({ code: 400, message: '请输入有效邮箱' });
  }
  if (!consent) return res.status(400).json({ code: 400, message: '请先同意体验权条款' });

  let story = storyId ? store.getStory(String(storyId)) : null;
  if (!story && txt) {
    try {
      story = store.saveStory(parseStory(String(txt), { title: String(title || '') }), { source: 'web' });
    } catch (err) {
      return res.status(400).json({ code: 400, message: err.message });
    }
  }
  if (!story) return res.status(400).json({ code: 400, message: '作品不存在，请先在 S3 生成或选择演示作品' });

  const record = store.createRequest({
    source: String(source || 'web'),
    email: String(email).trim().toLowerCase(),
    name: String(name).trim().slice(0, 40),
    storyId: story.id,
    storyTitle: story.title,
    scenes: story.stats.scenes,
    chars: story.stats.chars,
    castNames: story.cast.map((c) => c.name),
    fingerprint: story.fingerprint,
    note: String(note || '').slice(0, 500),
    status: 'pending',
  });

  let mail = { ok: false, mode: 'manual', reason: '自动回复未开启，站长将人工答复' };
  if (labConfig.autoReply) {
    const pdf = await certificateBuffer(record, story);
    mail = await sendMail({
      to: record.email,
      subject: `Re: ${labConfig.mailPrefix} ${story.title} · IP 体验权 ${record.id}`,
      text: [
        `你好，${record.name}：`,
        '',
        `《${story.title}》的 IP 体验权证书已生成。`,
        `证书编号：${record.id}`,
        `在线动画：${labConfig.siteUrl}${labConfig.basePath}/story/${story.id}`,
        `证书下载：${labConfig.siteUrl}${labConfig.basePath}/cert/${record.id}.pdf`,
        `校验页面：${labConfig.siteUrl}${labConfig.basePath}/verify/${record.id}`,
        '',
        '免费体验期 90 天，非独占、不可转让。直接回复本邮件即可获得我的亲自答复。',
        '',
        '—— Wayhug Lab',
      ].join('\n'),
      attachments: [{ filename: `Wayhug-IP-${record.id}.pdf`, content: pdf, contentType: 'application/pdf' }],
    });
    if (mail.ok) store.updateRequest(record.id, { status: 'replied', reply: { type: 'auto', at: new Date().toISOString() } });
  }

  res.json({
    code: 0,
    data: {
      record: {
        id: record.id,
        name: record.name,
        email: record.email,
        storyTitle: record.storyTitle,
        createdAt: record.createdAt,
        expiresAt: record.expiresAt,
        status: store.getRequest(record.id)?.status || record.status,
      },
      story: publicStory(story),
      certUrl: `${labConfig.basePath}/cert/${record.id}.pdf`,
      verifyUrl: `${labConfig.basePath}/verify/${record.id}`,
      animationUrl: `${labConfig.basePath}/story/${story.id}`,
      mail,
    },
  });
});

router.get('/cert/:id.pdf', async (req, res, next) => {
  const record = store.getRequest(req.params.id);
  if (!record) return res.status(404).json({ code: 404, message: '证书不存在' });
  const story = store.getStory(record.storyId);
  try {
    await streamCertificate(record, story, res);
  } catch (err) {
    next(err);
  }
});

router.get('/api/verify/:id', (req, res) => {
  const record = store.getRequest(req.params.id);
  if (!record) return res.status(404).json({ code: 404, message: '未找到该证书编号' });
  res.json({
    code: 0,
    data: {
      valid: true,
      id: record.id,
      holder: record.name,
      storyTitle: record.storyTitle,
      scenes: record.scenes,
      fingerprint: record.fingerprint,
      createdAt: record.createdAt,
      expiresAt: record.expiresAt,
      status: record.status,
    },
  });
});

router.get('/verify/:id', (req, res) => {
  const record = store.getRequest(req.params.id);
  const title = record ? '证书有效' : '未找到证书';
  const rows = record
    ? [
        ['证书编号', record.id],
        ['持有人', record.name],
        ['作品', record.storyTitle],
        ['场景数', `${record.scenes} 幕`],
        ['内容指纹', record.fingerprint],
        ['签发日期', record.createdAt.slice(0, 10)],
        ['体验期至', record.expiresAt.slice(0, 10)],
        ['状态', record.status === 'replied' ? '已答复' : '待人工答复'],
      ]
    : [['编号', req.params.id]];
  res.setHeader('Content-Type', 'text/html; charset=utf-8');
  res.send(`<!DOCTYPE html><html lang="zh-CN"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>${title} · Wayhug IP Lab</title>
<style>
:root{--ink:#2c3038;--accent:${record ? '#2f9e77' : '#e0567a'};}
*{box-sizing:border-box}body{margin:0;min-height:100vh;display:flex;align-items:center;justify-content:center;background:#f5f5f7;font-family:"HarmonyOS Sans SC","Microsoft YaHei","PingFang SC",sans-serif;color:var(--ink);padding:24px}
.card{width:100%;max-width:560px;background:#fff;border-radius:18px;padding:36px 32px;border-top:10px solid var(--accent);box-shadow:14px 14px 0 rgba(0,0,0,.06)}
h1{font-size:26px;margin:0 0 6px}p.sub{margin:0 0 24px;color:#8a8a95;font-size:13px;letter-spacing:.08em}
table{width:100%;border-collapse:collapse;font-size:14px}td{padding:11px 0;border-bottom:1px solid #f0f0f4}td:first-child{color:#8a8a95;width:120px}
a.back{display:inline-block;margin-top:24px;color:var(--accent);text-decoration:none;font-size:13px}
</style></head><body><div class="card"><h1>${title}</h1><p class="sub">WAYHUG.COM · IP EXPERIENCE CERTIFICATE</p>
<table>${rows.map(([k, v]) => `<tr><td>${k}</td><td>${v}</td></tr>`).join('')}</table>
<a class="back" href="${labConfig.basePath}/">← 返回实验场</a></div></body></html>`);
});

// ———————————————— 站长后台 ————————————————
router.get('/api/admin/requests', requireAdmin, (req, res) => {
  res.json({ code: 0, data: store.listRequests(Number(req.query.limit) || 100), stats: store.stats() });
});

router.post('/api/admin/reply', requireAdmin, async (req, res, next) => {
  const { id, text } = req.body || {};
  if (!id || !text) return res.status(400).json({ code: 400, message: '缺少证书编号或答复内容' });
  try {
    const mail = await (await import('./mail.js')).replyToRequest(String(id), String(text));
    res.json({ code: 0, data: { mail, request: store.getRequest(String(id)) } });
  } catch (err) {
    next(err);
  }
});

router.post('/api/admin/poll', requireAdmin, async (req, res) => {
  res.json({ code: 0, data: await pollOnce() });
});

// 便于本地联调：直接以站长身份投递一封「邮件」
router.post('/api/admin/ingest', requireAdmin, async (req, res, next) => {
  const { from, subject, body } = req.body || {};
  if (!from || !body) return res.status(400).json({ code: 400, message: '缺少 from / body' });
  try {
    const { ingestEmail } = await import('./mail.js');
    const result = await ingestEmail({ from, subject: subject || '[WAYHUG] 测试故事', body, messageId: `test-${Date.now()}` });
    res.json({ code: 0, data: result });
  } catch (err) {
    next(err);
  }
});

// 演示作品文本（S1 下载/展示）
router.get('/api/demo.txt', (req, res) => {
  try {
    const txt = readFileSync(join(root, 'stories', 'demo.txt'), 'utf8');
    res.setHeader('Content-Type', 'text/plain; charset=utf-8');
    res.send(txt);
  } catch {
    res.status(404).send('演示文本缺失');
  }
});

// 情绪词表（S3 页脚说明用）
router.get('/api/moods', (req, res) => {
  res.json({ code: 0, data: Object.entries(MOODS).map(([id, m]) => ({ id, name: m.name, bg: m.bg, accent: m.accent })) });
});

export default router;
