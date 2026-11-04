/* Get Straft: a dialog with the two store buttons and the price, and a small pill that opens it.
   The content comes from <template id="getapp-tpl"> (footer in layout.mjs). Nothing is sent anywhere.
   On the Home page, on screens 768 px wide and up, it opens once per browser session after the first scroll past the hero.
   Everywhere else, and after it is closed, only the pill shows. This file is loaded BEFORE site.js on purpose: the
   dialog must be in the page when site.js sets the regional price and the platform links. */
var tpl = document.getElementById('getapp-tpl');
var nav = document.body.getAttribute('data-nav') || '';
// The download pages and the pricing page already carry the same buttons.
if (tpl && tpl.content && nav !== 'download' && nav !== 'pricing') {
  var doc = document.documentElement, KEY = 'of_getapp_seen', wide = matchMedia('(min-width:768px)');
  var seen = function (v) { try { if (v === undefined) return sessionStorage.getItem(KEY); sessionStorage.setItem(KEY, v); } catch (e) { return null; } };
  var frag = tpl.content.cloneNode(true), dlg = frag.querySelector('dialog'), pill = frag.querySelector('.gp-pill'), opener = null;
  document.body.appendChild(frag);

  // The visitor's platform first. The DOM order changes, so the Tab order matches what is seen.
  var order = function () {
    var body = dlg.querySelector('.gp-body'), first = body.querySelector('[data-gp-os="' + (doc.getAttribute('data-os') === 'mac' ? 'mac' : 'win') + '"]');
    if (first) body.insertBefore(first, body.firstChild);
  };
  var open = function () {
    if (dlg.open) return;
    order(); opener = document.activeElement; dlg.showModal(); doc.classList.add('gp-open'); seen('1');
  };
  // The store message bubble (site.js) lives in the page, which sits under an open modal. Move it into the dialog
  // while the dialog is open, in viewport coordinates.
  // shortcut: reads the left and top that site.js has just set; upgrade if site.js changes how it positions the bubble.
  dlg.addEventListener('click', function (e) {
    if (e.target === dlg) { dlg.close(); return; }
    if (!e.target.closest('button[data-store]')) return;
    setTimeout(function () {
      var b = document.querySelector('.store-msg'); if (!b || !dlg.open) return;
      if (b.parentNode !== dlg) dlg.appendChild(b);
      b.style.position = 'fixed';
      b.style.left = (parseFloat(b.style.left) - scrollX) + 'px'; b.style.top = (parseFloat(b.style.top) - scrollY) + 'px';
    }, 0);
  });
  dlg.querySelector('[data-gp-close]').addEventListener('click', function () { dlg.close(); });
  dlg.addEventListener('close', function () {
    doc.classList.remove('gp-open');
    var b = dlg.querySelector('.store-msg');
    if (b) { document.body.appendChild(b); b.style.position = ''; b.classList.remove('show'); }
    pill.hidden = false;
    if (opener && opener.focus && document.contains(opener)) opener.focus();
  });
  pill.addEventListener('click', open);

  var hero = document.querySelector('.hero');
  var past = function () { return hero ? hero.getBoundingClientRect().bottom < innerHeight * 0.5 : scrollY > 400; };
  var showPill = function () { pill.hidden = false; };
  var onScroll = function () {
    if (!past()) return;
    removeEventListener('scroll', onScroll);
    if (!wide.matches) { showPill(); return; }
    setTimeout(function () {
      // Never on top of another dialog or the open menu; try again on the next scroll.
      if (document.querySelector('dialog[open]') || doc.classList.contains('menu-lock')) addEventListener('scroll', onScroll, { passive: true });
      else open();
    }, 400);
  };
  if (nav === 'home' && !seen() && wide.matches) addEventListener('scroll', onScroll, { passive: true });
  else setTimeout(showPill, 1500);
}
