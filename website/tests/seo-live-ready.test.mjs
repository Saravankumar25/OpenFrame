// What must be true before search engines are told about the site, and the scripts that tell them. Complements tests/seo.test.mjs,
// which already checks titles, canonicals, noindex pages, robots.txt, structured-data rules and the sitemap's file and image entries.
// This file adds: one h1, JSON-LD parses, nothing in the sitemap is redirected or blocked, sitemap limits, and the IndexNow and monitor scripts.
// Tests marked as dist tests run against dist/ (or OF_DIST) and are skipped when it is missing; the others need no build.
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { DIST, ROOT, SKIP, read, htmlFiles, pagePath, resolvesInDist } from './_helpers.mjs';
import { SITE_URL } from '../site.config.mjs';
import { findKey, sitemapEntries, sitemapUrls, changedUrls, payloads } from '../scripts/indexnow.mjs';
import { canonicalOf, h1Count, disallowedPaths, spread, pageProblems } from '../scripts/seo-monitor.mjs';

const pages = SKIP ? [] : htmlFiles().map((f) => ({ path: pagePath(f), s: read(f) })).filter((p) => !/<meta name="robots" content="noindex">/.test(p.s));
const sitemap = SKIP ? '' : read(path.join(DIST, 'sitemap.xml'));
const locs = [...sitemapEntries(sitemap).keys()];

test('dist: every indexable page has exactly one h1 and JSON-LD that parses with a schema.org context', { skip: SKIP }, () => {
  assert.ok(pages.length > 100);
  for (const p of pages) {
    assert.equal((p.s.match(/<h1[\s>]/g) || []).length, 1, `h1 count on ${p.path}`);
    for (const m of p.s.matchAll(/<script type="application\/ld\+json">([\s\S]*?)<\/script>/g)) {
      let o; assert.doesNotThrow(() => { o = JSON.parse(m[1]); }, `JSON-LD on ${p.path}`);
      assert.match(String(o['@context']), /^https:\/\/schema\.org\/?$/, `@context on ${p.path}`);
      assert.ok(o['@type'], `@type on ${p.path}`);
    }
  }
});

test('dist: no sitemap URL is a redirect source, is blocked by robots.txt, carries a query or lacks the trailing slash', { skip: SKIP }, () => {
  const redirected = new Set(read(path.join(DIST, '_redirects')).split('\n').map((l) => l.trim().split(/\s+/)[0]).filter((s) => s.startsWith('/')).flatMap((s) => [s, s.replace(/\/$/, ''), s.replace(/\/?$/, '/')]));
  const blocked = disallowedPaths(read(path.join(DIST, 'robots.txt')));
  for (const l of locs) {
    const u = new URL(l);
    assert.equal(u.search + u.hash, '', `${l} has a query or fragment`);
    assert.ok(u.pathname === '/' || u.pathname.endsWith('/') || /\.[a-z0-9]+$/.test(u.pathname), `${l} lacks the trailing slash the host serves`);
    assert.ok(!redirected.has(u.pathname), `${l} is a redirect source`);
    assert.ok(!blocked.some((d) => u.pathname.startsWith(d)), `${l} is disallowed by robots.txt`);
  }
});

test('dist: sitemap stays inside the protocol limits, lastmod is never in the future, image entries point at real files', { skip: SKIP }, () => {
  assert.ok(locs.length <= 50000 && Buffer.byteLength(sitemap) <= 50 * 1024 * 1024, 'sitemaps.org allows 50,000 URLs and 50 MB per file');
  const today = new Date().toISOString().slice(0, 10);
  for (const [l, d] of sitemapEntries(sitemap)) assert.ok(d && d <= today, `${l} lastmod ${d}`);
  for (const m of sitemap.matchAll(/<image:loc>([^<]+)<\/image:loc>/g)) {
    assert.ok(m[1].startsWith(SITE_URL + '/') && resolvesInDist(new URL(m[1].replace(/&amp;/g, '&')).pathname), `image does not exist: ${m[1]}`);
  }
});

test('dist: _headers does not hide the site from search with an X-Robots-Tag on every path', { skip: SKIP }, () => {
  const star = read(path.join(DIST, '_headers')).match(/^\/\*\n((?: {2}.+\n)+)/m)[1];
  assert.doesNotMatch(star, /x-robots-tag/i);
});

test('IndexNow key file: name equals content, 32 hex characters, no trailing newline, published in the build', () => {
  const key = findKey();
  assert.match(key, /^[0-9a-f]{32}$/);
  assert.equal(fs.readFileSync(path.join(ROOT, 'src', 'static', `${key}.txt`), 'utf8'), key);
  if (!SKIP) assert.equal(read(path.join(DIST, `${key}.txt`)), key);
});

test('IndexNow diff: new and changed pages are sent, unchanged pages are not, removed pages are reported', () => {
  const x = (rows) => `<urlset>${rows.map(([u, d]) => `<url><loc>${u}</loc>${d ? `<lastmod>${d}</lastmod>` : ''}<image:image><image:loc>${u}a.webp</image:loc></image:image></url>`).join('')}</urlset>`;
  const A = `${SITE_URL}/a/`, B = `${SITE_URL}/b/`, C = `${SITE_URL}/c/`, D = `${SITE_URL}/d/?x=1&amp;y=2`;
  const old = x([[A, '2026-11-01'], [B, '2026-11-01'], [C, '2026-11-01']]);
  const now = x([[A, '2026-11-01'], [B, '2026-11-05'], [D, '2026-11-05']]);
  assert.deepEqual(changedUrls(old, now), { changed: [B, `${SITE_URL}/d/?x=1&y=2`], removed: [C] });
  assert.deepEqual(changedUrls(now, now), { changed: [], removed: [] });
  assert.deepEqual(sitemapUrls(now, { onlyLastmod: '2026-11-05' }), [B, `${SITE_URL}/d/?x=1&y=2`]);
  assert.equal(sitemapEntries('<urlset></urlset>').size, 0, 'an empty or wrong snapshot is detected so the script can fall back');
  const [body] = payloads([A, B], SITE_URL, 'k'.repeat(32));
  assert.deepEqual(Object.keys(body), ['host', 'key', 'keyLocation', 'urlList']);
  assert.equal(payloads(['https://elsewhere.example/'], SITE_URL, 'k').length, 0);
});

test('monitor helpers: canonical, noindex, h1, robots groups and page sampling', () => {
  const ok = '<head><link rel="canonical" href="https://x.test/a/"><meta name="robots" content="index, follow"></head><h1>One</h1>';
  assert.equal(canonicalOf(ok), 'https://x.test/a/');
  assert.equal(h1Count(ok + '<h1 class="x">Two</h1>'), 2);
  assert.deepEqual(pageProblems('https://x.test/a/', 200, ok), []);
  assert.deepEqual(pageProblems('https://x.test/a/', 404, ok), ['status 404']);
  assert.ok(pageProblems('https://x.test/b/', 200, ok).some((p) => p.startsWith('canonical')));
  assert.ok(pageProblems('https://x.test/a/', 200, ok.replace('index, follow', 'noindex')).includes('meta robots says noindex'));
  assert.ok(pageProblems('https://x.test/a/', 200, ok, 'noindex').includes('X-Robots-Tag says noindex'));
  assert.deepEqual(disallowedPaths('User-agent: GPTBot\nDisallow: /\n\nUser-agent: *\nAllow: /\nDisallow: /a/ # note\nDisallow:\n\nSitemap: https://x.test/sitemap.xml\n'), ['/a/']);
  assert.deepEqual(disallowedPaths('User-agent: *\nDisallow: /\n'), ['/']);
  const list = Array.from({ length: 1000 }, (_, i) => i);
  const s = spread(list, 30);
  assert.ok(s.length <= 30 && s[0] === 0 && s.at(-1) === 999);
  assert.deepEqual(spread([1, 2], 30), [1, 2]);
});
