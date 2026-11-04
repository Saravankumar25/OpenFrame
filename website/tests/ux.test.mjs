// UX checks in a real browser: no sideways scroll at 320 px on any page, the mobile menu (Escape, focus, resize), heading
// order, the store-button message, tutorial previous/next links, control sizes, and contrast of the availability chips.
import test, { before, after } from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import AxeBuilder from '@axe-core/playwright';
import { SKIP, htmlFiles, pagePath, read } from './_helpers.mjs';
import { startServer } from '../scripts/serve.mjs';

let server, browser, base;
before(async () => {
  if (SKIP) return;
  server = await startServer(0);
  base = `http://127.0.0.1:${server.address().port}`;
  browser = await chromium.launch();
});
after(async () => { await browser?.close(); await new Promise((r) => (server ? server.close(r) : r())); });

async function withPage(fn, opts = {}) {
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 }, reducedMotion: 'reduce', ...opts });
  const page = await ctx.newPage();
  try { await fn(page); } finally { await ctx.close(); }
}

// Every page, except that the hundreds of tutorial pages share one layout: the first two of each platform stand for them.
const isTutorial = (p) => /^\/(windows|mac)\/tutorials\/.+/.test(p);
const pages = () => {
  const all = htmlFiles().map(pagePath);
  const tut = ['/windows/tutorials/', '/mac/tutorials/'].flatMap((b) => all.filter((p) => isTutorial(p) && p.startsWith(b)).slice(0, 2));
  return all.filter((p) => !isTutorial(p)).concat(tut);
};

test('no sideways scroll at 320 px on any page', { skip: SKIP }, () => withPage(async (page) => {
  const bad = [];
  for (const p of pages()) {
    await page.goto(base + p);
    const over = await page.evaluate(() => document.documentElement.scrollWidth - innerWidth);
    if (over > 0) bad.push(`${p} +${over}px`);
  }
  assert.deepEqual(bad, []);
}, { viewport: { width: 320, height: 700 } }));

test('no sideways scroll on the pages that used to overflow, at every width', { skip: SKIP }, () => withPage(async (page) => {
  const bad = [];
  for (const w of [320, 375, 390, 768, 1024, 1280, 1440, 1920]) {
    await page.setViewportSize({ width: w, height: 800 });
    for (const p of ['/switch/', '/download/', '/windows/', '/mac/', '/pricing/']) {
      await page.goto(base + p);
      const over = await page.evaluate(() => document.documentElement.scrollWidth - innerWidth);
      if (over > 0) bad.push(`${p} at ${w}: +${over}px`);
    }
  }
  assert.deepEqual(bad, []);
}));

test('heading levels never skip and each page has one h1', { skip: SKIP }, () => withPage(async (page) => {
  const bad = [];
  for (const p of pages()) {
    await page.goto(base + p);
    const levels = await page.$$eval('h1,h2,h3,h4,h5,h6', (els) => els.filter((e) => e.getClientRects().length || e.classList.contains('sr')).map((e) => +e.tagName[1]));
    let prev = 0;
    levels.forEach((l) => { if (l > prev + 1) bad.push(`${p}: h${prev} then h${l}`); prev = l; });
    if (levels.filter((l) => l === 1).length !== 1) bad.push(`${p}: ${levels.filter((l) => l === 1).length} h1`);
  }
  assert.deepEqual([...new Set(bad)], []);
}));

test('mobile menu: Escape closes it and returns focus, the page scrolls again, Tab stays inside', { skip: SKIP }, () => withPage(async (page) => {
  await page.goto(base + '/');
  const open = () => page.evaluate(() => ({ m: document.getElementById('menu').classList.contains('open-m'), x: document.getElementById('menutg').getAttribute('aria-expanded'), lock: document.documentElement.classList.contains('menu-lock'), inert: document.getElementById('main').hasAttribute('inert') }));
  await page.click('#menutg');
  assert.deepEqual(await open(), { m: true, x: 'true', lock: true, inert: true });
  assert.equal(await page.evaluate(() => getComputedStyle(document.documentElement).overflow), 'hidden');
  // Tab many times: focus never leaves the header
  for (let i = 0; i < 40; i++) {
    await page.keyboard.press('Tab');
    assert.ok(await page.evaluate(() => !!document.activeElement.closest('.site-h')), `tab ${i} left the header`);
  }
  await page.focus('#menu a[href="/pricing/"]');
  await page.keyboard.press('Escape');
  assert.deepEqual(await open(), { m: false, x: 'false', lock: false, inert: false });
  assert.equal(await page.evaluate(() => document.activeElement.id), 'menutg');
  assert.notEqual(await page.evaluate(() => getComputedStyle(document.documentElement).overflow), 'hidden');
  // growing to desktop width with the menu open restores scrolling
  await page.click('#menutg');
  await page.setViewportSize({ width: 1300, height: 800 });
  assert.deepEqual(await open(), { m: false, x: 'false', lock: false, inert: false });
  // a tap outside the open drawer closes it
  await page.setViewportSize({ width: 375, height: 700 });
  await page.click('#menutg');
  await page.mouse.click(5, 690);
  assert.equal((await open()).m, false);
}, { viewport: { width: 375, height: 700 } }));

test('desktop menu: Escape closes an open panel and returns focus to its button', { skip: SKIP }, () => withPage(async (page) => {
  await page.goto(base + '/');
  const b = page.locator('#menu .has-mega > button').first();
  await b.click();
  assert.equal(await b.getAttribute('aria-expanded'), 'true');
  await page.keyboard.press('Escape');
  assert.equal(await b.getAttribute('aria-expanded'), 'false');
  assert.ok(await b.evaluate((e) => e === document.activeElement));
}));

test('store buttons: a polite message appears beside the button, is announced, hides itself and shifts nothing', { skip: SKIP }, () => withPage(async (page) => {
  await page.goto(base + '/download/');
  const cases = [['win', 'Coming soon to Microsoft Store.'], ['mac', 'Coming soon to the Mac App Store.']];
  for (const [plat, text] of cases) {
    const btn = page.locator(`button[data-store="${plat}"]`).first();
    await btn.scrollIntoViewIfNeeded();
    const before = await page.evaluate(() => ({ h: document.documentElement.scrollHeight, w: document.documentElement.scrollWidth }));
    const docBox = () => btn.evaluate((e) => { return [e.offsetLeft, e.offsetTop, e.offsetWidth, e.offsetHeight]; });
    const boxBefore = await docBox();
    await btn.click();
    const bub = page.locator('.store-msg.show');
    assert.equal((await bub.textContent()).trim(), text);
    await page.waitForFunction(() => document.querySelector('.store-say')?.textContent.trim() !== '');
    assert.equal((await page.locator('.store-say').last().textContent()).trim(), text, 'announced');
    // near the button, inside the window
    const r = await bub.boundingBox(), b = await btn.boundingBox();
    assert.ok(Math.abs(r.y - (b.y + b.height)) < 40 || Math.abs(b.y - (r.y + r.height)) < 40, 'message is next to the button');
    assert.ok(r.x >= 0 && r.x + r.width <= 320 + 1, 'message is inside the window');
    assert.deepEqual(await page.evaluate(() => ({ h: document.documentElement.scrollHeight, w: document.documentElement.scrollWidth })), before, 'no layout shift');
    assert.deepEqual(await docBox(), boxBefore, 'button did not move');
    assert.equal(new URL(page.url()).pathname, '/download/');
    await page.waitForSelector('.store-msg.show', { state: 'detached', timeout: 9000 });
    assert.equal((await page.locator('.store-say').last().textContent()).trim(), '');
  }
}, { viewport: { width: 320, height: 700 } }));

test('tutorials: previous and next links, and steps are not headings', { skip: SKIP }, () => {
  const files = new Map(htmlFiles().map((f) => [pagePath(f), f]));
  const all = [...files.keys()].filter((p) => /^\/windows\/tutorials\/.+\//.test(p));
  const nav = (p) => read(files.get(p)).match(/<nav class="tnext"[^>]*>(.*?)<\/nav>/s)?.[1] ?? '';
  const s = read(files.get(all[5]));
  assert.match(s, /<nav class="tnext" aria-label="Previous and next tutorial"><a class="tn-prev" rel="prev"[^>]*>.*<a class="tn-next" rel="next"/s);
  assert.doesNotMatch(s, /<li class="tstep"><div class="tx"><h3>/);
  // only the first and the last tutorial of the platform lack one of the two links
  const withBoth = all.filter((p) => /rel="prev"/.test(nav(p)) && /rel="next"/.test(nav(p))).length;
  assert.equal(all.length - withBoth, 2);
});

test('header: skip link is the first stop, controls are at least 24 px, the menu button is a clear target', { skip: SKIP }, () => withPage(async (page) => {
  await page.goto(base + '/');
  await page.keyboard.press('Tab');
  assert.equal(await page.evaluate(() => document.activeElement.className), 'skip');
  assert.ok(await page.isVisible('.skip'));
  const small = await page.$$eval('.nav-ctls .seg button', (bs) => bs.map((b) => b.getBoundingClientRect()).filter((r) => r.width < 24 || r.height < 24).length);
  assert.equal(small, 0);
  const sizes = await page.$$eval('.nav-ctls .seg button', (bs) => bs.map((b) => { const r = b.getBoundingClientRect(); return [Math.round(r.width), Math.round(r.height)]; }));
  assert.ok(sizes.every(([w, h]) => w >= 32 && h >= 32), JSON.stringify(sizes));
}));

test('a primary call to action is reachable from phone to wide desktop (1100 to 1239 px has no room: the bar is full)', { skip: SKIP }, () => withPage(async (page) => {
  for (const w of [320, 390, 479, 480, 700, 1099, 1240, 1280, 1320, 1600]) {
    await page.setViewportSize({ width: w, height: 800 });
    await page.goto(base + '/pricing/');
    const bar = await page.evaluate(() => { const a = document.querySelector('.nav-get'); return !!a && a.getClientRects().length > 0; });
    if (bar) continue;
    // otherwise it is inside the drawer
    await page.click('#menutg').catch(() => {});
    assert.ok(await page.isVisible('#menu .menu-get a'), `no Get Straft at ${w}px`);
  }
}));

for (const theme of ['light', 'dark']) {
  test(`availability chips and key text meet contrast (${theme})`, { skip: SKIP }, () => withPage(async (page) => {
    for (const p of ['/features/screenplay/', '/features/scheduling/', '/tutorials/']) {
      await page.goto(base + p);
      await page.evaluate((t) => document.documentElement.setAttribute('data-theme', t), theme);
      const r = await new AxeBuilder({ page }).withRules(['color-contrast']).analyze();
      assert.deepEqual(r.violations.map((v) => v.nodes.map((n) => n.target.join(' ')).slice(0, 3)), [], `${p} ${theme}`);
    }
  }));
}

test('FAQ answers have stable ids', { skip: SKIP }, () => {
  const s = read(htmlFiles().find((f) => pagePath(f) === '/faq/'));
  const ids = [...s.matchAll(/<details class="faq-item" id="(q-[a-z0-9-]+)"/g)].map((m) => m[1]);
  assert.ok(ids.length > 10);
  assert.equal(new Set(ids).size, ids.length, 'ids are unique');
});
