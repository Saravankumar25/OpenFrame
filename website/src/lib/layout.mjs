// Page shell: head, header with the macOS | Windows switch, footer, and the shared ctx helpers.
import { html, raw, esc } from './html.mjs';
import { ICON, MARK, GRADIENT_DEFS } from './icons.mjs';
import { FEATURE_GROUPS, OS } from '../content/nav.mjs';
import { SITE_URL, SITE_NAME } from '../../site.config.mjs';

// Runs before first paint (its hash is allowed in the CSP): page platform wins, then the saved choice, then a guess.
export const OS_BOOT = "(function(){var d=document.documentElement,p=d.getAttribute('data-page-os'),s=null;try{s=localStorage.getItem('of_os')}catch(e){}if(p){d.setAttribute('data-os',p);try{localStorage.setItem('of_os',p)}catch(e){}return}if(s==='win'||s==='mac'){d.setAttribute('data-os',s);return}var u=(navigator.userAgentData&&navigator.userAgentData.platform)||navigator.platform||navigator.userAgent||'';d.setAttribute('data-os',/mac|iphone|ipad/i.test(u)?'mac':'win')})();";

export function makeHelpers(screens, missing) {
  const img = (key, alt, { eager = false, sizes = '(min-width: 1200px) 1120px, 100vw' } = {}) => {
    const m = screens[key];
    if (!m) { missing.add(key); return html`<span class="shot-missing">${alt}</span>`; }
    return html`<img src="${m.src}-1600.webp" srcset="${m.src}-800.webp 800w, ${m.src}-1600.webp 1600w" sizes="${sizes}" width="${m.w}" height="${m.h}" alt="${alt}"${raw(eager ? ' fetchpriority="high"' : ' loading="lazy"')} decoding="async">`;
  };
  const shot = (plat, screen, { alt = '', eager = false, crop = null, caption = '', sizes } = {}) => {
    const key = `${plat}/${screen}`, m = screens[key];
    let inner;
    if (crop && m) {
      // Crop with an SVG viewBox: no inline styles needed under the CSP.
      const [x, y, w, h] = crop;
      inner = html`<svg class="shot-crop" viewBox="${x} ${y} ${w} ${h}" role="img" aria-label="${alt}"><image href="${m.src}-1600.webp" width="${m.w}" height="${m.h}" preserveAspectRatio="none"/></svg>`;
    } else inner = img(key, alt, { eager, sizes });
    if (crop && !m) missing.add(key);
    return html`<figure class="shot shot-${plat}${crop ? ' is-crop' : ''}">${inner}${caption ? html`<figcaption>${caption}</figcaption>` : ''}</figure>`;
  };
  const osBlocks = ({ win, mac }) => html`<div class="os-only" data-os-show="win">${win}</div><div class="os-only" data-os-show="mac">${mac}</div>`;
  const osText = (win, mac) => html`<span data-os-show="win">${win}</span><span data-os-show="mac">${mac}</span>`;
  const kbd = (combo) => html`<span class="keys">${combo.split('+').map((k, i) => html`${i ? '+' : ''}<kbd>${k}</kbd>`)}</span>`;
  const cta = () => html`<section class="section dark cta-band"><div class="wrap center">
<h2>See it working before it ships.</h2>
<p class="lead">OpenFrame Studio is in team testing on Windows and Mac. Public release is planned on the Microsoft Store and the Mac App Store. Until then, the tutorials show every workflow on real screens.</p>
<div class="btn-row">${osBlocks({
    win: html`<a class="btn btn-primary btn-lg" href="/windows/tutorials/">${ICON.windows}Windows tutorials</a>`,
    mac: html`<a class="btn btn-primary btn-lg" href="/mac/tutorials/">${ICON.apple}macOS tutorials</a>` })}
<a class="btn btn-ghost-dark btn-lg" href="/download/">Release status</a></div></div></section>`;
  return { shot, img, osBlocks, osText, kbd, cta };
}

const NAV = [['tutorials', 'Tutorials', '/tutorials/', '/windows/tutorials/', '/mac/tutorials/'], ['pricing', 'Pricing', '/pricing/'], ['faq', 'FAQ', '/faq/'], ['download', 'Download', '/download/', '/windows/', '/mac/']];

function header(page, switchHref) {
  const cur = (k) => (page.nav === k ? raw(' aria-current="page"') : '');
  const mega = FEATURE_GROUPS.map(([g, items]) => html`<div class="mg"><p class="mg-h">${g}</p>${items.map(([slug, t, b]) => html`<a href="/features/${slug}/"><b>${t}</b><span>${b}</span></a>`)}</div>`);
  const pick = (k) => html`<a class="os-opt" data-os-pick="${k}" href="${switchHref[k]}" title="Show ${OS[k].name} screens, tutorials and downloads">${k === 'mac' ? ICON.apple : ICON.windows}<span>${OS[k].name}</span></a>`;
  return html`<a class="skip" href="#main">Skip to content</a>${GRADIENT_DEFS}
<div class="annc"><div class="wrap">OpenFrame Studio is in team testing on Windows and Mac. <a href="/download/">Release status</a></div></div>
<header class="site-h"><div class="wrap nav">
<a class="brand" href="/" aria-label="OpenFrame Studio home">${MARK}<span class="wm"><b><i>Open</i>Frame</b><small>STUDIO</small></span></a>
<nav class="menu-wrap" aria-label="Main"><ul class="menu" id="menu">
<li class="has-mega"><button type="button" class="menu-btn" aria-expanded="false" aria-controls="mega-features"${page.nav === 'features' ? raw(' data-current="true"') : ''}>Features${ICON.chev}</button>
<div class="mega" id="mega-features"><div class="mega-in">${mega}<div class="mega-foot"><span>One project, from first idea to final call sheet.</span><a href="/features/">All features</a></div></div></div></li>
${NAV.map(([k, t, h, w, m]) => html`<li><a href="${h}"${w ? raw(` data-os-href-win="${w}" data-os-href-mac="${m}"`) : ''}${cur(k)}>${t}</a></li>`)}
</ul></nav>
<div class="os-switch" role="group" aria-label="Platform">${pick('mac')}${pick('win')}</div>
<button type="button" class="menu-tg" id="menutg" aria-label="Menu" aria-controls="menu" aria-expanded="false">${ICON.menu}</button>
</div></header>`;
}

function footer() {
  const col = (t, items) => html`<div><p class="f-h">${t}</p><ul>${items.map(([a, h]) => html`<li><a href="${h}">${a}</a></li>`)}</ul></div>`;
  const all = FEATURE_GROUPS.flatMap(([, items]) => items);
  return html`<footer class="site-f"><div class="wrap"><div class="f-grid">
<div><a class="brand" href="/" aria-label="OpenFrame Studio home">${MARK}<span class="wm"><b><i>Open</i>Frame</b><small>STUDIO</small></span></a>
<p class="f-about">One local-first app for writers, screenwriters and production teams. Ideas, story, screenplay and production planning on your own computer.</p></div>
${col('Ideate and write', all.slice(0, 7).map(([s, t]) => [t, `/features/${s}/`]))}
${col('Plan and shoot', all.slice(7).map(([s, t]) => [t, `/features/${s}/`]).concat([['All features', '/features/']]))}
${col('Learn', [['Tutorials', '/tutorials/'], ['Windows tutorials', '/windows/tutorials/'], ['macOS tutorials', '/mac/tutorials/'], ['FAQ', '/faq/'], ['Switch from other software', '/switch/'], ['Compare', '/compare/']])}
${col('OpenFrame', [['Windows', '/windows/'], ['macOS', '/mac/'], ['Release status', '/download/'], ['Pricing', '/pricing/'], ['Local and private', '/local-and-private/'], ['Themes', '/themes/'], ['About', '/about/'], ['Contact', '/contact/'], ['Website privacy', '/legal/privacy/'], ['Website terms', '/legal/terms/']])}
</div><div class="f-bot"><span>&copy; 2026 OpenFrame Studio. Your projects stay on your computer.</span>
<span>Microsoft and Windows are trademarks of the Microsoft group of companies. Apple, Mac and macOS are trademarks of Apple Inc. OpenFrame Studio is not affiliated with Microsoft or Apple.</span></div></div></footer>`;
}

export function renderPage(page, { css, js, switchHref }) {
  const url = SITE_URL + page.path;
  const crumbs = page.breadcrumbs?.length ? [['Home', '/'], ...page.breadcrumbs] : null;
  const ld = [...(page.jsonld || [])];
  if (crumbs) ld.push({ '@context': 'https://schema.org', '@type': 'BreadcrumbList', itemListElement: crumbs.map(([name, p], i) => ({ '@type': 'ListItem', position: i + 1, name, item: SITE_URL + p })) });
  const og = page.ogImage ? (page.ogImage.startsWith('http') ? page.ogImage : SITE_URL + page.ogImage) : SITE_URL + '/og.png';
  const crumbNav = crumbs ? html`<nav class="breadcrumbs wrap" aria-label="Breadcrumb"><ol>${crumbs.map(([n, p], i) => html`<li>${i === crumbs.length - 1 ? html`<span aria-current="page">${n}</span>` : html`<a href="${p}">${n}</a>`}</li>`)}</ol></nav>` : '';
  return `<!doctype html>
<html lang="en" data-os="${page.os || 'win'}"${page.os ? ` data-page-os="${page.os}"` : ''}>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>${esc(page.title)}</title>
<meta name="description" content="${esc(page.description)}">
<link rel="canonical" href="${esc(url)}">
${page.noindex ? '<meta name="robots" content="noindex">\n' : ''}<meta property="og:type" content="website">
<meta property="og:site_name" content="${SITE_NAME}">
<meta property="og:title" content="${esc(page.ogTitle || page.title)}">
<meta property="og:description" content="${esc(page.description)}">
<meta property="og:url" content="${esc(url)}">
<meta property="og:image" content="${esc(og)}">
<meta name="twitter:card" content="summary_large_image">
<meta name="theme-color" content="#1d1a2e">
<link rel="icon" href="/favicon.svg" type="image/svg+xml">
<link rel="preload" href="/assets/fonts/inter-latin-wght-normal.woff2" as="font" type="font/woff2" crossorigin>
<link rel="stylesheet" href="${css}">
<script id="os-boot">${OS_BOOT}</script>
${ld.map((o) => `<script type="application/ld+json">${JSON.stringify(o).replace(/</g, '\\u003c')}</script>`).join('\n')}
</head>
<body data-nav="${esc(page.nav || '')}">
${header(page, switchHref)}
${crumbNav}
<main id="main">
${page.body}
</main>
${footer()}
<script src="${js}" defer></script>
</body>
</html>
`;
}
