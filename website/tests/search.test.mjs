// Search: ranking unit tests (Node, real index data) and browser tests on dist/ (palette, page, filters, fit at every width).
import test, { before, after } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import vm from 'node:vm';
import { chromium } from 'playwright';
import { DIST, ROOT, HAS_DIST } from './_helpers.mjs';
import { loadManual } from '../scripts/check-content.mjs';
import { buildSearchIndex } from '../scripts/search-index.mjs';
import { startServer } from '../scripts/serve.mjs';

// The engine is the first half of src/assets/js/search.js; outside a browser the file exports it.
const mod = { exports: {} };
vm.runInNewContext(fs.readFileSync(path.join(ROOT, 'src', 'assets', 'js', 'search.js'), 'utf8'), { module: mod });
const Core = mod.exports;
const data = buildSearchIndex({ win: await loadManual('win'), mac: await loadManual('mac') });
const ix = Core.prepare(data);
const WIN = { os: 'win', pref: 'win' };
const run = (q, f = WIN) => Core.search(ix, q, f);
const top = (q, n = 5, f = WIN) => run(q, f).hits.slice(0, n).map((h) => ix.E[h.i]);
const allText = (e) => [e.title, e.area, e.keywords, e.summary, ...e.steps].join(' ').toLowerCase();

test('index: tutorials of both platforms, feature pages and answers, no dashes', () => {
  const by = (t, o) => ix.E.filter((e) => e.type === t && (!o || e.os === o)).length;
  assert.ok(by('tutorial', 'win') > 100 && by('tutorial', 'mac') > 100, 'both platforms');
  assert.ok(by('feature') >= 10 && by('answer') >= 10);
  for (const e of ix.E) {
    assert.match(e.url, /^\/(windows|mac)\/tutorials\/[a-z0-9-]+\/$|^\/features\/[a-z0-9-]+\/$|^\/faq\/#q-[a-z0-9-]+$/);
    assert.ok(e.title && e.summary, e.url);
  }
  assert.ok(!/[–—]/.test(JSON.stringify(data)));
});

test('"call sheet" ranks call sheet tutorials first', () => {
  const t = top('call sheet', 5);
  assert.ok(t.every((e) => e.os === 'win'));
  assert.ok(t.filter((e) => e.area === 'Call sheets').length >= 4, t.map((e) => e.title).join(' | '));
  assert.ok(/call sheet/i.test(t[0].title));
});

test('typos: "screnplay" and "scheduel" still find the right pages', () => {
  assert.ok(top('screnplay', 5).every((e) => /screenplay/i.test(allText(e))));
  assert.ok(top('screnplay', 3).some((e) => /screenplay/i.test(e.title)));
  assert.ok(top('scheduel', 5).some((e) => /schedule/i.test(e.title)));
  assert.equal(Core.suggest(ix, 'x', WIN), '', 'nothing to suggest for a word that finds nothing close');
});

test('case, accents and word endings do not matter', () => {
  const a = top('call sheet', 8).map((e) => e.url).join();
  assert.equal(top('CALL   Sheet', 8).map((e) => e.url).join(), a);
  assert.equal(top('cáll shéet', 8).map((e) => e.url).join(), a);
  assert.ok(top('exporting scenes', 5).length > 0);
  assert.ok(top('sched', 5).some((e) => /schedule/i.test(e.title)), 'prefix');
});

test('synonyms', () => {
  assert.ok(top('remove location', 4).some((e) => /delete/i.test(e.title)), 'remove finds delete');
  assert.ok(top('save as', 6).some((e) => /export/i.test(e.title)), 'save as finds export');
  assert.ok(top('callsheet', 5).some((e) => /call sheet/i.test(e.title)), 'callsheet');
  assert.ok(top('stripboard', 5).some((e) => /schedule/i.test(allText(e))), 'stripboard');
  assert.ok(top('dark mode', 5).some((e) => /theme|appearance/i.test(allText(e))), 'dark mode');
  assert.ok(top('screenplay', 5).length && top('script', 5).length);
  assert.ok(run('dood').hits.length > 0, 'day out of days');
});

test('quoted phrases and -word exclusion', () => {
  const p = run('"call sheet"').hits;
  assert.ok(p.length > 0 && p.every((h) => [ix.E[h.i]._f.title, ix.E[h.i]._f.sum, ix.E[h.i]._f.kw, ix.E[h.i]._f.area, ...ix.E[h.i]._f.steps].some((t) => (' ' + t + ' ').includes(' call sheet '))));
  const base = run('export').hits, cut = run('export -pdf').hits;
  assert.ok(cut.length < base.length && cut.length > 0);
  assert.ok(cut.every((h) => !ix.post.get('pdf').has(h.i)));
  assert.ok(run('"call sheet" -revision').hits.every((h) => !ix.post.get('revision')?.has(h.i)));
  assert.equal(run('"zzzz qqqq"').hits.length, 0);
});

test('natural questions: stop words go, unknown words are set aside, nothing is thrown away silently', () => {
  assert.ok(top('how do I delete a project', 5).length > 0);
  const r = run('call sheet qwertyuiop');
  assert.ok(r.hits.length > 0 && r.ignored.join() === 'qwertyuiop');
  assert.equal(run('qwertyuiop').hits.length, 0);
  assert.equal(run('').hits.length, 0);
});

test('did you mean: the closest indexed word, only when it finds something', () => {
  const s = Core.suggest(ix, 'scrnplaay', WIN);
  assert.equal(s, 'screenplay');
  assert.equal(run('scrnplaay').hits.length, 0);
  assert.ok(run(s).hits.length > 0);
});

test('field boosts: a title match beats a step match', () => {
  const t = top('moodboard', 3);
  assert.ok(t.every((e) => /moodboard/i.test(e.title)), t.map((e) => e.title).join(' | '));
  const r = run('schedule');
  const first = ix.E[r.hits[0].i], last = ix.E[r.hits[r.hits.length - 1].i];
  assert.ok(/schedule/i.test(first.title) && !/schedule/i.test(last.title));
});

test('platform: the current one is favoured, the filter removes the other', () => {
  const mac = Core.search(ix, 'call sheet', { os: 'all', pref: 'mac' }).hits.slice(0, 4).map((h) => ix.E[h.i]);
  assert.ok(mac.filter((e) => e.os === 'mac').length >= 2);
  assert.ok(run('call sheet').hits.every((h) => ix.E[h.i].os !== 'mac'));
  assert.ok(Core.search(ix, 'call sheet', { os: 'mac', pref: 'mac' }).hits.every((h) => ix.E[h.i].os !== 'win'));
});

test('type, part and area filters, and browsing by filter alone', () => {
  const area = ix.E.find((e) => e.os === 'win' && e.area === 'Call sheets').area;
  assert.ok(run('create', { ...WIN, type: 'answer' }).hits.every((h) => ix.E[h.i].type === 'answer'));
  assert.ok(run('create', { ...WIN, area }).hits.every((h) => ix.E[h.i].area === area));
  assert.ok(run('create', { ...WIN, part: 'Shoot' }).hits.every((h) => ix.E[h.i].part === 'Shoot'));
  assert.ok(run('', { ...WIN, area }).hits.length > 3, 'empty query with an area lists the area');
  assert.equal(run('', WIN).hits.length, 0);
});

test('groups, highlight and snippet', () => {
  const r = run('call sheet'), g = Core.groups(ix, r.hits, { tutorial: 3 });
  assert.ok(g.length >= 1 && g[0].shown.length <= 3);
  assert.ok(g.every((x, i) => !i || g[i - 1].top >= x.top));
  assert.equal(Core.mark('Create a <b>Call Sheet</b> now', new Set(['call', 'sheet'])), 'Create a &lt;b&gt;<mark>Call</mark> <mark>Sheet</mark>&lt;/b&gt; now');
  const e = ix.E[r.hits[0].i], sn = Core.snippet(e, r.hl);
  assert.ok(sn.includes('<mark>') && !/<(?!\/?mark>)/.test(sn));
});

test('tutorial list filter matches by prefix, typo and synonym, and honours - and quotes', () => {
  const t = 'Make a shooting schedule. Beginner, 6 steps. Delete a day.';
  assert.ok(Core.matchText('', t) && Core.matchText('sched', t) && Core.matchText('shedule', t) && Core.matchText('remove day', t));
  assert.ok(!Core.matchText('budget', t) && !Core.matchText('schedule -delete', t) && Core.matchText('"shooting schedule"', t) && !Core.matchText('"schedule shooting"', t));
});

/* ---------- browser ---------- */
const SKIP = HAS_DIST && fs.existsSync(path.join(DIST, 'search-index.json')) ? false : 'dist/ has no search index: run the build first';
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
// The page splash covers the page for about a second; clicks wait until it has lifted.
async function go(page, p) { await page.goto(base + p); await page.waitForFunction(() => document.documentElement.classList.contains('ready')); }
const opts = (page) => page.$$eval('.srch-dlg [role=option]', (os) => os.map((o) => ({ id: o.id, href: o.getAttribute('href'), text: o.textContent })));
const dialogOpen = (page) => page.evaluate(() => !!document.querySelector('dialog.srch-dlg[open]'));
async function type(page, q) { await page.fill('#srch-q', q); await page.waitForSelector(`#srch-list[data-for="${q}"] [role=option] .srch-s`, { timeout: 5000 }); }

test('Ctrl K and "/" open the palette, the hint follows the platform, the index loads only then', { skip: SKIP }, () => withPage(async (page) => {
  const idx = [];
  page.on('request', (r) => { if (r.url().includes('search-index.json')) idx.push(r.url()); });
  await go(page, '/windows/tutorials/');
  assert.equal(idx.length, 0, 'not loaded before the first open');
  assert.equal(await page.textContent('.srch-btn .srch-k [data-os-show=win]'), 'Ctrl K');
  assert.equal(await page.textContent('.srch-btn .srch-k [data-os-show=mac]'), 'Cmd K');
  await page.keyboard.press('Control+k');
  await page.waitForSelector('dialog.srch-dlg[open]');
  assert.equal(await page.evaluate(() => document.activeElement.id), 'srch-q');
  assert.equal(await page.getAttribute('#srch-q', 'role'), 'combobox');
  await page.keyboard.press('Escape');
  assert.equal(await dialogOpen(page), false);
  await page.waitForFunction(() => !document.querySelector('dialog.srch-dlg').open && document.activeElement && !/^(INPUT|TEXTAREA|SELECT)$/.test(document.activeElement.tagName));
  await page.keyboard.press('/');
  await page.waitForSelector('dialog.srch-dlg[open]');
  await page.fill('#srch-q', 'budget');
  await page.waitForSelector('.srch-dlg [role=option]');
  assert.equal(idx.length, 1);
  await page.keyboard.press('Escape');
  await page.keyboard.press('Meta+k');
  assert.equal(await dialogOpen(page), true);
  await page.mouse.click(6, 6); // the dimmed page behind the palette
  assert.equal(await dialogOpen(page), false, 'a click outside closes it');
  // "/" typed in a field is just a character
  await go(page, '/search/');
  await page.fill('#sp-q', 'a/b');
  assert.equal(await dialogOpen(page), false);
}));

test('palette: "call sheet" finds call sheet tutorials, "screnplay" still finds screenplay ones', { skip: SKIP }, () => withPage(async (page) => {
  await go(page, '/windows/tutorials/');
  await page.click('.srch-btn');
  await type(page, 'call sheet');
  const a = await opts(page);
  assert.ok(a.length >= 3 && /call-sheet/.test(a[0].href), JSON.stringify(a[0]));
  assert.ok(a.every((o) => !o.href.startsWith('/mac/')), 'only the current platform');
  assert.ok((await page.$$eval('.srch-dlg mark', (m) => m.length)) > 0, 'highlights');
  await type(page, 'screnplay');
  const b = await opts(page);
  assert.ok(b.length >= 3 && b.some((o) => /screenplay/i.test(o.text)), JSON.stringify(b.slice(0, 2)));
  await type(page, 'remove');
  assert.ok((await opts(page)).some((o) => /delete/i.test(o.text)), 'synonym');
  assert.ok((await page.$$eval('.srch-dlg [role=group]', (g) => g.map((x) => x.getAttribute('aria-label')))).length >= 1);
}));

test('palette: keyboard (arrows wrap, Home, End), Enter navigates and is remembered, Esc closes and returns focus', { skip: SKIP }, () => withPage(async (page) => {
  await go(page, '/windows/tutorials/');
  await page.click('.srch-btn');
  await type(page, 'schedule');
  const o = await opts(page), act = () => page.getAttribute('#srch-q', 'aria-activedescendant');
  assert.equal(await act(), o[0].id, 'first result is ready for Enter');
  await page.keyboard.press('ArrowDown'); assert.equal(await act(), o[1].id);
  await page.keyboard.press('End'); assert.equal(await act(), o.at(-1).id);
  await page.keyboard.press('ArrowDown'); assert.equal(await act(), o[0].id, 'wraps');
  await page.keyboard.press('ArrowUp'); assert.equal(await act(), o.at(-1).id);
  await page.keyboard.press('Home'); assert.equal(await act(), o[0].id);
  assert.equal(await page.getAttribute(`#${o[0].id}`, 'aria-selected'), 'true');
  assert.equal(await page.getAttribute('#srch-q', 'aria-expanded'), 'true');
  // the live region is cleared and then set 40 ms later, so screen readers hear every update
  await page.waitForFunction(() => document.getElementById('srch-live').textContent === 'Results updated', null, { timeout: 3000 });
  assert.ok(!/\d/.test(await page.textContent('#srch-live')), 'no totals');
  await page.keyboard.press('Escape');
  assert.equal(await dialogOpen(page), false);
  assert.equal(await page.evaluate(() => document.activeElement.className.includes('srch-btn')), true, 'focus returns to the button');
  await page.keyboard.press('Control+k');
  await type(page, 'call sheet');
  await Promise.all([page.waitForURL(/\/windows\/tutorials\/call-sheet/), page.keyboard.press('Enter')]);
  assert.deepEqual(JSON.parse(await page.evaluate(() => localStorage.getItem('of_recent_search'))), ['call sheet']);
  await page.keyboard.press('Control+k');
  assert.ok((await page.textContent('.srch-dlg')).includes('call sheet'), 'recent search listed');
}));

test('palette: nothing found offers "did you mean"; Shift+Enter opens the full page', { skip: SKIP }, () => withPage(async (page) => {
  await go(page, '/mac/tutorials/');
  await page.click('.srch-btn');
  await page.fill('#srch-q', 'scrnplaay');
  await page.waitForSelector('.srch-dym');
  assert.equal(await page.textContent('.srch-dym'), 'screenplay');
  await page.click('.srch-dym');
  await page.waitForSelector('.srch-dlg [role=option]');
  assert.ok((await opts(page)).every((o) => !o.href.startsWith('/windows/')), 'Mac page, Mac results');
  await Promise.all([page.waitForURL(/\/search\/\?q=screenplay/), page.keyboard.press('Shift+Enter')]);
}));

test('/search/: filters write the URL, the back button and deep links restore them', { skip: SKIP }, () => withPage(async (page) => {
  await go(page, '/search/?q=export');
  await page.waitForSelector('.sp-g');
  assert.equal(await page.inputValue('#sp-q'), 'export');
  assert.ok(await page.evaluate(() => document.title.startsWith('export')));
  await page.click('[data-f=os][data-v=mac]');
  assert.match(page.url(), /[?&]os=mac/);
  assert.ok((await page.$$eval('.sp .srch-it', (a) => a.map((x) => x.getAttribute('href')))).every((h) => !h.startsWith('/windows/')));
  await page.click('[data-f=type][data-v=tutorial]');
  assert.match(page.url(), /[?&]type=tutorial/);
  assert.equal(await page.$$eval('.sp-g', (g) => g.length), 1);
  const area = await page.$eval('#sp-area option:nth-child(3)', (o) => o.value);
  await page.selectOption('#sp-area', area);
  assert.equal(new URL(page.url()).searchParams.get('area'), area);
  assert.ok((await page.$$eval('.sp .srch-m', (m) => m.map((x) => x.textContent))).every((t) => t.includes(area)));
  await page.goBack();
  assert.ok(!new URL(page.url()).searchParams.has('area') && new URL(page.url()).searchParams.get('type') === 'tutorial');
  assert.equal(await page.getAttribute('[data-f=type][data-v=tutorial]', 'aria-pressed'), 'true');
  await page.goBack();
  assert.ok(!new URL(page.url()).searchParams.has('type'));
  await page.click('[data-f=os][data-v=all]');
  assert.ok((await page.$$eval('.sp .srch-b', (b) => b.map((x) => x.textContent))).includes('Windows'), 'platform badges when both are shown');
  await page.fill('#sp-q', 'screnplay');
  await page.waitForFunction(() => document.querySelector('.sp-g'));
  await page.waitForFunction(() => location.search.includes('q=screnplay'));
  await go(page, '/search/?q=call+sheet&os=mac&type=tutorial&part=Shoot');
  await page.waitForSelector('.sp-g');
  assert.equal(await page.getAttribute('[data-f=os][data-v=mac]', 'aria-pressed'), 'true');
  assert.equal(await page.inputValue('#sp-part'), 'Shoot');
  assert.ok((await page.$$eval('.sp .srch-it', (a) => a.map((x) => x.getAttribute('href')))).every((h) => h.startsWith('/mac/tutorials/')));
  await page.click('#sp-reset');
  assert.equal(new URL(page.url()).search, '?q=call+sheet');
  await go(page, '/search/?q=zzzzzz');
  await page.waitForSelector('.sp-none');
  await page.waitForFunction(() => document.getElementById('sp-live').textContent === 'Results updated');
}));

test('tutorial list filter box filters the cards in place', { skip: SKIP }, () => withPage(async (page) => {
  await go(page, '/windows/tutorials/');
  const all = await page.$$eval('.tut', (c) => c.length);
  await page.fill('#tf-q', 'shedule');
  const shown = () => page.$$eval('.tut:not(.tf-off)', (c) => c.map((x) => x.textContent.toLowerCase()));
  await page.waitForFunction((n) => document.querySelectorAll('.tut:not(.tf-off)').length < n, all);
  const s = await shown();
  assert.ok(s.length > 0 && s.every((t) => /schedul/.test(t)));
  assert.equal(await page.$$eval('.tgrid.tf-off', (g) => g.length) > 0, true, 'empty groups hide');
  assert.match(await page.getAttribute('[data-tf-adv]', 'href'), /\/search\/\?q=shedule/);
  await page.fill('#tf-q', 'qqqqqqqq');
  await page.waitForFunction(() => !document.querySelector('.tf-none').hidden);
  await page.press('#tf-q', 'Escape');
  await page.waitForFunction((n) => document.querySelectorAll('.tut:not(.tf-off)').length === n, all);
}));

test('header and palette fit at every width: no overflow, no overlap', { skip: SKIP }, async () => {
  for (const w of [390, 640, 800, 1099, 1100, 1180, 1319, 1320, 1440, 1920]) {
    await withPage(async (page) => {
      for (const p of ['/', '/windows/tutorials/', '/search/']) {
        await go(page, p);
        const r = await page.evaluate(() => {
          const vis = (e) => e && getComputedStyle(e).display !== 'none' && e.getBoundingClientRect().width > 0;
          const nav = document.querySelector('.site-h .nav'), nb = nav.getBoundingClientRect();
          const items = ['.brand', '.menu', '.nav-ctls', '.srch-btn', '.nav-get', '.menu-tg'].map((s) => document.querySelector(s)).filter(vis).map((e) => ({ n: e.className.split(' ')[0], ...e.getBoundingClientRect().toJSON() }));
          const overlaps = [];
          items.forEach((a, i) => items.slice(i + 1).forEach((b) => { if (a.left < b.right - 0.5 && b.left < a.right - 0.5) overlaps.push(a.n + '/' + b.n); }));
          const btn = items.find((x) => x.n === 'srch-btn');
          return { page: document.documentElement.scrollWidth - innerWidth, navOver: nav.scrollWidth - nav.clientWidth, overlaps, btn: !!btn, inside: btn ? btn.left >= nb.left && btn.right <= nb.right : false, k: vis(document.querySelector('.srch-k')) };
        });
        assert.ok(r.page <= 0, `${w}px ${p}: page scrolls sideways by ${r.page}`);
        assert.ok(r.navOver <= 0, `${w}px ${p}: nav overflows by ${r.navOver}`);
        assert.deepEqual(r.overlaps, [], `${w}px ${p}`);
        assert.ok(r.btn && r.inside, `${w}px ${p}: search button is in the bar`);
        if (w >= 1320 && p === '/') assert.ok(r.k, 'shortcut hint shown on the full bar');
      }
      await go(page, '/windows/tutorials/');
      await page.click('.srch-btn');
      await type(page, 'call sheet');
      const d = await page.evaluate(() => { const b = document.querySelector('.srch-dlg').getBoundingClientRect(); return { over: document.documentElement.scrollWidth - innerWidth, l: b.left, r: b.right, w: innerWidth, bodyOver: document.querySelector('.srch-body').scrollWidth - document.querySelector('.srch-body').clientWidth }; });
      assert.ok(d.over <= 0 && d.l >= 0 && d.r <= d.w && d.bodyOver <= 0, `${w}px palette: ${JSON.stringify(d)}`);
      await page.keyboard.press('Escape');
      if (w <= 1099) {
        await page.click('#menutg');
        const o = await page.evaluate(() => ({ over: document.documentElement.scrollWidth - innerWidth, menu: !!document.querySelector('#menu.open-m'), s: document.querySelector('.srch-btn').getBoundingClientRect().right <= innerWidth }));
        assert.ok(o.menu && o.over <= 0 && o.s, `${w}px drawer ${JSON.stringify(o)}`);
        await page.click('.srch-btn');
        assert.equal(await dialogOpen(page), true, 'opening search closes the drawer first');
        assert.equal(await page.evaluate(() => !!document.querySelector('#menu.open-m')), false);
      }
    }, { viewport: { width: w, height: 800 } });
  }
});

test('the palette and the page pass axe', { skip: SKIP }, () => withPage(async (page) => {
  const { default: AxeBuilder } = await import('@axe-core/playwright');
  await go(page, '/windows/tutorials/');
  await page.click('.srch-btn');
  await type(page, 'call sheet');
  const a = await new AxeBuilder({ page }).include('dialog.srch-dlg').analyze();
  assert.deepEqual(a.violations.map((v) => v.id + ': ' + v.nodes.map((n) => n.target).join(' ')), []);
  await page.keyboard.press('Escape');
  await go(page, '/search/?q=export');
  await page.waitForSelector('.sp-g');
  const b = await new AxeBuilder({ page }).include('#sp').analyze();
  assert.deepEqual(b.violations.map((v) => v.id + ': ' + v.nodes.map((n) => n.target).join(' ')), []);
}));
