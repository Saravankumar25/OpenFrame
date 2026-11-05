// Cleaned, interactive screen pages for the website cards and viewer.
//   node scripts/shots.mjs            both platforms (Mac first)
//   ONLY=mac|win  one platform         FORCE=1  redo everything         MATCH=regex  only screens whose name matches
//   QA=0  skip the contact sheets
// Output, in .cache/shots/pages/<win|mac>/:
//   <light|dark>/<file>.html + <file>.css   the screen, same file names as the catalog "file"; no inline styles or scripts
//   assets/                                 shared per platform: Windows app CSS, Mac window pictures (hard links to the PNGs), Mac base CSS
// The product name in every page, stylesheet and text asset is the website's (src/lib/brand.mjs): the apps still show the old one.
// .cache/shots/manifest.json, per catalog name:
//   { file, page: [w, h]       CSS size to render the page iframe at
//     card: [x, y, w, h]       the rectangle of the page a card shows: always the whole page (owner decision 2026-11-01)
//     full: [x, y, w, h]       the rectangle the viewer shows (the whole page)
//     focus, sidebar }
// Contact sheets, rendered from these pages: .cache/shots/qa/<plat>-<theme>-NN.jpg
// src/content/shot-overrides.json may set "focus" per "<plat>/<name>" (a label for the contact sheets).
import fs from 'node:fs';
import crypto from 'node:crypto';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium } from 'playwright';
import sharp from 'sharp';
import { macRepair } from './mac-repairs.mjs';
import { BRAND_RULES, makeRebrand, rebrand, hasOldName } from '../src/lib/brand.mjs';

const ROOT = path.resolve(import.meta.dirname, '..');
const RES = path.join(ROOT, 'research');
const OUT = path.join(ROOT, '.cache', 'shots');
const PAGES = path.join(OUT, 'pages');
const MANIFEST = path.join(OUT, 'manifest.json');
const OVR = path.join(ROOT, 'src', 'content', 'shot-overrides.json');
const FORCE = !!process.env.FORCE;
const ONLY = process.env.ONLY || '';
const MATCH = process.env.MATCH ? new RegExp(process.env.MATCH) : null;
const QA = process.env.QA !== '0';
const THEMES = ['light', 'dark'];

const readJson = (f, d) => { try { return JSON.parse(fs.readFileSync(f, 'utf8')); } catch { return d; } };
const overrides = readJson(OVR, {});
const manifest = readJson(MANIFEST, { win: {}, mac: {} });
manifest.win ||= {}; manifest.mac ||= {};
const touched = { win: new Set(), mac: new Set() };
const setEntry = (plat, name, v) => { manifest[plat][name] = v; touched[plat].add(name); };
const saveManifest = () => { // merge into whatever is on disk, so two runs (mac and win) never overwrite each other
  fs.mkdirSync(OUT, { recursive: true });
  const cur = readJson(MANIFEST, { win: {}, mac: {} }); cur.win ||= {}; cur.mac ||= {};
  for (const p of ['win', 'mac']) for (const n of touched[p]) cur[p][n] = manifest[p][n];
  fs.writeFileSync(MANIFEST + '.tmp', JSON.stringify(cur, null, 1)); fs.renameSync(MANIFEST + '.tmp', MANIFEST);
};
const mtime = (f) => { try { return fs.statSync(f).mtimeMs; } catch { return 0; } };
const BRAND_JS = path.join(ROOT, 'src', 'lib', 'brand.mjs');
const stamp = Math.max(mtime(OVR), mtime(import.meta.filename), mtime(BRAND_JS), mtime(path.join(ROOT, 'scripts', 'mac-repairs.mjs')));
const pagePath = (plat, theme, file) => path.join(PAGES, plat, theme, file);
const isFresh = (plat, rec, srcTime) => !FORCE && !!manifest[plat][rec.name]?.page && THEMES.every((t) => mtime(pagePath(plat, t, rec.file)) >= Math.max(srcTime, stamp));
const round = (r) => r.map((v) => Math.round(v));
let leftovers = 0; // written files that still hold the old name (should stay 0)
const writeFile = (f, text) => { // every text written into the pages gets the product name of the website
  text = rebrand(text); if (hasOldName(text)) { leftovers++; console.error('OLD NAME LEFT', path.relative(ROOT, f)); }
  fs.mkdirSync(path.dirname(f), { recursive: true }); fs.writeFileSync(f, text);
};
const BRAND = { brand: BRAND_RULES, brandFn: makeRebrand.toString() }; // the rename rules, rebuilt inside the page

// ---------- the page cleaner, run inside the browser (self-contained: Playwright ships its source) ----------
// Strips comments, capture links, paths and account names; renames the product (o.brand, o.brandFn: src/lib/brand.mjs) in text,
// every attribute and the styles; moves every style attribute into a class in a returned stylesheet
// (so the page needs no inline styles); optionally re-roots the Mac window to the clean rectangle.
function inPage(o) {
  const rb = new Function('return ' + o.brandFn)()(o.brand);
  const scrub = (t) => rb(t
    .replace(/[A-Za-z]:\\(?:[^\\\r\n"<>]+\\)*?zz-capture\\home\\/gi, 'C:\\Users\\Maya\\')
    .replace(/[A-Za-z]:\\Users\\[^\\\r\n"<>]+/gi, 'C:\\Users\\Maya')
    .replace(/\/(?:var|private)\/folders\/[^\s"<>]*/g, '/Users/maya/Straft')
    .replace(/\/Users\/(?!maya\b|Shared\b)[^\/\s"<>]+/g, '/Users/maya') // the Mac capture folder /Users/Shared/Straft Demo is drawn in its pictures
    .replace(/Saravan Kumar/g, 'Maya Iyer').replace(/Saravan/g, 'Maya'));
  // the transparent text layer says what scripts/mac-repairs.mjs draws in the pictures
  const macText = (t) => (o.mac ? t.replace(/Straft Demo 2(?=\/Files\/capture-packages)/g, 'Straft Demo').replace(/capture-packages/g, 'Packages').replace(/\*\*(Location|Cast):\*\*/g, '$1:') : t);
  const baseStyle = document.querySelector('head style')?.textContent || '';
  const cls = document.documentElement.className;
  // comments and capture links
  const cw = document.createTreeWalker(document, NodeFilter.SHOW_COMMENT); const comments = []; while (cw.nextNode()) comments.push(cw.currentNode);
  comments.forEach((c) => c.remove());
  document.querySelectorAll('a[data-cap-link]').forEach((a) => a.remove());
  // the Mac window is re-rooted to the clean rectangle: window shifted, wrapper clips
  let root = document.documentElement;
  if (o.mac) {
    const win = document.querySelector('.win');
    const clean = document.createElement('div'); clean.className = 'of-clean';
    clean.style.width = o.mac.full[2] + 'px'; clean.style.height = o.mac.full[3] + 'px';
    win.style.left = -o.mac.full[0] + 'px'; win.style.top = -o.mac.full[1] + 'px';
    clean.appendChild(win); root = clean;
  }
  // a name split over inline elements (the Windows wordmark draws its first half in an <i>): one text node with the new name,
  // the element and its class kept so the app CSS still sizes it
  const SPLIT = /^\s*[Oo][Pp][Ee][Nn]\s*[Ff][Rr][Aa][Mm][Ee](?:\s+[Ss]tudio)?\s*$/;
  for (const e of [...root.querySelectorAll('*')].reverse()) {
    if (!e.firstElementChild || !SPLIT.test(e.textContent)) continue;
    if ([...e.querySelectorAll('*')].every((d) => /^(I|B|EM|STRONG|SPAN|SMALL|U)$/.test(d.nodeName))) e.textContent = rb(e.textContent.replace(/\s+/g, ' ').trim());
  }
  // text and attributes
  const tw = document.createTreeWalker(root, NodeFilter.SHOW_TEXT); const texts = []; while (tw.nextNode()) texts.push(tw.currentNode);
  for (const t of texts) if (!/^(STYLE|SCRIPT)$/.test(t.parentNode.nodeName)) { const v = macText(scrub(t.nodeValue)); if (v !== t.nodeValue) t.nodeValue = v; }
  const els = [root, ...root.querySelectorAll('*')];
  const SCRUBBED = ['value', 'aria-label', 'title', 'placeholder', 'alt', 'data-id'];
  for (const e of els) for (const { name: a, value: v } of [...(e.attributes || [])]) { if (!v || a === 'style') continue; const n = SCRUBBED.includes(a) ? macText(scrub(v)) : rb(v); if (n !== v) e.setAttribute(a, n); }
  if (document.title !== rb(document.title)) document.title = rb(document.title);
  // style elements and attributes become one stylesheet
  let css = '';
  document.querySelectorAll('style').forEach((s) => { if (!o.mac) css += s.textContent + '\n'; s.remove(); });
  const map = new Map();
  for (const e of [root, ...root.querySelectorAll('*')]) {
    if (!e.hasAttribute('style')) continue;
    let st = e.getAttribute('style').trim(); e.removeAttribute('style');
    if (!st) continue;
    (o.needles || []).forEach((nd, i) => { if (st.includes(nd)) e.classList.add('of-n' + i); });
    if (o.urlTo) st = st.replace(/url\((['"]?)assets\//g, 'url($1' + o.urlTo);
    let c = map.get(st);
    if (!c) {
      c = '_s' + map.size; map.set(st, c);
      // copied as written: a browser hands back nothing for a shorthand with var(), which lost the colour swatches and some borders
      const decl = st.split(/;(?![^(]*\))/).map((d) => d.trim()).filter(Boolean).map((d) => (/!important$/i.test(d) ? d : d + ' !important'));
      css += '.' + c + '.' + c + '.' + c + '{' + decl.join(';') + '}\n';
    }
    e.classList.add(c);
  }
  if (!o.mac) {
    const link = document.createElement('link'); link.rel = 'stylesheet'; link.href = o.cssName; document.head.appendChild(link);
    document.documentElement.removeAttribute('style');
  }
  return { css: rb(css), baseStyle: rb(baseStyle), cls: rb(cls), html: rb(o.mac ? root.outerHTML : '<!doctype html>\n' + document.documentElement.outerHTML) };
}

// ---------- MAC ----------
// The Mac pages carry one picture of the window plus the accessibility frames. The capture of 2026-11-05 is clean (nothing on the
// window edges, sidebars and toolbars drawn under sheets), so every page shows the whole window as captured; the flaws it does hold are repaired by the rules in scripts/mac-repairs.mjs.
const MAC_SRC = path.join(RES, 'MacHTML-src');

async function macGeometry(page, file) {
  await page.goto(pathToFileURL(file).href);
  return page.evaluate(() => {
    const has = (k) => !!document.querySelector('.region.' + k);
    const toast = [...document.querySelectorAll('[data-id=toast]')].some((e) => e.getBoundingClientRect().width > 20);
    return { side: !!document.querySelector('[data-id=sidebar-project-title]'), sheet: has('sheet'), menu: has('menu'), popover: has('popover'), toast };
  });
}

let macBaseName = null; // shared base stylesheet, one per platform, named by content
async function macWritePage(page, theme, rec, fullR) {
  await page.goto(pathToFileURL(path.join(MAC_SRC, theme, rec.file)).href);
  const r = await page.evaluate(inPage, { ...BRAND, mac: { full: fullR }, urlTo: `../assets/${theme}/`, needles: ['line-height:16px'] });
  // the source pages shrink the window in a narrow browser; a frame is always the window's own size
  const base = r.baseStyle.replace(/\[style\*="line-height:16px"\]/g, '.of-n0').replace(/@media \(max-width:\d+px\)\{\.win\{zoom:[\d.]+\}\}\s*/g, '') + `
html,body{margin:0!important;padding:0!important;background:transparent!important}
.of-clean{position:relative;overflow:hidden}
.of-clean>.win{position:absolute!important;margin:0!important;box-shadow:none!important}
`;
  const name = 'mac-' + crypto.createHash('sha1').update(base).digest('hex').slice(0, 8) + '.css';
  if (name !== macBaseName) { writeFile(path.join(PAGES, 'mac', 'assets', name), base); macBaseName = name; }
  const stem = rec.file.replace(/\.html$/, '');
  const esc = (t) => t.replace(/&/g, '&amp;').replace(/</g, '&lt;');
  writeFile(pagePath('mac', theme, rec.file), `<!doctype html>
<html lang="en" class="${r.cls}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>${esc(rebrand(rec.title))}</title><link rel="stylesheet" href="../assets/${name}"><link rel="stylesheet" href="${stem}.css"></head><body>${r.html}</body></html>
`);
  const source = path.join(MAC_SRC, theme, rec.picture), repaired = await macRepair(rec.name, theme, source);
  // pictures are cached for a week under their name, and a new capture reuses names: the content hash in the address brings the new one at once
  const version = crypto.createHash('sha1').update(repaired || fs.readFileSync(source)).digest('hex').slice(0, 10);
  writeFile(path.join(PAGES, 'mac', theme, stem + '.css'), r.css.replace(/\.png(["')])/g, `.png?v=${version}$1`));
  const dest = path.join(PAGES, 'mac', 'assets', theme, path.basename(rec.picture));
  fs.mkdirSync(path.dirname(dest), { recursive: true });
  fs.rmSync(dest, { force: true }); // an older hard link would keep the picture of an earlier capture
  if (repaired) { fs.writeFileSync(dest, repaired); return; }
  try { fs.linkSync(source, dest); } catch { fs.copyFileSync(source, dest); }
}

async function macOne(page, rec) {
  const name = rec.name, srcTime = Math.max(mtime(path.join(MAC_SRC, 'light', rec.file)), mtime(path.join(MAC_SRC, 'dark', rec.file)));
  if (isFresh('mac', rec, srcTime)) return false;
  const g = await macGeometry(page, path.join(MAC_SRC, 'light', rec.file));
  const full = [0, 0, rec.width, rec.height];
  const focus = overrides[`mac/${name}`]?.focus || (g.sheet ? 'sheet' : g.menu ? 'menu' : g.popover || g.toast ? 'panel' : 'window');
  for (const t of THEMES) await macWritePage(page, t, rec, full);
  setEntry('mac', name, { file: rec.file, page: [full[2], full[3]], card: full, full, focus, sidebar: g.side ? 'kept' : 'collapsed' });
  return true;
}

// ---------- WINDOWS ----------
// The Windows pages are the app's own DOM, shown whole at the captured 1600x1000 with the app navigation as captured
// (owner decision 2026-11-01: full windows, nothing cropped or collapsed). The one change: on screenplay writing screens the
// Scenes list is hidden, as the app's own "Hide the scene list" button does, so the script page is the centre of the screen.
// Breakdown keeps its list: it has its own "Hide scenes" button and its own screens with the list hidden.
const WIN_SRC = path.join(RES, 'WindowsHTML-src');
const SCENES_KEPT = /navigator|scene-context|scene-number|scene-hub|reorder|export-selected/; // screens about the scene list itself

// Flaws of the Windows capture, repaired on the website page so a new capture keeps the repair. List for the app work: docs/OPERATIONS.md section 3.
const WIN_REPAIRS = [
  [/^vault-(multiselect-bar|item-context-menu-multi)$/, { css: '.vcard:has(.selc.show) .vt{padding-left:24px}' }], // the selection tick covered the first letter of the title
  [/^story-board-multiselect-bar$/, { css: '.mbar{top:6px!important;height:62px!important}' }], // the bar cut the line under the title in half
  [/^notes-task-editor-related-results$/, { css: '[role=listbox] .li.clickable .grow{min-width:45%}[role=listbox] .li.clickable .xs.muted{flex:0 1 45%;min-width:0}' }], // a long result title was squeezed to one word per line
  [/^prod-moodboard-new-filled$|^prod-shot-(menu|drawer|move-dialog|attach-dialog)$|^prod-shots-(scene-actions-menu|copy-planning-dialog|export-dialog)$/, { dropError: true }], // an error from the step before, still on screen
];
async function winRepair(page, name) {
  const dropError = WIN_REPAIRS.some(([re, r]) => r.dropError && re.test(name));
  await page.evaluate((dropError) => {
    if (dropError) document.querySelectorAll('.errtxt').forEach((e) => { e.previousElementSibling?.removeAttribute('aria-invalid'); e.remove(); });
    const tw = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    while (tw.nextNode()) { const t = tw.currentNode, v = t.nodeValue.replace(/\b1 (scene|comment|act|sequence|beat|command)s\b/g, '1 $1').replace(/\bCapture tour\b/g, 'Maya Iyer'); if (v !== t.nodeValue) t.nodeValue = v; } // the app has no singular here; the capture signed its sample themes
  }, dropError);
}
const winRepairCss = (name) => WIN_REPAIRS.filter(([re, r]) => r.css && re.test(name)).map(([, r]) => r.css + '\n').join('');

async function winPrepare(page, hideScenes, slide) {
  if (!hideScenes) return;
  await page.evaluate((slide) => {
    const list = document.querySelector('nav.sp-nav');
    if (!list || [...document.querySelectorAll('button')].some((b) => b.textContent.trim() === 'Hide scenes')) return;
    // pop-ups keep their captured screen position; the script area widens to the left, so slide the ones over it (not dialogs, not the top bars)
    const w = list.getBoundingClientRect().width, x1 = list.getBoundingClientRect().right;
    const pops = [...document.querySelectorAll('.menu-pop,[role=menu],[role=listbox]')].filter((e) => !e.closest('[role=dialog]') && e.getBoundingClientRect().y > 92 && e.getBoundingClientRect().x >= x1);
    list.remove();
    const b = document.querySelector('button[title="Hide the scene list"]');
    if (b) { b.classList.remove('on'); b.setAttribute('aria-pressed', 'false'); b.setAttribute('title', 'Show the scene list'); }
    if (slide) for (const e of pops) e.style.translate = `${-w / 2}px 0`;
  }, slide);
}
async function winGeometry(page) {
  return page.evaluate(() => {
    const R = (e) => { const b = e.getBoundingClientRect(); return { x: b.x, y: b.y, w: b.width, h: b.height }; };
    const vis = (r) => r.w > 20 && r.h > 12;
    const nav = document.querySelector('nav.nav');
    return {
      nav: nav ? { ...R(nav), collapsed: nav.classList.contains('collapsed') } : null,
      dialogs: [...document.querySelectorAll('[role=dialog],[aria-modal=true]')].map(R).filter(vis),
      menus: [...document.querySelectorAll('.menu-pop,[role=menu]')].map(R).filter(vis),
      lists: [...document.querySelectorAll('[role=listbox]')].filter((e) => !e.closest('[role=dialog]') && /pop|menu|drop|select/.test(e.className)).map(R).filter(vis), // drop-downs only, not lists that are part of the page
      toasts: [...document.querySelectorAll('.toast')].map(R).filter(vis),
      overflow: document.documentElement.scrollWidth - window.innerWidth,
    };
  });
}

async function winOne(ctx, rec) {
  const name = rec.name, srcTime = Math.max(mtime(path.join(WIN_SRC, 'light', rec.file)), mtime(path.join(WIN_SRC, 'dark', rec.file)));
  if (isFresh('win', rec, srcTime)) return false;
  const ov = overrides[`win/${name}`] || {};
  const page = await ctx.newPage();
  const url = (t) => pathToFileURL(path.join(WIN_SRC, t, rec.file)).href;
  const hideScenes = !SCENES_KEPT.test(name), slide = !/^sp-panel-/.test(name); // a menu in the right panel does not move with the script
  const size = [1600, 1000];
  try {
    await page.setViewportSize({ width: size[0], height: size[1] });
    await page.goto(url('light'));
    await winPrepare(page, hideScenes, slide);
    const geo = await winGeometry(page);
    const focus = ov.focus || (geo.dialogs.length ? 'dialog' : geo.menus.length || geo.lists.length ? 'menu' : geo.toasts.length ? 'panel' : 'window');
    const sidebar = geo.nav && !geo.nav.collapsed ? 'kept' : 'collapsed'; // 'collapsed' also covers screens without a navigation
    const stem = rec.file.replace(/\.html$/, '');
    for (const theme of THEMES) {
      await page.goto(url(theme)); await winPrepare(page, hideScenes, slide); await winRepair(page, name);
      const r = await page.evaluate(inPage, { ...BRAND, cssName: stem + '.css' });
      writeFile(pagePath('win', theme, rec.file), r.html);
      writeFile(path.join(PAGES, 'win', theme, stem + '.css'), r.css + winRepairCss(name));
    }
    setEntry('win', name, { file: rec.file, page: size, card: [0, 0, size[0], size[1]], full: [0, 0, size[0], size[1]], focus, sidebar });
  } finally { await page.close(); }
  return true;
}

// ---------- contact sheets, rendered from the cleaned pages (what the site shows) ----------
const COLS = 4, ROWS = 5, TILE = 440, LABEL = 26, TH = 340;
async function contactSheets(plat, cat, browser) {
  const dir = path.join(OUT, 'qa'); fs.mkdirSync(dir, { recursive: true });
  const names = cat.filter((c) => manifest[plat][c.name]?.page);
  const ctx = await browser.newContext({ deviceScaleFactor: 1 });
  for (const theme of THEMES) {
    const tiles = new Array(names.length);
    let next = 0;
    await Promise.all(Array.from({ length: 4 }, async () => {
      const pg = await ctx.newPage();
      while (next < names.length) {
        const i = next++; const rec = names[i]; const m = manifest[plat][rec.name];
        try {
          await pg.setViewportSize({ width: m.page[0], height: m.page[1] });
          await pg.goto(pathToFileURL(pagePath(plat, theme, rec.file)).href);
          const buf = await pg.screenshot({ clip: { x: m.card[0], y: m.card[1], width: m.card[2], height: m.card[3] } });
          tiles[i] = await sharp(buf).resize({ width: TILE - 8, height: TH, fit: 'inside' }).jpeg({ quality: 82 }).toBuffer();
        } catch (err) { console.error('QA FAILED', rec.name, err.message); }
      }
      await pg.close();
    }));
    const per = COLS * ROWS;
    for (let s = 0; s * per < names.length; s++) {
      const comps = [];
      for (let i = 0; i < per; i++) {
        const idx = s * per + i; if (idx >= names.length || !tiles[idx]) continue;
        const rec = names[idx], m = manifest[plat][rec.name]; const col = i % COLS, row = Math.floor(i / COLS);
        const label = `${String(idx + 1).padStart(3, '0')} ${rec.name}`.slice(0, 54) + `  ${m.focus}/${m.sidebar}`;
        const svg = Buffer.from(`<svg xmlns="http://www.w3.org/2000/svg" width="${TILE}" height="${LABEL}"><text x="4" y="17" font-family="Arial" font-size="12" fill="#222">${label.replace(/&/g, '&amp;').replace(/</g, '&lt;')}</text></svg>`);
        comps.push({ input: tiles[idx], left: col * TILE + 4, top: row * (TH + LABEL) + 4 });
        comps.push({ input: svg, left: col * TILE, top: row * (TH + LABEL) + TH + 4 });
      }
      await sharp({ create: { width: COLS * TILE, height: ROWS * (TH + LABEL) + 8, channels: 3, background: '#d8d8dc' } }).composite(comps).jpeg({ quality: 80 }).toFile(path.join(dir, `${plat}-${theme}-${String(s + 1).padStart(2, '0')}.jpg`));
    }
  }
  await ctx.close();
}

// ---------- main ----------
async function pool(items, n, fn) {
  let i = 0, done = 0;
  await Promise.all(Array.from({ length: n }, async (_, w) => {
    while (i < items.length) { const it = items[i++]; try { await fn(it, w); } catch (err) { console.error('FAILED', it.name, err.message); } if (++done % 100 === 0) console.log(`  ${done}/${items.length}`); }
  }));
}

const browser = await chromium.launch();
try {
  if (ONLY !== 'win') {
    const all = readJson(path.join(RES, 'screens-mac.json'), []).filter((r) => !r.sim); // simulated screens come from sim-mac.mjs
    const cat = all.filter((r) => !MATCH || MATCH.test(r.name));
    console.log(`mac: ${cat.length} screens`);
    const pages = await Promise.all(Array.from({ length: 4 }, async () => (await browser.newContext({ viewport: { width: 1800, height: 1000 } })).newPage()));
    let made = 0;
    await pool(cat, 4, async (rec, w) => { if (await macOne(pages[w], rec)) made++; if (made % 25 === 0) saveManifest(); });
    saveManifest();
    console.log(`mac: ${made} made, ${cat.length - made} up to date`);
    if (QA && !MATCH) await contactSheets('mac', all, browser);
  }
  if (ONLY !== 'mac') {
    const all = readJson(path.join(RES, 'screens-win.json'), []);
    const cat = all.filter((r) => !MATCH || MATCH.test(r.name));
    console.log(`win: ${cat.length} screens`);
    fs.mkdirSync(path.join(PAGES, 'win', 'assets'), { recursive: true });
    for (const f of fs.readdirSync(path.join(WIN_SRC, 'assets'))) { // text assets (the app CSS: theme selectors, content strings) get the product name
      const src = path.join(WIN_SRC, 'assets', f), d = path.join(PAGES, 'win', 'assets', f);
      if (/\.(css|js|mjs|json|svg|html?|txt|md)$/i.test(f)) { const t = rebrand(fs.readFileSync(src, 'utf8')); if (!fs.existsSync(d) || fs.readFileSync(d, 'utf8') !== t) writeFile(d, t); }
      else if (!fs.existsSync(d)) fs.copyFileSync(src, d);
    }
    const ctxs = await Promise.all(Array.from({ length: 3 }, () => browser.newContext({ viewport: { width: 1600, height: 1000 }, deviceScaleFactor: 1 })));
    let made = 0;
    await pool(cat, 3, async (rec, w) => { if (await winOne(ctxs[w], rec)) made++; if (made % 25 === 0) saveManifest(); });
    saveManifest();
    console.log(`win: ${made} made, ${cat.length - made} up to date`);
    if (QA && !MATCH) await contactSheets('win', all, browser);
  }
} finally { await browser.close(); saveManifest(); if (leftovers) console.error(`${leftovers} written files still hold the old product name`); }
