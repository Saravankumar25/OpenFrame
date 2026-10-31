// Clean, spotlighted pictures of every app screen for the website cards.
//   node scripts/shots.mjs            both platforms (Mac first)
//   ONLY=mac|win  one platform         FORCE=1  redo everything         MATCH=regex  only screens whose name matches
//   QA=0  skip the contact sheets
// Output per screen and theme, in .cache/shots/<win|mac>/<light|dark>/:
//   <name>.card-720.webp, <name>.card-1280.webp   spotlighted crop, 4:3 to 16:9, never upscaled
//   <name>.full.webp                              the whole cleaned window, at most 2400 px wide
// .cache/shots/manifest.json lists card and full sizes, focus and sidebar state. Contact sheets: .cache/shots/qa/.
// <name> is the catalog name (research/screens-*.json "name"), the key the site already uses.
// Crops are computed from the light capture and reused for dark. src/content/shot-overrides.json wins last:
//   "mac/<name>": { "crop": [x, y, w, h] (CSS px of the window), "sidebar": "keep|crop", "focus": "..." }
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium } from 'playwright';
import sharp from 'sharp';

const ROOT = path.resolve(import.meta.dirname, '..');
const RES = path.join(ROOT, 'research');
const OUT = path.join(ROOT, '.cache', 'shots');
const MANIFEST = path.join(OUT, 'manifest.json');
const OVR = path.join(ROOT, 'src', 'content', 'shot-overrides.json');
const FORCE = !!process.env.FORCE;
const ONLY = process.env.ONLY || '';
const MATCH = process.env.MATCH ? new RegExp(process.env.MATCH) : null;
const QA = process.env.QA !== '0';
const THEMES = ['light', 'dark'];
const WEBP = { quality: 80, effort: 4 };
const MAX_FULL = 2400;
const LEGIBLE = 1150; // CSS px; wider sources get small text at card size
const A_MIN = 4 / 3, A_MAX = 16 / 9;

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
const outPath = (plat, theme, name, kind) => path.join(OUT, plat, theme, `${name}.${kind}.webp`);
const KINDS = ['card-720', 'card-1280', 'full'];
const isFresh = (plat, name, srcTime) => !FORCE && !!manifest[plat][name] && THEMES.every((t) => KINDS.every((k) => mtime(outPath(plat, t, name, k)) >= Math.max(srcTime, stamp)));
const clamp = (v, a, b) => Math.min(Math.max(v, a), b);
const round = (r) => r.map((v) => Math.round(v));

// ---------- geometry helpers (CSS px) ----------
// Smallest box around the subject S, plus roughly 12 percent of the window per side, widened or heightened to an
// aspect between 4:3 and 16:9, never smaller than half the window width, and kept inside the bounds B = [x0,y0,x1,y1].
function fitCrop(S, B, win) {
  const mx = 0.12 * win.w, my = 0.12 * win.h, minW = 0.5 * win.w;
  const bw = B[2] - B[0], bh = B[3] - B[1];
  let w = Math.max(S.w + 2 * mx, minW), h = S.h + 2 * my;
  if (w > LEGIBLE && S.w + 48 <= LEGIBLE) w = Math.max(LEGIBLE, S.w + 48);
  h = clamp(h, w * 9 / 16, w * 3 / 4);
  if (h < S.h + 48) { h = S.h + 48; w = Math.max(w, h * 4 / 3); }
  if (w > bw) { w = bw; h = Math.min(Math.max(h, w * 9 / 16), w * 3 / 4); }
  if (h > bh) { h = bh; w = Math.min(bw, Math.max(w, h * 4 / 3)); w = Math.min(w, h * 16 / 9); }
  const cx = S.x + S.w / 2, cy = S.y + S.h / 2;
  const x = clamp(cx - w / 2, B[0], B[2] - w), y = clamp(cy - h / 2, B[1], B[3] - h);
  return [x, y, w, h];
}
// Box covering a list of {x,y,w,h}, clamped to bounds.
function union(list, B) {
  const x0 = Math.max(B[0], Math.min(...list.map((r) => r.x))), y0 = Math.max(B[1], Math.min(...list.map((r) => r.y)));
  const x1 = Math.min(B[2], Math.max(...list.map((r) => r.x + r.w))), y1 = Math.min(B[3], Math.max(...list.map((r) => r.y + r.h)));
  return { x: x0, y: y0, w: Math.max(1, x1 - x0), h: Math.max(1, y1 - y0) };
}

// ---------- webp output ----------
async function writeSet(plat, theme, name, srcBuf, srcScale, crop, fullRect) {
  // crop and fullRect are CSS rects of the source; srcScale is source px per CSS px.
  const px = (r) => { const l = Math.round(r[0] * srcScale), t = Math.round(r[1] * srcScale); return { left: l, top: t, width: Math.round(r[2] * srcScale), height: Math.round(r[3] * srcScale) }; };
  const dir = path.join(OUT, plat, theme); fs.mkdirSync(dir, { recursive: true });
  const info = {};
  const dec = await sharp(srcBuf).raw().toBuffer({ resolveWithObject: true });
  const fit = (r) => { const left = Math.min(Math.max(0, r.left), dec.info.width - 2), top = Math.min(Math.max(0, r.top), dec.info.height - 2); return { left, top, width: Math.max(2, Math.min(r.width, dec.info.width - left)), height: Math.max(2, Math.min(r.height, dec.info.height - top)) }; };
  const cardRect = fit(px(crop)), fullR = fit(px(fullRect));
  const src = () => sharp(dec.data, { raw: dec.info });
  const cardBuf = await src().extract(cardRect).raw().toBuffer({ resolveWithObject: true });
  for (const [kind, wmax] of [['card-1280', 1280], ['card-720', 720]]) {
    const w = Math.min(wmax, cardRect.width);
    const r = await sharp(cardBuf.data, { raw: cardBuf.info }).resize({ width: w, withoutEnlargement: true }).webp(WEBP).toFile(outPath(plat, theme, name, kind));
    if (kind === 'card-1280') info.card = [r.width, r.height];
  }
  const fw = Math.min(MAX_FULL, fullR.width);
  const r = await src().extract(fullR).resize({ width: fw, withoutEnlargement: true }).webp(WEBP).toFile(outPath(plat, theme, name, 'full'));
  info.full = [r.width, r.height];
  return info;
}

// ---------- MAC ----------
// The Mac pages carry one 2x picture of the window plus the accessibility frames. Picture flaws handled here:
// capture strips on the edges, blank sidebars (not drawn while a sheet is up), OS rounded corners, and layouts wider than 16:9.
const MAC_SRC = path.join(RES, 'MacHTML-src');
const MAC_NAV = /sidebar|^project-home(-empty|-after-back)?$/;
const MAC_BAR = /toolbar|sidebar|assistant|search|menu/;
const PANEL_NAME = /comment|panel|inspector|assistant|^ai-|drawer/;

async function loadRaw(file) {
  const { data, info } = await sharp(file).ensureAlpha().raw().toBuffer({ resolveWithObject: true });
  const L = new Float32Array(info.width * info.height);
  for (let i = 0, j = 0; i < L.length; i++, j += 4) L[i] = (data[j] + data[j + 1] + data[j + 2]) / 3;
  return { L, W: info.width, H: info.height, data };
}
const median = (a) => { const s = Float32Array.from(a).sort(); return s[s.length >> 1]; };

// Width in src px of the capture strip on one edge (shadow, desktop bleed, black or red noise): walk in from the edge
// until the picture looks like its interior again. 0 when the edge already matches the interior.
function stripDepth(p, side) {
  const { W, H, data } = p; const maxK = 220, tol = 12;
  const lineAt = (k) => { // per-channel median of the line, so a dark strip next to a dark page is still told apart by its colour
    const ch = [[], [], []];
    const put = (x, y) => { const i = (y * W + x) * 4; ch[0].push(data[i]); ch[1].push(data[i + 1]); ch[2].push(data[i + 2]); };
    if (side === 'left' || side === 'right') { const x = side === 'left' ? k : W - 1 - k; for (let y = 160; y < H - 160; y += 3) put(x, y); }
    else { const y = side === 'top' ? k : H - 1 - k; for (let x = 420; x < W - 420; x += 3) put(x, y); }
    return ch.map(median);
  };
  const ref = lineAt(maxK + 40);
  const near = (k) => { const v = lineAt(k); return Math.max(Math.abs(v[0] - ref[0]), Math.abs(v[1] - ref[1]), Math.abs(v[2] - ref[2])) <= tol; };
  if (near(0) && near(1) && near(2)) return 0;
  for (let k = 3; k < maxK; k++) if (near(k) && near(k + 1) && near(k + 2)) return k;
  return 0;
}
// Fraction of the sidebar area that is not the dominant colour; near zero means the sidebar was not drawn.
function sidebarInk(p, sw) {
  const { L, W, H } = p; const hist = new Map(); let n = 0;
  for (let y = 130; y < H - 60; y += 2) for (let x = 24; x < Math.max(60, sw - 24); x += 2) { const k = L[y * W + x] >> 1; hist.set(k, (hist.get(k) || 0) + 1); n++; }
  return 1 - Math.max(...hist.values()) / n;
}
// Columns (CSS px) where a vertical edge runs through most of the body: panel dividers.
function dividers(p, s, y0, y1) {
  const { L, W } = p; const out = []; const ys = []; for (let y = Math.round(y0 * s); y < Math.round(y1 * s); y += 2) ys.push(y);
  for (let x = 6; x < W - 6; x++) {
    let hit = 0; for (const y of ys) if (Math.abs(L[y * W + x + 2] - L[y * W + x - 2]) > 4) hit++;
    if (hit / ys.length > 0.6) out.push(x / s);
  }
  const m = []; for (const c of out) { if (m.length && c - m[m.length - 1].b < 3) m[m.length - 1].b = c; else m.push({ a: c, b: c }); }
  return m.map((g) => g.a - 3 / s); // the text of the next panel can start right at the rule, so cut on its near side
}
// Runs [a,b] (CSS px) of columns (axis 'x') or rows (axis 'y') that nothing runs through inside the span [c0,c1] of the
// other axis: blank gutters and hairline rules, where a cut loses no text.
function clearRuns(p, s, axis, c0, c1) {
  const { L, W, H } = p; const runs = []; let a = -1;
  const n = axis === 'x' ? W : H; const cs = []; for (let c = Math.max(4, Math.round(c0 * s)); c < Math.min(axis === 'x' ? H : W, Math.round(c1 * s)) - 4; c += 2) cs.push(c);
  if (!cs.length) return runs;
  for (let i = 6; i <= n - 6; i++) {
    let bad = 0;
    for (const c of cs) { const v = axis === 'x' ? Math.abs(L[c * W + i] - L[c * W + i - 3]) : Math.abs(L[i * W + c] - L[i * W + c - 3]); if (v > 3) bad++; }
    const ok = bad / cs.length < 0.015;
    if (ok && a < 0) a = i;
    if ((!ok || i === n - 6) && a >= 0) { if (i - a >= 4) runs.push([a / s, i / s]); a = -1; }
  }
  return runs;
}
// Grow the crop symmetrically (inside bounds B) until its aspect is back between 4:3 and 16:9; null when it cannot.
function normalize(x0, y0, x1, y1, B) {
  let w = x1 - x0, h = y1 - y0;
  if (w / h > A_MAX) { const nh = w / A_MAX; if (nh > B[3] - B[1] + 0.5) return null; y0 -= (nh - h) / 2; y1 = y0 + nh; if (y0 < B[1]) { y1 += B[1] - y0; y0 = B[1]; } if (y1 > B[3]) { y0 -= y1 - B[3]; y1 = B[3]; } }
  else if (w / h < A_MIN) { let nw = h * A_MIN; if (nw > B[2] - B[0] + 4) return null; nw = Math.min(nw, B[2] - B[0]); x0 -= (nw - w) / 2; x1 = x0 + nw; if (x0 < B[0]) { x1 += B[0] - x0; x0 = B[0]; } if (x1 > B[2]) { x0 -= x1 - B[2]; x1 = B[2]; } }
  return [x0, y0, x1, y1];
}
// Share of sampled pixels in the 6 CSS px right of x that differ from their surroundings: text touching the edge.
function edgeInk(p, s, x, y0, y1) {
  const { L, W } = p; const v = []; for (let y = Math.round(y0 * s); y < Math.round(y1 * s); y += 2) for (let xx = Math.round(x * s); xx < Math.round((x + 6) * s); xx += 2) v.push(L[y * W + xx]);
  const med = median(v); return v.filter((q) => Math.abs(q - med) > 25).length / v.length;
}
function intersect(A, B) { const out = []; for (const [a0, a1] of A) for (const [b0, b1] of B) { const lo = Math.max(a0, b0), hi = Math.min(a1, b1); if (hi - lo >= 3) out.push([lo, hi]); } return out.sort((p, q) => p[0] - q[0]); }
// Nudge each crop edge to the nearest blank run within a tolerance, or onto the bound when close to it, so no text is
// cut at the edge. S (the subject) is never cut; the aspect stays between 4:3 and 16:9.
function snapCrop(p, s, crop, B, S, win) {
  let c = [crop[0], crop[1], crop[0] + crop[2], crop[1] + crop[3]];
  const tolX = 0.075 * win.w, tolY = 0.1 * win.h, pullX = 0.1 * win.w, pullY = 0.12 * win.h;
  const pick = (runs, target, lo, hi, tol) => {
    let best = null, bd = Infinity;
    for (const [a, b] of runs) { const t = Math.min(Math.max(target, a + 1), b - 1); const d = Math.abs(t - target); if (t >= lo && t <= hi && d <= tol && d < bd) { bd = d; best = t; } }
    return best;
  };
  const sx0 = S ? S.x - 8 : Infinity, sx1 = S ? S.x + S.w + 8 : -Infinity, sy0 = S ? S.y - 8 : Infinity, sy1 = S ? S.y + S.h + 8 : -Infinity;
  const maxW = Math.max(LEGIBLE * 1.1, crop[2]);
  // horizontal edges first, then restore the aspect by growing the height
  let [x0, y0, x1, y1] = c, t;
  const vr = clearRuns(p, s, 'x', y0, y1);
  if (x0 > B[0]) { if (x0 - B[0] <= pullX) x0 = B[0]; else if ((t = pick(vr, x0, B[0], sx0, tolX)) != null) x0 = t; }
  if (x1 < B[2]) { if (B[2] - x1 <= pullX) x1 = B[2]; else if ((t = pick(vr, x1, sx1, B[2], tolX)) != null) x1 = t; }
  const fit = x1 - x0 <= maxW ? normalize(x0, y0, x1, y1, B) : null;
  if (fit) [x0, y0, x1, y1] = fit; else [x0, y0, x1, y1] = c;
  const ok = (a, b, cc, d) => { const r = (cc - a) / (d - b); return r >= A_MIN - 0.02 && r <= A_MAX + 0.02; };
  const hr = clearRuns(p, s, 'y', x0, x1);
  if (y0 > B[1]) { if (y0 - B[1] <= pullY && ok(x0, B[1], x1, y1)) y0 = B[1]; else if ((t = pick(hr, y0, B[1], sy0, tolY)) != null && ok(x0, t, x1, y1)) y0 = t; }
  if (y1 < B[3]) { if (B[3] - y1 <= pullY && ok(x0, y0, x1, B[3])) y1 = B[3]; else if ((t = pick(hr, y1, sy1, B[3], tolY)) != null && ok(x0, y0, x1, t)) y1 = t; }
  return [x0, y0, x1 - x0, y1 - y0];
}

async function macGeometry(page, file) {
  await page.goto(pathToFileURL(file).href);
  return page.evaluate(() => {
    const w = document.querySelector('.win').getBoundingClientRect();
    const rel = (e) => { const b = e.getBoundingClientRect(); return { x: b.x - w.x, y: b.y - w.y, w: b.width, h: b.height }; };
    const tog = document.querySelector('[data-id=sidebar-toggle]');
    const regs = {};
    for (const e of document.querySelectorAll('.region')) { const c = ['sheet', 'menu', 'popover', 'toast', 'inspector'].find((k) => e.classList.contains(k)); if (c) (regs[c] ||= []).push(rel(e)); }
    const toast = [...document.querySelectorAll('[data-id=toast]')].map(rel).filter((r) => r.w > 20);
    if (toast.length) regs.toast = toast.slice(0, 1);
    const LEAF = /AXStaticText|AXButton|AXMenuButton|AXPopUpButton|AXTextField|AXCheckBox|AXImage|AXHeading|AXRadioButton|AXLink/;
    const leaves = [...document.querySelectorAll('.el[data-role]')].filter((e) => LEAF.test(e.dataset.role)).map(rel).filter((r) => r.x < 330 && r.x + r.w > 150 && r.w < 700 && r.y > 4);
    return { leaves, W: w.width, H: w.height, side: !!document.querySelector('[data-id=sidebar-project-title]'), tog: tog ? rel(tog).x : null, regs };
  });
}

async function macOne(page, rec) {
  const name = rec.name, srcTime = Math.max(mtime(path.join(MAC_SRC, 'light', rec.file)), mtime(path.join(MAC_SRC, 'dark', rec.file)));
  if (isFresh('mac', name, srcTime)) return false;
  const ov = overrides[`mac/${name}`] || {};
  const g = await macGeometry(page, path.join(MAC_SRC, 'light', rec.file));
  const pics = await Promise.all(THEMES.map(async (t) => ({ t, buf: path.join(MAC_SRC, t, rec.picture), raw: await loadRaw(path.join(MAC_SRC, t, rec.picture)) })));
  const lp = pics[0].raw, s = lp.W / g.W;
  // 1. clean inner rectangle: a few px inside the window, past any capture strip
  const e = { l: 6, t: 6, r: 6, b: 6 };
  for (const p of pics) for (const [k, side] of [['l', 'left'], ['t', 'top'], ['r', 'right'], ['b', 'bottom']]) e[k] = Math.max(e[k], Math.ceil(stripDepth(p.raw, side) / s) + 2);
  // 2. sidebar
  const hasSide = g.side && g.tog > 150;
  let sideW = 0, blank = false;
  if (hasSide) {
    const x0 = g.tog - 28, x1 = g.tog; let best = g.tog - 4, bestF = 0.5;
    const ys = []; for (let y = 130; y < lp.H - 120; y += 2) ys.push(y);
    for (let xc = Math.round(x0 * s); xc <= Math.round(x1 * s); xc++) { let hit = 0; for (const y of ys) if (Math.abs(lp.L[y * lp.W + xc + 1] - lp.L[y * lp.W + xc - 1]) > 3) hit++; const f = hit / ys.length; if (f > bestF) { bestF = f; best = xc / s; } }
    sideW = best;
    blank = pics.some((p) => sidebarInk(p.raw, sideW * s) < 0.006);
  }
  const keepWanted = ov.sidebar ? ov.sidebar === 'keep' : MAC_NAV.test(name);
  const subj = [];
  for (const k of ['sheet', 'menu', 'popover']) for (const r of g.regs[k] || []) subj.push({ ...r, k });
  const toast = !!(g.regs.toast || []).length; // a toast is small and low: the whole window is its context
  const narrowPanel = PANEL_NAME.test(name) || dividers(lp, s, 60, g.H - 30).some((x) => x > 0.5 * g.W && x < 0.85 * g.W);
  const S = subj.length ? union(subj, [0, 0, g.W, g.H]) : null;
  let keep = hasSide && !blank && keepWanted;
  let minX = e.l, topCut = 0;
  if (hasSide && !keep) {
    minX = Math.max(e.l, sideW + 1.5);
  }
  if (hasSide && !keep && blank && !(S && S.x < sideW + 16)) {
    // a blank sidebar was painted over the content, so text under it is clipped at its edge: start at the first clean gutter
    // below the toolbar and drop the toolbar rows (their buttons are clipped too)
    if (edgeInk(lp, s, minX, 100, g.H - e.b - 30) > 0.01) {
      const run = clearRuns(lp, s, 'x', 100, g.H - e.b - 30).find(([a, b]) => (a + b) / 2 >= minX + 4 && (a + b) / 2 <= minX + 300);
      if (run) { minX = (run[0] + run[1]) / 2; topCut = !(S && S.y < 108) ? 100 : 0; }
    }
  }
  if (hasSide && !keep && S && S.x < sideW + 16) { // subject sits over the sidebar: no clean cut
    if (blank) minX = Math.max(e.l, S.x - 12); else { keep = true; minX = e.l; }
  }
  const B = [minX, Math.max(e.t, topCut), g.W - e.r, g.H - e.b];
  const sidebar = !hasSide ? 'collapsed' : keep ? 'kept' : 'cropped';
  // 3. card crop
  let crop, focus;
  if (S) {
    crop = fitCrop(S, B, { w: g.W, h: g.H });
    focus = subj.some((r) => r.k === 'sheet') ? 'sheet' : subj.some((r) => r.k === 'menu') ? 'menu' : 'panel';
    crop = snapCrop(lp, s, crop, B, S, { w: g.W, h: g.H });
  } else {
    // project windows: the 52 px app toolbar adds nothing, so the card starts below it (navigation screens keep the whole window)
    let dropBar = !keep && g.W >= 1200 && !MAC_BAR.test(name);
    let Bc = dropBar ? [B[0], Math.max(B[1], 54), B[2], B[3]] : B;
    const win = { w: g.W, h: g.H };
    crop = paneCrop(lp, s, Bc, win, { noLeft: keep, noRight: narrowPanel, bar: !dropBar, inset: dropBar ? 0 : 56 });
    // without the toolbar the pane can be wider than 16:9; when that costs a real column, keep the toolbar and the whole width instead
    if (dropBar && (crop[2] < 0.85 * (Bc[2] - Bc[0]) || crop[2] / crop[3] > A_MAX + 0.01) && (B[2] - B[0]) / (B[3] - B[1]) <= A_MAX) { dropBar = false; Bc = B; crop = paneCrop(lp, s, Bc, win, { noLeft: keep, noRight: narrowPanel, bar: true, inset: 56 }); }
    focus = toast ? 'panel' : keep ? 'window' : 'content';
  }
  if (ov.crop) crop = ov.crop; if (ov.focus) focus = ov.focus;
  const full = [B[0], B[1], B[2] - B[0], B[3] - B[1]];
  crop = round(crop);
  const info = {};
  for (const p of pics) {
    const srcBuf = await sharp(path.join(MAC_SRC, p.t, rec.picture)).toBuffer();
    Object.assign(info, await writeSet('mac', p.t, name, srcBuf, s, crop, round(full)));
  }
  setEntry('mac', name, { card: info.card, full: info.full, focus, sidebar });
  return true;
}

// Bounding box of everything that is not background in the body of the window; null unless it covers under 40 percent.
function inkBox(p, s, B, { top = 56, bottom = 30, limit = true } = {}) {
  const { L, W } = p; const y0 = Math.round((B[1] + top) * s), y1 = Math.round((B[3] - bottom) * s), x0 = Math.round(B[0] * s), x1 = Math.round(B[2] * s);
  const hist = new Map(); for (let y = y0; y < y1; y += 6) for (let x = x0; x < x1; x += 6) { const k = Math.round(L[y * W + x]); hist.set(k, (hist.get(k) || 0) + 1); }
  let bg = 0, bn = 0; for (const [k, n] of hist) if (n > bn) { bn = n; bg = k; }
  const cols = new Uint16Array(x1 - x0), rows = new Uint16Array(y1 - y0);
  for (let y = y0; y < y1; y += 2) for (let x = x0; x < x1; x += 2) if (Math.abs(L[y * W + x] - bg) > 7) { cols[x - x0]++; rows[y - y0]++; }
  const first = (a) => a.findIndex((v) => v >= 3), last = (a) => { for (let i = a.length - 1; i >= 0; i--) if (a[i] >= 3) return i; return -1; };
  const [cx0, cx1, cy0, cy1] = [first(cols), last(cols), first(rows), last(rows)];
  if (cx0 < 0 || cy0 < 0) return null;
  const box = { x: (x0 + cx0) / s, y: (y0 + cy0) / s, w: (cx1 - cx0) / s, h: (cy1 - cy0) / s };
  if (!limit) return box;
  return box.w < 0.75 * (B[2] - B[0]) && box.w * box.h < 0.4 * (B[2] - B[0]) * (B[3] - B[1] - 86) ? box : null;
}

// Card for a screen with no pop-up: the pane Bc (app bar, status bar and navigation already dropped by the caller) trimmed to its
// content. A sparse pane is cropped to what is on screen; a full one is cut at a panel divider or blank gutter to fit 16:9.
function paneCrop(p, s, Bc, win, opt) {
  const ink = inkBox(p, s, Bc, { top: opt.inset || 0, bottom: opt.inset ? 30 : 0, limit: false });
  const sparse = !!ink && ink.w * ink.h < 0.45 * (Bc[2] - Bc[0]) * (Bc[3] - Bc[1]);
  let crop = [Bc[0], Bc[1], Bc[2] - Bc[0], Bc[3] - Bc[1]];
  if (sparse) crop = fitCrop(ink, Bc, win);
  else {
    if (ink) { const x0 = Math.max(Bc[0], ink.x - 6), x1 = Math.min(Bc[2], ink.x + ink.w + 6); crop = [x0, Bc[1], x1 - x0, Bc[3] - Bc[1]]; } // empty side margins go first
    crop = trimAspect(p, s, crop, opt);
    const n = normalize(crop[0], crop[1], crop[0] + crop[2], crop[1] + crop[3], Bc); // a narrow page: widen back into the pane
    if (n) crop = [n[0], n[1], n[2] - n[0], n[3] - n[1]];
  }
  return snapCrop(p, s, crop, Bc, sparse ? ink : null, win);
}

// Bring a wide crop (no subject) into 4:3..16:9 by cutting at a panel divider or blank gutter, never mid-text.
// noLeft: the sidebar is the subject, keep it. noRight: a narrow right-hand panel is the subject, keep it.
function trimAspect(p, s, crop, { noLeft, noRight, bar: hasBar = true }) {
  const [x, y, w, h] = crop;
  if (w / h <= A_MAX + 0.001) return crop;
  const top = y + (hasBar ? 52 : 4), bot = y + h - (hasBar ? 30 : 4);
  const need = w - h * A_MAX; // CSS px to remove
  const maxCut = w - h * A_MIN;
  const cands = [];
  const add = (list, rank) => { for (const c of list) { const cutL = c - x, cutR = x + w - c; if (!noLeft && cutL >= need && cutL <= maxCut) cands.push({ side: 'l', at: c, cut: cutL, rank }); if (!noRight && cutR >= need && cutR <= maxCut) cands.push({ side: 'r', at: c, cut: cutR, rank }); } };
  // prefer cuts that are also clear in the toolbar band, so no toolbar button is sliced
  const bar = hasBar ? clearRuns(p, s, 'x', y + 4, y + 46) : null;
  const barOk = (c) => !bar || bar.some(([a, b]) => c >= a + 1 && c <= b - 1);
  const divs = dividers(p, s, top, bot), runs = clearRuns(p, s, 'x', top, bot).map(([a, b]) => (a + b) / 2);
  add(divs.filter(barOk), 0); add(runs.filter(barOk), 1); add(divs, 2); add(runs, 3);
  cands.sort((a, b) => a.cut + a.rank * 40 - (b.cut + b.rank * 40) || (a.side === 'l' ? -1 : 1)); // the smallest cut wins; a clean toolbar earns 40 px
  const c = cands[0];
  if (!c && need <= 0.08 * w) return crop; // a few px too wide and no clean cut: the caller decides (never slice a button)
  if (c) return c.side === 'l' ? [c.at, y, x + w - c.at, h] : [x, y, c.at - x, h];
  return noLeft || !noRight && false ? [x, y, h * A_MAX, h] : noRight ? [x + w - h * A_MAX, y, h * A_MAX, h] : [x, y, h * A_MAX, h]; // no clean cut: straight cut on the free side
}

// ---------- WINDOWS ----------
// The Windows pages are the app's own DOM. Spotlight screens (dialog, menu, toast) are rendered at the captured 1600x1000
// and cropped. Other screens are rendered at 1200x720, the app's smallest window, where the layout reflows and stays legible.
const WIN_SRC = path.join(RES, 'WindowsHTML-src');
const WIN_BAR = /^shell-(project-menu|status|create-menu|search|toast|back|forward|display-name|open-recovery)|^app-|^apphome-|^licence-/;
const WIN_NAV = /^shell-(project-home|nav-|empty-project-home)/;

async function winPrepare(page, collapse) {
  return page.evaluate((collapse) => {
    const nav = document.querySelector('nav.nav');
    if (nav && collapse && !nav.classList.contains('collapsed')) {
      // pop-ups keep their captured screen position; the page content slides left by the width the navigation gives up, so slide them too
      // (those anchored to the top bar, y under 60, stay where they are)
      const pops = [...document.querySelectorAll('.menu-pop,[role=menu],[role=listbox]')].filter((e) => !e.closest('[role=dialog]') && e.getBoundingClientRect().y > 60);
      const w0 = nav.getBoundingClientRect().width;
      nav.classList.add('collapsed');
      nav.querySelectorAll('.navitem').forEach((b) => { const t = b.textContent.trim(); if (t && !b.classList.contains('nav-toggle')) { b.setAttribute('aria-label', t); b.setAttribute('title', t); } for (const c of [...b.childNodes]) if (c.nodeType === 3) c.remove(); });
      nav.querySelector('.foot')?.remove();
      const tg = nav.querySelector('.nav-toggle'); if (tg) { tg.setAttribute('aria-label', 'Expand the navigation'); tg.setAttribute('aria-expanded', 'false'); }
      const d = w0 - nav.getBoundingClientRect().width;
      for (const e of pops) e.style.translate = `${-d}px 0`;
    }
  }, collapse);
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
      layout: (() => { const l = document.querySelector('.layout'); return l ? R(l) : null; })(),
      panels: [...document.querySelectorAll('aside.drawer,aside.ai,aside.sp-tools,aside.spx-tools')].map(R).filter((r) => r.w >= 250 && r.h > 200),
      overflow: document.documentElement.scrollWidth - window.innerWidth,
    };
  });
}

async function winOne(ctx, rec) {
  const name = rec.name, srcTime = Math.max(mtime(path.join(WIN_SRC, 'light', rec.file)), mtime(path.join(WIN_SRC, 'dark', rec.file)));
  if (isFresh('win', name, srcTime)) return false;
  const ov = overrides[`win/${name}`] || {};
  const page = await ctx.newPage();
  const url = (t) => pathToFileURL(path.join(WIN_SRC, t, rec.file)).href;
  const keepWanted = ov.sidebar ? ov.sidebar === 'keep' : WIN_NAV.test(name);
  const big = () => page.setViewportSize({ width: 1600, height: 1000 });
  try {
    await big();
    await page.goto(url('light'));
    const probe = await winGeometry(page);
    const collapse = !!probe.nav && !keepWanted && !probe.nav.collapsed;
    await winPrepare(page, collapse);
    const geo = await winGeometry(page);
    const spot = [...geo.dialogs, ...geo.menus, ...geo.lists];
    let mode = spot.length ? 'subject' : 'window'; // a toast is small and low: the whole app is its context
    let focus = 'window', crop = null;
    const navOpen = geo.nav && !geo.nav.collapsed;
    let sidebar = navOpen ? 'kept' : 'collapsed'; // 'collapsed' also covers screens without a navigation
    if (mode === 'subject') {
      const S = union(spot, [0, 0, 1600, 1000]);
      focus = geo.dialogs.length ? 'dialog' : 'menu';
      let minX = 0;
      const Bw = [minX, 0, 1600, 1000];
      crop = fitCrop(S, Bw, { w: 1600, h: 1000 });
      const lum = await sharp(await page.screenshot({ type: 'png', scale: 'css' })).resize(1600, 1000).greyscale().raw().toBuffer();
      crop = snapCrop({ L: Float32Array.from(lum), W: 1600, H: 1000 }, 1, crop, Bw, S, { w: 1600, h: 1000 });
      if (geo.nav && crop[0] > 0.5 && crop[0] < geo.nav.w - 0.5) { // never slice the navigation: take all of it or none
        let [x, y, w, h] = crop;
        if (!keepWanted && S.x - 12 >= geo.nav.w) { w -= geo.nav.w - x; x = geo.nav.w; } else { w += x; x = 0; }
        const n = normalize(x, y, x + w, y + h, [0, 0, 1600, 1000]);
        crop = n ? [n[0], n[1], n[2] - n[0], n[3] - n[1]] : crop;
      }
      if (navOpen && keepWanted && crop[0] >= geo.nav.w - 1) sidebar = 'cropped';
    } else {
      if (geo.toasts.length) focus = 'panel';
      await page.setViewportSize({ width: 1200, height: 720 });
      await page.waitForTimeout(80);
      const g2 = await winGeometry(page);
      if (g2.overflow > 8) { mode = 'overflow'; crop = [0, 0, 1150, 736]; await big(); }
      else {
        // the main pane: right of the navigation, below the app bar, above the status bar; with a right-hand panel open, the panel plus the
        // pane next to it (about 1050 px). Navigation screens, and the library and home screens that have no pane, keep the whole window.
        const lum = await sharp(await page.screenshot({ type: 'png', scale: 'css' })).greyscale().raw().toBuffer();
        const p = { L: Float32Array.from(lum), W: 1200, H: 720 }, win = { w: 1200, h: 720 };
        const lay = g2.layout, wholeWin = (navOpen && keepWanted) || !lay || WIN_BAR.test(name);
        const Bc = wholeWin ? [0, 0, 1200, 720] : [g2.nav ? g2.nav.w : 0, lay.y, 1200, lay.y + lay.h];
        const panel = wholeWin ? null : g2.panels.find((r) => r.x > Bc[0] + 300);
        if (panel && Bc[2] - Bc[0] > 1100) { // start the pane at a blank gutter about 1050 px left of the panel's right edge, or keep the whole pane
          const right = panel.x + panel.w;
          const cs = clearRuns(p, 1, 'x', Bc[1], Bc[3]).map(([a, b]) => (a + b) / 2).filter((c) => c >= Bc[0] && c >= right - 1150 && c <= right - 950);
          if (cs.length) Bc[0] = cs.sort((a, b) => Math.abs(right - a - 1050) - Math.abs(right - b - 1050))[0];
        }
        crop = paneCrop(p, 1, Bc, win, { noRight: !!panel, bar: wholeWin, inset: wholeWin ? 56 : 0 });
        if (!geo.toasts.length) focus = wholeWin ? 'window' : 'content';
      }
    }
    if (ov.crop) { crop = ov.crop; mode = 'subject'; }
    if (ov.focus) focus = ov.focus;
    const info = {};
    for (const theme of THEMES) {
      if (theme === 'dark') { await big(); await page.goto(url('dark')); await winPrepare(page, collapse); }
      if (mode === 'window') await page.setViewportSize({ width: 1200, height: 720 }); else await big();
      await page.evaluate(() => document.fonts.ready);
      await page.waitForTimeout(60);
      const buf = await page.screenshot({ type: 'png' });
      const full = mode === 'window' ? [0, 0, 1200, 720] : [0, 0, 1600, 1000];
      Object.assign(info, await writeSet('win', theme, name, buf, 2, round(crop || full), full));
    }
    setEntry('win', name, { card: info.card, full: info.full, focus, sidebar, ...(mode === 'overflow' ? { note: 'overflow-crop' } : {}) });
  } finally { await page.close(); }
  return true;
}

// ---------- contact sheets ----------
const COLS = 4, ROWS = 5, TILE = 440, LABEL = 26;
async function contactSheets(plat, cat) {
  const dir = path.join(OUT, 'qa'); fs.mkdirSync(dir, { recursive: true });
  const names = cat.map((c) => c.name).filter((n) => manifest[plat][n]);
  const per = COLS * ROWS;
  for (const theme of THEMES) {
    for (let s = 0; s * per < names.length; s++) {
      const comps = [];
      for (let i = 0; i < per; i++) {
        const idx = s * per + i; if (idx >= names.length) break;
        const n = names[idx]; const f = outPath(plat, theme, n, 'card-720');
        if (!fs.existsSync(f)) continue;
        const m = manifest[plat][n]; const col = i % COLS, row = Math.floor(i / COLS);
        const th = Math.round(TILE * 9 / 16) + 30; // tallest tile (4:3 would be 330)
        const img = await sharp(f).resize({ width: TILE - 8, height: 340, fit: 'inside' }).jpeg({ quality: 82 }).toBuffer();
        const label = `${String(idx + 1).padStart(3, '0')} ${n}`.slice(0, 54) + `  ${m.focus}/${m.sidebar}`;
        const svg = Buffer.from(`<svg xmlns="http://www.w3.org/2000/svg" width="${TILE}" height="${LABEL}"><text x="4" y="17" font-family="Arial" font-size="12" fill="#222">${label.replace(/&/g, '&amp;').replace(/</g, '&lt;')}</text></svg>`);
        comps.push({ input: img, left: col * TILE + 4, top: row * (340 + LABEL) + 4 });
        comps.push({ input: svg, left: col * TILE, top: row * (340 + LABEL) + 344 });
      }
      await sharp({ create: { width: COLS * TILE, height: ROWS * (340 + LABEL) + 8, channels: 3, background: '#d8d8dc' } }).composite(comps).jpeg({ quality: 80 }).toFile(path.join(dir, `${plat}-${theme}-${String(s + 1).padStart(2, '0')}.jpg`));
    }
  }
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
    const cat = readJson(path.join(RES, 'screens-mac.json'), []).filter((r) => !MATCH || MATCH.test(r.name));
    console.log(`mac: ${cat.length} screens`);
    const pages = await Promise.all(Array.from({ length: 4 }, async () => (await browser.newContext({ viewport: { width: 1500, height: 900 } })).newPage()));
    let made = 0;
    await pool(cat, 4, async (rec, w) => { if (await macOne(pages[w], rec)) made++; if (made % 25 === 0) saveManifest(); });
    saveManifest();
    console.log(`mac: ${made} made, ${cat.length - made} up to date`);
    if (QA && !MATCH) await contactSheets('mac', readJson(path.join(RES, 'screens-mac.json'), []));
  }
  if (ONLY !== 'mac') {
    const cat = readJson(path.join(RES, 'screens-win.json'), []).filter((r) => !MATCH || MATCH.test(r.name));
    console.log(`win: ${cat.length} screens`);
    const ctxs = await Promise.all(Array.from({ length: 3 }, () => browser.newContext({ viewport: { width: 1600, height: 1000 }, deviceScaleFactor: 2 })));
    let made = 0;
    await pool(cat, 3, async (rec, w) => { if (await winOne(ctxs[w], rec)) made++; if (made % 25 === 0) saveManifest(); });
    saveManifest();
    console.log(`win: ${made} made, ${cat.length - made} up to date`);
    if (QA && !MATCH) await contactSheets('win', readJson(path.join(RES, 'screens-win.json'), []));
  }
} finally { await browser.close(); saveManifest(); }
