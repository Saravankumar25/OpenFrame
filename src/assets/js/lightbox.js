/* Screen viewer: clicking an app screen (a.shot-open) opens its full picture in a native dialog. Left and Right move
   between the screens on the page, a click or the zoom button toggles actual size (drag or wheel to pan), swipe
   changes screen on touch, Esc or a click outside closes and focus returns to where it was. Without script, or
   without <dialog>, the link opens the picture itself. */
(function () {
  if (typeof HTMLDialogElement === 'undefined') return;
  var doc = document.documentElement;
  var dlg, stage, img, capEl, bPrev, bNext, bZoom, bClose;
  var list = [], idx = 0, opener = null, actual = false, loaded = false, ratio = 1.6;
  var tx = 0, ty = 0, drag = null, moved = false;
  var calm = matchMedia('(prefers-reduced-motion: reduce)');

  var ICON = {
    x: '<path d="M6 6l12 12M18 6L6 18"/>',
    prev: '<path d="M15 5l-7 7 7 7"/>',
    next: '<path d="M9 5l7 7-7 7"/>',
    plus: '<circle cx="11" cy="11" r="6.5"/><path d="M20 20l-4.2-4.2M11 8.5v5M8.5 11h5"/>',
    minus: '<circle cx="11" cy="11" r="6.5"/><path d="M20 20l-4.2-4.2M8.5 11h5"/>'
  };
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
    img = document.createElement('img'); img.className = 'lb-img'; img.alt = ''; img.draggable = false;
    stage.appendChild(img);
    var spin = document.createElement('span'); spin.className = 'lb-spin'; spin.setAttribute('aria-hidden', 'true');
    capEl = document.createElement('p'); capEl.className = 'lb-cap'; capEl.setAttribute('aria-hidden', 'true');
    bClose = btn('lb-close', 'Close', ICON.x);
    bZoom = btn('lb-zoom', 'Show actual size', ICON.plus);
    bPrev = btn('lb-prev', 'Previous screen', ICON.prev);
    bNext = btn('lb-next', 'Next screen', ICON.next);
    [bClose, bZoom, stage, spin, capEl, bPrev, bNext].forEach(function (n) { dlg.appendChild(n); });
    document.body.appendChild(dlg);

    bClose.addEventListener('click', function () { dlg.close(); });
    bZoom.addEventListener('click', function () { toggle(); });
    bPrev.addEventListener('click', function () { go(-1); });
    bNext.addEventListener('click', function () { go(1); });
    dlg.addEventListener('close', onClose);
    dlg.addEventListener('keydown', function (e) {
      if (e.key === 'ArrowLeft') { go(-1); e.preventDefault(); }
      else if (e.key === 'ArrowRight') { go(1); e.preventDefault(); }
    });
    addEventListener('resize', function () { if (dlg.open) render(); });

    /* pointer: drag pans at actual size; a short move or a click toggles; a sideways swipe changes screen */
    stage.addEventListener('pointerdown', function (e) {
      if (e.button) return;
      drag = { x: e.clientX, y: e.clientY, tx: tx, ty: ty, t: e.pointerType }; moved = false;
      try { stage.setPointerCapture(e.pointerId); } catch (x) {}
    });
    stage.addEventListener('pointermove', function (e) {
      if (!drag) return;
      var dx = e.clientX - drag.x, dy = e.clientY - drag.y;
      if (Math.abs(dx) + Math.abs(dy) > 6) moved = true;
      if (actual && moved) { tx = drag.tx + dx; ty = drag.ty + dy; render(); }
    });
    stage.addEventListener('pointerup', function (e) {
      var d = drag; drag = null;
      if (!d) return;
      var dx = e.clientX - d.x, dy = e.clientY - d.y;
      if (!actual && d.t !== 'mouse' && Math.abs(dx) > 50 && Math.abs(dy) < Math.abs(dx) * .6) go(dx < 0 ? 1 : -1);
    });
    stage.addEventListener('pointercancel', function () { drag = null; });
    stage.addEventListener('click', function (e) {
      if (moved) { moved = false; return; }
      if (e.target === img) toggle(e.clientX, e.clientY); else dlg.close();
    });
    dlg.addEventListener('click', function (e) { if (e.target === dlg) dlg.close(); });
    stage.addEventListener('wheel', function (e) {
      if (!actual) return;
      e.preventDefault(); tx -= e.deltaX; ty -= e.deltaY; render();
    }, { passive: false });
  }

  function visible(a) { return a.getClientRects().length > 0; }
  function captionOf(a) {
    if (a.dataset.caption) return a.dataset.caption;
    var f = a.closest('figure'), c = f && f.querySelector('figcaption'), i = a.querySelector('img');
    return (c && c.textContent.trim()) || (i && i.alt) || '';
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
    var sbw = innerWidth - doc.clientWidth;
    doc.style.setProperty('--lb-sbw', sbw + 'px');
    doc.classList.add('lb-open');
    dlg.showModal();
    show();
  }

  function show() {
    var a = list[idx], pre = a.querySelector('img');
    actual = false; loaded = false; tx = ty = 0;
    dlg.classList.add('is-loading'); dlg.classList.remove('is-actual');
    capEl.textContent = captionOf(a); img.alt = capEl.textContent;
    bPrev.disabled = idx === 0; bNext.disabled = idx === list.length - 1;
    bPrev.hidden = bNext.hidden = list.length < 2;
    labelZoom();
    // the card the visitor clicked stands in until the full picture arrives
    if (pre && pre.naturalWidth) { ratio = pre.naturalWidth / pre.naturalHeight; img.src = pre.currentSrc || pre.src; } else img.removeAttribute('src');
    var want = a.href, full = new Image();
    full.onload = function () {
      if (list[idx] !== a) return;
      ratio = full.naturalWidth / full.naturalHeight; loaded = true;
      img.src = want; dlg.classList.remove('is-loading'); labelZoom(); render();
    };
    full.onerror = function () { if (list[idx] === a) dlg.classList.remove('is-loading'); };
    full.src = want;
    [idx - 1, idx + 1].forEach(function (j) { if (list[j]) new Image().src = list[j].href; });
    render();
  }

  function go(d) {
    var n = idx + d;
    if (n < 0 || n >= list.length) return;
    idx = n; show();
  }

  function labelZoom() {
    var t = actual ? 'Fit to window' : 'Show actual size';
    bZoom.setAttribute('aria-label', t); bZoom.title = t; bZoom.innerHTML = svg(actual ? ICON.minus : ICON.plus);
    bZoom.disabled = !loaded;
  }

  function toggle(cx, cy) {
    if (!loaded) return;
    var w = stage.clientWidth, h = stage.clientHeight, r = img.getBoundingClientRect();
    if (!actual) {
      // zoom in on the point that was clicked (the middle for the button)
      var fx = cx == null ? .5 : Math.min(1, Math.max(0, (cx - r.left) / r.width)), fy = cy == null ? .5 : Math.min(1, Math.max(0, (cy - r.top) / r.height));
      actual = true; tx = w / 2 - fx * img.naturalWidth; ty = h / 2 - fy * img.naturalHeight;
    } else actual = false;
    dlg.classList.toggle('is-actual', actual);
    if (!calm.matches) { img.classList.add('anim'); setTimeout(function () { img.classList.remove('anim'); }, 260); }
    labelZoom(); render();
  }

  // Size and place the picture: fitted and centred, or at its own size and panned (never past its edges).
  function render() {
    var w = stage.clientWidth, h = stage.clientHeight, iw, ih;
    if (actual) {
      iw = img.naturalWidth; ih = img.naturalHeight;
      tx = iw <= w ? (w - iw) / 2 : Math.min(0, Math.max(w - iw, tx));
      ty = ih <= h ? (h - ih) / 2 : Math.min(0, Math.max(h - ih, ty));
    } else {
      iw = Math.min(w, h * ratio); ih = iw / ratio; tx = (w - iw) / 2; ty = (h - ih) / 2;
    }
    img.style.width = iw + 'px'; img.style.height = ih + 'px';
    img.style.transform = 'translate(' + tx + 'px,' + ty + 'px)';
  }

  function onClose() {
    doc.classList.remove('lb-open'); doc.style.removeProperty('--lb-sbw');
    img.removeAttribute('src'); list = [];
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
