/* wayhug.com · 前台交互（原生 JS，无依赖） */
(function () {
  "use strict";
  const $ = (s, root) => (root || document).querySelector(s);
  const $$ = (s, root) => Array.from((root || document).querySelectorAll(s));

  /* ---------- 移动端：竖屏提示 + 软键盘兼容 ---------- */
  (function mobileAdapt() {
    // 竖屏时短暂提示横屏观看，3.5s 后消失，不遮挡画布
    const tip = document.getElementById("orient-tip");
    if (tip) {
      const checkOrient = () => {
        if (window.innerHeight > window.innerWidth) {
          tip.classList.add("show");
          clearTimeout(window.__orientTimer);
          window.__orientTimer = setTimeout(() => tip.classList.remove("show"), 3500);
        } else {
          tip.classList.remove("show");
        }
      };
      checkOrient();
      window.addEventListener("resize", checkOrient);
      window.addEventListener("orientationchange", checkOrient);
    }

    // 软键盘弹出时：把当前聚焦的输入框滚动到可见区域，
    // 解决微信/内置浏览器键盘收起后页面高度不回弹的问题
    const inputs = document.querySelectorAll("input, textarea, select");
    inputs.forEach((el) => {
      el.addEventListener("focus", () => {
        setTimeout(() => el.scrollIntoView({ block: "center", behavior: "smooth" }), 300);
      });
    });

    // 微信/内置浏览器：监听视口高度变化，键盘收起时强制重排
    let lastVh = window.innerHeight;
    window.addEventListener("resize", () => {
      const vh = window.innerHeight;
      if (vh > lastVh) {
        // 键盘收起，强制重排避免留白
        document.body.style.minHeight = vh + "px";
        setTimeout(() => (document.body.style.minHeight = ""), 100);
      }
      lastVh = vh;
    });
  })();

  /* 解锁键：从 #wkey=... 或 ?wkey=... 读入，存本标签页 + cookie，地址栏即时清除 */
  const WRITE_STORE = "wayhug-write";
  const WRITE_COOKIE = "wayhug_write";
  (function readWriteKey() {
    const m = (location.hash + location.search || "").match(/wkey=([^&]+)/);
    if (!m) return;
    let key = m[1];
    try {
      key = decodeURIComponent(key);
    } catch (e) {
      /* 保留原样 */
    }
    try {
      sessionStorage.setItem(WRITE_STORE, key);
    } catch (e) {
      /* 隐私模式下忽略 */
    }
    const host = location.hostname;
    const scope = host === "wayhug.com" || host.endsWith(".wayhug.com") ? "; domain=.wayhug.com" : "";
    document.cookie =
      WRITE_COOKIE + "=" + encodeURIComponent(key) + "; path=/; max-age=31536000; samesite=lax" + scope;
    history.replaceState(null, "", location.pathname);
  })();
  function writeKey() {
    try {
      const key = sessionStorage.getItem(WRITE_STORE);
      if (key) return key;
    } catch (e) {
      /* 继续读 cookie */
    }
    const m = ("; " + document.cookie).match(new RegExp("(?:^|; )" + WRITE_COOKIE + "=([^;]*)"));
    if (!m) return "";
    try {
      return decodeURIComponent(m[1]);
    } catch (e) {
      return m[1];
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

    if (writeKey()) status(msg, "已解锁，可直接生成。", "ok");

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
        let canvasLink = $("#dl-canvas");
        if (!canvasLink) {
          canvasLink = document.createElement("a");
          canvasLink.id = "dl-canvas";
          canvasLink.className = "chip";
          canvasLink.textContent = "256 色画布版";
          canvasLink.target = "_blank";
          canvasLink.rel = "noopener";
          $("#dl-json").after(canvasLink);
        }
        canvasLink.href = "/watch2d/" + story.id;
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
