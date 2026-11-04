/* The Home page pop-up: on every screen size it opens once per browser session, after the first scroll past the hero.
   Its one button leads to the download page. The content comes from <template id="getapp-tpl"> (footer in layout.mjs).
   Nothing is sent anywhere. */
var tpl = document.getElementById('getapp-tpl');
if (tpl && tpl.content) {
  var doc = document.documentElement, KEY = 'of_getapp_seen';
  var seen = function (v) { try { if (v === undefined) return sessionStorage.getItem(KEY); sessionStorage.setItem(KEY, v); } catch (e) { return null; } };
  var frag = tpl.content.cloneNode(true), dlg = frag.querySelector('dialog'), opener = null;
  document.body.appendChild(frag);

  // Closing on the link means the Back button never returns to a page with the dialog still open.
  dlg.addEventListener('click', function (e) { if (e.target === dlg || e.target.closest('a[href]')) dlg.close(); });
  dlg.querySelector('[data-gp-close]').addEventListener('click', function () { dlg.close(); });
  dlg.addEventListener('close', function () {
    doc.classList.remove('gp-open');
    if (opener && opener.focus && document.contains(opener)) opener.focus();
  });

  var hero = document.querySelector('.hero');
  var past = function () { return hero ? hero.getBoundingClientRect().bottom < innerHeight * 0.5 : scrollY > 400; };
  var onScroll = function () {
    if (!past()) return;
    removeEventListener('scroll', onScroll);
    setTimeout(function () {
      // Never on top of another dialog; try again on the next scroll.
      if (document.querySelector('dialog[open]')) { addEventListener('scroll', onScroll, { passive: true }); return; }
      opener = document.activeElement; dlg.showModal(); doc.classList.add('gp-open'); seen('1');
    }, 400);
  };
  if (!seen()) addEventListener('scroll', onScroll, { passive: true });
}
