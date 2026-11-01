// App screens as live, interactive pages: cards are scaled frames of /mockups/ pages, the viewer opens the real page,
// a screen without a page or crop stops the build with its name, and the pages are kept out of search.
import test, { before, after } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { chromium } from 'playwright';
import { DIST, ROOT, SKIP, htmlFiles, read, pagePath } from './_helpers.mjs';
import { startServer } from '../scripts/serve.mjs';
import { makeHelpers } from '../src/lib/layout.mjs';

test('cards are inert lazy frames of /mockups/ pages: no pictures, no "Open this screen", every page exists', { skip: SKIP }, () => {
  let cards = 0;
  for (const f of htmlFiles()) {
    const s = read(f);
    assert.doesNotMatch(s, /Open this screen/i, pagePath(f));
    assert.doesNotMatch(s, /\/shots\//, pagePath(f));
    for (const m of s.matchAll(/<span class="shot-view[^"]*" data-page="(\d+ \d+)" data-rect="(\d+ \d+ \d+ \d+)"><iframe ([^>]*)>/g)) {
      cards++;
      const a = m[3], src = a.match(/src="(\/mockups\/(win|mac)\/light\/[^"]+\.html)"/)?.[1];
      assert.ok(src, `${pagePath(f)}: card without a mockup page`);
      assert.ok(existsSync(path.join(DIST, src)) && existsSync(path.join(DIST, src.replace('/light/', '/dark/'))), `${src} (light and dark)`);
      assert.match(a, /tabindex="-1"/); assert.match(a, /aria-hidden="true"/);
    }
  }
  assert.ok(cards > 500, `only ${cards} cards found`);
});

test('no feature page lists "More screens" or leaves a Mac screen as a placeholder (owner decision 2026-11-01)', { skip: SKIP }, () => {
  for (const f of htmlFiles()) {
    const s = read(f);
    assert.doesNotMatch(s, /More screens:/, pagePath(f));
    assert.doesNotMatch(s, /shows no Mac screen/, pagePath(f));
  }
});

test('every card shows the whole window (owner decision 2026-11-01): no crops, Windows at its captured size', { skip: SKIP }, () => {
  for (const f of htmlFiles()) for (const m of read(f).matchAll(/data-page="(\d+) (\d+)" data-rect="(\d+) (\d+) (\d+) (\d+)"><iframe [^>]*src="\/mockups\/(win|mac)\//g)) {
    const [, w, h, x, y, cw, ch, plat] = m;
    assert.deepEqual([x, y, cw, ch], ['0', '0', w, h], `${pagePath(f)}: a cropped card`);
    if (plat === 'win') assert.deepEqual([w, h], ['1600', '1000'], `${pagePath(f)}: a resized Windows screen`);
  }
});

test('the first card of a page may load at once, every other frame is lazy', { skip: SKIP }, () => {
  for (const f of htmlFiles()) {
    const frames = [...read(f).matchAll(/<iframe [^>]*data-mock[^>]*>/g)].map((m) => m[0]);
    assert.ok(frames.filter((x) => !/loading="lazy"/.test(x)).length <= 2, `${pagePath(f)}: too many eager frames`);
  }
});

test('/mockups/ is out of search and the sitemap, and may be framed by this site only', { skip: SKIP }, () => {
  assert.doesNotMatch(read(path.join(DIST, 'sitemap.xml')), /mockups/);
  assert.match(read(path.join(DIST, 'robots.txt')), /^Disallow: \/mockups\/$/m);
  const h = read(path.join(DIST, '_headers'));
  const block = h.match(/\n\/mockups\/\*\n((?:  .*\n)+)/)?.[1] ?? '';
  assert.match(block, /! X-Frame-Options/); assert.match(block, /X-Robots-Tag: noindex/);
  assert.match(block, /frame-ancestors 'self'/); assert.match(block, /style-src 'self' 'unsafe-inline'/);
  assert.match(h.split('\n/mockups/*')[0], /frame-src 'self'/);
  assert.doesNotMatch(read(path.join(DIST, '_redirects')), /mockups/);
});

test('a screen missing from the manifest or without a page or crop is named, not skipped', () => {
  const cat = { win: new Map([['a', { file: '001-a.html', title: 'A' }], ['b', { file: '002-b.html', title: 'B' }], ['c', { file: '003-c.html', title: 'C' }]]), mac: new Map() };
  const ok = { page: [1600, 1000], card: [0, 0, 800, 500], full: [0, 0, 1600, 1000] };
  const shots = { win: { a: ok, b: { card: [1280, 768], full: [2400, 1440] } }, mac: {} }; // b has the old picture fields, c is absent
  const noShot = new Set(), missing = new Set();
  const h = makeHelpers(cat, missing, shots, noShot, (p, file) => file !== '001-a.html');
  h.mock('win', 'a'); h.mock('win', 'b'); h.mock('win', 'c'); h.mock('win', 'zz');
  assert.deepEqual([...noShot].sort(), ['win/a (no page 001-a.html)', 'win/b', 'win/c']);
  assert.deepEqual([...missing], ['win/zz']);
  const good = makeHelpers(cat, new Set(), { win: { a: ok }, mac: {} }, new Set(), () => true).mock('win', 'a', { caption: 'Cap' }).toString();
  assert.match(good, /<figure class="shot win"><a class="shot-open" href="\/mockups\/win\/light\/001-a\.html" data-mock data-page="1600 1000" data-full="0 0 1600 1000">/);
  assert.match(good, /viewBox="0 0 800 500"/); assert.match(good, /data-rect="0 0 800 500"/); assert.match(good, /<figcaption>Cap<\/figcaption>/);
  assert.doesNotMatch(makeHelpers(cat, new Set(), { win: { a: ok }, mac: {} }, new Set(), () => true).mock('win', 'a', { link: false }).toString(), /<a /);
});

test('the build stops and names every screen that has no page or crop in the manifest', () => {
  const tmp = mkdtempSync(path.join(tmpdir(), 'of-shots-'));
  try {
    mkdirSync(path.join(tmp, 'shots'));
    writeFileSync(path.join(tmp, 'shots', 'manifest.json'), JSON.stringify({ win: {}, mac: {} }));
    let err = '';
    try {
      execFileSync(process.execPath, ['scripts/build.mjs'], { cwd: ROOT, env: { ...process.env, OF_DIST: path.join(tmp, 'dist'), OF_SHOTS: path.join(tmp, 'shots'), OF_ALLOW_MISSING: '' }, stdio: 'pipe' });
    } catch (e) { err = String(e.stderr); }
    assert.match(err, /screens have no page or crop/);
    assert.match(err, /win\/[a-z0-9-]+/);
    assert.match(err, /mac\/[a-z0-9-]+/);
  } finally { rmSync(tmp, { recursive: true, force: true }); }
});

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
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  page.on('console', (m) => { if (m.type() === 'error' && !/favicon|Failed to load resource/.test(m.text())) errors.push(m.text()); });
  try { await fn(page); } finally { await ctx.close(); }
  assert.deepEqual(errors, []);
}
const FEAT = '/features/screenplay/';
const trigger = (page) => page.locator('a.shot-open:visible').first();
const ready = (page) => page.waitForFunction(() => document.querySelector('dialog.lb .lb-view') && !document.querySelector('dialog.lb .lb-view').classList.contains('is-loading'));
const lbSrc = (page) => page.$eval('dialog.lb .lb-frame', (i) => i.getAttribute('src'));
const useMac = (page) => page.click('.nav-ctls [data-set="os"][data-val="mac"]');

test('a card is a scaled frame: its crop rectangle fills the card exactly and the frame is inert', { skip: SKIP }, () => withPage(async (page) => {
  await page.goto(base + FEAT);
  const m = await page.locator('.shot-open:visible .shot-view').first().evaluate((v) => {
    const f = v.firstElementChild, fb = f.getBoundingClientRect(), vb = v.getBoundingClientRect();
    const [pw, ph] = v.dataset.page.split(' ').map(Number), [x, y, w] = v.dataset.rect.split(' ').map(Number);
    const k = vb.width / w;
    return { dx: fb.left - vb.left + x * k, dy: fb.top - vb.top + y * k, fw: fb.width - pw * k, fh: fb.height - ph * k, pe: getComputedStyle(f).pointerEvents, tab: f.tabIndex };
  });
  for (const k of ['dx', 'dy', 'fw', 'fh']) assert.ok(Math.abs(m[k]) < 1.5, `${k} off by ${m[k]}`);
  assert.equal(m.pe, 'none'); assert.equal(m.tab, -1);
}));

test('a click opens the real page in the viewer; Esc closes it and focus returns to the card', { skip: SKIP }, () => withPage(async (page) => {
  await page.goto(base + FEAT);
  const t = trigger(page);
  const href = await t.getAttribute('href');
  await t.click();
  await page.waitForSelector('dialog.lb[open]');
  await ready(page);
  assert.ok((await lbSrc(page)).endsWith(href));
  assert.ok((await page.getAttribute('dialog.lb .lb-new', 'href')).endsWith(href));
  assert.equal(await page.getAttribute('dialog.lb .lb-new', 'target'), '_blank');
  assert.ok(await page.isVisible('dialog.lb .lb-cap'));
  assert.doesNotMatch(await page.textContent('dialog.lb'), /\d+\s*(of|\/)\s*\d+/);
  assert.equal(await page.$eval('html', (h) => getComputedStyle(h).overflow), 'hidden');
  assert.equal(await page.$eval('dialog.lb .lb-frame', (f) => getComputedStyle(f).pointerEvents), 'auto');
  assert.ok((await page.frameLocator('dialog.lb .lb-frame').locator('body *').count()) > 3, 'the page itself is there');
  await page.keyboard.press('Escape');
  await page.waitForSelector('dialog.lb', { state: 'hidden' });
  assert.equal(await page.evaluate(() => document.activeElement?.getAttribute('href')), href);
  assert.notEqual(await page.$eval('html', (h) => getComputedStyle(h).overflow), 'hidden');
}));

test('the viewer fits the window with the screen\'s own shape and is interactive (hover works on the Mac page)', { skip: SKIP }, () => withPage(async (page) => {
  await page.goto(base + FEAT);
  await useMac(page);
  const [pw, ph] = (await trigger(page).getAttribute('data-page')).split(' ').map(Number);
  await trigger(page).click();
  await ready(page);
  const g = await page.evaluate(() => {
    const v = document.querySelector('dialog.lb .lb-view').getBoundingClientRect(), s = document.querySelector('dialog.lb .lb-stage').getBoundingClientRect();
    return { ar: v.width / v.height, inside: v.left >= s.left - 1 && v.right <= s.right + 1 && v.top >= s.top - 1 && v.bottom <= s.bottom + 1 };
  });
  assert.ok(g.inside && Math.abs(g.ar - pw / ph) < 0.02, JSON.stringify({ ...g, page: [pw, ph] })); // the whole window, its own shape
  // the Mac page is live: its text layer is real elements that take the pointer
  const frame = page.frameLocator('dialog.lb .lb-frame');
  assert.ok(await frame.locator('.el').count() > 10, 'the Mac page has its text layer');
  await frame.locator('.of-clean').hover();
  // keys pressed while the page has the focus still reach the viewer
  await frame.locator('.of-clean').click({ position: { x: 5, y: 5 } });
  await page.keyboard.press('Escape');
  await page.waitForSelector('dialog.lb', { state: 'hidden' });
}));

test('Enter opens a card, Left and Right move between screens, a click outside closes', { skip: SKIP }, () => withPage(async (page) => {
  await page.goto(base + FEAT);
  await trigger(page).focus();
  await page.keyboard.press('Enter');
  await page.waitForSelector('dialog.lb[open]');
  await ready(page);
  const first = await lbSrc(page);
  assert.ok(await page.isDisabled('dialog.lb .lb-prev'), 'nothing before the first screen');
  await page.keyboard.press('ArrowRight');
  await page.waitForFunction((f) => document.querySelector('dialog.lb .lb-frame').getAttribute('src') !== f, first);
  await ready(page);
  await page.keyboard.press('ArrowLeft');
  await page.waitForFunction((f) => document.querySelector('dialog.lb .lb-frame').getAttribute('src') === f, first);
  await page.mouse.click(640, 790);
  await page.waitForSelector('dialog.lb', { state: 'hidden' });
}));

test('the screen theme control swaps every frame and link, and the viewer opens the dark page', { skip: SKIP }, () => withPage(async (page) => {
  await page.goto(base + FEAT);
  const t = trigger(page);
  const f = t.locator('iframe');
  assert.match(await f.getAttribute('src'), /\/mockups\/(win|mac)\/light\//);
  await page.click('.nav-ctls [data-set="mock"][data-val="dark"]');
  assert.match(await f.getAttribute('src'), /\/mockups\/(win|mac)\/dark\//);
  assert.match(await t.getAttribute('href'), /\/dark\/.*\.html$/);
  await t.click();
  await page.waitForSelector('dialog.lb[open]');
  await ready(page);
  assert.match(await lbSrc(page), /\/dark\//);
  await page.keyboard.press('Escape');
  await page.click('.nav-ctls [data-set="mock"][data-val="light"]');
  assert.match(await f.getAttribute('src'), /\/light\//);
}));

test('a card shows the icon until its page has loaded; link:false is not clickable; a screen link opens the viewer', { skip: SKIP }, () => withPage(async (page) => {
  await page.goto(base + '/windows/');
  await page.waitForSelector('.tour-real .shot-open.loaded', { timeout: 8000 });
  assert.ok(!(await page.isVisible('.tour-real .mock-load')));
  await page.goto(base + '/');
  assert.equal(await page.locator('.shot-static a').count(), 0);
  await page.goto(base + FEAT);
  const link = page.locator('a.shot-link:visible').first();
  const text = (await link.textContent()).trim();
  await link.click();
  await page.waitForSelector('dialog.lb[open]');
  assert.equal((await page.textContent('dialog.lb .lb-cap')).trim(), text);
}));

test('a long page of screens loads its frames as they come near, and scrolling stays smooth', { skip: SKIP }, () => withPage(async (page) => {
  await page.goto(base + '/features/screenplay/');
  await page.waitForTimeout(1500);
  const total = await page.locator('.shot-view iframe').count();
  const loadedAtStart = await page.evaluate(() => [...document.querySelectorAll('.shot-open.loaded')].length);
  assert.ok(total > 8, `only ${total} frames on the page`);
  assert.ok(loadedAtStart < total, `all ${total} frames loaded at once`);
  const t = await page.evaluate(async () => {
    const h = document.documentElement.scrollHeight - innerHeight; let worst = 0, last = performance.now(), n = 0;
    await new Promise((res) => { const step = () => { const now = performance.now(); worst = Math.max(worst, now - last); last = now; n++; scrollTo(0, Math.min(h, scrollY + 60)); scrollY >= h - 1 ? res() : requestAnimationFrame(step); }; requestAnimationFrame(step); });
    return { worst: Math.round(worst), n };
  });
  console.log(`# ${total} frames, ${loadedAtStart} loaded at start, scroll: ${t.n} frames, worst frame ${t.worst} ms`);
  assert.ok(t.worst < 800, `a frame took ${t.worst} ms`);
}));

test('cards keep their shape and fill the frame at phone, tablet, laptop and wide widths', { skip: SKIP }, async () => {
  for (const width of [390, 768, 1280, 1920]) {
    await withPage(async (page) => {
      await page.goto(base + FEAT);
      const m = await page.locator('figure.shot:visible').first().evaluate((fig) => {
        const f = fig.querySelector('.shot-open').getBoundingClientRect(), v = fig.querySelector('.shot-view').getBoundingClientRect(), r = fig.querySelector('.shot-ratio').getBoundingClientRect();
        return { fw: f.width, vw: v.width, fh: f.height, vh: v.height, rh: r.height, over: document.documentElement.scrollWidth - innerWidth };
      });
      assert.ok(m.over <= 1, `sideways scroll at ${width}`);
      assert.ok(Math.abs(m.fw - m.vw) <= 2.5 && Math.abs(m.fh - m.vh) <= 2.5, `the page fills the frame at ${width}: ${JSON.stringify(m)}`);
      assert.ok(m.vh > 80);
    }, { viewport: { width, height: 900 } });
  }
});
