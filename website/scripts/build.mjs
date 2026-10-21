// Static build: src/ -> dist/. Run: npm run build. Pages come from src/pages/*.mjs (see docs/CONTRACT.md).
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { pathToFileURL } from 'node:url';
import * as esbuild from 'esbuild';
import { SITE_URL, SITE_NAME } from '../site.config.mjs';
import { html, raw, esc, md } from '../src/lib/html.mjs';
import { ICON, MARK } from '../src/lib/icons.mjs';
import { FEATURES, FEATURE_GROUPS, TUTORIAL_PATHS, OS } from '../src/content/nav.mjs';
import { renderPage, makeHelpers, OS_BOOT } from '../src/lib/layout.mjs';

const ROOT = path.resolve(import.meta.dirname, '..');
const SRC = path.join(ROOT, 'src'), DIST = path.join(ROOT, 'dist');
const rel = (...p) => path.join(ROOT, ...p);
const t0 = Date.now();

fs.rmSync(DIST, { recursive: true, force: true });
fs.mkdirSync(DIST, { recursive: true });

// ---------- data
const facts = JSON.parse(fs.readFileSync(rel('research', 'facts.json'), 'utf8'));
const manifestPath = rel('screens', 'manifest.json');
const screens = fs.existsSync(manifestPath) ? JSON.parse(fs.readFileSync(manifestPath, 'utf8')) : {};
delete screens._generated;
const tutorials = { win: {}, mac: {} };
for (const plat of ['win', 'mac']) {
  const dir = path.join(SRC, 'content', 'tutorials', plat);
  if (!fs.existsSync(dir)) continue;
  for (const f of fs.readdirSync(dir).filter((f) => f.endsWith('.mjs')).sort()) {
    const m = (await import(pathToFileURL(path.join(dir, f)).href)).default;
    tutorials[plat][m.id] = m;
  }
}

// ---------- assets (content-hashed names)
const hash = (buf) => crypto.createHash('sha256').update(buf).digest('hex').slice(0, 10);
const cssFiles = ['site.css', 'player.css', ...fs.readdirSync(path.join(SRC, 'assets', 'css')).filter((f) => f.startsWith('pages-')).sort()]
  .map((f) => path.join(SRC, 'assets', 'css', f)).filter((f) => fs.existsSync(f));
const cssOut = (await esbuild.transform(cssFiles.map((f) => fs.readFileSync(f, 'utf8')).join('\n'), { loader: 'css', minify: true })).code;
const jsFiles = ['site.js', 'player.js'].map((f) => path.join(SRC, 'assets', 'js', f)).filter((f) => fs.existsSync(f));
const jsOut = (await esbuild.transform(jsFiles.map((f) => fs.readFileSync(f, 'utf8')).join(';\n'), { loader: 'js', minify: true, target: 'es2019' })).code;
fs.mkdirSync(path.join(DIST, 'assets', 'fonts'), { recursive: true });
const cssName = `/assets/site.${hash(cssOut)}.css`, jsName = `/assets/site.${hash(jsOut)}.js`;
fs.writeFileSync(path.join(DIST, cssName), cssOut);
fs.writeFileSync(path.join(DIST, jsName), jsOut);
for (const f of fs.readdirSync(path.join(SRC, 'assets', 'fonts'))) fs.copyFileSync(path.join(SRC, 'assets', 'fonts', f), path.join(DIST, 'assets', 'fonts', f));
fs.cpSync(path.join(SRC, 'static'), DIST, { recursive: true });
if (fs.existsSync(rel('screens'))) fs.cpSync(rel('screens'), path.join(DIST, 'screens'), { recursive: true, filter: (p) => !p.endsWith('manifest.json') });

// ---------- pages
const missing = new Set();
const ctx = { html, raw, esc, md, ICON, MARK, FEATURES, FEATURE_GROUPS, TUTORIAL_PATHS, OS, facts, screens, tutorials, SITE_URL, ...makeHelpers(screens, missing) };
const pages = [];
for (const f of fs.readdirSync(path.join(SRC, 'pages')).filter((f) => f.endsWith('.mjs') && !f.startsWith('_')).sort()) {
  const mod = await import(pathToFileURL(path.join(SRC, 'pages', f)).href);
  for (const p of mod.default(ctx)) pages.push({ ...p, _from: f });
}
pages.push({
  path: '/404.html', title: 'Page not found | OpenFrame Studio', description: 'This page does not exist. Find OpenFrame Studio features, tutorials for Windows and macOS, pricing plans and answers to common questions.',
  noindex: true, nav: null,
  body: html`<section class="section hero center"><div class="narrow"><h1>This page is not here.</h1><p class="lead">The link may be old, or the address may have a typo. These pages will get you back on track.</p>
<div class="btn-row center"><a class="btn btn-primary btn-lg" href="/">Go to the home page</a><a class="btn btn-ghost btn-lg" href="/tutorials/">Browse tutorials</a><a class="btn btn-ghost btn-lg" href="/features/">See all features</a></div></div></section>`,
});

const paths = new Set();
for (const p of pages) {
  if (!/^\/([a-z0-9-]+\/)*$|^\/404\.html$/.test(p.path)) throw new Error(`bad path ${p.path} from ${p._from}`);
  if (paths.has(p.path)) throw new Error(`duplicate path ${p.path} from ${p._from}`);
  paths.add(p.path);
}

// Header switch targets: counterpart page, else the same section of the other platform, else its landing page.
function switchHref(p) {
  const out = { win: '/windows/', mac: '/mac/' };
  if (!p.os) return out;
  out[p.os] = p.path;
  const other = p.os === 'win' ? 'mac' : 'win', from = OS[p.os].slug, to = OS[other].slug;
  if (p.alternates?.[other]) { out[other] = p.alternates[other]; return out; }
  const m = p.path.match(/^\/(windows|mac)\/tutorials\/([a-z0-9-]+)\/$/);
  if (m) { out[other] = `/${to}/tutorials/?missing=${m[2]}`; return out; }
  const swapped = p.path.replace(`/${from}/`, `/${to}/`);
  out[other] = paths.has(swapped) ? swapped : `/${to}/`;
  return out;
}

const DASH = /[–—]/;
for (const p of pages) {
  const out = renderPage({ ...p, body: String(p.body) }, { css: cssName, js: jsName, switchHref: switchHref(p) });
  const text = out.replace(/<script[\s\S]*?<\/script>/g, '');
  if (DASH.test(text)) throw new Error(`em or en dash in ${p.path} (${p._from})`);
  const file = p.path === '/404.html' ? path.join(DIST, '404.html') : path.join(DIST, p.path, 'index.html');
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, out);
}
if (missing.size) {
  const list = [...missing].sort().join(', ');
  if (process.env.OF_ALLOW_MISSING) console.warn(`WARNING screens not captured: ${list}`);
  else throw new Error(`screens not in screens/manifest.json (run node scripts/capture-screens.mjs): ${list}`);
}

// ---------- tutorial to screen map (machine-readable, also kept in docs/)
const map = { generated: new Date().toISOString(), tutorials: [], screens: {} };
for (const plat of ['win', 'mac']) {
  for (const t of Object.values(tutorials[plat])) {
    if (t.available === false) { map.tutorials.push({ id: t.id, platform: plat, available: false, reason: t.reason || '' }); continue; }
    const url = `/${OS[plat].slug}/tutorials/${t.id}/`;
    map.tutorials.push({ id: t.id, platform: plat, available: true, feature: t.feature || null, path: t.path, title: t.title, url,
      screens: t.steps.map((s) => s.screen), steps: t.steps.map((s) => ({ screen: s.screen, action: s.action, target: s.target, label: s.label })) });
    for (const s of t.steps) {
      const k = `${plat}/${s.screen}`;
      map.screens[k] ||= [];
      if (!map.screens[k].includes(`${plat}:${t.id}`)) map.screens[k].push(`${plat}:${t.id}`);
    }
  }
}
fs.writeFileSync(path.join(DIST, 'tutorial-screen-map.json'), JSON.stringify(map, null, 1));
fs.mkdirSync(rel('docs'), { recursive: true });
fs.writeFileSync(rel('docs', 'tutorial-screen-map.json'), JSON.stringify({ ...map, generated: undefined }, null, 1));

// ---------- SEO and hosting files
const indexable = pages.filter((p) => !p.noindex && p.path !== '/404.html');
const today = new Date().toISOString().slice(0, 10);
fs.writeFileSync(path.join(DIST, 'sitemap.xml'), `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${indexable.map((p) => `<url><loc>${esc(SITE_URL + p.path)}</loc><lastmod>${today}</lastmod></url>`).join('\n')}\n</urlset>\n`);
fs.writeFileSync(path.join(DIST, 'robots.txt'), `User-agent: *\nAllow: /\n\nSitemap: ${SITE_URL}/sitemap.xml\n`);

const tutCount = (plat) => Object.values(tutorials[plat]).filter((t) => t.available !== false).length;
fs.writeFileSync(path.join(DIST, 'llms.txt'), `# ${SITE_NAME}

> ${SITE_NAME} is a local-first desktop app for writers, screenwriters and film production teams. One app covers ideas (Idea Vault), story (Beat Board, characters), the screenplay editor, script breakdown, cast, crew and locations, shot lists, storyboards, moodboards, the shooting schedule, call sheets, sides, reports and a simple budget. Projects are stored on the user's computer.

Facts:
- Platforms: Windows 11 (x64) and macOS 14 or later on Apple silicon Macs. One shared core, a native app on each platform.
- Release status: in internal team testing. There is no public download yet. Public release is planned on the Microsoft Store and the Mac App Store.
- Price model (planned, not on sale): one-time purchase. Target prices 3,000 rupees, 89 US dollars, 89 euros. A 7-day trial is planned with the store release.
- Privacy: no account, no telemetry, no cloud service. Works offline. Projects are not encrypted by the app.
- Offline assistant: optional. One-time download of about 1.2 GB that the person starts; runs on the computer afterwards; suggests changes that apply only when the person clicks Apply.
- Import: Final Draft (.fdx), Fountain, Word (.docx), PDF (best effort), plain text. Screenplay export: PDF, FDX, Fountain, DOCX, TXT.
- Collaboration is file-based (review packages, backups, project packages). There is no real-time collaboration.
- App screens are in English. Screenplay PDFs support Latin text and Devanagari, Telugu, Tamil, Kannada and Malayalam scripts.

## Pages
- [Features](${SITE_URL}/features/): every part of the app.
- [Windows](${SITE_URL}/windows/): requirements, release status, shortcuts.
- [macOS](${SITE_URL}/mac/): requirements, release status, shortcuts.
- [Windows tutorials](${SITE_URL}/windows/tutorials/): ${tutCount('win')} step-by-step tutorials on real Windows screens.
- [macOS tutorials](${SITE_URL}/mac/tutorials/): ${tutCount('mac')} step-by-step tutorials on real Mac screens.
- [Pricing](${SITE_URL}/pricing/), [FAQ](${SITE_URL}/faq/), [Local and private](${SITE_URL}/local-and-private/), [Switch from other software](${SITE_URL}/switch/).
`);

const bootHash = crypto.createHash('sha256').update(OS_BOOT).digest('base64');
const CSP = ["default-src 'self'", "script-src 'self' 'sha256-" + bootHash + "'", "style-src 'self'", "img-src 'self' data:", "font-src 'self'",
  "connect-src 'self'", "object-src 'none'", "base-uri 'self'", "form-action 'none'", "frame-ancestors 'none'", 'upgrade-insecure-requests'].join('; ');
fs.writeFileSync(path.join(DIST, '_headers'), `/*
  Content-Security-Policy: ${CSP}
  X-Content-Type-Options: nosniff
  X-Frame-Options: DENY
  Referrer-Policy: strict-origin-when-cross-origin
  Permissions-Policy: camera=(), microphone=(), geolocation=(), payment=(), usb=()
  Cross-Origin-Opener-Policy: same-origin
  Strict-Transport-Security: max-age=31536000; includeSubDomains
/assets/*
  Cache-Control: public, max-age=31536000, immutable
/screens/*
  Cache-Control: public, max-age=604800
/tutorial-screen-map.json
  X-Robots-Tag: noindex
`);
// Old URLs from the website mock keep working.
const OLD = { 'index.html': '/', 'pricing.html': '/pricing/', 'download.html': '/download/', 'tutorials.html': '/tutorials/', 'faq.html': '/faq/', 'migrate.html': '/switch/',
  'compare.html': '/compare/', 'why-local.html': '/local-and-private/', 'themes.html': '/themes/', 'about.html': '/about/', 'contact.html': '/contact/',
  'privacy.html': '/legal/privacy/', 'terms.html': '/legal/terms/', 'features/index.html': '/features/' };
const redirects = Object.entries(OLD).map(([a, b]) => `/${a} ${b} 301`).concat(FEATURES.map((f) => `/features/${f.slug}.html /features/${f.slug}/ 301`));
fs.writeFileSync(path.join(DIST, '_redirects'), redirects.join('\n') + '\n');

console.log(`built ${pages.length} pages (${tutCount('win')} Windows and ${tutCount('mac')} macOS tutorials) in ${Date.now() - t0} ms -> dist/`);
