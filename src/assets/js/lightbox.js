/* Screen viewer: clicking an app screen card (a.shot-open) opens the real interactive page in a native dialog, scaled
   to fit the window. Left and Right move between the screens on the page, Esc or a click outside closes, focus returns
   to the card. Without script, or without <dialog>, the link opens the page itself. */
(function () {
  if (typeof HTMLDialogElement === 'undefined') return;
  var doc = document.documentElement;
  var dlg, stage, view, frame, capEl, bPrev, bNext, bClose, newTab, spin;
  var list = [], idx = 0, opener = null, page = [1600, 1000], rect = [0, 0, 1600, 1000];

  function svg(d) { return '<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">' + d + '</svg>'; }
  function btn(cls, label, icon) {
    var b = document.createElement('button');
    b.type = 'button'; b.className = 'lb-btn ' + cls; b.setAttribute('aria-label', label); b.title = label; b.innerHTML = svg(icon);
    return b;
  }

  function build() {
    dlg = document.createElement('dialog');
    dlg.className = 'lb'; dlg.setAttribute('aria-label', 'Screen viewer');
    stage = document.createElement('div'); stage.className = 'lb-stage';
    view = document.createElement('div'); view.className = 'lb-view';
    // the loading mark is the one the page splash draws
    spin = document.createElement('span'); spin.className = 'mock-load lb-load'; spin.setAttribute('aria-hidden', 'true');
    var mk = document.querySelector('.loader .mk'); if (mk) spin.appendChild(mk.cloneNode(true));
    view.appendChild(spin); stage.appendChild(view);
    capEl = document.createElement('p'); capEl.className = 'lb-cap';
    newTab = document.createElement('a'); newTab.className = 'lb-new'; newTab.target = '_blank'; newTab.rel = 'noopener';
    newTab.innerHTML = svg('<path d="M14 4h6v6M20 4l-9 9M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5"/>') + '<span>Open in a new tab</span>';
    bClose = btn('lb-close', 'Close', '<path d="M6 6l12 12M18 6L6 18"/>');
    bPrev = btn('lb-prev', 'Previous screen', '<path d="M15 5l-7 7 7 7"/>');
    bNext = btn('lb-next', 'Next screen', '<path d="M9 5l7 7-7 7"/>');
    [bClose, newTab, stage, capEl, bPrev, bNext].forEach(function (n) { dlg.appendChild(n); });
    document.body.appendChild(dlg);

    bClose.addEventListener('click', function () { dlg.close(); });
    bPrev.addEventListener('click', function () { go(-1); });
    bNext.addEventListener('click', function () { go(1); });
    dlg.addEventListener('close', onClose);
    dlg.addEventListener('keydown', key);
    dlg.addEventListener('click', function (e) { if (e.target === dlg || e.target === stage) dlg.close(); });
    if ('ResizeObserver' in window) new ResizeObserver(fit).observe(stage); else addEventListener('resize', fit);
  }

  function editable(t) { return t && (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName)); }
  function key(e) {
    if (editable(e.target)) return;
    if (e.key === 'ArrowLeft') { go(-1); e.preventDefault(); } else if (e.key === 'ArrowRight') { go(1); e.preventDefault(); }
  }

  function nums(s, n, d) { var a = String(s || '').split(' ').map(Number); return a.length === n && a.every(isFinite) ? a : d; }
  function visible(a) { return a.getClientRects().length > 0; }
  function captionOf(a) {
    if (a.dataset.caption) return a.dataset.caption;
    var f = a.closest('figure'), c = f && f.querySelector('figcaption'), s = a.querySelector('.sr');
    return (c && c.textContent.trim()) || (s && s.textContent.trim()) || '';
  }

  function open(a) {
    if (!dlg) build();
    opener = a;
    var seen = {};
    list = [].filter.call(document.querySelectorAll('a.shot-open[href]'), function (x) {
      if (x !== a && (!visible(x) || seen[x.href])) return false;
      return (seen[x.href] = true);
    });
    idx = list.indexOf(a);
    doc.style.setProperty('--lb-sbw', (innerWidth - doc.clientWidth) + 'px');
    doc.classList.add('lb-open');
    dlg.showModal();
    show();
  }

  function show() {
    var a = list[idx];
    page = nums(a.dataset.page, 2, [1600, 1000]); rect = nums(a.dataset.full, 4, [0, 0, page[0], page[1]]);
    capEl.textContent = captionOf(a);
    newTab.href = a.href;
    bPrev.disabled = idx === 0; bNext.disabled = idx === list.length - 1;
    bPrev.hidden = bNext.hidden = list.length < 2;
    // a fresh frame each time keeps the browser's Back button out of it
    if (frame) frame.remove();
    frame = document.createElement('iframe'); frame.className = 'lb-frame'; frame.title = capEl.textContent || 'App screen';
    view.classList.add('is-loading');
    frame.addEventListener('load', function () {
      view.classList.remove('is-loading');
      // keys pressed inside the page still close or move the viewer
      try { frame.contentWindow.addEventListener('keydown', function (e) { if (e.key === 'Escape') dlg.close(); else key(e); }); } catch (x) {}
    });
    frame.src = a.href;
    view.appendChild(frame);
    fit();
  }

  function go(d) {
    var n = idx + d;
    if (n < 0 || n >= list.length) return;
    idx = n; show();
  }

  // The viewer shows the "full" rectangle of the page, as large as the window allows, with its shape kept.
  function fit() {
    if (!frame) return;
    var w = stage.clientWidth, h = stage.clientHeight;
    if (!w || !h) return;
    var k = Math.min(w / rect[2], h / rect[3], 2), vw = Math.round(rect[2] * k), vh = Math.round(rect[3] * k);
    view.style.width = vw + 'px'; view.style.height = vh + 'px';
    view.style.left = Math.round((w - vw) / 2) + 'px'; view.style.top = Math.round((h - vh) / 2) + 'px';
    frame.style.width = page[0] + 'px'; frame.style.height = page[1] + 'px';
    frame.style.transform = 'translate(' + (-rect[0] * k) + 'px,' + (-rect[1] * k) + 'px) scale(' + k + ')';
  }

  function onClose() {
    doc.classList.remove('lb-open'); doc.style.removeProperty('--lb-sbw');
    if (frame) { frame.remove(); frame = null; }
    list = [];
    var o = opener; opener = null;
    if (o && document.contains(o)) try { o.focus({ preventScroll: true }); } catch (e) {}
  }

  document.addEventListener('click', function (e) {
    if (e.defaultPrevented || e.button || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
    var a = e.target.closest && e.target.closest('a.shot-open[href]');
    if (!a) return;
    e.preventDefault(); open(a);
  });
})();
