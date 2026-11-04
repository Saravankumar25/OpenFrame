/* Straft website behaviour: the three independent controls (platform, website theme, app screen theme), app screen
   pictures, menus, home screen switcher, regional price. No dependencies, no tracking, nothing sent anywhere. */
(function () {
  var doc = document.documentElement;
  function store(k, v) { try { if (v === undefined) return localStorage.getItem(k); localStorage.setItem(k, v); } catch (e) { return null; } }
  var switchTo = {};
  try { switchTo = JSON.parse((document.getElementById('os-switch') || {}).textContent || '{}'); } catch (e) {}

  /* ---------- controls ---------- */
  function mark() {
    document.querySelectorAll('[data-set]').forEach(function (b) {
      var key = b.getAttribute('data-set'), attr = key === 'os' ? 'data-os' : key === 'theme' ? 'data-theme' : 'data-mock';
      b.setAttribute('aria-pressed', String(doc.getAttribute(attr) === b.getAttribute('data-val')));
    });
  }
  function osLinks(os) {
    document.querySelectorAll('[data-os-href-' + os + ']').forEach(function (a) { a.setAttribute('href', a.getAttribute('data-os-href-' + os)); });
  }
  // The app screen theme: every screen frame and the link that opens it point at /light/ or /dark/.
  function mockTheme(m) {
    document.querySelectorAll('a[data-mock],iframe[data-mock]').forEach(function (el) {
      ['src', 'href'].forEach(function (at) {
        var v = el.getAttribute(at);
        if (v === null) return;
        var nv = v.replace(/\/(light|dark)\//g, '/' + m + '/');
        if (nv !== v) el.setAttribute(at, nv);
      });
    });
  }
  function set(key, val) {
    if (key === 'os') {
      var pageOs = doc.getAttribute('data-page-os');
      store('of_os', val);
      if (pageOs && pageOs !== val && switchTo[val]) { location.href = switchTo[val]; return; }
      doc.setAttribute('data-os', val); osLinks(val);
    } else if (key === 'theme') { doc.setAttribute('data-theme', val); store('of_theme', val); }
    else if (key === 'mock') { doc.setAttribute('data-mock', val); store('of_mock', val); mockTheme(val); }
    mark();
  }
  function controls() {
    document.addEventListener('click', function (e) {
      var b = e.target.closest('[data-set]'); if (!b) return;
      set(b.getAttribute('data-set'), b.getAttribute('data-val'));
    });
    osLinks(doc.getAttribute('data-os'));
    if (doc.getAttribute('data-mock') !== 'light') mockTheme(doc.getAttribute('data-mock'));
    mark();
  }

  /* ---------- menus (desktop hover and click, mobile drawer) ---------- */
  function menus() {
    var menu = document.getElementById('menu'), tg = document.getElementById('menutg'), timers = new WeakMap();
    if (!menu || !tg) return;
    var head = tg.closest('.site-h') || menu, wide = matchMedia('(min-width:1100px)');
    function setOpen(li, on) { li.classList.toggle('open', on); li.firstElementChild.setAttribute('aria-expanded', String(on)); }
    function inDrawer() { return menu.classList.contains('open-m'); }
    // The drawer: the page behind it is inert and does not scroll; Escape, a link, a click outside or a wide window close it.
    function drawer(on, refocus) {
      menu.classList.toggle('open-m', on); tg.setAttribute('aria-expanded', String(on)); doc.classList.toggle('menu-lock', on);
      document.querySelectorAll('main, footer.site-f').forEach(function (e) { if (on) e.setAttribute('inert', ''); else e.removeAttribute('inert'); });
      if (!on) menu.querySelectorAll('.has-mega.open').forEach(function (o) { setOpen(o, false); });
      if (refocus) tg.focus();
    }
    menu.querySelectorAll('.has-mega').forEach(function (li) {
      li.addEventListener('mouseenter', function () { if (innerWidth <= 1099) return; clearTimeout(timers.get(li)); menu.querySelectorAll('.has-mega.open').forEach(function (o) { if (o !== li) setOpen(o, false); }); timers.set(li, setTimeout(function () { setOpen(li, true); }, 90)); });
      li.addEventListener('mouseleave', function () { if (innerWidth <= 1099) return; clearTimeout(timers.get(li)); timers.set(li, setTimeout(function () { setOpen(li, false); }, 170)); });
      li.firstElementChild.addEventListener('click', function () { setOpen(li, !li.classList.contains('open')); });
      li.addEventListener('focusout', function (e) { if (!li.contains(e.relatedTarget)) setOpen(li, false); });
    });
    document.addEventListener('keydown', function (e) {
      if (e.key === 'Escape' && !e.defaultPrevented) {
        var inside = head.contains(document.activeElement), openLi = menu.querySelector('.has-mega.open');
        if (inDrawer()) { drawer(false, inside); return; }
        if (openLi) { setOpen(openLi, false); if (openLi.contains(document.activeElement)) openLi.firstElementChild.focus(); }
      } else if (e.key === 'Tab' && inDrawer() && head.contains(document.activeElement)) {
        // keep Tab inside the header while the drawer is open
        var f = [].filter.call(head.querySelectorAll('a[href], button'), function (x) { return x.getClientRects().length; });
        var first = f[0], last = f[f.length - 1];
        if (e.shiftKey && document.activeElement === first) { e.preventDefault(); last.focus(); }
        else if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); first.focus(); }
      }
    });
    tg.addEventListener('click', function () { drawer(!inDrawer(), false); });
    menu.addEventListener('click', function (e) { if (inDrawer() && e.target.closest('a[href]')) drawer(false, false); });
    document.addEventListener('click', function (e) { if (inDrawer() && !head.contains(e.target)) drawer(false, false); });
    var onWide = function () { if (wide.matches && inDrawer()) drawer(false, false); };
    if (wide.addEventListener) wide.addEventListener('change', onWide); else wide.addListener(onWide);
  }

  /* ---------- store buttons: no listing yet, so a click says so, politely, next to the button (no layout shift) ---------- */
  function storeMsg() {
    var say = document.createElement('div'), bub = document.createElement('div'), timer, say0;
    say.className = 'sr store-say'; say.setAttribute('role', 'status'); say.setAttribute('aria-live', 'polite');
    bub.className = 'store-msg'; bub.setAttribute('aria-hidden', 'true');
    document.body.appendChild(say); document.body.appendChild(bub);
    var WHERE = { win: 'Microsoft Store', mac: 'the Mac App Store' };
    function hide() { clearTimeout(timer); clearTimeout(say0); bub.classList.remove('show'); say.textContent = ''; }
    addEventListener('resize', hide);
    document.addEventListener('keydown', function (e) { if (e.key === 'Escape') hide(); });
    document.addEventListener('click', function (e) {
      var b = e.target.closest('button[data-store]');
      if (!b) { hide(); return; }
      hide();
      var msg = 'Coming soon to ' + (WHERE[b.getAttribute('data-store')] || 'the store') + '.';
      bub.textContent = msg;
      var r = b.getBoundingClientRect(), w = bub.offsetWidth, h = bub.offsetHeight, up = r.bottom + h + 16 > innerHeight && r.top > h + 16;
      var left = Math.max(8, Math.min(r.left + r.width / 2 - w / 2, doc.clientWidth - w - 8));
      bub.style.left = (left + scrollX) + 'px';
      bub.style.top = ((up ? r.top - h - 10 : r.bottom + 10) + scrollY) + 'px';
      bub.classList.toggle('up', up);
      bub.style.setProperty('--ax', Math.round(Math.max(14, Math.min(r.left + r.width / 2 - left, w - 14))) + 'px');
      bub.classList.add('show');
      say0 = setTimeout(function () { say.textContent = msg; }, 60);
      timer = setTimeout(hide, 6000);
    });
  }

  /* ---------- home: Ideate, Write, Plan, Shoot switch between real screens ---------- */
  function stages() {
    document.querySelectorAll('[data-stages]').forEach(function (root) {
      var tabs = root.querySelectorAll('[role=tab]');
      tabs.forEach(function (t) {
        t.addEventListener('click', function () {
          tabs.forEach(function (o) {
            var on = o === t; o.setAttribute('aria-selected', String(on)); o.tabIndex = on ? 0 : -1;
            var p = document.getElementById(o.getAttribute('aria-controls')); if (p) p.hidden = !on;
          });
        });
        t.addEventListener('keydown', function (e) {
          var i = [].indexOf.call(tabs, t), n = e.key === 'ArrowRight' ? i + 1 : e.key === 'ArrowLeft' ? i - 1 : -9;
          if (n === -9) return; n = (n + tabs.length) % tabs.length; tabs[n].focus(); tabs[n].click();
        });
      });
    });
  }

  /* ---------- price for the visitor's region, with a manual choice ---------- */
  // A first guess from the time zone only (nothing is sent anywhere); the visitor can change it.
  function guessRegion() {
    var tz = ''; try { tz = Intl.DateTimeFormat().resolvedOptions().timeZone || ''; } catch (e) {}
    if (/^Asia\/(Kolkata|Calcutta)$/.test(tz)) return 'in';
    if (/^(Europe\/|Atlantic\/(Reykjavik|Canary|Madeira|Azores|Faroe))/.test(tz)) return 'eu';
    return 'world';
  }
  function region(r) {
    document.querySelectorAll('[data-price]').forEach(function (p) {
      p.querySelectorAll('[data-price-both]').forEach(function (s) { s.hidden = true; });
      p.querySelector('[data-price-in]').hidden = r !== 'in';
      p.querySelector('[data-price-eu]').hidden = r !== 'eu';
      p.querySelector('[data-price-us]').hidden = r !== 'world';
    });
    document.querySelectorAll('[data-region-set]').forEach(function (b) { b.setAttribute('aria-pressed', String(b.getAttribute('data-region-set') === r)); });
  }
  function pricing() {
    if (!document.querySelector('[data-price]')) return;
    var saved = store('of_region'), r = saved === 'in' || saved === 'eu' || saved === 'world' ? saved : guessRegion();
    region(r);
    document.addEventListener('click', function (e) {
      var b = e.target.closest('[data-region-set]'); if (!b) return;
      var v = b.getAttribute('data-region-set'); store('of_region', v); region(v);
    });
  }

  /* ---------- scroll reveal ---------- */
  function reveal() {
    var els = document.querySelectorAll('.rv');
    if (!('IntersectionObserver' in window)) { els.forEach(function (e) { e.classList.add('in'); }); return; }
    var io = new IntersectionObserver(function (es) { es.forEach(function (en) { if (en.isIntersecting) { en.target.classList.add('in'); io.unobserve(en.target); } }); }, { threshold: .15 });
    els.forEach(function (e) { io.observe(e); });
  }

  /* ---------- glass header tightens once the page scrolls ---------- */
  function headerState() {
    var h = document.querySelector('.site-h'); if (!h) return;
    var on = false, tick = false;
    // two thresholds, so the class never flips back and forth near one scroll position
    function upd() { tick = false; var v = on ? scrollY > 8 : scrollY > 32; if (v !== on) { on = v; h.classList.toggle('scrolled', v); } }
    addEventListener('scroll', function () { if (!tick) { tick = true; requestAnimationFrame(upd); } }, { passive: true });
    upd();
  }

  /* ---------- loading: the page splash (the Straft icon) lifts once the page has loaded; first visit of a session ---------- */
  function splash() {
    var done = false;
    function lift() { if (done) return; done = true; doc.classList.add('ready'); try { sessionStorage.setItem('of_seen', '1'); } catch (e) {} }
    if (doc.classList.contains('ready')) return;
    function onload() { setTimeout(lift, Math.max(0, 1100 - (window.performance ? performance.now() : 0))); }
    if (document.readyState === 'complete') onload(); else addEventListener('load', onload);
    setTimeout(lift, 4000);
    addEventListener('pageshow', function (e) { if (e.persisted) lift(); });
  }
  /* app screens are live pages in frames. A card shows one rectangle of its page: the page is laid out at its own
     size and scaled and moved so that the rectangle fills the card (a thumbnail card covers its box, top edge first). */
  function fitView(v) {
    var f = v.firstElementChild, pg = v.getAttribute('data-page').split(' '), r = v.getAttribute('data-rect').split(' ').map(Number);
    var w = v.clientWidth, h = v.clientHeight;
    if (!f || !w || !h || !r[2] || !r[3]) return;
    var cover = v.classList.contains('cover'), k = cover ? Math.max(w / r[2], h / r[3]) : w / r[2];
    f.style.width = pg[0] + 'px'; f.style.height = pg[1] + 'px';
    f.style.transform = 'translate(' + ((cover ? (w - r[2] * k) / 2 : 0) - r[0] * k) + 'px,' + (-r[1] * k) + 'px) scale(' + k + ')';
  }
  function shotViews() {
    var ro = 'ResizeObserver' in window ? new ResizeObserver(function (es) { es.forEach(function (en) { fitView(en.target); }); }) : null;
    document.querySelectorAll('.shot-view').forEach(function (v) { fitView(v); if (ro) ro.observe(v); });
    // a card waits behind the Straft icon (or its soft box) until its page has loaded
    function done(f) { var b = f.closest('.shot-open,.thumb'); if (b) b.classList.add('loaded'); }
    ['load', 'error'].forEach(function (t) {
      document.addEventListener(t, function (e) { if (e.target && e.target.tagName === 'IFRAME' && e.target.closest('.shot-view')) done(e.target); }, true);
    });
    document.querySelectorAll('.shot-view iframe').forEach(function (f) {
      try { if (f.contentDocument.readyState === 'complete' && f.contentWindow.location.href !== 'about:blank') done(f); } catch (e) {}
    });
  }
  /* error pages: Try again */
  function reloads() { document.addEventListener('click', function (e) { if (e.target.closest('[data-reload]')) location.reload(); }); }

  function init() { splash(); shotViews(); reloads(); controls(); menus(); storeMsg(); stages(); pricing(); reveal(); headerState(); }
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', init); else init();
  addEventListener('pageshow', function () { mark(); });
})();
