// SEO, structured data, sitemap, llms files, security headers, deploy limits and the IndexNow key. Run against OF_DIST or dist/.
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import crypto from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { DIST, ROOT, SKIP, read, walk, htmlFiles, pagePath, resolvesInDist } from './_helpers.mjs';
import { SITE_URL } from '../site.config.mjs';
import { BOOT } from '../src/lib/layout.mjs';
import { bodyImages, sitemapXml, trimWords, fitTitle } from '../src/lib/seo.mjs';
import { inspectDist, LIMITS } from '../scripts/dist-guard.mjs';
import { findKey, payloads, sitemapUrls } from '../scripts/indexnow.mjs';

const un = (s) => s.replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&quot;/g, '"').replace(/&#39;/g, "'").replace(/&amp;/g, '&');
const pages = SKIP ? [] : htmlFiles().map((f) => { const s = read(f); return { f, path: pagePath(f), s, noindex: /<meta name="robots" content="noindex">/.test(s) }; });
const meta = (s, k, attr = 'property') => s.match(new RegExp(`<meta ${attr}="${k}" content="([^"]*)"`))?.[1];
const ldOf = (s) => [...s.matchAll(/<script type="application\/ld\+json">([\s\S]*?)<\/script>/g)].map((m) => JSON.parse(m[1]));
const deep = pages.filter((p) => !p.noindex && p.path !== '/');
const sitemap = SKIP ? '' : read(path.join(DIST, 'sitemap.xml'));

test('every page has its own title (60 characters or fewer) and description (70 to 165), and a canonical URL', { skip: SKIP }, () => {
  const titles = new Map(), descs = new Map();
  for (const p of pages) {
    const t = un(p.s.match(/<title>([^<]*)<\/title>/)?.[1] ?? ''), d = un(meta(p.s, 'description', 'name') ?? '');
    assert.ok(t.length >= 15 && t.length <= 60, `title ${t.length} chars on ${p.path}: ${t}`);
    assert.ok(d.length >= 70 && d.length <= 165, `description ${d.length} chars on ${p.path}`);
    assert.doesNotMatch(t + d, /[–—]/, p.path);
    assert.equal(p.s.match(/<link rel="canonical" href="([^"]+)"/)?.[1], SITE_URL + p.path, `canonical on ${p.path}`);
    (titles.get(t) || titles.set(t, []).get(t)).push(p.path);
    (descs.get(d) || descs.set(d, []).get(d)).push(p.path);
  }
  assert.deepEqual([...titles].filter(([, v]) => v.length > 1), [], 'duplicate titles');
  assert.deepEqual([...descs].filter(([, v]) => v.length > 1).map(([, v]) => v), [], 'duplicate descriptions');
});

test('Open Graph and Twitter/X cards: large image with size and alt text, and the picture really has that size', { skip: SKIP }, () => {
  for (const p of pages) {
    const url = SITE_URL + p.path;
    assert.equal(meta(p.s, 'og:url'), url, p.path);
    assert.equal(meta(p.s, 'og:type'), 'website');
    for (const k of ['og:title', 'og:description', 'og:site_name', 'og:image:alt']) assert.ok(meta(p.s, k)?.length > 3, `${k} on ${p.path}`);
    assert.equal(meta(p.s, 'twitter:card', 'name'), 'summary_large_image', p.path);
    for (const k of ['twitter:title', 'twitter:description', 'twitter:image', 'twitter:image:alt']) assert.ok(meta(p.s, k, 'name')?.length > 3, `${k} on ${p.path}`);
    const img = meta(p.s, 'og:image');
    assert.equal(meta(p.s, 'twitter:image', 'name'), img);
    assert.match(img, /^https:\/\//, p.path);
    const w = Number(meta(p.s, 'og:image:width')), h = Number(meta(p.s, 'og:image:height'));
    assert.ok(w >= 600 && h >= 315, p.path);
  }
  const home = pages.find((p) => p.path === '/').s;
  const png = fs.readFileSync(path.join(DIST, 'og.png'));
  assert.equal(png.readUInt32BE(16), Number(meta(home, 'og:image:width')));
  assert.equal(png.readUInt32BE(20), Number(meta(home, 'og:image:height')));
  assert.ok(png.length < 600 * 1024, 'og.png stays under 600 KB so every platform accepts it');
});

test('robots: error pages are noindex and out of the sitemap, indexable pages are in it once, and every sitemap URL exists', { skip: SKIP }, () => {
  const locs = [...sitemap.matchAll(/<loc>([^<]+)<\/loc>/g)].map((m) => m[1]);
  assert.equal(new Set(locs).size, locs.length, 'duplicate sitemap URLs');
  for (const p of pages) {
    const inMap = locs.includes(SITE_URL + p.path);
    if (p.noindex) assert.ok(!inMap, `${p.path} is noindex but listed`);
    else assert.ok(inMap, `${p.path} missing from the sitemap`);
    if (!p.noindex) assert.match(p.s, /<meta name="robots" content="index, follow, max-image-preview:large/, p.path);
  }
  for (const l of locs) assert.ok(l.startsWith(SITE_URL + '/') && resolvesInDist(new URL(l).pathname), l);
  for (const m of sitemap.matchAll(/<lastmod>([^<]+)<\/lastmod>/g)) assert.match(m[1], /^\d{4}-\d\d-\d\d$/);
  const robots = read(path.join(DIST, 'robots.txt'));
  assert.match(robots, /^User-agent: \*\nAllow: \/\n/);
  assert.match(robots, new RegExp(`Sitemap: ${SITE_URL}/sitemap\\.xml`));
  assert.doesNotMatch(robots, /Disallow: \/\s*$/m, 'the whole site must not be blocked');
  if (fs.existsSync(path.join(DIST, 'search', 'index.html'))) assert.ok(pages.find((p) => p.path === '/search/').noindex, '/search/ is a results shell: noindex');
});

test('sitemap carries an image entry for every local picture shown in a page body', { skip: SKIP }, () => {
  const body = (s) => s.match(/<main id="main">([\s\S]*?)<\/main>/)?.[1] ?? '';
  const entry = (loc) => sitemap.match(new RegExp(`<url><loc>${loc.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}</loc>.*?</url>`))?.[0] ?? '';
  let checked = 0;
  for (const p of pages.filter((x) => !x.noindex)) {
    for (const img of bodyImages(body(p.s))) { assert.ok(entry(SITE_URL + p.path).includes(`<image:loc>${img}</image:loc>`), `${p.path} -> ${img}`); checked++; }
  }
  // the extraction itself, independent of what the pages hold today
  assert.deepEqual(bodyImages('<img src="/shots/win/light/a.webp" alt=""><img data-src="/x/b.png"><img src="data:image/svg+xml,%3Csvg"><img src="https://other.example/c.png"><img src="/shots/win/light/a.webp">'),
    [`${SITE_URL}/shots/win/light/a.webp`, `${SITE_URL}/x/b.png`]);
  const xml = sitemapXml([{ loc: SITE_URL + '/a/', lastmod: '2026-10-31', images: [`${SITE_URL}/shots/a.webp`] }]);
  assert.match(xml, /xmlns:image="http:\/\/www\.google\.com\/schemas\/sitemap-image\/1\.1"/);
  assert.match(xml, /<image:image><image:loc>[^<]+a\.webp<\/image:loc><\/image:image>/);
  console.log(`# ${checked} page pictures checked against the sitemap`);
});

test('structured data: allowed types only, no HowTo, FAQPage on /faq/ alone, breadcrumbs on every deep page', { skip: SKIP }, () => {
  const allowed = new Set(['Organization', 'WebSite', 'SoftwareApplication', 'BreadcrumbList', 'FAQPage']);
  for (const p of pages) {
    const ld = ldOf(p.s);
    for (const o of ld) {
      assert.ok(allowed.has(o['@type']), `${o['@type']} on ${p.path}`);
      assert.doesNotMatch(JSON.stringify(o), /Vighnajith|Rust|SQLite|Tauri|SwiftUI|llama|Gemma|[–—]/, `${p.path} names the company, the technology or uses a dash`);
      if (o['@type'] === 'FAQPage') assert.equal(p.path, '/faq/', 'FAQPage only on /faq/');
    }
    if (p.path === '/faq/') assert.ok(ld.some((o) => o['@type'] === 'FAQPage'));
    if (p.noindex) continue;
    const crumbs = ld.filter((o) => o['@type'] === 'BreadcrumbList');
    if (p.path === '/') assert.equal(crumbs.length, 0, 'no breadcrumb on the home page');
    else {
      assert.equal(crumbs.length, 1, `BreadcrumbList on ${p.path}`);
      const items = crumbs[0].itemListElement;
      assert.equal(items[0].item, SITE_URL + '/');
      assert.equal(items.at(-1).item, SITE_URL + p.path, `last crumb is the page itself (${p.path})`);
      items.forEach((it, i) => { assert.equal(it.position, i + 1); assert.ok(resolvesInDist(new URL(it.item).pathname), `${it.item} from ${p.path}`); });
    }
  }
  assert.ok(deep.length > 20);
});

test('structured data matches the page: FAQ answers, prices, founders and the site search', { skip: SKIP }, () => {
  const faq = pages.find((p) => p.path === '/faq/');
  const f = ldOf(faq.s).find((o) => o['@type'] === 'FAQPage');
  const text = un(faq.s.replace(/<script[\s\S]*?<\/script>/g, '').replace(/<[^>]+>/g, ' ')).replace(/\s+/g, ' ');
  assert.ok(f.mainEntity.length >= 10);
  for (const q of f.mainEntity) assert.ok(text.includes(q.name), `question not on the page: ${q.name}`);
  const home = ldOf(pages.find((p) => p.path === '/').s);
  const org = home.find((o) => o['@type'] === 'Organization');
  assert.equal(org.name, 'OpenFrame Studio');
  assert.equal(org.founder, undefined, 'founders are named on /about/ only');
  const site = home.find((o) => o['@type'] === 'WebSite');
  assert.equal(site.potentialAction['@type'], 'SearchAction');
  assert.equal(site.potentialAction.target.urlTemplate, `${SITE_URL}/search/?q={search_term_string}`);
  assert.ok(fs.existsSync(path.join(DIST, 'search', 'index.html')), 'the SearchAction target page must exist');
  const about = ldOf(pages.find((p) => p.path === '/about/').s).find((o) => o['@type'] === 'Organization');
  assert.deepEqual(about.founder.map((x) => x.name), ['Saravan Kumar M K', 'Badrinath Reddy A']);
  for (const p of pages) for (const app of ldOf(p.s).filter((o) => o['@type'] === 'SoftwareApplication')) {
    assert.ok(app.operatingSystem && app.applicationCategory, p.path);
    const shown = /₹2,999/.test(p.s) && /US\$89/.test(p.s);
    assert.equal(!!app.offers, shown, `offers only where the price is shown (${p.path})`);
    if (!app.offers) continue;
    const [inr, usd] = app.offers;
    assert.equal(inr.price + inr.priceCurrency, '2999INR'); assert.equal(inr.eligibleRegion.name, 'IN');
    assert.equal(usd.price + usd.priceCurrency, '89USD'); assert.equal(usd.ineligibleRegion.name, 'IN');
    for (const o of app.offers) assert.equal(o.priceValidUntil, undefined, 'the price has no end date, so none is stated');
  }
  assert.ok(ldOf(pages.find((p) => p.path === '/pricing/').s).some((o) => o.offers), 'pricing declares the offers');
});

test('title and description helpers cut at word boundaries', () => {
  assert.equal(trimWords('Set a scene time and page count on the Mac today', 30), 'Set a scene time and page');
  assert.equal(trimWords('short', 30), 'short');
  assert.equal(fitTitle('Build a call sheet on Windows'), 'Build a call sheet on Windows | OpenFrame Studio');
  assert.equal(fitTitle('Open Notes and Tasks and start from an empty page on Windows'), 'Open Notes and Tasks and start from an empty page on Windows');
  assert.equal(fitTitle('Pricing | OpenFrame Studio'), 'Pricing | OpenFrame Studio');
});

test('llms.txt and llms-full.txt are accurate, plain, and free of company and technology details', { skip: SKIP }, () => {
  const stack = /\b(Rust|SQLite|Tauri|React|TypeScript|SwiftUI|UniFFI|llama\.cpp|Gemma|Vulkan|Hugging Face|SHA-256|Vighnajith|Pvt|Private Limited)\b|[–—]/;
  const short = read(path.join(DIST, 'llms.txt')), full = read(path.join(DIST, 'llms-full.txt'));
  for (const s of [short, full]) { assert.doesNotMatch(s, stack); assert.match(s, /₹2,999/); assert.match(s, /US\$89/); assert.match(s, /7-day free trial/); }
  assert.match(short, /^# OpenFrame Studio\n\n> /);
  assert.doesNotMatch(short, /€|euro/i);
  for (const m of (short + full).matchAll(/\]\((https?:\/\/[^)]+)\)/g)) {
    assert.ok(m[1].startsWith(SITE_URL), m[1]);
    assert.ok(resolvesInDist(new URL(m[1]).pathname), `llms link does not exist: ${m[1]}`);
  }
  for (const p of ['/features/', '/pricing/', '/faq/', '/windows/tutorials/', '/mac/tutorials/']) assert.ok(short.includes(SITE_URL + p), p);
});

test('security headers: HSTS without preload, strict CSP with the boot hash and Cloudflare Web Analytics hosts, no framing', { skip: SKIP }, () => {
  const h = read(path.join(DIST, '_headers'));
  const star = h.match(/^\/\*\n((?: {2}.+\n)+)/m)[1];
  const get = (n) => star.match(new RegExp(`^ {2}${n}: (.+)$`, 'm'))?.[1];
  assert.equal(get('Strict-Transport-Security'), 'max-age=31536000');
  const csp = get('Content-Security-Policy');
  const dir = (n) => csp.split('; ').find((d) => d.startsWith(n + ' ')) ?? '';
  const boot = crypto.createHash('sha256').update(BOOT).digest('base64');
  assert.ok(dir('script-src').includes(`'sha256-${boot}'`), 'boot script hash');
  assert.equal(dir('script-src'), `script-src 'self' 'sha256-${boot}' https://static.cloudflareinsights.com`);
  assert.equal(dir('connect-src'), "connect-src 'self' https://cloudflareinsights.com");
  for (const d of ["default-src 'self'", "style-src 'self'", "frame-src 'none'", "frame-ancestors 'none'", "object-src 'none'", "base-uri 'self'", "form-action 'none'", 'upgrade-insecure-requests']) assert.ok(csp.split('; ').includes(d), d);
  assert.doesNotMatch(csp, /unsafe-inline|unsafe-eval|\*|http:/);
  assert.equal(get('X-Content-Type-Options'), 'nosniff');
  assert.equal(get('X-Frame-Options'), 'DENY');
  assert.equal(get('Referrer-Policy'), 'strict-origin-when-cross-origin');
  assert.equal(get('Cross-Origin-Opener-Policy'), 'same-origin');
  assert.equal(get('Cross-Origin-Resource-Policy'), 'same-origin');
  for (const f of ['camera', 'microphone', 'geolocation', 'payment', 'usb']) assert.match(get('Permissions-Policy'), new RegExp(`${f}=\\(\\)`));
  assert.doesNotMatch(h, /preload/i);
  assert.ok(h.split('\n').every((l) => l.length < 2000), 'Cloudflare Pages allows 2,000 characters per header line');
  assert.ok(h.split('\n').filter((l) => /^\S/.test(l)).length <= 100, 'Cloudflare Pages allows 100 header rules');
  // the hashed boot script is the page's only inline script, and the page's boot script is the one the header allows
  for (const p of pages) {
    const inline = [...p.s.matchAll(/<script(?![^>]*\bsrc=)([^>]*)>([\s\S]*?)<\/script>/g)].filter((m) => !/application\/(ld\+)?json/.test(m[1]));
    assert.equal(inline.length, 1, `inline scripts on ${p.path}`);
    assert.equal(crypto.createHash('sha256').update(inline[0][2]).digest('base64'), boot, p.path);
  }
});

test('pages are strict-CSP clean: no inline style or event handler, nothing from another site, no mixed content, outbound links are safe', { skip: SKIP }, () => {
  for (const p of pages) {
    const noLd = p.s.replace(/<script[\s\S]*?<\/script>/g, '');
    assert.doesNotMatch(noLd, /\sstyle="/, `inline style on ${p.path}`);
    assert.doesNotMatch(noLd, /\son[a-z]+="/i, `inline handler on ${p.path}`);
    assert.doesNotMatch(noLd, /<form\b/i, `a form on ${p.path}: add Turnstile and rate limits first (docs/SECURITY.md)`);
    for (const m of noLd.matchAll(/<(?:script|link|img|iframe|source|video|audio)\b[^>]*?\b(?:src|href)="(https?:)?\/\/([^"]+)"/g)) if (!m[0].includes('rel="canonical"') && !(m[1] + '//' + m[2]).startsWith(SITE_URL)) assert.fail(`${p.path} loads ${m[2]} from outside`);
    assert.doesNotMatch(noLd.replace(/xmlns="http:\/\/www\.w3\.org\/[^"]+"/g, ''), /(?:src|href|action)="http:\/\//, `mixed content on ${p.path}`);
    for (const m of noLd.matchAll(/<a\b[^>]*\bhref="https?:\/\/[^"]+"[^>]*>/g)) assert.match(m[0], /rel="[^"]*noopener/, `${p.path}: ${m[0]}`);
  }
});

test('redirects cannot be used as open redirects and stay inside the host limits', { skip: SKIP }, () => {
  const lines = read(path.join(DIST, '_redirects')).trim().split('\n');
  assert.ok(lines.length <= 2000, 'Cloudflare Pages allows 2,000 static redirects');
  for (const l of lines) {
    const [from, to, code] = l.split(/\s+/);
    assert.match(from, /^\/[^:]*$/, `no placeholders in the source: ${l}`);
    assert.match(to, /^\/(?!\/)[^\s:]*$/, `target must be a path on this site: ${l}`);
    assert.doesNotMatch(to, /\/\*|:splat|\\/, l);
    assert.ok(['301', '302'].includes(code), l);
  }
});

test('nothing private or secret reaches the build, and it fits the Cloudflare Pages limits', { skip: SKIP }, () => {
  const r = inspectDist(DIST, { secrets: [process.env.CLOUDFLARE_API_TOKEN] });
  assert.deepEqual(r.problems, []);
  assert.ok(r.files < LIMITS.files, `${r.files} files`);
  for (const f of walk(DIST)) assert.ok(fs.statSync(f).size < LIMITS.fileBytes, f);
  assert.ok(!fs.existsSync(path.join(DIST, 'functions')) && !fs.existsSync(path.join(DIST, '_worker.js')), 'no Pages Functions: the site has no server code');
  console.log(`# ${r.files} files in the build (limit ${LIMITS.files})`);
});

test('the build guard catches a secret, a private folder and an oversize-looking leak', () => {
  const d = fs.mkdtempSync(path.join(os.tmpdir(), 'of-guard-'));
  try {
    fs.mkdirSync(path.join(d, 'research')); fs.writeFileSync(path.join(d, 'research', 'a.txt'), 'x');
    fs.writeFileSync(path.join(d, '.env'), 'A=1'); fs.writeFileSync(path.join(d, 'app.js.map'), '{}');
    fs.writeFileSync(path.join(d, 'a.html'), 'CLOUDFLARE_API_TOKEN=abc'); fs.writeFileSync(path.join(d, 'b.txt'), 'my-token-value-12345');
    const r = inspectDist(d, { secrets: ['my-token-value-12345'] });
    for (const k of ['private folder', '.env', 'app.js.map', 'secret-like', 'saved secret']) assert.ok(r.problems.some((x) => x.includes(k)), k);
  } finally { fs.rmSync(d, { recursive: true, force: true }); }
});

test('repository: nothing secret or private is tracked, deploy.mjs never prints the token, no server code exists', () => {
  let tracked = [];
  try { tracked = execFileSync('git', ['ls-files'], { cwd: ROOT, encoding: 'utf8' }).split('\n').filter(Boolean); } catch { /* no git here */ }
  for (const f of tracked) {
    assert.doesNotMatch(f, /(^|\/)\.env|\.(pem|key|p12|pfx|map)$|^manuals\/|^dist\/|^\.cache\/|^\.wrangler\//, `tracked: ${f}`);
    if (/\.(mjs|js|json|md|toml|yml|yaml|txt|html|css)$/.test(f) && fs.existsSync(path.join(ROOT, f)) && fs.statSync(path.join(ROOT, f)).size < 2e6) {
      const t = fs.readFileSync(path.join(ROOT, f), 'utf8');
      for (const re of [/-----BEGIN [A-Z ]*PRIVATE KEY-----/, /\bAKIA[0-9A-Z]{16}\b/, /\bgh[pousr]_[A-Za-z0-9]{36,}\b/, /\bsk-[A-Za-z0-9]{32,}\b/]) assert.doesNotMatch(t, re, `${f} holds a secret`);
    }
  }
  const deploy = fs.readFileSync(path.join(ROOT, 'scripts', 'deploy.mjs'), 'utf8');
  assert.doesNotMatch(deploy, /console\.(log|error|warn|info)\([^)]*(env|TOKEN)/i, 'deploy.mjs must not print credentials');
  assert.match(deploy, /\[A-Za-z0-9\._-\]/, 'branch name is validated before it reaches the shell');
  for (const f of ['functions', '_worker.js', 'wrangler.toml', 'wrangler.json', '.env']) assert.ok(!fs.existsSync(path.join(ROOT, f)), `${f} exists: the site must stay static (docs/SECURITY.md)`);
});

test('IndexNow: a key file is published, and the script builds correct requests without sending any', { skip: SKIP }, () => {
  const key = findKey();
  assert.match(key, /^[0-9a-f]{32}$/);
  assert.equal(read(path.join(DIST, `${key}.txt`)), key);
  const urls = sitemapUrls(sitemap);
  assert.ok(urls.length > 100 && urls.every((u) => u.startsWith(SITE_URL)));
  const [b] = payloads(urls, SITE_URL, key);
  assert.deepEqual(Object.keys(b), ['host', 'key', 'keyLocation', 'urlList']);
  assert.equal(b.keyLocation, `${SITE_URL}/${key}.txt`);
  assert.equal(b.host, new URL(SITE_URL).host);
  assert.equal(payloads([...Array(10001)].map((_, i) => `${SITE_URL}/p${i}/`), SITE_URL, key).length, 2, 'batches of at most 10,000');
  assert.equal(payloads(['https://elsewhere.example/x/'], SITE_URL, key).length, 0, 'URLs of another host are dropped');
});

test('fast pages: fonts preloaded with crossorigin and swapped, one stylesheet, deferred script, every picture sized and described', { skip: SKIP }, () => {
  for (const p of pages) {
    const pre = [...p.s.matchAll(/<link rel="preload"[^>]*>/g)].map((m) => m[0]);
    for (const l of pre) assert.match(l, /as="font"[^>]*crossorigin|crossorigin[^>]*as="font"/, l);
    assert.equal((p.s.match(/<link rel="stylesheet"/g) || []).length, 1, p.path);
    assert.match(p.s, /<script src="\/assets\/site\.[0-9a-f]+\.js" defer><\/script>/, p.path);
    for (const m of p.s.matchAll(/<img\b[^>]*>/g)) {
      assert.match(m[0], /\swidth="\d+"/, `${p.path}: ${m[0]}`);
      assert.match(m[0], /\sheight="\d+"/, `${p.path}: ${m[0]}`);
      assert.match(m[0], /\salt="/, `${p.path}: ${m[0]}`);
    }
  }
  const css = read(path.join(DIST, 'assets', fs.readdirSync(path.join(DIST, 'assets')).find((n) => /^site\..*\.css$/.test(n))));
  const faces = css.match(/@font-face\{[^}]*\}/g);
  assert.ok(faces.length >= 3);
  for (const f of faces) assert.match(f, /font-display:swap/);
});
