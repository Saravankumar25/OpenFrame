// Tutorial demonstration player. Plays recorded frames of the real app with a drawn cursor and highlights.
// Markup and step data come from src/pages/tutorials.mjs; this file only adds behaviour.
(function () {
  'use strict';
  var reduce = window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  var TRAVEL = 700, DWELL = 1600, CHAR = 45, FADE = 300;

  function Player(root) {
    var data;
    try { data = JSON.parse(root.querySelector('.player-data').textContent || '[]'); } catch (e) { return; }
    if (!data.length) return;
    var q = function (s) { return root.querySelector(s); };
    var stage = q('.pl-stage');
    var frames = [].slice.call(root.querySelectorAll('.pl-frame'));
    var segs = [].slice.call(root.querySelectorAll('.pl-seg'));
    var ring = q('.pl-ring'), ripple = q('.pl-ripple'), typeBox = q('.pl-type'), typeText = q('.pl-type-text');
    var keyChip = q('.pl-key'), badge = q('.pl-badge'), cursor = q('.pl-cursor');
    var count = q('.pl-count'), label = q('.pl-label'), status = q('.pl-status');
    var playBtn = q('[data-act="play"]'), fsBtn = q('[data-act="fs"]');
    var list = document.querySelector('[data-steps-for="' + root.id + '"]');
    var items = list ? [].slice.call(list.querySelectorAll('li[data-step]')) : [];
    var cur = 0, playing = false, userPaused = false, autoPaused = false, started = false, timers = [], run = 0;
    var cursorAt = null, savedY = 0;

    root.classList.add('is-live');

    function later(fn, ms) { var id = run; timers.push(setTimeout(function () { if (id === run) fn(); }, ms)); }
    function cancel() { run++; timers.forEach(clearTimeout); timers = []; }
    function show(el, on) { el.classList.toggle('is-on', !!on); }

    // The area the active frame really occupies inside the stage (frames keep their aspect ratio).
    function box(i) {
      var f = frames[i], sw = stage.clientWidth, sh = stage.clientHeight;
      var fw = +f.getAttribute('width') || sw, fh = +f.getAttribute('height') || sh;
      var s = Math.min(sw / fw, sh / fh), w = fw * s, h = fh * s;
      return { x: (sw - w) / 2, y: (sh - h) / 2, w: w, h: h };
    }
    function rectPx(i) {
      var r = data[i].rect; if (!r) return null;
      var b = box(i);
      return { x: b.x + r[0] * b.w, y: b.y + r[1] * b.h, w: r[2] * b.w, h: r[3] * b.h };
    }
    function place(el, r, pad) {
      pad = pad || 0;
      el.style.left = (r.x - pad) + 'px'; el.style.top = (r.y - pad) + 'px';
      el.style.width = (r.w + pad * 2) + 'px'; el.style.height = (r.h + pad * 2) + 'px';
    }
    function moveCursor(x, y, animate) {
      cursor.style.transition = animate && !reduce ? 'transform ' + TRAVEL + 'ms cubic-bezier(.2,.8,.2,1)' : 'none';
      cursor.style.transform = 'translate(' + x + 'px,' + y + 'px)';
      cursorAt = { x: x, y: y };
    }

    function setFrame(i) {
      frames.forEach(function (f, k) {
        f.classList.toggle('is-next', k === i + 1);
        if (k !== i) f.classList.remove('is-active', 'is-scroll');
      });
      var f = frames[i];
      if (!f.classList.contains('is-active')) {
        if (getComputedStyle(f).display === 'none') { f.classList.add('is-next'); void f.offsetWidth; }
        f.classList.remove('is-next');
        f.classList.add('is-active');
      }
    }

    function sync(i, announce) {
      count.textContent = 'Step ' + (i + 1) + ' of ' + data.length;
      label.textContent = data[i].label;
      segs.forEach(function (s, k) {
        s.classList.toggle('is-done', k < i);
        s.classList.toggle('is-active', k === i);
        if (k === i) s.setAttribute('aria-current', 'step'); else s.removeAttribute('aria-current');
      });
      items.forEach(function (li, k) {
        if (k === i) li.setAttribute('aria-current', 'step'); else li.removeAttribute('aria-current');
      });
      if (announce) status.textContent = 'Step ' + (i + 1) + ' of ' + data.length + ': ' + data[i].label;
    }

    function clearMarks() {
      [ring, ripple, typeBox, keyChip, badge].forEach(function (el) { show(el, false); });
      ripple.classList.remove('is-double');
      typeText.textContent = '';
    }

    // Draws the marks for step i. Calls done() when the step has finished its own animation.
    function act(i, done) {
      var s = data[i], r = rectPx(i), a = s.action;
      if (a === 'scroll' && !reduce) frames[i].classList.add('is-scroll');
      if (a === 'view') { show(badge, true); return done(); }
      if (!r) {
        if (a === 'key' && s.key) { keyChip.textContent = s.key; keyChip.classList.add('is-free'); show(keyChip, true); }
        return done();
      }
      keyChip.classList.remove('is-free');
      var cx = r.x + r.w / 2, cy = r.y + r.h / 2;
      var from = cursorAt;
      var travel = !reduce && (!from || Math.abs(from.x - cx) > 1 || Math.abs(from.y - cy) > 1);
      if (!from) moveCursor(Math.max(0, cx - 80), cy + 60, false);
      show(cursor, true);
      void cursor.offsetWidth;
      moveCursor(cx, cy, true);
      later(function () {
        place(ring, r, 4); show(ring, true);
        if (a === 'type') {
          place(typeBox, r, 0); show(typeBox, true);
          var t = String(s.text || ''), n = 0;
          if (reduce) { typeText.textContent = t; return done(); }
          (function tick() {
            typeText.textContent = t.slice(0, n);
            if (n++ < t.length) later(tick, CHAR); else done();
          })();
          return;
        }
        if (a === 'key') {
          keyChip.textContent = s.key || '';
          keyChip.style.left = cx + 'px'; keyChip.style.top = (r.y + r.h + 10) + 'px';
          show(keyChip, !!s.key);
          return done();
        }
        // click, rightclick, doubleclick, drag and anything else: a ripple at the target centre.
        ripple.style.left = cx + 'px'; ripple.style.top = cy + 'px';
        ripple.classList.toggle('is-double', a === 'doubleclick');
        ripple.classList.remove('is-on'); void ripple.offsetWidth; show(ripple, !reduce);
        done();
      }, travel ? TRAVEL : 0);
    }

    function go(i, opts) {
      opts = opts || {};
      cancel();
      cur = Math.max(0, Math.min(data.length - 1, i));
      clearMarks();
      if (!data[cur].rect) show(cursor, false);
      setFrame(cur);
      sync(cur, opts.announce);
      later(function () {
        act(cur, function () {
          if (!playing) return;
          later(function () {
            if (cur < data.length - 1) go(cur + 1);
            else setPlaying(false);
          }, DWELL);
        });
      }, reduce ? 0 : FADE);
    }

    function setPlaying(on, announce) {
      playing = on;
      root.classList.toggle('is-playing', on);
      playBtn.setAttribute('aria-label', on ? 'Pause' : 'Play');
      if (announce) status.textContent = on ? 'Playing' : 'Paused';
    }
    function play(announce) {
      if (cur >= data.length - 1) { setPlaying(true, announce); return go(0); }
      setPlaying(true, announce); go(cur);
    }
    function pause(announce) { setPlaying(false, announce); cancel(); }
    function toggle() { started = true; if (playing) { userPaused = true; pause(true); } else { userPaused = false; play(true); } }
    function step(d) { userPaused = true; setPlaying(false); go(cur + d, { announce: true }); }
    function replay() { userPaused = false; cursorAt = null; setPlaying(true, true); go(0); }

    // Full screen: the Fullscreen API on the player, or a fixed full-window class where it is missing.
    function isFs() { return document.fullscreenElement === root || root.classList.contains('is-fs'); }
    function enterFs() {
      savedY = window.scrollY;
      if (root.requestFullscreen) {
        root.requestFullscreen().catch(function () { root.classList.add('is-fs'); afterFs(); });
      } else { root.classList.add('is-fs'); afterFs(); }
    }
    function exitFs() {
      if (document.fullscreenElement === root) document.exitFullscreen();
      else { root.classList.remove('is-fs'); afterFs(); }
    }
    function afterFs() {
      var on = isFs();
      document.documentElement.classList.toggle('pl-fs-lock', root.classList.contains('is-fs'));
      root.classList.toggle('is-full', on);
      fsBtn.setAttribute('aria-label', on ? 'Exit full screen' : 'Full screen');
      if (!on) { window.scrollTo(0, savedY); fsBtn.focus({ preventScroll: true }); }
      requestAnimationFrame(redraw);
    }
    document.addEventListener('fullscreenchange', function () { if (document.fullscreenElement === root || root.classList.contains('is-full')) afterFs(); });

    // Keep marks aligned with the stage when its size changes.
    function redraw() {
      var r = rectPx(cur); if (!r) return;
      place(ring, r, 4); place(typeBox, r, 0);
      moveCursor(r.x + r.w / 2, r.y + r.h / 2, false);
      ripple.style.left = (r.x + r.w / 2) + 'px'; ripple.style.top = (r.y + r.h / 2) + 'px';
      keyChip.style.left = (r.x + r.w / 2) + 'px'; keyChip.style.top = (r.y + r.h + 10) + 'px';
    }
    if (window.ResizeObserver) new ResizeObserver(redraw).observe(stage);

    root.addEventListener('click', function (e) {
      var b = e.target.closest('button'); if (!b || !root.contains(b)) return;
      var a = b.getAttribute('data-act');
      if (a === 'play') toggle();
      else if (a === 'prev') step(-1);
      else if (a === 'next') step(1);
      else if (a === 'replay') replay();
      else if (a === 'fs') (isFs() ? exitFs : enterFs)();
      else if (b.hasAttribute('data-seg')) { userPaused = true; setPlaying(false); go(+b.getAttribute('data-seg'), { announce: true }); }
    });
    root.addEventListener('keydown', function (e) {
      if (e.altKey || e.ctrlKey || e.metaKey) return;
      var onButton = e.target !== root && e.target.closest('button');
      var k = e.key;
      if ((k === ' ' && !onButton) || k === 'k' || k === 'K') { e.preventDefault(); toggle(); }
      else if (k === 'ArrowLeft') { e.preventDefault(); step(-1); }
      else if (k === 'ArrowRight') { e.preventDefault(); step(1); }
      else if (k === 'Home') { e.preventDefault(); replay(); }
      else if (k === 'f' || k === 'F') { e.preventDefault(); (isFs() ? exitFs : enterFs)(); }
    });
    document.addEventListener('keydown', function (e) {
      if (e.key === 'Escape' && root.classList.contains('is-fs')) { e.preventDefault(); exitFs(); }
    });
    items.forEach(function (li, k) {
      li.addEventListener('click', function (e) {
        if (window.getSelection && String(window.getSelection()) && !e.target.closest('button')) return;
        userPaused = true; setPlaying(false);
        go(k, { announce: true });
        root.focus({ preventScroll: true });
      });
    });

    if ('IntersectionObserver' in window) {
      new IntersectionObserver(function (es) {
        var seen = es[0].isIntersecting;
        if (seen && !playing && !userPaused && !reduce && (!started || autoPaused)) { started = true; autoPaused = false; play(); }
        else if (!seen && playing && !isFs()) { autoPaused = true; pause(); }
      }, { threshold: 0.4 }).observe(root);
    }
    go(0);
  }

  function init() { [].forEach.call(document.querySelectorAll('[data-player]'), Player); }
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', init); else init();
})();
