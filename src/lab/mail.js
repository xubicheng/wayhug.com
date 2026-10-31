/**
 * 服务层 · 邮件指令系统（对齐 jshnp.com 的邮件驱动模式）
 * 收：IMAP 轮询 unseen 邮件 → 主题/正文里的 [WAYHUG] 标记 → 解析 TXT → 生成动画 + 证书
 * 发：SMTP 回复（自动附 A4 证书 PDF / 站长人工答复）
 * 未配置 SMTP/IMAP 时全部降级：数据仍落库，站长可在 /lab/admin 人工处理。
 */

import nodemailer from 'nodemailer';
import { ImapFlow } from 'imapflow';
import { simpleParser } from 'mailparser';
import { labConfig, smtpReady, imapReady } from './config.js';
import { parseStory } from './story.js';
import { certificateBuffer } from './certificate.js';
import { buildAnimationHTML } from './animate.js';
import * as store from './store.js';

let transporter = null;
let timer = null;
let polling = false;
const log = (...args) => console.log('[lab:mail]', ...args);

function transport() {
  if (!smtpReady()) return null;
  if (!transporter) {
    transporter = nodemailer.createTransport({
      host: labConfig.smtp.host,
      port: labConfig.smtp.port,
      secure: labConfig.smtp.secure,
      auth: { user: labConfig.smtp.user, pass: labConfig.smtp.pass },
    });
  }
  return transporter;
}

/** 发信；未配置 SMTP 时返回 manual，由站长人工处理 */
export async function sendMail({ to, subject, text, html, attachments = [] }) {
  const t = transport();
  if (!t) return { ok: false, mode: 'manual', reason: 'SMTP 未配置，转为人工答复' };
  try {
    const info = await t.sendMail({
      from: `"Wayhug Lab" <${labConfig.smtp.user}>`,
      to,
      bcc: labConfig.ownerEmail || undefined,
      subject,
      text,
      html,
      attachments,
    });
    return { ok: true, mode: 'smtp', messageId: info.messageId };
  } catch (err) {
    log('发信失败:', err.message);
    return { ok: false, mode: 'manual', reason: err.message };
  }
}

function certMailText(record, story) {
  return [
    `你好，${record.name || '朋友'}：`,
    '',
    `你提交的故事《${record.storyTitle}》已完成 TXT → JSON → CSS → HTML5 的动画生成。`,
    `体验权证书编号：${record.id}`,
    `在线查看动画：${labConfig.siteUrl}${labConfig.basePath}/story/${record.storyId}`,
    `下载 A4 纪念证书：${labConfig.siteUrl}${labConfig.basePath}/cert/${record.id}.pdf`,
    `体验期至：${record.expiresAt.slice(0, 10)}`,
    '',
    '本证书为免费体验授权（非独占、不可转让），可在个人作品集中展示。',
    '如需人工答复，直接回复本邮件即可，我会亲自回信。',
    '',
    '—— Wayhug Lab',
  ].join('\n');
}

/** 受理一封邮件：解析故事 → 落库 → 自动回证书（或留待人工） */
export async function ingestEmail({ from, subject = '', body, messageId }) {
  const cleanSubject = String(subject).replace(/^(re|fw|fwd)\s*[:：]\s*/i, '').trim();
  const marker = labConfig.mailPrefix;
  const tagged = cleanSubject.includes(marker) || String(body).includes('#wayhug');
  if (!tagged) return { ok: false, skipped: '缺少 [WAYHUG] 标记，已忽略' };

  const title = cleanSubject.split(marker).pop().replace(/^[\s:：-]+/, '').trim() || '';
  let story;
  try {
    story = parseStory(body, { title });
  } catch (err) {
    return { ok: false, error: `故事解析失败：${err.message}` };
  }

  const existing = store.getStory(story.id);
  const saved = existing || store.saveStory(story, { source: 'mail', from });
  if (existing) {
    // 同一份文本重复提交：复用作品，只新建体验记录
    saved.id = existing.id;
  }

  const record = store.createRequest({
    source: 'mail',
    from,
    email: from,
    name: (String(from).split('@')[0] || '朋友').slice(0, 40),
    storyId: saved.id,
    storyTitle: saved.title,
    scenes: saved.stats.scenes,
    chars: saved.stats.chars,
    castNames: saved.cast.map((c) => c.name),
    fingerprint: saved.fingerprint,
    status: 'pending',
  });

  let mail = { ok: false, mode: 'manual', reason: '自动回复未开启' };
  if (labConfig.autoReply) {
    const pdf = await certificateBuffer(record, saved);
    mail = await sendMail({
      to: from,
      subject: `Re: ${marker} ${saved.title} · IP 体验权 ${record.id}`,
      text: certMailText(record, saved),
      html: `<p>${certMailText(record, saved).replace(/\n/g, '<br>')}</p>`,
      attachments: [{ filename: `Wayhug-IP-${record.id}.pdf`, content: pdf, contentType: 'application/pdf' }],
    });
    if (mail.ok) store.updateRequest(record.id, { status: 'replied', reply: { type: 'auto', at: new Date().toISOString() } });
  }

  return { ok: true, story: { id: saved.id, title: saved.title, scenes: saved.stats.scenes }, record, mail };
}

/** 站长人工答复（网页后台 / 邮件指令共用） */
export async function replyToRequest(id, text) {
  const record = store.getRequest(id);
  if (!record) throw new Error(`未找到体验记录 ${id}`);
  const story = store.getStory(record.storyId);
  const pdf = story ? await certificateBuffer(record, story) : null;
  const mail = await sendMail({
    to: record.email,
    subject: `Re: ${labConfig.mailPrefix} ${record.storyTitle} · 站长答复（${record.id}）`,
    text: `${text}\n\n—— Wayhug Lab\n证书：${labConfig.siteUrl}${labConfig.basePath}/cert/${record.id}.pdf`,
    attachments: pdf
      ? [{ filename: `Wayhug-IP-${record.id}.pdf`, content: pdf, contentType: 'application/pdf' }]
      : [],
  });
  if (mail.ok) {
    store.updateRequest(id, { status: 'replied', reply: { type: 'manual', text, at: new Date().toISOString() } });
  }
  return mail;
}

function ownerCommand(from, body) {
  if (!labConfig.ownerEmail || from.toLowerCase() !== labConfig.ownerEmail.toLowerCase()) return null;
  const firstLine = String(body).split('\n')[0].trim();
  const list = /^(列表|list)$/i.test(firstLine);
  const reply = firstLine.match(/^(回复|reply)\s+([A-Za-z0-9-]+)\s*[:：]?\s*([\s\S]+)$/i);
  if (list) return { type: 'list' };
  if (reply) return { type: 'reply', id: reply[2].toUpperCase(), text: reply[3].trim() };
  return null;
}

async function runOwnerCommand(cmd, from) {
  if (cmd.type === 'list') {
    const pending = store.listRequests().filter((r) => r.status === 'pending').slice(0, 20);
    const text = pending.length
      ? pending.map((r) => `${r.id}  ${r.storyTitle}  ${r.email}  ${r.createdAt.slice(0, 10)}`).join('\n')
      : '暂无待回复的体验申请。';
    await sendMail({ to: from, subject: `${labConfig.mailPrefix} 待办列表`, text });
    return;
  }
  if (cmd.type === 'reply') {
    const result = await replyToRequest(cmd.id, cmd.text);
    await sendMail({
      to: from,
      subject: `${labConfig.mailPrefix} 人工答复${result.ok ? '已发送' : '未发送'} ${cmd.id}`,
      text: result.ok ? `已回复 ${cmd.id}` : `发送失败：${result.reason}（请检查 SMTP 配置）`,
    });
  }
}

/** 轮询一次 INBOX，返回处理摘要 */
export async function pollOnce() {
  if (!imapReady()) return { skipped: 'IMAP 未启用' };
  if (polling) return { skipped: '上一次轮询尚未结束' };
  polling = true;
  const client = new ImapFlow({
    host: labConfig.imap.host,
    port: labConfig.imap.port,
    secure: labConfig.imap.secure,
    auth: { user: labConfig.imap.user, pass: labConfig.imap.pass },
    logger: false,
  });
  const summary = { scanned: 0, stories: 0, skipped: 0, errors: 0 };
  try {
    await client.connect();
    const lock = await client.getMailboxLock('INBOX');
    try {
      for await (const msg of client.fetch({ seen: false }, { source: true, envelope: true })) {
        summary.scanned += 1;
        try {
          const parsed = await simpleParser(msg.source);
          const from = (parsed.from?.value?.[0]?.address || '').toLowerCase();
          const subject = parsed.subject || '';
          const body = (parsed.text || '').trim();
          const messageId = parsed.messageId || `${from}-${msg.uid}-${msg.seq}`;

          if (!store.isNewMail(messageId)) {
            summary.skipped += 1;
            continue;
          }
          if (labConfig.imap.senders.length && !labConfig.imap.senders.includes(from)) {
            summary.skipped += 1;
            continue;
          }

          const cmd = ownerCommand(from, body);
          if (cmd) {
            await runOwnerCommand(cmd, from);
          } else {
            const result = await ingestEmail({ from, subject, body, messageId });
            if (result.ok) summary.stories += 1;
            else summary.skipped += 1;
          }
        } catch (err) {
          summary.errors += 1;
          log('单封邮件处理失败:', err.message);
        } finally {
          await client.messageFlagsAdd(msg.uid, ['\\Seen'], { uid: true }).catch(() => {});
        }
      }
    } finally {
      lock.release();
    }
    await client.logout();
  } catch (err) {
    summary.errors += 1;
    log('IMAP 轮询失败:', err.message);
  } finally {
    polling = false;
  }
  if (summary.scanned) log('轮询完成:', JSON.stringify(summary));
  return summary;
}

/** 启动常驻轮询（未启用时直接返回） */
export function startMailService() {
  if (!imapReady()) {
    log('IMAP 未启用（LAB_IMAP_ENABLED=false 或缺少账号），邮件指令为人工模式');
    return;
  }
  if (timer) return;
  timer = setInterval(() => {
    pollOnce().catch((err) => log('轮询异常:', err.message));
  }, Math.max(60000, labConfig.imap.intervalMs));
  timer.unref?.();
  log(`IMAP 轮询已启动：${labConfig.imap.host} 每 ${Math.round(labConfig.imap.intervalMs / 1000)}s`);
  setTimeout(() => pollOnce().catch(() => {}), 5000).unref?.();
}

/** 生成动画页地址（邮件正文用） */
export function animationUrl(storyId) {
  return `${labConfig.siteUrl}${labConfig.basePath}/story/${storyId}`;
}

export { buildAnimationHTML };
