// Mobile performance check: loads pages from OF_DIST (default dist/) in headless Chromium on a throttled phone profile
// (slow 4G, 4x slower CPU), cold cache, and prints LCP, CLS, FCP and compressed transfer per page.
// Usage: OF_DIST=dist node scripts/perf.mjs [path ...]   (defaults to /, /features/, /pricing/ and one tutorial)
import { createServer } from 'node:http';
import { readFile, stat } from 'node:fs/promises';
import { brotliCompressSync, constants } from 'node:zlib';
import path from 'node:path';
import { chromium, devices } from 'playwright';

const ROOT = path.resolve(process.env.OF_DIST || 'dist');
const PATHS = process.argv.slice(2).length ? process.argv.slice(2) : ['/', '/features/', '/pricing/', '/windows/tutorials/create-first-project/'];
const RUNS = Number(process.env.RUNS) || 3;
const TYPES = { '.html': 'text/html; charset=utf-8', '.css': 'text/css', '.js': 'text/javascript', '.json': 'application/json', '.webp': 'image/webp', '.png': 'image/png', '.svg': 'image/svg+xml', '.woff2': 'font/woff2', '.txt': 'text/plain', '.xml': 'application/xml', '.ico': 'image/x-icon', '.webmanifest': 'application/manifest+json' };
const COMPRESSIBLE = /\.(html|css|js|json|svg|txt|xml|webmanifest|ico)$/;

// Serves dist like the host does: Brotli for text, long cache for /assets, no framing tricks.
const server = createServer(async (req, res) => {
  let p = decodeURIComponent(new URL(req.url, 'http://x').pathname);
  let file = path.join(ROOT, p);
  if (!file.startsWith(ROOT)) { res.writeHead(403).end(); return; }
  try { if ((await stat(file)).isDirectory()) file = path.join(file, 'index.html'); } catch { res.writeHead(404).end(); return; }
  try {
    let body = await readFile(file);
    const h = { 'content-type': TYPES[path.extname(file)] || 'application/octet-stream', 'cache-control': p.startsWith('/assets/') ? 'public, max-age=31536000, immutable' : 'public, max-age=0, must-revalidate' };
    if (COMPRESSIBLE.test(file) && /br/.test(req.headers['accept-encoding'] || '')) { body = brotliCompressSync(body, { params: { [constants.BROTLI_PARAM_QUALITY]: 5 } }); h['content-encoding'] = 'br'; }
    h['content-length'] = body.length;
    res.writeHead(200, h).end(body);
  } catch { res.writeHead(404).end(); }
});
await new Promise((r) => server.listen(0, '127.0.0.1', r));
const base = `http://127.0.0.1:${server.address().port}`;
const browser = await chromium.launch();
const median = (a) => [...a].sort((x, y) => x - y)[Math.floor(a.length / 2)];

async function once(url) {
  const ctx = await browser.newContext({ ...devices['Pixel 5'] });
  const page = await ctx.newPage();
  const cdp = await ctx.newCDPSession(page);
  await cdp.send('Network.enable');
  await cdp.send('Network.emulateNetworkConditions', { offline: false, latency: 150, downloadThroughput: (1.6 * 1024 * 1024) / 8, uploadThroughput: (750 * 1024) / 8 });
  await cdp.send('Emulation.setCPUThrottlingRate', { rate: 4 });
  const bytes = { html: 0, css: 0, js: 0, font: 0, image: 0, other: 0 }; let count = 0; const imgs = [];
  cdp.on('Network.loadingFinished', (e) => { reqs.get(e.requestId) && (reqs.get(e.requestId).len = e.encodedDataLength); });
  const reqs = new Map();
  cdp.on('Network.responseReceived', (e) => reqs.set(e.requestId, { type: e.type, url: e.response.url }));
  await page.addInitScript(() => {
    window.__m = { lcp: 0, lcpEl: '', cls: 0 };
    new PerformanceObserver((l) => { for (const e of l.getEntries()) { window.__m.lcp = e.startTime; window.__m.lcpEl = e.element ? e.element.tagName + (e.element.className ? '.' + String(e.element.className).split(' ')[0] : '') : ''; } }).observe({ type: 'largest-contentful-paint', buffered: true });
    new PerformanceObserver((l) => { for (const e of l.getEntries()) if (!e.hadRecentInput) window.__m.cls += e.value; }).observe({ type: 'layout-shift', buffered: true });
  });
  await page.goto(base + url, { waitUntil: 'load' });
  await page.waitForTimeout(1500);
  const m = await page.evaluate(() => ({ ...window.__m, fcp: performance.getEntriesByName('first-contentful-paint')[0]?.startTime || 0 }));
  for (const r of reqs.values()) {
    count++;
    const k = r.type === 'Document' ? 'html' : r.type === 'Stylesheet' ? 'css' : r.type === 'Script' ? 'js' : r.type === 'Font' ? 'font' : r.type === 'Image' ? 'image' : 'other';
    bytes[k] += r.len || 0; if (k === 'image' && !r.url.startsWith('data:')) imgs.push([r.url.replace(base, ''), r.len]);
  }
  await ctx.close();
  return { ...m, bytes, count, imgs };
}

console.log(`Profile: Pixel 5, slow 4G (1.6 Mbps, 150 ms), CPU x4, cold cache, Brotli, median of ${RUNS}. Source: ${ROOT}`);
for (const u of PATHS) {
  const r = []; for (let i = 0; i < RUNS; i++) r.push(await once(u));
  const t = (k) => median(r.map((x) => x[k])); const last = r[r.length - 1];
  const total = Object.values(last.bytes).reduce((a, b) => a + b, 0);
  console.log(`${u}\n  LCP ${Math.round(t('lcp'))} ms (${last.lcpEl})  FCP ${Math.round(t('fcp'))} ms  CLS ${t('cls').toFixed(3)}  requests ${last.count}  transfer ${(total / 1024).toFixed(0)} KB  ${Object.entries(last.bytes).map(([k, v]) => `${k} ${(v / 1024).toFixed(0)}`).join(', ')}`);
  if (last.imgs.length) console.log('  images: ' + last.imgs.slice(0, 6).map(([n, l]) => `${n} ${(l / 1024).toFixed(0)} KB`).join('; ') + (last.imgs.length > 6 ? ` ... (${last.imgs.length})` : ''));
}
await browser.close(); server.close();
