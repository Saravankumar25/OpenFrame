#!/usr/bin/env node
// Builds the MacHTML folder from a capture folder written by the capture tour
// (apps/macos/Sources/OpenFrameE2EDriver/Scenarios/CaptureTour.swift).
//
// Usage: node scripts/mac-html/build-mac-html.mjs <capture-run-dir> <MacHTML-dir> [commit]
// The run dir is the folder the runner wrote: one folder per zz-capture scenario, each with a capture/ folder
// holding <name>.png and <name>.json. No dependencies.

import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';

import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const argv = process.argv.slice(2);
if (argv[0] === '--both') {
  // Orchestration: build the dark and the light set with identical page numbers, then the root index.
  const [, lightRun, darkRun, root, commit = ''] = argv;
  const self = fileURLToPath(import.meta.url);
  const run = (args) => {
    const r = spawnSync('node', [self, ...args], { encoding: 'utf8' });
    if (r.status !== 0) { console.error(r.stdout + r.stderr); process.exit(1); }
    return r.stdout;
  };
  fs.rmSync(root, { recursive: true, force: true });
  fs.mkdirSync(root, { recursive: true });
  const names = path.join(root, '.names.json');
  run([darkRun, path.join(root, 'dark'), commit, '--mode', 'dark', '--emit-names', names]);
  run([lightRun, path.join(root, 'light'), commit, '--mode', 'light', '--only', names]);
  const readNames = (m) => new Set(JSON.parse(fs.readFileSync(path.join(root, m, 'pages.json'), 'utf8')).map((p) => p.name));
  const dark = readNames('dark'), light = readNames('light');
  if (dark.size !== light.size) {
    const keep = JSON.parse(fs.readFileSync(names, 'utf8'));
    keep.names = keep.names.filter((n) => dark.has(n) && light.has(n));
    fs.writeFileSync(names, JSON.stringify(keep));
    run([darkRun, path.join(root, 'dark'), commit, '--mode', 'dark', '--only', names]);
    run([lightRun, path.join(root, 'light'), commit, '--mode', 'light', '--only', names]);
  }
  fs.rmSync(names);
  const pages = JSON.parse(fs.readFileSync(path.join(root, 'dark', 'pages.json'), 'utf8'));
  const groups = [];
  for (const p of pages) { let g = groups.find((x) => x.area === p.area); if (!g) groups.push((g = { area: p.area, pages: [] })); g.pages.push(p); }
  const e = (s) => String(s ?? '').replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
  const skipped = [];
  for (const run of [lightRun, darkRun]) (function collect(dir) {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const p = path.join(dir, entry.name);
      if (entry.isDirectory()) collect(p);
      else if (entry.name === 'skipped.tsv') for (const line of fs.readFileSync(p, 'utf8').split('\n')) if (line.trim() && !skipped.includes(line.replace('\t', ': '))) skipped.push(line.replace('\t', ': '));
    }
  })(run);
  const css = fs.readFileSync(path.join(root, 'dark', 'index.html'), 'utf8').match(/<style>([\s\S]*?)<\/style>/)[1];
  const index = `<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>OpenFrame Studio for Mac, every screen</title><style>${css}
.sw{display:inline-flex;border:1px solid var(--line);border-radius:8px;overflow:hidden;margin-left:12px}
.sw button{border:0;padding:5px 16px;font:inherit;font-size:13px;background:var(--bg);color:var(--fg);cursor:pointer}
.sw button.on{background:var(--acc);color:#fff}
.set{display:none}.set.on{display:block}</style></head>
<body><header class="top"><h1>OpenFrame Studio for Mac: every screen</h1>
<span class="meta">${pages.length} screens in ${groups.length} areas, each in the default theme, light and dark${commit ? ' &middot; commit ' + e(commit) : ''}</span>
<span class="sw" role="group" aria-label="Appearance"><button id="b-light" type="button">Light</button><button id="b-dark" type="button" class="on">Dark</button></span></header>
<div class="index">
<p class="meta" style="color:var(--mut)">Each page is a real picture of the running app with its real controls laid on top. Controls that lead to another captured screen are links. Pick Light or Dark above; every page also has the same switch.</p>
${['dark', 'light'].map((m) => `<div class="set ${m === 'dark' ? 'on' : ''}" id="set-${m}">
<nav>${groups.map((g) => `<a href="#${m}-${e(g.area.replace(/\W+/g, '-'))}">${e(g.area)} (${g.pages.length})</a>`).join(' &middot; ')}</nav>
${groups.map((g) => `<h2 id="${m}-${e(g.area.replace(/\W+/g, '-'))}">${e(g.area)} <small>(${g.pages.length})</small></h2>
<div class="grid">${g.pages.map((p) => `<a class="card" href="${m}/${e(p.file)}"><img loading="lazy" src="${m}/assets/${e(p.name)}.png" alt=""><div>${p.n} ${e(p.title)}<small>${e(p.name)}</small></div></a>`).join('')}</div>`).join('\n')}
</div>`).join('\n')}
</div>
<script>(function(){function set(m){['light','dark'].forEach(function(x){document.getElementById('set-'+x).classList.toggle('on',x===m);document.getElementById('b-'+x).classList.toggle('on',x===m)});try{localStorage.setItem('machtml-mode',m)}catch(e){}}
document.getElementById('b-light').onclick=function(){set('light')};document.getElementById('b-dark').onclick=function(){set('dark')};
var m=location.hash==='#light'?'light':null;try{m=m||localStorage.getItem('machtml-mode')}catch(e){}if(m)set(m);})();</script>
</body></html>`;
  fs.writeFileSync(path.join(root, 'index.html'), index);
  const readme = `# MacHTML: every screen of OpenFrame Studio for the Mac

${pages.length} screens in ${groups.length} areas, each captured twice from the real app: the default theme in Light mode (\`light/\`) and in Dark mode (\`dark/\`). Open \`index.html\` and use the Light and Dark switch; every page has the same switch. Page \`NNN-name.html\` is the same screen in both folders.

## How these were made

- The capture tour (\`apps/macos/Sources/OpenFrameE2EDriver/Scenarios/CaptureTour.swift\`, scenarios \`zz-capture/*\`) drives the real app through the accessibility tree, seeds a project called Night Bus (the bundled example screenplay, story acts and cards, characters, Idea Vault items, a breakdown, a schedule with days, a call sheet, budget lines, notes and tasks) and visits every screen it can reach. It is run twice. Each run first sets Settings, Appearance, Mode to Light or Dark (default theme), so the app draws itself in that mode.
- For every screen it writes a 2x picture of the whole window (title bar, toolbar, sidebar and content, as the window server draws it, with any sheet or popover at its real position) and a list of every accessibility element with its role, identifier, label, value, enabled state and frame in points.
- Menus are drawn from the real menu items, because an open menu is not a window the app can draw.
- \`scripts/mac-html/build-mac-html.mjs\` turns that into one page per screen. The page shows the window at its own size (one CSS pixel is one point; the 2x picture is shown at point size, so it is sharp), with the window corner radius and shadow, and every element laid over it as a real HTML element (buttons, menus as select boxes, fields with their value, text you can select) within a point of the real frame. The "Show HTML only" switch hides the picture so the HTML layer draws the screen alone. Where a control leads to another captured screen it is a link. Nothing is loaded from the network.
- Commit of the app these came from: \`${commit || 'unknown'}\`.

## Regenerate

    apps/macos/E2E/capture.sh "<path to OpenFrame Studio.app or the debug executable>"

That runs the tour in Light and in Dark, builds this folder, and renders every page in WebKit to compare it with its picture (\`scripts/mac-html/check-pages.swift\`). The generator alone:

    node scripts/mac-html/build-mac-html.mjs --both <light run folder> <dark run folder> MacHTML <commit>

## Not captured

- Screens that need the system file panel (open, save, choose a folder): Open Review Package, Restore Backup or Package and the Browse buttons. The panel runs in another process and blocks the driver.
- Pop-up menus are drawn from their items, not photographed.
- The licence and trial screens (the internal build has none) and the AI Assistant in its ready and download states (a debug build has no model files; the panel shows its unavailable state). The team gate shows only on a build that has one.
- Voice recording (needs the microphone permission dialog), drag and drop states, and screens that need real files (image attachments, PDFs).
${skipped.length ? '\n## Steps the last run skipped\n\n' + skipped.map((s) => '- ' + s.slice(0, 200)).join('\n') + '\n' : ''}`;
  fs.writeFileSync(path.join(root, 'README.md'), readme);
  console.log(`built ${pages.length} screens x 2 appearances in ${root}`);
  process.exit(0);
}

const [runDir, outDir, commitArg] = argv;
const flag = (n) => { const i = argv.indexOf(n); return i >= 0 ? argv[i + 1] : null; };
const mode = flag('--mode');
const emitNames = flag('--emit-names');
const onlyFile = flag('--only');
const otherMode = mode === 'light' ? 'dark' : mode === 'dark' ? 'light' : null;
if (!runDir || !outDir) {
  console.error('usage: build-mac-html.mjs <capture-run-dir> <MacHTML-dir> [commit] | --both <light run> <dark run> <MacHTML-dir> [commit]');
  process.exit(2);
}

// 1. Collect every capture.
const pages = [];
function walk(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, entry.name);
    if (entry.isDirectory()) walk(p);
    else if (entry.name.endsWith('.json') && path.basename(dir) === 'capture') {
      try {
        const meta = JSON.parse(fs.readFileSync(p, 'utf8'));
        const png = p.replace(/\.json$/, '.png');
        if (meta.name && fs.existsSync(png)) pages.push({ ...meta, pngPath: png });
      } catch (e) {
        console.error('skipped unreadable', p, e.message);
      }
    }
  }
}
walk(runDir);
// A name taken twice keeps the later one.
const byName = new Map();
for (const p of pages) byName.set(p.name, p);
let list = [...byName.values()].sort((a, b) => a.order - b.order || a.name.localeCompare(b.name));
const alias = new Map();
if (onlyFile) {
  const only = JSON.parse(fs.readFileSync(onlyFile, 'utf8'));
  const order = new Map(only.names.map((n, i) => [n, i]));
  list = list.filter((p) => order.has(p.name)).sort((x, y) => order.get(x.name) - order.get(y.name));
  for (const [k, v] of Object.entries(only.alias || {})) alias.set(k, v);
  for (const p of list) if (p.from && alias.has(p.from)) p.from = alias.get(p.from);
} else {
// Two screens with the same picture and the same elements are one page: keep the first, point links to it.
const seenPic = new Map();
list = list.filter((p) => {
  const key = createHash('md5').update(fs.readFileSync(p.pngPath)).digest('hex');
  if (seenPic.has(key)) { alias.set(p.name, seenPic.get(key)); return false; }
  seenPic.set(key, p.name);
  return true;
});
for (const p of list) if (p.from && alias.has(p.from)) p.from = alias.get(p.from);
// Pages that show exactly the same texts and controls are one screen too (a tab that did not change anything).
const seenSig = new Map();
list = list.filter((p) => {
  const text = (p.elements || [])
    .filter((e) => ['AXStaticText', 'AXButton', 'AXRadioButton', 'AXMenuItem', 'AXTextField', 'AXTextArea', 'AXCheckBox'].includes(e.role))
    .map((e) => `${e.label}|${e.title}|${e.value}`);
  const key = createHash('md5').update(text.join('\n')).digest('hex');
  if (seenSig.has(key)) { alias.set(p.name, seenSig.get(key)); return false; }
  seenSig.set(key, p.name);
  return true;
});
for (const p of list) if (p.from && alias.has(p.from)) p.from = alias.get(p.from);
console.log(`dropped ${alias.size} duplicate screens`);
if (list.length === 0) {
  console.error('no captures found in', runDir);
  process.exit(1);
}

}
if (emitNames) fs.writeFileSync(emitNames, JSON.stringify({ names: list.map((p) => p.name), alias: Object.fromEntries(alias) }));

fs.rmSync(outDir, { recursive: true, force: true });
fs.mkdirSync(path.join(outDir, 'assets'), { recursive: true });

const esc = (s) => String(s ?? '').replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
const num = (i) => String(i + 1).padStart(3, '0');
list.forEach((p, i) => {
  p.num = num(i);
  p.file = `${p.num}-${p.name}.html`;
  fs.copyFileSync(p.pngPath, path.join(outDir, 'assets', `${p.name}.png`));
});
const pageByName = new Map(list.map((p) => [p.name, p]));

// 2. Links: a control pressed on page A that leads to page B (B.from = A, B.via = the control).
const links = new Map(); // from page name -> Map(via -> target page)
for (const p of list) {
  if (!p.from || !pageByName.has(p.from)) continue;
  if (!links.has(p.from)) links.set(p.from, new Map());
  const m = links.get(p.from);
  if (p.via && !m.has(p.via)) m.set(p.via, p);
}
// The sidebar goes to the first page of each workspace.
const areaOfKey = {
  home: 'Project Home', screenplay: 'Screenplay', story: 'Story', ideaVault: 'Idea Vault', breakdown: 'Breakdown',
  schedule: 'Schedule', callSheets: 'Call Sheets', budget: 'Budget', visual: 'Visual Planning', files: 'Files',
  notes: 'Notes and Tasks', activity: 'Activity', recentlyDeleted: 'Recently Deleted',
};
const firstOfArea = new Map();
for (const p of list) if (!firstOfArea.has(p.area)) firstOfArea.set(p.area, p);
const homeRows = new Map([['project-card-', null]]);

const CONTROL = new Set(['AXButton', 'AXMenuButton', 'AXPopUpButton', 'AXRadioButton', 'AXCheckBox', 'AXLink', 'AXMenuItem', 'AXDisclosureTriangle', 'AXTab', 'AXSlider']);
const FIELD = new Set(['AXTextField', 'AXTextArea', 'AXComboBox', 'AXSearchField', 'AXSecureTextField']);
const TEXT = new Set(['AXStaticText', 'AXHeading']);
const ROWLIKE = new Set(['AXRow', 'AXCell', 'AXOutline', 'AXTable', 'AXList']);

function target(p, e) {
  const m = links.get(p.name);
  if (m) {
    for (const k of [e.id, e.label, e.title]) if (k && m.has(k)) return m.get(k);
  }
  const sb = /^sidebar-(.+)$/.exec(e.id || '');
  if (sb && areaOfKey[sb[1]] && firstOfArea.has(areaOfKey[sb[1]])) {
    const t = firstOfArea.get(areaOfKey[sb[1]]);
    if (t.name !== p.name) return t;
  }
  return null;
}

function name(e) { return e.label || e.title || (TEXT.has(e.role) ? e.value : '') || e.value || e.id || ''; }

function overlay(p, e, idx) {
  const f2 = (n) => Number(n).toFixed(2);
  const style = `left:${f2(e.x)}px;top:${f2(e.y)}px;width:${f2(e.w)}px;height:${f2(e.h)}px`;
  const cls = ['el', 'win-' + e.window, e.enabled === false ? 'off' : ''].filter(Boolean).join(' ');
  const attrs = `data-id="${esc(e.id)}" data-role="${esc(e.role)}" style="${style}"`;
  const label = name(e);
  const aria = label ? ` aria-label="${esc(label)}" title="${esc(label)}"` : '';
  if (FIELD.has(e.role)) {
    const tag = e.role === 'AXTextArea' || e.h > 60 ? 'textarea' : 'input';
    return tag === 'textarea'
      ? `<textarea class="${cls} field" ${attrs}${aria}${e.enabled === false ? ' disabled' : ''}>${esc(e.value)}</textarea>`
      : `<input class="${cls} field" ${attrs}${aria} value="${esc(e.value)}"${e.enabled === false ? ' disabled' : ''}>`;
  }
  if (e.role === 'AXPopUpButton' || e.role === 'AXMenuButton') {
    const shown = e.value || e.title || e.label || '';
    return `<select class="${cls} ctl menu" ${attrs}${aria}${e.enabled === false ? ' disabled' : ''}><option>${esc(shown)}</option></select>`;
  }
  if (CONTROL.has(e.role)) {
    const t = target(p, e);
    if (t) return `<a class="${cls} ctl link" ${attrs}${aria} href="${esc(t.file)}" title="${esc(label)} (opens: ${esc(t.title)})">${esc(label)}</a>`;
    return `<button type="button" class="${cls} ctl" ${attrs}${aria}${e.enabled === false ? ' disabled' : ''}>${esc(label)}</button>`;
  }
  if (TEXT.has(e.role)) {
    const text = e.value || e.label || e.title;
    const single = e.h <= 24;
    const fs = single ? Math.min(Math.max(e.h * 0.8, 9), 22) : 13;
    const lh = single ? e.h : 16;
    return `<span class="${cls} txt${single ? '' : ' multi'}" ${attrs.replace(/style="[^"]*"/, '')} style="${style};line-height:${f2(lh)}px;font-size:${f2(fs)}px">${esc(text)}</span>`;
  }
  const t = ROWLIKE.has(e.role) || e.id ? target(p, e) : null;
  if (t) return `<a class="${cls} link area" ${attrs}${aria} href="${esc(t.file)}" title="${esc(label)} (opens: ${esc(t.title)})"></a>`;
  if (e.id || label) return `<div class="${cls} box" ${attrs}${aria}></div>`;
  return '';
}

const CSS = `
:root{--bg:#d9d9de;--fg:#1d1d1f;--mut:#6b6b73;--card:#f6f6f8;--line:#c4c4cc;--acc:#c46a00;--hi:rgba(255,149,0,.9)}
@media (prefers-color-scheme:dark){:root{--bg:#2a2a2d;--fg:#f2f2f4;--mut:#a0a0a8;--card:#1f1f22;--line:#3a3a40;--acc:#ff9f0a}}
*{box-sizing:border-box}
body{margin:0;background:var(--bg);color:var(--fg);font:14px/1.45 -apple-system,BlinkMacSystemFont,"SF Pro Text","Helvetica Neue",system-ui,sans-serif}
a{color:var(--acc)}
header.top{display:flex;flex-wrap:wrap;gap:8px 16px;align-items:baseline;padding:14px 20px;border-bottom:1px solid var(--line);background:var(--card)}
header.top h1{font-size:18px;margin:0}
header.top .meta{color:var(--mut)}
nav.pager{display:flex;gap:12px;margin-left:auto;flex-wrap:wrap}
nav.pager a,nav.pager label{padding:4px 10px;border:1px solid var(--line);border-radius:6px;text-decoration:none;background:var(--bg);color:var(--fg);font-size:13px}
section.info{padding:10px 20px;color:var(--mut);font-size:13px}
section.info ul{margin:4px 0 0;padding-left:18px;columns:3 260px}
.wrap{overflow:auto;padding:28px 40px 56px;display:flex}
.stage{position:relative;margin:0 auto;flex:none;overflow:hidden;border-radius:10px;background:var(--stagebg,#1c1c1e);box-shadow:0 22px 70px 4px rgba(0,0,0,.55),0 0 0 .5px rgba(0,0,0,.5);font-family:-apple-system,BlinkMacSystemFont,"SF Pro Text",system-ui,sans-serif;color:var(--sfg,#fff)}
.stage>img{display:block;position:absolute;left:0;top:0;width:100%;height:100%;border-radius:10px;user-select:none;image-rendering:auto}
.el{position:absolute;margin:0;padding:0;border:0;background:transparent;color:transparent;font:inherit;font-size:13px;text-align:left;overflow:hidden;text-decoration:none;appearance:none;-webkit-appearance:none;-webkit-text-fill-color:transparent;opacity:1}
.txt{white-space:pre;cursor:text;user-select:text}
.txt.multi{white-space:pre-wrap}
.txt::selection,.field::selection{background:rgba(255,149,0,.45);color:transparent}
.field{caret-color:transparent;resize:none;cursor:text}
.field:focus{outline:2px solid var(--hi)}
.ctl,.link{cursor:pointer;display:block}
.ctl:hover,.link:hover{outline:2px solid var(--hi);outline-offset:-1px;background:rgba(255,149,0,.12)}
.ctl.off{cursor:not-allowed}
.box{pointer-events:none}
.link.area:hover{background:rgba(255,149,0,.08)}
body.behind .win-main{pointer-events:none}
body.show .ctl,body.show .field{outline:1px dashed rgba(255,149,0,.9);outline-offset:-1px}
body.show .link{outline:2px solid rgba(0,160,255,.9);outline-offset:-1px;background:rgba(0,160,255,.12)}
body.show .box{outline:1px dotted rgba(128,128,255,.45)}
/* HTML only: the picture is hidden and the real elements draw the screen */
body.htmlonly .stage>img{display:none}
body.htmlonly .txt,body.htmlonly .ctl,body.htmlonly .field{color:var(--sfg);-webkit-text-fill-color:var(--sfg)}
body.htmlonly .ctl,body.htmlonly .field{border:1px solid rgba(128,128,128,.35);border-radius:6px;display:flex;align-items:center;justify-content:center;padding:0 6px;background:rgba(128,128,128,.12)}
body.htmlonly .field{justify-content:flex-start}
body.htmlonly .box{outline:1px dotted rgba(128,128,128,.25)}
body.htmlonly .win-sheet.box,body.htmlonly .win-menu.ctl{background:var(--stagebg)}
/* bare: just the window at its own size, for the pixel check */
body.bare header.top,body.bare section.info{display:none}
html:has(body.bare){overflow:hidden}
body.bare{background:#000;overflow:hidden}
body.bare .wrap{padding:0;display:block;overflow:hidden}
body.bare .stage{margin:0;box-shadow:none;border-radius:0;background:#000}
body.bare .stage>img{border-radius:0}
body.bare .el{outline:none!important;background:transparent!important}
.index{padding:20px;max-width:1500px;margin:0 auto}
.index h2{margin:28px 0 10px;font-size:16px;border-bottom:1px solid var(--line);padding-bottom:4px}
.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(240px,1fr));gap:14px}
.card{background:var(--card);border:1px solid var(--line);border-radius:10px;overflow:hidden;text-decoration:none;color:var(--fg);display:block}
.card:hover{border-color:var(--acc)}
.card img{width:100%;aspect-ratio:1440/652;object-fit:cover;object-position:top left;display:block;background:#000}
.card div{padding:8px 10px;font-size:13px}
.card small{color:var(--mut);display:block}
`;

const JS = `
(function(){
var b=document.body;
if(location.hash==='#bare')b.classList.add('bare');
if(location.hash==='#html')b.classList.add('htmlonly');
function bind(id,cls){var c=document.getElementById(id);if(c)c.addEventListener('change',function(){b.classList.toggle(cls,c.checked)});}
bind('show','show');bind('htmlonly','htmlonly');
document.addEventListener('keydown',function(e){if(e.target&&/INPUT|TEXTAREA|SELECT/.test(e.target.tagName))return;
var a=document.querySelector(e.key==='ArrowLeft'?'a[rel=prev]':e.key==='ArrowRight'?'a[rel=next]':'a[rel=none]');if(a)location.href=a.href;});
})();
`;

list.forEach((p, i) => {
  const prev = list[i - 1], next = list[i + 1];
  const els = p.elements || [];
  const behind = p.sheets > 0 || els.some((e) => e.window === 'menu');
  const body = els.map((e, idx) => overlay(p, e, idx)).filter(Boolean).join('\n');
  const controls = [];
  const seen = new Set();
  for (const e of els) {
    if (!(CONTROL.has(e.role) || FIELD.has(e.role))) continue;
    const l = name(e);
    if (!l || seen.has(l) || l.length > 60) continue;
    seen.add(l);
    controls.push(`<li>${esc(l)} <small>(${esc(e.role.replace('AX', '').toLowerCase())}${e.id ? ', ' + esc(e.id) : ''})</small></li>`);
    if (controls.length >= 18) break;
  }
  const from = p.from && pageByName.get(p.from);
  const html = `<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>${esc(p.title)} - OpenFrame Studio for Mac</title><style>${CSS}</style></head>
<body class="${behind ? 'behind' : ''}">
<header class="top">
<h1>${esc(p.title)}</h1>
<span class="meta">${esc(p.area)} &middot; page ${p.num} of ${list.length} &middot; window ${Math.round(p.width)}&times;${Math.round(p.height)} pt &middot; ${mode ? mode + ' appearance (<a href="../' + otherMode + '/' + esc(p.file) + '">' + otherMode + ' version</a>)' : (p.dark === false ? 'light' : 'dark') + ' appearance'}</span>
<nav class="pager">
${prev ? `<a rel="prev" href="${esc(prev.file)}">&larr; ${esc(prev.title)}</a>` : ''}
<a href="${mode ? '../index.html' : 'index.html'}${mode === 'light' ? '#light' : ''}">All pages</a>
${next ? `<a rel="next" href="${esc(next.file)}">${esc(next.title)} &rarr;</a>` : ''}
<label><input type="checkbox" id="show"> Show controls</label>
<label><input type="checkbox" id="htmlonly"> Show HTML only</label>
</nav>
</header>
<section class="info">
${from ? `Reached from <a href="${esc(from.file)}">${esc(from.title)}</a>${p.via ? ` by <code>${esc(p.via)}</code>` : ''}. ` : ''}Controls on this screen:
<ul>${controls.join('')}</ul>
</section>
<div class="wrap"><div class="stage" style="width:${p.width}px;height:${p.height}px;--stagebg:${esc(p.bg || '#1c1c1e')};--sfg:${(mode ? mode === 'light' : p.dark === false) ? '#1d1d1f' : '#f2f2f4'}">
<img src="assets/${esc(p.name)}.png" alt="${esc(p.title)}" width="${p.width}" height="${p.height}">
${body}
</div></div>
<script>${JS}</script>
</body></html>
`;
  fs.writeFileSync(path.join(outDir, p.file), html);
});

// 3. The index, grouped by area in tour order.
const groups = [];
for (const p of list) {
  let g = groups.find((x) => x.area === p.area);
  if (!g) groups.push((g = { area: p.area, pages: [] }));
  g.pages.push(p);
}
const index = `<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>OpenFrame Studio for Mac, every screen</title><style>${CSS}</style></head>
<body><header class="top"><h1>OpenFrame Studio for Mac: every screen</h1>
<span class="meta">${list.length} pages in ${groups.length} areas${commitArg ? ' &middot; commit ' + esc(commitArg) : ''}</span></header>
<div class="index">
<p class="meta" style="color:var(--mut)">Each page is a real picture of the running app with its real controls laid on top. Controls that lead to another captured screen are links. Use the Show controls switch on a page to see them.</p>
<nav>${groups.map((g) => `<a href="#${esc(g.area.replace(/\W+/g, '-'))}">${esc(g.area)} (${g.pages.length})</a>`).join(' &middot; ')}</nav>
${groups.map((g) => `<h2 id="${esc(g.area.replace(/\W+/g, '-'))}">${esc(g.area)} <small>(${g.pages.length})</small></h2>
<div class="grid">${g.pages.map((p) => `<a class="card" href="${esc(p.file)}"><img loading="lazy" src="assets/${esc(p.name)}.png" alt=""><div>${p.num} ${esc(p.title)}<small>${esc(p.name)}</small></div></a>`).join('')}</div>`).join('\n')}
</div></body></html>
`;
fs.writeFileSync(path.join(outDir, 'index.html'), index);
fs.writeFileSync(path.join(outDir, 'pages.json'), JSON.stringify(list.map((p) => ({ n: p.num, name: p.name, title: p.title, area: p.area, from: p.from, via: p.via, file: p.file, width: p.width, height: p.height, elements: (p.elements || []).length })), null, 1));
console.log(`wrote ${list.length} pages in ${groups.length} areas to ${outDir}`);
for (const g of groups) console.log(`  ${g.area}: ${g.pages.length}`);

if (mode) { console.log(`wrote ${list.length} pages (${mode}) to ${outDir}`); process.exit(0); }

// 4. README with how the pages were made and what the tour could not reach.
const skipped = [];
(function collect(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, entry.name);
    if (entry.isDirectory()) collect(p);
    else if (entry.name === 'skipped.tsv') {
      for (const line of fs.readFileSync(p, 'utf8').split('\n')) if (line.trim()) skipped.push(line.replace('\t', ': '));
    }
  }
})(runDir);
const readme = `# MacHTML: every screen of OpenFrame Studio for the Mac

${list.length} pages in ${groups.length} areas, made from the real app. Open \`index.html\`.

## How these were made

- The capture tour (\`apps/macos/Sources/OpenFrameE2EDriver/Scenarios/CaptureTour.swift\`, scenarios \`zz-capture/*\`) drives the
  real app through the accessibility tree, seeds a project called Night Bus (the bundled example screenplay, story acts and
  cards, characters, Idea Vault items, a breakdown, a schedule with days, a call sheet, budget lines, notes and tasks) and
  visits every screen it can reach.
- For every screen it writes a 2x picture of the window (the window server's own image of the window, with any sheet
  drawn over it) and a list of every accessibility element with its role, identifier, label, value, enabled state and frame.
  Menus are drawn from the real menu items, because an open menu is not a window the app can draw.
- \`scripts/mac-html/build-mac-html.mjs\` turns that into one page per screen: the picture, and over it every element as a
  real HTML element (buttons, fields with their value, text you can select) with its label and identifier
  (\`data-id\`). Where a control leads to another captured screen it is a link. No scripts are needed to read the pages and
  nothing is loaded from the network.
- Commit of the app these came from: \`${commitArg || 'unknown'}\`.

## Regenerate

    apps/macos/E2E/capture.sh "<path to OpenFrame Studio.app or the debug executable>"

That runs the tour and then the generator (it writes this folder again). The generator alone:

    node scripts/mac-html/build-mac-html.mjs <capture run folder> MacHTML <commit>

## Not captured

- Screens that need the system file panel (open, save, choose a folder): Open Review Package, Restore Backup or Package
  and the Browse buttons. The panel runs in another process and blocks the driver.
- Pop-up menus are drawn from their items, not photographed.
- The team gate (a debug build has none), the licence and trial screens (the internal build has none) and the AI
  Assistant in its ready and download states (the debug build has no model files; the panel shows its unavailable state).
- Voice recording (needs the microphone permission dialog), drag and drop states, and screens that need real
  files (image attachments, PDFs).
${skipped.length ? '\n## Steps the last run skipped\n\n' + skipped.map((s) => '- ' + s.slice(0, 200)).join('\n') + '\n' : ''}`;
fs.writeFileSync(path.join(outDir, 'README.md'), readme);
