// Browser tests on dist/: the three independent controls, platform pages, screen frames, regional price, phone width, axe.
import test, { before, after } from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import AxeBuilder from '@axe-core/playwright';
import { SKIP } from './_helpers.mjs';
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
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  page.on('console', (m) => { if (m.type() === 'error' && !/favicon/.test(m.text())) errors.push(m.text()); });
  try { await fn(page); } finally { await ctx.close(); }
  assert.deepEqual(errors, []);
}
const attr = (page, a) => page.evaluate((x) => document.documentElement.getAttribute(x), a);
const frameSrc = (page) => page.$$eval('img[data-mock]', (is) => is.map((i) => i.getAttribute('src')));

test('website theme and app screen theme are independent and remembered', { skip: SKIP }, () => withPage(async (page) => {
  await page.goto(base + '/');
  await page.click('.nav-ctls [data-set="theme"][data-val="dark"]');
  assert.equal(await attr(page, 'data-theme'), 'dark');
  assert.equal(await attr(page, 'data-mock'), 'light');
  assert.ok((await frameSrc(page)).every((s) => s.includes('/light/')));
  await page.waitForFunction(() => getComputedStyle(document.body).backgroundColor === 'rgb(14, 13, 21)');
  await page.click('.nav-ctls [data-set="mock"][data-val="dark"]');
  assert.equal(await attr(page, 'data-theme'), 'dark');
  assert.ok((await frameSrc(page)).every((s) => s.includes('/dark/')));
  await page.click('.nav-ctls [data-set="theme"][data-val="light"]');
  assert.equal(await attr(page, 'data-mock'), 'dark');
  await page.waitForFunction(() => getComputedStyle(document.body).backgroundColor === 'rgb(255, 255, 255)');
  await page.goto(base + '/pricing/');
  assert.equal(await attr(page, 'data-theme'), 'light');
  assert.equal(await attr(page, 'data-mock'), 'dark');
}));

test('platform control swaps Windows and Mac screens in place and is kept across pages', { skip: SKIP }, () => withPage(async (page) => {
  await page.goto(base + '/');
  await page.click('.nav-ctls [data-set="os"][data-val="win"]');
  assert.ok(await page.isVisible('[data-os-show="win"] .shot.win'));
  assert.ok(!(await page.isVisible('[data-os-show="mac"] .shot.mac')));
  await page.click('.nav-ctls [data-set="os"][data-val="mac"]');
  assert.ok(await page.isVisible('[data-os-show="mac"] .shot.mac'));
  assert.ok(!(await page.isVisible('[data-os-show="win"] .shot.win')));
  assert.equal(await page.getAttribute('a[data-os-href-mac]', 'href'), '/mac/tutorials/');
  await page.goto(base + '/features/call-sheets/');
  assert.equal(await attr(page, 'data-os'), 'mac');
}));

test('on a platform page the platform control goes to the other platform', { skip: SKIP }, () => withPage(async (page) => {
  await page.goto(base + '/windows/tutorials/');
  await Promise.all([page.waitForURL('**/mac/tutorials/'), page.click('.nav-ctls [data-set="os"][data-val="mac"]')]);
  assert.equal(await attr(page, 'data-os'), 'mac');
  assert.ok((await frameSrc(page)).every((s) => s.startsWith('/shots/mac/')));
}));

test('app screens are pictures that fill one frame and keep their shape', { skip: SKIP }, () => withPage(async (page) => {
  await page.goto(base + '/windows/');
  assert.equal(await page.locator('iframe').count(), 0);
  const box = await page.$eval('.tour-real .shot-open', (b) => { const i = b.querySelector('img'), r = b.getBoundingClientRect(); return { w: r.width, h: r.height, ar: i.width / i.height }; });
  assert.ok(Math.abs(box.w / box.h - box.ar) < 0.05, JSON.stringify(box));
}));

test('price follows the region: India shows rupees, elsewhere dollars, and the choice can be changed', { skip: SKIP }, async () => {
  await withPage(async (page) => {
    await page.goto(base + '/pricing/');
    assert.ok(await page.isVisible('.price [data-price-in]'));
    assert.ok(!(await page.isVisible('.price [data-price-us]')));
    await page.click('[data-region-set="world"]');
    assert.ok(await page.isVisible('.price [data-price-us]'));
    await page.reload();
    assert.ok(await page.isVisible('.price [data-price-us]'));
  }, { timezoneId: 'Asia/Kolkata' });
  await withPage(async (page) => {
    await page.goto(base + '/pricing/');
    assert.ok(await page.isVisible('.price [data-price-us]'));
    assert.ok(!(await page.isVisible('.price [data-price-in]')));
    assert.ok(!(await page.isVisible('.price [data-price-eu]')));
  }, { timezoneId: 'America/New_York' });
  await withPage(async (page) => {
    await page.goto(base + '/pricing/');
    assert.ok(await page.isVisible('.price [data-price-eu]'), 'a European time zone shows euros');
    assert.ok(!(await page.isVisible('.price [data-price-us]')));
    await page.click('[data-region-set="world"]');
    assert.ok(await page.isVisible('.price [data-price-us]'));
    assert.ok(!(await page.isVisible('.price [data-price-eu]')));
  }, { timezoneId: 'Europe/Berlin' });
});

test('store buttons are enabled and clicking them goes nowhere yet', { skip: SKIP }, () => withPage(async (page) => {
  await page.goto(base + '/download/');
  for (const t of ['Download for Windows: Microsoft Store', 'Download for Mac: Mac App Store']) {
    const b = page.getByRole('button', { name: t }).first();
    assert.ok(await b.isVisible());
    assert.ok(await b.isEnabled());
    await b.click();
    assert.equal(new URL(page.url()).pathname, '/download/');
  }
}));
for (const p of ['/', '/pricing/', '/features/screenplay/', '/windows/tutorials/', '/about/', '/faq/']) {
  test(`no sideways scroll at phone width: ${p}`, { skip: SKIP }, () => withPage(async (page) => {
    await page.goto(base + p);
    const over = await page.evaluate(() => document.documentElement.scrollWidth - innerWidth);
    assert.ok(over <= 1, `${p} overflows by ${over}px`);
  }, { viewport: { width: 375, height: 800 } }));
}

for (const p of ['/', '/pricing/', '/download/', '/about/', '/faq/']) {
  test(`axe finds no serious problems: ${p}`, { skip: SKIP }, () => withPage(async (page) => {
    await page.goto(base + p);
    const r = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
    const bad = r.violations.filter((v) => ['serious', 'critical'].includes(v.impact)).map((v) => `${v.id}: ${v.nodes.length}`);
    assert.deepEqual(bad, []);
  }));
}

test('the loading icon lifts after the first load and stays away for the rest of the session', { skip: SKIP }, () => withPage(async (page) => {
  await page.goto(base + '/');
  await page.waitForFunction(() => document.documentElement.classList.contains('ready'), null, { timeout: 6000 });
  await page.waitForSelector('#loader', { state: 'hidden', timeout: 4000 });
  await page.goto(base + '/pricing/');
  assert.ok(await page.evaluate(() => document.documentElement.classList.contains('ready')), 'no splash on later pages');
}, { reducedMotion: 'no-preference' }));

test('each app screen shows the icon until its picture has loaded', { skip: SKIP }, () => withPage(async (page) => {
  await page.goto(base + '/windows/');
  await page.waitForSelector('.tour-real .shot-open.loaded', { timeout: 8000 });
  assert.ok(!(await page.isVisible('.tour-real .mock-load')));
}));

test('error page: the code, the icon and Try again on a 500', { skip: SKIP }, () => withPage(async (page) => {
  await page.goto(base + '/500.html');
  assert.equal(await page.textContent('.err-code'), '500');
  assert.ok(await page.isVisible('.err-mark svg'));
  await Promise.all([page.waitForNavigation(), page.click('[data-reload]')]);
  assert.equal(new URL(page.url()).pathname, '/500.html');
}));
