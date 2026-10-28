/* wayhug.com · 前台交互（原生 JS，无依赖） */
(function () {
  "use strict";
  const $ = (s, root) => (root || document).querySelector(s);
  const $$ = (s, root) => Array.from((root || document).querySelectorAll(s));

  /* 解锁键：从 #wkey=... 读入后只留在本标签页，不出现在地址栏 */
  const WRITE_STORE = "wayhug-write";
  (function readWriteKey() {
    const m = (location.hash || "").match(/wkey=([^&]+)/);
    if (!m) return;
    try {
      sessionStorage.setItem(WRITE_STORE, decodeURIComponent(m[1]));
    } catch (e) {
      /* 隐私模式下忽略 */
    }
    history.replaceState(null, "", location.pathname + location.search);
  })();
  function writeKey() {
    try {
      return sessionStorage.getItem(WRITE_STORE) || "";
    } catch (e) {
      return "";
    }
  }

  async function api(path, options) {
    const res = await fetch(path, options);
    const type = res.headers.get("content-type") || "";
    const data = type.includes("application/json") ? await res.json() : await res.text();
    if (!res.ok || (data && data.ok === false)) {
      const msg = (data && (data.error || data.message)) || (typeof data === "string" ? data : "请求失败");
      throw new Error(msg);
    }
    return data;
  }

  function status(el, text, kind) {
    if (!el) return;
    el.textContent = text || "";
    el.classList.remove("ok", "err");
    if (kind) el.classList.add(kind);
  }

  function fmt(n) {
    return (n || 0).toLocaleString("zh-CN");
  }

  /* ---------- 故事下拉 ---------- */
  async function loadStories() {
    const select = $("#exp-story");
    if (!select) return;
    try {
      const data = await api("/api/stories");
      const current = select.value || "demo";
      select.innerHTML = "";
      const demo = document.createElement("option");
      demo.value = "demo";
      demo.textContent = "《小狐与月亮邮局》· 演示作品";
      select.appendChild(demo);
      (data.stories || []).forEach((s) => {
        if (s.id === "demo") return;
        const opt = document.createElement("option");
        opt.value = s.id;
        opt.textContent = "《" + s.title + "》· " + s.scenes + " 幕";
        select.appendChild(opt);
      });
      select.value = current;
    } catch (e) {
      /* 静默：默认 demo 可用 */
    }
  }

  function addStoryOption(id, title, scenes) {
    const select = $("#exp-story");
    if (!select) return;
    if (!select.querySelector('option[value="' + id + '"]')) {
      const opt = document.createElement("option");
      opt.value = id;
      opt.textContent = "《" + title + "》· " + scenes + " 幕";
      select.appendChild(opt);
    }
    select.value = id;
  }

  /* ---------- 生成动画 ---------- */
  function bindCreate() {
    const form = $("#create-form");
    if (!form) return;
    const title = $("#story-title");
    const text = $("#story-text");
    const btn = $("#create-btn");
    const msg = $("#create-status");
    const result = $("#create-result");
    const preview = $("#create-preview");

    const example = $("#example-btn");
    if (example) {
      example.addEventListener("click", async () => {
        try {
          const res = await fetch("/api/demo.txt");
          const txt = await res.text();
          text.value = txt.trim();
          title.value = "小狐与月亮邮局";
          status(msg, "示例已填入，可直接生成。", "ok");
        } catch (e) {
          status(msg, "示例加载失败：" + e.message, "err");
        }
      });
    }

    form.addEventListener("submit", async (e) => {
      e.preventDefault();
      const body = text.value.trim();
      if (body.length < 10) {
        status(msg, "故事太短了，至少写一句完整的话。", "err");
        text.focus();
        return;
      }
      btn.disabled = true;
      status(msg, "正在解析 TXT，生成 JSON / CSS / HTML5…");
      try {
        const headers = { "content-type": "application/json" };
        const key = writeKey();
        if (key) headers["x-wayhug-write"] = key;
        const data = await api("/api/generate", {
          method: "POST",
          headers,
          body: JSON.stringify({ title: title.value.trim(), text: body }),
        });
        const story = data.story;
        $("#create-heading").textContent = "《" + story.title + "》";
        $("#create-meta").textContent =
          story.stats.scenes +
          " 幕 · " +
          story.stats.chars +
          " 字符 · " +
          story.cast.map((c) => c.name).join(" / ") +
          " · 指纹 " +
          story.fingerprint.slice(0, 8);
        preview.src = "/watch/" + story.id + "?t=" + Date.now();
        $("#dl-html").href = "/dl/" + story.id + "/html";
        $("#dl-css").href = "/dl/" + story.id + "/css";
        $("#dl-json").href = "/dl/" + story.id + "/json";
        const cert = $("#create-cert");
        if (cert) cert.href = "#experience";
        result.classList.add("show");
        addStoryOption(story.id, story.title, story.stats.scenes);
        status(msg, "生成完成。可直接预览，或到下一步领取纪念证书。", "ok");
        result.scrollIntoView({ behavior: "smooth", block: "nearest" });
      } catch (err) {
        status(msg, err.message, "err");
      } finally {
        btn.disabled = false;
      }
    });
  }

  /* ---------- 领取体验权 ---------- */
  function bindExperience() {
    const form = $("#exp-form");
    if (!form) return;
    const btn = $("#exp-btn");
    const msg = $("#exp-status");
    const result = $("#exp-result");

    form.addEventListener("submit", async (e) => {
      e.preventDefault();
      const name = $("#exp-name").value.trim();
      const email = $("#exp-email").value.trim();
      const consent = $("#exp-consent").checked;
      if (!name) {
        status(msg, "请填写称呼。", "err");
        $("#exp-name").focus();
        return;
      }
      if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) {
        status(msg, "请填写有效邮箱，仅用于作者人工联系。", "err");
        $("#exp-email").focus();
        return;
      }
      if (!consent) {
        status(msg, "请勾选授权说明后再领取。", "err");
        return;
      }
      btn.disabled = true;
      status(msg, "正在生成 A4 证书…");
      try {
        const data = await api("/api/experience", {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({
            name,
            email,
            storyId: $("#exp-story").value || "demo",
            message: $("#exp-message").value.trim(),
          }),
        });
        const req = data.request;
        $("#exp-id").textContent = req.id;
        $("#exp-name-out").textContent = req.name;
        $("#exp-title-out").textContent = req.storyTitle;
        $("#cert-link").href = data.certUrl;
        $("#verify-link").href = data.verifyUrl;
        $("#watch-link").href = "/watch/" + req.storyId;
        const mail = data.mail || {};
        if (mail.ok) {
          status(msg, "证书已生成，同时已发送到 " + req.email + "；也可在此直接下载打印。", "ok");
        } else {
          status(msg, "证书已生成，可直接下载打印；如需人工答复，请写信给作者。", "ok");
        }
        result.classList.add("show");
        result.scrollIntoView({ behavior: "smooth", block: "nearest" });
      } catch (err) {
        status(msg, err.message, "err");
      } finally {
        btn.disabled = false;
      }
    });
  }

  /* ---------- 数字统计填充 ---------- */
  function bindStats() {
    $$("[data-stat]").forEach((el) => {
      el.textContent = fmt(Number(el.getAttribute("data-stat")));
    });
  }

  document.addEventListener("DOMContentLoaded", () => {
    bindCreate();
    bindExperience();
    bindStats();
    loadStories();
  });
})();
