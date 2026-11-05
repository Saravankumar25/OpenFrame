// Static build: src/ -> dist/. Run: npm run build. Pages come from src/pages/*.mjs (see docs/CONTRACT.md).
// The real app screens are interactive pages made by scripts/shots.mjs (.cache/shots/pages, sizes and crops listed in
// manifest.json) and copied to /mockups/<win|mac>/<light|dark>/ (shared assets in /mockups/<plat>/assets/); the pages
// show them in frames and in the viewer. The screen catalogs research/screens-<plat>.json name every one of them.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { pathToFileURL } from 'node:url';
import * as esbuild from 'esbuild';
import { SITE_URL, SITE_NAME } from '../site.config.mjs';
import { html, raw, esc, md } from '../src/lib/html.mjs';
import { ICON, MARK } from '../src/lib/icons.mjs';
import { FEATURES, FEATURE_GROUPS, PARTS, OS, COMPANY } from '../src/content/nav.mjs';
import { renderPage, makeHelpers, BOOT } from '../src/lib/layout.mjs';
import { loadManual } from './check-content.mjs';
import { CATALOG } from '../src/content/catalog.mjs';
import { writeSitemapAndRobots, writeLlms, headersFile } from './seo-files.mjs';
import { DISCORD_INVITE } from '../src/content/social.mjs';

const ROOT = path.resolve(import.meta.dirname, '..');
const SRC = path.join(ROOT, 'src'), DIST = process.env.OF_DIST ? path.resolve(process.env.OF_DIST) : path.join(ROOT, 'dist'); // OF_DIST: a private output folder for parallel work
const rel = (...p) => path.join(ROOT, ...p);
const t0 = Date.now();
const SHOTS = process.env.OF_SHOTS ? path.resolve(process.env.OF_SHOTS) : rel('.cache', 'shots'); // OF_SHOTS: a folder of test pictures

fs.rmSync(DIST, { recursive: true, force: true });
fs.mkdirSync(DIST, { recursive: true });

// ---------- data
const facts = JSON.parse(fs.readFileSync(rel('research', 'facts.json'), 'utf8'));
const cat = {};
for (const plat of ['win', 'mac']) cat[plat] = new Map(JSON.parse(fs.readFileSync(rel('research', `screens-${plat}.json`), 'utf8')).map((s) => [s.name, s]));
const manual = { win: await loadManual('win'), mac: await loadManual('mac') };

// ---------- assets (content-hashed names)
const hash = (buf) => crypto.createHash('sha256').update(buf).digest('hex').slice(0, 10);
const cssFiles = ['site.css', 'pages-base.css', 'theme.css', 'shots.css', 'search.css', 'legal.css', 'getapp.css', 'mosaic.css'].map((f) => path.join(SRC, 'assets', 'css', f));
const cssOut = (await esbuild.transform(cssFiles.map((f) => fs.readFileSync(f, 'utf8')).join('\n'), { loader: 'css', minify: true })).code;
// Each script is its own scope, so the files cannot clash on names.
const jsSrc = ['getapp.js', 'site.js', 'lightbox.js', 'search.js', 'mosaic.js'].map((f) => `;(function(){\n${fs.readFileSync(path.join(SRC, 'assets', 'js', f), 'utf8')}\n})();`).join('\n');
const jsOut = (await esbuild.transform(jsSrc, { loader: 'js', minify: true, target: 'es2019' })).code;
fs.mkdirSync(path.join(DIST, 'assets', 'fonts'), { recursive: true });
const cssName = `/assets/site.${hash(cssOut)}.css`, jsName = `/assets/site.${hash(jsOut)}.js`;
fs.writeFileSync(path.join(DIST, cssName), cssOut);
fs.writeFileSync(path.join(DIST, jsName), jsOut);
for (const f of fs.readdirSync(path.join(SRC, 'assets', 'fonts'))) fs.copyFileSync(path.join(SRC, 'assets', 'fonts', f), path.join(DIST, 'assets', 'fonts', f));
fs.cpSync(path.join(SRC, 'static'), DIST, { recursive: true });


// ---------- app screen pages
// manifest.json: { win|mac: { <screen name>: { page: [w, h], card: [x, y, w, h], full: [x, y, w, h], ... } } }. A screen
// the pages show but the manifest or the pages folder lacks stops the build further down, with its name.
const manifestFile = path.join(SHOTS, 'manifest.json');
if (!fs.existsSync(manifestFile) && !process.env.OF_ALLOW_MISSING) throw new Error(`screen manifest missing: ${manifestFile} (run node scripts/shots.mjs)`);
const shots = fs.existsSync(manifestFile) ? JSON.parse(fs.readFileSync(manifestFile, 'utf8')) : { win: {}, mac: {} };
if (fs.existsSync(path.join(SHOTS, 'pages'))) fs.cpSync(path.join(SHOTS, 'pages'), path.join(DIST, 'mockups'), { recursive: true });
const hasPage = (plat, file) => ['light', 'dark'].every((t) => fs.existsSync(path.join(SHOTS, 'pages', plat, t, file)));

// ---------- pages
const missing = new Set(), noShot = new Set();
const ctx = { html, raw, esc, md, ICON, MARK, FEATURES, FEATURE_GROUPS, PARTS, OS, COMPANY, facts, cat, manual, SITE_URL, ...makeHelpers(cat, missing, shots, noShot, hasPage) };
const pages = [];
for (const f of fs.readdirSync(path.join(SRC, 'pages')).filter((f) => f.endsWith('.mjs') && !f.startsWith('_')).sort()) {
  const mod = await import(pathToFileURL(path.join(SRC, 'pages', f)).href);
  for (const p of mod.default(ctx)) pages.push({ ...p, _from: f });
}

const paths = new Set();
for (const p of pages) {
  if (!/^\/([a-z0-9-]+\/)*$|^\/(403|404|500|503)\.html$/.test(p.path)) throw new Error(`bad path ${p.path} from ${p._from}`);
  if (paths.has(p.path)) throw new Error(`duplicate path ${p.path} from ${p._from}`);
  paths.add(p.path);
}

// Platform control targets on a platform page: its counterpart, else the other platform's landing page.
function switchHref(p) {
  const out = { win: '/windows/', mac: '/mac/' };
  if (!p.os) return out;
  out[p.os] = p.path;
  const other = p.os === 'win' ? 'mac' : 'win';
  if (p.alternates?.[other]) { out[other] = p.alternates[other]; return out; }
  const swapped = p.path.replace(`/${OS[p.os].slug}/`, `/${OS[other].slug}/`);
  out[other] = paths.has(swapped) ? swapped : `/${OS[other].slug}/`;
  return out;
}

const DASH = /[–—]/;
for (const p of pages) {
  const out = renderPage({ ...p, body: String(p.body) }, { css: cssName, js: jsName, switchHref: switchHref(p) });
  const text = out.replace(/<script[\s\S]*?<\/script>/g, '');
  if (DASH.test(text)) throw new Error(`em or en dash in ${p.path} (${p._from})`);
  const file = p.path.endsWith('.html') ? path.join(DIST, p.path) : path.join(DIST, p.path, 'index.html');
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, out);
}
if (missing.size) {
  const list = [...missing].sort().join(', ');
  if (process.env.OF_ALLOW_MISSING) console.warn(`WARNING screens not in the catalog: ${list}`);
  else throw new Error(`screens not in research/screens-<plat>.json: ${list}`);
}
if (noShot.size) {
  const list = [...noShot].sort().join(', ');
  if (process.env.OF_ALLOW_MISSING) console.warn(`WARNING ${noShot.size} screens have no page or crop in ${manifestFile}: ${list}`);
  else throw new Error(`${noShot.size} screens have no page or crop in ${manifestFile}: ${list}`);
}

await (await import('./search-index.mjs')).writeSearchIndex(DIST, manual); // search index

// ---------- tutorial to screen map (machine-readable, also kept in docs/)
const map = { tutorials: [], screens: {} };
for (const plat of ['win', 'mac']) {
  for (const a of manual[plat]) for (const w of a.workflows) {
    map.tutorials.push({ id: w.id, platform: plat, area: a.area, title: w.title, url: `/${OS[plat].slug}/tutorials/${w.id}/`, screens: w.steps.map((s) => s.screen) });
    for (const s of w.steps) { const k = `${plat}/${s.screen}`; map.screens[k] ||= []; if (!map.screens[k].includes(w.id)) map.screens[k].push(w.id); }
  }
}
// (the map stays in docs/ only: it names internal screen files and is not published)
fs.writeFileSync(rel('docs', 'tutorial-screen-map.json'), JSON.stringify(map, null, 1));

// ---------- SEO and hosting files
const today = new Date().toISOString().slice(0, 10);
writeSitemapAndRobots(DIST, pages); // sitemap.xml and robots.txt (scripts/seo-files.mjs)
// ---------- standard site files
const webmanifest = { name: SITE_NAME, short_name: 'Straft', description: 'Film production software for ideas, scripts, schedules and call sheets.', start_url: '/', scope: '/', display: 'browser',
  background_color: '#1c1b1a', theme_color: '#1f1e1d', lang: 'en',
  icons: [{ src: '/icon-192.png', sizes: '192x192', type: 'image/png' }, { src: '/icon-512.png', sizes: '512x512', type: 'image/png' }, { src: '/icon-maskable-512.png', sizes: '512x512', type: 'image/png', purpose: 'maskable' }] };
fs.writeFileSync(path.join(DIST, 'site.webmanifest'), JSON.stringify(webmanifest, null, 2));
fs.mkdirSync(path.join(DIST, '.well-known'), { recursive: true });
const nextYear = new Date(Date.now() + 365 * 864e5).toISOString().replace(/\.\d+Z$/, 'Z');
fs.writeFileSync(path.join(DIST, '.well-known', 'security.txt'), `Contact: ${SITE_URL}/contact/
Expires: ${nextYear}
Preferred-Languages: en
Canonical: ${SITE_URL}/.well-known/security.txt
Policy: ${SITE_URL}/legal/privacy/
`);
fs.writeFileSync(path.join(DIST, 'humans.txt'), `/* TEAM */
Technical Co-Founding Engineer: Saravan Kumar M K and team
Product Co-Founding Engineer: Badrinath Reddy A and team

/* SITE */
Last update: ${today}
Standards: HTML5, CSS3
Language: English
Store: ${SITE_URL}/
`);
const n = (plat) => manual[plat].reduce((k, a) => k + a.workflows.length, 0);
writeLlms(DIST, { paths, manual, FEATURE_GROUPS, CATALOG, OS }); // llms.txt and llms-full.txt

fs.writeFileSync(path.join(DIST, '_headers'), headersFile(BOOT)); // security headers, CSP and cache rules
// Old URLs keep working: the design package's .html pages and the earlier tutorial ids.
const OLD = { 'index.html': '/', 'pricing.html': '/pricing/', 'download.html': '/download/', 'tutorials.html': '/tutorials/', 'faq.html': '/faq/', 'migrate.html': '/switch/',
  'compare.html': '/compare/', 'why-local.html': '/local-and-private/', 'themes.html': '/themes/', 'about.html': '/about/', 'contact.html': '/contact/',
  'privacy.html': '/legal/privacy/', 'terms.html': '/legal/terms/', 'features/index.html': '/features/' };
const OLD_TUTORIALS = ['ai-assistant-ask', 'ai-assistant-setup', 'backup-restore', 'breakdown', 'budget', 'call-sheet', 'cast-crew', 'characters', 'create-project', 'drafts-revisions', 'export-script', 'files', 'first-launch', 'idea-vault', 'import-script', 'locations', 'manage-projects', 'moodboard', 'pages-templates', 'project-tour', 'recently-deleted', 'reports', 'schedule', 'screenplay-notes', 'settings', 'shot-list', 'sides', 'story-board', 'storyboard', 'tasks', 'themes-appearance', 'write-screenplay'];
const redirects = ['/features/ai-assistant/ /features/straft-ai/ 301', '/features/ai-assistant.html /features/straft-ai/ 301'].concat(Object.entries(OLD).map(([a, b]) => `/${a} ${b} 301`)).concat(FEATURES.map((f) => `/features/${f.slug}.html /features/${f.slug}/ 301`));
for (const plat of ['win', 'mac']) for (const id of OLD_TUTORIALS) {
  const p = `/${OS[plat].slug}/tutorials/${id}/`;
  if (!paths.has(p)) redirects.push(`${p} /${OS[plat].slug}/tutorials/ 302`);
}
// The per-platform screen lists were folded into the tutorials (owner, 2026-10-31).
redirects.push('/windows/screens/ /windows/tutorials/ 301', '/mac/screens/ /mac/tutorials/ 301');
// The branded Discord link: a temporary (302) redirect, so the invite can change without a cached move. It is the only rule that leaves this site.
redirects.push(`/discord ${DISCORD_INVITE} 302`, `/discord/ ${DISCORD_INVITE} 302`);
fs.writeFileSync(path.join(DIST, '_redirects'), redirects.join('\n') + '\n');

const files = fs.readdirSync(DIST, { recursive: true }).length;
console.log(`built ${pages.length} pages (${n('win')} Windows and ${n('mac')} Mac tutorials, ${files} files) in ${Date.now() - t0} ms -> dist/`);
