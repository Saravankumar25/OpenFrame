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

// True when text still holds the old name outside the kept domains (for checks).
export const hasOldName = (t) => new RegExp(`${NAME}${KEEP}`).test(t);
