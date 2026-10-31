// App screens as pictures: no live frames left, every picture the pages name is in dist, a missing picture stops
// the build with the screen name, and the screen viewer (lightbox) works with mouse, keyboard and the theme control.
import test, { before, after } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { chromium } from 'playwright';
import { DIST, ROOT, SKIP, htmlFiles, read, pagePath } from './_helpers.mjs';
import { startServer } from '../scripts/serve.mjs';

test('no live screen frames, no "Open this screen" link and no /mockups/ link on any page', { skip: SKIP }, () => {
  for (const f of htmlFiles()) {
    const s = read(f);
    assert.doesNotMatch(s, /<iframe/i, pagePath(f));
    assert.doesNotMatch(s, /Open this screen/i, pagePath(f));
    assert.doesNotMatch(s, /\/mockups\//, pagePath(f));
  }
  assert.ok(!existsSync(path.join(DIST, 'mockups')));
});

test('every screen picture a page names exists in dist, and each has size, alt text and a caption-only figcaption', { skip: SKIP }, () => {
  const urls = new Set();
  let figures = 0;
  for (const f of htmlFiles()) {
    const s = read(f);
    for (const m of s.matchAll(/(?:src|href)="(\/shots\/[^"]+)"/g)) urls.add(m[1]);
    for (const m of s.matchAll(/srcset="([^"]*\/shots\/[^"]+)"/g)) for (const part of m[1].split(',')) urls.add(part.trim().split(/\s+/)[0]);
    for (const m of s.matchAll(/<figure class="shot [^"]*">([\s\S]*?)<\/figure>/g)) {
      figures++;
      assert.match(m[1], /<img [^>]*width="\d+" height="\d+" alt="[^"]+"/, pagePath(f));
      assert.match(m[1], /srcset="[^"]*720w, [^"]*1280w"/, pagePath(f));
    }
  }
  assert.ok(figures > 50, `only ${figures} screen figures found`);
  const missing = [...urls].filter((u) => !existsSync(path.join(DIST, u)));
  assert.deepEqual(missing.slice(0, 10), []);
});

test('the build stops and names every screen that has no picture in the manifest', () => {
  const tmp = mkdtempSync(path.join(tmpdir(), 'of-shots-'));
  try {
    mkdirSync(path.join(tmp, 'shots'));
    writeFileSync(path.join(tmp, 'shots', 'manifest.json'), JSON.stringify({ win: {}, mac: {} }));
    let err = '';
    try {
      execFileSync(process.execPath, ['scripts/build.mjs'], { cwd: ROOT, env: { ...process.env, OF_DIST: path.join(tmp, 'dist'), OF_SHOTS: path.join(tmp, 'shots'), OF_ALLOW_MISSING: '' }, stdio: 'pipe' });
    } catch (e) { err = String(e.stderr); }
    assert.match(err, /screens have no picture/);
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
const lbSrc = (page) => page.$eval('dialog.lb .lb-img', (i) => i.getAttribute('src'));

test('a click opens the screen in the viewer; Esc closes it and focus returns to the screen', { skip: SKIP }, () => withPage(async (page) => {
  await page.goto(base + FEAT);
  const t = trigger(page);
  const href = await t.getAttribute('href');
  await t.click();
  await page.waitForSelector('dialog.lb[open]');
  await page.waitForFunction((h) => document.querySelector('dialog.lb .lb-img')?.src.endsWith(h), href);
  assert.ok(await page.isVisible('dialog.lb .lb-cap'));
  assert.doesNotMatch(await page.textContent('dialog.lb'), /\d+\s*(of|\/)\s*\d+/);
  assert.equal(await page.$eval('html', (h) => getComputedStyle(h).overflow), 'hidden');
  assert.equal(await page.getAttribute('dialog.lb .lb-close', 'aria-label'), 'Close');
  await page.keyboard.press('Escape');
  await page.waitForSelector('dialog.lb', { state: 'hidden' });
  assert.equal(await page.evaluate(() => document.activeElement?.getAttribute('href')), href);
  assert.notEqual(await page.$eval('html', (h) => getComputedStyle(h).overflow), 'hidden');
}));

test('Enter on a focused screen opens the viewer; Left and Right move between screens; a click outside closes', { skip: SKIP }, () => withPage(async (page) => {
  await page.goto(base + FEAT);
  await trigger(page).focus();
  await page.keyboard.press('Enter');
  await page.waitForSelector('dialog.lb[open]');
  const first = await lbSrc(page);
  await page.waitForFunction(() => !document.querySelector('dialog.lb').classList.contains('is-loading'));
  assert.ok(await page.isDisabled('dialog.lb .lb-prev'), 'nothing before the first screen');
  await page.keyboard.press('ArrowRight');
  await page.waitForFunction(() => !document.querySelector('dialog.lb').classList.contains('is-loading'));
  const second = await lbSrc(page);
  assert.ok(second && second !== first, 'the next screen is shown');
  await page.keyboard.press('ArrowLeft');
  await page.waitForFunction((f) => document.querySelector('dialog.lb .lb-img').getAttribute('src') === f, first);
  await page.mouse.click(640, 790);
  await page.waitForSelector('dialog.lb', { state: 'hidden' });
}));

test('clicking the picture toggles actual size and the close button works', { skip: SKIP }, () => withPage(async (page) => {
  await page.goto(base + FEAT);
  await trigger(page).click();
  await page.waitForFunction(() => document.querySelector('dialog.lb').classList.contains('is-loading') === false);
  const fit = await page.$eval('dialog.lb .lb-img', (i) => i.getBoundingClientRect().width);
  await page.click('dialog.lb .lb-zoom');
  await page.waitForSelector('dialog.lb.is-actual');
  const nat = await page.$eval('dialog.lb .lb-img', (i) => ({ w: i.getBoundingClientRect().width, n: i.naturalWidth }));
  assert.ok(nat.w > fit && Math.abs(nat.w - nat.n) < 2, `actual ${nat.w} vs natural ${nat.n} (fit ${fit})`);
  assert.equal(await page.getAttribute('dialog.lb .lb-zoom', 'aria-label'), 'Fit to window');
  await page.mouse.move(640, 400);
  await page.mouse.wheel(0, 300);
  await page.click('dialog.lb .lb-zoom');
  await page.waitForFunction(() => !document.querySelector('dialog.lb').classList.contains('is-actual'));
  await page.click('dialog.lb .lb-close');
  await page.waitForSelector('dialog.lb', { state: 'hidden' });
}));

test('the screen theme control swaps src, srcset and the link, and the viewer opens the dark picture', { skip: SKIP }, () => withPage(async (page) => {
  await page.goto(base + FEAT);
  const t = trigger(page);
  const img = t.locator('img');
  assert.match(await img.getAttribute('src'), /\/shots\/(win|mac)\/light\//);
  await page.click('.nav-ctls [data-set="mock"][data-val="dark"]');
  assert.match(await img.getAttribute('src'), /\/shots\/(win|mac)\/dark\/.*card-720/);
  assert.match(await img.getAttribute('srcset'), /\/dark\/.*720w, .*\/dark\/.*1280w/);
  assert.match(await t.getAttribute('href'), /\/dark\/.*\.full\.webp$/);
  await t.click();
  await page.waitForSelector('dialog.lb[open]');
  await page.waitForFunction(() => /\/dark\//.test(document.querySelector('dialog.lb .lb-img').getAttribute('src') || ''));
  await page.keyboard.press('Escape');
  await page.click('.nav-ctls [data-set="mock"][data-val="light"]');
  assert.match(await img.getAttribute('src'), /\/light\//);
}));

test('a picture with link:false is not clickable and a "More screens" link opens the viewer', { skip: SKIP }, () => withPage(async (page) => {
  await page.goto(base + '/');
  assert.equal(await page.locator('.shot-static a').count(), 0);
  await page.goto(base + FEAT);
  const link = page.locator('a.shot-link:visible').first();
  const text = (await link.textContent()).trim();
  await link.click();
  await page.waitForSelector('dialog.lb[open]');
  assert.equal((await page.textContent('dialog.lb .lb-cap')).trim(), text);
}));

test('the pictures keep their shape and fill the frame at phone, tablet, laptop and wide widths', { skip: SKIP }, async () => {
  for (const width of [390, 768, 1280, 1920]) {
    await withPage(async (page) => {
      await page.goto(base + FEAT);
      const m = await page.locator('figure.shot:visible').first().evaluate((fig) => {
        const f = fig.querySelector('.shot-open').getBoundingClientRect(), i = fig.querySelector('img').getBoundingClientRect();
        return { fw: f.width, iw: i.width, ih: i.height, over: document.documentElement.scrollWidth - innerWidth };
      });
      assert.ok(m.over <= 1, `sideways scroll at ${width}`);
      assert.ok(Math.abs(m.fw - m.iw) <= 2.5, `picture fills the frame at ${width}: ${m.fw} vs ${m.iw}`);
      assert.ok(m.ih > 100);
    }, { viewport: { width, height: 900 } });
  }
});
