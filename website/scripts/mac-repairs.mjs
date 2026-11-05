// Recorded repairs of the Mac capture of 2026-11-05 (research/MacHTML-src is never edited and no Mac is available to recapture).
// macRepair returns the repaired PNG of a window picture, or null when the picture needs no repair. Pictures are 1 image pixel per CSS pixel.
import fs from 'node:fs';
import path from 'node:path';
import sharp from 'sharp';

const clamp = (v, lo, hi) => Math.min(hi, Math.max(lo, v));

// ---------- light script page: the Dark page option was left on in the capture of screens 080 to 107 ----------
const PAGE_DARK = [30, 30, 34]; // the dark page as captured
const PAGE_DIMMED = [112, 112, 112]; // the same page dimmed behind a sheet
const DIM_K = (188 - 112) / (255 - 30), DIM_C = 112 - 30 * DIM_K; // dimming seen on the page: v -> v * K + C, from 30 -> 112 and white -> 188
// Dark page tone to white page tone, measured on 101-screenplay-editor-selection against 068-screenplay-editor, which hold the same text at the same place.
const TONE = [[30, 255], [40, 254], [52, 250], [63, 243], [72, 238], [87, 230], [103, 214], [116, 200], [131, 186], [147, 174], [160, 163], [178, 141], [197, 121], [208, 104], [218, 94], [225, 83], [230, 70], [233, 56], [236, 24], [255, 20]];
const tone = (v) => {
  for (let i = 1; i < TONE.length; i++) if (v <= TONE[i][0]) { const [a, x] = TONE[i - 1], [b, y] = TONE[i]; return x + (y - x) * (v - a) / (b - a); }
  return 20;
};
const SEL_DARK = [64, 106, 196], SEL_LIGHT = [179, 215, 255]; // selected text: the dark page colour as captured, the white page colour from 130-screenplay-comment-composer-quote

// A window pixel of the page, as the white page would show it. Below the page tone it is a shadow of something drawn above: multiply, never invert.
function pagePixel(r, g, b, dimmed) {
  const bg = dimmed ? PAGE_DIMMED[0] : PAGE_DARK[0], white = dimmed ? 188 : 255;
  if (!dimmed && b - r > 15) { // selection: white text on blue
    if (r < SEL_DARK[0]) return SEL_LIGHT.map((c) => c * r / SEL_DARK[0]);
    const t = (r - SEL_DARK[0]) / (255 - SEL_DARK[0]);
    return SEL_LIGHT.map((c) => c * (1 - t) + 20 * t);
  }
  if (r < bg) { const o = white * r / bg; return [o, o, o]; }
  const o = dimmed ? tone((r - DIM_C) / DIM_K) * DIM_K + DIM_C : tone(r);
  return [o, o, o];
}

// Signed distance to a rounded rectangle, negative inside.
const roundedDist = (s, px, py) => {
  const qx = Math.abs(px - (s.x + s.w / 2)) - s.w / 2 + s.r, qy = Math.abs(py - (s.y + s.h / 2)) - s.h / 2 + s.r;
  return Math.hypot(Math.max(qx, 0), Math.max(qy, 0)) + Math.min(Math.max(qx, qy), 0) - s.r;
};

// The sheets and menus drawn above the page, from the regions of the capture page. A menu's frame has 6 clear points each side.
function overlays(sourcePngPath, extra = []) {
  const html = fs.readFileSync(sourcePngPath.replace(/[\\/]assets[\\/]([^\\/]+)\.png$/, '/$1.html'), 'utf8');
  const shapes = [...extra];
  for (const m of html.matchAll(/class="region (sheet|menu|popover)"[^>]*?style="left:(-?\d+)px;top:(-?\d+)px;width:(\d+)px;height:(\d+)px/g)) {
    const [x, y, w, h] = m.slice(2, 6).map(Number);
    shapes.push(m[1] === 'sheet' ? { x, y, w, h, r: 26 } : { x: x + 6, y: y - 0.5, w: w - 12, h, r: 10 });
  }
  return shapes;
}

// The rectangle of the page: every column and row with a good run of page tone. Nothing else in these windows holds that exact tone that often.
function findPane(data, W, H, tint) {
  const cols = new Uint16Array(W), rows = new Uint16Array(H);
  for (let y = 0, i = 0; y < H; y++) for (let x = 0; x < W; x++, i += 4) if (data[i] === tint[0] && data[i + 1] === tint[1] && data[i + 2] === tint[2]) { cols[x]++; rows[y]++; }
  const first = (a) => a.findIndex((n) => n >= 40), last = (a) => a.length - 1 - [...a].reverse().findIndex((n) => n >= 40);
  return first(cols) < 0 ? null : { x0: first(cols), x1: last(cols), y0: first(rows), y1: last(rows) };
}

function lightPage(data, W, H, sourcePngPath, rep) {
  let pane = findPane(data, W, H, PAGE_DARK), dimmed = false;
  if (!pane) { pane = findPane(data, W, H, PAGE_DIMMED); dimmed = true; }
  if (!pane) throw new Error('mac-repairs: no dark page in ' + sourcePngPath);
  if (rep.paneX0 !== undefined) pane.x0 = rep.paneX0;
  const shapes = overlays(sourcePngPath, rep.shapes), src = Buffer.from(data);
  const at = (x, y, c) => src[(clamp(y, 0, H - 1) * W + clamp(x, 0, W - 1)) * 4 + c];
  const covered = (x, y) => shapes.some((s) => roundedDist(s, x + 0.5, y + 0.5) < 2.5);
  const pageTone = dimmed ? PAGE_DIMMED[0] : PAGE_DARK[0];
  // How much a shadow from above darkens the page here, from the page tones around the pixel. Averaged: on the dark page a shadow has
  // only a few tones, and multiplied up they would show as contour lines.
  const shadowAt = (x, y) => {
    let sum = 0, n = 0;
    for (let sy = y - 3; sy <= y + 3; sy++) for (let sx = x - 3; sx <= x + 3; sx++) if (at(sx, sy, 0) <= pageTone && !covered(sx, sy)) { sum += at(sx, sy, 0); n++; }
    return n ? sum / n / pageTone : 1;
  };
  for (let y = pane.y0; y <= pane.y1; y++) for (let x = pane.x0; x <= pane.x1; x++) {
    const i = (y * W + x) * 4;
    let edge = null, inside = false, near = false;
    for (const s of shapes) {
      const d = roundedDist(s, x + 0.5, y + 0.5);
      if (d <= -3) { inside = true; break; }
      // the anti-aliased rim: wider at the corners, where the radius is only an estimate; on a straight edge the text above it stays
      const corner = Math.abs(x + 0.5 - (s.x + s.w / 2)) > s.w / 2 - s.r && Math.abs(y + 0.5 - (s.y + s.h / 2)) > s.h / 2 - s.r;
      if (d < (corner ? 2.5 : 0.8)) { edge = s; break; }
      if (d < 45) near = true;
    }
    if (inside) continue;
    if (edge) { // edge of the overlay: its colour over the white page, by how much of it covers the pixel
      const d0 = roundedDist(edge, x + 0.5, y + 0.5), e = 0.5, nx = roundedDist(edge, x + 0.5 + e, y + 0.5) - roundedDist(edge, x + 0.5 - e, y + 0.5), ny = roundedDist(edge, x + 0.5, y + 0.5 + e) - roundedDist(edge, x + 0.5, y + 0.5 - e);
      const out = Math.round(3 - d0), inn = Math.round(3 + d0);
      const ox = x + Math.round(nx * out), oy = y + Math.round(ny * out), ix = x - Math.round(nx * inn), iy = y - Math.round(ny * inn);
      const backR = Math.min(at(ox, oy, 0), pageTone);
      const under = at(ox, oy, 2) - at(ox, oy, 0) > 15 ? pagePixel(at(ox, oy, 0), at(ox, oy, 1), at(ox, oy, 2), dimmed) : pagePixel(backR, backR, backR, dimmed); // a selection reaches under the corner of a menu
      const cov = at(ix, iy, 0) > backR ? clamp((src[i] - backR) / (at(ix, iy, 0) - backR), 0, 1) : 1;
      for (let c = 0; c < 3; c++) data[i + c] = Math.round(cov * at(ix, iy, c) + (1 - cov) * under[c]);
      continue;
    }
    const chroma = src[i + 2] - src[i] > 15, shade = near && !chroma ? shadowAt(x, y) : 1; // text under a shadow: take the shadow off, change the tone, put it back
    const o = shade < 1 ? pagePixel(Math.max(src[i] / shade, pageTone), 0, 0, dimmed).map((c) => c * shade) : pagePixel(src[i], src[i + 1], src[i + 2], dimmed);
    for (let c = 0; c < 3; c++) data[i + c] = Math.round(clamp(o[c], 0, 255));
  }
}

// ---------- text drawn into a picture ----------
const FONT = 'Segoe UI, Arial, Helvetica, sans-serif'; // the Mac's own font is not available on the machine that builds the pages
const esc = (t) => t.replace(/&/g, '&amp;').replace(/</g, '&lt;');

// Fill a rectangle row by row with the colour just left of it, and return the ink colour that was there (furthest from the background).
function eraseText(data, W, { x, y, w, h }) {
  let ink = null, far = -1;
  for (let py = y; py < y + h; py++) {
    const bgI = (py * W + x - 1) * 4;
    for (let px = x; px < x + w; px++) {
      const i = (py * W + px) * 4, dist = Math.abs(data[i] - data[bgI]) + Math.abs(data[i + 1] - data[bgI + 1]) + Math.abs(data[i + 2] - data[bgI + 2]);
      if (dist > far) { far = dist; ink = [data[i], data[i + 1], data[i + 2]]; }
      for (let c = 0; c < 4; c++) data[i + c] = data[bgI + c];
    }
  }
  return ink;
}

// Text at 4x, scaled down: no hinting, as the Mac draws it, with a thin stroke for its heavier weight. runs: [[text, weight], ...]
async function textLayer(runs, size, ink, clipW) {
  const S = 4, H = Math.ceil(size * 1.6);
  const spans = runs.map(([t, weight]) => `<tspan font-weight="${weight}">${esc(t)}</tspan>`).join('');
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${S * (clipW + 8)}" height="${S * H}"><text x="${S}" y="${S * (size * 1.15)}" font-family="${FONT}" font-size="${S * size}" fill="rgb(${ink.join(",")})" stroke="rgb(${ink.join(",")})" stroke-width="${ink[0] > 128 ? 1.1 : 0.7}" xml:space="preserve">${spans}</text></svg>`;
  const big = await sharp(Buffer.from(svg)).png().toBuffer();
  const trimmed = await sharp(big).trim({ threshold: 1 }).toBuffer({ resolveWithObject: true }); // measures the ink
  const inkW = Math.ceil((trimmed.info.trimOffsetLeft ? -trimmed.info.trimOffsetLeft : 0) + trimmed.info.width) / S;
  const layer = await sharp(big).resize(clipW + 8, H, { kernel: 'lanczos3' }).png().toBuffer();
  return { layer, inkW, baseline: size * 1.15 };
}

// lines: [{ runs, size, x | center, base, clip }]. x is where the first letter starts, base the baseline row.
async function drawLines(data, W, H, lines, ink) {
  const composites = [];
  for (const l of lines) {
    const clip = l.clip ?? 420;
    const { layer, inkW, baseline } = await textLayer(l.runs, l.size, ink, clip);
    const left = Math.round((l.center !== undefined ? l.center - inkW / 2 : l.x) - 1), top = Math.round(l.base - baseline);
    const crop = await sharp(layer).extract({ left: 0, top: 0, width: Math.min(clip + 1, W - left), height: Math.min(Math.ceil(l.size * 1.6), H - top) }).png().toBuffer();
    composites.push({ input: crop, left, top });
  }
  return sharp(data, { raw: { width: W, height: H, channels: 4 } }).composite(composites).png({ compressionLevel: 9 }).toBuffer();
}

// ---------- the table ----------
const pathRepair = (file, geom) => ({ kind: 'path', file, geom });
const FIELD = { light: { x: 548, y: 360, w: 420, h: 18, left: 550, base: 374, size: 13, clip: 416 }, dark: { x: 548, y: 360, w: 420, h: 18, left: 550, base: 374, size: 13, clip: 416 } };
const SMALL = (x, y, w, dx, dw, centered) => ({
  light: { x: x - 2, y, w: w + 4, h: 13, ...(centered ? { center: x + w / 2 } : { left: x }), base: y + 10, size: 11.2 },
  dark: { x: dx - 2, y, w: dw + 4, h: 13, ...(centered ? { center: dx + dw / 2 } : { left: dx }), base: y + 10, size: 11.2 },
});
const LIGHT_PAGE = { kind: 'lightPage', themes: ['light'] };

export const MAC_REPAIRS = {
  // Test folder name 'capture-packages' (and 'Straft Demo 2' in dark) drawn in the save path field.
  'package-backup-ready': pathRepair('Night Bus backup.ofbackup', FIELD),
  'package-backup-portable': pathRepair('Night Bus backup.ofbackup', FIELD),
  // The same test folder name in the centred line under "Backup created."
  'package-backup-created': pathRepair('Night Bus backup.ofbackup', SMALL(609, 425, 383, 604, 392, true)),
  // The same test folder name in the path under the export result.
  'package-export-full-project-done': pathRepair('Night Bus.ofproject', SMALL(544, 446, 343, 544, 352, false)),
  // The same test folder name in the path under the share result.
  'package-share-script-done': pathRepair('Night Bus review.ofscriptreview', SMALL(524, 434, 401, 524, 410, false)),
  // The same test folder name in the centred line under "Response package exported."
  'package-review-response-exported': pathRepair('Night Bus reply.ofresponse', SMALL(610, 162, 380, 606, 389, true)),
  // Raw markdown stars shown in the Location and Cast lines of the strip drawer.
  'schedule-strip-drawer': {
    kind: 'text',
    erase: { x: 1266, y: 280, w: 316, h: 56 },
    lines: [
      { runs: [['Location:', 600], [' Kesarganj Junction - Lost Property Office', 400]], x: 1266, base: 292, size: 13 },
      { runs: [['(Idea)', 400]], x: 1266, base: 307, size: 13 },
      { runs: [['Cast:', 600], [' Kiran, Devi', 400]], x: 1266, base: 332, size: 13 },
    ],
  },
  // An empty toast shell left at the bottom centre of the window.
  'pages-files-list': { kind: 'paintRows', themes: ['light'], rect: { x: 625, y: 745, w: 350, h: 47 } },
  // A focus ring drawn around the Folders label: the same label without the ring, 32 rows higher, in a neighbour screen.
  'vault-collection-empty': { kind: 'patch', themes: ['dark'], rect: { x: 236, y: 284, w: 200, h: 34 }, from: '320-vault-search-none.png', dy: -32 },
  'vault-collection-with-item': { kind: 'patch', themes: ['dark'], rect: { x: 236, y: 284, w: 200, h: 34 }, from: '320-vault-search-none.png', dy: -32 },
  // The same ring, dimmed behind the sheet: the neighbour sheet has the label at the same place.
  'vault-add-to-collection-sheet': { kind: 'patch', themes: ['dark'], rect: { x: 236, y: 284, w: 200, h: 34 }, from: '326-vault-rename-collection-sheet.png', dy: 0 },
  // Dark page option left on in the capture: the script page is dark on a light window. Screens that show the option on purpose are not listed.
  ...Object.fromEntries([
    'screenplay-tools-writing-room-mode', 'screenplay-tools-import-screenplay', 'screenplay-tools-export-screenplay', 'screenplay-draft-menu',
    'screenplay-draft-new-draft', 'screenplay-draft-manage-drafts', 'screenplay-element-picker', 'screenplay-element-picker-dialogue', 'screenplay-comments',
    'screenplay-compare-drafts', 'screenplay-find-and-replace', 'screenplay-picker-menu', 'screenplay-new-screenplay-sheet', 'screenplay-new-screenplay-duplicate',
    'screenplay-rename-screenplay-sheet', 'screenplay-delete-screenplay-sheet', 'screenplay-navigator-columns-menu',
    'screenplay-navigator-columns-shown', 'screenplay-scene-row-context', 'screenplay-editor-context', 'screenplay-editor-selection',
    'screenplay-editor-context-selection', 'screenplay-editor-context-spelling', 'screenplay-zoom-50', 'screenplay-zoom-150', 'screenplay-zoom-200',
  ].map((n) => [n, LIGHT_PAGE])),
  // The same, with the scene list hidden: the page runs under the sidebar's soft edge, a band of slightly lighter tones.
  'screenplay-scenes-hidden': { ...LIGHT_PAGE, paneX0: 228 },
  // The same, in the focus layout; the floating title pill is drawn above the page and is not in the page regions.
  'screenplay-layout-focus': { ...LIGHT_PAGE, shapes: [{ x: 1281, y: 67.5, w: 304, h: 33, r: 14 }] },
};

export async function macRepair(name, theme, sourcePngPath) {
  const rep = MAC_REPAIRS[name];
  if (!rep || (rep.themes && !rep.themes.includes(theme))) return null;
  const { data, info } = await sharp(sourcePngPath).ensureAlpha().raw().toBuffer({ resolveWithObject: true });
  const W = info.width, H = info.height;
  const out = () => sharp(data, { raw: { width: W, height: H, channels: 4 } }).png({ compressionLevel: 9 }).toBuffer();
  if (rep.kind === 'lightPage') { lightPage(data, W, H, sourcePngPath, rep); return out(); }
  if (rep.kind === 'path') {
    const g = rep.geom[theme], ink = eraseText(data, W, g);
    const line = { runs: [[`/Users/Shared/Straft Demo/Files/Packages/${rep.file}`, 400]], size: g.size, base: g.base, clip: g.clip, ...(g.center !== undefined ? { center: g.center } : { x: g.left }) };
    return drawLines(data, W, H, [line], ink);
  }
  if (rep.kind === 'text') { const ink = eraseText(data, W, rep.erase); return drawLines(data, W, H, rep.lines, ink); }
  if (rep.kind === 'paintRows') { // each row: a straight blend between the colours either side of the rectangle
    const { x, y, w, h } = rep.rect;
    for (let py = y; py < Math.min(y + h, H); py++) for (let px = x; px < x + w; px++) for (let c = 0; c < 3; c++) {
      const a = data[(py * W + x - 1) * 4 + c], b = data[(py * W + x + w) * 4 + c];
      data[(py * W + px) * 4 + c] = Math.round(a + (b - a) * (px - x + 1) / (w + 1));
    }
    return out();
  }
  if (rep.kind === 'patch') {
    const { data: from } = await sharp(path.join(path.dirname(sourcePngPath), rep.from)).ensureAlpha().raw().toBuffer({ resolveWithObject: true });
    const { x, y, w, h } = rep.rect;
    for (let py = y; py < y + h; py++) from.copy(data, (py * W + x) * 4, ((py + rep.dy) * W + x) * 4, ((py + rep.dy) * W + x + w) * 4);
    return out();
  }
  throw new Error('mac-repairs: unknown kind ' + rep.kind);
}
