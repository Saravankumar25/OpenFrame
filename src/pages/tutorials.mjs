// Tutorials: /tutorials/ (choose a platform), /<platform>/tutorials/ (every tutorial by manual part and area),
// and /<platform>/tutorials/<id>/ (one written, step-by-step tutorial on the real screens). Source: the manual areas in src/content/manual.
const SLUG = { win: 'windows', mac: 'mac' };
const NAME = { win: 'Windows', mac: 'Mac' };

function thumb(ctx, plat, name) {
  const s = ctx.cat[plat].get(name);
  if (!s) return '';
  const src = plat === 'mac' ? `/mockups/mac/light/assets/thumbs/${s.file.replace('.html', '.png')}` : `/mockups/win/thumbs/light/${s.file.replace('.html', '.jpg')}`;
  return ctx.html`<img data-mock src="${src}" alt="" loading="lazy" decoding="async" width="480" height="${Math.round(480 * s.height / s.width)}">`;
}

// The content is written once for the internal manuals; on the website a "chapter" is the area's tutorials.
const web = (t) => String(t ?? '').replace(/\bthis chapter\b/gi, 'these tutorials').replace(/\bthe ([A-Z][\w ,&]*?) chapter\b/g, 'the $1 tutorials').replace(/\bchapters?\b/gi, 'tutorials').replace(/\b(this|the) manual\b/gi, 'the tutorials');

export default function tutorials(ctx) {
  const { html, mock, PARTS, manual, cat } = ctx;
  const md = (t) => ctx.md(web(t));
  const pages = [];
  const all = {};
  for (const plat of ['win', 'mac']) all[plat] = manual[plat].flatMap((a) => a.workflows.map((w) => ({ ...w, area: a })));

  // ---------- /tutorials/ ----------
  pages.push({
    path: '/tutorials/', nav: 'tutorials',
    title: 'Tutorials for Windows and Mac | OpenFrame Studio',
    description: 'Step-by-step written tutorials for OpenFrame Studio on Windows and on the Mac, on the real app screens of each platform, from the first start to backups.',
    breadcrumbs: [['Tutorials', '/tutorials/']],
    body: html`<section class="phero"><div class="wrap"><span class="eyebrow">Tutorials</span><h1>Everything you can do, step by step.</h1>
<p class="lead">Written, step-by-step tutorials on the real screens. Pick your platform.</p>
<div class="hero-cta"><a class="btn btn-primary btn-lg" href="/windows/tutorials/">Windows tutorials</a><a class="btn btn-dark btn-lg" href="/mac/tutorials/">Mac tutorials</a></div></div></section>
<section class="sec-s pt30"><div class="wrap narrow"><div class="facts"><h3>How the tutorials work</h3><ul class="ticks">
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
<div class="fchips">${PARTS.filter(([k]) => manual[plat].some((a) => a.part === k)).map(([k, t]) => html`<a class="fchip" href="#${partId(k)}">${t}</a>`)}</div></div></section>
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
      const k = a.workflows.findIndex((x) => x.id === w.id);
      const prev = a.workflows[k - 1], next = a.workflows[k + 1];
      const counterpart = all[other].find((x) => x.id === w.id);
      const related = (w.related || []).map((r) => all[plat].find((x) => x.id === r)).filter(Boolean);
      const title = `${w.title} on ${NAME[plat]}`;
      pages.push({
        path: `${base}${w.id}/`, os: plat, nav: 'tutorials',
        alternates: { [plat]: `${base}${w.id}/`, [other]: counterpart ? `/${SLUG[other]}/tutorials/${w.id}/` : `/${SLUG[other]}/tutorials/` },
        title: title.length > 60 ? w.title.slice(0, 60) : title,
        description: `${w.purpose} A step-by-step OpenFrame Studio tutorial for ${NAME[plat]}.`.slice(0, 165),
        breadcrumbs: [['Tutorials', '/tutorials/'], [NAME[plat], base], [w.title, `${base}${w.id}/`]],
        jsonld: [{ '@context': 'https://schema.org', '@type': 'HowTo', name: w.title, description: w.purpose,
          step: w.steps.map((s, n) => ({ '@type': 'HowToStep', position: n + 1, text: s.do })) }],
        body: html`<section class="phero"><div class="wrap"><p class="crumbs"><a href="${base}">${NAME[plat]} tutorials</a> &rsaquo; ${a.area}</p>
<span class="eyebrow">${a.area}</span><h1>${w.title}</h1><p class="lead">${web(w.purpose)}</p>
<p class="tmeta"><span>${w.level || 'Beginner'}</span><span>${w.steps.length} steps</span><span>${NAME[plat]}</span></p></div></section>
<section class="sec-s pt30"><div class="wrap">
${w.prerequisites?.length ? html`<div class="tbox"><h2>Before you start</h2><ul>${w.prerequisites.map((p) => html`<li>${md(p)}</li>`)}</ul></div>` : ''}
<ol class="tsteps">${w.steps.map((s) => html`<li class="tstep"><div class="tx"><h3>${md(s.do)}</h3>${s.explain ? html`<p>${md(s.explain)}</p>` : ''}</div><div>${mock(plat, s.screen)}</div></li>`)}</ol>
<div class="tbox"><h2>What you get</h2><p>${md(w.result)}</p></div>
${w.cautions?.length ? html`<div class="tbox warn"><h2>Watch out for</h2><ul>${w.cautions.map((c) => html`<li>${md(c)}</li>`)}</ul></div>` : ''}
${related.length ? html`<div class="tbox"><h2>Related tutorials</h2><ul>${related.map((r) => html`<li><a href="${base}${r.id}/">${r.title}</a></li>`)}</ul></div>` : ''}
<p class="tmeta">${prev ? html`<a href="${base}${prev.id}/">&larr; ${prev.title}</a>` : ''}<a href="${base}">All ${NAME[plat]} tutorials</a>${next ? html`<a href="${base}${next.id}/">${next.title} &rarr;</a>` : ''}</p>
</div></section>`,
      });
    }

  }
  return pages;
}
