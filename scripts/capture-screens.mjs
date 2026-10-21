// Capture the app screens the site needs.  Usage: node scripts/capture-screens.mjs [--force]
// Needs: tutorials in src/content/tutorials/{win,mac}/*.mjs and SCREENS exports in src/pages/*.mjs, src/content/*.mjs.
import { chromium } from 'playwright';
import sharp from 'sharp';
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const ROOT = path.resolve(import.meta.dirname, '..');
const OUT = path.join(ROOT, 'screens');
const SANITIZE_VERSION = 1; // bump when the rules below change, so cached images are rebuilt
const FORCE = process.argv.includes('--force');
const SRC = {
  win: process.env.OF_WIN_HTML || path.join(ROOT, 'research/WindowsHTML-src'),
  mac: process.env.OF_MAC_HTML || path.join(ROOT, 'research/MacHTML-src'),
};
const srcFile = (plat, screen) => path.join(SRC[plat], 'light', screen + '.html');

// ---- 1. what is needed: need[plat][screen] = { selectors: Set, og: bool }
const need = { win: new Map(), mac: new Map() };
const add = (plat, screen, selector, tag) => {
  if (!need[plat].has(screen)) need[plat].set(screen, { selectors: new Map(), og: false });
  const e = need[plat].get(screen);
  if (selector) { if (!e.selectors.has(selector)) e.selectors.set(selector, []); e.selectors.get(selector).push(tag); }
};
const markOg = (plat, screen) => { add(plat, screen); need[plat].get(screen).og = true; };
add('win', '003-app-home-empty'); add('mac', '003-home-empty');

const load = async (file) => {
  try { return await import(pathToFileURL(file).href); }
  catch (err) { console.warn(`warn: skipped ${path.relative(ROOT, file)} (${String(err.message).split('\n')[0]})`); return null; }
};
const mjsIn = (dir) => (fs.existsSync(dir) ? fs.readdirSync(dir).filter((f) => f.endsWith('.mjs')).sort().map((f) => path.join(dir, f)) : []);

for (const plat of ['win', 'mac']) {
  for (const file of mjsIn(path.join(ROOT, 'src/content/tutorials', plat))) {
    const t = (await load(file))?.default;
    if (!t || t.available === false) continue;
    (t.steps || []).forEach((s, i) => { if (s.screen) add(plat, s.screen, s.target || null, `${plat}/${t.id} step ${i + 1}`); });
  }
}
for (const file of [...mjsIn(path.join(ROOT, 'src/pages')), ...mjsIn(path.join(ROOT, 'src/content'))]) {
  const S = (await load(file))?.SCREENS;
  if (!S) continue;
  for (const plat of ['win', 'mac']) for (const s of S[plat] || []) markOg(plat, s);
}

// ---- previous manifest (cache)
const manifestPath = path.join(OUT, 'manifest.json');
let prev = {};
try { prev = JSON.parse(fs.readFileSync(manifestPath, 'utf8')); } catch { /* none yet */ }

// ---- 2. jobs
const stale = prev._sanitize !== SANITIZE_VERSION;
const jobs = [], stats = { rendered: 0, cached: 0, missingSource: [], missingTargets: [] };
const manifest = {};
for (const plat of ['win', 'mac']) {
  fs.mkdirSync(path.join(OUT, plat), { recursive: true });
  for (const [screen, e] of need[plat]) {
    const key = `${plat}/${screen}`, base = path.join(OUT, plat, screen);
    const src = srcFile(plat, screen);
    if (!fs.existsSync(src)) { stats.missingSource.push(key); continue; }
    const p = prev[key];
    const haveFiles = fs.existsSync(base + '-1600.webp') && fs.existsSync(base + '-800.webp') && (!e.og || fs.existsSync(base + '-1200.jpg'));
    const fresh = haveFiles && p && fs.statSync(src).mtimeMs < Math.min(fs.statSync(base + '-1600.webp').mtimeMs, fs.statSync(base + '-800.webp').mtimeMs);
    const sels = [...e.selectors.keys()];
    if (!FORCE && !stale && fresh && sels.every((s) => s in (p.targets || {}))) {
      manifest[key] = { ...p, targets: Object.fromEntries(sels.map((s) => [s, p.targets[s]])) };
      for (const s of sels) if (p.targets[s] === null) stats.missingTargets.push(`${key} ${s} (${e.selectors.get(s).join(', ')})`);
      stats.cached++; continue;
    }
    jobs.push({ plat, screen, key, base, src, e, sels });
  }
}

// ---- 3. render, 4 at a time
const browser = await chromium.launch();
const ctx = {
  win: await browser.newContext({ viewport: { width: 1600, height: 1000 }, deviceScaleFactor: 1 }),
  mac: await browser.newContext({ viewport: { width: 1600, height: 1000 }, deviceScaleFactor: 2 }),
};

// Runs in the page before the screenshot: swaps the owner's name, initials, local paths and CPU model for neutral
// text (text nodes, every attribute, input values, title). Returns any leftover private string for the self-check.
function sanitizePage() {
  const swap = (t) => t
    .replace(/Saravan Kumar/g, 'Maya Iyer')
    .replace(/[A-Za-z]:\\[^\s"'<>|?*\u00b7]*/g, (m) => {
      const tail = m.match(/[.,;:)]+$/)?.[0] || '';
      const last = m.slice(0, m.length - tail.length).split('\\').filter(Boolean).pop() || '';
      return 'C:\\Users\\Maya\\Documents\\OpenFrame\\' + last + tail;
    })
    .replace(/\/Users\/[^/\s"'<>]+\//g, '/Users/maya/')
    .replace(/Intel\(R\) Core\(TM\)[^\u00b7<"]*?(?=\s*(\u00b7|$))/g, 'Multi-core processor');
  const walker = document.createTreeWalker(document, NodeFilter.SHOW_TEXT);
  for (let n = walker.nextNode(); n; n = walker.nextNode()) {
    const v = swap(n.nodeValue);
    if (v !== n.nodeValue) n.nodeValue = v;
  }
  for (const el of document.querySelectorAll('*')) {
    for (const a of [...el.attributes]) { const v = swap(a.value); if (v !== a.value) el.setAttribute(a.name, v); }
    if ('value' in el && typeof el.value === 'string') { const v = swap(el.value); if (v !== el.value) el.value = v; }
    if (/avatar/i.test(el.className?.toString?.() || '') && el.textContent.trim() === 'SK') {
      const w = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
      for (let n = w.nextNode(); n; n = w.nextNode()) n.nodeValue = n.nodeValue.replace('SK', 'MI');
    }
  }
  document.title = swap(document.title);
  const dump = document.documentElement.outerHTML + '\n' + [...document.querySelectorAll('input,textarea')].map((e) => e.value).join('\n');
  return dump.match(/saravan|sarava|E2E_|AppData|Temp\\|\\Users\\(?!Maya)|C:\\OpenFrame|\.run\\|Intel\(R\)|>\s*SK\s*</i)?.[0] || null;
}

async function render({ plat, screen, key, base, src, e, sels }) {
  const page = await ctx[plat].newPage();
  try {
    await page.goto(pathToFileURL(src).href, { waitUntil: 'load' });
    await page.waitForLoadState('networkidle').catch(() => {});
    await page.evaluate(() => document.fonts.ready);
    const leak = await page.evaluate(sanitizePage);
    if (leak) throw new Error('private text left after sanitizing: ' + leak);
    const root = page.locator(plat === 'mac' ? '.win' : 'html');
    const box = await root.boundingBox();
    if (!box) throw new Error('no capture area');
    const png = plat === 'mac' ? await root.screenshot({ type: 'png' }) : await page.screenshot({ type: 'png' });
    const targets = {};
    for (const s of sels) {
      const loc = root.locator(s), n = await loc.count();
      if (n === 0) { targets[s] = null; stats.missingTargets.push(`${key} ${s} (${e.selectors.get(s).join(', ')})`); continue; }
      if (n > 1) console.warn(`warn: ${key} selector matched ${n}, using first: ${s} (${e.selectors.get(s).join(', ')})`);
      const b = await loc.first().boundingBox();
      targets[s] = b ? [b.x - box.x, b.y - box.y, b.width, b.height].map(Math.round) : null;
      if (!b) stats.missingTargets.push(`${key} ${s} (not visible; ${e.selectors.get(s).join(', ')})`);
    }
    const img = sharp(png);
    const webp = { quality: 78, effort: 5 };
    await Promise.all([
      img.clone().resize({ width: 1600, withoutEnlargement: false }).webp(webp).toFile(base + '-1600.webp'),
      img.clone().resize({ width: 800 }).webp(webp).toFile(base + '-800.webp'),
      e.og ? img.clone().resize({ width: 1200 }).flatten({ background: '#ffffff' }).jpeg({ quality: 80 }).toFile(base + '-1200.jpg') : null,
    ]);
    manifest[key] = { w: Math.round(box.width), h: Math.round(box.height), src: `/screens/${plat}/${screen}`, targets };
    stats.rendered++;
  } catch (err) {
    console.warn(`warn: ${key} failed: ${err.message}`);
    stats.failed = (stats.failed || []).concat(key);
  } finally { await page.close(); }
}

let next = 0;
await Promise.all(Array.from({ length: 4 }, async () => { while (next < jobs.length) await render(jobs[next++]); }));
await browser.close();

// ---- 4. drop what is no longer needed, write manifest
let removed = 0;
for (const plat of ['win', 'mac']) {
  const keep = new Set([...need[plat].keys()]);
  for (const f of fs.readdirSync(path.join(OUT, plat))) {
    const m = f.match(/^(.*)-(1600\.webp|800\.webp|1200\.jpg)$/);
    if (m && !keep.has(m[1])) { fs.unlinkSync(path.join(OUT, plat, f)); removed++; }
  }
}
const sorted = Object.fromEntries(Object.keys(manifest).sort().map((k) => [k, manifest[k]]));
fs.writeFileSync(manifestPath, JSON.stringify({ ...sorted, _sanitize: SANITIZE_VERSION, _generated: new Date().toISOString() }, null, 1) + '\n');

let bytes = 0;
for (const plat of ['win', 'mac']) for (const f of fs.readdirSync(path.join(OUT, plat))) bytes += fs.statSync(path.join(OUT, plat, f)).size;
console.log(`screens needed: ${need.win.size} win, ${need.mac.size} mac`);
console.log(`rendered ${stats.rendered}, cached ${stats.cached}, removed ${removed} files, images ${(bytes / 1048576).toFixed(1)} MB`);
if (stats.missingSource.length) console.log(`missing source html (${stats.missingSource.length}):\n  ${stats.missingSource.join('\n  ')}`);
if (stats.missingTargets.length) console.log(`missing targets (${stats.missingTargets.length}):\n  ${stats.missingTargets.join('\n  ')}`);
if (stats.failed) { console.log(`failed: ${stats.failed.join(', ')}`); process.exitCode = 1; }
