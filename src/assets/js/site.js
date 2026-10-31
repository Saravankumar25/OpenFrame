/* OpenFrame website behaviour: the three independent controls (platform, website theme, app screen theme), app screen
   frames, menus, home screen switcher, regional price. No dependencies, no tracking, nothing sent anywhere. */
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
  // The app screen theme: every embedded screen and every "Open this screen" link points at /light/ or /dark/.
  function mockTheme(m) {
    document.querySelectorAll('iframe[data-mock],a[data-mock],img[data-mock]').forEach(function (el) {
      var at = el.tagName === 'A' ? 'href' : 'src', v = el.getAttribute(at);
      var nv = v.replace(/\/(light|dark)\//, '/' + m + '/');
      if (nv !== v) el.setAttribute(at, nv);
    });
  }
  function set(key, val) {
    if (key === 'os') {
      var pageOs = doc.getAttribute('data-page-os');
      store('of_os', val);
      if (pageOs && pageOs !== val && switchTo[val]) { location.href = switchTo[val]; return; }
      doc.setAttribute('data-os', val); osLinks(val); frames();
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

  /* ---------- app screen frames: scale the captured page to the frame width ---------- */
  var ro = 'ResizeObserver' in window ? new ResizeObserver(function (es) { es.forEach(function (en) { fit(en.target); }); }) : null;
  function fit(box) {
    var w = +box.getAttribute('data-w') || 1600, h = +box.getAttribute('data-h') || 1000, f = box.querySelector('iframe');
    if (!f || !box.clientWidth) return;
    box.style.aspectRatio = w + ' / ' + h;
    f.style.width = w + 'px'; f.style.height = h + 'px';
    f.style.transform = 'scale(' + (box.clientWidth / w) + ')';
  }
  function frames() {
    document.querySelectorAll('.mock').forEach(function (box) {
      if (!box._of) { box._of = 1; if (ro) ro.observe(box); }
      fit(box);
    });
  }

  /* ---------- menus (desktop hover and click, mobile drawer) ---------- */
  function menus() {
    var menu = document.getElementById('menu'), tg = document.getElementById('menutg'), timers = new WeakMap();
    if (!menu || !tg) return;
    function setOpen(li, on) { li.classList.toggle('open', on); li.firstElementChild.setAttribute('aria-expanded', String(on)); }
    menu.querySelectorAll('.has-mega').forEach(function (li) {
      li.addEventListener('mouseenter', function () { if (innerWidth <= 1099) return; clearTimeout(timers.get(li)); menu.querySelectorAll('.has-mega.open').forEach(function (o) { if (o !== li) setOpen(o, false); }); timers.set(li, setTimeout(function () { setOpen(li, true); }, 90)); });
      li.addEventListener('mouseleave', function () { if (innerWidth <= 1099) return; clearTimeout(timers.get(li)); timers.set(li, setTimeout(function () { setOpen(li, false); }, 170)); });
      li.firstElementChild.addEventListener('click', function () { setOpen(li, !li.classList.contains('open')); });
      li.addEventListener('focusout', function (e) { if (!li.contains(e.relatedTarget)) setOpen(li, false); });
    });
    document.addEventListener('keydown', function (e) { if (e.key === 'Escape') menu.querySelectorAll('.open').forEach(function (o) { setOpen(o, false); }); });
    tg.addEventListener('click', function () { var on = menu.classList.toggle('open-m'); tg.setAttribute('aria-expanded', String(on)); document.body.style.overflow = on ? 'hidden' : ''; });
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
          frames();
        });
        t.addEventListener('keydown', function (e) {
          var i = [].indexOf.call(tabs, t), n = e.key === 'ArrowRight' ? i + 1 : e.key === 'ArrowLeft' ? i - 1 : -9;
          if (n === -9) return; n = (n + tabs.length) % tabs.length; tabs[n].focus(); tabs[n].click();
        });
      });
    });
  }

  /* ---------- price for the visitor's region, with a manual choice ---------- */
  function guessIndia() {
    var tz = ''; try { tz = Intl.DateTimeFormat().resolvedOptions().timeZone || ''; } catch (e) {}
    return /^Asia\/(Kolkata|Calcutta)$/.test(tz);
  }
  function region(r) {
    document.querySelectorAll('[data-price]').forEach(function (p) {
      p.querySelectorAll('[data-price-both]').forEach(function (s) { s.hidden = true; });
      p.querySelector('[data-price-in]').hidden = r !== 'in';
      p.querySelector('[data-price-us]').hidden = r === 'in';
    });
    document.querySelectorAll('[data-region-name]').forEach(function (s) { s.textContent = r === 'in' ? 'India' : 'outside India'; });
    document.querySelectorAll('[data-region-set]').forEach(function (b) { b.setAttribute('aria-pressed', String(b.getAttribute('data-region-set') === r)); });
  }
  function pricing() {
    if (!document.querySelector('[data-price]')) return;
    var saved = store('of_region'), r = saved === 'in' || saved === 'world' ? saved : guessIndia() ? 'in' : 'world';
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
    function upd() { tick = false; var v = scrollY > 24; if (v !== on) { on = v; h.classList.toggle('scrolled', v); } }
    addEventListener('scroll', function () { if (!tick) { tick = true; requestAnimationFrame(upd); } }, { passive: true });
    upd();
  }

  /* ---------- loading: the page splash (the OpenFrame icon) lifts once the page has loaded; first visit of a session ---------- */
  function splash() {
    var done = false;
    function lift() { if (done) return; done = true; doc.classList.add('ready'); try { sessionStorage.setItem('of_seen', '1'); } catch (e) {} }
    if (doc.classList.contains('ready')) return;
    function onload() { setTimeout(lift, Math.max(0, 1100 - (window.performance ? performance.now() : 0))); }
    if (document.readyState === 'complete') onload(); else addEventListener('load', onload);
    setTimeout(lift, 4000);
    addEventListener('pageshow', function (e) { if (e.persisted) lift(); });
  }
  /* each app screen waits behind the same icon until its page has loaded */
  function frameLoads() {
    document.addEventListener('load', function (e) { var f = e.target; if (f && f.tagName === 'IFRAME') { var b = f.closest('.mock'); if (b) b.classList.add('loaded'); } }, true);
    document.querySelectorAll('.mock iframe').forEach(function (f) { try { if (f.contentDocument && f.contentDocument.readyState === 'complete' && f.contentDocument.body && f.contentDocument.body.childNodes.length) f.closest('.mock').classList.add('loaded'); } catch (e) {} });
  }
  /* error pages: Try again */
  function reloads() { document.addEventListener('click', function (e) { if (e.target.closest('[data-reload]')) location.reload(); }); }

  function init() { splash(); frameLoads(); reloads(); controls(); frames(); menus(); stages(); pricing(); reveal(); headerState(); }
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', init); else init();
  addEventListener('pageshow', function () { mark(); });
})();
