// The files search engines, answer engines and the host read: sitemap.xml (with image entries and honest lastmod),
// robots.txt, llms.txt, llms-full.txt and _headers. Called from build.mjs. Documented in docs/SEO.md and docs/SECURITY.md.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { bodyImages, sitemapXml } from '../src/lib/seo.mjs';
import { SITE_URL, SITE_NAME } from '../site.config.mjs';

const ROOT = path.resolve(import.meta.dirname, '..');
const LASTMOD = path.join(ROOT, 'src', 'content', 'lastmod.json');
const write = (DIST, f, s) => fs.writeFileSync(path.join(DIST, f), s);

// lastmod is the day a page's own content (title, description, body) last changed, not the build day: search engines
// ignore lastmod once it proves unreliable. The hashes live in src/content/lastmod.json and are committed.
function lastmods(pages) {
  let old = {};
  try { old = JSON.parse(fs.readFileSync(LASTMOD, 'utf8')); } catch { /* first run or a half-written file: dates restart today */ }
  const today = new Date().toISOString().slice(0, 10);
  const next = {};
  for (const p of pages) {
    const h = crypto.createHash('sha256').update(`${p.title}\n${p.description}\n${p.body}`).digest('hex').slice(0, 12);
    next[p.path] = old[p.path]?.h === h ? old[p.path] : { h, d: today };
  }
  if (JSON.stringify(old) !== JSON.stringify(next)) { fs.writeFileSync(LASTMOD + '.tmp', JSON.stringify(next, null, 1) + '\n'); fs.renameSync(LASTMOD + '.tmp', LASTMOD); }
  return next;
}

export function writeSitemapAndRobots(DIST, pages) {
  const indexable = pages.filter((p) => !p.noindex);
  const lm = lastmods(indexable);
  write(DIST, 'sitemap.xml', sitemapXml(indexable.map((p) => ({ loc: SITE_URL + p.path, lastmod: lm[p.path].d, images: bodyImages(p.body) }))));
  // Files that are for the site's own scripts, not for search results, stay out of crawls; so do the app screen pages
  // (they are shown in frames on the site's pages, and no page of their own is meant to rank).
  const hidden = ['/search-index.json', '/mockups/'].filter((f) => fs.existsSync(path.join(DIST, f)));
  write(DIST, 'robots.txt', `User-agent: *\nAllow: /\n${hidden.map((f) => `Disallow: ${f}\n`).join('')}\nSitemap: ${SITE_URL}/sitemap.xml\n`);
}

export function writeLlms(DIST, { paths, manual, FEATURE_GROUPS, CATALOG, OS }) {
  const link = (p, label, note) => (paths.has(p) ? `- [${label}](${SITE_URL}${p})${note ? `: ${note}` : ''}\n` : '');
  write(DIST, 'llms.txt', `# ${SITE_NAME}

> ${SITE_NAME} is a local-first desktop app for writers, screenwriters and film production teams. One app covers ideas (Idea Vault), story (Story page, Beat Board, characters), the screenplay editor, script breakdown, cast, crew and locations, shot lists, storyboards, moodboards, the shooting schedule, call sheets, sides, reports and a light budget. Projects are stored on the user's computer.

Facts:
- Platforms: Windows 11 (64-bit) and macOS 14 or later on Apple silicon Macs. Sold through the Microsoft Store and the Mac App Store.
- Price: one-time lifetime purchase, ₹2,999 in India, €89 in Europe and US$89 in other countries, after a 7-day free trial. The store sets the final checkout amount.
- Privacy: no OpenFrame account, no telemetry, no cloud service. Works offline. Projects are not encrypted by the app.
- OpenFrame AI: optional. One-time download of about 1.2 GB that the person starts; runs on the computer; suggests changes that apply only when the person clicks Apply.
- Import: Final Draft (.fdx), Fountain, Word (.docx), PDF (best effort), plain text. Screenplay export: PDF, FDX, Fountain, DOCX, TXT.
- Collaboration: no live co-editing and no cloud sharing. Teams pass project files and review packages between computers.

## Pages
${link('/features/', 'Features', 'every capability, marked for Windows and Mac.')}${link('/pricing/', 'Pricing', 'the one-time price and the 7-day trial.')}${link('/download/', 'Download')}${link('/faq/', 'FAQ', 'platforms, privacy, files, OpenFrame AI and the price.')}${link('/local-and-private/', 'Local and private', 'where files live and what the app never sends.')}${link('/switch/', 'Switch to OpenFrame Studio', 'which script formats come in and go out.')}${link('/compare/', 'Compare')}${link('/themes/', 'Themes')}${link('/about/', 'About')}${link('/windows/tutorials/', 'Windows tutorials', 'step-by-step tutorials on the real Windows screens.')}${link('/mac/tutorials/', 'Mac tutorials', 'step-by-step tutorials on the real Mac screens.')}
## Optional
- [Complete feature catalog and tutorial list](${SITE_URL}/llms-full.txt)
`);
  // llms-full.txt: the whole feature catalog and every tutorial, as plain text for answer engines.
  const full = [`# ${SITE_NAME}: complete feature catalog and tutorials`, '', `Platforms: Windows 11 and macOS 14 or later on Apple silicon. Price: one-time purchase, ₹2,999 in India, €89 in Europe and US$89 in other countries, after a 7-day free trial.`, ''];
  for (const [, items] of FEATURE_GROUPS) for (const [slug, title] of items) {
    full.push(`## ${title} (${SITE_URL}/features/${slug}/)`, '');
    for (const f of CATALOG[slug] || []) full.push(`- ${f.name} [${[f.win?.length && 'Windows', f.mac?.length && 'Mac'].filter(Boolean).join(', ')}]: ${f.does} ${f.purpose}`);
    full.push('');
  }
  for (const plat of ['win', 'mac']) {
    full.push(`## ${OS[plat].name} tutorials (${SITE_URL}/${OS[plat].slug}/tutorials/)`, '');
    for (const a of manual[plat]) for (const w of a.workflows) full.push(`- [${w.title}](${SITE_URL}/${OS[plat].slug}/tutorials/${w.id}/): ${w.purpose}`);
    full.push('');
  }
  write(DIST, 'llms-full.txt', full.join('\n'));
}

// ---------- _headers (Cloudflare Pages). Limits: 100 rules, 2,000 characters per line.
export function headersFile(BOOT) {
  const bootHash = crypto.createHash('sha256').update(BOOT).digest('base64');
  // Cloudflare Web Analytics is off until the owner switches it on in the dashboard; its script and beacon hosts are allowed now.
  const CSP = ["default-src 'self'", `script-src 'self' 'sha256-${bootHash}' https://static.cloudflareinsights.com`, "style-src 'self'", "img-src 'self' data:", "font-src 'self'",
    "connect-src 'self' https://cloudflareinsights.com", "manifest-src 'self'", "frame-src 'self'", "object-src 'none'", "worker-src 'none'", "base-uri 'self'", "form-action 'none'", "frame-ancestors 'none'", 'upgrade-insecure-requests'].join('; ');
  // The app screens are static pages with their own inline styles and no script. Only this site may frame them.
  const MOCK_CSP = ["default-src 'none'", "style-src 'self' 'unsafe-inline'", "img-src 'self' data:", "font-src 'self' data:", "base-uri 'none'", "form-action 'none'", "frame-ancestors 'self'"].join('; ');
  const PERMISSIONS = ['accelerometer', 'bluetooth', 'browsing-topics', 'camera', 'clipboard-read', 'display-capture', 'geolocation', 'gyroscope', 'hid', 'idle-detection', 'magnetometer', 'microphone', 'midi', 'payment', 'screen-wake-lock', 'serial', 'usb'].map((f) => `${f}=()`).join(', ');
  return `/*
  Content-Security-Policy: ${CSP}
  X-Content-Type-Options: nosniff
  X-Frame-Options: DENY
  Referrer-Policy: strict-origin-when-cross-origin
  Permissions-Policy: ${PERMISSIONS}
  Cross-Origin-Opener-Policy: same-origin
  Cross-Origin-Resource-Policy: same-origin
  X-Permitted-Cross-Domain-Policies: none
  Strict-Transport-Security: max-age=31536000
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
/llms.txt
  Content-Type: text/plain; charset=utf-8
/llms-full.txt
  Content-Type: text/plain; charset=utf-8
/humans.txt
  Content-Type: text/plain; charset=utf-8
/favicon.ico
  Cache-Control: public, max-age=604800
/*.png
  Cache-Control: public, max-age=604800
/og.png
  ! Cross-Origin-Resource-Policy
  Cross-Origin-Resource-Policy: cross-origin
/search-index.json
  X-Robots-Tag: noindex
  Cache-Control: public, max-age=3600
`;
}
