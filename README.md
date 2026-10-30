# OpenFrame Studio website

The public website for OpenFrame Studio, a product of Vighnajith Advisories Private Limited. It follows the approved
design package (`research/package-src`, the Website Share Package) and shows the real app screens: every Windows and
Mac screen is embedded as its captured HTML page, in light or dark. Static HTML, CSS and JavaScript, generated at build
time and hosted on Cloudflare Pages.

Live: https://openframe-studio.pages.dev

## Three independent controls

The bar at the top of every page holds: **Platform** (Windows or Mac: which screens, tutorials and text show),
**Website** (light, white background, or dark, black background) and **App screens** (light or dark screen pages).
Each choice is kept in the browser (`of_os`, `of_theme`, `of_mock`) and none changes another.

## Commands

| Command | What it does |
|---|---|
| `npm install` | Installs the build and test tools (esbuild, sharp, Playwright, axe). Run `npx playwright install chromium` once. |
| `npm run screens` | Reads every screen into `research/screens-<win|mac>.json` and renders Windows thumbnails and full pictures into `.cache/`. Needs the screen sources (below). |
| `npm run check:content` | Checks the manual content against the screens (names, every screen covered, writing rules). |
| `npm run manuals` | Builds the two PDF user manuals into `manuals/`. |
| `npm run build` | Writes the site to `dist/`, with the screen pages under `/mockups/`. `OF_SKIP_MOCKUPS=1` skips copying screens for quick page builds. |
| `npm test` | Static checks, content checks and browser tests (controls, regional price, phone width, axe). |
| `npm run serve` | Serves `dist/` on http://localhost:4173. |
| `npm run deploy` | Deploys `dist/` to the Cloudflare Pages project `openframe-studio`. |

## Sources

- Screen pages: `research/WindowsHTML-src` (copy of `windows/WindowsHTML`, 819 screens) and `research/MacHTML-src`
  (copy of `mac/MacHTML`, 760 screens), both git-ignored.
- Manual and tutorials: `src/content/manual/<win|mac>/<area>.mjs`, one file per app area (schema in
  `docs/CONTENT-BRIEF.md`). Every workflow there is a website tutorial and a manual section.
- Feature catalog: `src/content/catalog.mjs`. Feature page copy: `src/content/features.mjs`.
- Store links: `STORE` in `site.config.mjs` (or `OF_STORE_WIN`, `OF_STORE_MAC`). Empty means the store buttons show but
  do nothing when clicked.

See [docs/OPERATIONS.md](docs/OPERATIONS.md) for deployment and the custom domain.
