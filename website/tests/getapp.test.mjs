// The Home page pop-up: when it opens, who sees it, no price, its link to the download page, wording, axe, fit at phone width.
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
const scrollDown = (page) => page.evaluate(() => window.scrollTo(0, 2600));
const PHONE = { viewport: { width: 390, height: 800 }, isMobile: true, hasTouch: true };

for (const [name, opts] of [['1280', {}], ['390 phone', PHONE]]) {
  test(`Home at ${name}: opens by itself after the first scroll past the hero, and only once per session`, { skip: SKIP }, () => withPage(async (page) => {
    await go(page, '/');
    await page.waitForTimeout(800);
    assert.equal(await isOpen(page), false, 'not before the scroll');
    await scrollDown(page);
    await page.waitForSelector('dialog.gp-dlg[open]', { timeout: 4000 });
    assert.equal(await page.evaluate(() => document.documentElement.classList.contains('gp-open') && getComputedStyle(document.documentElement).overflow), 'hidden', 'page scroll is locked');
    assert.equal(await page.evaluate(() => sessionStorage.getItem('of_getapp_seen')), '1');
    const box = await page.evaluate(() => { const r = document.querySelector('.gp-box').getBoundingClientRect(); return { l: r.left, r: innerWidth - r.right, t: r.top, b: innerHeight - r.bottom, over: document.documentElement.scrollWidth - document.documentElement.clientWidth }; });
    assert.ok(box.l >= 8 && box.r >= 8 && box.t >= 0 && box.b >= 0 && box.over === 0, JSON.stringify(box));
    await page.keyboard.press('Escape');
    assert.equal(await isOpen(page), false);
    await page.reload();
    await page.waitForFunction(() => document.documentElement.classList.contains('ready'));
    await scrollDown(page);
    await page.waitForTimeout(1200);
    assert.equal(await isOpen(page), false, 'not again in the same session');
  }, opts));
}

test('the pop-up names no price and its one button goes to the download page, which shows the price', { skip: SKIP }, () => withPage(async (page) => {
  await go(page, '/');
  await scrollDown(page);
  await page.waitForSelector('dialog.gp-dlg[open]', { timeout: 4000 });
  const t = await page.evaluate(() => document.querySelector('dialog.gp-dlg').textContent);
  assert.match(t, /on sale/);
  assert.doesNotMatch(t, /[₹€$]|\d,\d|pric|pay once|\bpaid\b/i);
  assert.deepEqual(await page.$$eval('dialog.gp-dlg a', (a) => a.map((x) => x.getAttribute('href'))), ['/download/']);
  assert.equal(await page.locator('dialog.gp-dlg .gp-price, dialog.gp-dlg [data-price]').count(), 0);
  await Promise.all([page.waitForURL('**/download/'), page.click('dialog.gp-dlg a')]);
  assert.ok(await page.isVisible('.dl-price [data-price]'), 'the price is on the download page');
  await page.waitForFunction(() => document.documentElement.classList.contains('ready'));
  await page.goBack();
  assert.equal(await isOpen(page), false, 'Back does not return to an open dialog');
}));

test('the close button and the backdrop close it, and Tab stays inside', { skip: SKIP }, () => withPage(async (page) => {
  await go(page, '/');
  await scrollDown(page);
  await page.waitForSelector('dialog.gp-dlg[open]', { timeout: 4000 });
  assert.equal(await page.evaluate(() => document.activeElement.closest('dialog.gp-dlg') !== null), true, 'focus moves into the dialog');
  for (let i = 0; i < 8; i++) {
    await page.keyboard.press('Tab');
    assert.ok(await page.evaluate(() => document.activeElement === document.body || !!document.activeElement.closest('dialog.gp-dlg')), 'Tab ' + i);
  }
  await page.click('[data-gp-close]');
  assert.equal(await isOpen(page), false, 'the close button closes it');
  await page.evaluate(() => { document.querySelector('dialog.gp-dlg').showModal(); });
  await page.mouse.click(8, 8);
  assert.equal(await isOpen(page), false, 'a click on the backdrop closes it');
}));

test('only the Home page has it, and no page has a floating Get Straft button', { skip: SKIP }, () => withPage(async (page) => {
  for (const p of ['/faq/', '/download/', '/pricing/', '/windows/tutorials/']) {
    await go(page, p);
    await scrollDown(page);
    await page.waitForTimeout(800);
    assert.equal(await page.evaluate(() => !!document.querySelector('.gp-pill, dialog.gp-dlg')), false, p);
  }
  await go(page, '/');
  await page.waitForTimeout(1800);
  assert.equal(await page.locator('.gp-pill').count(), 0);
  assert.equal(await page.locator('.site-h .nav-get').count(), 1);
  assert.equal(await page.evaluate(() => [...document.querySelectorAll('footer.site-f a, footer.site-f button')].some((e) => /get straft/i.test(e.textContent))), false, 'not in the footer');
}));

test('wording: no form, no social links, no dashes', { skip: SKIP }, () => {
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
    await go(page, '/');
    await scrollDown(page);
    await page.waitForSelector('dialog.gp-dlg[open]', { timeout: 4000 });
    const a = await new AxeBuilder({ page }).include('dialog.gp-dlg').analyze();
    assert.deepEqual(a.violations.map((v) => v.id + ': ' + v.nodes.map((n) => n.target).join(' ')), []);
  }, {}, `localStorage.setItem('of_theme','${dark ? 'dark' : 'light'}')`));
}
