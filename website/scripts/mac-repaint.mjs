// Repaints the product name in the Mac window pictures (the captures still draw the old name in their pixels).
// Used by scripts/shots.mjs for every Mac screen and theme, after any sidebar repair.
//
// How it works, per text element of the accessibility overlay whose drawn text holds the old name:
//  1. The element's box (points) is scaled to the picture (2x). In it the background colour (the most common colour) and the ink
//     pixels are found; ink rows make the text lines, ink columns with word-sized gaps make the word segments of each line.
//  2. The words of the overlay text are matched to the segments (dynamic programming over predicted word widths): this tells which
//     words sit on which line, and skips what is not text (an icon at the start, a key shortcut at the end of a menu row, a
//     picture above a caption). A long word (a path) may be split over two lines.
//  3. Inter (the website font, close to SF Pro) is fitted to each line's own original text: size from the ink height, weight from
//     the ink mass, letter spacing from the ink width, baseline and pen position from the ink position. The fit is checked
//     against the pixels (correlation); a poor fit leaves the element alone.
//  4. Only lines whose text changes are redrawn: the original ink is erased (a Coons patch between the pixels around it, so
//     gradients and shadows continue) and the new line is drawn with the fitted font, in the measured ink colour. When the change
//     moves words between lines (a path that wrapped), the lines from the first changed one on are re-wrapped to the box width.
// Menus, sheets and popovers drawn over an element clip it. (Scroll area frames are not reliable clips: text scrolled out of view
// is found by the fit check instead, which leaves a line alone unless its pixels match its text.)
import fs from 'node:fs';
import path from 'node:path';
import sharp from 'sharp';
import { BRAND_RULES, hasOldName } from '../src/lib/brand.mjs';

const ROOT = path.resolve(import.meta.dirname, '..');
const FONT = path.join(ROOT, 'src', 'assets', 'fonts', 'inter-latin-wght-normal.woff2');
const NAME_RX = BRAND_RULES.find((r) => r[2] === 'name')[0]; // the old name, as a regex source (never spelled out here)

// ---------- 1. the elements, collected inside the loaded source page (self-contained) ----------
export function collectNameText(o) {
  const has = (t) => !!t && new RegExp(o.rx).test(t);
  const win = document.querySelector('.win'); const wr = win.getBoundingClientRect();
  const rel = (e) => { const b = e.getBoundingClientRect(); return { x: b.x - wr.x, y: b.y - wr.y, w: b.width, h: b.height }; };
  const RANK = { menu: 3, popover: 2, sheet: 1 };
  const rankOf = (r) => (r ? RANK[[...r.classList].find((c) => c in RANK)] || 0 : 0);
  const regs = [...win.querySelectorAll('.region')].map((r) => ({ r, rank: rankOf(r), box: rel(r) }));
  const own = (e) => [...e.childNodes].filter((n) => n.nodeType === 3).map((n) => n.nodeValue).join('');
  const out = [], seen = new Set();
  for (const e of win.querySelectorAll('.el')) {
    const role = e.dataset.role || '';
    let text = own(e), kind = 'text';
    if (!text.trim()) text = '';
    if (!text && /textfield|textarea|searchfield|combobox/.test(e.className)) { text = e.value || e.getAttribute('value') || ''; kind = 'field'; }
    if (!text && /RadioButton|CheckBox/.test(role)) { text = e.getAttribute('aria-label') || ''; kind = 'control'; } // the label drawn next to the control
    if (!text && !/statictext|heading/.test(e.className) && ![...e.querySelectorAll('.el')].some((d) => has(own(d)) || has(d.getAttribute('aria-label')) || has(d.value))) {
      text = e.getAttribute('aria-label') || ''; kind = 'label';
    }
    if (!has(text)) continue;
    const box = rel(e);
    if (box.w < 4 || box.h < 4) continue;
    const key = [Math.round(box.x), Math.round(box.y), Math.round(box.w), Math.round(box.h), text].join('|');
    if (seen.has(key)) continue; seen.add(key);
    const reg = e.closest('.region'); const rank = rankOf(reg);
    const clip = { x: 0, y: 0, w: wr.width, h: wr.height }; // (region and scroll area frames are not reliable clips)
    // drawn over part of it: a menu, popover or sheet (of at least its own kind) that covers some but not all of its box. (The DOM
    // does not tell which of two sheets is on top: a sheet on a sheet can hold its elements in the lower one's region.)
    const ownBox = reg ? rel(reg) : null;
    const same = (a, b) => a && b && ['x', 'y', 'w', 'h'].every((k) => Math.abs(a[k] - b[k]) < 1);
    const holds = (a, b) => a.x <= b.x + 0.5 && a.y <= b.y + 0.5 && a.x + a.w >= b.x + b.w - 0.5 && a.y + a.h >= b.y + b.h - 0.5;
    const meets = (a, b) => a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
    const box0 = rel(e);
    const occ = regs.filter((g) => g.rank >= Math.max(1, rank) && !g.r.contains(e) && !same(g.box, ownBox) && meets(g.box, box0) && !holds(g.box, box0)).map((g) => g.box);
    const centres = [];
    for (let a = e.parentElement; a && a !== win.parentElement; a = a.parentElement) if (a.classList.contains('el') || a.classList.contains('region') || a === win) { const b = rel(a); centres.push([b.x + b.w / 2, b.w]); }
    const field = kind === 'field' || /button|popupbutton|menubutton/.test(e.className) && !/menuitem/.test(e.className);
    out.push({ text, kind, cls: e.className.replace(/^el\s+/, ''), lh: parseFloat(getComputedStyle(e).lineHeight) || 0, box, clip, occ, centres, field, menu: /menuitem/.test(e.className) });
  }
  return { W: wr.width, H: wr.height, els: out };
}

// ---------- the text renderer: an offscreen page drawing Inter on a canvas ----------
async function pageSetup(b64) {
  const bin = atob(b64); const buf = new Uint8Array(bin.length); for (let i = 0; i < bin.length; i++) buf[i] = bin.charCodeAt(i);
  const ff = new FontFace('RP', buf, { weight: '100 900' }); await ff.load(); document.fonts.add(ff);
  window.cv = document.createElement('canvas');
  const font = (j) => `${Math.round(j.w)} ${j.f}px RP`;
  window.adv = (ctx, j, t) => { ctx.font = font(j); ctx.letterSpacing = j.ls + 'px'; return ctx.measureText(t).width - (t.length ? j.ls : 0); };
  window.renderJobs = (jobs) => jobs.map((j) => {
    const c = window.cv; let ctx = c.getContext('2d');
    const adv = window.adv(ctx, j, j.t);
    const pad = Math.ceil(j.f * 0.5) + 4, W = Math.ceil(adv + 2 * pad + Math.abs(j.ls) * j.t.length), H = Math.ceil(j.f * 1.9) + 6, px = pad, py = Math.ceil(j.f * 1.25) + 3;
    c.width = W; c.height = H; ctx = c.getContext('2d');
    ctx.font = font(j); ctx.letterSpacing = j.ls + 'px'; ctx.fillStyle = '#000'; ctx.textBaseline = 'alphabetic';
    ctx.fillText(j.t, px + (j.fx || 0), py + (j.fy || 0));
    const d = ctx.getImageData(0, 0, W, H).data; let s = '';
    const a = new Uint8Array(W * H); for (let i = 0; i < a.length; i++) a[i] = d[i * 4 + 3];
    for (let i = 0; i < a.length; i += 8192) s += String.fromCharCode.apply(null, a.subarray(i, i + 8192));
    return { adv, W, H, px, py, a: btoa(s) };
  });
  // greedy wrap, the way AppKit breaks: at spaces; a word longer than the line breaks after / - . _ or anywhere
  window.wrapJob = (j) => {
    const ctx = window.cv.getContext('2d'); const width = (t) => window.adv(ctx, j, t);
    const lines = []; let cur = j.first || '';
    let max = j.max0 ?? j.max;
    const push = () => { lines.push(cur); cur = ''; max = j.max; };
    for (const word of j.t.split(/\s+/).filter(Boolean)) {
      const cand = cur ? cur + ' ' + word : word;
      if (width(cand) <= max) { cur = cand; continue; }
      if (cur && width(word) <= j.max) { push(); cur = word; continue; }
      let rest = cur ? ' ' + word : word; // a long word: fill the line, then break inside it
      while (rest) {
        const base = cur; let best = -1, bestSoft = -1;
        for (let k = 1; k <= rest.length; k++) { if (width(base + rest.slice(0, k)) > max) break; best = k; if (/[\/\-._]/.test(rest[k - 1])) bestSoft = k; }
        const k = bestSoft > 0 ? bestSoft : best;
        if (k <= 0) { if (cur) { push(); rest = rest.trimStart(); continue; } cur = rest[0]; rest = rest.slice(1); continue; }
        cur = (base + rest.slice(0, k)).trimStart(); rest = rest.slice(k);
        if (rest) push();
      }
    }
    if (cur) lines.push(cur);
    return lines;
  };
}

export async function makeRenderer(context) {
  const page = await context.newPage();
  await page.setContent('<!doctype html><html><body></body></html>');
  await page.evaluate(pageSetup, fs.readFileSync(FONT).toString('base64'));
  const cache = new Map();
  return {
    async render(jobs) {
      const need = jobs.filter((j) => !cache.has(JSON.stringify(j)));
      if (need.length) {
        const res = await page.evaluate((js) => window.renderJobs(js), need);
        need.forEach((j, i) => { const r = res[i]; r.a = Buffer.from(r.a, 'base64'); cache.set(JSON.stringify(j), r); });
        if (cache.size > 4000) cache.clear();
      }
      return jobs.map((j) => inkOf(cache.get(JSON.stringify(j))));
    },
    wrap: (job) => page.evaluate((j) => window.wrapJob(j), job),
    prefix: (job) => page.evaluate((j) => { const ctx = window.cv.getContext('2d'); return [...j.t].map((_, i) => window.adv(ctx, j, j.t.slice(0, i + 1))); }, job),
    close: () => page.close(),
  };
}

// ink statistics of a rendered or measured alpha map (values 0..1 or 0..255 scaled by k)
function inkOf(r) {
  if (r.ink) return r;
  const { W, H, a } = r; let x0 = W, x1 = -1, y0 = H, y1 = -1, m = 0, sx = 0, sy = 0;
  for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) { const v = a[y * W + x] / 255; if (!v) continue; m += v; sx += v * x; sy += v * y; if (v >= 0.4) { if (x < x0) x0 = x; if (x > x1) x1 = x; if (y < y0) y0 = y; if (y > y1) y1 = y; } }
  r.ink = { x0, x1, y0, y1, m, cx: sx / (m || 1), cy: sy / (m || 1) };
  let vy = 0; for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) { const v = a[y * W + x] / 255; if (v) vy += v * (y - r.ink.cy) ** 2; }
  r.ink.sy = Math.sqrt(vy / (m || 1)); // the vertical spread of the ink, a sub-pixel measure of the text size
  r.ink.left = edge(r, 1); r.ink.right = edge(r, -1);
  return r;
}
// sub-pixel ink edge: where the strongest pixel of a column first reaches half coverage, from the left (dir 1) or right (dir -1)
function edge(r, dir) {
  const { W, H, a } = r; const col = new Float64Array(W);
  for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) { const v = a[y * W + x] / 255; if (v > col[x]) col[x] = v; }
  const thr = 0.5;
  if (dir > 0) { for (let x = 0; x < W; x++) if (col[x] >= thr) { const p = x ? col[x - 1] : 0; return x - 1 + (thr - p) / Math.max(1e-6, col[x] - p); } }
  else for (let x = W - 1; x >= 0; x--) if (col[x] >= thr) { const p = x < W - 1 ? col[x + 1] : 0; return x + 1 - (thr - p) / Math.max(1e-6, col[x] - p); }
  return 0;
}

// ---------- image helpers ----------
const dist = (d, i, c) => Math.hypot(d[i] - c[0], d[i + 1] - c[1], d[i + 2] - c[2]);
function modeColour(img, x0, y0, x1, y1, skip) {
  const h = new Map(); const { data, W } = img;
  for (let y = y0; y < y1; y++) for (let x = x0; x < x1; x++) { if (skip(x, y)) continue; const i = (y * W + x) * 4; const k = (data[i] >> 3) << 10 | (data[i + 1] >> 3) << 5 | data[i + 2] >> 3; h.set(k, (h.get(k) || 0) + 1); }
  if (!h.size) return null;
  const k = [...h].sort((a, b) => b[1] - a[1])[0][0];
  const sum = [0, 0, 0]; let n = 0; // mean of the pixels in that bucket
  for (let y = y0; y < y1; y++) for (let x = x0; x < x1; x++) { if (skip(x, y)) continue; const i = (y * W + x) * 4; if (((data[i] >> 3) << 10 | (data[i + 1] >> 3) << 5 | data[i + 2] >> 3) !== k) continue; sum[0] += data[i]; sum[1] += data[i + 1]; sum[2] += data[i + 2]; n++; }
  return sum.map((v) => v / n);
}
const median = (a) => { if (!a.length) return 0; const s = [...a].sort((p, q) => p - q); return s[s.length >> 1]; };

// ---------- 2. lines and word segments ----------
function analyse(img, R, skip) {
  const { data, W } = img; const [x0, y0, x1, y1] = R;
  const bg = modeColour(img, x0, y0, x1, y1, skip); if (!bg) return null;
  const w = x1 - x0, h = y1 - y0; const D = new Float32Array(w * h); const ds = [];
  for (let y = y0; y < y1; y++) for (let x = x0; x < x1; x++) { if (skip(x, y)) continue; const v = dist(data, (y * W + x) * 4, bg); D[(y - y0) * w + x - x0] = v; ds.push(v); }
  ds.sort((a, b) => a - b); const d99 = ds[Math.floor(ds.length * 0.995)] || 0;
  if (d99 < 40) return null; // no ink in the box: out of view or not drawn
  const T = Math.max(22, 0.3 * d99);
  const M = new Uint8Array(w * h); for (let i = 0; i < M.length; i++) M[i] = D[i] >= T ? 1 : 0;
  // rows with ink -> bands; rows inked across the box are rules or borders, not text
  const rowN = new Int32Array(h); for (let y = 0; y < h; y++) { let n = 0; for (let x = 0; x < w; x++) n += M[y * w + x]; rowN[y] = n > 0.8 * w ? 0 : n; }
  let bands = []; for (let y = 0; y < h; y++) if (rowN[y]) { const b = bands[bands.length - 1]; if (b && y - b[1] <= 3) b[1] = y; else bands.push([y, y]); }
  bands = bands.filter((b) => b[1] - b[0] >= 3);
  if (!bands.length) return null;
  // each band (a line, or something else) gets its own ink threshold: a grey caption under a dark title keeps its letter shapes
  const lines = bands.map(([t, b]) => {
    const dv = []; for (let y = t; y <= b; y++) for (let x = 0; x < w; x++) if (M[y * w + x]) dv.push(D[y * w + x]);
    dv.sort((p, q) => p - q); const Tb = Math.max(18, 0.3 * (dv[Math.floor(dv.length * 0.95)] || T));
    const Mb = (x, y) => D[y * w + x] >= Tb;
    let bt = b, bb = t; for (let y = t; y <= b; y++) for (let x = 0; x < w; x++) if (Mb(x, y)) { if (y < bt) bt = y; if (y > bb) bb = y; }
    if (bb < bt) { bt = t; bb = b; }
    const G = Math.max(3, Math.round(0.2 * (bb - bt + 1)));
    const col = new Int32Array(w); for (let y = bt; y <= bb; y++) for (let x = 0; x < w; x++) col[x] += Mb(x, y) ? 1 : 0;
    const segs = []; let cur = null;
    for (let x = 0; x < w; x++) { if (col[x]) { if (cur && x - cur[1] <= G) cur[1] = x; else segs.push(cur = [x, x]); } }
    return { t: bt + y0, b: bb + y0, G, segs: segs.map(([a, c]) => {
      let st = bb, sb = bt; for (let y = bt; y <= bb; y++) for (let x = a; x <= c; x++) if (Mb(x, y)) { if (y < st) st = y; if (y > sb) sb = y; }
      return { x0: a + x0, x1: c + x0, h: sb - st + 1 };
    }) };
  });
  const bh = median(lines.map((l) => l.b - l.t + 1)), G = median(lines.map((l) => l.G));
  // the text height: the median segment height, weighted by width (an icon in the line is narrow and tall)
  const sg = lines.flatMap((l) => l.segs).sort((p, q) => p.h - q.h); const tw = sg.reduce((a, g) => a + g.x1 - g.x0 + 1, 0);
  let acc = 0, th = bh; for (const g of sg) { acc += g.x1 - g.x0 + 1; if (acc >= tw / 2) { th = g.h; break; } }
  return { bg, d99, T, lines, bh: th, G };
}

// ink colour of a set of rectangles: the median of the most contrasting pixels
function inkColour(img, rects, bg, skip) {
  const px = []; const { data, W } = img;
  for (const [x0, y0, x1, y1] of rects) for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) { if (skip(x, y)) continue; const i = (y * W + x) * 4; px.push([dist(data, i, bg), data[i], data[i + 1], data[i + 2]]); }
  px.sort((a, b) => b[0] - a[0]); const top = px.slice(0, Math.max(3, Math.round(px.length * 0.06)));
  return [1, 2, 3].map((c) => median(top.map((p) => p[c])));
}
// alpha map of the original ink in a rectangle, against bg toward ink
function alphaMap(img, [x0, y0, x1, y1], bg, ink, skip) {
  const { data, W } = img; const w = x1 - x0 + 1, h = y1 - y0 + 1; const a = new Uint8Array(w * h);
  const v = [ink[0] - bg[0], ink[1] - bg[1], ink[2] - bg[2]]; const vv = v[0] * v[0] + v[1] * v[1] + v[2] * v[2] || 1;
  for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) {
    if (skip(x, y)) continue; const i = (y * W + x) * 4;
    const t = ((data[i] - bg[0]) * v[0] + (data[i + 1] - bg[1]) * v[1] + (data[i + 2] - bg[2]) * v[2]) / vv;
    a[(y - y0) * w + x - x0] = Math.round(255 * Math.max(0, Math.min(1, t)));
  }
  return { W: w, H: h, a };
}

// ---------- 3. words to segments ----------
// seq: the segments of all lines in reading order, each { line, x0, x1, first, last }; words with predicted ink widths p.
// Returns per word the segment index range [i0, i1] (i1 > i0 when a word is drawn as two pieces or wraps over lines), or null.
function matchWords(seq, p, em, gapW, opts) {
  const n = seq.length, m = p.length; const INF = 1e9;
  const c = (sw, pw) => Math.abs(sw - pw) / (pw + 0.35 * em);
  const best = Array.from({ length: n + 1 }, () => new Float64Array(m + 1).fill(INF)); const from = Array.from({ length: n + 1 }, () => new Array(m + 1));
  best[0][0] = 0;
  for (let i = 0; i <= n; i++) for (let j = 0; j <= m; j++) {
    const b = best[i][j]; if (b >= INF) continue;
    const step = (ni, nj, cost, how) => { if (b + cost < best[ni][nj]) { best[ni][nj] = b + cost; from[ni][nj] = [i, j, how]; } };
    if (i < n) {
      const s = seq[i], sw = s.x1 - s.x0 + 1;
      // not text: an icon or a key at either end of a line, or anything before the first / after the last word
      if (j === 0 || j === m) step(i + 1, j, 0.5, 'skip');
      else if (s.first || s.last) step(i + 1, j, 0.6 + 0.2 * Math.min(1, sw / em), 'skip');
      if (j < m) {
        step(i + 1, j + 1, c(sw, p[j]), 'one');
        // one word drawn as two pieces on the same line (a wide letter gap)
        if (i + 1 < n && seq[i + 1].line === s.line) step(i + 2, j + 1, c(seq[i + 1].x1 - s.x0 + 1, p[j]) + 0.25, 'two');
        // one word wrapped over several lines (a long path): the end of one line, whole lines, the start of the next
        let tot = sw;
        for (let q = i + 1; q < n && q - i < 10 && p[j] > 0.35 * opts.width; q++) { // (only a word too long for most of a line wraps)
          const a = seq[q - 1], z = seq[q];
          if (!(a.last && z.first && z.line === a.line + 1)) break;
          tot += z.x1 - z.x0 + 1; step(q + 1, j + 1, c(tot, p[j]) + 0.15, 'wrap');
          if (!z.last) break;
        }
        // two words in one segment (a narrow space)
        if (j + 1 < m) step(i + 1, j + 2, c(sw, p[j] + gapW + p[j + 1]) + 0.35, 'merge');
      }
    }
    // words cut off at the end (by the box, or by a menu or sheet over it)
    if (j < m && i === n && j > 0) step(i, j + 1, opts.cut ? 0.02 : 0.25, 'hidden');
  }
  if (best[n][m] >= INF) return null;
  const res = new Array(m).fill(null); let i = n, j = m; const path = [];
  while (i || j) { const [pi, pj, how] = from[i][j]; path.push([pi, pj, i, j, how]); i = pi; j = pj; }
  const pair = [];
  for (const [pi, pj, ni, nj, how] of path) {
    if (how === 'one' || how === 'two' || how === 'wrap' || how === 'merge') pair.push(best[ni][nj] - best[pi][pj]);
    if (how === 'one' || how === 'two' || how === 'wrap') res[pj] = { i0: pi, i1: ni - 1, how };
    if (how === 'merge') { res[pj] = { i0: pi, i1: pi, how: 'merge' }; res[pj + 1] = { i0: pi, i1: pi, how: 'merge2' }; }
    if (how === 'hidden') res[pj] = { hidden: true };
  }
  // the fit of the matched words alone (skipped icons and keys and hidden words are judged by the caller)
  return { res, cost: pair.length ? pair.reduce((a, v) => a + v, 0) / pair.length : 9, hidden: res.filter((r) => r?.hidden).length };
}

// ---------- 4. erase and draw ----------
// Fill a rectangle from the 2 px ring around it: a Coons patch (so linear gradients and shadows continue in both directions); where
// one side of the ring is hidden or holds ink, from the other two sides only. Ring pixels that look like ink are left out.
function erase(out, W, H, [x0, y0, x1, y1], bg, inkD, skip, ink) {
  x0 = Math.max(1, x0); y0 = Math.max(1, y0); x1 = Math.min(W - 2, x1); y1 = Math.min(H - 2, y1);
  if (x1 < x0 || y1 < y0) return;
  const al = []; for (let y = y0; y <= y1; y += 2) for (let x = x0; x <= x1; x += 2) al.push(out[(y * W + x) * 4 + 3]); const aBg = median(al); // (see-through pixels are not this text)
  const get = (x, y) => { if (x < 0 || y < 0 || x >= W || y >= H || skip(x, y)) return null; const i = (y * W + x) * 4; const c = [out[i], out[i + 1], out[i + 2], out[i + 3]]; return Math.hypot(c[0] - bg[0], c[1] - bg[1], c[2] - bg[2]) > 0.4 * inkD || Math.abs(c[3] - aBg) > 25 ? null : c; };
  const ring = (pts) => { const v = pts.map(([x, y]) => get(x, y)).filter(Boolean); if (!v.length) return null; return [0, 1, 2, 3].map((k) => median(v.map((c) => c[k]))); };
  const L = [], Rr = [], T = [], B = [];
  for (let y = y0; y <= y1; y++) { L[y] = ring([[x0 - 1, y], [x0 - 2, y], [x0 - 1, y - 1], [x0 - 1, y + 1]]); Rr[y] = ring([[x1 + 1, y], [x1 + 2, y], [x1 + 1, y - 1], [x1 + 1, y + 1]]); }
  for (let x = x0; x <= x1; x++) { T[x] = ring([[x, y0 - 1], [x, y0 - 2], [x - 1, y0 - 1], [x + 1, y0 - 1]]); B[x] = ring([[x, y1 + 1], [x, y1 + 2], [x - 1, y1 + 1], [x + 1, y1 + 1]]); }
  // a ring pixel left out (ink, or hidden) takes its nearest neighbour along the side
  const gaps = (a, i0, i1) => { let last = null; const idx = []; for (let i = i0; i <= i1; i++) if (a[i]) idx.push(i); if (!idx.length) return;
    let k = 0; for (let i = i0; i <= i1; i++) { while (k + 1 < idx.length && Math.abs(idx[k + 1] - i) <= Math.abs(idx[k] - i)) k++; if (!a[i]) a[i] = a[idx[k]]; } void last; };
  gaps(L, y0, y1); gaps(Rr, y0, y1); gaps(T, x0, x1); gaps(B, x0, x1);
  // a side that crosses into something else (the end of a pill, a field border) while the opposite side is plain is left out
  const dev = (a) => median(a.filter(Boolean).map((c) => Math.hypot(c[0] - bg[0], c[1] - bg[1], c[2] - bg[2])));
  const dL = dev(L), dR = dev(Rr), dT = dev(T), dB = dev(B);
  const drop = (a) => a.fill(null);
  if (dL > 8 && dR < 4) drop(L); if (dR > 8 && dL < 4) drop(Rr); if (dT > 8 && dB < 4) drop(T); if (dB > 8 && dT < 4) drop(B);
  // only the ink and a 2 px fringe around it change: whatever else is in the rectangle (a rounded end, a border) stays
  const mw = x1 - x0 + 1, mh = y1 - y0 + 1, hit = new Uint8Array(mw * mh), mask = new Uint8Array(mw * mh);
  const v = [ink[0] - bg[0], ink[1] - bg[1], ink[2] - bg[2]], vv = v[0] * v[0] + v[1] * v[1] + v[2] * v[2] || 1;
  for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) { const i = (y * W + x) * 4; if (Math.abs(out[i + 3] - aBg) <= 25 && ((out[i] - bg[0]) * v[0] + (out[i + 1] - bg[1]) * v[1] + (out[i + 2] - bg[2]) * v[2]) / vv >= 0.12) hit[(y - y0) * mw + x - x0] = 1; }
  for (let y = 0; y < mh; y++) for (let x = 0; x < mw; x++) if (hit[y * mw + x]) for (let dy = -2; dy <= 2; dy++) for (let dx = -2; dx <= 2; dx++) { const X = x + dx, Y = y + dy; if (X >= 0 && Y >= 0 && X < mw && Y < mh) mask[Y * mw + X] = 1; }
  const w = x1 - x0 + 2, h = y1 - y0 + 2;
  const bg4 = [...bg, 255];
  const tl = T[x0] || L[y0] || bg4, tr = T[x1] || Rr[y0] || bg4, bl = B[x0] || L[y1] || bg4, br = B[x1] || Rr[y1] || bg4;
  for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) {
    if (!mask[(y - y0) * mw + x - x0] || skip(x, y)) continue;
    const u = (x - x0 + 1) / w, v = (y - y0 + 1) / h, i = (y * W + x) * 4;
    const hz = L[y] && Rr[y], vt = T[x] && B[x];
    for (let k = 0; k < 4; k++) { // the alpha too: a shadow over the window can be partly transparent in the capture
      let val;
      if (hz && vt) {
        val = (1 - u) * L[y][k] + u * Rr[y][k] + (1 - v) * T[x][k] + v * B[x][k]
          - ((1 - u) * (1 - v) * tl[k] + u * (1 - v) * tr[k] + (1 - u) * v * bl[k] + u * v * br[k]);
      } else if (hz) val = (1 - u) * L[y][k] + u * Rr[y][k];
      else if (vt) val = (1 - v) * T[x][k] + v * B[x][k];
      else val = (L[y] || Rr[y] || T[x] || B[x] || bg4)[k];
      out[i + k] = Math.max(0, Math.min(255, Math.round(val)));
    }
  }
}
function draw(out, W, H, r, ox, oy, ink, skip) {
  for (let y = 0; y < r.H; y++) for (let x = 0; x < r.W; x++) {
    const a = r.a[y * r.W + x] / 255; if (!a) continue;
    const X = x + ox, Y = y + oy; if (X < 0 || Y < 0 || X >= W || Y >= H || skip(X, Y)) continue;
    const i = (Y * W + X) * 4; for (let k = 0; k < 3; k++) out[i + k] = Math.round(out[i + k] * (1 - a) + ink[k] * a);
  }
}
// normalised correlation of two alpha maps placed in picture coordinates, both blurred by rb px (so that two typefaces with
// letters of slightly different widths still agree where the words, lines and weight agree)
function ncc(A, ax, ay, Bm, bx, by, rb = 0) {
  const x0 = Math.min(ax, bx) - rb, y0 = Math.min(ay, by) - rb, x1 = Math.max(ax + A.W, bx + Bm.W) + rb, y1 = Math.max(ay + A.H, by + Bm.H) + rb;
  const w = x1 - x0, h = y1 - y0; const P = new Float32Array(w * h), Q = new Float32Array(w * h);
  for (let y = 0; y < A.H; y++) for (let x = 0; x < A.W; x++) P[(y + ay - y0) * w + x + ax - x0] = A.a[y * A.W + x] / 255;
  for (let y = 0; y < Bm.H; y++) for (let x = 0; x < Bm.W; x++) Q[(y + by - y0) * w + x + bx - x0] = Bm.a[y * Bm.W + x] / 255;
  const blur = (M) => { if (!rb) return M; const T = new Float32Array(w * h), O = new Float32Array(w * h);
    for (let y = 0; y < h; y++) { let acc = 0; for (let x = -rb; x < w; x++) { if (x + rb < w) acc += M[y * w + x + rb]; if (x - rb - 1 >= 0) acc -= M[y * w + x - rb - 1]; if (x >= 0) T[y * w + x] = acc; } }
    for (let x = 0; x < w; x++) { let acc = 0; for (let y = -rb; y < h; y++) { if (y + rb < h) acc += T[(y + rb) * w + x]; if (y - rb - 1 >= 0) acc -= T[(y - rb - 1) * w + x]; if (y >= 0) O[y * w + x] = acc; } }
    return O; };
  const p = blur(P), q = blur(Q); let sab = 0, saa = 0, sbb = 0;
  for (let i = 0; i < p.length; i++) { sab += p[i] * q[i]; saa += p[i] * p[i]; sbb += q[i] * q[i]; }
  return sab / Math.sqrt(saa * sbb || 1);
}

// ---------- the whole picture ----------
// pic: PNG buffer or file; geo: collectNameText's result; newText(t): the text the website shows for t.
// opts.donor(el, theme): a fit of the same element from another screen, for an element a menu or sheet partly covers.
// Returns { png, lines, notes } (png null when nothing was repainted).
export async function repaintPicture(pic, geo, renderer, newText, opts = {}) {
  const notes = [];
  if (!geo.els.length) return { png: null, lines: 0, notes };
  const { data, info } = await sharp(pic).ensureAlpha().raw().toBuffer({ resolveWithObject: true });
  const W = info.width, H = info.height, s = W / geo.W;
  const img = { data, W, H }; const out = Buffer.from(data);
  let lines = 0;
  for (const el of geo.els) {
    const short = el.text.slice(0, 48).replace(/\s+/g, ' ');
    try {
      const sk = skipper(el, s, img);
      // a partly covered element takes its fit from a screen where it is whole; one that does not fit here (dimmed under a sheet) too
      const fromDonor = async () => { const d = opts.donor && await opts.donor(el); if (!d) return null; const a = await adopt(img, s, el, d, sk, renderer); if (a.why) { notes.push(`${short}: donor ${a.why}`); return null; } return a; };
      let fit = sk.cut ? await fromDonor() : null;
      if (!fit) { fit = await fitElement(img, s, el, renderer, sk); if (fit.why && !sk.cut) fit = (await fromDonor()) || fit; }
      if (fit.why) { notes.push(`${short}: ${fit.why}`); continue; }
      if (fit.alignUnsure && opts.donor) { const d = await opts.donor(el); if (d && d.TL.length > 1) fit.align = d.align; }
      const n = await applyFit(out, img, s, el, fit, renderer, newText, sk, notes);
      lines += n;
    } catch (err) { notes.push(`${short}: ${err.stack}`); }
  }
  if (!lines) return { png: null, lines, notes };
  return { png: await sharp(out, { raw: { width: W, height: H, channels: 4 } }).png({ compressionLevel: 9 }).toBuffer(), lines, notes };
}

// What of an element's box can be seen: its scroll area and region clip it; menus, sheets and popovers over it hide parts.
function skipper(el, s, img) {
  const occ = el.occ.map((o) => body(img, [Math.floor(o.x * s), Math.floor(o.y * s), Math.ceil((o.x + o.w) * s), Math.ceil((o.y + o.h) * s)], s));
  const cl = [Math.floor(el.clip.x * s), Math.floor(el.clip.y * s), Math.ceil((el.clip.x + el.clip.w) * s), Math.ceil((el.clip.y + el.clip.h) * s)];
  const skip = (x, y) => x < cl[0] || y < cl[1] || x >= cl[2] || y >= cl[3] || occ.some((o) => x >= o[0] && x < o[2] && y >= o[1] && y < o[3]);
  const b = el.box; let vis = 0, tot = 0;
  for (let y = Math.floor(b.y * s); y < (b.y + b.h) * s; y += 2) for (let x = Math.floor(b.x * s); x < (b.x + b.w) * s; x += 2) { tot++; if (!skip(x, y)) vis++; }
  return { skip, cl, vis: vis / Math.max(1, tot), cut: vis < 0.995 * tot };
}

// Place a line's original text in its style: baseline and pen from the ink; score: how well it then matches the pixels.
async function place(renderer, l) {
  const r = (await renderer.render([{ t: l.text, f: l.f, w: l.w, ls: l.ls }]))[0];
  l.base = l.rect[1] + l.A.ink.cy - (r.ink.cy - r.py); // picture y of the baseline
  l.penL = l.rect[0] + l.A.ink.left - (r.ink.left - r.px); // picture x of the pen, from the left ink edge
  l.inkL = l.rect[0] + l.A.ink.left; l.inkR = l.rect[0] + l.A.ink.right;
  const ox = Math.round(l.penL - r.px), oy = Math.round(l.base - r.py);
  l.score = ncc(l.A, l.rect[0], l.rect[1], r, ox, oy, Math.max(2, Math.round(l.f * 0.15)));
  return { r, ox, oy, score: l.score };
}

// The drawn body of a menu, popover or sheet: its region also holds the shadow around it, which is window content.
const bodies = new WeakMap();
function body(img, r, s) {
  if (!img) return r;
  let m = bodies.get(img); if (!m) bodies.set(img, m = new Map());
  const key = r.join(','); if (m.has(key)) return m.get(key);
  const { data, W, H } = img; const [x0, y0, x1, y1] = r.map((v, i) => Math.max(0, Math.min(i % 2 ? H : W, v)));
  const w = x1 - x0, h = y1 - y0; let out = [x0, y0, x1, y1];
  const col = modeColour(img, x0 + (w >> 2), y0 + (h >> 2), x1 - (w >> 2), y1 - (h >> 2), () => false);
  if (col && w > 40 && h > 40) {
    // from inside the body outward, the first pixel unlike the body after a run of body pixels: the edge (a border, then shadow)
    const near = (x, y) => dist(data, (y * W + x) * 4, col) < 12;
    const lim = Math.round(20 * s);
    const scan = (fixed, outer, dir, horiz) => { // dir: the outward direction
      let seen = 0;
      for (let k = lim; k >= 0; k--) { const p = outer - dir * k; if (horiz ? near(p, fixed) : near(fixed, p)) seen++; else if (seen >= 3) return k + 1; else seen = 0; }
      return 0;
    };
    const rows = [], cols = []; for (let q = 1; q < 8; q++) { rows.push(y0 + Math.round(h * (0.2 + 0.6 * q / 8))); cols.push(x0 + Math.round(w * (0.2 + 0.6 * q / 8))); }
    const ins = [median(rows.map((y) => scan(y, x0, -1, true))), median(cols.map((x) => scan(x, y0, -1, false))), median(rows.map((y) => scan(y, x1 - 1, 1, true))), median(cols.map((x) => scan(x, y1 - 1, 1, false)))].map((v) => Math.max(0, v - 2)); // the border (2 px) is the body's
    out = [x0 + ins[0], y0 + ins[1], x1 - ins[2], y1 - ins[3]];
  }
  m.set(key, out); return out;
}
const q4 = (v) => Math.round(v * 4) / 4;
// the size at which Inter's ink for the line's text spreads vertically as much as the original's (weight 500 as the reference)
async function fitSize(renderer, l, em) {
  let f = em; for (let it = 0; it < 4; it++) { const r = (await renderer.render([{ t: l.text, f: q4(f), w: 500, ls: 0 }]))[0]; const nf = f * l.A.ink.sy / Math.max(0.3, r.ink.sy); if (Math.abs(nf - f) < 0.1) { f = nf; break; } f = nf; }
  return q4(f);
}
export const dbg = [];

// Fit Inter to the element's drawn text. Returns { bg, ink, inkD, TL (the drawn lines), hiddenTail, align, overflow, R } or { why }.
export async function fitElement(img, s, el, renderer, sk) {
  const { W, H } = img; const skip = sk.skip;
  const b = el.box; const ins = el.field ? 3 * s : 0; // a field or bezel button: its border is not text
  const R = [Math.max(0, Math.floor(b.x * s + ins)), Math.max(0, Math.floor(b.y * s + (el.field ? ins : -1 * s))), Math.min(W, Math.ceil((b.x + b.w) * s - ins)), Math.min(H, Math.ceil((b.y + b.h) * s + (el.field ? -ins : 1 * s)))];
  if (R[2] - R[0] < 4 || R[3] - R[1] < 4) return { why: 'empty box' };
  if (sk.vis < 0.15) return { why: 'hidden' };
  const an = analyse(img, R, skip); if (!an) return { why: 'no ink' };
  // words and their predicted widths (Inter at a size from the line height)
  const text = el.text.replace(/\s+/g, ' ').trim(); const words = text.split(' ');
  const probe = (await renderer.render([{ t: text, f: 100, w: 450, ls: 0 }]))[0];
  const em = q4(100 * an.bh / Math.max(1, probe.ink.y1 - probe.ink.y0 + 1));
  const wr = await renderer.render(words.map((t) => ({ t, f: em, w: 450, ls: 0 })));
  const sp = await renderer.render([{ t: 'a a', f: em, w: 450, ls: 0 }, { t: 'aa', f: em, w: 450, ls: 0 }]);
  const gapW = Math.max(2, sp[0].adv - sp[1].adv);
  const seq = []; an.lines.forEach((ln, li) => ln.segs.forEach((g, k) => seq.push({ line: li, x0: g.x0, x1: g.x1, first: k === 0, last: k === ln.segs.length - 1 })));
  const pw = wr.map((r) => Math.max(1, r.ink.x1 - r.ink.x0 + 1));
  // text cut off: the whole text would not fit on the one drawn line, or a menu or sheet covers part of the box
  const full = pw.reduce((a, v) => a + v, 0) + gapW * (words.length - 1);
  let mt = null, k = 1;
  for (let it = 0; it < 4; it++) {
    mt = matchWords(seq, pw.map((v) => v * k), em, gapW * k, { cut: sk.cut, width: R[2] - R[0] });
    if (!mt) break;
    const ratios = mt.res.map((r, j) => (r && !r.hidden && r.how === 'one' && words[j].length > 2 ? (seq[r.i0].x1 - seq[r.i0].x0 + 1) / pw[j] : null)).filter(Boolean);
    if (!ratios.length) break; const nk = median(ratios); if (Math.abs(nk - k) < 0.01) break; k = nk;
  }
  if (process.env.REPAINT_DEBUG) console.log('MATCH', JSON.stringify({ text, em, k, full, Rw: R[2] - R[0], bh: an.bh, G: an.G, lines: an.lines.map((l) => [l.t, l.b, l.segs.map((g) => [g.x0, g.x1])]), pw, mt }));
  if (!mt || mt.cost > 0.3) return { why: `words do not match the pixels (${mt ? mt.cost.toFixed(2) : 'none'})` };
  let overflow = (an.lines.length === 1 && k * full > 1.1 * (R[2] - R[0])) || sk.cut;
  // words missing at the end: cut off on the last line (by its box, or a menu or sheet), or on further lines that something
  // covers (a toast): then the last line is full, the first missing word could not have fitted on it
  let covered = false;
  if (mt.hidden && !overflow) {
    const j = mt.res.findIndex((r) => r?.hidden), last = seq.filter((g) => mt.res.some((r) => r && !r.hidden && r.i1 >= 0 && seq[r.i1] === g)).pop();
    covered = !!last && last.x1 + gapW * k + pw[j] * k > R[2] + 2 && R[3] - an.lines[last.line].b > 0.8 * an.bh;
    if (!covered) return { why: `words missing from the pixels (${mt.hidden})` };
  }
  // the text of each drawn line, from the matched words (a wrapped word is cut where the widths say)
  const L = an.lines.map((ln) => ({ t: ln.t, b: ln.b, words: [], x0: Infinity, x1: -Infinity }));
  const hiddenTail = [];
  const extend = (ln, g) => { ln.x0 = Math.min(ln.x0, g.x0); ln.x1 = Math.max(ln.x1, g.x1); };
  for (let j = 0; j < words.length; j++) {
    const r = mt.res[j]; if (!r) continue;
    if (r.hidden) { hiddenTail.push(words[j]); continue; }
    if (r.how === 'wrap') {
      const wd = words[j], pieces = seq.slice(r.i0, r.i1 + 1), widths = pieces.map((g) => g.x1 - g.x0 + 1), tot = widths.reduce((a, v) => a + v, 0);
      const adv = await renderer.prefix({ t: wd, f: em, w: 450, ls: 0 }); const total = adv[adv.length - 1];
      let at = 0, acc = 0; const cuts = [];
      for (let q = 0; q < pieces.length - 1; q++) {
        acc += widths[q]; let bq = at + 1, be = Infinity;
        for (let c = at + 1; c < wd.length; c++) { const e = Math.abs(adv[c - 1] / total - acc / tot) * total - (/[\/\-._]/.test(wd[c - 1]) ? 0.6 * em : 0); if (e < be) { be = e; bq = c; } }
        cuts.push(bq); at = bq;
      }
      const parts = []; let from = 0; for (const c of [...cuts, wd.length]) { parts.push(wd.slice(from, c)); from = c; }
      pieces.forEach((g, q) => { const ln = L[g.line]; ln.words.push(parts[q]); extend(ln, g); if (q < pieces.length - 1) ln.joinNext = true; });
    } else {
      const a = seq[r.i0], z = seq[r.i1], ln = L[a.line];
      if (r.how !== 'merge2') ln.words.push(words[j]); else ln.words[ln.words.length - 1] += ' ' + words[j];
      extend(ln, a); extend(ln, z);
    }
  }
  const TL = L.filter((l) => l.words.length); if (!TL.length) return { why: 'no words drawn' };
  for (const l of TL) l.text = l.words.join(' ');
  // a line ending in punctuation the label has but the screen does not draw ("<title>. <description>" on two lines)
  for (const l of TL.slice(0, -1)) if (/[.,;:]$/.test(l.text)) {
    const [a, b] = await renderer.render([{ t: l.text, f: em, w: 450, ls: 0 }, { t: l.text.slice(0, -1), f: em, w: 450, ls: 0 }]);
    const wO = l.x1 - l.x0 + 1; if (Math.abs((b.ink.x1 - b.ink.x0 + 1) * k - wO) < Math.abs((a.ink.x1 - a.ink.x0 + 1) * k - wO)) { l.text = l.text.slice(0, -1); l.dropped = true; }
  }
  // ink colour and alpha of each line (a title and its caption can differ)
  const ink = inkColour(img, TL.map((l) => [l.x0, l.t, l.x1, l.b]), an.bg, skip);
  const inkD = Math.hypot(ink[0] - an.bg[0], ink[1] - an.bg[1], ink[2] - an.bg[2]);
  if (inkD < 30) return { why: 'ink too faint' };
  for (const l of TL) {
    const own = inkColour(img, [[l.x0, l.t, l.x1, l.b]], an.bg, skip);
    l.ink = Math.hypot(own[0] - ink[0], own[1] - ink[1], own[2] - ink[2]) > 24 && Math.hypot(own[0] - an.bg[0], own[1] - an.bg[1], own[2] - an.bg[2]) >= 30 ? own : ink;
    l.rect = [l.x0 - 2, l.t - 2, l.x1 + 2, l.b + 2]; l.A = alphaMap(img, l.rect, an.bg, l.ink, skip); inkOf(l.A);
  }
  // fit Inter to each line: size from the ink height, weight from the ink mass, letter spacing from the ink width
  for (const l of TL) {
    l.n = [...l.text].length;
    if (overflow && l === TL[TL.length - 1] && hiddenTail.length) continue; // its width is cut
    const wO = l.A.ink.right - l.A.ink.left;
    l.f = await fitSize(renderer, l, em);
    const ws = [300, 400, 500, 600, 700, 800]; const rs = await renderer.render(ws.map((w) => ({ t: l.text, f: l.f, w, ls: 0 })));
    const mO = l.A.ink.m; let wt = ws[ws.length - 1];
    if (mO <= rs[0].ink.m) wt = ws[0];
    else for (let q = 0; q < ws.length - 1; q++) { const m0 = rs[q].ink.m, m1 = rs[q + 1].ink.m; if (mO >= m0 && mO <= m1) { wt = ws[q] + (ws[q + 1] - ws[q]) * (mO - m0) / Math.max(1e-6, m1 - m0); break; } }
    l.w = Math.round(wt / 10) * 10;
    const r0 = (await renderer.render([{ t: l.text, f: l.f, w: l.w, ls: 0 }]))[0];
    l.ls = l.n > 1 ? Math.round(20 * Math.max(-0.06 * l.f, Math.min(0.08 * l.f, (wO - (r0.ink.right - r0.ink.left)) / (l.n - 1)))) / 20 : 0;
    l.own = (await place(renderer, l)).score; // how well the line's own style fits (a line cut by a scroll area fits badly)
  }
  // one style per element unless a line clearly differs (a card with a title and a description); short lines take the element's
  const good = TL.filter((l) => l.f && l.own >= 0.7), fitted = good.filter((l) => l.n >= 6); const ref = fitted.length ? fitted : good;
  if (!ref.length) {
    // only a cut line: its size from the ink height, the rest from the typical system text
    const l = TL[0]; l.f = await fitSize(renderer, l, em); const ws = [300, 400, 500, 600, 700, 800]; const rs = await renderer.render(ws.map((w) => ({ t: l.text, f: l.f, w, ls: 0 })));
    let bw = 500, be = Infinity; rs.forEach((r, q) => { const e = Math.abs(r.ink.m - l.A.ink.m); if (e < be) { be = e; bw = ws[q]; } }); l.w = bw; l.ls = 0; ref.push(l);
  }
  const mf = median(ref.map((l) => l.f)), mw = median(ref.map((l) => l.w)), ml = median(ref.map((l) => l.ls));
  for (const l of TL) {
    const own = l.f && l.n >= 6 && l.own >= 0.7 && (Math.abs(l.f / mf - 1) > 0.1 || Math.abs(l.w - mw) > 120); // a different style
    if (!own) { l.f = mf; l.w = mw; l.ls = ml; }
  }
  // place each line's original text: baseline and pen position from the ink, and check the fit
  for (const l of TL) {
    const { r, ox, oy } = await place(renderer, l);
    if (process.env.REPAINT_DEBUG) dbg.push({ text: l.text, A: l.A, ax: l.rect[0], ay: l.rect[1], r, ox, oy, f: l.f, w: l.w, ls: l.ls, score: l.score, bg: an.bg, ink, segs: an.lines.map((q) => q.segs.length) });
  }
  // (a line that fits badly is only a problem if it has to be redrawn: applyFit checks)
  // alignment: left, centred or right
  const bx0 = b.x * s, bx1 = (b.x + b.w) * s;
  let align = 'left';
  if (TL.length > 1) {
    const spread = (a) => Math.max(...a) - Math.min(...a);
    const ls = TL.map((l) => l.inkL), rs = TL.map((l) => l.inkR), cs = TL.map((l) => (l.inkL + l.inkR) / 2);
    align = spread(ls) <= 4 * s ? 'left' : spread(cs) <= 3 * s ? 'centre' : spread(rs) <= 3 * s ? 'right' : 'left';
  } else if (!(overflow && hiddenTail.length)) {
    const l = TL[0], gl = (l.inkL - bx0) / s, gr = (bx1 - l.inkR) / s, c = (l.inkL + l.inkR) / 2 / s;
    if (gl > 4 && gr > 4 && Math.abs(gl - gr) < 3) align = 'centre';
    else if (gl > 8 && gr < 3) align = 'right';
    else if (gl < 4 && gr < 4 && el.centres.some(([cx, w]) => w > b.w + 20 && Math.abs(cx - c) < 1.5)) align = 'centre';
  }
  // one full-width line of a box made for more (the rest covered): left or centred cannot be told here
  const alignUnsure = TL.length === 1 && b.h * s > 1.7 * (TL[0].b - TL[0].t + 1) && (TL[0].inkL - bx0) / s < 4 && (bx1 - TL[0].inkR) / s < 4;
  for (const l of TL) delete l.A;
  return { bg: an.bg, ink, inkD, TL, hiddenTail: covered ? [] : hiddenTail, align, alignUnsure, overflow, R };
}

// A fit from another screen, checked against this picture (the same text must be in the same place) and given this picture's colours.
async function adopt(img, s, el, fit, sk, renderer) {
  const { skip } = sk; const TL = fit.TL.map((l) => ({ ...l }));
  const bg = modeColour(img, fit.R[0], fit.R[1], fit.R[2], fit.R[3], skip); if (!bg) return { why: 'hidden' };
  const vis = TL.map((l) => [l.x0, l.t, l.x1, l.b]);
  const seen = (rs) => { let n = 0; for (const [x0, y0, x1, y1] of rs) for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) if (!skip(x, y)) n++; return n; };
  const ink = seen(vis) > 400 ? inkColour(img, vis, bg, skip) : fit.ink;
  const inkD = Math.hypot(ink[0] - bg[0], ink[1] - bg[1], ink[2] - bg[2]); if (inkD < 30) return { why: 'ink too faint' };
  for (const l of TL) {
    const own = seen([[l.x0, l.t, l.x1, l.b]]) > 400 ? inkColour(img, [[l.x0, l.t, l.x1, l.b]], bg, skip) : null;
    l.ink = l.ink === fit.ink ? ink : own && Math.hypot(own[0] - bg[0], own[1] - bg[1], own[2] - bg[2]) >= 30 ? own : ink;
    const A = alphaMap(img, [l.x0 - 2, l.t - 2, l.x1 + 2, l.b + 2], bg, l.ink, skip);
    const r = (await renderer.render([{ t: l.text, f: l.f, w: l.w, ls: l.ls }]))[0];
    const ox = Math.round(l.penL - r.px), oy = Math.round(l.base - r.py);
    const masked = { W: r.W, H: r.H, a: Uint8Array.from(r.a, (v, i) => (skip(ox + i % r.W, oy + Math.floor(i / r.W)) ? 0 : v)) };
    if (masked.a.every((v) => !v)) continue; // this line is wholly covered here
    const sc = ncc(A, l.x0 - 2, l.t - 2, masked, ox, oy, Math.max(2, Math.round(l.f * 0.15)));
    if (sc < 0.7) return { why: `does not match here (${sc.toFixed(2)})` };
  }
  return { ...fit, bg, ink, inkD, TL };
}

// Erase the changed lines and draw the new ones.
async function applyFit(out, img, s, el, fit, renderer, newText, sk, notes) {
  const { W, H } = img; const { skip, cl } = sk; const { TL, hiddenTail, align, overflow, ink, inkD, bg } = fit;
  const short = el.text.slice(0, 48).replace(/\s+/g, ' ');
  const text = el.text.replace(/\s+/g, ' ').trim();
  const bx1 = (el.box.x + el.box.w) * s;
  const oldLines = TL.map((l, i) => l.text + (i === TL.length - 1 && hiddenTail.length ? ' ' + hiddenTail.join(' ') : ''));
  const newLines = oldLines.map((t) => newText(t));
  const norm = (t) => t.replace(/\s+/g, ' ').trim();
  const joined = (a) => a.reduce((acc, t, i) => acc + (i && !TL[i - 1].joinNext ? ' ' : '') + t, '');
  const fullNew = newText(joined(oldLines)); // the drawn text (without label punctuation the screen does not draw, or covered words)
  const first = newLines.findIndex((t, i) => t !== oldLines[i]);
  if (first < 0) { notes.push(`${short}: no visible change`); return 0; }
  const lineByLine = norm(joined(newLines)) === norm(fullNew) && !TL.some((l, i) => l.joinNext && (newLines[i] !== oldLines[i] || newLines[i + 1] !== oldLines[i + 1]));
  // the visible right end of the text: a line cut by its box keeps whole words only
  const visR = Math.min(bx1, cl[2], ...el.occ.map((o) => o.x * s).filter((x) => x > TL[0].inkL));
  let plan;
  if (lineByLine) plan = TL.map((l, i) => (newLines[i] !== oldLines[i] ? { l, st: l, t: newLines[i], base: l.base } : null)).filter(Boolean);
  else {
    // keep the lines before the first change, re-wrap the rest to the box
    let rest = norm(fullNew);
    if (first) { const keepNew = norm(newText(joined(oldLines.slice(0, first)))); if (!rest.startsWith(keepNew)) { notes.push(`${short}: cannot keep the lines before the change`); return 0; } rest = rest.slice(keepNew.length).trim(); }
    const l0 = TL[first];
    const pitch = TL.length > 1 ? median(TL.slice(1).map((l, i) => l.base - TL[i].base)) : (el.lh || 16) * s;
    const inset = el.kind === 'label' ? Math.max(0, l0.inkL - el.box.x * s) : 0; // text inside a container: the same margin on the right
    const max = bx1 - Math.min(inset, 12 * s) - l0.penL;
    const wrapped = await renderer.wrap({ t: rest, f: l0.f, w: l0.w, ls: l0.ls, max });
    if (!wrapped.length) return 0;
    if (first + wrapped.length > TL.length) notes.push(`${short}: re-wrapped text needs ${first + wrapped.length} lines, was ${TL.length}`);
    plan = wrapped.map((t, i) => ({ l: TL[Math.min(first + i, TL.length - 1)], st: l0, t, base: l0.base + i * pitch }));
  }
  // erase the original ink of the changed lines
  const eraseLines = lineByLine ? plan.map((p) => p.l) : TL.slice(first);
  const bad = eraseLines.filter((l) => !(l.score >= 0.7));
  if (bad.length) { notes.push(`${short}: poor fit ${bad.map((l) => l.score?.toFixed(2)).join(',')} "${bad[0].text.slice(0, 30)}"`); return 0; }
  for (const l of eraseLines) {
    const i = TL.indexOf(l), prev = TL[i - 1], next = TL[i + 1];
    const top = Math.max(l.t - 3, prev ? Math.ceil((prev.b + l.t) / 2) + 1 : 0), bot = Math.min(l.b + 3, next ? Math.floor((l.b + next.t) / 2) - 1 : H);
    const cut = i === TL.length - 1 && hiddenTail.length; // the line runs to the end of its box: nothing past it is the text's
    erase(out, W, H, [Math.max(l.x0 - 3, fit.R[0] - 2), top, Math.min(l.x1 + 3, cut ? fit.R[2] - 1 : fit.R[2] + 2), bot], bg, inkD, skip, ink);
  }
  // draw the new lines
  for (const p of plan) {
    const st = p.st;
    let t = p.t;
    const job = (tt, fx = 0, fy = 0) => ({ t: tt, f: st.f, w: st.w, ls: st.ls, fx, fy });
    let r = (await renderer.render([job(t)]))[0];
    // a line cut by its own box keeps whole words only
    if (overflow && hiddenTail.length && p.l === TL[TL.length - 1] && align === 'left') {
      while (t.includes(' ') && st.penL + (r.ink.right - r.px) > visR - 1) { t = t.slice(0, t.lastIndexOf(' ')); r = (await renderer.render([job(t)]))[0]; }
    }
    let pen;
    if (align === 'left') pen = st.penL;
    else { const oc = (p.l.inkL + p.l.inkR) / 2; pen = align === 'centre' ? oc - ((r.ink.left + r.ink.right) / 2 - r.px) : p.l.inkR - (r.ink.right - r.px); }
    const fx = q4(pen - Math.floor(pen)), fy = q4(p.base - Math.floor(p.base));
    const rr = (await renderer.render([job(t, fx, fy)]))[0];
    draw(out, W, H, rr, Math.floor(pen) - rr.px, Math.floor(p.base) - rr.py, st.ink || ink, skip);
  }
  notes.push(`${short}: ${plan.length} line(s) ${align} ${lineByLine ? 'in place' : 're-wrapped'} f=${TL[0].f} w=${TL[0].w} ls=${TL[0].ls} fit=${TL.map((l) => l.score?.toFixed(2)).join(',')}`);
  return plan.length;
}

// Finds, for an element that a menu or sheet partly covers here, the same element (same text, same box) fully visible in another
// screen of the same theme, and fits it there. list(theme) gives the candidate screens, nearest first: [{ html, png }].
// page is a Playwright page the finder may navigate.
export function donorFinder({ page, renderer, list }) {
  const fits = new Map(), html = new Map();
  const esc = (t) => t.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
  return (theme, rec) => (el) => {
    const key = [theme, el.text, ...['x', 'y', 'w', 'h'].map((k) => Math.round(el.box[k]))].join('|');
    if (!fits.has(key)) fits.set(key, (async () => {
      const snip = esc(el.text.slice(0, 40)), snip2 = el.text.slice(0, 40);
      for (const c of list(theme, rec)) {
        if (!html.has(c.html)) html.set(c.html, fs.existsSync(c.html) ? fs.readFileSync(c.html, 'utf8') : '');
        const h = html.get(c.html); if (!h.includes(snip) && !h.includes(snip2)) continue;
        await page.goto('file://' + c.html);
        const geo = await page.evaluate(collectNameText, { rx: NAME_RX });
        const d = geo.els.find((e) => e.text === el.text && ['x', 'y', 'w', 'h'].every((k) => Math.abs(e.box[k] - el.box[k]) < 0.6));
        if (!d) continue;
        const { data, info } = await sharp(c.png).ensureAlpha().raw().toBuffer({ resolveWithObject: true });
        const dimg = { data, W: info.width, H: info.height }; const s = info.width / geo.W, sk = skipper(d, s, dimg); if (sk.cut) continue;
        const fit = await fitElement(dimg, s, d, renderer, sk);
        if (!fit.why) return fit;
      }
      return null;
    })());
    return fits.get(key);
  };
}

export const hasName = hasOldName;
export const nameRx = NAME_RX;
