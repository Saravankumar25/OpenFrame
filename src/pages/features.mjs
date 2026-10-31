// /features/ index and one page per feature group, laid out as the approved website design's feature pages.
// Page copy: src/content/features.mjs. Sub-feature catalog: src/content/catalog.mjs (every verified capability,
// with its screens per platform). Screens are named without their number (research/screens-<plat>.json).
import { FEATURE_PAGES } from '../content/features.mjs';
import { CATALOG } from '../content/catalog.mjs';
import { APP } from './info.mjs';

const nameOf = (s) => (s ? s.replace(/^\d+-/, '') : s);
// Tutorials (manual workflows) that teach each feature group: matched on the area slug or the workflow title.
// Tutorials (manual workflows) that teach each feature group: [area slugs, workflow title words].
const LEARN = {
  'idea-vault': [/^idea-vault$/, null], 'story-board': [/^story$/, null], moodboards: [/^moodboards$/, /\bmoodboards?\b/i],
  screenplay: [/^screenplay$/, null], 'drafts-revisions': [/^$/, /\b(drafts?|revisions?|compare|lock|review|comments?)\b/i],
  'import-export': [/^import-screenplay$/, /\b(import|export)/i], 'openframe-ai': [/^openframe-ai$/, null], breakdown: [/^breakdown$/, null],
  'cast-crew-locations': [/^(locations|cast-and-crew)$/, /\b(cast|crew|locations?)\b/i], 'shot-lists-storyboards': [/^(storyboards|shot-lists)$/, /\b(shots?|storyboards?)\b/i],
  scheduling: [/^(shooting-schedule|schedule|day-out-of-days)$/, null], 'call-sheets': [/^call-sheets$/, null],
  'sides-daily-view': [/^(sides|daily-view)$/, null], 'reports-budget': [/^(reports|budget)$/, null],
  'backups-recovery': [/^(backups-and-packages|packages|recovery|recently-deleted)$/, null],
};

export function learnFor(ctx, slug, plat, max = 9) {
  const [area, words] = LEARN[slug];
  return ctx.manual[plat].flatMap((a) => a.workflows.map((w) => ({ ...w, area: a })))
    .filter((w) => area.test(w.area.slug) || (words && words.test(w.title))).slice(0, max);
}

function screenPair(ctx, shots, caption) {
  const win = shots.win && ctx.mock('win', nameOf(shots.win), { caption });
  const mac = shots.mac && ctx.mock('mac', nameOf(shots.mac), { caption });
  return ctx.html`${win ? ctx.html`<div data-os-show="win">${win}</div>` : ''}${mac ? ctx.html`<div data-os-show="mac">${mac}</div>` : ''}`;
}

function catalogSection(ctx, slug, title) {
  const { html } = ctx;
  const items = CATALOG[slug] || [];
  if (!items.length) return '';
  const avail = (f) => html`<div class="avail">${f.win?.length ? html`<span class="yes">Windows</span>` : html`<span>Not on Windows</span>`}${f.mac?.length || f.onMac ? html`<span class="yes">Mac</span>` : html`<span>Not on Mac</span>`}</div>`;
  const shots = (f, plat) => {
    const list = f[plat] || [];
    if (!list.length && plat === 'mac' && f.onMac) return html`<p class="more-shots">Available in the Mac app. This page shows no Mac screen for it.</p>`;
    if (!list.length) return html`<p class="more-shots">This capability is not part of the ${plat === 'win' ? 'Windows' : 'Mac'} app.</p>`;
    return html`${ctx.mock(plat, list[0], { caption: f.name })}${list.length > 1 ? html`<p class="more-shots">More screens: ${list.slice(1).map((n, i) => html`${i ? ', ' : ''}<a data-mock href="/mockups/${plat}/light/${ctx.cat[plat].get(n)?.file}" target="_blank" rel="noopener">${ctx.cat[plat].get(n)?.title || n}</a>`)}</p>` : ''}`;
  };
  return html`<section class="sec-s" id="catalog"><div class="wrap"><div class="sec-head"><span class="eyebrow">Complete list</span><h2>Everything in ${title}.</h2>
<p class="lead">Each one with what it does, what it is for, and where it is available.</p></div>
<div class="feat-list">${items.map((f) => html`<article class="feat" id="${f.id}"><div><h3>${f.name}</h3><p class="what">${f.summary}</p>
<dl><dt>What it does</dt><dd>${f.does}</dd><dt>What it is for</dt><dd>${f.purpose}</dd></dl>${avail(f)}</div>
<div class="shots"><div data-os-show="win">${shots(f, 'win')}</div><div data-os-show="mac">${shots(f, 'mac')}</div></div></article>`)}</div></div></section>`;
}

function learnSection(ctx, slug) {
  const { html } = ctx;
  const list = (plat) => {
    const base = plat === 'win' ? '/windows/tutorials/' : '/mac/tutorials/';
    const ws = learnFor(ctx, slug, plat);
    if (!ws.length) return html`<p class="center"><a class="more" href="${base}">All ${plat === 'win' ? 'Windows' : 'Mac'} tutorials &rarr;</a></p>`;
    return html`<div class="grid3">${ws.map((w) => html`<a class="card linkcard" href="${base}${w.id}/"><h3>${w.title}</h3><p>${w.purpose}</p></a>`)}</div>`;
  };
  return html`<section class="tint sec-s"><div class="wrap"><div class="sec-head"><span class="eyebrow">Explore it</span><h2>Step-by-step tutorials.</h2><p class="lead">Written guides on the real screens of your platform.</p></div>
${ctx.osBlocks({ win: list('win'), mac: list('mac') })}</div></section>`;
}

function featurePage(ctx, f, slug) {
  const { html } = ctx;
  const meta = ctx.FEATURES.find((x) => x.slug === slug);
  const page = FEATURE_PAGES[slug];
  const body = html`
<section class="phero"><div class="wrap"><span class="eyebrow">${meta.group}</span><h1>${page.h1}</h1><p class="lead">${page.lead}</p>
<div class="hero-cta">${ctx.storeRow()}</div>
<div class="tour-real">${screenPair(ctx, page.hero, page.title)}</div></div></section>
${page.rows.map((r, n) => html`<section class="sec-s${n % 2 ? ' tint' : ''}"><div class="wrap"><div class="frow${n % 2 ? ' rev' : ''}">
<div class="ftxt rv"><h2>${r.heading}</h2><p>${r.text}</p>${r.bullets.length ? html`<ul class="ticks">${r.bullets.map((b) => html`<li><span>${b}</span></li>`)}</ul>` : ''}</div>
<div class="rv d2">${screenPair(ctx, r.shots, `${page.title}: ${r.heading}`)}</div></div></div></section>`)}
${catalogSection(ctx, slug, page.title)}
<section class="sec-s"><div class="wrap"><div class="facts"><h3>Good to know</h3><ul class="ticks">${page.good.map((g) => html`<li><span>${g}</span></li>`)}</ul>
${page.platform.length ? html`<h3 class="mt26">Windows and Mac</h3><ul class="ticks">${page.platform.map((g) => html`<li><span>${g}</span></li>`)}</ul>` : ''}
<p class="mt18 muted">${page.flow}</p></div></div></section>
${learnSection(ctx, slug)}
<section class="sec-s"><div class="wrap"><div class="sec-head"><h2>Keep going</h2></div><div class="grid3">
${page.related.map((s) => ctx.FEATURES.find((x) => x.slug === s)).filter(Boolean).map((r) => html`<a class="card linkcard" href="/features/${r.slug}/"><h3>${r.title}</h3><p>${r.blurb}</p></a>`)}</div></div></section>
${ctx.cta()}`;
  return {
    path: `/features/${slug}/`, nav: 'features', os: null, alternates: null,
    title: `${page.title} | OpenFrame Studio`, description: page.description, body,
    breadcrumbs: [['Features', '/features/'], [meta.title, `/features/${slug}/`]],
  };
}

function indexPage(ctx) {
  const { html } = ctx;
  const count = Object.values(CATALOG).reduce((n, l) => n + l.length, 0);
  const body = html`
<section class="phero"><div class="wrap"><span class="eyebrow">All features</span><h1>One film, moving forward.</h1>
<p class="lead">From the first scrap of an idea to the final call sheet. Every part of OpenFrame talks to every other part, so you never retype anything.</p>
<p class="tmeta">Each feature is marked for Windows and Mac.</p></div></section>
<section class="sec-s pt30"><div class="wrap">
${ctx.FEATURE_GROUPS.map(([group, items]) => html`<div class="cat-group"><h2>${group}</h2><div class="grid3 mt18">
${items.map(([slug, title]) => html`<a class="card linkcard" href="/features/${slug}/"><h3>${title}</h3><p>${FEATURE_PAGES[slug].lead}</p></a>`)}
</div></div>`)}
</div></section>
${count ? html`<section class="tint sec-s"><div class="wrap"><div class="sec-head"><span class="eyebrow">Complete catalog</span><h2>Every capability at a glance.</h2></div>
${ctx.FEATURE_GROUPS.flatMap(([, items]) => items).map(([slug, title]) => (CATALOG[slug] || []).length ? html`<div class="lib-area"><h2><a href="/features/${slug}/#catalog">${title}</a></h2><ul class="lib">
${CATALOG[slug].map((f) => html`<li><b><a href="/features/${slug}/#${f.id}">${f.name}</a></b>${f.summary} <span class="small-print">${[f.win?.length && 'Windows', (f.mac?.length || f.onMac) && 'Mac'].filter(Boolean).join(' and ')}</span></li>`)}</ul></div>` : '')}
</div></section>` : ''}
${ctx.cta()}`;
  return {
    path: '/features/', nav: 'features', os: null, alternates: null,
    title: 'Features | OpenFrame Studio',
    description: 'Every feature of OpenFrame Studio: Idea Vault, Beat Board, screenplay editor, breakdown, schedule, call sheets, sides, reports, budget and backups.',
    body, breadcrumbs: [['Features', '/features/']],
    jsonld: [{ ...APP(ctx, 'Windows 11, macOS 14 or later', '/features/'), featureList: Object.values(CATALOG).flat().map((f) => f.name) }],
  };
}

export default function (ctx) {
  return [indexPage(ctx), ...ctx.FEATURES.map((f) => featurePage(ctx, f, f.slug))];
}
