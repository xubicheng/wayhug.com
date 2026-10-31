/* Wayhug IP Lab · 前端共享工具（无框架、无构建） */
(function () {
  const base = '/lab';
  let infoCache = null;

  async function info() {
    if (infoCache) return infoCache;
    const res = await fetch(`${base}/api/info`);
    if (!res.ok) throw new Error('无法读取实验场状态');
    const json = await res.json();
    infoCache = json.data;
    return infoCache;
  }

  async function post(path, body) {
    const res = await fetch(`${base}${path}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body || {}),
    });
    const json = await res.json().catch(() => ({}));
    if (!res.ok) throw new Error(json.message || `请求失败（${res.status}）`);
    return json.data;
  }

  function esc(s) {
    return String(s == null ? '' : s)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }

  function fmtDate(iso) {
    if (!iso) return '-';
    const d = new Date(iso);
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
  }

  function download(filename, text, mime) {
    const blob = new Blob([text], { type: mime || 'text/plain;charset=utf-8' });
    const a = document.createElement('a');
    a.href = URL.createObjectURL(blob);
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(a.href), 2000);
  }

  function mailto(email, subject, body) {
    const q = [];
    if (subject) q.push(`subject=${encodeURIComponent(subject)}`);
    if (body) q.push(`body=${encodeURIComponent(body)}`);
    return `mailto:${email}${q.length ? '?' + q.join('&') : ''}`;
  }

  /** 填充 data-lab-* 占位符 */
  async function hydrate() {
    let data;
    try {
      data = await info();
    } catch {
      return;
    }
    document.querySelectorAll('[data-lab="contact"]').forEach((el) => {
      el.textContent = data.contactEmail;
      if (el.tagName === 'A') el.href = `mailto:${data.contactEmail}`;
    });
    document.querySelectorAll('[data-lab="mailbox"]').forEach((el) => {
      el.textContent = data.contactEmail;
    });
    document.querySelectorAll('[data-lab="smtp"]').forEach((el) => {
      const on = data.capability.smtp === 'on';
      el.textContent = on ? 'SMTP 已启用' : 'SMTP 未启用 · 人工答复';
      el.className = `tag ${on ? 'on' : 'off'}`;
    });
    document.querySelectorAll('[data-lab="imap"]').forEach((el) => {
      const on = data.capability.imap === 'on';
      el.textContent = on ? 'IMAP 已启用' : 'IMAP 未启用';
      el.className = `tag ${on ? 'on' : 'off'}`;
    });
    document.querySelectorAll('[data-lab="admin"]').forEach((el) => {
      el.textContent = data.capability.admin ? '后台已启用' : '后台未配置令牌';
      el.className = `tag ${data.capability.admin ? 'on' : 'off'}`;
    });
    document.querySelectorAll('[data-lab="stats"]').forEach((el) => {
      el.textContent = `作品 ${data.stats.stories} · 证书 ${data.stats.requests}`;
    });
    document.querySelectorAll('[data-lab="base"]').forEach((el) => {
      el.textContent = data.basePath;
    });
  }

  document.addEventListener('DOMContentLoaded', hydrate);

  window.LAB = { base, info, post, esc, fmtDate, download, mailto, hydrate };
})();
