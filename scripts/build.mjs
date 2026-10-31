// Static build: src/ -> dist/. Run: npm run build. Pages come from src/pages/*.mjs (see docs/CONTRACT.md).
// The real app screens (research/WindowsHTML-src, research/MacHTML-src) are copied to /mockups/<win|mac>/<light|dark>/
// and embedded in the pages; the screen catalogs research/screens-<plat>.json name every one of them.
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

const ROOT = path.resolve(import.meta.dirname, '..');
const SRC = path.join(ROOT, 'src'), DIST = process.env.OF_DIST ? path.resolve(process.env.OF_DIST) : path.join(ROOT, 'dist'); // OF_DIST: a private output folder for parallel work
const rel = (...p) => path.join(ROOT, ...p);
const t0 = Date.now();
const SKIP_MOCKUPS = !!process.env.OF_SKIP_MOCKUPS; // faster local page builds; never for a deploy

if (SKIP_MOCKUPS && fs.existsSync(path.join(DIST, 'mockups'))) {
  for (const e of fs.readdirSync(DIST)) if (e !== 'mockups') fs.rmSync(path.join(DIST, e), { recursive: true, force: true });
} else fs.rmSync(DIST, { recursive: true, force: true });
fs.mkdirSync(DIST, { recursive: true });

// ---------- data
const facts = JSON.parse(fs.readFileSync(rel('research', 'facts.json'), 'utf8'));
const cat = {};
for (const plat of ['win', 'mac']) cat[plat] = new Map(JSON.parse(fs.readFileSync(rel('research', `screens-${plat}.json`), 'utf8')).map((s) => [s.name, s]));
const manual = { win: await loadManual('win'), mac: await loadManual('mac') };

// ---------- assets (content-hashed names)
const hash = (buf) => crypto.createHash('sha256').update(buf).digest('hex').slice(0, 10);
const cssFiles = ['site.css', 'pages-base.css', 'theme.css', 'shots.css', 'search.css', 'legal.css'].map((f) => path.join(SRC, 'assets', 'css', f));
const cssOut = (await esbuild.transform(cssFiles.map((f) => fs.readFileSync(f, 'utf8')).join('\n'), { loader: 'css', minify: true })).code;
// Each script is its own scope, so the files cannot clash on names.
const jsSrc = ['site.js', 'lightbox.js', 'search.js'].map((f) => `;(function(){\n${fs.readFileSync(path.join(SRC, 'assets', 'js', f), 'utf8')}\n})();`).join('\n');
const jsOut = (await esbuild.transform(jsSrc, { loader: 'js', minify: true, target: 'es2019' })).code;
fs.mkdirSync(path.join(DIST, 'assets', 'fonts'), { recursive: true });
const cssName = `/assets/site.${hash(cssOut)}.css`, jsName = `/assets/site.${hash(jsOut)}.js`;
fs.writeFileSync(path.join(DIST, cssName), cssOut);
fs.writeFileSync(path.join(DIST, jsName), jsOut);
for (const f of fs.readdirSync(path.join(SRC, 'assets', 'fonts'))) fs.copyFileSync(path.join(SRC, 'assets', 'fonts', f), path.join(DIST, 'assets', 'fonts', f));
fs.cpSync(path.join(SRC, 'static'), DIST, { recursive: true });


// ---------- app screens
// The captures ran in test folders under a test account. Published copies show ordinary example paths and names
// instead (Windows screens are live text; on the Mac only the text layer can change, the window picture cannot).
const clean = (h) => h
  .replace(/C:\\OpenFrame\\tests\\e2e\\\.run\\[^\\<"]+\\zz-capture\\appdata/g, 'C:\\Users\\You\\AppData\\Local')
  .replace(/C:\\OpenFrame\\tests\\e2e\\\.run\\[^\\<"]+\\zz-capture\\home/g, 'C:\\Users\\You')
  .replace(/C%3A%5COpenFrame%5Ctests%5Ce2e%5C\.run%5C[^"<%]+%5Czz-capture%5Cappdata/g, 'C%3A%5CUsers%5CYou%5CAppData%5CLocal')
  .replace(/C%3A%5COpenFrame%5Ctests%5Ce2e%5C\.run%5C[^"<%]+%5Czz-capture%5Chome/g, 'C%3A%5CUsers%5CYou')
  .replace(/C:\\OpenFrame\\tests\\e2e\\\.run\\[^\\<"]+\\zz-capture/g, 'C:\\Users\\You\\Documents')
  .replace(/C%3A%5COpenFrame%5Ctests%5Ce2e%5C\.run%5C[^"<%]+%5Czz-capture/g, 'C%3A%5CUsers%5CYou%5CDocuments')
  .replace(/(?:\/private)?\/var\/folders\/[^/<"]+\/[^/<"]+\/T\/\/?openframe-e2e\.[^/<"]+/g, '/Users/you')
  .replace(/\/Users\/you\/data-zz-capture[^/<"]*/g, '/Users/you/OpenFrame')
  .replace(/Saravan Kumar/g, 'Your Name').replace(/E2E Tester/g, 'Your Name')
  // No build notes in published pages: the Windows generator comment (commit, capture size, script) and the Mac
  // viewer bar (page numbers, capture size) go; the screens themselves are unchanged.
  .replace(/<!--[\s\S]*?-->\n?/g, '')
  .replace(/<div class="band">[\s\S]*?<\/header>\s*(?:<div class="leads">[\s\S]*?<\/div>\s*)?<\/div>/, '');
const MAC_EMBED = '<style>.band{display:none}.wrap{padding:0;overflow:hidden}.win{box-shadow:none;border-radius:0}html,body{background:var(--winbg,transparent)}</style>';
if (!SKIP_MOCKUPS) {
  const W = rel('research', 'WindowsHTML-src'), M = rel('research', 'MacHTML-src');
  for (const d of [W, M]) if (!fs.existsSync(path.join(d, 'light'))) throw new Error(`screen source missing: ${d}`);
  const out = path.join(DIST, 'mockups');
  for (const t of ['light', 'dark']) {
    fs.mkdirSync(path.join(out, 'win', t), { recursive: true });
    for (const f of fs.readdirSync(path.join(W, t))) fs.writeFileSync(path.join(out, 'win', t, f), clean(fs.readFileSync(path.join(W, t, f), 'utf8')));
  }
  fs.cpSync(path.join(W, 'assets'), path.join(out, 'win', 'assets'), { recursive: true });
  const thumbs = rel('.cache', 'win-thumbs');
  if (!fs.existsSync(thumbs)) throw new Error('Windows thumbnails missing: run node scripts/render-screens.mjs');
  fs.cpSync(thumbs, path.join(out, 'win', 'thumbs'), { recursive: true });
  for (const t of ['light', 'dark']) {
    fs.cpSync(path.join(M, t, 'assets'), path.join(out, 'mac', t, 'assets'), { recursive: true });
    for (const f of fs.readdirSync(path.join(M, t)).filter((f) => f.endsWith('.html'))) {
      // The Mac pages carry a viewer bar above the window; the embedded copy shows the window alone.
      const s = clean(fs.readFileSync(path.join(M, t, f), 'utf8')).replace('</head>', `${MAC_EMBED}</head>`);
      fs.writeFileSync(path.join(out, 'mac', t, f), s);
    }
  }
}

// ---------- pages
const missing = new Set();
const ctx = { html, raw, esc, md, ICON, MARK, FEATURES, FEATURE_GROUPS, PARTS, OS, COMPANY, facts, cat, manual, SITE_URL, ...makeHelpers(cat, missing) };
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

// ---------- tutorial to screen map (machine-readable, also kept in docs/)
const map = { tutorials: [], screens: {} };
for (const plat of ['win', 'mac']) {
  for (const a of manual[plat]) for (const w of a.workflows) {
    map.tutorials.push({ id: w.id, platform: plat, area: a.area, title: w.title, url: `/${OS[plat].slug}/tutorials/${w.id}/`, screens: w.steps.map((s) => s.screen) });
    for (const s of w.steps) { const k = `${plat}/${s.screen}`; map.screens[k] ||= []; if (!map.screens[k].includes(w.id)) map.screens[k].push(w.id); }
  }
}
fs.writeFileSync(path.join(DIST, 'tutorial-screen-map.json'), JSON.stringify(map, null, 1));
fs.writeFileSync(rel('docs', 'tutorial-screen-map.json'), JSON.stringify(map, null, 1));

// ---------- SEO and hosting files
const indexable = pages.filter((p) => !p.noindex);
const today = new Date().toISOString().slice(0, 10);
fs.writeFileSync(path.join(DIST, 'sitemap.xml'), `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${indexable.map((p) => `<url><loc>${esc(SITE_URL + p.path)}</loc><lastmod>${today}</lastmod></url>`).join('\n')}\n</urlset>\n`);
fs.writeFileSync(path.join(DIST, 'robots.txt'), `User-agent: *\nAllow: /\nDisallow: /mockups/\n\nSitemap: ${SITE_URL}/sitemap.xml\n`);
// ---------- standard site files
const webmanifest = { name: SITE_NAME, short_name: 'OpenFrame', description: 'Film production software for ideas, scripts, schedules and call sheets.', start_url: '/', scope: '/', display: 'browser',
  background_color: '#0e0d15', theme_color: '#1d1a2e', lang: 'en',
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
Technical Founding Engineer: Saravan Kumar M K
Product Founding Engineer: Badrinath Reddy A

/* SITE */
Last update: ${today}
Standards: HTML5, CSS3
Language: English
Store: ${SITE_URL}/
`);
const n = (plat) => manual[plat].reduce((k, a) => k + a.workflows.length, 0);
fs.writeFileSync(path.join(DIST, 'llms.txt'), `# ${SITE_NAME}

> ${SITE_NAME} is a local-first desktop app for writers, screenwriters and film production teams. One app covers ideas (Idea Vault), story (Story page, Beat Board, characters), the screenplay editor, script breakdown, cast, crew and locations, shot lists, storyboards, moodboards, the shooting schedule, call sheets, sides, reports and a light budget. Projects are stored on the user's computer.

Facts:
- Platforms: Windows 11 (x64) and macOS 14 or later on Apple silicon Macs. Distributed through the Microsoft Store and the Mac App Store.
- Price: one-time lifetime purchase, ₹2,999 in India and US$89 elsewhere, after a 7-day free trial. The store sets the final checkout amount.
- Privacy: no OpenFrame account, no telemetry, no cloud service. Works offline. Projects are not encrypted by the app.
- OpenFrame AI: optional. One-time download of about 1.2 GB that the person starts; runs on the computer; suggests changes that apply only when the person clicks Apply.
- Import: Final Draft (.fdx), Fountain, Word (.docx), PDF (best effort), plain text. Screenplay export: PDF, FDX, Fountain, DOCX, TXT.

## Pages
- [Features](${SITE_URL}/features/)
- [Windows tutorials](${SITE_URL}/windows/tutorials/): step-by-step tutorials on the real Windows screens.
- [Mac tutorials](${SITE_URL}/mac/tutorials/): step-by-step tutorials on the real Mac screens.
- [Complete feature catalog and tutorial list](${SITE_URL}/llms-full.txt)
- [Pricing](${SITE_URL}/pricing/), [FAQ](${SITE_URL}/faq/), [About](${SITE_URL}/about/).
`);

// llms-full.txt: the whole feature catalog and every tutorial, as plain text for answer engines.
const full = [`# ${SITE_NAME}: complete feature catalog and tutorials`, '', `Platforms: Windows 11 and macOS 14 or later on Apple silicon. Price: one-time purchase, ₹2,999 in India and US$89 elsewhere, after a 7-day free trial.`, ''];
for (const [, items] of FEATURE_GROUPS) for (const [slug, title] of items) {
  full.push(`## ${title} (${SITE_URL}/features/${slug}/)`, '');
  for (const f of CATALOG[slug] || []) full.push(`- ${f.name} [${[f.win?.length && 'Windows', (f.mac?.length || f.onMac) && 'Mac'].filter(Boolean).join(', ')}]: ${f.does} ${f.purpose}`);
  full.push('');
}
for (const plat of ['win', 'mac']) {
  full.push(`## ${OS[plat].name} tutorials (${SITE_URL}/${OS[plat].slug}/tutorials/)`, '');
  for (const a of manual[plat]) for (const w of a.workflows) full.push(`- [${w.title}](${SITE_URL}/${OS[plat].slug}/tutorials/${w.id}/): ${w.purpose}`);
  full.push('');
}
fs.writeFileSync(path.join(DIST, 'llms-full.txt'), full.join(String.fromCharCode(10)));

const bootHash = crypto.createHash('sha256').update(BOOT).digest('base64');
const CSP = ["default-src 'self'", "script-src 'self' 'sha256-" + bootHash + "'", "style-src 'self'", "img-src 'self' data:", "font-src 'self'",
  "connect-src 'self'", "frame-src 'self'", "object-src 'none'", "base-uri 'self'", "form-action 'none'", "frame-ancestors 'none'", 'upgrade-insecure-requests'].join('; ');
// The app screens are static pages with their own inline styles and no script; only this site may frame them.
const MOCK_CSP = ["default-src 'none'", "style-src 'self' 'unsafe-inline'", "img-src 'self' data:", "font-src 'self'", "base-uri 'none'", "form-action 'none'", "frame-ancestors 'self'"].join('; ');
fs.writeFileSync(path.join(DIST, '_headers'), `/*
  Content-Security-Policy: ${CSP}
  X-Content-Type-Options: nosniff
  X-Frame-Options: DENY
  Referrer-Policy: strict-origin-when-cross-origin
  Permissions-Policy: camera=(), microphone=(), geolocation=(), payment=(), usb=()
  Cross-Origin-Opener-Policy: same-origin
  Strict-Transport-Security: max-age=31536000; includeSubDomains
/mockups/*
  ! Content-Security-Policy
  ! X-Frame-Options
  Content-Security-Policy: ${MOCK_CSP}
  X-Frame-Options: SAMEORIGIN
  X-Robots-Tag: noindex
  Cache-Control: public, max-age=604800
/assets/*
  Cache-Control: public, max-age=31536000, immutable
/403.html
  X-Robots-Tag: noindex
/404.html
  X-Robots-Tag: noindex
/500.html
  X-Robots-Tag: noindex
/503.html
  X-Robots-Tag: noindex
  Retry-After: 300
/site.webmanifest
  Content-Type: application/manifest+json; charset=utf-8
  Cache-Control: public, max-age=86400
/.well-known/security.txt
  Content-Type: text/plain; charset=utf-8
  Cache-Control: public, max-age=86400
/favicon.ico
  Cache-Control: public, max-age=604800
/*.png
  Cache-Control: public, max-age=604800
/tutorial-screen-map.json
  X-Robots-Tag: noindex
`);
// Old URLs keep working: the design package's .html pages and the earlier tutorial ids.
const OLD = { 'index.html': '/', 'pricing.html': '/pricing/', 'download.html': '/download/', 'tutorials.html': '/tutorials/', 'faq.html': '/faq/', 'migrate.html': '/switch/',
  'compare.html': '/compare/', 'why-local.html': '/local-and-private/', 'themes.html': '/themes/', 'about.html': '/about/', 'contact.html': '/contact/',
  'privacy.html': '/legal/privacy/', 'terms.html': '/legal/terms/', 'features/index.html': '/features/' };
const OLD_TUTORIALS = ['ai-assistant-ask', 'ai-assistant-setup', 'backup-restore', 'breakdown', 'budget', 'call-sheet', 'cast-crew', 'characters', 'create-project', 'drafts-revisions', 'export-script', 'files', 'first-launch', 'idea-vault', 'import-script', 'locations', 'manage-projects', 'moodboard', 'pages-templates', 'project-tour', 'recently-deleted', 'reports', 'schedule', 'screenplay-notes', 'settings', 'shot-list', 'sides', 'story-board', 'storyboard', 'tasks', 'themes-appearance', 'write-screenplay'];
const redirects = ['/features/ai-assistant/ /features/openframe-ai/ 301', '/features/ai-assistant.html /features/openframe-ai/ 301'].concat(Object.entries(OLD).map(([a, b]) => `/${a} ${b} 301`)).concat(FEATURES.map((f) => `/features/${f.slug}.html /features/${f.slug}/ 301`));
for (const plat of ['win', 'mac']) for (const id of OLD_TUTORIALS) {
  const p = `/${OS[plat].slug}/tutorials/${id}/`;
  if (!paths.has(p)) redirects.push(`${p} /${OS[plat].slug}/tutorials/ 302`);
}
// The per-platform screen lists were folded into the tutorials (owner, 2026-10-31).
redirects.push('/windows/screens/ /windows/tutorials/ 301', '/mac/screens/ /mac/tutorials/ 301');
fs.writeFileSync(path.join(DIST, '_redirects'), redirects.join('\n') + '\n');

const files = fs.readdirSync(DIST, { recursive: true }).length;
console.log(`built ${pages.length} pages (${n('win')} Windows and ${n('mac')} Mac tutorials, ${files} files) in ${Date.now() - t0} ms -> dist/`);
