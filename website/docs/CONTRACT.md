# Build contract (all agents read this first)

Repository: C:\OpenFrameWebsite (pushed later as `website/` in GitHub Saravankumar25/OpenFrame). Static site, plain
Node ESM, no framework. `npm run build` writes `dist/`. Deployed on Cloudflare Pages (https://openframe-studio.pages.dev).

## Truth rules (non negotiable)
- Read research/facts.json and research/capability-matrix.md. Claim only what they show. Status today: internal
  testing, unsigned builds, team password, no public download, no live trial, no live prices (targets only:
  3,000 rupees / 89 dollars / 89 euros, "planned"), store release planned (Microsoft Store, Mac App Store).
- Platforms: Windows 11 (x64). macOS 14 or later on Apple silicon. Never Windows 10 or Intel Macs.
- AI: optional, offline after a one-time ~1.2 GB download the person starts; suggests only, nothing changes until Apply.
- 17 themes. App screens in English. Writing in English plus Hindi, Telugu, Tamil, Kannada, Malayalam scripts for PDF.
- Collaboration is file-based (review packages, Review Queue, backups, project packages). No real-time collaboration.
- No em dashes (U+2014) or en dashes in copy. No "seamless, unlock, elevate, supercharge, game-changer, effortless,
  revolutionize, cutting-edge". Plain English, short sentences. No eyebrow/kicker labels above headings. No
  invented testimonials, ratings, counts or awards. No email capture forms (no backend exists).

## Files and owners
| Path | Owner |
|---|---|
| site.config.mjs, src/lib/*, src/content/nav.mjs, src/assets/css/site.css, src/assets/js/site.js, scripts/build.mjs, src/pages/_*.mjs | lead |
| src/assets/js/player.js, src/assets/css/player.css, scripts/capture-screens.mjs, screens/** | player agent |
| src/content/features.mjs, src/pages/features.mjs, src/pages/home.mjs, src/assets/css/pages-marketing.css | marketing agent |
| src/content/copy.mjs, src/pages/info.mjs (pricing, faq, migrate, compare, local-and-private, themes, about, contact, legal, download, /windows/, /mac/), src/assets/css/pages-info.css | info agent |
| src/content/tutorials/catalog.mjs, src/pages/tutorials.mjs, src/assets/css/pages-tutorials.css | lead |
| src/content/tutorials/win/<id>.mjs | Windows tutorial agents |
| src/content/tutorials/mac/<id>.mjs | Mac tutorial agents |
| tests/** , playwright.config.mjs | test agent |
Never edit a file you do not own. Need a change elsewhere: say so in your final reply.

## Page modules (src/pages/*.mjs)
Each exports `default function (ctx) => Page[]`. Page:
```js
{ path: '/pricing/',            // always leading and trailing slash; '/' for home
  title: 'Pricing | OpenFrame Studio',   // unique, <= 60 chars
  description: '...',           // unique, 120-160 chars
  body: html`...`,              // contents of <main>, starting with the page h1 (exactly one h1)
  nav: 'pricing',               // header item to mark current: features|tutorials|pricing|faq|download
  os: null | 'win' | 'mac',     // set only on platform-specific pages (/windows/..., /mac/...)
  alternates: { win: '/windows/tutorials/x/', mac: '/mac/tutorials/x/' } | null, // counterpart pages, if any
  breadcrumbs: [['Features', '/features/'], ['Screenplay Editor', '/features/screenplay/']], // excluding Home
  jsonld: [ {...schema.org object...} ], // optional extra JSON-LD
  ogImage: '/screens/win/123-x-1600.webp' | undefined, // build converts to absolute
}
```
ctx = { html, raw, esc, md, ICON, MARK, FEATURES, FEATURE_GROUPS, TUTORIAL_PATHS, OS, facts, screens, tutorials,
        shot, osBlocks, osText, kbd, cta, SITE_URL }
- `html\`\`` escapes interpolations; use `raw()` only for trusted markup. `md('**b** [t](/x/)')` for inline copy.
- `shot(plat, screen, { alt, eager, crop:[x,y,w,h], caption })` returns a figure with a responsive real app screen
  (`plat` 'win'|'mac', `screen` = the capture file name without .html, e.g. '003-app-home-empty'). Any screen used
  through shot() is captured automatically by the player agent's capture script (it scans src/ for shot( calls via
  the screens list exported by each page module: `export const SCREENS = { win: [...], mac: [...] }`). ALWAYS export
  SCREENS listing every screen your module uses.
- `osBlocks({ win: html, mac: html })` renders both variants; CSS shows the one matching the visitor's platform.
- `osText('Ctrl+K', 'Cmd+K')` inline variant. `kbd('Ctrl+K')` renders keycaps.
- `cta()` renders the standard closing band (truthful: download status, tutorials link).
- Internal links: absolute paths with trailing slash (`/features/screenplay/`). No `.html` links.
- No inline `style=""` attributes and no inline `<script>` (strict CSP). Put styles in your CSS file.
- Images: only via shot() or files you put under src/static/ (copied to dist root).

## CSS
Tokens and components live in site.css (lead). Use these classes; add page-specific ones in your own CSS file with a
page prefix. Tokens: --ink --ink2 --muted --bg --cream --tint --line --line2 --accent --accent-ink (accessible accent
for small text) --accent-soft --ok --dark --dark2 --r --shadow --ease --font.
Components: .wrap (1180px) .narrow (760px) .section .section.tint .section.dark .sec-head .lead .grid2 .grid3 .card
.btn .btn-primary .btn-dark .btn-ghost .btn-sm .btn-lg .ticks (list with check icons) .pill .note (callout)
.shot (screen figure) .breadcrumbs .prose (long text) .split (2 columns) .kv (definition grid) .table.

## Screens
Windows: research/WindowsHTML-src/light/NNN-name.html (1600x1000). Mac: research/MacHTML-src/light/NNN-name.html
(window picture; pages.json has titles). Use the newest captures only; never the package's old app/ or HTML_Mockups/.
Windows screens show the Windows app, Mac screens the Mac app. Never swap.

## Tutorial modules (one file per tutorial per platform)
`src/content/tutorials/win/<id>.mjs` and `src/content/tutorials/mac/<id>.mjs`, each `export default {...}`:
```js
export default {
  id: 'create-project',           // same id on both platforms when it is the same workflow
  available: true,                // false: only id, title, reason are needed (no page is built; the switch explains)
  reason: '',                     // why it is not available on this platform (plain words)
  path: 'getting-started',        // one of TUTORIAL_PATHS keys in src/content/nav.mjs
  order: 2,                       // order inside the path
  feature: 'screenplay',          // a FEATURES slug from nav.mjs, or null
  title: 'Create your first project',          // task-shaped, <= 60 chars
  purpose: 'One sentence: what you will be able to do.',
  difficulty: 'Beginner',         // Beginner | Intermediate | Advanced
  minutes: 3,
  prerequisites: ['OpenFrame Studio is installed and open.'],
  objectives: ['Create a project', 'Choose its type'],
  steps: [                        // from research/tutorials-<plat>-*.json, verified screens and targets
    { screen: '003-app-home-empty', action: 'click', target: 'role=button[name="Create Project"]',
      label: 'Click Create Project', text: null, key: null,
      body: 'One or two plain sentences telling the reader what to do and what they will see.' },
    { screen: '...', action: 'view', target: null, label: 'Your project opens', text: null, key: null, body: '...' },
  ],
  result: 'What the reader has at the end.',
  mistakes: ['Common mistake and how to avoid it.'],
  troubleshooting: [['Problem the reader may hit', 'What to do']],
  related: ['write-screenplay', 'idea-vault'],  // ids
  next: 'project-tour',                           // id of the recommended next tutorial
};
```
Rules: `screen` has no `.html`. Platform words: Windows says Ctrl, dialog, right-click; Mac says Cmd, sheet,
Control-click. Text must match the screens (exact UI names). No em dashes. 3 to 9 steps. Only claim what the screens
and the capability matrix show. URLs: /windows/tutorials/<id>/ and /mac/tutorials/<id>/.
