// Tutorials: /tutorials/ (choose a platform), /<platform>/tutorials/ (every tutorial by manual part and area),
// and /<platform>/tutorials/<id>/ (one written, step-by-step tutorial on the real screens). Source: the manual areas in src/content/manual.
import { trimWords, fitTitle } from '../lib/seo.mjs';
const SLUG = { win: 'windows', mac: 'mac' };
const NAME = { win: 'Windows', mac: 'Mac' };

function thumb(ctx, plat, name) {
  const p = ctx.cat[plat].get(name) && ctx.pic(plat, name);
  if (!p) return '';
  return ctx.html`<img data-mock src="${p.c720}" alt="" loading="lazy" decoding="async" width="720" height="${Math.round(720 * p.h / p.w)}">`;
}

// The content is written once for the internal manuals; on the website a "chapter" is the area's tutorials.
const web = (t) => String(t ?? '').replace(/\bthis chapter\b/gi, 'these tutorials').replace(/\bthe ([A-Z][\w ,&]*?) chapter\b/g, 'the $1 tutorials').replace(/\bchapters?\b/gi, 'tutorials').replace(/\b(this|the) manual\b/gi, 'the tutorials');

export default function tutorials(ctx) {
  const { html, mock, PARTS, manual, cat } = ctx;
  const md = (t) => ctx.md(web(t));
  const pages = [];
  const all = {}, ordered = {};
  for (const plat of ['win', 'mac']) all[plat] = manual[plat].flatMap((a) => a.workflows.map((w) => ({ ...w, area: a })));
  // The reading order of the platform index (by manual part, then area): previous and next follow it, so a tutorial that is first or last in its area still points on.
  for (const plat of ['win', 'mac']) ordered[plat] = PARTS.flatMap(([k]) => manual[plat].filter((a) => a.part === k).flatMap((a) => a.workflows.map((w) => all[plat].find((x) => x.id === w.id))));

  // ---------- /tutorials/ ----------
  pages.push({
    path: '/tutorials/', nav: 'tutorials',
    title: 'Tutorials for Windows and Mac | OpenFrame Studio',
    description: 'Step-by-step written tutorials for OpenFrame Studio on Windows and on the Mac, on the real app screens of each platform, from the first start to backups.',
    breadcrumbs: [['Tutorials', '/tutorials/']],
    body: html`<section class="phero"><div class="wrap"><span class="eyebrow">Tutorials</span><h1>Everything you can do, step by step.</h1>
<p class="lead">Written, step-by-step tutorials on the real screens. Pick your platform.</p>
<div class="hero-cta"><a class="btn btn-primary btn-lg" href="/windows/tutorials/">Windows tutorials</a><a class="btn btn-dark btn-lg" href="/mac/tutorials/">Mac tutorials</a></div></div></section>
<section class="sec-s pt30"><div class="wrap narrow"><div class="facts"><h2>How the tutorials work</h2><ul class="ticks">
<li><span>Each tutorial lists what you need first, then the steps in order, each with the screen you see at that moment.</span></li>
<li><span>Use the controls at the top of the page to show the app screens in light or dark. The website theme does not change them.</span></li></ul></div></div></section>`,
  });

  for (const plat of ['win', 'mac']) {
    const base = `/${SLUG[plat]}/tutorials/`;
    const other = plat === 'win' ? 'mac' : 'win';
    const card = (w) => html`<a class="tut" href="${base}${w.id}/"><div class="thumb">${w.steps[0] ? thumb(ctx, plat, w.steps[0].screen) : ''}</div><div class="tb"><span class="meta">${w.level || 'Beginner'} &middot; ${w.steps.length} steps</span><h3>${w.title}</h3><p>${web(w.purpose)}</p></div></a>`;
    const partId = (k) => `part-${k.toLowerCase().replace(/\W+/g, '-')}`;
    // ---------- platform index ----------
    pages.push({
      path: base, os: plat, nav: 'tutorials', alternates: { [plat]: base, [other]: `/${SLUG[other]}/tutorials/` },
      title: `${NAME[plat]} tutorials | OpenFrame Studio`,
      description: `Explore OpenFrame Studio on ${NAME[plat]} with step-by-step tutorials, from the first start to backups, each on the real ${NAME[plat]} app screens.`,
      breadcrumbs: [['Tutorials', '/tutorials/'], [NAME[plat], base]],
      body: html`<section class="phero"><div class="wrap"><span class="eyebrow">${NAME[plat]} tutorials</span><h1>Everything you can do, step by step.</h1>
<p class="lead">Written, step-by-step tutorials on the real ${NAME[plat]} screens. Pick a task and follow it.</p>
<div class="fchips">${PARTS.filter(([k]) => manual[plat].some((a) => a.part === k)).map(([k, t]) => html`<a class="fchip" href="#${partId(k)}">${t}</a>`)}</div>
<div class="tf" data-tfilter><div class="tf-in"><svg viewBox="0 0 24 24" aria-hidden="true" focusable="false" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="7"/><path d="M20 20l-3.5-3.5"/></svg><input id="tf-q" type="search" autocomplete="off" spellcheck="false" aria-label="Filter these tutorials" placeholder="Filter these tutorials"></div><a class="tf-adv" href="/search/" data-tf-adv>Advanced search</a></div>
<p class="tf-none" hidden>No tutorial here matches. <a href="/search/" data-tf-adv>Search everything</a></p></div></section>
<section class="sec-s pt30"><div class="wrap">
${PARTS.map(([k, t, d]) => {
        const areas = manual[plat].filter((a) => a.part === k);
        if (!areas.length) return '';
        return html`<div class="tut-part" id="${partId(k)}"><h2>${t}</h2><p>${d}</p>
${areas.map((a) => html`<h3 class="mt26">${a.area}</h3><div class="tgrid">${a.workflows.map((w) => card({ ...w, area: a }))}</div>`)}</div>`;
      })}
</div></section>`,
    });

    // ---------- one page per tutorial ----------
    for (const w of all[plat]) {
      const a = w.area;
      const k = ordered[plat].findIndex((x) => x.id === w.id);
      const prev = ordered[plat][k - 1], next = ordered[plat][k + 1];
      const hop = (x, rel, label) => html`<a class="tn-${rel}" rel="${rel}" href="${base}${x.id}/"><span>${label}</span><b>${x.title}</b>${x.area !== a ? html`<small>${x.area.area}</small>` : ''}</a>`;
      const counterpart = all[other].find((x) => x.id === w.id);
      const related = (w.related || []).map((r) => all[plat].find((x) => x.id === r)).filter(Boolean);
      // Two tutorials can share a title (for example in Budget and in Reports): the area tells them apart in search results.
      const dup = all[plat].some((x) => x !== w && x.title === w.title);
      const title = `${w.title}${dup ? ` (${a.area})` : ""} on ${NAME[plat]}`;
      pages.push({
        path: `${base}${w.id}/`, os: plat, nav: 'tutorials',
        alternates: { [plat]: `${base}${w.id}/`, [other]: counterpart ? `/${SLUG[other]}/tutorials/${w.id}/` : `/${SLUG[other]}/tutorials/` },
        title: title.length > 60 ? trimWords(title, 60) : fitTitle(title),
        description: trimWords(`${w.purpose} A step-by-step ${a.area} tutorial for OpenFrame Studio on ${NAME[plat]}.`, 160),
        breadcrumbs: [['Tutorials', '/tutorials/'], [NAME[plat], base], [w.title, `${base}${w.id}/`]],
        body: html`<section class="phero"><div class="wrap"><p class="crumbs"><a href="${base}">${NAME[plat]} tutorials</a> &rsaquo; ${a.area}</p>
<span class="eyebrow">${a.area}</span><h1>${w.title}</h1><p class="lead">${web(w.purpose)}</p>
<p class="tmeta"><span>${w.level || 'Beginner'}</span><span>${w.steps.length} steps</span><span>${NAME[plat]}</span></p></div></section>
<section class="sec-s pt30"><div class="wrap">
${w.prerequisites?.length ? html`<div class="tbox"><h2>Before you start</h2><ul>${w.prerequisites.map((p) => html`<li>${md(p)}</li>`)}</ul></div>` : ''}
<h2 class="sr">Steps</h2>
<ol class="tsteps">${w.steps.map((s) => html`<li class="tstep"><div class="tx"><p class="do">${md(s.do)}</p>${s.explain ? html`<p>${md(s.explain)}</p>` : ''}</div><div>${mock(plat, s.screen)}</div></li>`)}</ol>
<div class="tbox"><h2>What you get</h2><p>${md(w.result)}</p></div>
${w.cautions?.length ? html`<div class="tbox warn"><h2>Watch out for</h2><ul>${w.cautions.map((c) => html`<li>${md(c)}</li>`)}</ul></div>` : ''}
${related.length ? html`<div class="tbox"><h2>Related tutorials</h2><ul>${related.map((r) => html`<li><a href="${base}${r.id}/">${r.title}</a></li>`)}</ul></div>` : ''}
<nav class="tnext" aria-label="Previous and next tutorial">${prev ? hop(prev, 'prev', 'Previous') : ''}${next ? hop(next, 'next', 'Next') : ''}</nav>
<p class="tmeta"><a href="${base}">All ${NAME[plat]} tutorials</a></p>
</div></section>`,
      });
    }

  }
  return pages;
}
