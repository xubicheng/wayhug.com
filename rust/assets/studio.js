/* Wayhug Studio · 后台交互 */
(function () {
  "use strict";
  const $ = (s, r) => (r || document).querySelector(s);
  const tokenInput = $("#token");
  const statusEl = $("#admin-status");
  let token = localStorage.getItem("wayhug-token") || "";
  if (tokenInput) tokenInput.value = token;

  function setStatus(el, text, kind) {
    if (!el) return;
    el.textContent = text || "";
    el.classList.remove("ok", "err");
    if (kind) el.classList.add(kind);
  }

  async function admin(path, options) {
    const opt = options || {};
    const headers = Object.assign({}, opt.headers || {});
    if (token) headers["x-wayhug-token"] = token;
    const url = path + (path.includes("?") ? "&" : "?") + "token=" + encodeURIComponent(token);
    const res = await fetch(url, Object.assign({}, opt, { headers }));
    const ct = res.headers.get("content-type") || "";
    const data = ct.includes("json") ? await res.json() : await res.text();
    if (!res.ok || (data && data.ok === false)) {
      throw new Error((data && (data.error || data.message)) || "请求失败");
    }
    return data;
  }

  async function loadRequests() {
    if (!token) {
      setStatus(statusEl, "请输入管理令牌。", "err");
      return;
    }
    setStatus(statusEl, "加载中…");
    try {
      const data = await admin("/api/admin/requests");
      const rows = data.requests || [];
      const tbody = $("#requests tbody");
      if (!tbody) return;
      tbody.innerHTML = "";
      if (!rows.length) {
        tbody.innerHTML = '<tr><td colspan="5">暂无申请。</td></tr>';
      }
      rows.forEach((r) => {
        const tr = document.createElement("tr");
        const pending = r.status !== "replied";
        tr.innerHTML =
          "<td class='mono'>" + r.id + "</td>" +
          "<td>" + escapeHtml(r.name) + "<br><span class='mono'>" + escapeHtml(r.email) + "</span><br><span class='mono'>来源 " + escapeHtml(r.source) + "</span></td>" +
          "<td>《" + escapeHtml(r.storyTitle) + "》<br><span class='mono'>" + r.scenes + " 幕 · " + r.chars + " 字符</span></td>" +
          "<td><span class='badge-status " + (pending ? "pending" : "") + "'>" + (pending ? "待答复" : "已答复") + "</span><br><span class='mono'>" + escapeHtml((r.mail && r.mail.mode) || "") + "</span></td>" +
          "<td><div class='reply-box'><textarea data-reply='" + r.id + "' placeholder='写给访客的答复（会附证书 PDF）'>" + escapeHtml(r.replyText || "") + "</textarea><button class='btn small primary' data-send='" + r.id + "'>发送答复</button></div></td>";
        tbody.appendChild(tr);
      });
      setStatus(statusEl, "共 " + rows.length + " 条，待答复 " + rows.filter((r) => r.status !== "replied").length + " 条。", "ok");
    } catch (e) {
      setStatus(statusEl, e.message, "err");
    }
  }

  function escapeHtml(s) {
    return String(s == null ? "" : s)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#39;");
  }

  document.addEventListener("click", async (e) => {
    const send = e.target.closest("[data-send]");
    if (send) {
      const id = send.getAttribute("data-send");
      const box = document.querySelector("[data-reply='" + id + "']");
      const text = box ? box.value.trim() : "";
      if (!text) {
        alert("请先写答复内容。");
        return;
      }
      send.disabled = true;
      try {
        const data = await admin("/api/admin/reply", {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ id, text }),
        });
        alert(data.mail && data.mail.ok ? "已发送答复与证书。" : "已记录答复；SMTP 未发送，请检查邮箱配置。");
        loadRequests();
      } catch (err) {
        alert(err.message);
      } finally {
        send.disabled = false;
      }
    }
  });

  async function loadGallery() {
    if (!token) return;
    try {
      const data = await admin("/api/admin/sketches");
      const gallery = $("#gallery");
      if (!gallery) return;
      gallery.innerHTML = "";
      (data.sketches || []).forEach((s) => {
        const el = document.createElement("div");
        el.className = "sketch";
        const colors = (s.palette || [])
          .map((c) => "<i style='background:" + escapeHtml(c) + "'></i>")
          .join("");
        el.innerHTML =
          "<img loading='lazy' src='/sketch/" + encodeURIComponent(s.id) + "?token=" + encodeURIComponent(token) + "' alt='" + escapeHtml(s.title) + "'>" +
          "<div class='meta'><strong>" + escapeHtml(s.title) + "</strong>" +
          escPhase(s.phase) + " · " + escElement(s.element) + " · " + s.width + "×" + s.height +
          "<div class='palette'>" + colors + "</div></div>";
        gallery.appendChild(el);
      });
    } catch (e) {
      /* 未登录时静默 */
    }
  }

  function escPhase(k) {
    return { dawn: "晨", day: "昼", dusk: "暮", night: "夜" }[k] || k;
  }
  function escElement(k) {
    return { wind: "风", fire: "火", water: "水", earth: "土" }[k] || k;
  }

  document.addEventListener("DOMContentLoaded", () => {
    const save = $("#save-token");
    if (save) {
      save.addEventListener("click", () => {
        token = tokenInput.value.trim();
        localStorage.setItem("wayhug-token", token);
        loadRequests();
        loadGallery();
      });
    }
    if (tokenInput) {
      tokenInput.addEventListener("keydown", (e) => {
        if (e.key === "Enter") {
          token = tokenInput.value.trim();
          localStorage.setItem("wayhug-token", token);
          loadRequests();
          loadGallery();
        }
      });
    }

    const form = $("#sketch-form");
    if (form) {
      form.addEventListener("submit", async (e) => {
        e.preventDefault();
        const status = $("#sk-status");
        const file = $("#sk-file").files[0];
        if (!file) {
          setStatus(status, "请选择图片。", "err");
          return;
        }
        const fd = new FormData();
        fd.append("file", file);
        fd.append("title", $("#sk-title").value);
        fd.append("phase", $("#sk-phase").value);
        fd.append("element", $("#sk-element").value);
        fd.append("tags", $("#sk-tags").value);
        fd.append("note", $("#sk-note").value);
        setStatus(status, "上传并分析中…");
        try {
          const data = await admin("/api/admin/sketches", { method: "POST", body: fd });
          const s = data.sketch;
          setStatus(status, "已入库：" + s.id + " · 建议 " + escPhase(s.suggestedPhase) + " / " + escElement(s.suggestedElement) + " · 主色 " + s.palette.join(" "), "ok");
          form.reset();
          loadGallery();
        } catch (err) {
          setStatus(status, err.message, "err");
        }
      });
    }

    const poll = $("#poll-btn");
    if (poll) {
      poll.addEventListener("click", async () => {
        const status = $("#poll-status");
        poll.disabled = true;
        setStatus(status, "轮询中（最多 150 秒）…");
        try {
          const data = await admin("/api/admin/poll", { method: "POST" });
          const s = data.summary || {};
          setStatus(status, "拉取 " + (s.fetched || 0) + " · 建单 " + (s.ingested || 0) + " · 回复 " + (s.replied || 0) + " · 跳过 " + (s.skipped || 0), "ok");
          loadRequests();
        } catch (err) {
          setStatus(status, err.message, "err");
        } finally {
          poll.disabled = false;
        }
      });
    }

    if (token) {
      loadRequests();
      loadGallery();
    }
  });
})();
