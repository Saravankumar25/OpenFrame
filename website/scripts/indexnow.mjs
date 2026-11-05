// IndexNow: tells Bing, Yandex, Naver, Seznam, Yep and Amazon which pages changed, so they do not wait for a crawl.
// Google does not take part in IndexNow; it reads sitemap.xml. Spec: https://www.indexnow.org/documentation
// Run it AFTER a production deploy, against the live domain (the key file must be live first). Modes, exactly one is required:
//   node scripts/indexnow.mjs --since <old-sitemap.xml> --soft   CI mode: pages new, changed or gone since the old sitemap
//   node scripts/indexnow.mjs --changed                          pages whose lastmod is today (when no old sitemap exists)
//   node scripts/indexnow.mjs --all                              every sitemap URL: the one-time first run, never routine
//   add --dry-run to print what would be sent, --sitemap <file> to read another sitemap than dist/sitemap.xml
// --since and --changed refuse to send more than MAX_AUTO URLs (a site-wide lastmod change must not become a full resubmission).
// --soft makes every failure a warning with exit code 0, so a search-engine outage never fails a deploy.
// The old sitemap is the live one, saved BEFORE the deploy (docs/SEO-CHECKLIST.md, section "Deploy workflow").
// The key is public by design: it only proves that the file at /<key>.txt is ours. It is not a secret and is committed.
// The script still never prints the key or the request body, so run logs stay free of it.
// The key file is src/static/<key>.txt (the file name is the key and the content is the key), published at /<key>.txt.
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { SITE_URL } from '../site.config.mjs';

const ROOT = path.resolve(import.meta.dirname, '..');
export const ENDPOINT = 'https://api.indexnow.org/indexnow';
export const MAX_AUTO = 100; // refuse above this in --since and --changed; --all is the explicit way to send more
const RETRIES = 3; // after the first try, for 429, 5xx and network errors

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
const PERMANENT = { 400: 'invalid format', 403: 'the key is not valid (key file missing or wrong content)', 422: 'a URL is not on the key host, or the key does not match the protocol' };

/** Milliseconds to wait from a Retry-After header (seconds or an HTTP date), capped at one minute; null when absent or unreadable. */
export function retryAfterMs(h, now = Date.now()) {
  if (!h) return null;
  const secs = /^\d+$/.test(h.trim()) ? Number(h) : (Date.parse(h) - now) / 1000;
  return Number.isFinite(secs) ? Math.min(Math.max(secs, 0), 60) * 1000 : null;
}
/** Refuses anything that is not an https URL on the site host. It throws and never filters, so a bad list cannot go out by accident. */
export function assertOwnHost(body, siteUrl) {
  const host = new URL(siteUrl).host;
  if (body.host !== host) throw new Error(`refusing to submit for host ${body.host}: only ${host} is allowed`);
  for (const u of body.urlList) {
    let p;
    try { p = new URL(u); } catch { throw new Error(`refusing to submit a URL that does not parse: ${u}`); }
    if (p.host !== host || p.protocol !== 'https:') throw new Error(`refusing to submit ${u}: only https URLs on ${host} are allowed`);
  }
}
/**
 * POSTs one body to the official endpoint (the address is fixed here, never a parameter). 200 and 202 are success; 400, 403 and 422 are
 * permanent and throw at once; 429, 5xx and network errors are retried RETRIES times with backoff (2, 4, 8 seconds, or Retry-After).
 * Messages never contain the key or the body. `io` lets tests replace fetch and sleep.
 */
export async function submit(body, siteUrl, io = {}) {
  const doFetch = io.fetch ?? fetch, wait = io.sleep ?? sleep;
  assertOwnHost(body, siteUrl);
  let last = '';
  for (let attempt = 0; attempt <= RETRIES; attempt++) {
    let delay = 2000 * 2 ** attempt;
    try {
      const r = await doFetch(ENDPOINT, { method: 'POST', headers: { 'content-type': 'application/json; charset=utf-8' }, body: JSON.stringify(body), signal: AbortSignal.timeout(30000) });
      if (r.status === 200 || r.status === 202) return r.status;
      if (PERMANENT[r.status]) throw Object.assign(new Error(`IndexNow rejected the submission with ${r.status}: ${PERMANENT[r.status]}`), { permanent: true });
      if (r.status !== 429 && r.status < 500) throw Object.assign(new Error(`IndexNow answered an unexpected ${r.status}`), { permanent: true });
      last = `status ${r.status}`;
      delay = retryAfterMs(r.headers?.get?.('retry-after')) ?? delay;
    } catch (e) {
      if (e.permanent) throw e;
      last = e.name === 'TimeoutError' ? 'timeout' : `network error (${e.code ?? e.name})`;
    }
    if (attempt < RETRIES) { console.log(`IndexNow: ${last}, retrying in ${Math.round(delay / 1000)}s (${attempt + 1} of ${RETRIES})`); await wait(delay); }
  }
  throw new Error(`IndexNow still failing after ${RETRIES} retries (${last})`);
}

async function main() {
  const args = process.argv.slice(2);
  const flag = (n) => args.includes(n), value = (n) => args[args.indexOf(n) + 1];
  const modes = ['--since', '--changed', '--all'].filter(flag);
  if (modes.length !== 1) throw new Error('choose exactly one of --since <old-sitemap.xml>, --changed or --all (--all is the one-time first submission of every URL)');
  const host = new URL(SITE_URL).host;
  if (/\.pages\.dev$/.test(host) && !flag('--dry-run')) throw new Error('SITE_URL is still a pages.dev address. Set OF_SITE_URL to the custom domain, or use --dry-run.');
  const key = findKey();
  const xml = fs.readFileSync(flag('--sitemap') ? path.resolve(value('--sitemap')) : path.join(ROOT, 'dist', 'sitemap.xml'), 'utf8');
  const today = new Date().toISOString().slice(0, 10);
  let urls, how, auto = true;
  if (flag('--since')) {
    let old = '';
    try { old = fs.readFileSync(value('--since'), 'utf8'); } catch { /* no snapshot: handled below */ }
    if (sitemapEntries(old).size) {
      const d = changedUrls(old, xml);
      urls = [...d.changed, ...d.removed];
      how = `${d.changed.length} new or changed, ${d.removed.length} removed since the old sitemap`;
    } else { urls = sitemapUrls(xml, { onlyLastmod: today }); how = `no usable old sitemap, so only pages with lastmod ${today}`; }
  } else if (flag('--changed')) { urls = sitemapUrls(xml, { onlyLastmod: today }); how = `pages with lastmod ${today}`; }
  else { urls = sitemapUrls(xml); how = 'every sitemap URL'; auto = false; }
  const bodies = payloads(urls, SITE_URL, key);
  const own = bodies.reduce((n, b) => n + b.urlList.length, 0);
  if (own !== urls.length) throw new Error(`${urls.length - own} URL(s) are not on ${host}; nothing was sent`);
  console.log(`IndexNow: ${urls.length} URL(s) for ${host} (${how}), ${bodies.length} request(s)`);
  if (auto && urls.length > MAX_AUTO) throw new Error(`${urls.length} URLs is more than the automatic limit of ${MAX_AUTO}, so nothing was sent. Check with --dry-run; if the whole site really changed, run --all once by hand`);
  if (flag('--dry-run')) { urls.slice(0, 20).forEach((u) => console.log(`  ${u}`)); if (urls.length > 20) console.log(`  ... and ${urls.length - 20} more`); return; }
  if (!bodies.length) return console.log('IndexNow: nothing changed, nothing sent');
  // The key file must already be live, or the engines reject the call. A fresh deploy can take a few seconds to show.
  let live = '';
  for (let i = 0; i < 4 && live !== key; i++) {
    if (i) await sleep(5000);
    live = await get(`${SITE_URL}/${key}.txt`).then((r) => (r.ok ? r.text() : '')).then((t) => t.trim(), () => '');
  }
  if (live !== key) throw new Error(`${SITE_URL}/<key>.txt does not return the key`);
  for (const b of bodies) console.log(`IndexNow answered ${await submit(b, SITE_URL)} for ${b.urlList.length} URL(s) (200 received, 202 received and the key is still being checked)`);
}
if (process.argv[1] && pathToFileURL(path.resolve(process.argv[1])).href === import.meta.url) {
  try { await main(); } catch (e) {
    if (!process.argv.includes('--soft')) throw e;
    // shortcut: a failed run is not retried later; the next deploy diffs against the new sitemap, so repeat by hand with --since or wait for the next change
    console.log(`${process.env.GITHUB_ACTIONS ? '::warning title=IndexNow::' : ''}IndexNow skipped (soft mode, the deploy is not affected): ${e.message}`);
  }
}
