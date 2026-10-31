// Page shell in the approved website design: control strip (platform, website theme, screen theme), header with the
// Features and Why OpenFrame menus, footer with the company, and the shared ctx helpers (app screens, store buttons).
import { html, raw, esc } from './html.mjs';
import { ICON, MARK, GRADIENT_DEFS } from './icons.mjs';
import { FEATURE_GROUPS, WHY, OS, COMPANY } from '../content/nav.mjs';
import { SITE_URL, SITE_NAME, STORE } from '../../site.config.mjs';

// Runs before first paint (its hash is allowed in the CSP). Platform: the page's own platform, then the saved
// choice, then a guess. Website theme: saved choice, then the system setting. Screen theme: saved choice, else light.
export const BOOT = "(function(){var d=document.documentElement;d.classList.add('js');var g=function(k){try{return localStorage.getItem(k)}catch(e){return null}},p=d.getAttribute('data-page-os'),s=g('of_os');if(p){d.setAttribute('data-os',p);try{localStorage.setItem('of_os',p)}catch(e){}}else if(s==='win'||s==='mac'){d.setAttribute('data-os',s)}else{var u=(navigator.userAgentData&&navigator.userAgentData.platform)||navigator.platform||navigator.userAgent||'';d.setAttribute('data-os',/mac|iphone|ipad/i.test(u)?'mac':'win')}var t=g('of_theme');if(t!=='light'&&t!=='dark'){t=window.matchMedia&&matchMedia('(prefers-color-scheme: dark)').matches?'dark':'light'}d.setAttribute('data-theme',t);d.setAttribute('data-mock',g('of_mock')==='dark'?'dark':'light');try{if(sessionStorage.getItem('of_seen'))d.classList.add('ready')}catch(e){}})();";

// Store marks are single-colour and take the button's text colour, so they sit cleanly on any button fill.
const MSICON = raw('<svg class="i logo" viewBox="0 0 24 24" aria-hidden="true" focusable="false" fill="currentColor"><path d="M3 3h8.5v8.5H3zM12.5 3H21v8.5h-8.5zM3 12.5h8.5V21H3zM12.5 12.5H21V21h-8.5z"/></svg>');
const STUDIO = raw('<small aria-hidden="true"><i>S</i><i>T</i><i>U</i><i>D</i><i>I</i><i>O</i></small>');
export const WORDMARK = html`<span class="wm"><b><i>Open</i>Frame</b>${STUDIO}</span>`;

export function makeHelpers(cat, missing) {
  // A real captured app screen, embedded as its HTML page and scaled to the frame. The screen theme control swaps
  // /light/ and /dark/ in the src; the page is the same screen in both folders.
  const mock = (plat, name, { caption = '', alt = '', link = true, eager = false } = {}) => {
    const s = cat[plat]?.get(name);
    if (!s) { missing.add(`${plat}/${name}`); return html`<span class="shot-missing">${plat}/${name}</span>`; }
    const src = `/mockups/${plat}/light/${s.file}`;
    const label = alt || s.title;
    return html`<figure class="screen"><div class="mock ${plat}" data-w="${s.width}" data-h="${s.height}"><span class="mock-load" aria-hidden="true">${MARK}</span><iframe data-mock src="${src}" title="${OS[plat].name} screen: ${label}"${raw(eager ? '' : ' loading="lazy"')} tabindex="-1" aria-hidden="true"></iframe></div>${caption || link ? html`<figcaption class="mock-cap"><span>${caption || label}</span>${link ? html`<a data-mock href="${src}" target="_blank" rel="noopener">Open this screen<span class="sr"> in a new tab</span></a>` : ''}</figcaption>` : ''}</figure>`;
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
  // Price for the visitor's region (India: rupees, elsewhere: US dollars). Without script both are listed.
  const price = () => html`<span class="pr" data-price><span data-price-in>₹2,999</span><span data-price-both> in India, </span><span data-price-us>US$89</span><span data-price-both> elsewhere</span></span>`;
  const cta = () => html`<section class="cta-band"><div class="wrap">
<span class="eyebrow">Ready when you are</span>
<h2>Try it free for 7 days. Pay once if you keep it.</h2>
<p class="lead">One app for the whole film, on your own computer. One payment, no subscription.</p>
<div class="hero-cta">${storeRow()}</div>
<div class="hero-cta"><a class="btn btn-ghost" href="/tutorials/" data-os-href-win="/windows/tutorials/" data-os-href-mac="/mac/tutorials/">Read the tutorials</a></div>
</div></section>`;
  return { mock, osBlocks, osText, storeBtn, storeRow, price, cta };
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
};
const seg = (name, key, opts) => html`<div class="ctl" role="group" aria-label="${name}"><span class="ctl-ic" title="${name}">${CTL_ICON[key]}</span><span class="ctl-name" aria-hidden="true">${name}</span><span class="seg">${opts.map(([v, t]) => html`<button type="button" data-set="${key}" data-val="${v}" aria-pressed="false" aria-label="${name}: ${t}" title="${name}: ${t}">${CTL_ICON[v]}</button>`)}</span></div>`;

function header(page, switchHref) {
  const cur = (k) => (page.nav === k ? raw(' aria-current="page"') : '');
  const feat = FEATURE_GROUPS.map(([g, items]) => html`<div class="mg"><h4>${g}</h4>${items.map(([slug, t, b]) => html`<a href="/features/${slug}/"><b>${t}</b><span>${b}</span></a>`)}</div>`);
  const why = WHY.map(([href, t, b]) => html`<div class="mg"><a href="${href}"><b>${t}</b><span>${b}</span></a></div>`);
  // The three controls sit in the header on wide screens and inside the menu drawer on narrow ones.
  const ctls = html`${seg('Platform', 'os', [['win', 'Windows'], ['mac', 'Mac']])}${seg('Website theme', 'theme', [['light', 'Light'], ['dark', 'Dark']])}${seg('App screen theme', 'mock', [['light', 'Light'], ['dark', 'Dark']])}`;
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
<li class="menu-ctls">${ctls}</li>
</ul>
<div class="ctls nav-ctls">${ctls}</div>
<a class="btn btn-primary btn-sm nav-get" href="/download/"${cur('download')}>Get OpenFrame</a>
<button type="button" class="menu-tg" id="menutg" aria-label="Menu" aria-controls="menu" aria-expanded="false">${ICON.menu}</button>
</div></header>
<script type="application/json" id="os-switch">${raw(JSON.stringify(switchHref))}</script>`;
}

function footer() {
  const col = (t, items) => html`<div><h4>${t}</h4><ul>${items.map(([a, h]) => html`<li><a href="${h}">${a}</a></li>`)}</ul></div>`;
  const all = FEATURE_GROUPS.flatMap(([, items]) => items);
  return html`<footer class="site-f"><div class="wrap"><div class="f-grid">
<div><a class="brand" href="/" >${MARK}${WORDMARK}<span class="sr"> Studio, home</span></a>
<p class="about">A film production software for all your needs. From film lovers, to film lovers across the world.</p>
<div class="f-company"><img src="/company/vighnajith-advisories-logo.webp" width="1774" height="887" alt="${COMPANY} logo" loading="lazy"><span>${SITE_NAME} is a product of ${COMPANY}.</span></div></div>
${col('Ideate & Write', all.slice(0, 7).map(([s, t]) => [t, `/features/${s}/`]))}
${col('Plan & Shoot', all.slice(7).map(([s, t]) => [t, `/features/${s}/`]).concat([['All features', '/features/']]))}
${col('Why OpenFrame', WHY.map(([h, t]) => [t, h]).concat([['Windows', '/windows/'], ['Mac', '/mac/']]))}
${col('Support', [['Windows tutorials', '/windows/tutorials/'], ['Mac tutorials', '/mac/tutorials/'], ['FAQ', '/faq/'], ['Contact us', '/contact/'], ['Download', '/download/'], ['Privacy Policy', '/legal/privacy/'], ['Terms of Use', '/legal/terms/'], ['About us', '/about/']])}
</div><div class="f-bot"><span>&copy; 2026 ${COMPANY}. <b>Your privacy stays on your computer.</b></span>
<span>Microsoft and Microsoft Store are trademarks of the Microsoft group of companies. Apple, Mac, macOS and Mac App Store are trademarks of Apple Inc.</span></div></div></footer>`;
}

export function renderPage(page, { css, js, switchHref }) {
  const url = SITE_URL + page.path;
  const crumbs = page.breadcrumbs?.length ? [['Home', '/'], ...page.breadcrumbs] : null;
  const ld = [...(page.jsonld || [])];
  if (crumbs) ld.push({ '@context': 'https://schema.org', '@type': 'BreadcrumbList', itemListElement: crumbs.map(([name, p], i) => ({ '@type': 'ListItem', position: i + 1, name, item: SITE_URL + p })) });
  const og = page.ogImage ? (page.ogImage.startsWith('http') ? page.ogImage : SITE_URL + page.ogImage) : SITE_URL + '/og.png';
  return `<!doctype html>
<html lang="en" data-os="${page.os || 'win'}"${page.os ? ` data-page-os="${page.os}"` : ''} data-theme="light" data-mock="light">
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
<meta property="og:image:width" content="1200">
<meta property="og:image:height" content="630">
<meta property="og:image:alt" content="OpenFrame Studio: a film production software for all your needs, with a real app screen.">
<meta property="og:locale" content="en_US">
<meta name="twitter:card" content="summary_large_image">
<meta name="twitter:image:alt" content="OpenFrame Studio: a film production software for all your needs, with a real app screen.">
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
<link rel="preload" href="/assets/fonts/inter-latin-wght-normal.woff2" as="font" type="font/woff2" crossorigin>
<link rel="preload" href="/assets/fonts/newsreader-latin-wght-normal.woff2" as="font" type="font/woff2" crossorigin>
<link rel="preload" href="/assets/fonts/sora-latin-wght-normal.woff2" as="font" type="font/woff2" crossorigin>
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
