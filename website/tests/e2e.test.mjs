import test, { before, after } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readdirSync } from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium } from 'playwright';
import AxeBuilder from '@axe-core/playwright';
import { ROOT, SKIP } from './_helpers.mjs';
import { startServer } from '../scripts/serve.mjs';

async function tutorialIds(plat) {
  const dir = path.join(ROOT, 'src/content/tutorials', plat);
  if (!existsSync(dir)) return [];
  const out = [];
  for (const f of readdirSync(dir).filter((x) => x.endsWith('.mjs')).sort()) {
    const t = (await import(pathToFileURL(path.join(dir, f)).href)).default;
    if (t.available !== false) out.push({ id: t.id, path: t.path, order: t.order, feature: t.feature });
  }
  return out;
}
const winTut = SKIP ? [] : await tutorialIds('win');
const macTut = SKIP ? [] : await tutorialIds('mac');
const firstWin = winTut.slice().sort((a, b) => a.order - b.order)[0];
const winOnly = winTut.find((t) => !macTut.some((m) => m.id === t.id));

let server, browser, base;
before(async () => {
  if (SKIP) return;
  server = await startServer(0);
  base = `http://127.0.0.1:${server.address().port}`;
  browser = await chromium.launch();
});
after(async () => {
  await browser?.close();
  await new Promise((r) => (server ? server.close(r) : r()));
});

async function withPage(fn, opts = {}) {
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 }, ...opts });
  const page = await ctx.newPage();
  const errors = [];
  page.on('console', (m) => { if (m.type() === 'error') errors.push(`console: ${m.text()}`); });
  page.on('pageerror', (e) => errors.push(`pageerror: ${e.message}`));
  try { return await fn(page, errors); } finally { await ctx.close(); }
}

// Platform switch in the header: a button, tab, radio, menu item or link whose text is exactly the platform name.
const osSwitch = (page, name) =>
  page.locator('header').locator('button, [role=radio], [role=tab], [role=menuitem], [role=menuitemradio], a')
    .filter({ hasText: new RegExp(`^\\s*${name}\\s*$`, 'i') }).first();
const dataOs = (page) => page.evaluate(() => document.documentElement.dataset.os);

test('home loads with no console errors', { skip: SKIP }, () => withPage(async (page, errors) => {
  const res = await page.goto(base + '/', { waitUntil: 'networkidle' });
  assert.equal(res.status(), 200);
  assert.equal(await page.locator('h1').count(), 1);
  assert.deepEqual(errors, []);
}));

test('header platform switch sets data-os, persists in of_os across navigation', { skip: SKIP }, () => withPage(async (page) => {
  await page.goto(base + '/', { waitUntil: 'networkidle' });
  await osSwitch(page, 'macOS').click();
  assert.equal(await dataOs(page), 'mac');
  assert.equal(await page.evaluate(() => localStorage.getItem('of_os')), 'mac');
  await page.goto(base + '/faq/', { waitUntil: 'networkidle' });
  assert.equal(await dataOs(page), 'mac', 'mac not kept after navigation');
  await osSwitch(page, 'Windows').click();
  assert.equal(await dataOs(page), 'win');
  assert.equal(await page.evaluate(() => localStorage.getItem('of_os')), 'win');
  await page.goto(base + '/pricing/', { waitUntil: 'networkidle' });
  assert.equal(await dataOs(page), 'win', 'win not kept after navigation');
}));

test('tutorial page: macOS switch goes to the same tutorial on Mac', { skip: SKIP || !firstWin }, () => withPage(async (page) => {
  const id = firstWin.id;
  await page.goto(`${base}/windows/tutorials/${id}/`, { waitUntil: 'networkidle' });
  await osSwitch(page, 'macOS').click();
  const hasMac = macTut.some((t) => t.id === id);
  if (hasMac) {
    await page.waitForURL(`**/mac/tutorials/${id}/`);
  } else {
    await page.waitForURL(new RegExp(`/mac/tutorials/\\?missing=${id}`));
    const notice = page.locator('[role=status], [role=alert], .note, .notice, [data-missing]').filter({ hasText: /./ }).first();
    assert.ok(await notice.isVisible(), 'missing-tutorial notice is not visible');
  }
}));

test('tutorial missing on Mac shows a notice', { skip: SKIP || !winOnly }, () => withPage(async (page) => {
  await page.goto(`${base}/windows/tutorials/${winOnly.id}/`, { waitUntil: 'networkidle' });
  await osSwitch(page, 'macOS').click();
  await page.waitForURL(new RegExp(`/mac/tutorials/\\?missing=${winOnly.id}`));
  const notice = page.locator('[role=status], [role=alert], .note, .notice, [data-missing]').filter({ hasText: /./ }).first();
  assert.ok(await notice.isVisible(), 'missing-tutorial notice is not visible');
}));

test('tutorial player: next, prev, play/pause, fullscreen, keyboard', { skip: SKIP || !firstWin }, () => withPage(async (page) => {
  await page.goto(`${base}/windows/tutorials/${firstWin.id}/`, { waitUntil: 'networkidle' });
  const current = () => page.locator('ol [aria-current=step]').first().innerText();
  const first = await current();
  const btn = (re) => page.getByRole('button', { name: re }).first();

  await btn(/^next/i).click();
  const second = await current();
  assert.notEqual(second, first, 'next did not change the active step');
  await btn(/^(prev|previous|back)/i).click();
  assert.equal(await current(), first, 'prev did not return to the first step');

  const pp = btn(/^(play|pause)/i);
  const before = (await pp.getAttribute('aria-label')) ?? (await pp.innerText()) + (await pp.getAttribute('aria-pressed'));
  await pp.click();
  const after = (await pp.getAttribute('aria-label')) ?? (await pp.innerText()) + (await pp.getAttribute('aria-pressed'));
  assert.notEqual(after, before, 'play/pause did not toggle');
  if (/pause/i.test(after) || after.includes('true')) await pp.click(); // leave paused so autoplay cannot move steps

  await page.locator('[data-player]').first().focus();
  const beforeKey = await current();
  await page.keyboard.press('ArrowRight');
  assert.notEqual(await current(), beforeKey, 'ArrowRight did not advance');

  await btn(/full ?screen/i).click();
  await page.waitForTimeout(300);
  const fs = await page.evaluate(() => !!document.fullscreenElement || !!document.querySelector('[class*="fullscreen"], [class*="is-fs"]'));
  assert.ok(fs, 'no fullscreen state after clicking the fullscreen button');
}));

const widths = [[375, 812], [768, 1024], [1280, 800]];
test('no horizontal scroll at 375, 768, 1280', { skip: SKIP }, async () => {
  const paths = ['/', '/features/', '/pricing/', '/faq/', '/windows/tutorials/'];
  const feat = await page0Feature();
  if (feat) paths.push(feat);
  if (firstWin) paths.push(`/windows/tutorials/${firstWin.id}/`);
  const errs = [];
  for (const [w, h] of widths) {
    await withPage(async (page) => {
      for (const p of paths) {
        await page.goto(base + p, { waitUntil: 'networkidle' });
        const m = await page.evaluate(() => ({ sw: document.documentElement.scrollWidth, iw: window.innerWidth }));
        if (m.sw > m.iw + 1) errs.push(`${p} at ${w}: scrollWidth ${m.sw} > ${m.iw}`);
      }
    }, { viewport: { width: w, height: h } });
  }
  assert.deepEqual(errs, []);
});

async function page0Feature() {
  return withPage(async (page) => {
    await page.goto(base + '/features/', { waitUntil: 'networkidle' });
    const hrefs = await page.locator('main a[href^="/features/"]').evaluateAll((as) => as.map((a) => a.getAttribute('href')));
    return hrefs.find((h) => h !== '/features/') || null;
  });
}

test('back and forward navigation works', { skip: SKIP }, () => withPage(async (page) => {
  await page.goto(base + '/', { waitUntil: 'networkidle' });
  await page.goto(base + '/faq/', { waitUntil: 'networkidle' });
  await page.goto(base + '/pricing/', { waitUntil: 'networkidle' });
  await page.goBack();
  assert.match(page.url(), /\/faq\/$/);
  await page.goBack();
  assert.match(page.url(), /\/$/);
  assert.equal(await page.locator('h1').count(), 1);
  await page.goForward();
  assert.match(page.url(), /\/faq\/$/);
}));

test('unknown URL returns 404 with a link home', { skip: SKIP }, () => withPage(async (page) => {
  const res = await page.goto(base + '/no/such/page/');
  assert.equal(res.status(), 404);
  const home = page.locator('main a[href="/"]').first();
  assert.ok(await home.count(), 'no link to / inside main on the 404 page');
}));

test('axe: no serious or critical violations', { skip: SKIP }, () => withPage(async (page) => {
  const paths = ['/', '/faq/'];
  if (firstWin) paths.splice(1, 0, `/windows/tutorials/${firstWin.id}/`);
  const errs = [];
  for (const p of paths) {
    await page.goto(base + p, { waitUntil: 'networkidle' });
    const r = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa']).analyze();
    for (const v of r.violations.filter((x) => ['serious', 'critical'].includes(x.impact))) {
      errs.push(`${p}: ${v.id} (${v.impact}) ${v.nodes.length} nodes, e.g. ${v.nodes[0]?.target?.join(' ')}`);
    }
  }
  assert.deepEqual(errs, []);
}));
