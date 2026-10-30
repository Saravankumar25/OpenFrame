// Static checks over dist/: page basics, links, app screens, store buttons, company, prices, no internal wording.
import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readdirSync } from 'node:fs';
import path from 'node:path';
import { DIST, SKIP, htmlFiles, pagePath, read, resolvesInDist } from './_helpers.mjs';

const pages = htmlFiles().filter((f) => !f.includes(`${path.sep}mockups${path.sep}`));
const html = new Map(pages.map((f) => [f, read(f)]));
const COMPANY = 'Vighnajith Advisories Private Limited';

test('every page has a title, description, canonical, one h1 and the boot script', { skip: SKIP }, () => {
  for (const [f, s] of html) {
    const p = pagePath(f);
    assert.match(s, /<title>[^<]{5,}<\/title>/, p);
    assert.match(s, /<meta name="description" content="[^"]{60,}"/, p);
    assert.match(s, /<link rel="canonical" href="https:\/\/[^"]+"/, p);
    assert.equal((s.match(/<h1[\s>]/g) || []).length, 1, `h1 count on ${p}`);
    assert.match(s, /<script id="boot">/, p);
  }
});

test('no em or en dashes and no internal release wording in page text', { skip: SKIP }, () => {
  const bad = /team testing|unsigned|team password|internal build|not ready yet|public release is planned|will deploy|under development|early-bird|launch sale|\/var\/folders|C:\Users/i;
  for (const [f, s] of html) {
    const text = s.replace(/<script[\s\S]*?<\/script>/g, '').replace(/<[^>]+>/g, ' ');
    assert.doesNotMatch(text, /[\u2013\u2014]/, pagePath(f));
    assert.doesNotMatch(text, bad, pagePath(f));
  }
});

test('every internal link, image and app screen resolves', { skip: SKIP }, () => {
  const missing = [];
  for (const [f, s] of html) {
    for (const m of s.matchAll(/(?:href|src)="(\/[^"#?]*)/g)) if (!resolvesInDist(m[1])) missing.push(`${pagePath(f)} -> ${m[1]}`);
  }
  assert.deepEqual(missing.slice(0, 20), []);
});

test('company name in every footer, founders on About, supplied logo present', { skip: SKIP }, () => {
  for (const [f, s] of html) assert.ok(s.includes(`&copy; 2026 ${COMPANY}`), pagePath(f));
  const about = read(path.join(DIST, 'about', 'index.html'));
  assert.match(about, /Badrinath Reddy A<\/h3><p>Responsible for the product idea/);
  assert.match(about, /Saravan Kumar MK<\/h3><p>Responsible for the end-to-end technical implementation/);
  assert.ok(existsSync(path.join(DIST, 'company', 'vighnajith-advisories-logo.webp')));
  for (const s of html.values()) assert.doesNotMatch(s, /Vignajith|Advisorys|Sarvan Kumar/);
});

test('store buttons show the approved labels and are not links until listings exist', { skip: SKIP }, () => {
  const s = read(path.join(DIST, 'download', 'index.html'));
  assert.match(s, /<button type="button" class="btn btn-primary[^"]*" data-store="win">.*Download for Windows: Microsoft Store<\/button>/);
  assert.match(s, /<button type="button" class="btn btn-mac[^"]*" data-store="mac">.*Download for Mac: Mac App Store<\/button>/);
  assert.doesNotMatch(s, / disabled>/);
  for (const v of html.values()) assert.doesNotMatch(v, /apps\.microsoft\.com|apps\.apple\.com|ms-windows-store:/);
});

test('prices: rupees for India, US dollars elsewhere, no euro price', { skip: SKIP }, () => {
  const s = read(path.join(DIST, 'pricing', 'index.html'));
  assert.match(s, /data-price-in>₹2,999</);
  assert.match(s, /data-price-us>US\$89</);
  assert.match(s, /<select id="region" data-region>/);
  for (const v of html.values()) assert.doesNotMatch(v, /€|89 euros|3,000 rupees/);
});

test('sitemap lists every indexable page and no mockup; headers let only this site frame the screens', { skip: SKIP }, () => {
  const sm = read(path.join(DIST, 'sitemap.xml'));
  for (const [f, s] of html) if (!/noindex/.test(s) && !f.endsWith('404.html')) assert.ok(sm.includes(`${pagePath(f)}</loc>`), pagePath(f));
  assert.doesNotMatch(sm, /mockups/);
  const h = read(path.join(DIST, '_headers'));
  assert.match(h, /\/mockups\/\*\n  ! Content-Security-Policy\n  ! X-Frame-Options/);
  assert.match(h, /frame-ancestors 'self'/);
});

test('published app screens show no test-machine paths or the capture account name', { skip: SKIP }, () => {
  const bad = /zz-capture|tests\e2e\\.run|\/var\/folders|openframe-e2e\.|Saravan Kumar|E2E Tester/;
  const hits = [];
  for (const plat of ['win', 'mac']) for (const t of ['light', 'dark']) {
    const dir = path.join(DIST, 'mockups', plat, t);
    for (const f of readdirSync(dir).filter((x) => x.endsWith('.html'))) if (bad.test(read(path.join(dir, f)))) hits.push(`${plat}/${t}/${f}`);
  }
  assert.deepEqual(hits.slice(0, 10), []);
});
