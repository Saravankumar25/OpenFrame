// IndexNow: tells Bing, Yandex, Naver, Seznam, Yep and Amazon which pages changed, so they do not wait for a crawl.
// Google does not take part in IndexNow; it reads sitemap.xml. Spec: https://www.indexnow.org/documentation
// Run it AFTER a deploy, against the live domain (the key file must be live first). Modes:
//   node scripts/indexnow.mjs --since <old-sitemap.xml> --soft   CI mode: pages new, changed or gone since the old sitemap
//   node scripts/indexnow.mjs --changed                          pages whose lastmod is today (used when no old sitemap exists)
//   node scripts/indexnow.mjs                                    every sitemap URL (the first run, or to repeat a failed one)
//   node scripts/indexnow.mjs --dry-run ...                      prints what would be sent, sends nothing
// --soft makes every failure a warning with exit code 0, so a search-engine outage never fails a deploy.
// The old sitemap is the live one, saved BEFORE the deploy (docs/SEO-CHECKLIST.md, section "Deploy workflow").
// The key is public by design: it only proves that the file at /<key>.txt is ours. It is not a secret and is committed.
// The key file is src/static/<key>.txt (the file name is the key and the content is the key), published at /<key>.txt.
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { SITE_URL } from '../site.config.mjs';

const ROOT = path.resolve(import.meta.dirname, '..');
export const ENDPOINT = 'https://api.indexnow.org/indexnow';

export function findKey(dir = path.join(ROOT, 'src', 'static')) {
  const f = fs.readdirSync(dir).find((n) => /^[0-9a-f]{32}\.txt$/.test(n));
  if (!f) throw new Error('no IndexNow key file in src/static');
  const key = f.slice(0, -4);
  if (fs.readFileSync(path.join(dir, f), 'utf8').trim() !== key) throw new Error(`${f} must contain exactly its own name without .txt`);
  return key;
}
const unesc = (s) => s.replace(/&amp;/g, '&');
/** Map of loc -> lastmod ('' when a page has none). */
export function sitemapEntries(xml) {
  const out = new Map();
  for (const m of xml.matchAll(/<url>([\s\S]*?)<\/url>/g)) {
    const loc = m[1].match(/<loc>([^<]+)<\/loc>/)?.[1];
    if (loc) out.set(unesc(loc), m[1].match(/<lastmod>([^<]+)<\/lastmod>/)?.[1] ?? '');
  }
  return out;
}
export function sitemapUrls(xml, { onlyLastmod } = {}) {
  return [...sitemapEntries(xml)].filter(([, d]) => !onlyLastmod || d === onlyLastmod).map(([u]) => u);
}
/** What a deploy changed: pages that are new or have a different lastmod, plus pages that left the sitemap (IndexNow wants those too). */
export function changedUrls(oldXml, newXml) {
  const before = sitemapEntries(oldXml), after = sitemapEntries(newXml);
  return {
    changed: [...after].filter(([u, d]) => before.get(u) !== d).map(([u]) => u),
    removed: [...before.keys()].filter((u) => !after.has(u)),
  };
}
/** The request body. IndexNow accepts at most 10,000 URLs per call and every URL must be on the key's host. */
export function payloads(urls, siteUrl, key) {
  const host = new URL(siteUrl).host;
  const own = urls.filter((u) => new URL(u).host === host);
  const out = [];
  for (let i = 0; i < own.length; i += 10000) out.push({ host, key, keyLocation: `${siteUrl}/${key}.txt`, urlList: own.slice(i, i + 10000) });
  return out;
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const get = (url) => fetch(url, { signal: AbortSignal.timeout(15000), headers: { 'user-agent': 'straft-indexnow' } });

async function main() {
  const args = process.argv.slice(2);
  const flag = (n) => args.includes(n), value = (n) => args[args.indexOf(n) + 1];
  const host = new URL(SITE_URL).host;
  if (/\.pages\.dev$/.test(host) && !flag('--dry-run')) throw new Error('SITE_URL is still a pages.dev address. Set OF_SITE_URL to the custom domain, or use --dry-run.');
  const key = findKey();
  const xml = fs.readFileSync(path.join(ROOT, 'dist', 'sitemap.xml'), 'utf8');
  const today = new Date().toISOString().slice(0, 10);
  let urls, how;
  if (flag('--since')) {
    let old = '';
    try { old = fs.readFileSync(value('--since'), 'utf8'); } catch { /* no snapshot: handled below */ }
    if (sitemapEntries(old).size) {
      const d = changedUrls(old, xml);
      urls = [...d.changed, ...d.removed];
      how = `${d.changed.length} new or changed, ${d.removed.length} removed since the old sitemap`;
    } else { urls = sitemapUrls(xml, { onlyLastmod: today }); how = `no usable old sitemap, so only pages with lastmod ${today}`; }
  } else if (flag('--changed')) { urls = sitemapUrls(xml, { onlyLastmod: today }); how = `pages with lastmod ${today}`; }
  else { urls = sitemapUrls(xml); how = 'every sitemap URL'; }
  const bodies = payloads(urls, SITE_URL, key);
  console.log(`IndexNow: ${urls.length} URL(s) for ${host} (${how}), ${bodies.length} request(s)`);
  if (flag('--dry-run')) { urls.slice(0, 20).forEach((u) => console.log(`  ${u}`)); if (urls.length > 20) console.log(`  ... and ${urls.length - 20} more`); return; }
  if (!bodies.length) return console.log('IndexNow: nothing changed, nothing sent');
  // The key file must already be live, or the engines reject the call. A fresh deploy can take a few seconds to show.
  let live = '';
  for (let i = 0; i < 4 && live !== key; i++) {
    if (i) await sleep(5000);
    live = await get(`${SITE_URL}/${key}.txt`).then((r) => (r.ok ? r.text() : '')).then((t) => t.trim(), () => '');
  }
  if (live !== key) throw new Error(`${SITE_URL}/${key}.txt does not return the key`);
  for (const b of bodies) {
    const r = await fetch(ENDPOINT, { method: 'POST', headers: { 'content-type': 'application/json; charset=utf-8' }, body: JSON.stringify(b), signal: AbortSignal.timeout(30000) });
    console.log(`IndexNow answered ${r.status} for ${b.urlList.length} URL(s) (200 received, 202 received and the key is still being checked)`);
    if (r.status !== 200 && r.status !== 202) throw new Error(`IndexNow rejected the submission: ${r.status} (400 format, 403 key, 422 host or repeat, 429 too many)`);
  }
}
if (process.argv[1] && pathToFileURL(path.resolve(process.argv[1])).href === import.meta.url) {
  try { await main(); } catch (e) {
    if (!process.argv.includes('--soft')) throw e;
    // shortcut: a failed run is not retried; repeat it by hand with no flags (every URL), at most once an hour, or wait for the next change
    console.log(`IndexNow skipped (soft mode, the deploy is not affected): ${e.message}`);
  }
}
