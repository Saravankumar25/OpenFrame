/* OpenFrame website behaviour: platform switch, menus, missing-tutorial notice. No dependencies, no tracking. */
(function () {
  var doc = document.documentElement;
  function store(k, v) { try { if (v === undefined) return localStorage.getItem(k); localStorage.setItem(k, v); } catch (e) { return null; } }
  function sess(k, v) { try { if (v === undefined) return sessionStorage.getItem(k); if (v === null) sessionStorage.removeItem(k); else sessionStorage.setItem(k, v); } catch (e) { return null; } }

  /* Platform-aware links: Tutorials and Download point at the visitor's platform. */
  function applyOs(os) {
    doc.setAttribute('data-os', os);
    document.querySelectorAll('[data-os-href-' + os + ']').forEach(function (a) { a.setAttribute('href', a.getAttribute('data-os-href-' + os)); });
    document.querySelectorAll('.os-opt').forEach(function (a) {
      if (a.getAttribute('data-os-pick') === os) a.setAttribute('aria-current', 'true'); else a.removeAttribute('aria-current');
    });
  }

  function osSwitch() {
    var pageOs = doc.getAttribute('data-page-os');
    applyOs(doc.getAttribute('data-os') || 'win');
    document.querySelectorAll('.os-opt').forEach(function (a) {
      a.addEventListener('click', function (e) {
        var os = a.getAttribute('data-os-pick');
        store('of_os', os);
        if (pageOs) {
          /* Platform page: go to the matching page of the other platform, keeping the reading position. */
          if (os === pageOs) { e.preventDefault(); return; }
          var max = Math.max(1, doc.scrollHeight - innerHeight);
          sess('of_scroll', JSON.stringify({ to: a.pathname, ratio: scrollY / max }));
          return;
        }
        /* Shared page: swap the platform-specific content in place, no reload. */
        e.preventDefault();
        applyOs(os);
      });
    });
    /* Restore the reading position after a platform switch. */
    var saved = sess('of_scroll');
    if (saved) {
      sess('of_scroll', null);
      try {
        var s = JSON.parse(saved);
        if (s.to === location.pathname && s.ratio > 0) addEventListener('load', function () { scrollTo(0, s.ratio * Math.max(1, doc.scrollHeight - innerHeight)); });
      } catch (e) { /* ignore a malformed value */ }
    }
  }

  /* Features mega menu and the small-screen menu. */
  function menus() {
    var menu = document.getElementById('menu'), tg = document.getElementById('menutg');
    if (!menu) return;
    var wide = matchMedia('(min-width: 1081px)');
    menu.querySelectorAll('.has-mega').forEach(function (li) {
      var btn = li.querySelector('.menu-btn'), t;
      function set(on) { li.classList.toggle('open', on); btn.setAttribute('aria-expanded', on ? 'true' : 'false'); }
      btn.addEventListener('click', function () { set(!li.classList.contains('open')); });
      li.addEventListener('mouseenter', function () { if (!wide.matches) return; clearTimeout(t); t = setTimeout(function () { set(true); }, 80); });
      li.addEventListener('mouseleave', function () { if (!wide.matches) return; clearTimeout(t); t = setTimeout(function () { set(false); }, 160); });
      li.addEventListener('focusout', function (e) { if (wide.matches && !li.contains(e.relatedTarget)) set(false); });
      document.addEventListener('keydown', function (e) { if (e.key === 'Escape' && li.classList.contains('open')) { set(false); btn.focus(); } });
    });
    if (tg) tg.addEventListener('click', function () {
      var hdr = document.querySelector('.site-h');
      if (hdr) doc.style.setProperty('--hdr', Math.round(hdr.getBoundingClientRect().bottom) + 'px');
      var on = menu.classList.toggle('open-m');
      tg.setAttribute('aria-expanded', on ? 'true' : 'false');
      document.body.style.overflow = on ? 'hidden' : '';
    });
    wide.addEventListener('change', function () { menu.classList.remove('open-m'); if (tg) tg.setAttribute('aria-expanded', 'false'); document.body.style.overflow = ''; });
  }

  /* /<platform>/tutorials/?missing=<id>: explain that the tutorial does not exist for this platform. */
  function missingNotice() {
    var box = document.getElementById('t-missing');
    if (!box) return;
    var id = new URLSearchParams(location.search).get('missing');
    if (!id || !/^[a-z0-9-]{1,60}$/.test(id)) return;
    var data = {};
    var el = document.getElementById('t-missing-data');
    if (el) { try { data = JSON.parse(el.textContent) || {}; } catch (e) { data = {}; } }
    var os = doc.getAttribute('data-page-os') === 'mac' ? 'macOS' : 'Windows';
    var info = Object.prototype.hasOwnProperty.call(data, id) ? data[id] : null;
    var title = info && info.title ? info.title : id.replace(/-/g, ' ');
    box.textContent = 'The tutorial "' + title + '" is not available for ' + os + '. ' + (info && info.reason ? info.reason + ' ' : '') + 'Here are all the ' + os + ' tutorials.';
    box.setAttribute('role', 'status');
    box.hidden = false;
  }

  function init() { osSwitch(); menus(); missingNotice(); }
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', init); else init();
})();
