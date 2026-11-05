// Simulated Mac screens (owner decision 2026-11-01): the Mac capture of 2026-11-05 has no AI model installed, so the AI chat, the proposals and the
// Straft AI settings while it runs have no real Mac screen, and a few dialogs were never reached. Each simulated screen is a real Mac capture (window, sidebar,
// toolbar) with its AI panel, settings pane or dialog redrawn as live HTML in the Mac look, with the words of the Mac source.
// The captured screens stay as they are. Run after shots.mjs: node scripts/sim-mac.mjs
// Writes: .cache/shots/pages/mac/<theme>/sim-*.html, manifest entries, and the records appended to research/screens-mac.json (sim: true). Safe to run again.
import crypto from 'node:crypto'; import fs from 'node:fs'; import path from 'node:path'; import sharp from 'sharp';
import { chromium } from 'playwright'; import { pathToFileURL } from 'node:url';
import { SIMS, SIM_AREA, panelCss, settingsCss, TOKENS } from './sim-mac-screens.mjs';
import { FREE_SIMS, freeCss } from './sim-mac-free.mjs';
import { rebrand, rebrandDeep } from '../src/lib/brand.mjs'; // product name as on the website (the base pages are already renamed)

const ROOT = path.resolve(import.meta.dirname, '..');
const SHOTS = path.join(ROOT, '.cache', 'shots'), PAGES = path.join(SHOTS, 'pages', 'mac'), MANIFEST = path.join(SHOTS, 'manifest.json');
const CATFILE = path.join(ROOT, 'research', 'screens-mac.json');
const readJson = (f) => JSON.parse(fs.readFileSync(f, 'utf8'));
const manifest = readJson(MANIFEST);
const cat = readJson(CATFILE).filter((s) => !s.sim);
const byName = new Map(cat.map((s) => [s.name, s]));
const hex = (p) => '#' + [...p].map((v) => v.toString(16).padStart(2, '0')).join('');
const rgba = (h) => ({ r: parseInt(h.slice(1, 3), 16), g: parseInt(h.slice(3, 5), 16), b: parseInt(h.slice(5, 7), 16), alpha: 1 });

// positions, in window CSS px, of the parts of a Mac page; extra is { name: selector } for a sim's own measures
async function rects(page, file, extra = {}) {
  await page.goto(pathToFileURL(file).href);
  return page.evaluate((qs) => {
    const w = document.querySelector('.win').getBoundingClientRect();
    const r = (q) => { const e = document.querySelector(q); if (!e) return null; const b = e.getBoundingClientRect(); return [b.x - w.x, b.y - w.y, b.width, b.height].map(Math.round); };
    const body = '[data-id=ai-panel] .scrollarea', pane = '[data-id=settings-ai]';
    return { win: [Math.round(w.width), Math.round(w.height)], chip: r('[data-id=ai-mode-chip]'), body: r(body), bodyHtml: document.querySelector(body)?.outerHTML, pane: r(pane), paneHtml: document.querySelector(pane)?.outerHTML, m: Object.fromEntries(Object.entries(qs).map(([k, q]) => [k, r(q)])) };
  }, extra);
}
async function pixel(png, x, y, s) {
  const { data } = await sharp(png).extract({ left: Math.round(x * s), top: Math.round(y * s), width: 1, height: 1 }).raw().toBuffer({ resolveWithObject: true });
  return data.slice(0, 3);
}
// flat colour over rects in window CSS px; "atop" keeps the picture's alpha, so the window's rounded corners stay exactly as captured
async function paint(png, s, list, color) {
  const ov = await Promise.all(list.map(async ([x, y, w, h]) => ({ input: await sharp({ create: { width: Math.round(w * s), height: Math.round(h * s), channels: 4, background: color } }).png().toBuffer(), left: Math.round(x * s), top: Math.round(y * s), blend: 'atop' })));
  return sharp(png).composite(ov).png({ compressionLevel: 9 }).toBuffer();
}
// moves a part of the picture down (a banner pushes the page below it)
async function shiftDown(png, s, [x, y, w, h], dy) {
  const part = await sharp(png).extract({ left: Math.round(x * s), top: Math.round(y * s), width: Math.round(w * s), height: Math.round(h * s) }).png().toBuffer();
  return sharp(png).composite([{ input: part, left: Math.round(x * s), top: Math.round((y + dy) * s) }]).png({ compressionLevel: 9 }).toBuffer();
}
// a real sheet: the window dimmed, with the traffic lights as a real capture draws them under a sheet
async function dimWindow(png, s, theme, tk, win) {
  const lights = path.join(PAGES, 'assets', theme, path.basename(byName.get('breakdown-export-sheet').picture));
  const box = { left: Math.round(12 * s), top: Math.round(14 * s), width: Math.round(72 * s), height: Math.round(24 * s) };
  const dimmed = await paint(png, s, [[0, 0, win[0], win[1]]], tk.dimColor);
  return sharp(dimmed).composite([{ input: await sharp(lights).extract(box).png().toBuffer(), left: box.left, top: box.top }]).png({ compressionLevel: 9 }).toBuffer();
}
const without = (html, part, name) => {
  if (!part || !html.includes(part)) throw new Error(`sim ${name}: the part to replace was not found in its base page`);
  return html.replace(part, '');
};

// One simulated page from its base page, in one theme.
async function build(page, sim, theme) {
  const base = byName.get(sim.base);
  if (!base) throw new Error(`sim ${sim.name}: base screen ${sim.base} not in the catalog`);
  const stem = base.file.replace(/\.html$/, '');
  const dir = path.join(PAGES, theme), asDir = path.join(PAGES, 'assets', theme);
  const baseHtml = path.join(dir, base.file), basePng = path.join(asDir, path.basename(base.picture));
  const g = await rects(page, baseHtml, sim.measure);
  const tk = TOKENS[theme];
  const png0 = fs.readFileSync(basePng);
  const s = (await sharp(png0).metadata()).width / g.win[0];
  const [W, H] = g.win;
  let png = png0, html = fs.readFileSync(baseHtml, 'utf8'), body, css;
  if (sim.kind === 'panel') {
    const [x, y, w, h] = g.body, c = g.chip;
    png = await paint(png, s, [[x, y, w, h]], rgba(hex(await pixel(png, x + 40, H - 60, s))));
    png = await paint(png, s, [[c[0] - 4, c[1] - 2, c[2] + 8, c[3] + 4]], rgba(hex(await pixel(png, c[0] - 12, c[1] + 8, s))));
    const chip = `<span class="sim-root sim-chip m-chip green" style="right:${W - c[0] - c[2]}px;top:${c[1]}px">${sim.chip || 'AI Ready'}</span>`;
    body = `<div class="sim-root sim-panel" style="left:${x}px;top:${y}px;width:${w}px;height:${h}px">${sim.panel()}</div>${chip}${sim.over ? sim.over(g) : ''}`;
    css = panelCss(tk);
    html = without(html, g.bodyHtml, sim.name).replace(/(data-id="ai-mode-chip"[^>]*>)Off(<\/span>)/, '$1AI Ready$2');
  } else if (sim.kind === 'free') {
    if (sim.shift) { const { rect, dy } = sim.shift(g); png = await shiftDown(png, s, rect, dy); }
    for (const [rect, [sx, sy]] of sim.paint ? sim.paint(g) : []) png = await paint(png, s, [rect], rgba(hex(await pixel(png, sx, sy, s))));
    if (sim.sheet) png = await dimWindow(png, s, theme, tk, g.win);
    body = sim.html(g);
    css = freeCss(tk);
  } else {
    const [x, y, w, h] = g.pane;
    png = await paint(png, s, [[x, y, w, h]], rgba(hex(await pixel(png, 8, H - 40, s))));
    body = `<div class="sim-root sim-set" style="left:${x}px;top:${y}px;width:${w}px;height:${h}px">${sim.panel()}</div>${sim.over ? sim.over(g) : ''}`;
    css = settingsCss(tk);
    html = without(html, g.paneHtml, sim.name);
  }
  const name = `sim-${sim.name}`, newPng = `${name}.png`;
  fs.writeFileSync(path.join(asDir, newPng), png);
  const re = new RegExp(path.basename(base.picture).replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'g');
  const version = crypto.createHash('sha1').update(png).digest('hex').slice(0, 10); // as in shots.mjs: a changed picture gets a new address
  fs.writeFileSync(path.join(dir, `${name}.css`), fs.readFileSync(path.join(dir, `${stem}.css`), 'utf8').replace(re, newPng).replace(/\?v=[0-9a-f]+/g, `?v=${version}`));
  html = html.replace(`href="${stem}.css"`, `href="${name}.css"`).replace(/<title>.*?<\/title>/, `<title>${sim.title.replace(/&/g, '&amp;')}</title>`);
  html = html.replace('</head>', `<style>${css}</style></head>`).replace(/<\/div><\/div><\/body>/, `${body}</div></div></body>`);
  fs.writeFileSync(path.join(dir, `${name}.html`), rebrand(html));
}

// re-runnable: a sim that was removed leaves nothing behind
for (const theme of ['light', 'dark']) for (const dir of [path.join(PAGES, theme), path.join(PAGES, 'assets', theme)]) for (const f of fs.readdirSync(dir)) if (f.startsWith('sim-')) fs.rmSync(path.join(dir, f));
for (const [k, v] of Object.entries(manifest.mac)) if (v.sim) delete manifest.mac[k];

const browser = await chromium.launch();
const page = await (await browser.newContext({ viewport: { width: 1800, height: 1000 } })).newPage();
const records = [];
let n = 900;
for (const sim of [...SIMS, ...FREE_SIMS.map((f) => ({ ...f, kind: 'free' }))]) {
  for (const theme of ['light', 'dark']) await build(page, sim, theme);
  const base = byName.get(sim.base), mf = manifest.mac[sim.base];
  if (!mf) throw new Error(`no manifest entry for ${sim.base}`);
  manifest.mac[sim.name] = { ...mf, file: `sim-${sim.name}.html`, focus: 'panel', sidebar: 'kept', sim: true };
  records.push({ n: String(++n), name: sim.name, file: `sim-${sim.name}.html`, title: rebrand(sim.title), area: sim.kind === 'free' ? base.area : SIM_AREA, picture: `assets/sim-${sim.name}.png`, width: base.width, height: base.height, texts: rebrandDeep(sim.texts), sim: true });
}
await browser.close();
fs.writeFileSync(MANIFEST + '.tmp', JSON.stringify(manifest, null, 1)); fs.renameSync(MANIFEST + '.tmp', MANIFEST);
fs.writeFileSync(CATFILE, JSON.stringify([...cat, ...records], null, 1) + '\n');
console.log(`sim-mac: ${records.length} simulated Mac screens written`);
