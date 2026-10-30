/**
 * autosave.js — 表单自动保存到 localStorage，刷新后自动恢复
 *
 * 功能：
 *   1. 监听页面所有 input/textarea/select 的 change + input 事件
 *   2. 保存键 = `autosave:${pathname}:${name}`（pathname 隔离不同页面）
 *   3. DOMContentLoaded 时恢复值（如果当前值为空）
 *   4. 表单 submit 时清除该页面所有 autosave 数据
 *   5. 控制台 log 方便调试（`sessionStorage._autosave_log = '1'` 开启）
 *
 * 不保存的字段：passphrase/password/website/hp（口令、蜜罐）
 * 不恢复已有非空值（避免覆盖用户刚输入的内容）
 */
(function () {
  'use strict';

  var PREFIX = 'autosave:';
  var NO_SAVE = { passphrase: 1, password: 1, website: 1, hp: 1, captcha: 1, csrf: 1 };

  function log() {
    if (sessionStorage._autosave_log === '1') {
      console.log.apply(console, ['[autosave]'].concat(Array.prototype.slice.call(arguments)));
    }
  }

  function keyFor(el) {
    var path = location.pathname || '/';
    var name = el.name || el.id || '';
    if (!name) return null;
    return PREFIX + path + ':' + name;
  }

  function save(el) {
    var k = keyFor(el);
    if (!k) return;
    if (NO_SAVE[el.name]) { log('skip', el.name); return; }
    var v;
    if (el.tagName === 'TEXTAREA') {
      v = el.value;
    } else if (el.tagName === 'SELECT') {
      v = el.value;
    } else if (el.type === 'checkbox' || el.type === 'radio') {
      // 不保存 checkbox/radio（通常跟页面状态绑定，恢复反而搞怪）
      return;
    } else if (el.type === 'file') {
      // localStorage 存不下文件，跳过（但可提示用户）
      return;
    } else {
      v = el.value;
    }
    try {
      localStorage.setItem(k, v);
      log('saved', el.name, 'len=' + v.length);
    } catch (e) {
      // quota exceeded 等
      console.warn('[autosave] save failed:', e.message);
    }
  }

  function restore(el) {
    var k = keyFor(el);
    if (!k) return false;
    var saved = localStorage.getItem(k);
    if (saved === null || saved === undefined || saved === '') return false;
    // 如果字段已经有非空值（用户手动输入了），不覆盖
    if (el.value && el.value.trim() !== '' && el.value !== saved) {
      log('skip restore: field already has value', el.name);
      return false;
    }
    if (el.tagName === 'TEXTAREA') {
      el.value = saved;
    } else if (el.tagName === 'SELECT') {
      el.value = saved;
    } else if (el.type === 'password') {
      // 恢复密码太危险，跳过
      return false;
    } else {
      el.value = saved;
    }
    log('restored', el.name, 'from', k);
    return true;
  }

  function clearPage() {
    var path = location.pathname || '/';
    var prefix = PREFIX + path + ':';
    var removed = 0;
    for (var i = localStorage.length - 1; i >= 0; i--) {
      var k = localStorage.key(i);
      if (k && k.indexOf(prefix) === 0) {
        localStorage.removeItem(k);
        removed++;
      }
    }
    if (removed > 0) log('cleared', removed, 'keys for', path);
  }

  function collect() {
    var els = document.querySelectorAll('input, textarea, select');
    Array.prototype.forEach.call(els, function (el) {
      // 跳过 submit/button/hidden 里的蜜罐
      if (el.type === 'submit' || el.type === 'button' || el.type === 'reset') return;
      if (el.classList && el.classList.contains('hp')) return;

      // 恢复（只在页面首次加载时尝试）
      restore(el);

      // 保存事件
      el.addEventListener('input', function () { save(el); });
      el.addEventListener('change', function () { save(el); });
      el.addEventListener('blur', function () { save(el); });
    });

    // submit 时清除（防止残留已提交的数据下次又恢复出来）
    document.querySelectorAll('form').forEach(function (form) {
      form.addEventListener('submit', function () {
        log('form submitted, clearing autosave for page');
        clearPage();
      });
    });

    // beforeunload 兜底保存
    window.addEventListener('beforeunload', function () {
      document.querySelectorAll('input, textarea, select').forEach(function (el) {
        save(el);
      });
    });
  }

  // 启动
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', collect);
  } else {
    collect();
  }

  // 暴露到 window 方便调试
  window._autosave = {
    clear: clearPage,
    log: function () { sessionStorage._autosave_log = '1'; },
    clearAll: function () {
      for (var i = localStorage.length - 1; i >= 0; i--) {
        var k = localStorage.key(i);
        if (k && k.indexOf(PREFIX) === 0) localStorage.removeItem(k);
      }
    }
  };

  log('enabled on', location.pathname);
})();
