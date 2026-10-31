/**
 * 内核层 · 实验场配置（LAB）
 * 说明：只读取环境变量，不写死任何账号；未配置的功能自动降级为「人工模式」。
 */

const env = process.env;

function bool(v, def = false) {
  if (v === undefined || v === '') return def;
  return String(v).toLowerCase() === 'true';
}

export const labConfig = {
  siteUrl: (env.LAB_SITE_URL || env.SITE_URL || 'https://wayhug.com').replace(/\/$/, ''),
  basePath: env.LAB_BASE_PATH || '/lab',
  contactEmail: env.LAB_CONTACT_EMAIL || env.CONTACT_EMAIL || 'bc.xu@hotmail.com',
  ownerEmail: env.LAB_OWNER_EMAIL || env.NOTIFY_EMAIL || '',
  adminToken: env.LAB_ADMIN_TOKEN || '',
  autoReply: bool(env.LAB_AUTO_REPLY, false),
  mailPrefix: env.LAB_MAIL_PREFIX || '[WAYHUG]',
  smtp: {
    host: env.SMTP_HOST || '',
    port: Number(env.SMTP_PORT || 465),
    secure: bool(env.SMTP_SECURE, true),
    user: env.SMTP_USER || '',
    pass: env.SMTP_PASS || '',
  },
  imap: {
    enabled: bool(env.LAB_IMAP_ENABLED, false),
    host: env.MAIL_IMAP_HOST || '',
    port: Number(env.MAIL_IMAP_PORT || 993),
    secure: bool(env.MAIL_IMAP_SECURE, true),
    user: env.MAIL_IMAP_USER || '',
    pass: env.MAIL_IMAP_PASS || '',
    intervalMs: Number(env.MAIL_IMAP_INTERVAL_MS || 120000),
    senders: (env.MAIL_IMAP_SENDERS || '')
      .split(',')
      .map((s) => s.trim().toLowerCase())
      .filter(Boolean),
  },
};

export function smtpReady() {
  const { host, user, pass } = labConfig.smtp;
  return Boolean(host && user && pass);
}

export function imapReady() {
  const { enabled, host, user, pass } = labConfig.imap;
  return Boolean(enabled && host && user && pass);
}

/** 面向页面的能力清单（不泄露密码） */
export function capability() {
  return {
    smtp: smtpReady() ? 'on' : 'off',
    imap: imapReady() ? 'on' : 'off',
    autoReply: labConfig.autoReply,
    admin: Boolean(labConfig.adminToken),
    contactEmail: labConfig.contactEmail,
  };
}
