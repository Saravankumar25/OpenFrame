// /features/ index and one page per feature. Content lives in src/content/features.mjs.
import { FEATURE_PAGES, SCREENS as FEATURE_SCREENS } from '../content/features.mjs';

export const SCREENS = FEATURE_SCREENS;

const human = (id) => id.charAt(0).toUpperCase() + id.slice(1).replace(/-/g, ' ');

// Real screen for the visitor's platform. A row without a Mac screen shows the Windows app to everyone, labelled.
export function pairShot(ctx, s, alt, opts = {}) {
  const win = ctx.shot('win', s.win, { ...opts, crop: s.crop, alt: `${alt}, in OpenFrame Studio for Windows` });
  if (!s.mac) {
    return ctx.osBlocks({ win, mac: ctx.shot('win', s.win, { ...opts, crop: s.crop, alt: `${alt}, in OpenFrame Studio for Windows`, caption: 'Shown in the Windows app. The Mac app works the same way.' }) });
  }
  return ctx.osBlocks({ win, mac: ctx.shot('mac', s.mac, { ...opts, alt: `${alt}, in OpenFrame Studio for Mac` }) });
}

// Tutorials that exist as pages for this platform. Without a catalog, every id is linked.
export function tutorialsFor(ctx, plat, ids) {
  const list = ctx.tutorials?.[plat];
  if (!list) return ids.map((id) => ({ id, title: human(id) }));
  const arr = Array.isArray(list) ? list : Object.values(list);
  return ids.map((id) => arr.find((t) => t && t.id === id)).filter((t) => t && t.available !== false);
}

function learnBlock(ctx, ids) {
  const list = (plat) => {
    const ts = tutorialsFor(ctx, plat, ids);
    const base = plat === 'win' ? '/windows/tutorials/' : '/mac/tutorials/';
    if (!ts.length) return ctx.html`<p>Step-by-step guides are in <a href="${base}">the ${plat === 'win' ? 'Windows' : 'Mac'} tutorials</a>.</p>`;
    return ctx.html`<ul class="mk-learn">${ts.map((t) => ctx.html`<li><a href="${base}${t.id}/">${t.title || human(t.id)}${ctx.ICON.arrow}</a></li>`)}</ul>`;
  };
  return ctx.osBlocks({ win: list('win'), mac: list('mac') });
}

function featurePage(ctx, f, slug) {
  const { html } = ctx;
  const meta = ctx.FEATURES.find((x) => x.slug === slug);
  const page = FEATURE_PAGES[slug];
  const i = ctx.FEATURES.indexOf(meta);
  const prev = ctx.FEATURES[i - 1], next = ctx.FEATURES[i + 1];
  const body = html`
<section class="mk-fhero">
  <div class="wrap">
    <div class="mk-fhero-text">
      <h1>${page.h1}</h1>
      <p class="lead">${page.lead}</p>
    </div>
    <div class="mk-fhero-shot">${pairShot(ctx, page.hero, page.title, { eager: true })}</div>
  </div>
</section>
${page.rows.map((r, n) => html`
<section class="section${n % 2 ? ' tint' : ''}">
  <div class="wrap mk-row${n % 2 ? ' mk-row-flip' : ''}">
    <div class="mk-row-text">
      <h2>${r.heading}</h2>
      <p>${r.text}</p>
      ${r.bullets.length ? html`<ul class="ticks">${r.bullets.map((b) => html`<li>${b}</li>`)}</ul>` : ''}
    </div>
    <div class="mk-row-shot">${pairShot(ctx, r.shots, `${page.title}: ${r.heading}`)}</div>
  </div>
</section>`)}
<section class="section">
  <div class="wrap mk-facts">
    <div>
      <h2>How it fits</h2>
      <p>${page.flow}</p>
      <p class="mk-stage-nav">
        ${prev ? html`<a href="/features/${prev.slug}/"><span>Before</span> ${prev.title}</a>` : ''}
        ${next ? html`<a href="/features/${next.slug}/"><span>After</span> ${next.title}</a>` : ''}
      </p>
    </div>
    <div>
      <h2>Good to know</h2>
      <ul class="mk-know">${page.good.map((g) => html`<li>${ctx.ICON.info}<span>${g}</span></li>`)}</ul>
      ${page.platform.length ? html`<h3>Windows and Mac</h3><ul class="mk-know">${page.platform.map((g) => html`<li>${ctx.ICON.laptop}<span>${g}</span></li>`)}</ul>` : ''}
    </div>
  </div>
</section>
<section class="section tint">
  <div class="wrap mk-facts">
    <div id="learn">
      <h2>Learn it</h2>
      <p>Short tutorials made from the real screens, for your platform.</p>
      ${learnBlock(ctx, page.tutorials)}
    </div>
    <div>
      <h2>Related features</h2>
      <ul class="mk-related">${page.related.map((s) => ctx.FEATURES.find((x) => x.slug === s)).filter(Boolean).map((r) => html`
        <li><a href="/features/${r.slug}/"><strong>${r.title}</strong><span>${r.blurb}</span></a></li>`)}</ul>
    </div>
  </div>
</section>
${ctx.cta()}`;
  return {
    path: `/features/${slug}/`,
    title: `${page.title} | OpenFrame Studio`,
    description: page.description,
    body,
    nav: 'features',
    os: null,
    alternates: null,
    breadcrumbs: [['Features', '/features/'], [meta.title, `/features/${slug}/`]],
    ogImage: `/screens/win/${page.hero.win}-1200.jpg`,
  };
}

function indexPage(ctx) {
  const { html } = ctx;
  const body = html`
<section class="mk-fhero mk-index-hero">
  <div class="wrap narrow">
    <h1>Everything the film needs, in one project.</h1>
    <p class="lead">OpenFrame Studio follows the work from the first idea to the call sheet. Each part reads from the one before it, so nothing is typed twice.</p>
  </div>
</section>
${ctx.FEATURE_GROUPS.map(([group, items], n) => html`
<section class="section${n % 2 ? ' tint' : ''} mk-group">
  <div class="wrap">
    <h2>${group}</h2>
    <ul class="mk-cards">${items.map(([slug, title, blurb]) => html`
      <li><a class="mk-card" href="/features/${slug}/">
        <span class="mk-card-shot">${pairShot(ctx, FEATURE_PAGES[slug].hero, title)}</span>
        <strong>${title}</strong>
        <span>${blurb}</span>
      </a></li>`)}</ul>
  </div>
</section>`)}
${ctx.cta()}`;
  return {
    path: '/features/',
    title: 'Features | OpenFrame Studio',
    description: 'Every feature of OpenFrame Studio: Idea Vault, Beat Board, screenplay editor, breakdown, schedule, call sheets, sides, reports, budget and backups.',
    body,
    nav: 'features',
    os: null,
    alternates: null,
    breadcrumbs: [['Features', '/features/']],
    ogImage: `/screens/win/${FEATURE_PAGES.screenplay.hero.win}-1200.jpg`,
  };
}

export default function (ctx) {
  return [indexPage(ctx), ...ctx.FEATURES.map((f) => featurePage(ctx, f, f.slug))];
}
