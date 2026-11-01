// Builds dist/search-index.json: one entry per tutorial (both platforms), feature page and FAQ answer.
// Called once from scripts/build.mjs. The browser loads this single same-origin file the first time search opens.
import fs from 'node:fs';
import path from 'node:path';
import { FEATURES, PARTS, OS } from '../src/content/nav.mjs';
import { FEATURE_PAGES } from '../src/content/features.mjs';
import { CATALOG } from '../src/content/catalog.mjs';
import { FAQ } from '../src/content/copy.mjs';

// Same wording rule as the tutorial pages: the manual says "chapter", the website says "tutorials".
const web = (t) => String(t ?? '').replace(/\bthis chapter\b/gi, 'these tutorials').replace(/\bthe ([A-Z][\w ,&]*?) chapter\b/g, 'the $1 tutorials').replace(/\bchapters?\b/gi, 'tutorials').replace(/\b(this|the) manual\b/gi, 'the tutorials');
const plain = (t) => web(t).replace(/\[([^\]]+)\]\([^)]*\)/g, '$1').replace(/\*\*|`/g, '').replace(/\s+/g, ' ').trim();
// Cut at the end of a sentence when one falls near the limit, else at a word.
const short = (t, n) => {
  if (t.length <= n) return t;
  const cut = t.slice(0, n), dot = cut.lastIndexOf('. ');
  return dot > n * 0.5 ? cut.slice(0, dot + 1) : cut.replace(/\s+\S*$/, '') + '...';
};
const slug = (s) => s.toLowerCase().replace(/\W+/g, '-');
// Same id rule as faqItem in src/pages/info.mjs, so an answer opens at its own question.
const faqId = (q) => 'q-' + slug(q).replace(/^-+|-+$/g, '').slice(0, 60).replace(/-+$/, '');

export function buildSearchIndex(manual) {
  const out = [];
  for (const os of ['win', 'mac']) {
    for (const a of manual[os]) for (const w of a.workflows) {
      const steps = w.steps.map((s) => short(plain(`${s.do} ${s.explain || ''}`), 190));
      steps.push(plain(w.result), ...(w.cautions || []).map((c) => short(plain(c), 200)));
      out.push({ type: 'tutorial', os, part: a.part, area: a.area, title: plain(w.title), summary: plain(w.purpose), steps,
        url: `/${OS[os].slug}/tutorials/${w.id}/`, keywords: w.id.replace(/-/g, ' ') });
    }
  }
  for (const f of FEATURES) {
    const p = FEATURE_PAGES[f.slug];
    const steps = [...p.rows.map((r) => plain(`${r.heading}. ${r.text} ${r.bullets.join('. ')}`)),
      ...(CATALOG[f.slug] || []).map((c) => short(plain(`${c.name}: ${c.summary} ${c.does}`), 220)), ...p.good.map(plain)];
    out.push({ type: 'feature', os: 'both', part: f.group, area: 'Features', title: f.title, summary: plain(p.lead), steps,
      url: `/features/${f.slug}/`, keywords: plain(`${p.h1} ${f.blurb}`) });
  }
  for (const [group, items] of FAQ) for (const [q, a] of items) {
    out.push({ type: 'answer', os: 'both', part: '', area: 'FAQ', title: q, summary: plain(a), steps: [], url: `/faq/#${faqId(q)}`, keywords: group });
  }
  return { parts: PARTS.map(([k, t]) => [k, t]), entries: out };
}

export function writeSearchIndex(dist, manual) {
  const index = buildSearchIndex(manual);
  fs.writeFileSync(path.join(dist, 'search-index.json'), JSON.stringify(index));
  return index.entries.length;
}
