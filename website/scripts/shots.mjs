// Cleaned, interactive screen pages for the website cards and viewer.
//   node scripts/shots.mjs            both platforms (Mac first)
//   ONLY=mac|win  one platform         FORCE=1  redo everything         MATCH=regex  only screens whose name matches
//   QA=0  skip the contact sheets
// Output, in .cache/shots/pages/<win|mac>/:
//   <light|dark>/<file>.html + <file>.css   the screen, same file names as the catalog "file"; no inline styles or scripts
//   assets/                                 shared per platform: Windows app CSS, Mac window pictures (hard links to the 2x PNGs), Mac base CSS
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
const stamp = Math.max(mtime(OVR), mtime(import.meta.filename));
const pagePath = (plat, theme, file) => path.join(PAGES, plat, theme, file);
const isFresh = (plat, rec, srcTime) => !FORCE && !!manifest[plat][rec.name]?.page && THEMES.every((t) => mtime(pagePath(plat, t, rec.file)) >= Math.max(srcTime, stamp));
const round = (r) => r.map((v) => Math.round(v));
const writeFile = (f, text) => { fs.mkdirSync(path.dirname(f), { recursive: true }); fs.writeFileSync(f, text); };

// ---------- the page cleaner, run inside the browser (self-contained: Playwright ships its source) ----------
// Strips comments, capture links, paths and account names; moves every style attribute into a class in a returned stylesheet
// (so the page needs no inline styles); optionally re-roots the Mac window to the clean rectangle.
function inPage(o) {
  const scrub = (t) => t
    .replace(/[A-Za-z]:\\(?:[^\\\r\n"<>]+\\)*?zz-capture\\home\\/gi, 'C:\\Users\\Maya\\')
    .replace(/[A-Za-z]:\\Users\\[^\\\r\n"<>]+/gi, 'C:\\Users\\Maya')
    .replace(/\/(?:var|private)\/folders\/[^\s"<>]*/g, '/Users/maya/Straft')
    .replace(/\/Users\/(?!maya\b)[^\/\s"<>]+/g, '/Users/maya')
    .replace(/Saravan Kumar/g, 'Maya Iyer').replace(/Saravan/g, 'Maya');
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
  // text and attributes
  const tw = document.createTreeWalker(root, NodeFilter.SHOW_TEXT); const texts = []; while (tw.nextNode()) texts.push(tw.currentNode);
  for (const t of texts) if (!/^(STYLE|SCRIPT)$/.test(t.parentNode.nodeName)) { const v = scrub(t.nodeValue); if (v !== t.nodeValue) t.nodeValue = v; }
  const els = [root, ...root.querySelectorAll('*')];
  for (const e of els) for (const a of ['value', 'aria-label', 'title', 'placeholder', 'alt', 'data-id']) { const v = e.getAttribute && e.getAttribute(a); if (v) { const n = scrub(v); if (n !== v) e.setAttribute(a, n); } }
  // style elements and attributes become one stylesheet
  let css = '';
  document.querySelectorAll('style').forEach((s) => { if (!o.mac) css += s.textContent + '\n'; s.remove(); });
  const map = new Map(); const tmp = document.createElement('div');
  for (const e of [root, ...root.querySelectorAll('*')]) {
    if (!e.hasAttribute('style')) continue;
    let st = e.getAttribute('style').trim(); e.removeAttribute('style');
    if (!st) continue;
    (o.needles || []).forEach((nd, i) => { if (st.includes(nd)) e.classList.add('of-n' + i); });
    if (o.urlTo) st = st.replace(/url\((['"]?)assets\//g, 'url($1' + o.urlTo);
    let c = map.get(st);
    if (!c) {
      c = '_s' + map.size; map.set(st, c);
      tmp.style.cssText = st; const decl = [];
      for (let i = 0; i < tmp.style.length; i++) { const n = tmp.style[i]; decl.push(n + ':' + tmp.style.getPropertyValue(n) + ' !important'); }
      css += '.' + c + '.' + c + '.' + c + '{' + decl.join(';') + '}\n';
    }
    e.classList.add(c);
  }
  if (!o.mac) {
    const link = document.createElement('link'); link.rel = 'stylesheet'; link.href = o.cssName; document.head.appendChild(link);
    document.documentElement.removeAttribute('style');
  }
  return { css, baseStyle, cls, html: o.mac ? root.outerHTML : '<!doctype html>\n' + document.documentElement.outerHTML };
}

// ---------- MAC ----------
// The Mac pages carry one 2x picture of the window plus the accessibility frames. Picture flaws handled here: capture strips on the
// edges and OS rounded corners (trimmed: the page window is the clean rectangle of the picture), and blank sidebars (not drawn while a
// sheet is up), which are painted in from the nearest screen of the same area with a drawn sidebar, shaded to match. Nothing else is cut.
const MAC_SRC = path.join(RES, 'MacHTML-src');
let MAC_ALL = []; // the whole Mac catalog, for sidebar donors

async function loadRaw(file) {
  const { data, info } = await sharp(file).ensureAlpha().raw().toBuffer({ resolveWithObject: true });
  const L = new Float32Array(info.width * info.height);
  for (let i = 0, j = 0; i < L.length; i++, j += 4) L[i] = (data[j] + data[j + 1] + data[j + 2]) / 3;
  return { L, W: info.width, H: info.height, data };
}
const median = (a) => { const s = Float32Array.from(a).sort(); return s[s.length >> 1]; };

// Width in src px of the capture strip on one edge (desktop bleed, black or coloured noise): walk in from the edge while the
// line is junk, that is strongly coloured or near black and unlike the interior. A line that only differs from the interior
// (a toolbar, a banner, a panel) is window and stops the walk, so the window itself is never cut.
// over: src px rectangles [x0, y0, x1, y1] (sheets, menus) left out of the samples, since they are not the window's interior.
function stripDepth(p, side, over = [], dark = false) {
  const { W, H, data } = p; const maxK = 220, tol = 12;
  const lineAt = (k) => { // per-channel median of the line, so a dark strip next to a dark page is still told apart by its colour
    const ch = [[], [], []]; let seen = 0;
    const put = (x, y) => { if (over.some((o) => x >= o[0] && x < o[2] && y >= o[1] && y < o[3])) return; seen++; const i = (y * W + x) * 4; if (data[i + 3] < 200) return; ch[0].push(data[i]); ch[1].push(data[i + 1]); ch[2].push(data[i + 2]); }; // see-through pixels carry no colour
    if (side === 'left' || side === 'right') { const x = side === 'left' ? k : W - 1 - k; for (let y = 160; y < H - 160; y += 3) put(x, y); }
    else { const y = side === 'top' ? k : H - 1 - k; for (let x = 420; x < W - 420; x += 3) put(x, y); }
    return ch[0].length * 2 < seen ? [NaN, NaN, NaN] : ch.map(median); // mostly see-through: not a line to judge
  };
  const ref = lineAt(maxK + 40);
  const near = (k) => { const v = lineAt(k); return Math.max(Math.abs(v[0] - ref[0]), Math.abs(v[1] - ref[1]), Math.abs(v[2] - ref[2])) <= tol; };
  const junk = (k) => { const v = lineAt(k); if (v.some(Number.isNaN)) return false; const mx = Math.max(...v), mn = Math.min(...v); return (mx > 60 && (mx - mn) / mx > 0.5) || (!dark && mx < 60); }; // a dark window is itself near black
  // the junk can have thin clean lines between its bands, so up to 6 of them are stepped over
  let last = 0;
  for (let k = 0; k < maxK && !near(k); k++) { if (junk(k)) last = k + 1; else if (k - last >= 6) break; }
  return last;
}
// Share of sampled pixels in a src px box with a sharp step to their neighbour: text and icons. A drawn sidebar scores about
// 0.03 and a drawn toolbar row about 0.05; an undrawn one is near zero even when its fill is a gradient.
// skip(x, y) leaves a pixel out (a sheet or menu over the box).
function detail(p, x0, x1, y0, y1, skip) {
  const { L, W } = p; let n = 0, hit = 0;
  for (let y = Math.round(y0); y < y1; y += 2) for (let x = Math.max(2, Math.round(x0)); x < x1; x += 2) { if (skip?.(x, y)) continue; n++; if (Math.abs(L[y * W + x] - L[y * W + x - 2]) > 40) hit++; }
  return n ? hit / n : 0;
}
const sideDetail = (p, w) => detail(p, 24, w - 24, 130, p.H - 60);

async function macGeometry(page, file) {
  await page.goto(pathToFileURL(file).href);
  return page.evaluate(() => {
    const w = document.querySelector('.win').getBoundingClientRect();
    const rel = (e) => { const b = e.getBoundingClientRect(); return { x: b.x - w.x, y: b.y - w.y, w: b.width, h: b.height }; };
    const tog = document.querySelector('[data-id=sidebar-toggle]');
    const regs = {};
    for (const e of document.querySelectorAll('.region')) { const c = ['sheet', 'menu', 'popover'].find((k) => e.classList.contains(k)); if (c) (regs[c] ||= []).push(rel(e)); }
    const toast = [...document.querySelectorAll('[data-id=toast]')].map(rel).filter((r) => r.w > 20);
    return { W: w.width, H: w.height, side: !!document.querySelector('[data-id=sidebar-project-title]'), tog: tog ? rel(tog).x : null, regs, toast: toast.length > 0 };
  });
}

let macBaseName = null; // shared base stylesheet, one per platform, named by content
async function macWritePage(page, theme, rec, fullR, patched) {
  await page.goto(pathToFileURL(path.join(MAC_SRC, theme, rec.file)).href);
  const r = await page.evaluate(inPage, { mac: { full: fullR }, urlTo: `../assets/${theme}/`, needles: ['line-height:16px'] });
  const base = r.baseStyle.replace(/\[style\*="line-height:16px"\]/g, '.of-n0') + `
html,body{margin:0!important;padding:0!important;background:transparent!important}
.of-clean{position:relative;overflow:hidden}
.of-clean>.win{position:absolute!important;margin:0!important;border-radius:0!important;box-shadow:none!important}
`;
  const name = 'mac-' + crypto.createHash('sha1').update(base).digest('hex').slice(0, 8) + '.css';
  if (name !== macBaseName) { writeFile(path.join(PAGES, 'mac', 'assets', name), base); macBaseName = name; }
  const stem = rec.file.replace(/\.html$/, '');
  const esc = (t) => t.replace(/&/g, '&amp;').replace(/</g, '&lt;');
  writeFile(pagePath('mac', theme, rec.file), `<!doctype html>
<html lang="en" class="${r.cls}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>${esc(rec.title)}</title><link rel="stylesheet" href="../assets/${name}"><link rel="stylesheet" href="${stem}.css"></head><body>${r.html}</body></html>
`);
  writeFile(path.join(PAGES, 'mac', theme, stem + '.css'), r.css);
  const pic = path.basename(rec.picture), dest = path.join(PAGES, 'mac', 'assets', theme, pic);
  fs.mkdirSync(path.dirname(dest), { recursive: true });
  // the asset is a hard link to the source picture, or a repaired copy: remove the old one first so writing never reaches the source
  if (patched) { fs.rmSync(dest, { force: true }); fs.writeFileSync(dest, patched); }
  else if (!fs.existsSync(dest) || fs.statSync(dest).nlink < 2) { fs.rmSync(dest, { force: true }); try { fs.linkSync(path.join(MAC_SRC, theme, rec.picture), dest); } catch { fs.copyFileSync(path.join(MAC_SRC, theme, rec.picture), dest); } }
}

// Paint a drawn sidebar into a picture whose sidebar is blank. The donor is the nearest catalog screen (same area first) with the same
// picture size and a cleanly drawn sidebar and toolbar; under a sheet its pixels are dimmed to match the window.
async function fillSidebar(rec, theme, raw, sideW, overlays, dims) {
  const w = Math.round(sideW * raw.W / rec.width);
  const idx = MAC_ALL.findIndex((r) => r.name === rec.name);
  const near = MAC_ALL.map((r, i) => ({ r, d: Math.abs(i - idx) + (r.area === rec.area ? 0 : 10000) + (i > idx ? 0.5 : 0) }))
    .filter(({ r }) => r !== rec && r.width === rec.width && r.height === rec.height && (w === 0 ? r.area === rec.area && !r.texts?.includes('Sidebar') : r.texts?.includes('Sidebar'))).sort((a, b) => a.d - b.d);
  const s = raw.W / rec.width, TB = Math.round(50 * s); // the first toolbar row ends about 50 CSS px down
  const over = overlays.map((o) => [(o.x - 30) * s, (o.y - 30) * s, (o.x + o.w + 30) * s, (o.y + o.h + 30) * s]); // a sheet or menu, with its shadow
  const isOver = (x, y) => over.some((o) => x >= o[0] && x < o[2] && y >= o[1] && y < o[3]);
  // dominant colour of a region, in 4-step buckets
  const mode = (p, x0, x1, y0, y1, skip) => { const h = new Map(); for (let y = y0; y < y1; y += 3) for (let x = x0; x < x1; x += 3) { if (skip && isOver(x, y)) continue; const i = (y * p.W + x) * 4, k = `${p.data[i] >> 2},${p.data[i + 1] >> 2},${p.data[i + 2] >> 2}`; h.set(k, (h.get(k) || 0) + 1); } return [...h].sort((a, b) => b[1] - a[1])[0][0].split(',').map((v) => +v * 4 + 2); };
  const inSheet = (x, y) => overlays.some((o) => x >= o.x * s && x < (o.x + o.w) * s && y >= o.y * s && y < (o.y + o.h) * s);
  const toolbar = (p) => detail(p, w + 2, raw.W - 60, 4, TB, inSheet);
  for (const { r } of near) {
    const d = await loadRaw(path.join(MAC_SRC, theme, r.picture));
    if (d.W !== raw.W || d.H !== raw.H || (w > 0 && sideDetail(d, w) < 0.015) || toolbar(d) < (w === 0 ? 0.012 : 0.035)) continue; // the donor must be a clean capture
    // shade: a sheet dims the window, so scale the donor by the ratio of the two windows' main colours (sheet excluded)
    const box = [w + 40, raw.W - 40, Math.round(120 * s), raw.H - Math.round(60 * s)];
    const m1 = mode(raw, ...box, true), m0 = mode(d, ...box, true), sum = (v) => v[0] + v[1] + v[2];
    // one factor, so no colour cast; only a sheet dims the window, never below about half
    const k = Array(3).fill(dims ? Math.min(1, Math.max(0.45, sum(m1) / Math.max(1, sum(m0)))) : 1);
    const out = Buffer.from(raw.data);
    const put = (x, y) => { const i = (y * raw.W + x) * 4; for (let c = 0; c < 3; c++) out[i + c] = Math.min(255, Math.round(d.data[i + c] * k[c])); };
    // sidebar column: replaced whole, except under a sheet or menu drawn over it
    for (let y = 0; y < raw.H; y++) for (let x = 0; x < w; x++) if (!inSheet(x, y)) put(x, y);
    // toolbar: some captures also drew its buttons without labels; then the whole first toolbar row comes from the donor
    if (toolbar(raw) < (w === 0 ? 0.006 : 0.025)) for (let y = 0; y < TB; y++) for (let x = w; x < raw.W - (w === 0 ? 4 : 30) * s; x++) if (!inSheet(x, y)) put(x, y);
    return { png: await sharp(out, { raw: { width: raw.W, height: raw.H, channels: 4 } }).png({ compressionLevel: 9 }).toBuffer(), donor: r.name, changed: w > 0 || toolbar(raw) < 0.006 };
  }
  return null;
}

async function macOne(page, rec) {
  const name = rec.name, srcTime = Math.max(mtime(path.join(MAC_SRC, 'light', rec.file)), mtime(path.join(MAC_SRC, 'dark', rec.file)));
  if (isFresh('mac', rec, srcTime)) return false;
  const ov = overrides[`mac/${name}`] || {};
  const g = await macGeometry(page, path.join(MAC_SRC, 'light', rec.file));
  const pics = await Promise.all(THEMES.map(async (t) => ({ t, raw: await loadRaw(path.join(MAC_SRC, t, rec.picture)) })));
  const lp = pics[0].raw, s = lp.W / g.W;
  // 1. sidebar: a real one is kept; a blank one (not drawn in the capture) is painted in from a donor screen
  const hasSide = g.side && g.tog > 150;
  let sideW = 0, sidebar = hasSide ? 'kept' : 'collapsed';
  const patched = {};
  if (hasSide) {
    const x0 = g.tog - 28, x1 = g.tog; let best = g.tog - 4, bestF = 0.5;
    const ys = []; for (let y = 130; y < lp.H - 120; y += 2) ys.push(y);
    for (let xc = Math.round(x0 * s); xc <= Math.round(x1 * s); xc++) { let hit = 0; for (const y of ys) if (Math.abs(lp.L[y * lp.W + xc + 1] - lp.L[y * lp.W + xc - 1]) > 3) hit++; const f = hit / ys.length; if (f > bestF) { bestF = f; best = xc / s; } }
    sideW = best;
    for (const p of pics) {
      if (sideDetail(p.raw, sideW * s) >= 0.005) continue; // drawn: it has text
      const f = await fillSidebar(rec, p.t, p.raw, sideW + 2, Object.values(g.regs).flat(), !!g.regs.sheet?.length);
      if (!f) throw new Error(`blank sidebar and no donor screen: ${p.t}/${name}`);
      patched[p.t] = f.png; p.raw = await loadRaw(f.png); sidebar = `filled from ${f.donor}`;
    }
  }
  // a window without a sidebar (Application Home and its sheets) can also have a toolbar the capture drew as empty pills
  const tbd = (p) => detail(p, Math.round(300 * s), p.W - Math.round(60 * s), Math.round(10 * s), Math.round(46 * s));
  if (!hasSide && g.W >= 1200) for (const p of pics) {
    if (tbd(p.raw) >= 0.005 || Math.max(...pics.map((q) => tbd(q.raw))) < 0.012) continue; // empty in this theme only: a screen with no toolbar by design is left alone
    const f = await fillSidebar(rec, p.t, p.raw, 0, Object.values(g.regs).flat(), !!g.regs.sheet?.length);
    if (f && f.changed) { patched[p.t] = f.png; p.raw = await loadRaw(f.png); sidebar = `collapsed, toolbar filled from ${f.donor}`; }
  }
  // 2. clean inner rectangle: a few px inside the window, past any capture strip
  const e = { l: 6, t: 6, r: 6, b: 6 };
  // a sheet or menu reaches from near the top, so it is left out of the samples (with its shadow, and in full height)
  const over = Object.values(g.regs).flat().map((o) => [(o.x - 30) * s, 0, (o.x + o.w + 30) * s, lp.H]);
  for (const p of pics) for (const [k, side] of [['l', 'left'], ['t', 'top'], ['r', 'right'], ['b', 'bottom']]) { const d = stripDepth(p.raw, side, side === 'top' || side === 'bottom' ? over : [], p.t === 'dark'); e[k] = Math.max(e[k], Math.ceil(d / s) + 2); }
  const full = round([e.l, e.t, g.W - e.r - e.l, g.H - e.b - e.t]); // the clean window, whole
  const focus = ov.focus || (g.regs.sheet ? 'sheet' : g.regs.menu ? 'menu' : g.regs.popover || g.toast ? 'panel' : 'window');
  for (const t of THEMES) await macWritePage(page, t, rec, full, patched[t]);
  setEntry('mac', name, { file: rec.file, page: [full[2], full[3]], card: [0, 0, full[2], full[3]], full: [0, 0, full[2], full[3]], focus, sidebar });
  return true;
}

// ---------- WINDOWS ----------
// The Windows pages are the app's own DOM, shown whole at the captured 1600x1000 with the app navigation as captured
// (owner decision 2026-11-01: full windows, nothing cropped or collapsed). The one change: on screenplay writing screens the
// Scenes list is hidden, as the app's own "Hide the scene list" button does, so the script page is the centre of the screen.
const WIN_SRC = path.join(RES, 'WindowsHTML-src');
const SCENES_KEPT = /navigator|scene-context|scene-number|scene-hub|reorder|export-selected/; // screens about the scene list itself

async function winPrepare(page, hideScenes) {
  if (!hideScenes) return;
  await page.evaluate(() => {
    const list = document.querySelector('nav.sp-nav');
    if (!list) return;
    // pop-ups keep their captured screen position; the script area widens to the left, so slide the ones over it (not dialogs, not the top bars)
    const w = list.getBoundingClientRect().width, x1 = list.getBoundingClientRect().right;
    const pops = [...document.querySelectorAll('.menu-pop,[role=menu],[role=listbox]')].filter((e) => !e.closest('[role=dialog]') && e.getBoundingClientRect().y > 92 && e.getBoundingClientRect().x >= x1);
    list.remove();
    const b = document.querySelector('button[title="Hide the scene list"]');
    if (b) { b.classList.remove('on'); b.setAttribute('aria-pressed', 'false'); b.setAttribute('title', 'Show the scene list'); }
    for (const e of pops) e.style.translate = `${-w / 2}px 0`;
  });
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
  const hideScenes = !SCENES_KEPT.test(name);
  const size = [1600, 1000];
  try {
    await page.setViewportSize({ width: size[0], height: size[1] });
    await page.goto(url('light'));
    await winPrepare(page, hideScenes);
    const geo = await winGeometry(page);
    const focus = ov.focus || (geo.dialogs.length ? 'dialog' : geo.menus.length || geo.lists.length ? 'menu' : geo.toasts.length ? 'panel' : 'window');
    const sidebar = geo.nav && !geo.nav.collapsed ? 'kept' : 'collapsed'; // 'collapsed' also covers screens without a navigation
    const stem = rec.file.replace(/\.html$/, '');
    for (const theme of THEMES) {
      await page.goto(url(theme)); await winPrepare(page, hideScenes);
      const r = await page.evaluate(inPage, { cssName: stem + '.css' });
      writeFile(pagePath('win', theme, rec.file), r.html);
      writeFile(path.join(PAGES, 'win', theme, stem + '.css'), r.css);
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
    MAC_ALL = all;
    const cat = all.filter((r) => !MATCH || MATCH.test(r.name));
    console.log(`mac: ${cat.length} screens`);
    const pages = await Promise.all(Array.from({ length: 4 }, async () => (await browser.newContext({ viewport: { width: 1500, height: 900 } })).newPage()));
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
    for (const f of fs.readdirSync(path.join(WIN_SRC, 'assets'))) { const d = path.join(PAGES, 'win', 'assets', f); if (!fs.existsSync(d)) fs.copyFileSync(path.join(WIN_SRC, 'assets', f), d); }
    const ctxs = await Promise.all(Array.from({ length: 3 }, () => browser.newContext({ viewport: { width: 1600, height: 1000 }, deviceScaleFactor: 1 })));
    let made = 0;
    await pool(cat, 3, async (rec, w) => { if (await winOne(ctxs[w], rec)) made++; if (made % 25 === 0) saveManifest(); });
    saveManifest();
    console.log(`win: ${made} made, ${cat.length - made} up to date`);
    if (QA && !MATCH) await contactSheets('win', all, browser);
  }
} finally { await browser.close(); saveManifest(); }
