// Live check of what search engines see: robots.txt, sitemap.xml, a spread of sitemap pages (status, canonical, noindex, one h1),
// a real 404 for a missing page, llms.txt and the IndexNow key file. Public requests only, no credentials. Exits 1 on any failure.
//   node scripts/seo-monitor.mjs                  checks SITE_URL (https://straftstudio.com) with 30 pages from the sitemap
//   node scripts/seo-monitor.mjs --sample 100     more pages;   --all  every sitemap URL (about a thousand requests)
//   node scripts/seo-monitor.mjs https://other.example   another address
// Run it after a deploy (CI) or on a schedule. It complements scripts/cloudflare-ops.mjs verify (redirects, TLS), which it does not repeat.
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { SITE_URL } from '../site.config.mjs';
import { findKey, sitemapUrls } from './indexnow.mjs';

export const canonicalOf = (html) => html.match(/<link rel="canonical" href="([^"]+)"/)?.[1];
export const robotsMetaOf = (html) => html.match(/<meta name="robots" content="([^"]*)"/)?.[1] ?? '';
export const h1Count = (html) => (html.match(/<h1[\s>]/g) || []).length;
/** Disallow paths that apply to every crawler (the `User-agent: *` group). */
export function disallowedPaths(robots) {
  const out = []; let star = false, inGroupHead = false;
  for (const raw of robots.split(/\r?\n/)) {
    const line = raw.replace(/#.*/, '').trim(), m = line.match(/^([A-Za-z-]+):\s*(.*)$/);
    if (!m) continue;
    const k = m[1].toLowerCase();
    if (k === 'user-agent') { if (!inGroupHead) star = false; inGroupHead = true; if (m[2] === '*') star = true; continue; }
    inGroupHead = false;
    if (star && k === 'disallow' && m[2]) out.push(m[2]);
  }
  return out;
}
/** At most n URLs spread evenly over the list, always the first and the last. */
export function spread(list, n) {
  if (list.length <= n) return list;
  return [...new Set(Array.from({ length: n }, (_, i) => list[Math.round((i * (list.length - 1)) / (n - 1))]))];
}
/** Problems with one page's HTML and headers; an empty list means it is fine for search. */
export function pageProblems(url, status, html, xRobots = '') {
  const p = [];
  if (status !== 200) return [`status ${status}`];
  if (canonicalOf(html) !== url) p.push(`canonical is ${canonicalOf(html)}`);
  if (/noindex/i.test(robotsMetaOf(html))) p.push('meta robots says noindex');
  if (/noindex/i.test(xRobots)) p.push('X-Robots-Tag says noindex');
  if (h1Count(html) !== 1) p.push(`${h1Count(html)} h1 elements`);
  return p;
}

async function main() {
  const args = process.argv.slice(2);
  const base = (args.find((a) => /^https?:\/\//.test(a)) || SITE_URL).replace(/\/+$/, '');
  const all = args.includes('--all'), n = Number(args[args.indexOf('--sample') + 1]) || 30;
  const problems = [];
  const ok = (m) => console.log(`  ok    ${m}`), bad = (m) => { problems.push(m); console.log(`  FAIL  ${m}`); };
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  const get = async (url) => {
    try { const r = await fetch(url, { redirect: 'manual', signal: AbortSignal.timeout(20000), headers: { 'user-agent': 'straft-seo-monitor' } }); return { status: r.status, headers: r.headers, text: await r.text() }; }
    catch (e) { return { status: 0, headers: new Headers(), text: '', error: e.cause?.code || e.message }; }
  };
  // A fresh deploy can take a moment to reach every edge, so a failing check is tried up to three times.
  const check = async (label, fn) => {
    let last = '';
    for (let i = 0; i < 3; i++) { if (i) await sleep(5000); last = await fn(); if (!last) return ok(label); }
    bad(`${label}: ${last}`);
  };

  console.log(`SEO check of ${base}`);
  let robots = '';
  await check('robots.txt: 200, names the sitemap, does not block the site', async () => {
    const r = await get(`${base}/robots.txt`); robots = r.text;
    if (r.status !== 200) return `status ${r.status}`;
    if (!r.text.includes(`Sitemap: ${base}/sitemap.xml`)) return 'no Sitemap line for this address';
    return disallowedPaths(r.text).some((d) => d === '/') ? 'Disallow: / blocks everything' : '';
  });

  let urls = [];
  await check('sitemap.xml: 200, XML, 100 or more URLs, all on this host, no duplicates, none blocked by robots.txt', async () => {
    const r = await get(`${base}/sitemap.xml`);
    if (r.status !== 200) return `status ${r.status}`;
    if (!/xml/.test(r.headers.get('content-type') || '')) return `content-type ${r.headers.get('content-type')}`;
    urls = sitemapUrls(r.text);
    if (urls.length < 100) return `only ${urls.length} URLs`;
    if (new Set(urls).size !== urls.length) return 'duplicate URLs';
    const off = urls.find((u) => !u.startsWith(base + '/'));
    if (off) return `${off} is on another host`;
    const blocked = urls.find((u) => disallowedPaths(robots).some((d) => new URL(u).pathname.startsWith(d)));
    return blocked ? `${blocked} is disallowed by robots.txt` : '';
  });

  const pages = urls.length ? [...new Set([`${base}/`, ...(all ? urls : spread(urls, n))])] : [`${base}/`];
  let next = 0, failed = 0;
  const worker = async () => {
    while (next < pages.length) {
      const url = pages[next++];
      let p = [];
      for (let i = 0; i < 3; i++) { if (i) await sleep(5000); const r = await get(url); p = pageProblems(url, r.status, r.text, r.headers.get('x-robots-tag') || ''); if (!p.length) break; }
      if (p.length) { failed++; bad(`${url}: ${p.join('; ')}`); }
    }
  };
  await Promise.all(Array.from({ length: 6 }, worker));
  if (!failed) ok(`${pages.length} page(s) answer 200 with their own canonical, no noindex and one h1`);

  await check('a missing page returns a real 404', async () => { const r = await get(`${base}/seo-monitor-missing-page/`); return r.status === 404 ? '' : `status ${r.status}`; });
  await check('llms.txt answers 200 as text', async () => { const r = await get(`${base}/llms.txt`); return r.status === 200 && /text\/plain/.test(r.headers.get('content-type') || '') ? '' : `status ${r.status}`; });
  let key = '';
  try { key = findKey(); } catch { /* run from a folder without the source: the key check is skipped */ }
  if (key) await check('IndexNow key file returns the key', async () => { const r = await get(`${base}/${key}.txt`); return r.status === 200 && r.text.trim() === key ? '' : `status ${r.status} or wrong content`; });

  console.log(problems.length ? `\n${problems.length} problem(s)` : '\nall SEO checks passed');
  process.exit(problems.length ? 1 : 0);
}
if (process.argv[1] && pathToFileURL(path.resolve(process.argv[1])).href === import.meta.url) await main();
