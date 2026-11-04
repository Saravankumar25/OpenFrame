// Static checks over dist/: page basics, links, app screens, store buttons, company, prices, no internal wording.
import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { FOUNDERS } from '../src/content/founders.mjs';
import { DIST, SKIP, htmlFiles, pagePath, read, resolvesInDist } from './_helpers.mjs';

const pages = htmlFiles();
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
  // Exact names, Saravan Kumar M K first, each credited with their team (owner, 2026-11-04).
  const a = about.indexOf('>Saravan Kumar M K</a> and team</h3>'), b = about.indexOf('<h3>Badrinath Reddy A and team</h3>');
  assert.match(about, /<h3><a class="name-link" href="https:\/\/saravankumar25\.github\.io" target="_blank" rel="noopener noreferrer">Saravan Kumar M K<\/a> and team<\/h3>/);
  assert.ok(a > 0 && b > a, 'founder order');
  assert.match(about, /Technical Co-Founding Engineer/);
  assert.match(about, /Saravan Kumar M K led the technical development of Straft Studio with the engineering team/);
  FOUNDERS[0].text.forEach((p, i) => assert.equal(p.length, FOUNDERS[1].text[i].length, `founder paragraph ${i + 1}: same number of characters`));
  assert.doesNotMatch(about.slice(about.indexOf('<div class="founders">'), about.indexOf('</section>', about.indexOf('<div class="founders">'))), /<h4>|<ul class="ticks">/, 'founder profiles are prose');
  assert.match(about, /Product Co-Founding Engineer/);
  assert.match(about, /Badrinath Reddy A led the product development of Straft Studio with the product team/);
  assert.ok(existsSync(path.join(DIST, 'company', 'vighnajith-advisories-logo.webp')));
  for (const s of html.values()) assert.doesNotMatch(s, /Vignajith|Advisorys|Sarvan Kumar|Kumar MK|Reddy Avuku|web[- ]?series/i);
});

test('store buttons show the approved labels and are not links until listings exist', { skip: SKIP }, () => {
  const s = read(path.join(DIST, 'download', 'index.html'));
  assert.match(s, /<button type="button" class="btn btn-primary[^"]*" data-store="win">.*Download for Windows: Microsoft Store<\/button>/);
  assert.match(s, /<button type="button" class="btn btn-mac[^"]*" data-store="mac">.*Download for Mac: Mac App Store<\/button>/);
  assert.doesNotMatch(s, / disabled>/);
  for (const v of html.values()) assert.doesNotMatch(v, /apps\.microsoft\.com|apps\.apple\.com|ms-windows-store:/);
});

test('launch prices: rupees for India, euros for Europe, US dollars for the USA and other countries, originals crossed out', { skip: SKIP }, () => {
  const s = read(path.join(DIST, 'pricing', 'index.html'));
  assert.match(s, /data-price-in><s>₹7,999<\/s> <b>₹3,999<\/b></);
  assert.match(s, /data-price-eu><s>€149<\/s> <b>€89<\/b></);
  assert.match(s, /data-price-us><s>US\$149<\/s> <b>US\$89<\/b></);
  assert.match(s, /class="launch">Launch Price</);
  const card = s.slice(s.indexOf('<div class="pcard win'), s.indexOf('class="price-note'));
  assert.doesNotMatch(card, /<ul|class="ticks"|class="badge"|class="price-sub"/, 'the card holds only the name, the prices and the store buttons');
  assert.match(card, /data-store="win"[\s\S]*data-store="mac"/);
  assert.match(s, /<div class="region-tg"[^>]*>[\s\S]*data-region-set="in"[^>]*>India<[\s\S]*data-region-set="eu"[^>]*>Europe<[\s\S]*data-region-set="world"[^>]*>USA and others</);
  assert.doesNotMatch(s, /<select/);
  for (const v of html.values()) assert.doesNotMatch(v, /89 euros|3,000 rupees|Outside India/);
});

test('sitemap lists every indexable page and no app screen page; /mockups/ is disallowed and may be framed by this site only', { skip: SKIP }, () => {
  const sm = read(path.join(DIST, 'sitemap.xml'));
  for (const [f, s] of html) if (!/noindex/.test(s) && !f.endsWith('404.html')) assert.ok(sm.includes(`${pagePath(f)}</loc>`), pagePath(f));
  assert.doesNotMatch(sm, /mockups/);
  assert.ok(existsSync(path.join(DIST, 'mockups', 'win', 'light')) && existsSync(path.join(DIST, 'mockups', 'mac', 'dark')));
  assert.match(read(path.join(DIST, 'robots.txt')), /^Disallow: \/mockups\/$/m);
  const h = read(path.join(DIST, '_headers'));
  assert.match(h, /\/mockups\/\*\n  ! Content-Security-Policy\n  ! X-Frame-Options/);
  assert.match(h, /frame-ancestors 'self'/);
  assert.match(h, /frame-src 'self'/);
  assert.doesNotMatch(read(path.join(DIST, '_redirects')), /mockups/);
});

test('public pages reveal nothing about how the product is built', { skip: SKIP }, () => {
  const stack = /\b(Rust|SQLite|Tauri|React|TypeScript|SwiftUI|UniFFI|llama\.cpp|Gemma|Vulkan|Metal|Hugging Face|SHA-256|fuzz[\w-]*)\b|captured from commit|\d+ (Windows|Mac) screens/;
  for (const [f, s] of html) {
    const text = s.replace(/<script[\s\S]*?<\/script>/g, '').replace(/<[^>]+>/g, ' ');
    assert.doesNotMatch(text, stack, pagePath(f));
  }
  for (const f of ['llms.txt', 'llms-full.txt']) assert.doesNotMatch(read(path.join(DIST, f)), stack, f);
});
