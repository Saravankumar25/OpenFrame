// The home hero film in a real browser: the words end up alone on the screen, reduced motion gets no film and no download,
// Skip works, and every photograph is served by this site and has its source on record.
import test, { before, after } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { chromium } from 'playwright';
import { SKIP, DIST } from './_helpers.mjs';
import { startServer } from '../scripts/serve.mjs';
import { HERO_PHOTOS } from '../src/content/hero-photos.mjs';

let server, browser, base;
before(async () => {
  if (SKIP) return;
  server = await startServer(0);
  base = `http://127.0.0.1:${server.address().port}`;
  browser = await chromium.launch();
});
after(async () => { await browser?.close(); await new Promise((r) => (server ? server.close(r) : r())); });

async function withPage(fn, opts = {}) {
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 }, ...opts });
  const page = await ctx.newPage();
  try { await fn(page); } finally { await ctx.close(); }
}
const gone = (page, timeout = 20000) => page.waitForFunction(() => !document.querySelector('.mos') && !document.querySelector('.mos-skip') && !document.querySelector('.hero-top').classList.contains('mos-intro'), null, { timeout });

test('every hero photograph is on disk and comes from the Unsplash record', { skip: SKIP }, () => {
  assert.equal(new Set(HERO_PHOTOS.map((p) => p.short)).size, HERO_PHOTOS.length);
  for (const p of HERO_PHOTOS) {
    assert.ok(p.by && p.user && /^\d+-[0-9a-f]+$/.test(p.id), p.short);
    assert.ok(fs.existsSync(path.join(DIST, 'hero', `${p.short}-320.webp`)), p.short);
    // the number in an Unsplash image name is its upload time, in seconds on the oldest photographs
    const stamp = Number(p.id.split('-')[0]);
    assert.ok((stamp < 1e12 ? stamp * 1000 : stamp) < Date.UTC(2023, 8, 1), `${p.short} is too recent`);
  }
});

for (const viewport of [{ width: 1280, height: 800 }, { width: 390, height: 844 }, { width: 320, height: 640 }]) {
  test(`reduced motion at ${viewport.width} px: no film, no photographs downloaded, no sideways scroll`, { skip: SKIP }, () => withPage(async (page) => {
    const photos = [];
    page.on('request', (r) => { if (r.url().includes('/hero/')) photos.push(r.url()); });
    await page.goto(base + '/');
    await gone(page, 3000);
    await page.waitForTimeout(300);
    assert.deepEqual(photos, []);
    assert.ok(await page.isVisible('.hero-copy h1'));
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
  }, { viewport, reducedMotion: 'reduce' }));
}

test('the film runs under 8 seconds, shows a dense wall, and leaves nothing behind but the words', { skip: SKIP }, () => withPage(async (page) => {
  await page.goto(base + '/');
  await page.waitForFunction(() => document.querySelector('.hero-top').classList.contains('mos-intro'));
  await page.waitForFunction(() => document.querySelector('.mos-skip') && !document.querySelector('.mos-skip').hidden, null, { timeout: 8000 });
  const start = Date.now();
  await page.waitForFunction(() => document.querySelectorAll('.mos img.show').length >= 20, null, { timeout: 12000 });
  await gone(page);
  const seconds = (Date.now() - start) / 1000;
  // 6.5 seconds, and with the splash of about 1.2 seconds a visitor still sees the words alone before 8
  assert.ok(seconds > 6 && seconds < 7, `${seconds} s`);
  assert.equal(await page.locator('.hero-top img').count(), 0);
  assert.equal(await page.evaluate(() => getComputedStyle(document.querySelector('.hero-copy')).visibility), 'visible');
  assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
}));

test('Skip or a key press shows the words at once, the tiles leave, and a later visit in the session has no film', { skip: SKIP }, () => withPage(async (page) => {
  await page.goto(base + '/');
  await page.waitForFunction(() => document.querySelector('.mos-skip') && !document.querySelector('.mos-skip').hidden, null, { timeout: 8000 });
  await page.click('.mos-skip');
  assert.equal(await page.evaluate(() => document.querySelector('.hero-top').classList.contains('mos-intro')), false);
  // the tiles are off the screen in under a second, so the headline is not read over photographs for long
  await gone(page, 1500);
  await page.goto(base + '/pricing/');
  await page.goto(base + '/');
  await gone(page, 2000);
}));

test('a key press during the film counts as skipping', { skip: SKIP }, () => withPage(async (page) => {
  await page.goto(base + '/');
  await page.waitForFunction(() => document.querySelector('.mos-skip') && !document.querySelector('.mos-skip').hidden, null, { timeout: 8000 });
  await page.keyboard.press('Tab');
  await gone(page, 6000);
}));
