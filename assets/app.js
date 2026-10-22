(() => {
  'use strict';
  const t0 = performance.now();

  // Rust 通过 initialization script 注入：{lang, debug, path, name, dir, text, error, lossy}
  const S = window.__MDTOOL__ || {};
  const $ = (id) => document.getElementById(id);

  // ---------- 界面文字（中文系统显示中文，否则英文） ----------
  const T = S.lang === 'zh' ? {
    edit: '编辑', read: '阅读', save: '保存',
    modeTip: '切换阅读 / 编辑 (Ctrl+E)', saveTip: '保存 (Ctrl+S)',
    saveFailed: '保存失败：',
    changedOutside: '文件已在外部被修改。', reload: '重新加载', keepMine: '保留我的修改',
    cannotOpen: '无法打开文件',
    lossy: '无法识别文件编码，内容可能显示为乱码；保存时将按 UTF-8 写入。',
    usage: '<h1>mdtool</h1><p>小巧、快速的 Markdown 查看 / 编辑器。</p>' +
      '<h3>用法</h3><ul>' +
      '<li><code>mdtool.exe 文件.md</code>：打开文件</li>' +
      '<li><code>mdtool.exe --install</code>：安装并关联 .md 文件</li>' +
      '<li><code>mdtool.exe --uninstall</code>：取消关联</li></ul>' +
      '<h3>快捷键</h3><ul><li><code>Ctrl+E</code>：切换阅读 / 编辑</li><li><code>Ctrl+S</code>：保存</li></ul>',
  } : {
    edit: 'Edit', read: 'Read', save: 'Save',
    modeTip: 'Toggle reading / editing (Ctrl+E)', saveTip: 'Save (Ctrl+S)',
    saveFailed: 'Could not save: ',
    changedOutside: 'This file was changed by another program.', reload: 'Reload', keepMine: 'Keep my changes',
    cannotOpen: 'Cannot open file',
    lossy: 'Unknown text encoding. The content may look garbled, and saving will write it as UTF-8.',
    usage: '<h1>mdtool</h1><p>A tiny, fast Markdown viewer &amp; editor.</p>' +
      '<h3>Usage</h3><ul>' +
      '<li><code>mdtool.exe file.md</code>: open a file</li>' +
      '<li><code>mdtool.exe --install</code>: install and associate .md files</li>' +
      '<li><code>mdtool.exe --uninstall</code>: remove the association</li></ul>' +
      '<h3>Shortcuts</h3><ul><li><code>Ctrl+E</code>: toggle reading / editing</li><li><code>Ctrl+S</code>: save</li></ul>',
  };
  document.documentElement.lang = S.lang === 'zh' ? 'zh-CN' : 'en';
  $('btn-mode').textContent = T.edit;
  $('btn-mode').title = T.modeTip;
  $('btn-save').textContent = T.save;
  $('btn-save').title = T.saveTip;
  const preview = $('preview');
  const content = $('content');
  const editorEl = $('editor');
  const send = (msg) => window.ipc && window.ipc.postMessage(JSON.stringify(msg));
  if (S.debug) {
    const log = (msg) => send({ cmd: 'log', msg: String(msg) });
    window.addEventListener('error', (e) => log(`error: ${e.message} @${e.filename}:${e.lineno}`));
    window.addEventListener('unhandledrejection', (e) => log(`rejection: ${e.reason}`));
    window.addEventListener('keydown', (e) => log(`key: ${e.ctrlKey ? 'Ctrl+' : ''}${e.key}`), true);
  }

  let text = typeof S.text === 'string' ? S.text : null;
  let savedText = text;
  let dirty = false;
  let editing = false;
  let editor = null;          // CodeMirror 实例，首次进入编辑模式时创建
  let applyingExternal = false;
  let pendingSave = null;
  let renderTimer = 0;

  // ---------- 资源懒加载 ----------
  const loaded = new Map();
  function loadScript(src) {
    if (!loaded.has(src)) {
      loaded.set(src, new Promise((resolve, reject) => {
        const s = document.createElement('script');
        s.src = src;
        s.onload = resolve;
        s.onerror = reject;
        document.head.appendChild(s);
      }));
    }
    return loaded.get(src);
  }
  function loadCss(href) {
    if (!loaded.has(href)) {
      const l = document.createElement('link');
      l.rel = 'stylesheet';
      l.href = href;
      // 插在 style.css 前面，保证我们自己的样式优先
      document.head.insertBefore(l, document.querySelector('link[href="style.css"]'));
      loaded.set(href, Promise.resolve());
    }
    return loaded.get(href);
  }

  // ---------- Markdown 渲染器 ----------
  const md = window.markdownit({ html: true, linkify: true })
    .use(window.markdownitTaskLists)
    .use(window.markdownitFootnote);

  function slugify(s) {
    return s.trim().toLowerCase()
      .replace(/[^\p{L}\p{N}\s_-]/gu, '')
      .replace(/\s/g, '-');
  }

  // 给块级元素加 data-line（编辑时同步滚动用），给标题加 id（锚点跳转用）
  md.core.ruler.push('mdtool_attrs', (state) => {
    const used = Object.create(null);
    const tokens = state.tokens;
    for (let i = 0; i < tokens.length; i++) {
      const t = tokens[i];
      if (t.map && t.nesting !== -1) t.attrSet('data-line', String(t.map[0]));
      if (t.type === 'heading_open') {
        const inline = tokens[i + 1];
        let slug = slugify(inline.children
          .filter((c) => c.type === 'text' || c.type === 'code_inline')
          .map((c) => c.content).join(''));
        if (slug in used) slug += '-' + (++used[slug]);
        else used[slug] = 0;
        t.attrSet('id', slug);
      }
    }
  });

  let hljsReady = false;
  let mathReady = false;
  md.set({
    highlight: (code, lang) => {
      if (!hljsReady || !lang || !window.hljs.getLanguage(lang)) return '';
      try {
        return window.hljs.highlight(code, { language: lang, ignoreIllegals: true }).value;
      } catch (_) {
        return '';
      }
    },
  });

  const needsMath = (s) => s.includes('$');
  const needsHljs = (s) => /^\s*(```|~~~)\s*\S/m.test(s);

  // 公式、代码高亮库按需加载，加载完成后重新渲染一次
  function ensureExtras(s) {
    const jobs = [];
    if (!mathReady && needsMath(s)) {
      loadCss('vendor/katex/katex.min.css');
      jobs.push(loadScript('vendor/katex/katex.min.js')
        .then(() => loadScript('vendor/texmath.js'))
        .then(() => {
          md.use(window.texmath, { engine: window.katex, delimiters: 'dollars', katexOptions: { throwOnError: false } });
          mathReady = true;
        }));
    }
    if (!hljsReady && needsHljs(s)) {
      jobs.push(loadScript('vendor/hljs/highlight.min.js').then(() => { hljsReady = true; }));
    }
    if (jobs.length) Promise.all(jobs).then(() => render(), (e) => console.error(e));
  }

  // ---------- 本地路径 ----------
  const encodedDir = (S.dir || '').split('/').map(encodeURIComponent).join('/');
  const docBase = location.origin + '/abs/' + encodedDir + '/';
  const isExternal = (u) => /^[a-z][a-z0-9+.-]*:/i.test(u) && !/^[a-z]:[\\/]/i.test(u) && !/^file:/i.test(u);

  // 把文档里的相对/绝对本地路径转换成 /abs/... 地址
  function localUrl(u) {
    if (/^file:\/\/\/?/i.test(u)) u = u.replace(/^file:\/\/\/?/i, '');
    if (/^[a-z]:[\\/]/i.test(u)) return new URL('/abs/' + u.replace(/\\/g, '/'), location.origin);
    return new URL(u, docBase);
  }
  function localPath(u) {
    const url = localUrl(u);
    return decodeURIComponent(url.pathname.replace(/^\/abs\//, ''));
  }

  function fixImages(root) {
    for (const img of root.querySelectorAll('img[src]')) {
      const src = img.getAttribute('src');
      if (!src || src.startsWith('data:') || isExternal(src)) continue;
      img.src = localUrl(src).href;
    }
  }

  // ---------- 渲染 ----------
  let lineEls = [];
  let firstRender = true;

  function render() {
    if (text === null) return;
    content.innerHTML = md.render(text);
    fixImages(content);
    lineEls = Array.from(content.querySelectorAll('[data-line]'));
    ensureExtras(text);
    if (firstRender) {
      firstRender = false;
      send({ cmd: 'rendered', perf: [Math.round(t0), Math.round(performance.now())] });
    }
  }

  function scheduleRender() {
    clearTimeout(renderTimer);
    renderTimer = setTimeout(render, 150);
  }

  function escapeHtml(s) {
    return s.replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));
  }

  function showStatic(html) {
    content.innerHTML = html;
    $('toolbar').hidden = true;
    send({ cmd: 'rendered' });
  }

  // ---------- 顶部提示条 ----------
  function showBanner(message, actions) {
    $('banner-text').textContent = message;
    const box = $('banner-actions');
    box.innerHTML = '';
    for (const [label, fn] of actions || []) {
      const b = document.createElement('button');
      b.textContent = label;
      b.onclick = () => { hideBanner(); fn(); };
      box.appendChild(b);
    }
    $('banner').hidden = false;
  }
  function hideBanner() { $('banner').hidden = true; }

  // ---------- 状态 ----------
  function setDirty(v) {
    if (v === dirty) return;
    dirty = v;
    send({ cmd: 'dirty', value: v });
  }
  const refreshDirty = () => setDirty(text !== savedText);

  function applyText(newText) {
    text = newText;
    if (editor) {
      const cursor = editor.getCursor();
      const scroll = editor.getScrollInfo();
      applyingExternal = true;
      editor.setValue(newText);
      editor.setCursor(cursor);
      editor.scrollTo(scroll.left, scroll.top);
      applyingExternal = false;
    }
    refreshDirty();
    render();
  }

  // ---------- 编辑模式 ----------
  async function loadEditor() {
    loadCss('vendor/cm/codemirror.css');
    await loadScript('vendor/cm/codemirror.js');
    await loadScript('vendor/cm/overlay.js');
    await loadScript('vendor/cm/markdown.js');
    await loadScript('vendor/cm/gfm.js');
    editor = window.CodeMirror(editorEl, {
      value: text,
      mode: { name: 'gfm', highlightFormatting: true },
      lineWrapping: true,
      indentUnit: 4,
      extraKeys: { Tab: (cm) => cm.replaceSelection('    ') },
    });
    editor.on('change', () => {
      if (applyingExternal) return;
      text = editor.getValue();
      refreshDirty();
      scheduleRender();
    });
    editor.on('scroll', syncPreview);
  }

  // 按编辑器顶部行号，把预览滚动到对应位置（相邻两个块之间按比例插值）
  function syncPreview() {
    if (!lineEls.length) return;
    const line = editor.lineAtHeight(editor.getScrollInfo().top, 'local');
    let i = 0;
    while (i + 1 < lineEls.length && +lineEls[i + 1].dataset.line <= line) i++;
    const el = lineEls[i];
    const next = lineEls[i + 1];
    const base = preview.getBoundingClientRect().top - preview.scrollTop;
    const top = el.getBoundingClientRect().top - base;
    let y = top;
    if (next) {
      const l0 = +el.dataset.line;
      const l1 = +next.dataset.line;
      if (l1 > l0) {
        const nextTop = next.getBoundingClientRect().top - base;
        y = top + (nextTop - top) * Math.min(1, (line - l0) / (l1 - l0));
      }
    }
    preview.scrollTop = Math.max(0, y - 24);
  }

  async function toggleEdit() {
    if (text === null) return;
    editing = !editing;
    document.body.classList.toggle('editing', editing);
    $('btn-mode').textContent = editing ? T.read : T.edit;
    if (editing) {
      if (!editor) await loadEditor();
      editor.refresh();
      editor.focus();
    } else {
      clearTimeout(renderTimer);
      render();
      preview.focus({ preventScroll: true });
    }
  }

  function save(close) {
    if (text === null) return;
    pendingSave = text;
    send({ cmd: 'save', text, close: !!close });
  }

  // ---------- 事件 ----------
  window.addEventListener('keydown', (e) => {
    const key = e.key.toLowerCase();
    if (e.ctrlKey && !e.altKey && key === 'e') { e.preventDefault(); toggleEdit(); }
    else if (e.ctrlKey && !e.altKey && key === 's') { e.preventDefault(); save(false); }
    else if (key === 'f5' || (e.ctrlKey && key === 'r')) e.preventDefault(); // 防止刷新页面丢失编辑
  }, true);

  $('btn-mode').onclick = toggleEdit;
  $('btn-save').onclick = () => save(false);

  content.addEventListener('click', (e) => {
    const a = e.target.closest('a[href]');
    if (!a) return;
    e.preventDefault();
    const href = a.getAttribute('href');
    if (href.startsWith('#')) {
      const target = document.getElementById(decodeURIComponent(href.slice(1)));
      if (target) target.scrollIntoView();
    } else if (isExternal(href)) {
      send({ cmd: 'open', url: href });
    } else {
      send({ cmd: 'open', path: localPath(href.split('#')[0]) });
    }
  });

  // ---------- 供 Rust 调用 ----------
  window.mdtool = {
    onSaved(ok, err) {
      if (ok) {
        savedText = pendingSave;
        refreshDirty();
        if (S.lossy) { S.lossy = false; hideBanner(); }
      } else {
        alert(T.saveFailed + err);
      }
      pendingSave = null;
    },
    onExternalChange(newText) {
      if (newText === savedText) return;
      savedText = newText;
      if (!dirty || newText === text) {
        applyText(newText);
      } else {
        showBanner(T.changedOutside, [
          [T.reload, () => applyText(newText)],
          [T.keepMine, () => {}],
        ]);
      }
    },
    requestSaveAndClose() { save(true); },
  };

  // ---------- 启动 ----------
  if (S.error) {
    showStatic('<h2>' + T.cannotOpen + '</h2><p><code>' + escapeHtml(S.path || '') + '</code></p><p class="error">' + escapeHtml(S.error) + '</p>');
  } else if (text === null) {
    showStatic(T.usage);
  } else {
    render();
    // 让 PgDn / 方向键直接滚动正文
    preview.focus({ preventScroll: true });
    if (S.lossy) showBanner(T.lossy);
  }
})();
