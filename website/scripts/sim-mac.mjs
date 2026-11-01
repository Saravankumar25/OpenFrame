// Simulated Mac screens (owner decision 2026-11-01): the Mac captures never ran the AI assistant (the build had no Offline AI), so the
// AI chat, proposals and the Offline AI details have no real Mac screen. Each simulated screen is a real Mac capture (window, sidebar,
// toolbar) whose AI panel or Offline AI pane is redrawn as live HTML in the Mac look, with the same words as the Windows screens.
// The captured screens stay as they are (the Mac manual describes them); the simulated ones are new screens. Run after shots.mjs: node scripts/sim-mac.mjs
// Writes: .cache/shots/pages/mac/<theme>/sim-*.html, manifest entries, and the records appended to research/screens-mac.json (sim: true).
import fs from 'node:fs'; import path from 'node:path'; import sharp from 'sharp';
import { chromium } from 'playwright'; import { pathToFileURL } from 'node:url';
import { SIMS, SIM_AREA, panelCss, settingsCss, TOKENS } from './sim-mac-screens.mjs';
import { FREE_SIMS, freeCss, ACCENT } from './sim-mac-free.mjs';

const ROOT = path.resolve(import.meta.dirname, '..');
const SHOTS = path.join(ROOT, '.cache', 'shots'), PAGES = path.join(SHOTS, 'pages', 'mac'), MANIFEST = path.join(SHOTS, 'manifest.json');
const CATFILE = path.join(ROOT, 'research', 'screens-mac.json');
const readJson = (f) => JSON.parse(fs.readFileSync(f, 'utf8'));
const manifest = readJson(MANIFEST);
const cat = readJson(CATFILE).filter((s) => !s.sim);
const byName = new Map(cat.map((s) => [s.name, s]));
const hex = (p) => '#' + [...p].map((v) => v.toString(16).padStart(2, '0')).join('');
const rgba = (h) => ({ r: parseInt(h.slice(1, 3), 16), g: parseInt(h.slice(3, 5), 16), b: parseInt(h.slice(5, 7), 16), alpha: 1 });

// positions, in window CSS px, of the AI panel parts of a processed Mac page
async function rects(page, file) {
  await page.goto(pathToFileURL(file).href); await page.waitForTimeout(150);
  return page.evaluate(() => {
    const w = document.querySelector('.win').getBoundingClientRect();
    const r = (q) => { const e = document.querySelector(q); if (!e) return null; const b = e.getBoundingClientRect(); return [b.x - w.x, b.y - w.y, b.width, b.height].map(Math.round); };
    return { win: [Math.round(w.width), Math.round(w.height)], panel: r('[data-id=ai-panel]'), chip: r('[data-id=ai-mode-chip]'), close: r('[data-id=ai-close]') };
  });
}
// the most saturated opaque pixel of a region: the accent colour of the text there
async function accentOf(png, box, s) {
  const { data, info } = await sharp(png).extract({ left: Math.round(box[0] * s), top: Math.round(box[1] * s), width: Math.round(box[2] * s), height: Math.round(box[3] * s) }).raw().toBuffer({ resolveWithObject: true });
  let best = [0, 0, 0], bs = -1;
  for (let i = 0; i < data.length; i += info.channels) { const sc = Math.max(data[i], data[i + 1], data[i + 2]) - Math.min(data[i], data[i + 1], data[i + 2]); if (data[i + 3] > 200 && sc > bs) { bs = sc; best = [data[i], data[i + 1], data[i + 2]]; } }
  return `rgb(${best.join(',')})`;
}
async function pixel(png, x, y, s) {
  const { data } = await sharp(png).extract({ left: Math.round(x * s), top: Math.round(y * s), width: 1, height: 1 }).raw().toBuffer({ resolveWithObject: true });
  return data.slice(0, 3);
}
// flat colour over rects given in window CSS px
async function paint(png, s, list, color) {
  const ov = await Promise.all(list.map(async ([x, y, w, h]) => ({ input: await sharp({ create: { width: Math.round(w * s), height: Math.round(h * s), channels: 4, background: color } }).png().toBuffer(), left: Math.round(x * s), top: Math.round(y * s) })));
  return sharp(png).composite(ov).png({ compressionLevel: 9 }).toBuffer();
}
const dropUnavailable = (html) => html
  .replace(/<div class="el group _s\d+" data-id="ai-unavailable"[^>]*>.*?<\/div>/s, '')
  .replace(/(<span class="el statictext _s\d+" data-id="ai-mode-chip"[^>]*>)Off(<\/span>)/, '$1AI Ready$2');

// One simulated page from its base page, in one theme.
async function build(page, sim, theme) {
  const base = byName.get(sim.base);
  if (!base) throw new Error(`sim ${sim.name}: base screen ${sim.base} not in the catalog`);
  const stem = base.file.replace(/\.html$/, '');
  const dir = path.join(PAGES, theme), asDir = path.join(PAGES, 'assets', theme);
  const baseHtml = path.join(dir, base.file), basePng = path.join(asDir, path.basename(base.picture));
  const g = await rects(page, baseHtml);
  const tk = TOKENS[theme];
  const png0 = fs.readFileSync(basePng);
  const s = (await sharp(png0).metadata()).width / g.win[0];
  let png = png0, html = fs.readFileSync(baseHtml, 'utf8'), body, css;
  if (sim.kind === 'panel') {
    const [px, py] = g.panel, [W, H] = g.win;
    const bg = hex(await pixel(png, px + 40, py + 120, s)), hdr = hex(await pixel(png, px + 30, py + 70, s));
    const acc = await accentOf(png, g.close, s);
    const top = py + 86, left = px + 6, right = W - 12, bottom = H - 12; // 6 px margin: the window edge and its rounded corner stay as captured
    png = await paint(png, s, [[left, top, right - left, bottom - top]], rgba(bg));
    png = await paint(png, s, [[g.chip[0] - 34, g.chip[1] - 3, g.chip[2] + 38, g.chip[3] + 6]], rgba(hdr));
    const chip = `<span class="sim-chip" style="left:${g.chip[0] + g.chip[2] - 62}px;top:${g.chip[1] - 1}px">${sim.chip || 'AI Ready'}</span>`;
    body = `<div class="sim-panel" style="left:${left}px;top:${top}px;width:${right - left}px;height:${bottom - top}px">${sim.panel(tk)}</div>${chip}${sim.over ? sim.over(tk, { left, top, right, bottom }) : ''}`;
    css = panelCss(tk, bg, hdr, acc);
    html = dropUnavailable(html);
  } else if (sim.kind === 'free') {
    // a dialog over the dimmed window, or a notice painted into a flat area of the capture
    for (const [rect, [sx, sy]] of sim.paint || []) png = await paint(png, s, [rect], rgba(hex(await pixel(png, sx, sy, s))));
    body = sim.html();
    css = freeCss(tk, ACCENT[theme]);
  } else {
    const [W, H] = g.win;
    const bg = hex(await pixel(png, 6, 130, s));
    png = await paint(png, s, [[2, 90, W - 4, H - 94]], rgba(bg));
    body = `<div class="sim-set" style="left:2px;top:90px;width:${W - 4}px;height:${H - 94}px">${sim.panel(tk)}</div>${sim.over ? sim.over(tk, { W, H }) : ''}`;
    css = settingsCss(tk, bg, await accentOf(png0, [W / 2 - 70, 30, 140, 70], s));
  }
  const name = `sim-${sim.name}`, newPng = `${name}.png`;
  fs.writeFileSync(path.join(asDir, newPng), png);
  const re = new RegExp(path.basename(base.picture).replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'g');
  fs.writeFileSync(path.join(dir, `${name}.css`), fs.readFileSync(path.join(dir, `${stem}.css`), 'utf8').replace(re, newPng));
  html = html.replace(`href="${stem}.css"`, `href="${name}.css"`).replace(/<title>.*?<\/title>/, `<title>${sim.title.replace(/&/g, '&amp;')}</title>`);
  html = html.replace('</head>', `<style>${css}</style></head>`).replace(/<\/div><\/div><\/body>/, `${body}</div></div></body>`);
  fs.writeFileSync(path.join(dir, `${name}.html`), html);
}

const browser = await chromium.launch();
const page = await (await browser.newContext({ viewport: { width: 1500, height: 900 } })).newPage();
const records = [];
let n = 900;
for (const sim of [...SIMS, ...FREE_SIMS.map((f) => ({ ...f, kind: 'free' }))]) {
  for (const theme of ['light', 'dark']) await build(page, sim, theme);
  const base = byName.get(sim.base), mf = manifest.mac[sim.base];
  if (!mf) throw new Error(`no manifest entry for ${sim.base}`);
  manifest.mac[sim.name] = { ...mf, file: `sim-${sim.name}.html`, focus: 'panel', sidebar: 'kept', sim: true };
  records.push({ n: String(++n), name: sim.name, file: `sim-${sim.name}.html`, title: sim.title, area: sim.kind === 'free' ? base.area : SIM_AREA, picture: `assets/sim-${sim.name}.png`, width: base.width, height: base.height, texts: sim.texts, sim: true });
}
await browser.close();
fs.writeFileSync(MANIFEST + '.tmp', JSON.stringify(manifest, null, 1)); fs.renameSync(MANIFEST + '.tmp', MANIFEST);
fs.writeFileSync(CATFILE, JSON.stringify([...cat, ...records], null, 1) + '\n');
console.log(`sim-mac: ${records.length} simulated Mac screens written`);
