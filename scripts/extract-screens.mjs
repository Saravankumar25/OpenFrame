// Screen catalog: one record per captured app screen, read from the real mockups.
// Windows: loads every WindowsHTML page in Chromium and reads the visible controls of the active layer (the open
// dialog or menu, else the window). Mac: MacHTML ships pages.json with the accessibility text of every screen.
// Output: research/screens-win.json, research/screens-mac.json. Run: node scripts/extract-screens.mjs [win|mac]
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium } from 'playwright';

const ROOT = path.resolve(import.meta.dirname, '..');
export const SRC = { win: path.join(ROOT, 'research', 'WindowsHTML-src'), mac: path.join(ROOT, 'research', 'MacHTML-src') };
const only = process.argv[2];

// Windows file-name prefix to the area name used in the manual and tutorials (Mac pages.json has its own areas).
const WIN_AREAS = [
  [/^app-library-setup/, 'Library setup'], [/^app-first-launch/, 'First launch'], [/^(app-new-project|app-home|home-new-project)/, 'Application Home'],
  [/^apphome-/, 'Application Home'], [/^sp-import/, 'Import a screenplay'], [/^shell-/, 'Project window and navigation'],
  [/^vault-/, 'Idea Vault'], [/^story-/, 'Story and Beat Board'], [/^sp-/, 'Screenplay'], [/^bd-/, 'Breakdown'],
  [/^ai-/, 'AI Assistant'], [/^pkg-/, 'Backups, packages and exchange'], [/^licence-/, 'Trial and purchase'],
  [/^prod-(overview|catalog)/, 'Production Plan: overview and catalog'], [/^prod-location/, 'Locations'],
  [/^prod-(cast|crew)/, 'Cast and crew'], [/^prod-moodboard/, 'Moodboards'], [/^prod-storyboard/, 'Storyboards'],
  [/^prod-shot/, 'Shot lists'], [/^prod-schedule/, 'Shooting schedule'], [/^prod-dood/, 'Day Out of Days'],
  [/^prod-daily/, 'Daily View'], [/^prod-sides/, 'Sides'], [/^prod-reports/, 'Reports'], [/^prod-budget/, 'Budget'],
  [/^cs-/, 'Call sheets'], [/^files-/, 'Files'], [/^notes-/, 'Notes and tasks'], [/^activity-/, 'Activity'],
  [/^trash-/, 'Recently Deleted'], [/^templates-/, 'Templates'], [/^settings-/, 'Settings'],
];
const PREFIX = /^(app-library-setup|app-first-launch|app|apphome|home|sp-import|shell|vault|story|sp|bd|ai|pkg|licence|prod|cs|files|notes|activity|trash|templates|settings)-/;
const humanize = (name) => {
  const s = name.replace(PREFIX, '').replace(/-/g, ' ');
  return s.charAt(0).toUpperCase() + s.slice(1);
};
// Test-machine paths and the capture account name are not product content.
const PRIVATE = /\/var\/folders|\/Users\/|[A-Z]:\\Users\\|AppData|E2E Tester/i;
const clean = (arr) => [...new Set(arr.map((s) => String(s).replace(/\s+/g, ' ').trim()).filter((s) => s && s.length < 400 && !PRIVATE.test(s)))];

async function extractWin() {
  const dir = path.join(SRC.win, 'light');
  const files = fs.readdirSync(dir).filter((f) => f.endsWith('.html')).sort();
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
  const out = [];
  for (const file of files) {
    const [, n, name] = file.match(/^(\d+)-(.+)\.html$/);
    await page.goto(pathToFileURL(path.join(dir, file)).href, { waitUntil: 'load' });
    const info = await page.evaluate(() => {
      const vis = (el) => { const r = el.getBoundingClientRect(); const cs = getComputedStyle(el); return r.width > 1 && r.height > 1 && cs.visibility !== 'hidden' && cs.display !== 'none' && +cs.opacity > 0.05; };
      const txt = (el) => (el.getAttribute('aria-label') || el.innerText || el.value || el.getAttribute('title') || '').trim();
      const layers = [...document.querySelectorAll('[role=dialog],[role=alertdialog],[role=menu],[role=listbox]')].filter(vis);
      const top = layers[layers.length - 1] || null;
      const scope = top || document.getElementById('root') || document.body;
      const q = (sel) => [...scope.querySelectorAll(sel)].filter(vis);
      const label = (el) => {
        if (el.getAttribute('aria-label')) return el.getAttribute('aria-label');
        if (el.id) { const l = document.querySelector(`label[for="${CSS.escape(el.id)}"]`); if (l) return l.innerText; }
        const p = el.closest('label'); return p ? p.innerText : el.getAttribute('placeholder') || el.name || '';
      };
      const heading = (root) => { const h = [...root.querySelectorAll('h1,h2,h3,[role=heading]')].find(vis); return h ? h.innerText.trim() : ''; };
      const ws = document.querySelector('main h1, .ws-title, main h2');
      return {
        layer: top ? (top.getAttribute('role')) : 'window',
        layerTitle: top ? (top.getAttribute('aria-label') || heading(top)) : '',
        workspace: ws && vis(ws) ? ws.innerText.trim() : '',
        headings: q('h1,h2,h3,h4,[role=heading]').map((e) => e.innerText),
        buttons: q('button,[role=button],a[href]').map(txt),
        fields: q('input:not([type=hidden]):not([type=checkbox]):not([type=radio]),textarea,select').map((e) => ({ label: label(e), value: e.tagName === 'SELECT' ? (e.selectedOptions[0]?.text || '') : (e.value || e.getAttribute('value') || '') })),
        toggles: q('input[type=checkbox],input[type=radio],[role=switch],[role=checkbox],[role=radio]').map((e) => ({ label: label(e) || txt(e), on: e.checked || e.getAttribute('aria-checked') === 'true' })),
        tabs: q('[role=tab]').map((e) => ({ label: txt(e), on: e.getAttribute('aria-selected') === 'true' })),
        items: q('[role=menuitem],[role=menuitemcheckbox],[role=menuitemradio],[role=option]').map(txt),
        alerts: [...document.querySelectorAll('[role=alert],[role=status],.toast')].filter(vis).map((e) => e.innerText),
        nav: [...document.querySelectorAll('nav [aria-current=page], [aria-current=page]')].filter(vis).map(txt),
      };
    });
    const area = (WIN_AREAS.find(([re]) => re.test(name)) || [null, 'Other'])[1];
    out.push({
      n, name, file, title: humanize(name), area, layer: info.layer, layerTitle: clean([info.layerTitle])[0] || '',
      workspace: clean([info.workspace])[0] || '', current: clean(info.nav),
      headings: clean(info.headings), buttons: clean(info.buttons).slice(0, 80),
      fields: info.fields.map((f) => ({ label: clean([f.label])[0] || '', value: clean([f.value])[0] || '' })).filter((f) => f.label || f.value),
      toggles: info.toggles.map((t) => ({ label: clean([t.label])[0] || '', on: t.on })).filter((t) => t.label),
      tabs: info.tabs.map((t) => ({ label: clean([t.label])[0] || '', on: t.on })).filter((t) => t.label),
      items: clean(info.items), alerts: clean(info.alerts), width: 1600, height: 1000,
    });
    if (out.length % 100 === 0) console.log(`win ${out.length}/${files.length}`);
  }
  await browser.close();
  return out;
}

function extractMac() {
  const pages = JSON.parse(fs.readFileSync(path.join(SRC.mac, 'light', 'pages.json'), 'utf8'));
  return pages.map((p) => ({ n: p.n, name: p.name, file: p.file, title: p.title, area: p.area, picture: p.picture, width: p.width, height: p.height, texts: clean(p.expect || []) }));
}

for (const plat of only ? [only] : ['win', 'mac']) {
  const list = plat === 'win' ? await extractWin() : extractMac();
  const dark = fs.readdirSync(path.join(SRC[plat], 'dark')).filter((f) => f.endsWith('.html'));
  const missingDark = list.filter((s) => !dark.includes(s.file)).map((s) => s.file);
  if (missingDark.length) throw new Error(`${plat}: no dark page for ${missingDark.join(', ')}`);
  fs.writeFileSync(path.join(ROOT, 'research', `screens-${plat}.json`), JSON.stringify(list, null, 1));
  const areas = {};
  for (const s of list) areas[s.area] = (areas[s.area] || 0) + 1;
  console.log(`${plat}: ${list.length} screens`, areas);
}
