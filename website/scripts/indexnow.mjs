// IndexNow: tells Bing, Yandex, Naver, Seznam and Yep which pages changed, so they do not wait for a crawl.
// Google does not use IndexNow; it reads sitemap.xml. NOT run during development: run it once, after a deploy,
// when the custom domain is live and OF_SITE_URL points at it:
//   OF_SITE_URL=https://example.com node scripts/build.mjs && node scripts/deploy.mjs
//   OF_SITE_URL=https://example.com node scripts/indexnow.mjs            (all sitemap URLs)
//   OF_SITE_URL=https://example.com node scripts/indexnow.mjs --changed  (only pages whose lastmod is today)
//   node scripts/indexnow.mjs --dry-run                                   (prints what would be sent, sends nothing)
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
export function sitemapUrls(xml, { onlyLastmod } = {}) {
  return [...xml.matchAll(/<url>([\s\S]*?)<\/url>/g)]
    .filter((m) => !onlyLastmod || new RegExp(`<lastmod>${onlyLastmod}</lastmod>`).test(m[1]))
    .map((m) => m[1].match(/<loc>([^<]+)<\/loc>/)?.[1].replace(/&amp;/g, '&')).filter(Boolean);
}
/** The request body. IndexNow accepts at most 10,000 URLs per call and every URL must be on the key's host. */
export function payloads(urls, siteUrl, key) {
  const host = new URL(siteUrl).host;
  const own = urls.filter((u) => new URL(u).host === host);
  const out = [];
  for (let i = 0; i < own.length; i += 10000) out.push({ host, key, keyLocation: `${siteUrl}/${key}.txt`, urlList: own.slice(i, i + 10000) });
  return out;
}

async function main() {
  const args = new Set(process.argv.slice(2));
  const host = new URL(SITE_URL).host;
  if (/\.pages\.dev$/.test(host) && !args.has('--dry-run')) throw new Error('SITE_URL is still a pages.dev address. Set OF_SITE_URL to the custom domain after it is live, or use --dry-run.');
  const key = findKey();
  const xml = fs.readFileSync(path.join(ROOT, 'dist', 'sitemap.xml'), 'utf8');
  const urls = sitemapUrls(xml, args.has('--changed') ? { onlyLastmod: new Date().toISOString().slice(0, 10) } : {});
  const bodies = payloads(urls, SITE_URL, key);
  console.log(`${urls.length} URLs for ${host}, ${bodies.length} request(s)`);
  if (args.has('--dry-run')) return;
  // The key file must already be live, or the engines reject the call.
  const live = await fetch(`${SITE_URL}/${key}.txt`).then((r) => (r.ok ? r.text() : ''));
  if (live.trim() !== key) throw new Error(`${SITE_URL}/${key}.txt does not return the key: deploy first`);
  for (const b of bodies) {
    const r = await fetch(ENDPOINT, { method: 'POST', headers: { 'content-type': 'application/json; charset=utf-8' }, body: JSON.stringify(b) });
    console.log(`IndexNow answered ${r.status} (200 or 202 means accepted)`);
    if (r.status >= 400) throw new Error(`IndexNow rejected the submission: ${r.status}`);
  }
}
if (process.argv[1] && pathToFileURL(path.resolve(process.argv[1])).href === import.meta.url) await main();
