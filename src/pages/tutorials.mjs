// Tutorial hub, the two platform indexes and one page per available tutorial per platform.
// Tutorial screens are collected from the tutorial modules by the capture script, so this module lists none.
export const SCREENS = {};

const PLATS = ['win', 'mac'];
const other = (p) => (p === 'win' ? 'mac' : 'win');

// Accepts either a module namespace ({ default }) or the plain object.
const load = (m) => (m && m.default ? m.default : m) || null;

function byPlat(ctx) {
  const out = {};
  for (const p of PLATS) {
    const all = Object.values((ctx.tutorials && ctx.tutorials[p]) || {}).map(load).filter(Boolean);
    out[p] = {
      all: new Map(all.map((t) => [t.id, t])),
      live: all.filter((t) => t.available !== false).sort((a, b) => pathIndex(ctx, a.path) - pathIndex(ctx, b.path) || (a.order ?? 99) - (b.order ?? 99)),
    };
  }
  return out;
}
const pathIndex = (ctx, key) => { const i = ctx.TUTORIAL_PATHS.findIndex(([k]) => k === key); return i < 0 ? 99 : i; };
const urlOf = (ctx, p, id) => `/${ctx.OS[p].slug}/tutorials/${id ? `${id}/` : ''}`;
const logo = (ctx, p) => (p === 'win' ? ctx.ICON.windows : ctx.ICON.apple);
const plural = (n, one, many) => `${n} ${n === 1 ? one : many}`;

// Grow a description from sentences until it reaches 120 characters, never past 160.
function describe(parts) {
  let out = '';
  for (const s of parts) {
    const next = out ? `${out} ${s}` : s;
    if (next.length > 160) continue;
    out = next;
    if (out.length >= 120) break;
  }
  if (out.length > 160) out = `${out.slice(0, 157).replace(/\s+\S*$/, '')}.`;
  return out;
}

function pickTitle(cands) {
  return cands.find((t) => t.length <= 60) || cands[cands.length - 1];
}

// Frame data for one step: image source, size and the target rectangle as fractions of the frame.
function frame(ctx, p, step) {
  const screen = String(step.screen || '').replace(/\.html$/, '');
  const m = (ctx.screens && ctx.screens[`${p}/${screen}`]) || null;
  const w = m ? m.w : 1600;
  const h = m ? m.h : 1000;
  const src = m ? m.src : `/screens/${p}/${screen}`;
  const px = m && step.target && m.targets ? m.targets[step.target] : null;
  const r4 = (n) => Math.round(n * 10000) / 10000;
  const rect = Array.isArray(px) && px.length === 4 ? [r4(px[0] / w), r4(px[1] / h), r4(px[2] / w), r4(px[3] / h)] : null;
  return { w, h, src, rect };
}

function player(ctx, p, t, id) {
  const { html, raw, ICON } = ctx;
  const steps = t.steps || [];
  const frames = steps.map((s) => frame(ctx, p, s));
  const data = steps.map((s, i) => ({ label: s.label, action: s.action, rect: frames[i].rect, text: s.text ?? null, key: s.key ?? null }));
  // JSON inside a script element: escape "<" so the content can never close the element.
  const json = JSON.stringify(data).replace(/</g, '\\u003c');
  const sizes = '(min-width: 1000px) 920px, calc(100vw - 32px)';
  const n = steps.length;
  return html`<div class="player" id="${id}" data-player tabindex="0" aria-roledescription="demonstration" aria-label="Animated demonstration: ${t.title}, ${plural(n, 'step', 'steps')}">
  <div class="pl-stage">
    ${frames.map((f, i) => html`<img class="pl-frame${i === 0 ? ' is-active' : ''}" data-frame="${i}" src="${f.src}-1600.webp" srcset="${f.src}-800.webp 800w, ${f.src}-1600.webp 1600w" sizes="${sizes}" width="${f.w}" height="${f.h}" alt="${steps[i].label}"${raw(i === 0 ? ' fetchpriority="high"' : ' loading="lazy" decoding="async"')}>`)}
    <div class="pl-overlay" aria-hidden="true">
      <div class="pl-ring"></div>
      <div class="pl-ripple"></div>
      <div class="pl-type"><span class="pl-type-text"></span><span class="pl-caret"></span></div>
      <div class="pl-key"></div>
      <div class="pl-badge">Result</div>
      <svg class="pl-cursor" viewBox="0 0 24 24" width="24" height="24" focusable="false"><path d="M5 2.5v17.2l4.6-4.4 3 6.6 3.1-1.4-3-6.5h6.4z" fill="#1d1a2e" stroke="#fff" stroke-width="1.5" stroke-linejoin="round"/></svg>
    </div>
    <span class="pl-tag">Demonstration of the real app</span>
  </div>
  <div class="pl-bar">
    <p class="pl-caption"><span class="pl-count">Step 1 of ${n}</span> <span class="pl-label">${steps[0] ? steps[0].label : ''}</span></p>
    <div class="pl-progress">
      ${steps.map((s, i) => html`<button type="button" class="pl-seg${i === 0 ? ' is-active' : ''}" data-seg="${i}" aria-label="Go to step ${i + 1}: ${s.label}"><span></span></button>`)}
    </div>
    <div class="pl-controls">
      <button type="button" class="pl-btn" data-act="prev" aria-label="Previous step">${ICON.prev}</button>
      <button type="button" class="pl-btn pl-play" data-act="play" aria-label="Play">${ICON.play}${ICON.pause}</button>
      <button type="button" class="pl-btn" data-act="next" aria-label="Next step">${ICON.next}</button>
      <button type="button" class="pl-btn" data-act="replay" aria-label="Replay from the start">${ICON.replay}</button>
      <button type="button" class="pl-btn pl-fs" data-act="fs" aria-label="Full screen">${ICON.expand}${ICON.shrink}</button>
    </div>
  </div>
  <p class="pl-status visually-hidden" aria-live="polite"></p>
  <script type="application/json" class="player-data">${raw(json)}</script>
</div>`;
}

function card(ctx, p, t) {
  const { html, ICON } = ctx;
  return html`<a class="card t-card" href="${urlOf(ctx, p, t.id)}">
  <h3>${t.title}</h3>
  <p>${t.purpose}</p>
  <ul class="t-meta t-meta-sm">
    <li>${ICON.level}${t.difficulty}</li>
    <li>${ICON.clock}${plural(t.minutes, 'minute', 'minutes')}</li>
    <li>${plural((t.steps || []).length, 'step', 'steps')}</li>
  </ul>
</a>`;
}

function hub(ctx, T) {
  const { html, OS, TUTORIAL_PATHS, ICON } = ctx;
  const entry = (p) => html`<a class="t-entry" href="${urlOf(ctx, p)}">
  <span class="t-entry-logo">${logo(ctx, p)}</span>
  <span class="t-entry-text"><strong>${OS[p].name} tutorials</strong><span>${plural(T[p].live.length, 'tutorial', 'tutorials')} for ${OS[p].long}</span></span>
  ${ICON.arrow}
</a>`;
  const list = (p) => html`<section class="t-hub-list" aria-labelledby="t-list-${p}">
  <h2 id="t-list-${p}">${logo(ctx, p)} All ${OS[p].name} tutorials</h2>
  ${TUTORIAL_PATHS.map(([key, name]) => {
    const items = T[p].live.filter((t) => t.path === key);
    return items.length ? html`<h3>${name}</h3><ul class="t-links">${items.map((t) => html`<li><a href="${urlOf(ctx, p, t.id)}">${t.title}</a></li>`)}</ul>` : '';
  })}
</section>`;
  const body = html`<div class="wrap t-hub">
  <h1>Tutorials</h1>
  <p class="lead">Short, step by step lessons built from screens of the real app. Each one shows the exact buttons to click and what you will see. Pick your computer to begin.</p>
  ${ctx.osBlocks({
    win: html`<p class="t-hint">You appear to be on Windows. Start with the Windows tutorials, or switch to macOS at any time.</p>`,
    mac: html`<p class="t-hint">You appear to be on a Mac. Start with the macOS tutorials, or switch to Windows at any time.</p>`,
  })}
  <div class="t-entries">${entry('win')}${entry('mac')}</div>
  <section class="t-paths" aria-labelledby="t-paths-h">
    <h2 id="t-paths-h">Learning paths</h2>
    <p>The tutorials are grouped into paths. Follow a path from the top, or open any tutorial on its own.</p>
    <ol class="t-path-list">
      ${TUTORIAL_PATHS.map(([key, name, blurb]) => html`<li><strong>${name}</strong><span>${blurb}</span><span class="t-path-count">${OS.win.name}: ${T.win.live.filter((t) => t.path === key).length}. ${OS.mac.name}: ${T.mac.live.filter((t) => t.path === key).length}.</span></li>`)}
    </ol>
  </section>
  <div class="grid2 t-hub-lists">${list('win')}${list('mac')}</div>
</div>`;
  return {
    path: '/tutorials/',
    title: 'Tutorials | OpenFrame Studio',
    description: describe([
      'Step by step OpenFrame Studio tutorials for Windows 11 and macOS, built from screens of the real app.',
      'Learn writing, planning and production paths.',
      'Learn at your own pace.',
    ]),
    body,
    nav: 'tutorials',
    os: null,
    alternates: null,
    breadcrumbs: [['Tutorials', '/tutorials/']],
  };
}

function index(ctx, T, p) {
  const { html, raw, OS, TUTORIAL_PATHS } = ctx;
  const o = other(p);
  // Tutorials that exist on the other platform but not here; site.js shows one when ?missing=<id> is in the URL.
  const missing = {};
  for (const t of T[o].live) {
    const here = T[p].all.get(t.id);
    if (!here || here.available === false) {
      missing[t.id] = { title: t.title, reason: (here && here.reason) || `This tutorial has not been made for ${OS[p].name} yet.` };
    }
  }
  const missingJson = JSON.stringify(missing).replace(/</g, '\\u003c');
  const off = [...T[p].all.values()].filter((t) => t.available === false);
  const body = html`<div class="wrap t-index">
  <h1>${OS[p].name} tutorials</h1>
  <p class="lead">${plural(T[p].live.length, 'tutorial', 'tutorials')} for OpenFrame Studio on ${OS[p].long}. Each one uses screens of the ${OS[p].name} app.</p>
  <p class="t-missing note" id="t-missing" hidden></p>
  <script type="application/json" id="t-missing-data">${raw(missingJson)}</script>
  ${TUTORIAL_PATHS.map(([key, name, blurb]) => {
    const items = T[p].live.filter((t) => t.path === key);
    return items.length ? html`<section class="t-group" aria-labelledby="t-g-${key}">
    <h2 id="t-g-${key}">${name}</h2>
    <p class="t-group-blurb">${blurb}</p>
    <div class="grid3">${items.map((t) => card(ctx, p, t))}</div>
  </section>` : '';
  })}
  ${off.length ? html`<section class="t-group" aria-labelledby="t-g-off">
    <h2 id="t-g-off">Not available on ${OS[p].name}</h2>
    <ul class="t-off">${off.map((t) => html`<li><strong>${t.title || t.id}</strong>${t.reason ? html`: ${t.reason}` : ''}</li>`)}</ul>
  </section>` : ''}
  <p class="t-switch"><a href="${urlOf(ctx, o)}">${logo(ctx, o)} See the ${OS[o].name} tutorials</a></p>
</div>`;
  return {
    path: urlOf(ctx, p),
    title: `${OS[p].name} tutorials | OpenFrame Studio`,
    description: describe([
      `Every OpenFrame Studio tutorial for ${OS[p].long}, grouped by learning path.`,
      'Each one lists its difficulty, time and steps.',
      `Screens come from the real ${OS[p].name} app.`,
    ]),
    body,
    nav: 'tutorials',
    os: p,
    alternates: { win: urlOf(ctx, 'win'), mac: urlOf(ctx, 'mac') },
    breadcrumbs: [['Tutorials', '/tutorials/'], [OS[p].name, urlOf(ctx, p)]],
  };
}

function detail(ctx, T, p, t) {
  const { html, OS, ICON, SITE_URL } = ctx;
  const o = other(p);
  const twin = T[o].all.get(t.id);
  const twinLive = twin && twin.available !== false;
  const steps = t.steps || [];
  const pid = `player-${t.id}`;
  const first = steps[0] ? frame(ctx, p, steps[0]) : null;
  const ogImage = first ? `${first.src}-1600.webp` : undefined;
  const liveOf = (id) => { const x = T[p].all.get(id); return x && x.available !== false ? x : null; };
  const related = (t.related || []).map(liveOf).filter(Boolean);
  const next = t.next ? liveOf(t.next) : null;
  const url = urlOf(ctx, p, t.id);

  const body = html`<article class="wrap t-detail">
  <header class="t-head">
    <h1>${t.title}</h1>
    <p class="lead">${t.purpose}</p>
    <ul class="t-meta">
      <li>${logo(ctx, p)}${OS[p].long}</li>
      <li>${ICON.level}${t.difficulty}</li>
      <li>${ICON.clock}${plural(t.minutes, 'minute', 'minutes')}</li>
      <li>${plural(steps.length, 'step', 'steps')}</li>
    </ul>
  </header>
  <div class="t-before">
    ${(t.prerequisites || []).length ? html`<section aria-labelledby="t-pre"><h2 id="t-pre">Before you start</h2><ul class="ticks">${t.prerequisites.map((x) => html`<li>${x}</li>`)}</ul></section>` : ''}
    ${(t.objectives || []).length ? html`<section aria-labelledby="t-obj"><h2 id="t-obj">What you will do</h2><ul class="ticks">${t.objectives.map((x) => html`<li>${x}</li>`)}</ul></section>` : ''}
  </div>
  ${steps.length ? player(ctx, p, t, pid) : ''}
  <p class="t-demo-note">${ICON.info}<span>This is a demonstration made from screens of the real ${OS[p].name} app. It is a recording you can watch, not a working copy of the app.</span></p>
  <section aria-labelledby="t-steps-h">
    <h2 id="t-steps-h">Steps</h2>
    <ol class="t-steps" data-steps-for="${pid}">
      ${steps.map((s, i) => html`<li data-step="${i}">
        <button type="button" class="t-step-num" aria-label="Show step ${i + 1} in the demonstration">${i + 1}</button>
        <div><h3>${s.label}</h3>${s.body ? html`<p>${s.body}</p>` : ''}</div>
      </li>`)}
    </ol>
  </section>
  ${t.result ? html`<section aria-labelledby="t-res"><h2 id="t-res">Result</h2><p>${t.result}</p></section>` : ''}
  ${(t.mistakes || []).length ? html`<section aria-labelledby="t-mis"><h2 id="t-mis">Common mistakes</h2><ul>${t.mistakes.map((x) => html`<li>${x}</li>`)}</ul></section>` : ''}
  ${(t.troubleshooting || []).length ? html`<section aria-labelledby="t-tr"><h2 id="t-tr">Troubleshooting</h2><dl class="t-trouble">${t.troubleshooting.map(([q, a]) => html`<dt>${q}</dt><dd>${a}</dd>`)}</dl></section>` : ''}
  <div class="t-after">
    ${next ? html`<a class="btn btn-primary" href="${urlOf(ctx, p, next.id)}">Next: ${next.title} ${ICON.arrow}</a>` : ''}
    ${twinLive
      ? html`<a class="btn btn-ghost" href="${urlOf(ctx, o, t.id)}">${logo(ctx, o)} See this tutorial on ${OS[o].name}</a>`
      : html`<p class="t-twin-note">Not available on ${OS[o].name}: ${(twin && twin.reason) || `this tutorial has not been made for ${OS[o].name} yet.`}</p>`}
  </div>
  ${related.length ? html`<section aria-labelledby="t-rel"><h2 id="t-rel">Related tutorials</h2><div class="grid3">${related.map((r) => card(ctx, p, r))}</div></section>` : ''}
</article>`;

  const jsonld = [
    { '@context': 'https://schema.org', '@type': 'TechArticle', headline: t.title, description: t.purpose,
      proficiencyLevel: t.difficulty, url: `${SITE_URL}${url}`, ...(ogImage ? { image: `${SITE_URL}${ogImage}` } : {}),
      about: { '@type': 'SoftwareApplication', name: 'OpenFrame Studio', operatingSystem: OS[p].long } },
    { '@context': 'https://schema.org', '@type': 'HowTo', name: t.title, description: t.purpose,
      totalTime: `PT${t.minutes}M`, ...(ogImage ? { image: `${SITE_URL}${ogImage}` } : {}),
      step: steps.map((s, i) => ({ '@type': 'HowToStep', position: i + 1, name: s.label, text: s.body || s.label, url: `${SITE_URL}${url}#t-steps-h` })) },
  ];
  return {
    path: url,
    title: pickTitle([
      `${t.title} on ${OS[p].name} | OpenFrame Studio tutorial`,
      `${t.title} on ${OS[p].name} | OpenFrame Studio`,
      `${t.title} on ${OS[p].name}`,
    ]),
    description: describe([
      t.purpose,
      `A ${t.minutes} minute ${String(t.difficulty).toLowerCase()} tutorial for OpenFrame Studio on ${OS[p].long}.`,
      `${plural(steps.length, 'step', 'steps')}, shown on real app screens.`,
      `For ${OS[p].name}.`,
      'Step by step.',
    ]),
    body,
    nav: 'tutorials',
    os: p,
    alternates: twinLive ? { [p]: url, [o]: urlOf(ctx, o, t.id) } : null,
    breadcrumbs: [['Tutorials', '/tutorials/'], [OS[p].name, urlOf(ctx, p)], [t.title, url]],
    jsonld,
    ogImage,
  };
}

export default function tutorials(ctx) {
  const T = byPlat(ctx);
  const pages = [hub(ctx, T)];
  for (const p of PLATS) {
    pages.push(index(ctx, T, p));
    for (const t of T[p].live) pages.push(detail(ctx, T, p, t));
  }
  return pages;
}
