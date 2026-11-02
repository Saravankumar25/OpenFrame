/* Home page behaviour: announcement bar, hero tour with progress ring, product accordion, scroll-driven story,
   promise carousel and the soft glow and tilt. Everything is progressive: without script the page is a plain stack.
   Automatic motion (tour, carousel) can be paused (WCAG 2.2.2), stops while hovered or focused, and never starts
   when the visitor asks for reduced motion. No inline styles: only custom properties are set. */
var HOME = document.body && document.body.getAttribute('data-nav') === 'home';
if (HOME) (function () {
  var reduce = window.matchMedia && matchMedia('(prefers-reduced-motion: reduce)').matches;
  var $ = function (s, r) { return (r || document).querySelector(s); };
  var $$ = function (s, r) { return [].slice.call((r || document).querySelectorAll(s)); };
  var get = function (k) { try { return localStorage.getItem(k); } catch (e) { return null; } };
  var set = function (k, v) { try { localStorage.setItem(k, v); } catch (e) {} };

  /* ---------- announcement bar ---------- */
  function annc() {
    var bar = $('#annc'); if (!bar) return;
    if (get('of_annc') === '1') bar.hidden = true;
    var x = $('[data-annc-close]', bar);
    if (x) x.addEventListener('click', function () { bar.hidden = true; set('of_annc', '1'); });
  }

  /* ---------- hero tour ---------- */
  function hero() {
    var root = $('[data-hx-tabs]'); if (!root) return;
    var panel = root.closest('[data-hx]');
    var tabs = $$('[role=tab]', root), panes = $$('[role=tabpanel]', root);
    var pp = $('[data-hx-pp]', root), cur = 0;
    function show(i, focus) {
      cur = (i + tabs.length) % tabs.length;
      tabs.forEach(function (t, k) {
        var on = k === cur; t.setAttribute('aria-selected', String(on)); t.tabIndex = on ? 0 : -1;
        if (panes[k]) panes[k].hidden = !on;
        var ring = t.querySelector('.ring'); if (ring) { ring.style.animation = 'none'; void ring.offsetWidth; ring.style.animation = ''; }
      });
      if (focus) tabs[cur].focus();
    }
    tabs.forEach(function (t, k) {
      t.addEventListener('click', function () { show(k); });
      t.addEventListener('keydown', function (e) {
        var n = e.key === 'ArrowRight' ? cur + 1 : e.key === 'ArrowLeft' ? cur - 1 : e.key === 'Home' ? 0 : e.key === 'End' ? tabs.length - 1 : null;
        if (n === null) return; e.preventDefault(); show(n, true);
      });
    });
    var prev = $('[data-hx-prev]', root), next = $('[data-hx-next]', root);
    if (prev) prev.addEventListener('click', function () { show(cur - 1); });
    if (next) next.addEventListener('click', function () { show(cur + 1); });
    // automatic tour: the ring animation ends, the next product shows
    if (!reduce) {
      panel.classList.add('is-auto');
      root.addEventListener('animationend', function (e) { if (e.animationName === 'ringGo') show(cur + 1); });
      var hold = function (on) { panel.classList.toggle('is-hold', on); };
      root.addEventListener('pointerenter', function () { hold(true); });
      root.addEventListener('pointerleave', function () { hold(false); });
      root.addEventListener('focusin', function () { hold(true); });
      root.addEventListener('focusout', function () { hold(false); });
      if (pp) pp.addEventListener('click', function () {
        var p = pp.getAttribute('aria-pressed') !== 'true'; pp.setAttribute('aria-pressed', String(p));
        pp.setAttribute('aria-label', p ? 'Play the automatic tour' : 'Pause the automatic tour');
        panel.classList.toggle('is-paused', p);
      });
      // only run while the hero is on screen
      if ('IntersectionObserver' in window) new IntersectionObserver(function (en) { panel.classList.toggle('is-hold', !en[0].isIntersecting); }, { threshold: .2 }).observe(panel);
      // soft pointer glow
      var raf = 0;
      panel.addEventListener('pointermove', function (e) {
        if (raf) return; raf = requestAnimationFrame(function () {
          raf = 0; var r = panel.getBoundingClientRect();
          panel.style.setProperty('--mx', (e.clientX - r.left) + 'px'); panel.style.setProperty('--my', (e.clientY - r.top) + 'px'); panel.style.setProperty('--glow', '1');
        });
      });
      panel.addEventListener('pointerleave', function () { panel.style.setProperty('--glow', '0'); });
    } else if (pp) pp.hidden = true;
  }

  /* ---------- product accordion (four cards) ---------- */
  function cards() {
    var row = $('[data-ccs]'); if (!row) return;
    var cs = $$('[data-cc]', row), t0;
    function on(c) {
      cs.forEach(function (o) { var is = o === c; o.classList.toggle('is-on', is); var b = $('.cc-t', o); if (b) b.setAttribute('aria-expanded', String(is)); });
    }
    cs.forEach(function (c) {
      c.addEventListener('click', function () { on(c); });
      c.addEventListener('pointerenter', function (e) { if (e.pointerType !== 'mouse') return; clearTimeout(t0); t0 = setTimeout(function () { on(c); }, 120); });
      c.addEventListener('pointerleave', function () { clearTimeout(t0); });
      c.addEventListener('focusin', function () { on(c); });
    });
  }

  /* ---------- scroll-driven story ---------- */
  function story() {
    var root = $('[data-ss]'); if (!root) return;
    var items = $$('[data-ss-item]', root), shots = $$('[data-ss-shot]', root), track = $('.ss-track', root);
    var n = items.length, cur = -1, wide = window.matchMedia('(min-width: 901px)');
    track.style.setProperty('--n', n);
    function setActive(i) {
      if (i === cur) return; cur = i;
      items.forEach(function (it, k) {
        it.classList.toggle('is-on', k === i); it.classList.toggle('is-past', k < i);
        var b = $('.ss-btn', it); if (b) b.setAttribute('aria-expanded', String(k === i));
      });
      shots.forEach(function (s, k) { s.classList.toggle('is-on', k === i); if (k === i) s.removeAttribute('aria-hidden'); else s.setAttribute('aria-hidden', 'true'); });
    }
    var ticking = false;
    function onScroll() {
      if (ticking || !wide.matches) return; ticking = true;
      requestAnimationFrame(function () {
        ticking = false;
        var r = track.getBoundingClientRect(), total = r.height - window.innerHeight;
        var p = total > 0 ? Math.min(.999, Math.max(0, -r.top / total)) : 0;
        setActive(Math.min(n - 1, Math.floor(p * n)));
      });
    }
    addEventListener('scroll', onScroll, { passive: true }); addEventListener('resize', function () { cur = -1; onScroll(); if (!wide.matches) setActive(0); });
    items.forEach(function (it, k) {
      $('.ss-btn', it).addEventListener('click', function () {
        if (wide.matches) {
          var r = track.getBoundingClientRect(), total = r.height - window.innerHeight, top = window.scrollY + r.top;
          window.scrollTo({ top: top + total * ((k + .5) / n), behavior: reduce ? 'auto' : 'smooth' });
        } else setActive(k);
      });
    });
    setActive(0); onScroll();
  }

  /* ---------- promise carousel ---------- */
  function carousel() {
    var root = $('[data-pcs]'); if (!root) return;
    var slides = $$('[data-pc]', root), dots = $$('[data-pc-dot]', root), pp = $('[data-pc-pp]', root), cur = 0, n = slides.length;
    function paint() {
      slides.forEach(function (s, i) {
        var d = (i - cur + n) % n;
        s.classList.toggle('is-act', d === 0); s.classList.toggle('is-next', d === 1); s.classList.toggle('is-prev', d === n - 1);
        if (d === 0) s.removeAttribute('aria-hidden'); else s.setAttribute('aria-hidden', 'true');
      });
      dots.forEach(function (b, i) { b.classList.toggle('is-on', i === cur); if (i === cur) b.setAttribute('aria-current', 'true'); else b.removeAttribute('aria-current'); });
      var on = $('.pc-dot.is-on i', root); if (on) { on.style.animation = 'none'; void on.offsetWidth; on.style.animation = ''; }
    }
    function go(i) { cur = (i + n) % n; paint(); }
    slides.forEach(function (s, i) { var b = $('[data-pc-go]', s); if (b) b.addEventListener('click', function () { go(i); }); });
    dots.forEach(function (b, i) { b.addEventListener('click', function () { go(i); }); });
    root.addEventListener('keydown', function (e) { if (e.key === 'ArrowRight') go(cur + 1); else if (e.key === 'ArrowLeft') go(cur - 1); });
    if (!reduce) {
      root.classList.add('is-auto');
      root.addEventListener('animationend', function (e) { if (e.animationName === 'dotGo') go(cur + 1); });
      var hold = function (on) { root.classList.toggle('is-hold', on); };
      root.addEventListener('pointerenter', function () { hold(true); }); root.addEventListener('pointerleave', function () { hold(false); });
      root.addEventListener('focusin', function () { hold(true); }); root.addEventListener('focusout', function () { hold(false); });
      if (pp) pp.addEventListener('click', function () {
        var p = pp.getAttribute('aria-pressed') !== 'true'; pp.setAttribute('aria-pressed', String(p));
        pp.setAttribute('aria-label', p ? 'Play the automatic slides' : 'Pause the automatic slides'); root.classList.toggle('is-paused', p);
      });
      if ('IntersectionObserver' in window) new IntersectionObserver(function (en) { root.classList.toggle('is-hold', !en[0].isIntersecting); }, { threshold: .3 }).observe(root);
    } else if (pp) pp.hidden = true;
    paint();
  }

  /* ---------- gentle tilt on gradient frames (mouse only) ---------- */
  function tilt() {
    if (reduce || !window.matchMedia('(hover: hover) and (pointer: fine)').matches) return;
    $$('.gframe').forEach(function (f) {
      f.addEventListener('pointermove', function (e) {
        var r = f.getBoundingClientRect(), x = (e.clientX - r.left) / r.width - .5, y = (e.clientY - r.top) / r.height - .5;
        f.style.setProperty('--tx', (x * 5).toFixed(2) + 'deg'); f.style.setProperty('--ty', (-y * 4).toFixed(2) + 'deg');
      });
      f.addEventListener('pointerleave', function () { f.style.setProperty('--tx', '0deg'); f.style.setProperty('--ty', '0deg'); });
    });
  }

  function init() { annc(); hero(); cards(); story(); carousel(); tilt(); }
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', init); else init();
})();
