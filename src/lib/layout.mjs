// Page shell in the approved website design: control strip (platform, website theme, screen theme), header with the
// Features and Why OpenFrame menus, footer with the company, and the shared ctx helpers (app screens, store buttons).
import { html, raw, esc } from './html.mjs';
import { ICON, MARK, GRADIENT_DEFS } from './icons.mjs';
import { FEATURE_GROUPS, WHY, OS, COMPANY, COMPANY_ADDRESS, SUPPORT_EMAIL } from '../content/nav.mjs';
import { SITE_URL, SITE_NAME, STORE } from '../../site.config.mjs';
import { headMeta, jsonLd } from './seo.mjs';

// Runs before first paint (its hash is allowed in the CSP). Platform: the page's own platform, then the saved
// choice, then a guess. Website theme: saved choice, then the system setting. Screen theme: saved choice, else light.
export const BOOT = "(function(){var d=document.documentElement;d.classList.add('js');var g=function(k){try{return localStorage.getItem(k)}catch(e){return null}},p=d.getAttribute('data-page-os'),s=g('of_os');if(p){d.setAttribute('data-os',p);try{localStorage.setItem('of_os',p)}catch(e){}}else if(s==='win'||s==='mac'){d.setAttribute('data-os',s)}else{var u=(navigator.userAgentData&&navigator.userAgentData.platform)||navigator.platform||navigator.userAgent||'';d.setAttribute('data-os',/mac|iphone|ipad/i.test(u)?'mac':'win')}var t=g('of_theme');if(t!=='light'&&t!=='dark'){t=window.matchMedia&&matchMedia('(prefers-color-scheme: dark)').matches?'dark':'light'}d.setAttribute('data-theme',t);d.setAttribute('data-mock',g('of_mock')==='dark'?'dark':'light');try{if(sessionStorage.getItem('of_seen'))d.classList.add('ready')}catch(e){}})();";

// Store marks are single-colour and take the button's text colour, so they sit cleanly on any button fill.
const MSICON = raw('<svg class="i logo" viewBox="0 0 24 24" aria-hidden="true" focusable="false" fill="currentColor"><path d="M3 3h8.5v8.5H3zM12.5 3H21v8.5h-8.5zM3 12.5h8.5V21H3zM12.5 12.5H21V21h-8.5z"/></svg>');
const STUDIO = raw('<small aria-hidden="true"><i>S</i><i>T</i><i>U</i><i>D</i><i>I</i><i>O</i></small>');
export const WORDMARK = html`<span class="wm"><b><i>Open</i>Frame</b>${STUDIO}</span>`;

export function makeHelpers(cat, missing, shots = { win: {}, mac: {} }, noShot = new Set(), hasPage = () => true) {
  // One screen = one interactive page, /mockups/<plat>/<theme>/<file>.html (made by scripts/shots.mjs). The manifest
  // gives its size (page), the rectangle a card shows (card) and the rectangle the viewer shows (full), in CSS px.
  // Links name the light folder; the screen theme control swaps /light/ and /dark/ in every [data-mock] attribute.
  const pic = (plat, name) => {
    const m = shots[plat]?.[name], s = cat[plat]?.get(name);
    const rect = (r, n) => Array.isArray(r) && r.length === n && r.every((v) => Number.isFinite(v) && v >= 0);
    if (!s || !m || !rect(m.page, 2) || !rect(m.card, 4) || !rect(m.full, 4) || !m.card[2] || !m.card[3]) { noShot.add(`${plat}/${name}`); return null; }
    if (!hasPage(plat, s.file)) { noShot.add(`${plat}/${name} (no page ${s.file})`); return null; }
    return { url: `/mockups/${plat}/light/${s.file}`, page: m.page, card: m.card, full: m.full };
  };
  const data = (p) => html`data-page="${p.page.join(' ')}" data-full="${p.full.join(' ')}"`;
  // The live, scaled page inside a card. site.js sizes and offsets the iframe from data-page and data-rect so that
  // the card rectangle fills the frame; the card is not interactive (the frame around it is the link).
  const view = (p, label, { eager = false, cover = false } = {}) => html`<span class="shot-view${cover ? ' cover' : ''}" data-page="${p.page.join(' ')}" data-rect="${p.card.join(' ')}"><iframe data-mock src="${p.url}" title="${label}" tabindex="-1" aria-hidden="true" scrolling="no"${raw(eager ? '' : ' loading="lazy"')}></iframe></span>`;
  // A real captured app screen as a live page. Clicking it opens the viewer (lightbox.js) with the full interactive
  // page; without script the link opens the page itself. link:false makes it a plain, non-clickable frame.
  const mock = (plat, name, { caption = '', alt = '', link = true, eager = false } = {}) => {
    const s = cat[plat]?.get(name);
    if (!s) { missing.add(`${plat}/${name}`); return html`<span class="shot-missing">${plat}/${name}</span>`; }
    const p = pic(plat, name);
    if (!p) return html`<span class="shot-missing">${plat}/${name}</span>`;
    const label = alt || `${OS[plat].name} screen: ${s.title}`;
    const inner = html`<span class="mock-load" aria-hidden="true">${MARK}</span><svg class="shot-ratio" viewBox="0 0 ${p.card[2]} ${p.card[3]}" width="${p.card[2]}" height="${p.card[3]}" aria-hidden="true" focusable="false"></svg>${view(p, label, { eager })}`;
    const frame = link ? html`<a class="shot-open" href="${p.url}" data-mock ${data(p)}>${inner}<span class="sr">${label}</span></a>` : html`<span class="shot-open shot-static" role="img" aria-label="${label}">${inner}</span>`;
    return html`<figure class="shot ${plat}">${frame}${caption ? html`<figcaption>${caption}</figcaption>` : ''}</figure>`;
  };
  // A small live preview for a tutorial card: the card rectangle covers the box, top edge first.
  const thumbView = (plat, name) => {
    const p = cat[plat]?.get(name) && pic(plat, name);
    return p ? view(p, '', { cover: true }) : '';
  };
  // A text link that opens one screen in the viewer (the "Shown in a screen above" lines).
  const shotLink = (plat, name, text) => {
    const p = pic(plat, name);
    return p ? html`<a class="shot-open shot-link" href="${p.url}" data-mock ${data(p)} data-caption="${text}">${text}</a>` : html`<span class="shot-missing">${plat}/${name}</span>`;
  };
  const osBlocks = ({ win, mac }) => html`<div data-os-show="win">${win}</div><div data-os-show="mac">${mac}</div>`;
  const osText = (win, mac) => html`<span data-os-show="win">${win}</span><span data-os-show="mac">${mac}</span>`;
  // Store buttons: a link once a store listing exists (site.config.mjs STORE); until then a normal-looking button
  // that does nothing when clicked (owner decision 2026-10-31).
  const storeBtn = (plat, cls = '') => {
    const label = plat === 'win' ? 'Download for Windows: Microsoft Store' : 'Download for Mac: Mac App Store';
    const icon = plat === 'win' ? MSICON : ICON.apple;
    const kind = plat === 'win' ? 'btn-primary' : 'btn-mac';
    return STORE[plat] ? html`<a class="btn ${kind} ${cls}" href="${STORE[plat]}" rel="noopener">${icon}${label}</a>`
      : html`<button type="button" class="btn ${kind} ${cls}" data-store="${plat}">${icon}${label}</button>`;
  };
  const storeRow = (cls = 'btn-lg') => html`<div class="store-row">${storeBtn('win', cls)}${storeBtn('mac', cls)}</div>`;
  // Price for the visitor's region (India: rupees, Europe: euros, other countries: US dollars). Without script all three are listed.
  const price = () => html`<span class="pr" data-price><span data-price-in>₹2,999</span><span data-price-both> in India, </span><span data-price-eu>€89</span><span data-price-both> in Europe, </span><span data-price-us>US$89</span><span data-price-both> in other countries</span></span>`;
  const cta = () => html`<section class="cta-band"><div class="wrap">
<span class="eyebrow">Ready when you are</span>
<h2>Try it free for 7 days. Pay once if you keep it.</h2>
<p class="lead">One app for the whole film, on your own computer. One payment, no subscription.</p>
<div class="hero-cta">${storeRow()}</div>
<div class="hero-cta"><a class="btn btn-ghost" href="/tutorials/" data-os-href-win="/windows/tutorials/" data-os-href-mac="/mac/tutorials/">Read the tutorials</a></div>
</div></section>`;
  return { mock, shotLink, thumbView, pic, osBlocks, osText, storeBtn, storeRow, price, cta };
}

// The three controls are icon buttons; each has a spoken name and a tooltip, and its group has a small icon label.
const I = (d, fill = false) => raw(`<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"${fill ? ' fill="currentColor"' : ' fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"'}>${d}</svg>`);
const CTL_ICON = {
  os: I('<rect x="3" y="4" width="18" height="12" rx="2"/><path d="M8 20h8M12 16v4"/>'),
  theme: I('<circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3a14 14 0 0 1 0 18M12 3a14 14 0 0 0 0 18"/>'),
  mock: I('<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 9h18M7 6.5h.01M10 6.5h.01"/>'),
  win: I('<path d="M3 5.5l7.5-1v7H3zM11.5 4.4L21 3v8.5h-9.5zM3 12.5h7.5v7L3 18.5zM11.5 12.5H21V21l-9.5-1.4z"/>', true),
  mac: I('<path d="M16.4 12.7c0-2.4 2-3.6 2.1-3.7-1.1-1.7-2.9-1.9-3.5-1.9-1.5-.2-2.9.9-3.7.9-.8 0-1.9-.9-3.2-.8-1.6 0-3.2 1-4 2.4-1.7 3-.4 7.4 1.2 9.8.8 1.2 1.8 2.5 3 2.4 1.2 0 1.7-.8 3.1-.8s1.9.8 3.2.8c1.3 0 2.2-1.2 3-2.4.9-1.4 1.3-2.7 1.3-2.8-.1 0-2.5-1-2.5-3.9zM14 5.4c.7-.8 1.1-1.9 1-3-1 0-2.1.7-2.8 1.5-.6.7-1.2 1.8-1 2.9 1.1.1 2.1-.6 2.8-1.4z"/>', true),
  light: I('<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/>'),
  dark: I('<path d="M20 14.5A8 8 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5z"/>'),
  // the app screen theme shows a small app window, so it reads differently from the website theme's sun and moon
  mlight: I('<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 9h18"/>'),
  mdark: I('<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 9h18v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" fill="currentColor"/>'),
};
const seg = (name, key, opts) => html`<div class="ctl" role="group" aria-label="${name}"><span class="ctl-ic" title="${name}">${CTL_ICON[key]}</span><span class="ctl-name" aria-hidden="true">${name}</span><span class="seg">${opts.map(([v, t, ic = v]) => html`<button type="button" data-set="${key}" data-val="${v}" aria-pressed="false" aria-label="${name}: ${t}" title="${name}: ${t}">${CTL_ICON[ic]}</button>`)}</span></div>`;

function header(page, switchHref) {
  const cur = (k) => (page.nav === k ? raw(' aria-current="page"') : '');
  const feat = FEATURE_GROUPS.map(([g, items]) => html`<div class="mg"><p class="mg-h">${g}</p>${items.map(([slug, t, b]) => html`<a href="/features/${slug}/"><b>${t}</b><span>${b}</span></a>`)}</div>`);
  const why = WHY.map(([href, t, b]) => html`<div class="mg"><a href="${href}"><b>${t}</b><span>${b}</span></a></div>`);
  // The three controls sit in the header on wide screens and inside the menu drawer on narrow ones.
  const ctls = html`${seg('Platform', 'os', [['win', 'Windows'], ['mac', 'Mac']])}${seg('Website theme', 'theme', [['light', 'Light'], ['dark', 'Dark']])}${seg('App screen theme', 'mock', [['light', 'Light', 'mlight'], ['dark', 'Dark', 'mdark']])}`;
  return html`<a class="skip" href="#main">Skip to content</a>${GRADIENT_DEFS}
<header class="site-h"><div class="wrap nav">
<a class="brand draw" href="/" >${MARK}${WORDMARK}<span class="sr"> Studio, home</span></a>
<ul class="menu" id="menu">
<li class="has-mega"><button type="button" aria-expanded="false"${page.nav === 'features' ? raw(' class="is-current"') : ''}>Features${ICON.chev}</button><div class="mega"><div class="mega-in">${feat}
<div class="mega-foot"><span>One film, moving forward, from first idea to final call sheet.</span><a href="/features/">See everything OpenFrame does &rarr;</a></div></div></div></li>
<li class="has-mega"><button type="button" aria-expanded="false">Why OpenFrame${ICON.chev}</button><div class="mega small"><div class="mega-in">${why}</div></div></li>
<li><a href="/pricing/"${cur('pricing')}>Pricing</a></li>
<li><a href="/tutorials/" data-os-href-win="/windows/tutorials/" data-os-href-mac="/mac/tutorials/"${cur('tutorials')}>Tutorials</a></li>
<li><a href="/faq/"${cur('faq')}>FAQ</a></li>
<li><a href="/about/"${cur('about')}>About Us</a></li>
<li class="menu-get"><a class="btn btn-primary" href="/download/">Get OpenFrame</a></li>
<li class="menu-ctls">${ctls}</li>
</ul>
<div class="ctls nav-ctls">${ctls}</div>
<a class="srch-btn" href="/search/" data-search-open aria-haspopup="dialog" aria-keyshortcuts="Control+K Meta+K /"><svg viewBox="0 0 24 24" aria-hidden="true" focusable="false" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="7"/><path d="M20 20l-3.5-3.5"/></svg><span class="sr">Search</span><kbd class="srch-k" aria-hidden="true"><span data-os-show="win">Ctrl K</span><span data-os-show="mac">Cmd K</span></kbd></a>
<a class="btn btn-primary btn-sm nav-get" href="/download/"${cur('download')}>Get OpenFrame</a>
<button type="button" class="menu-tg" id="menutg" aria-label="Menu" aria-controls="menu" aria-expanded="false"><span class="tg-open">${ICON.menu}</span><span class="tg-close">${ICON.cross}</span></button>
</div></header>
<script type="application/json" id="os-switch">${raw(JSON.stringify(switchHref))}</script>`;
}

const LEGAL_LINKS = [['Privacy', '/legal/privacy/'], ['Terms', '/legal/terms/'], ['Refunds', '/legal/refunds/'], ['Cookies', '/legal/cookies/'], ['Notices', '/legal/notices/']];
function footer() {
  const col = (t, items) => html`<div><p class="f-h">${t}</p><ul>${items.map(([a, h]) => html`<li><a href="${h}">${a}</a></li>`)}</ul></div>`;
  const all = FEATURE_GROUPS.flatMap(([, items]) => items);
  // The Get OpenFrame dialog and pill (src/assets/js/getapp.js clones this). Same store buttons and price as everywhere else.
  const { storeBtn, price } = makeHelpers();
  const getapp = html`<template id="getapp-tpl"><dialog class="gp-dlg" aria-labelledby="gp-title"><div class="gp-box">
<button type="button" class="gp-x" data-gp-close aria-label="Close">${ICON.cross}</button>
<div class="gp-top"><h2 id="gp-title">Try OpenFrame Studio free for 7 days</h2><p class="gp-lead">Pay once. Own forever. No subscription.</p><p class="gp-price">Keep it for ${price()}, paid once.</p></div>
<div class="gp-body">
<section class="gp-sec" data-gp-os="win" aria-labelledby="gp-win"><h3 id="gp-win">Windows</h3><p>Get it from the Microsoft Store.</p>${storeBtn('win', 'btn-lg')}</section>
<section class="gp-sec" data-gp-os="mac" aria-labelledby="gp-mac"><h3 id="gp-mac">Mac</h3><p>Get it from the Mac App Store.</p>${storeBtn('mac', 'btn-lg')}</section>
<p class="gp-links"><a href="/pricing/">See pricing</a><a href="/tutorials/" data-os-href-win="/windows/tutorials/" data-os-href-mac="/mac/tutorials/">Explore the tutorials</a></p>
</div></div></dialog><button type="button" class="gp-pill" hidden>Get OpenFrame</button></template>`;
  return html`<footer class="site-f"><div class="wrap"><div class="f-grid">
<div><a class="brand" href="/" >${MARK}${WORDMARK}<span class="sr"> Studio, home</span></a>
<p class="about">A film production software for all your needs. From film lovers, to film lovers across the world.</p>
<div class="f-company"><img src="/company/vighnajith-advisories-logo-192.webp" srcset="/company/vighnajith-advisories-logo-96.webp 96w, /company/vighnajith-advisories-logo-192.webp 192w, /company/vighnajith-advisories-logo-288.webp 288w" sizes="96px" width="96" height="48" alt="${COMPANY} logo" loading="lazy"><span>${SITE_NAME} is a product of ${COMPANY}.</span></div></div>
${col('Ideate & Write', all.slice(0, 7).map(([s, t]) => [t, `/features/${s}/`]))}
${col('Plan & Shoot', all.slice(7).map(([s, t]) => [t, `/features/${s}/`]).concat([['All features', '/features/']]))}
${col('Why OpenFrame', WHY.map(([h, t]) => [t, h]).concat([['Windows', '/windows/'], ['Mac', '/mac/']]))}
${col('Support', [['Windows tutorials', '/windows/tutorials/'], ['Mac tutorials', '/mac/tutorials/'], ['FAQ', '/faq/'], ['Contact us', '/contact/'], ['Download', '/download/'], ['About us', '/about/']])}
</div><nav class="f-legal" aria-label="Legal">${LEGAL_LINKS.map(([a, h]) => html`<a href="${h}">${a}</a>`)}</nav><div class="f-bot"><span>&copy; ${new Date().getFullYear()} ${COMPANY}. <b>Your privacy stays on your computer.</b></span>
<span>Microsoft and Microsoft Store are trademarks of the Microsoft group of companies. Apple, Mac, macOS and Mac App Store are trademarks of Apple Inc.</span></div>
<p class="f-pub">Published by ${COMPANY} · ${COMPANY_ADDRESS} · <a href="mailto:${SUPPORT_EMAIL}">${SUPPORT_EMAIL}</a></p></div></footer>${getapp}`;
}

export function renderPage(page, { css, js, switchHref }) {
  const ld = jsonLd(page);
  // No font preloads on purpose: on a slow phone they share the line with the stylesheet and delay first paint by about 1 s (docs/SEO.md).
  return `<!doctype html>
<html lang="en" data-os="${page.os || 'win'}"${page.os ? ` data-page-os="${page.os}"` : ''} data-theme="light" data-mock="light">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
${headMeta(page)}
<meta name="theme-color" content="#ffffff" media="(prefers-color-scheme: light)">
<meta name="theme-color" content="#0e0d15" media="(prefers-color-scheme: dark)">
<meta name="application-name" content="OpenFrame Studio">
<meta name="apple-mobile-web-app-title" content="OpenFrame">
<link rel="icon" href="/favicon.svg" type="image/svg+xml">
<link rel="icon" href="/favicon.ico" sizes="48x48">
<link rel="icon" href="/favicon-32x32.png" type="image/png" sizes="32x32">
<link rel="icon" href="/favicon-16x16.png" type="image/png" sizes="16x16">
<link rel="apple-touch-icon" href="/apple-touch-icon.png">
<link rel="manifest" href="/site.webmanifest">
<link rel="stylesheet" href="${css}">
<script id="boot">${BOOT}</script>
${ld.map((o) => `<script type="application/ld+json">${JSON.stringify(o).replace(/</g, '\\u003c')}</script>`).join('\n')}
</head>
<body data-nav="${esc(page.nav || '')}">
<div class="loader" id="loader" aria-hidden="true"><div class="loader-in">${MARK}</div></div>
${header(page, switchHref)}
<main id="main">
${page.body}
</main>
${footer()}
<script src="${js}" defer></script>
</body>
</html>
`;
}
