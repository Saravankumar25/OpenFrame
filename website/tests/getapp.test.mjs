// Get Straft dialog and pill: when it opens, who sees it, platform order, regional price, wording, axe, fit at phone width.
import test, { before, after } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { chromium } from 'playwright';
import AxeBuilder from '@axe-core/playwright';
import { DIST, ROOT, HAS_DIST } from './_helpers.mjs';
import { startServer } from '../scripts/serve.mjs';

const SKIP = HAS_DIST ? false : 'dist/ is missing: run the build first';
let server, browser, base;
before(async () => {
  if (SKIP) return;
  server = await startServer(0);
  base = `http://127.0.0.1:${server.address().port}`;
  browser = await chromium.launch();
});
after(async () => { await browser?.close(); await new Promise((r) => (server ? server.close(r) : r())); });

async function withPage(fn, opts = {}, init) {
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 }, reducedMotion: 'reduce', ...opts });
  const page = await ctx.newPage();
  if (init) await page.addInitScript(init);
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  page.on('console', (m) => { if (m.type() === 'error' && !/favicon|Failed to load resource/.test(m.text())) errors.push(m.text()); });
  try { await fn(page); } finally { await ctx.close(); }
  assert.deepEqual(errors, []);
}
// The page splash covers the page for about a second; interaction waits until it has lifted.
async function go(page, p) { await page.goto(base + p); await page.waitForFunction(() => document.documentElement.classList.contains('ready')); }
const isOpen = (page) => page.evaluate(() => !!document.querySelector('dialog.gp-dlg[open]'));
const pillShown = (page) => page.evaluate(() => { const p = document.querySelector('.gp-pill'); return !!p && !p.hidden && getComputedStyle(p).display !== 'none'; });
const scrollDown = (page) => page.evaluate(() => window.scrollTo(0, 2600));
const PHONE = { viewport: { width: 390, height: 800 }, isMobile: true, hasTouch: true };

test('Home at 1280: opens by itself after the first scroll past the hero, and only once per session', { skip: SKIP }, () => withPage(async (page) => {
  await go(page, '/');
  await page.waitForTimeout(800);
  assert.equal(await isOpen(page), false, 'not before the scroll');
  assert.equal(await pillShown(page), false, 'no pill while the dialog is still to come');
  await scrollDown(page);
  await page.waitForSelector('dialog.gp-dlg[open]', { timeout: 4000 });
  assert.equal(await page.evaluate(() => document.documentElement.classList.contains('gp-open') && getComputedStyle(document.documentElement).overflow), 'hidden', 'page scroll is locked');
  assert.equal(await page.evaluate(() => sessionStorage.getItem('of_getapp_seen')), '1');
  assert.equal(await pillShown(page), false, 'the pill hides while the dialog is open');
  await page.keyboard.press('Escape');
  assert.equal(await isOpen(page), false);
  // the dialog's close event is a task of its own, so the pill comes back one tick after Escape
  await page.waitForFunction(() => { const p = document.querySelector('.gp-pill'); return !p.hidden && getComputedStyle(p).display !== 'none'; }, null, { timeout: 2000 });
  await page.reload();
  await page.waitForFunction(() => document.documentElement.classList.contains('ready'));
  await scrollDown(page);
  await page.waitForTimeout(1200);
  assert.equal(await isOpen(page), false, 'not again in the same session');
  await page.waitForSelector('.gp-pill:not([hidden])', { timeout: 4000 });
}));

test('Home at 390: never opens by itself; the pill shows after about 1.5 seconds', { skip: SKIP }, () => withPage(async (page) => {
  await go(page, '/');
  await scrollDown(page);
  await page.waitForTimeout(1200);
  assert.equal(await isOpen(page), false);
  await page.waitForSelector('.gp-pill:not([hidden])', { timeout: 4000 });
  assert.equal(await page.evaluate(() => sessionStorage.getItem('of_getapp_seen')), null);
  const r = await page.evaluate(() => { const b = document.querySelector('.gp-pill').getBoundingClientRect(); return { right: innerWidth - b.right, bottom: innerHeight - b.bottom }; });
  assert.ok(r.right >= 12 && r.bottom >= 12, 'inside the screen with a margin: ' + JSON.stringify(r));
}, PHONE));

test('the pill opens the dialog, Escape closes it, focus returns to the pill, the backdrop closes it too', { skip: SKIP }, () => withPage(async (page) => {
  await go(page, '/faq/');
  await page.waitForSelector('.gp-pill:not([hidden])', { timeout: 4000 });
  assert.equal(await isOpen(page), false);
  await page.focus('.gp-pill');
  await page.keyboard.press('Enter');
  await page.waitForSelector('dialog.gp-dlg[open]');
  assert.equal(await page.evaluate(() => document.activeElement.closest('dialog.gp-dlg') !== null), true, 'focus moves into the dialog');
  assert.equal(await page.evaluate(() => document.querySelector('dialog.gp-dlg h2').textContent), 'Try Straft Studio free for 7 days');
  await page.keyboard.press('Escape');
  assert.equal(await isOpen(page), false);
  assert.equal(await page.evaluate(() => document.activeElement.classList.contains('gp-pill')), true, 'focus is back on the pill');
  await page.click('.gp-pill');
  await page.waitForSelector('dialog.gp-dlg[open]');
  await page.mouse.click(8, 8);
  assert.equal(await isOpen(page), false, 'a click on the backdrop closes it');
  await page.click('.gp-pill');
  await page.click('[data-gp-close]');
  assert.equal(await isOpen(page), false, 'the close button closes it');
}));

test('Tab stays inside the dialog', { skip: SKIP }, () => withPage(async (page) => {
  await go(page, '/faq/');
  await page.waitForSelector('.gp-pill:not([hidden])', { timeout: 4000 });
  await page.click('.gp-pill');
  for (let i = 0; i < 12; i++) {
    await page.keyboard.press('Tab');
    const inside = await page.evaluate(() => document.activeElement === document.body || !!document.activeElement.closest('dialog.gp-dlg'));
    assert.ok(inside, 'Tab ' + i);
  }
}));

test('no pill on /download/ and /pricing/', { skip: SKIP }, () => withPage(async (page) => {
  for (const p of ['/download/', '/pricing/']) {
    await go(page, p);
    await page.waitForTimeout(2000);
    assert.equal(await page.evaluate(() => !!document.querySelector('.gp-pill, dialog.gp-dlg')), false, p);
  }
}));

test('the pill hides while search or the menu is open', { skip: SKIP }, () => withPage(async (page) => {
  await go(page, '/faq/');
  await page.waitForSelector('.gp-pill:not([hidden])', { timeout: 4000 });
  await page.click('.srch-btn');
  await page.waitForSelector('dialog.srch-dlg[open]');
  assert.equal(await pillShown(page), false, 'search open');
  await page.keyboard.press('Escape');
  await page.waitForFunction(() => { const p = document.querySelector('.gp-pill'); return getComputedStyle(p).display !== 'none'; });
  await page.setViewportSize({ width: 390, height: 800 });
  await page.click('#menutg');
  assert.equal(await pillShown(page), false, 'menu open');
}));

test('the platform follows data-os: the visitor\'s platform comes first', { skip: SKIP }, async () => {
  for (const os of ['win', 'mac']) {
    await withPage(async (page) => {
      await go(page, '/faq/');
      await page.waitForSelector('.gp-pill:not([hidden])', { timeout: 4000 });
      await page.click('.gp-pill');
      const order = () => page.$$eval('dialog.gp-dlg [data-gp-os]', (s) => s.map((x) => x.getAttribute('data-gp-os')));
      const other = os === 'win' ? 'mac' : 'win';
      assert.deepEqual(await order(), [os, other]);
      assert.equal(await page.$$eval('dialog.gp-dlg .btn[data-store]', (b) => b.map((x) => x.getAttribute('data-store')).join()), `${os},${other}`);
      // switching the platform while it is closed changes the order on the next open
      await page.keyboard.press('Escape');
      await page.evaluate((o) => document.documentElement.setAttribute('data-os', o), other);
      await page.click('.gp-pill');
      assert.deepEqual(await order(), [other, os]);
    }, {}, `localStorage.setItem('of_os','${os}')`);
  }
});

test('the price follows the region', { skip: SKIP }, async () => {
  for (const [r, want, not] of [['in', '₹3,999', /€|US\$/], ['eu', '€89', /₹|US\$/], ['world', 'US$89', /₹|€/]]) {
    await withPage(async (page) => {
      await go(page, '/faq/');
      await page.waitForSelector('.gp-pill:not([hidden])', { timeout: 4000 });
      await page.click('.gp-pill');
      const t = await page.evaluate(() => document.querySelector('dialog.gp-dlg .gp-price').innerText);
      assert.ok(t.includes(want), `${r}: ${t}`);
      assert.ok(!not.test(t), `${r}: ${t}`);
      assert.match(await page.evaluate(() => document.querySelector('dialog.gp-dlg .gp-lead').textContent), /Pay once\. Own forever\. No subscription\./);
    }, {}, `localStorage.setItem('of_region','${r}')`);
  }
});

test('the store buttons are the site\'s own: enabled, data-store, and the bubble shows above the dialog', { skip: SKIP }, () => withPage(async (page) => {
  await go(page, '/faq/');
  await page.waitForSelector('.gp-pill:not([hidden])', { timeout: 4000 });
  await page.click('.gp-pill');
  const btns = await page.$$eval('dialog.gp-dlg .btn', (b) => b.map((x) => ({ tag: x.tagName, store: x.getAttribute('data-store'), off: x.disabled, text: x.textContent.trim() })));
  assert.deepEqual(btns, [{ tag: 'BUTTON', store: 'win', off: false, text: 'Download for Windows: Microsoft Store' }, { tag: 'BUTTON', store: 'mac', off: false, text: 'Download for Mac: Mac App Store' }]);
  assert.deepEqual(await page.$$eval('dialog.gp-dlg .gp-links a', (a) => a.map((x) => x.getAttribute('href'))), ['/pricing/', '/windows/tutorials/']);
  await page.click('dialog.gp-dlg .btn[data-store=win]');
  await page.waitForSelector('.store-msg.show');
  // getapp.js moves the bubble into the dialog one tick after site.js has placed it
  await page.waitForFunction(() => document.querySelector('.store-msg').parentNode === document.querySelector('dialog.gp-dlg'), null, { timeout: 2000 });
  const top = await page.evaluate(() => {
    const b = document.querySelector('.store-msg'), r = b.getBoundingClientRect();
    // elementFromPoint skips the bubble (pointer-events none), so check that it lives inside the modal dialog and is on screen
    return { text: b.textContent, r: [r.left, r.top, r.right, r.bottom], inDlg: b.parentNode === document.querySelector('dialog.gp-dlg'), onTop: b.parentNode === document.querySelector('dialog.gp-dlg') && r.left >= 0 && r.right <= innerWidth && r.top >= 0 && r.bottom <= innerHeight };
  });
  assert.equal(top.text, 'Coming soon to Microsoft Store.');
  assert.equal(top.onTop, true, 'the bubble is not hidden behind the dialog ' + JSON.stringify(top));
  await page.keyboard.press('Escape');
  await page.waitForFunction(() => document.querySelector('.store-msg').parentNode === document.body);
}));

test('wording: no sale talk, no form, no social links, no dashes', { skip: SKIP }, () => {
  const html = fs.readFileSync(path.join(DIST, 'index.html'), 'utf8');
  const tpl = html.match(/<template id="getapp-tpl">([\s\S]*?)<\/template>/);
  assert.ok(tpl, 'the template is on the page');
  const text = tpl[1].replace(/<[^>]+>/g, ' ');
  assert.doesNotMatch(tpl[1], /launch sale|early.?bird|lowest price|<form|<input|facebook|instagram|twitter|youtube|linkedin|style=|<script|\son[a-z]+=/i);
  assert.doesNotMatch(text, /[–—]|\blearn|\bcourse|seamless|unlock|elevate|robust/i);
  for (const f of ['getapp.js', 'getapp.css']) {
    const src = fs.readFileSync(path.join(ROOT, 'src', 'assets', f.endsWith('js') ? 'js' : 'css', f), 'utf8');
    assert.doesNotMatch(src, /[–—]/, f);
  }
});

for (const dark of [false, true]) {
  test(`axe passes on the open dialog (${dark ? 'dark' : 'light'} website theme)`, { skip: SKIP }, () => withPage(async (page) => {
    await go(page, '/faq/');
    await page.waitForSelector('.gp-pill:not([hidden])', { timeout: 4000 });
    await page.click('.gp-pill');
    const a = await new AxeBuilder({ page }).include('dialog.gp-dlg').analyze();
    assert.deepEqual(a.violations.map((v) => v.id + ': ' + v.nodes.map((n) => n.target).join(' ')), []);
  }, {}, `localStorage.setItem('of_theme','${dark ? 'dark' : 'light'}')`));
}

test('no horizontal overflow at 390, with the pill and with the dialog open', { skip: SKIP }, () => withPage(async (page) => {
  await go(page, '/faq/');
  await page.waitForSelector('.gp-pill:not([hidden])', { timeout: 4000 });
  const over = () => page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  assert.equal(await over(), 0, 'pill');
  await page.click('.gp-pill');
  assert.equal(await over(), 0, 'dialog');
  const box = await page.evaluate(() => { const r = document.querySelector('.gp-box').getBoundingClientRect(); return { l: r.left, r: innerWidth - r.right, t: r.top, b: innerHeight - r.bottom }; });
  assert.ok(box.l >= 8 && box.r >= 8 && box.t >= 0 && box.b >= 0, JSON.stringify(box));
}, PHONE));
