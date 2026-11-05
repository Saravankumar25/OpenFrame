// The standard files of a product site: error pages, icon set, web manifest, security.txt, humans.txt, headers.
import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readdirSync } from 'node:fs';
import path from 'node:path';
import { DIST, SKIP, read } from './_helpers.mjs';

const file = (...p) => path.join(DIST, ...p);

test('error pages 403, 404, 500 and 503 exist, are noindex, name their code and show the icon', { skip: SKIP }, () => {
  for (const code of [403, 404, 500, 503]) {
    const s = read(file(`${code}.html`));
    assert.match(s, /<meta name="robots" content="noindex">/, code);
    assert.match(s, new RegExp(`aria-label="Error ${code}">${code}</p>`), code);
    assert.match(s, /class="err-mark"[^>]*><svg class="mk"/, `${code} shows the Straft icon`);
    assert.match(s, /href="\/"/, code);
    assert.ok(!read(file('sitemap.xml')).includes(`/${code}.html`), `${code} not in the sitemap`);
  }
  assert.match(read(file('500.html')), /data-reload/);
  assert.match(read(file('_headers')), /\/503\.html\n  X-Robots-Tag: noindex\n  Retry-After: 300/);
});

test('every page carries the loading icon, and no page uses a generic spinner', { skip: SKIP }, () => {
  for (const f of ['index.html', '404.html', 'pricing/index.html']) assert.match(read(file(f)), /<div class="loader" id="loader" aria-hidden="true"><div class="loader-in"><svg class="mk"/, f);
  const css = read(file('assets', readdirSync(file('assets')).find((n) => /^site..*.css$/.test(n))));
  assert.doesNotMatch(css, /@keyframes (spin|rotate|spinner)/i);
});

test('icon set, manifest, security.txt and humans.txt are published and consistent', { skip: SKIP }, () => {
  for (const f of ['favicon.svg', 'favicon.ico', 'favicon-16x16.png', 'favicon-32x32.png', 'apple-touch-icon.png', 'icon-192.png', 'icon-512.png', 'icon-maskable-512.png', 'og.png']) assert.ok(existsSync(file(f)), f);
  const m = JSON.parse(read(file('site.webmanifest')));
  assert.equal(m.name, 'Straft Studio');
  for (const i of m.icons) assert.ok(existsSync(file(i.src.slice(1))), i.src);
  assert.ok(m.icons.some((i) => i.purpose === 'maskable'));
  const sec = read(file('.well-known', 'security.txt'));
  assert.match(sec, /^Contact: https:\/\/.+\nExpires: \d{4}-\d\d-\d\dT[\d:]+Z\n/);
  assert.match(sec, /Canonical: https:\/\/.+\/\.well-known\/security\.txt/);
  const h = read(file('humans.txt'));
  assert.ok(h.indexOf('Saravan Kumar M K') < h.indexOf('Badrinath Reddy A'));
  const home = read(file('index.html'));
  for (const l of ['rel="manifest" href="/site.webmanifest"', 'rel="apple-touch-icon" href="/apple-touch-icon.png"', 'href="/favicon.ico"', 'property="og:image:width"', 'media="(prefers-color-scheme: dark)"']) assert.ok(home.includes(l), l);
});

test('_headers: the extensionless error pages are noindex, images cache for a week, and the rules stay under the host limits', { skip: SKIP }, () => {
  const h = read(file('_headers'));
  for (const p of ['/403', '/404', '/500', '/503']) assert.match(h, new RegExp(`^${p}\n  X-Robots-Tag: noindex$`, 'm'), p);
  for (const p of ['/hero/*', '/company/*']) assert.match(h, new RegExp(`^${p.replace('*', '\\*')}\n  Cache-Control: public, max-age=604800$`, 'm'), p);
  assert.ok(h.split('\n').filter((l) => /^\S/.test(l)).length <= 100, 'Cloudflare Pages allows 100 header rules');
  assert.ok(h.split('\n').every((l) => l.length <= 2000), 'a header line is over 2,000 characters');
});

test('/discord is a 302 redirect to the invite, with no page of its own and no sitemap entry; the old invite is gone', { skip: SKIP }, () => {
  const rules = read(file('_redirects')).trim().split('\n').map((l) => l.trim().split(/\s+/));
  for (const from of ['/discord', '/discord/']) assert.deepEqual(rules.find((r) => r[0] === from), [from, 'https://discord.gg/AuQRHDpzP4', '302'], from);
  assert.equal(rules.filter((r) => r[0].startsWith('/discord')).length, 2, 'no other /discord rule');
  assert.ok(!existsSync(file('discord')) && !existsSync(file('discord.html')), 'no HTML page for /discord');
  assert.ok(!/\/discord/.test(read(file('sitemap.xml'))), 'not in the sitemap');
  assert.ok(!/wPKGPf5mm/.test(read(file('index.html')) + read(file('_redirects'))), 'old invite is gone');
});

test('the eight official profiles show in the footer and on Contact, and Organization sameAs lists the seven public ones', { skip: SKIP }, () => {
  const want = ['https://www.youtube.com/@StraftStudio', 'https://www.facebook.com/straftstudio/', 'https://www.instagram.com/straftstudio/', 'https://x.com/straftstudio/', 'https://www.threads.com/@straftstudio/', 'https://www.linkedin.com/company/straftstudio/', 'https://www.reddit.com/user/straftstudio/', 'https://straftstudio.com/discord'];
  for (const f of ['index.html', 'contact/index.html', 'about/index.html']) {
    const s = read(file(f));
    for (const u of want) assert.ok(s.includes(`<a href="${u}" target="_blank" rel="noopener noreferrer" aria-label="Straft Studio on `), `${f} ${u}`);
  }
  const ld = [...read(file('about', 'index.html')).matchAll(/<script type="application\/ld\+json">([\s\S]*?)<\/script>/g)].map((m) => JSON.parse(m[1]).flat?.() ?? JSON.parse(m[1])).flat();
  const org = ld.find((o) => o['@type'] === 'Organization');
  assert.deepEqual(org.sameAs, want.slice(0, 7), 'Discord is an invite, not a profile');
});
