// Head tags, structured data and sitemap helpers. Everything here is derived from the page objects, so a new page
// gets correct meta tags, Open Graph, Twitter/X cards, robots rules and JSON-LD without extra work.
import { esc } from './html.mjs';
import { FOUNDERS } from '../content/founders.mjs';
import { SUPPORT_EMAIL } from '../content/nav.mjs';
import { SAME_AS } from '../content/social.mjs';
import { SITE_URL, SITE_NAME, STORE } from '../../site.config.mjs';

export const OG = { path: '/og.png', width: 1200, height: 630, type: 'image/png',
  alt: 'Straft Studio logo, the headline "A film production software for all your needs" and the app Home screen.' };
const ID = { org: `${SITE_URL}/#organization`, site: `${SITE_URL}/#website`, app: `${SITE_URL}/#software` };
const DESC = 'A local-first desktop app for writers, screenwriters and film production teams: ideas, story, screenplay, breakdown, schedule, call sheets and reports in one project.';

// ---------- structured data builders (info.mjs re-exports ORG and APP for the pages)
export const ORG = (ctx = { SITE_URL }) => ({ '@context': 'https://schema.org', '@type': 'Organization', '@id': ID.org, name: SITE_NAME, url: ctx.SITE_URL + '/',
  logo: { '@type': 'ImageObject', url: ctx.SITE_URL + '/icon-512.png', width: 512, height: 512 },
  slogan: 'From film lovers, to film lovers across the world.', email: SUPPORT_EMAIL, sameAs: SAME_AS,
  contactPoint: { '@type': 'ContactPoint', contactType: 'customer support', email: SUPPORT_EMAIL, availableLanguage: 'English' },
  founder: FOUNDERS.map((f) => ({ '@type': 'Person', name: f.name, jobTitle: f.role, description: f.text[0], ...(f.link ? { url: f.link, sameAs: [f.link] } : {}) })) });
// Availability is stated only once a store listing exists (site.config.mjs STORE); until then no claim is made that it can be bought.
const AVAIL = STORE.win || STORE.mac ? { availability: 'https://schema.org/InStock' } : {};
// India pays in rupees, Europe in euros, the USA and other countries in US dollars; all one-time launch prices. No priceValidUntil: no end date is set.
const OFFERS = (url) => [
  { '@type': 'Offer', price: '3999', priceCurrency: 'INR', eligibleRegion: { '@type': 'Country', name: 'IN' }, url, ...AVAIL, description: 'One-time purchase at the launch price after a 7-day free trial. India.' },
  { '@type': 'Offer', price: '89', priceCurrency: 'EUR', eligibleRegion: { '@type': 'Place', name: 'Europe' }, url, ...AVAIL, description: 'One-time purchase at the launch price after a 7-day free trial. Europe.' },
  { '@type': 'Offer', price: '89', priceCurrency: 'USD', ineligibleRegion: [{ '@type': 'Country', name: 'IN' }, { '@type': 'Place', name: 'Europe' }], url, ...AVAIL, description: 'One-time purchase at the launch price after a 7-day free trial. USA and other countries.' }];
export const APP = (ctx = { SITE_URL }, os, p = '/') => ({ '@context': 'https://schema.org', '@type': 'SoftwareApplication', '@id': ID.app, name: SITE_NAME, url: ctx.SITE_URL + p,
  applicationCategory: 'MultimediaApplication', applicationSubCategory: 'Screenwriting and film production planning', operatingSystem: os,
  description: DESC, image: ctx.SITE_URL + OG.path, inLanguage: 'en',
  publisher: { '@type': 'Organization', '@id': ID.org, name: SITE_NAME }, offers: OFFERS(ctx.SITE_URL + '/pricing/') });
export const WEBSITE = (ctx = { SITE_URL }) => ({ '@context': 'https://schema.org', '@type': 'WebSite', '@id': ID.site, name: SITE_NAME, alternateName: 'Straft', url: ctx.SITE_URL + '/', inLanguage: 'en',
  publisher: { '@type': 'Organization', '@id': ID.org, name: SITE_NAME },
  potentialAction: { '@type': 'SearchAction', target: { '@type': 'EntryPoint', urlTemplate: ctx.SITE_URL + '/search/?q={search_term_string}' }, 'query-input': 'required name=search_term_string' } });

// ---------- rules that keep JSON-LD honest: it may only say what the page shows
function matchVisible(o, body) {
  const t = String(body);
  const x = { ...o };
  if (x['@type'] === 'SoftwareApplication' && !/₹3,999/.test(t)) delete x.offers; // price not shown on this page
  if (x['@type'] === 'Organization' && !t.includes(FOUNDERS[0].name)) delete x.founder; // founders only where they are named
  return x;
}
// Page-level structured data: what the page declares, plus the price offer on /pricing/ and a BreadcrumbList on deep pages.
export function jsonLd(page) {
  const ld = [...(page.jsonld || [])];
  if (page.path === '/pricing/' && !ld.some((o) => o['@type'] === 'SoftwareApplication')) ld.push(APP({ SITE_URL }, 'Windows 11, macOS 14 or later', '/pricing/'));
  if (page.breadcrumbs?.length) {
    const crumbs = [['Home', '/'], ...page.breadcrumbs];
    ld.push({ '@context': 'https://schema.org', '@type': 'BreadcrumbList', itemListElement: crumbs.map(([name, p], i) => ({ '@type': 'ListItem', position: i + 1, name, item: SITE_URL + p })) });
  }
  return ld.map((o) => matchVisible(o, page.body));
}

// ---------- titles and descriptions
/** Cut at a word boundary, never mid-word and never with a dash or ellipsis. */
export function trimWords(s, max) {
  if (s.length <= max) return s;
  const cut = s.slice(0, max + 1).replace(/\s+\S*$/, '').replace(/[\s,;:]+$/, '');
  return cut;
}
/** Add the brand when the title has none and the result still fits the 60 characters search results show. */
export const fitTitle = (t) => (/Straft/.test(t) || t.length + SITE_NAME.length + 3 > 60 ? t : `${t} | ${SITE_NAME}`);

// ---------- <head> tags
export function headMeta(page) {
  const url = SITE_URL + page.path;
  const title = fitTitle(page.title);
  const ogUrl = page.ogImage ? (page.ogImage.startsWith('http') ? page.ogImage : SITE_URL + page.ogImage) : SITE_URL + OG.path;
  const w = page.ogImage ? page.ogWidth || OG.width : OG.width, h = page.ogImage ? page.ogHeight || OG.height : OG.height;
  const alt = page.ogAlt || OG.alt;
  const robots = page.noindex ? 'noindex' : 'index, follow, max-image-preview:large, max-snippet:-1, max-video-preview:-1';
  const m = (k, v, attr = 'property') => `<meta ${attr}="${k}" content="${esc(v)}">`;
  return [
    `<title>${esc(title)}</title>`,
    m('description', page.description, 'name'),
    `<link rel="canonical" href="${esc(url)}">`,
    m('robots', robots, 'name'),
    m('og:type', 'website'), m('og:site_name', SITE_NAME), m('og:title', page.ogTitle || title), m('og:description', page.description), m('og:url', url),
    m('og:image', ogUrl), m('og:image:type', page.ogImage ? page.ogType || 'image/png' : OG.type), m('og:image:width', w), m('og:image:height', h), m('og:image:alt', alt), m('og:locale', 'en_US'),
    m('twitter:card', 'summary_large_image', 'name'), m('twitter:title', page.ogTitle || title, 'name'), m('twitter:description', page.description, 'name'),
    m('twitter:image', ogUrl, 'name'), m('twitter:image:alt', alt, 'name'),
  ].join('\n');
}

// ---------- sitemap
/** Local content images in a page body (not inline data: pictures), as absolute URLs. */
export function bodyImages(body) {
  const out = new Set();
  for (const m of String(body).matchAll(/<img\b[^>]*?\b(?:data-)?src="([^"]+)"/g)) {
    const s = m[1].replace(/&amp;/g, '&');
    if (s.startsWith('data:') || /^https?:\/\//.test(s) && !s.startsWith(SITE_URL)) continue;
    out.add(s.startsWith('http') ? s : SITE_URL + (s.startsWith('/') ? s : '/' + s));
  }
  return [...out];
}
export function sitemapXml(entries) {
  const row = ({ loc, lastmod, images }) => `<url><loc>${esc(loc)}</loc>${lastmod ? `<lastmod>${lastmod}</lastmod>` : ''}${images.slice(0, 1000).map((i) => `<image:image><image:loc>${esc(i)}</image:loc></image:image>`).join('')}</url>`;
  return `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:image="http://www.google.com/schemas/sitemap-image/1.1">\n${entries.map(row).join('\n')}\n</urlset>\n`;
}
