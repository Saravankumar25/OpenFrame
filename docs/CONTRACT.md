# Build contract

Rewritten 2026-10-31 for the redesign that follows the approved Website Share Package. The earlier contract (player
agent, WebP screens, `src/content/tutorials`) is superseded and kept in git history.

## Truth rules

- Claim only what `research/capability-matrix.md`, `research/facts.json` or a captured screen shows.
- Windows screens show the Windows app, Mac screens the Mac app. Never swap. A Mac page with no Mac screen for a point
  shows the text without a screen.
- No em or en dashes (the build fails on them). No internal release wording (no team testing, unsigned builds,
  passwords, plans or dates); `tests/static.test.mjs` fails on it.
- Company: Vighnajith Advisories Private Limited, named in the footer only (owner decision 2026-10-31). Founders, in this order: Saravan Kumar M K (Technical Founding Engineer: end-to-end
  technical implementation) and Badrinath Reddy A (Product Founding Engineer: idea, ideation, design, direction).
- Prices: ₹2,999 in India, US$89 elsewhere, one-time, after a 7-day trial. No other currency.
- Store buttons look normal and do nothing when clicked until `STORE` in `site.config.mjs` holds a real listing link.

## Page modules

Each `src/pages/*.mjs` default export takes `ctx` and returns pages `{ path, title (max 60), description, body, nav,
os, alternates, breadcrumbs, jsonld }`. `ctx` holds `html`, `md`, `mock(plat, screenName, opts)` (an embedded app
screen), `storeBtn`, `storeRow`, `price`, `cta`, `osBlocks`, `cat` (screen catalogs), `manual` (manual areas) and
`manuals` (PDF metadata). Screens are named without their number; the build fails on a name not in
`research/screens-<plat>.json`.

## Content

Manual areas and tutorials: `docs/CONTENT-BRIEF.md`. Feature catalog: `src/content/catalog.mjs`.
