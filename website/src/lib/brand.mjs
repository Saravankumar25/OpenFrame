// Brand rename for text taken from the app screens (the desktop apps still carry the old product name until they are renamed).
// The product is Straft Studio. The old name is matched in every spelling (one word, two words, with a hyphen or underscore,
// any case) and replaced case-aware: STRAFT for an all-caps form, straft for one starting in lowercase, Straft otherwise
// (so the old "<name> Studio" becomes "Straft Studio"). "an <old name>" becomes "a Straft".
// Domains and addresses that still use the old name (the site domain, its email addresses, the Pages project host) are left
// as they are: a form directly followed by "...studio.app" or "...studio.pages.dev" is kept.
// The patterns spell the old name with character classes, so a repo grep for it stays clean.
//
// BRAND_RULES is plain data ([source, flags, kind]) so the same rules can be handed to a browser page (scripts/shots.mjs and
// scripts/render-screens.mjs run their cleaners there) and rebuilt there with makeRebrand's source.

const NAME = '[Oo][Pp][Ee][Nn][\\s_-]?[Ff][Rr][Aa][Mm][Ee]'; // the old name, any case, any separator
const KEEP = '(?![\\w-]*studio\\.(?:app|pages\\.dev))'; // not part of a domain or email address

export const BRAND_RULES = [
  [`\\b([Aa])[Nn](\\s+)(?=${NAME}${KEEP})`, 'g', 'article'], // an / An / AN before the name -> a / A / A
  [`${NAME}${KEEP}`, 'g', 'name'],
];

// Self-contained (no outer references), so it can also be rebuilt inside a page: new Function('return ' + makeRebrand)().
export function makeRebrand(rules) {
  const rx = rules.map(([s, f, kind]) => [new RegExp(s, f), kind]);
  const word = (m) => (m === m.toUpperCase() ? 'STRAFT' : m.charAt(0) === m.charAt(0).toLowerCase() ? 'straft' : 'Straft');
  return (t) => {
    if (typeof t !== 'string' || !t) return t;
    for (const [re, kind] of rx) t = t.replace(re, kind === 'article' ? (_, a, sp) => a + sp : word);
    return t;
  };
}

export const rebrand = makeRebrand(BRAND_RULES);

// Every string in a JSON-like value (keys are left alone).
export const rebrandDeep = (v) => (typeof v === 'string' ? rebrand(v)
  : Array.isArray(v) ? v.map(rebrandDeep)
  : v && typeof v === 'object' ? Object.fromEntries(Object.entries(v).map(([k, x]) => [k, rebrandDeep(x)])) : v);

// Renames the product in a live page, for pictures taken of it (run with page.evaluate(rebrandDom, { brand: BRAND_RULES,
// brandFn: makeRebrand.toString(), attrs })). Self-contained. Changes what is drawn: text nodes, the document title, a name split over
// inline elements (the Windows wordmark draws its first half in an <i>; it becomes one text node, element and class kept), and
// the attributes listed in attrs (default: value and placeholder, which inputs draw). Attributes that stylesheets select on (the
// theme name in data-theme) are left alone, since the page still loads the app's own stylesheets.
export function rebrandDom(o) {
  const rb = new Function('return ' + o.brandFn)()(o.brand);
  const SPLIT = /^\s*[Oo][Pp][Ee][Nn]\s*[Ff][Rr][Aa][Mm][Ee](?:\s+[Ss]tudio)?\s*$/;
  for (const e of [...document.querySelectorAll('body *')].reverse()) {
    if (!e.firstElementChild || !SPLIT.test(e.textContent)) continue;
    if ([...e.querySelectorAll('*')].every((d) => /^(I|B|EM|STRONG|SPAN|SMALL|U)$/.test(d.nodeName))) e.textContent = rb(e.textContent.replace(/\s+/g, ' ').trim());
  }
  const tw = document.createTreeWalker(document, NodeFilter.SHOW_TEXT); let n = 0;
  while (tw.nextNode()) { const t = tw.currentNode; if (/^(STYLE|SCRIPT)$/.test(t.parentNode.nodeName)) continue; const v = rb(t.nodeValue); if (v !== t.nodeValue) { t.nodeValue = v; n++; } }
  for (const a of o.attrs || ['value', 'placeholder']) for (const e of document.querySelectorAll(`[${a}]`)) {
    const v = e.getAttribute(a), w = rb(v); if (w === v) continue;
    e.setAttribute(a, w); if (a === 'value' && 'value' in e && e.value === v) e.value = w; n++;
  }
  for (const e of document.querySelectorAll('input,textarea')) if (e.value && rb(e.value) !== e.value) { e.value = rb(e.value); n++; }
  if (rb(document.title) !== document.title) document.title = rb(document.title);
  return n;
}

// True when text still holds the old name outside the kept domains (for checks).
export const hasOldName = (t) => new RegExp(`${NAME}${KEEP}`).test(t); // the same forms the name rule renames
