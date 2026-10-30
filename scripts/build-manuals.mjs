// The two user manuals as PDF, generated from src/content/manual/<win|mac>/*.mjs and the real screens:
//   manuals/openframe-studio-windows-user-manual.pdf, manuals/openframe-studio-macos-user-manual.pdf, manuals/manuals.json
// Pictures: Windows from .cache/win-full (scripts/render-screens.mjs), Mac from the MacHTML window pictures.
// A figure is the whole window, or, when only part of the window changed since the previous figure of the same
// workflow (a dialog, a menu), that part at full resolution. The manuals are internal reference for writing the
// tutorials; they are never published on the website (owner decision 2026-10-31).
// Run: node scripts/build-manuals.mjs [win|mac]
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium } from 'playwright';
import sharp from 'sharp';
import { loadManual, checkContent } from './check-content.mjs';
import { PARTS, COMPANY } from '../src/content/nav.mjs';
import { SHORTCUTS } from '../src/content/copy.mjs';
import { esc } from '../src/lib/html.mjs';

const ROOT = path.resolve(import.meta.dirname, '..');
const OUT = path.join(ROOT, 'manuals');
const CACHE = path.join(ROOT, '.cache', 'manual-img');
const LIMIT = 25 * 1024 * 1024;
const only = process.argv[2];
const EDITION = 'Edition of 31 October 2026';
const META = {
  win: { name: 'Windows', title: 'OpenFrame Studio: Windows User Manual', file: 'openframe-studio-windows-user-manual.pdf', sys: 'Windows 11 (64-bit)', col: 1 },
  mac: { name: 'Mac', title: 'OpenFrame Studio: macOS User Manual', file: 'openframe-studio-macos-user-manual.pdf', sys: 'macOS 14 or later on Apple silicon', col: 2 },
};

const { errors } = await checkContent(only ? [only] : ['win', 'mac']);
if (errors.length) { console.error(errors.join('\n')); throw new Error(`${errors.length} content problems; fix them before building the manuals`); }

// ---------- pictures
const source = (plat, s) => plat === 'win'
  ? path.join(ROOT, '.cache', 'win-full', s.file.replace('.html', '.png'))
  : path.join(ROOT, 'research', 'MacHTML-src', 'light', s.picture);
const small = (file) => sharp(file).resize(200, null, { fit: 'inside' }).removeAlpha().raw().toBuffer({ resolveWithObject: true });
// Bounding box of the pixels that differ between two pictures of the same size, in fractions of the picture.
async function changed(a, b) {
  const A = await small(a), B = await small(b);
  if (A.info.width !== B.info.width || A.info.height !== B.info.height) return null;
  const { width: w, height: h, channels: c } = A.info;
  let x0 = w, y0 = h, x1 = -1, y1 = -1;
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
    const i = (y * w + x) * c;
    if (Math.abs(A.data[i] - B.data[i]) + Math.abs(A.data[i + 1] - B.data[i + 1]) + Math.abs(A.data[i + 2] - B.data[i + 2]) > 60) {
      if (x < x0) x0 = x; if (y < y0) y0 = y; if (x > x1) x1 = x; if (y > y1) y1 = y;
    }
  }
  if (x1 < 0) return { none: true };
  return { x: x0 / w, y: y0 / h, w: (x1 - x0 + 1) / w, h: (y1 - y0 + 1) / h };
}
const made = new Map();
async function picture(plat, s, prev) {
  const src = source(plat, s);
  if (!fs.existsSync(src)) throw new Error(`no picture for ${plat}/${s.name}: ${src}`);
  let box = null;
  if (prev) {
    const d = await changed(source(plat, prev), src);
    if (d && !d.none && d.w * d.h < 0.4 && d.w > 0.12 && d.h > 0.1) box = d;
  }
  const key = `q46-${plat}-${s.n}${box ? `-c${Math.round(box.x * 100)}-${Math.round(box.y * 100)}-${Math.round(box.w * 100)}-${Math.round(box.h * 100)}` : ''}`;
  if (made.has(key)) return made.get(key);
  const out = path.join(CACHE, `${key}.jpg`);
  if (!fs.existsSync(out)) {
    const meta = await sharp(src).metadata();
    let img = sharp(src).flatten({ background: '#ffffff' });
    if (box) {
      const pad = 0.025;
      const left = Math.max(0, Math.floor((box.x - pad) * meta.width)), top = Math.max(0, Math.floor((box.y - pad) * meta.height));
      const width = Math.min(meta.width - left, Math.ceil((box.w + 2 * pad) * meta.width)), height = Math.min(meta.height - top, Math.ceil((box.h + 2 * pad) * meta.height));
      img = img.extract({ left, top, width, height }).resize(Math.min(960, plat === 'mac' ? Math.round(width * 0.65) : width), null, { withoutEnlargement: true });
    } else img = img.resize(960);
    await img.jpeg({ quality: 46, mozjpeg: true }).toFile(out);
  }
  const r = { file: pathToFileURL(out).href, crop: !!box };
  made.set(key, r);
  return r;
}

// ---------- document
const md = (s) => esc(s).replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>').replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, '$1');

function controls(plat, s) {
  if (plat === 'mac') return s.texts?.length ? `<p class="ctl"><b>Text and controls on this screen:</b> ${s.texts.slice(0, 40).map(esc).join(' · ')}</p>` : '';
  const parts = [];
  if (s.layerTitle) parts.push(`<b>${s.layer === 'menu' ? 'Menu' : 'Dialog'}:</b> ${esc(s.layerTitle)}`);
  if (s.tabs?.length) parts.push(`<b>Tabs:</b> ${s.tabs.map((t) => esc(t.label) + (t.on ? ' (selected)' : '')).join(', ')}`);
  if (s.items?.length) parts.push(`<b>Items:</b> ${s.items.slice(0, 30).map(esc).join(' · ')}`);
  if (s.fields?.length) parts.push(`<b>Fields:</b> ${s.fields.slice(0, 20).map((f) => esc(f.label || 'Field') + (f.value ? `: “${esc(f.value.slice(0, 60))}”` : '')).join('; ')}`);
  if (s.toggles?.length) parts.push(`<b>Options:</b> ${s.toggles.slice(0, 20).map((t) => esc(t.label) + (t.on ? ' (on)' : ' (off)')).join(', ')}`);
  if (s.buttons?.length) parts.push(`<b>Buttons:</b> ${s.buttons.slice(0, 40).map(esc).join(' · ')}`);
  if (s.alerts?.length) parts.push(`<b>Messages:</b> ${s.alerts.slice(0, 6).map(esc).join(' · ')}`);
  return parts.length ? `<p class="ctl">${parts.join('<br>')}</p>` : '';
}

async function chapterHtml(plat, a, n, cat, figs) {
  const byName = new Map(cat.map((s) => [s.name, s]));
  let h = `<section class="chapter" id="ch-${a.slug}"><p class="kicker">Chapter ${n}</p><h1>${esc(a.area)}</h1>`;
  h += (a.intro || []).map((p) => `<p>${md(p)}</p>`).join('');
  if (a.layout?.length) h += `<h2>The screen at a glance</h2><dl class="layout">${a.layout.map((l) => `<dt>${esc(l.name)}</dt><dd>${md(l.text)}</dd>`).join('')}</dl>`;
  let wn = 0;
  for (const w of a.workflows) {
    wn++;
    h += `<section class="wf" id="wf-${w.id}"><h2>${n}.${wn} ${esc(w.title)}</h2><p class="purpose">${md(w.purpose)}${w.level ? ` <span class="lvl">${esc(w.level)}</span>` : ''}</p>`;
    if (w.prerequisites?.length) h += `<div class="box"><b>Before you start</b><ul>${w.prerequisites.map((p) => `<li>${md(p)}</li>`).join('')}</ul></div>`;
    h += '<ol class="steps">';
    let prev = null;
    for (const st of w.steps) {
      const s = byName.get(st.screen);
      const pic = await picture(plat, s, prev);
      figs.n++;
      if (!figs.where.has(s.name)) figs.where.set(s.name, `${figs.n}`);
      h += `<li><p class="do">${md(st.do)}</p>${st.explain ? `<p>${md(st.explain)}</p>` : ''}<figure class="${pic.crop ? 'crop' : ''}"><img src="${pic.file}" alt=""><figcaption>Figure ${figs.n}. ${esc(s.title)}${pic.crop ? ' (the part of the window that changed)' : ''}</figcaption></figure></li>`;
      prev = s;
    }
    h += '</ol>';
    h += `<div class="box ok"><b>What you get</b><p>${md(w.result)}</p></div>`;
    if (w.cautions?.length) h += `<div class="box warn"><b>Watch out for</b><ul>${w.cautions.map((c) => `<li>${md(c)}</li>`).join('')}</ul></div>`;
    if (w.related?.length) h += `<p class="rel">Related: ${w.related.map((r) => `<a href="#wf-${r}">${esc(figs.titles.get(r) || r)}</a>`).join(', ')}</p>`;
    h += '</section>';
  }
  if (a.shortcuts?.length) h += `<h2>Keyboard shortcuts in ${esc(a.area)}</h2><table>${a.shortcuts.map(([k, d]) => `<tr><td class="k">${esc(k)}</td><td>${md(d)}</td></tr>`).join('')}</table>`;
  if (a.limits?.length) h += `<h2>Limits</h2><ul>${a.limits.map((l) => `<li>${md(l)}</li>`).join('')}</ul>`;
  if (a.troubleshooting?.length) h += `<h2>Troubleshooting</h2><dl class="trouble">${a.troubleshooting.map((t) => `<dt>${md(t.problem)}</dt><dd>${md(t.fix)}</dd>`).join('')}</dl>`;
  // Every screen of the area, with its description and the controls read from the screen.
  const own = cat.filter((x) => x.area === a.area);
  h += `<h2>Every screen in ${esc(a.area)}</h2><p class="note">All ${own.length} screens and states of this area, in the order they appear in the app.</p>`;
  for (const s of own) {
    const at = figs.where.get(s.name);
    let fig = '';
    if (!at) { const pic = await picture(plat, s, null); figs.n++; figs.where.set(s.name, `${figs.n}`); fig = `<figure class="ref"><img src="${pic.file}" alt=""><figcaption>Figure ${figs.n}. ${esc(s.title)}</figcaption></figure>`; }
    h += `<div class="scr" id="scr-${s.name}"><h3>${esc(s.title)}</h3><p>${md(a.screens[s.name] || '')}${at ? ` Pictured in Figure ${at}.` : ''}</p>${controls(plat, s)}${fig}</div>`;
  }
  return h + '</section>';
}

const CSS = `
@font-face{font-family:Inter;src:url(${pathToFileURL(path.join(ROOT, 'src/assets/fonts/inter-latin-wght-normal.woff2')).href}) format("woff2");font-weight:100 900}
*{box-sizing:border-box}
body{font-family:"Segoe UI",Inter,system-ui,sans-serif;font-size:10.2pt;line-height:1.5;color:#1d1a2e;margin:0}
h1,h2,h3{line-height:1.2;color:#1d1a2e;break-after:avoid}
h1{font-size:24pt;margin:0 0 10pt;letter-spacing:-.02em}
h2{font-size:14.5pt;margin:20pt 0 6pt}
h3{font-size:11.5pt;margin:12pt 0 3pt}
p{margin:0 0 6pt}
a{color:#b53713;text-decoration:none}
.cover{height:255mm;display:flex;flex-direction:column;justify-content:space-between;break-after:page}
.cover .brand{font-size:30pt;letter-spacing:-.02em}.cover .brand i{font-style:normal;font-weight:300}.cover .brand b{font-weight:700}
.cover .studio{color:#d2441c;letter-spacing:.6em;font-size:9pt;font-weight:700}
.cover h1{font-size:34pt;margin-top:40mm}
.cover .sub{font-size:13pt;color:#4b4660}
.cover .co{display:flex;align-items:center;gap:14pt}.cover .co img{width:56mm}
.cover .co p{font-size:10pt;color:#4b4660}
.front{break-after:page}
.toc ol{list-style:none;padding:0;margin:0}
.toc .part{font-weight:800;text-transform:uppercase;letter-spacing:.08em;font-size:9pt;color:#d2441c;margin:12pt 0 4pt}
.toc .ch{display:flex;justify-content:space-between;border-bottom:.5pt dotted #c9c2b5;padding:2.5pt 0;font-weight:600;color:#1d1a2e}
.toc .wfs{font-size:8.6pt;color:#4b4660;margin:2pt 0 4pt 12pt}
.partpage{break-before:page;padding-top:70mm}
.partpage h1{font-size:30pt}
.chapter{break-before:page}
.kicker{color:#d2441c;font-weight:800;text-transform:uppercase;letter-spacing:.12em;font-size:9pt;margin:0 0 4pt}
.wf{margin-top:8pt}
.purpose{color:#4b4660}
.lvl{display:inline-block;font-size:7.5pt;font-weight:700;border:.6pt solid #d9d2c5;border-radius:8pt;padding:0 5pt;margin-left:4pt;color:#6f6a82}
ol.steps{padding-left:16pt;margin:6pt 0}
ol.steps>li{margin:0 0 9pt}
ol.steps .do{font-weight:650}
figure{margin:5pt 0 4pt;break-inside:avoid}
figure img{display:block;width:100%;border:.6pt solid #d9d2c5;border-radius:3pt}
figure.crop img{width:auto;max-width:100%;max-height:120mm}
figure.ref img{max-height:95mm;width:auto;max-width:100%}
figcaption{font-size:8pt;color:#6f6a82;margin-top:2pt}
.box{border:.6pt solid #e8e2d8;background:#fffaf3;border-radius:4pt;padding:6pt 9pt;margin:6pt 0;break-inside:avoid}
.box.ok{background:#f1f8f4;border-color:#cfe5d8}.box.warn{background:#fff6e5;border-color:#efd7a3}
.box ul{margin:3pt 0 0;padding-left:14pt}
.rel{font-size:9pt;color:#4b4660}
dl.layout dt,dl.trouble dt{font-weight:700;margin-top:5pt}
dl.layout dd,dl.trouble dd{margin:1pt 0 0 0}
table{border-collapse:collapse;width:100%;font-size:9.2pt;margin:4pt 0}
td,th{border-bottom:.5pt solid #e8e2d8;padding:3pt 5pt;text-align:left;vertical-align:top}
td.k{white-space:nowrap;font-weight:700;width:34%}
.scr{border-top:.5pt solid #e8e2d8;padding-top:4pt;margin-top:6pt}
.scr h3{margin-top:2pt}
.ctl{font-size:8.3pt;color:#4b4660}
.note{color:#6f6a82;font-size:9pt}
.index{columns:2;column-gap:10mm;font-size:8.4pt}
.index p{margin:0 0 1.5pt;break-inside:avoid}
`;

async function build(plat, browser) {
  const M = META[plat];
  const cat = JSON.parse(fs.readFileSync(path.join(ROOT, 'research', `screens-${plat}.json`), 'utf8'));
  const areas = await loadManual(plat);
  const covered = new Set(areas.map((a) => a.area));
  const uncovered = [...new Set(cat.filter((s) => !covered.has(s.area)).map((s) => s.area))];
  if (uncovered.length) throw new Error(`${plat}: areas without a manual chapter: ${uncovered.join(', ')}`);
  const figs = { n: 0, where: new Map(), titles: new Map(areas.flatMap((a) => a.workflows.map((w) => [w.id, w.title]))) };
  const logo = pathToFileURL(path.join(ROOT, 'src', 'static', 'company', 'vighnajith-advisories-logo.webp')).href;
  const nWf = areas.reduce((k, a) => k + a.workflows.length, 0);

  const chapters = [];
  let n = 0;
  for (const [part] of PARTS) for (const a of areas.filter((x) => x.part === part)) { n++; chapters.push({ a, n, part, html: await chapterHtml(plat, a, n, cat, figs) }); }
  const head = `<!doctype html><html lang="en"><head><meta charset="utf-8"><title>${esc(M.title)}</title><style>${CSS}</style></head><body>`;
  const cover = `<section class="cover"><div><div class="brand"><i>Open</i><b>Frame</b></div><div class="studio">STUDIO</div>
<h1>${plat === 'mac' ? 'macOS' : 'Windows'} User Manual</h1><p class="sub">Every workspace, screen, control and workflow of OpenFrame Studio for ${esc(M.name)}, explained step by step.</p>
<p class="sub">${esc(M.sys)} · ${EDITION}</p></div>
<div class="co"><img src="${logo}" alt="${esc(COMPANY)} logo"><p>OpenFrame Studio is a product of ${esc(COMPANY)}.<br>© 2026 ${esc(COMPANY)}.</p></div></section>`;
  const keys = SHORTCUTS.filter((r) => r[M.col]);
  const front = `<section class="front"><h1>About this manual</h1>
<p>This manual explains how to use OpenFrame Studio for ${esc(M.name)}. It follows the app from the first start to backups and settings. Each chapter covers one area of the app: what it is for, how its screen is laid out, every task you can do there as numbered steps, and then every screen and state of the area with the text and controls it shows.</p>
<p>The pictures are real screens of the ${esc(M.name)} app, shown in the Light appearance of the default theme. The example project in the pictures is called Night Bus; your own projects look the same with your own content. Where only part of the window changes from one step to the next, the figure shows that part.</p>
<p>Names of buttons, menus and fields are written exactly as the app shows them. ${plat === 'win' ? 'Keys are written as they appear on a Windows keyboard, for example Ctrl+K.' : 'Keys are written as Mac shortcuts, for example Cmd+K for Command and K (shown in menus as ⌘K). Opt is the Option key.'}</p>
<h2>What you need</h2><p>${esc(M.sys)}. No OpenFrame account and no internet connection are needed to work. A 7-day free trial starts with the app; after the trial, buy OpenFrame Studio in the ${plat === 'win' ? 'Microsoft Store' : 'Mac App Store'} to keep using its full functionality.</p>
<h2>Contents</h2><div class="toc"><ol>${PARTS.map(([part, t]) => {
    const cs = chapters.filter((c) => c.part === part);
    if (!cs.length) return '';
    return `<li class="part">${esc(t)}</li>${cs.map((c) => `<li><a class="ch" href="#ch-${c.a.slug}"><span>${c.n}. ${esc(c.a.area)}</span><span>{{p:${c.a.slug}}}</span></a><div class="wfs">${c.a.workflows.map((w, i) => `${c.n}.${i + 1} ${esc(w.title)}`).join(' · ')}</div></li>`).join('')}`;
  }).join('')}
<li class="part">Appendices</li><li><a class="ch" href="#app-keys"><span>A. Keyboard shortcuts</span><span>{{p:app-keys}}</span></a></li><li><a class="ch" href="#app-index"><span>B. Index of screens</span><span>{{p:app-index}}</span></a></li></ol></div></section>`;
  const keysHtml = `<section class="chapter" id="app-keys"><p class="kicker">Appendix A</p><h1>Keyboard shortcuts</h1><p>The default shortcuts of OpenFrame Studio for ${esc(M.name)}. You can change them in Settings, Keyboard Shortcuts; the app warns you about conflicts.</p>
<table><tr><th>Action</th><th>Shortcut</th></tr>${keys.map((r) => `<tr><td>${esc(r[0])}</td><td class="k">${esc(r[M.col])}</td></tr>`).join('')}</table>
${chapters.filter((c) => c.a.shortcuts?.length).map((c) => `<h3>${esc(c.a.area)}</h3><table>${c.a.shortcuts.map(([k, d]) => `<tr><td>${md(d)}</td><td class="k">${esc(k)}</td></tr>`).join('')}</table>`).join('')}</section>`;
  const indexHtml = `<section class="chapter" id="app-index"><p class="kicker">Appendix B</p><h1>Index of screens</h1><p>All ${cat.length} screens and states of the ${esc(M.name)} app covered by this manual, by chapter.</p><div class="index">
${chapters.map((c) => `<p><b>${c.n}. ${esc(c.a.area)}</b></p>${cat.filter((s) => s.area === c.a.area).map((s) => `<p>${esc(s.title)}, Figure ${figs.where.get(s.name)}</p>`).join('')}`).join('')}</div></section>`;
  const partPage = (part) => { const p = PARTS.find(([k]) => k === part); return `<section class="partpage"><p class="kicker">Part</p><h1>${esc(p[1])}</h1><p>${esc(p[2])}</p></section>`; };

  const page = await browser.newPage();
  const pdfOpts = { format: 'A4', printBackground: true, displayHeaderFooter: true, outline: true, tagged: true,
    headerTemplate: `<div style="font:7.5pt 'Segoe UI',sans-serif;color:#8a8499;width:100%;padding:0 17mm;display:flex;justify-content:space-between"><span>${esc(M.title)}</span><span>${esc(COMPANY)}</span></div>`,
    footerTemplate: `<div style="font:7.5pt 'Segoe UI',sans-serif;color:#8a8499;width:100%;text-align:center">Page <span class="pageNumber"></span> of <span class="totalPages"></span></div>`,
    margin: { top: '20mm', bottom: '20mm', left: '17mm', right: '17mm' } };
  const pages = (buf) => (buf.toString('latin1').match(/\/Type\s*\/Page(?![s\w])/g) || []).length;
  const tmp = path.join(ROOT, '.cache', `manual-${plat}.html`);
  const render = async (doc) => { fs.writeFileSync(tmp, doc); await page.goto(pathToFileURL(tmp).href, { waitUntil: 'load' }); await page.evaluate(() => document.fonts.ready); return page.pdf(pdfOpts); };

  // Pass 1: every chapter, part page and appendix starts on a new page, so their page counts give the contents numbers.
  const startOf = {};
  let at = pages(await render(head + cover + front.replace(/\{\{p:[^}]+\}\}/g, '000') + '</body></html>')) + 1;
  let lastPart = null, body = '';
  for (const c of chapters) {
    if (c.part !== lastPart) { at += 1; lastPart = c.part; body += partPage(c.part); }
    startOf[c.a.slug] = at;
    at += pages(await render(head + c.html + '</body></html>'));
    body += c.html;
  }
  startOf['app-keys'] = at; at += pages(await render(head + keysHtml + '</body></html>'));
  startOf['app-index'] = at;
  // Pass 2: the whole manual.
  const pdf = await render(head + cover + front.replace(/\{\{p:([^}]+)\}\}/g, (m, k) => String(startOf[k])) + body + keysHtml + indexHtml + '</body></html>');
  await page.close();
  const total = pages(pdf);
  if (pdf.subarray(0, 5).toString() !== '%PDF-') throw new Error(`${M.file}: not a PDF`);
  if (pdf.length > LIMIT) console.warn(`${M.file} is ${(pdf.length / 1048576).toFixed(1)} MiB`);
  fs.writeFileSync(path.join(OUT, M.file), pdf);
  console.log(`${M.file}: ${total} pages, ${(pdf.length / 1048576).toFixed(1)} MiB, ${chapters.length} chapters, ${nWf} workflows, ${figs.n} figures, ${cat.length} screens`);
  return { file: M.file, pages: total, mb: (pdf.length / 1048576).toFixed(1), areas: chapters.length, workflows: nWf, screens: cat.length, figures: figs.n };
}

fs.mkdirSync(OUT, { recursive: true });
fs.mkdirSync(CACHE, { recursive: true });
// One metadata file per platform, so the two manuals can be built at the same time.
const browser = await chromium.launch();
for (const plat of only ? [only] : ['win', 'mac']) fs.writeFileSync(path.join(OUT, `manual-${plat}.json`), JSON.stringify(await build(plat, browser), null, 1));
await browser.close();
