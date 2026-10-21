# OpenFrame Studio website

The public website for OpenFrame Studio: product pages, platform pages for Windows and macOS, and 62 animated
tutorials (32 for Windows, 30 for macOS) built from the real app screens. Static HTML, CSS and JavaScript, generated at
build time and hosted on Cloudflare Pages.

Live: https://openframe-studio.pages.dev

## Commands

| Command | What it does |
|---|---|
| `npm install` | Installs the build and test tools (esbuild, sharp, Playwright, axe). Run `npx playwright install chromium` once. |
| `npm run screens` | Renders the app screens used by pages and tutorials into `screens/` (WebP) and `screens/manifest.json` (sizes and click targets). Needs the screen sources (see below). The output is committed, so a normal build does not need the sources. |
| `npm run build` | Writes the site to `dist/`. Fails on a missing screen, a duplicate URL, or an em or en dash in page text. |
| `npm test` | Static checks over `dist/`, tutorial data checks, and browser tests (platform switch, player, phone widths, axe). |
| `npm run serve` | Serves `dist/` on http://localhost:4173 with clean URLs and the 404 page. |
| `npm run deploy` | Deploys `dist/` to the Cloudflare Pages project `openframe-studio` (needs `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID`). |

## How it is built

- `site.config.mjs`: the public address (`OF_SITE_URL`), used for canonical URLs, Open Graph, the sitemap and llms.txt.
- `src/lib/`: the HTML helper (escapes by default), icons, and the page layout (header with the macOS | Windows switch,
  footer, screen figures).
- `src/content/`: navigation and taxonomy (`nav.mjs`), feature copy (`features.mjs`), page copy (`copy.mjs`), and one
  tutorial module per tutorial per platform (`tutorials/win/*.mjs`, `tutorials/mac/*.mjs`).
- `src/pages/`: page modules. Each returns a list of pages (path, title, description, body, platform, alternates).
- `src/assets/`: `site.css` and page styles, `site.js` (platform switch, menus), `player.js` (the tutorial player), the
  self-hosted Inter font.
- `scripts/`: `build.mjs`, `capture-screens.mjs`, `serve.mjs`, `deploy.mjs`, `make-og.mjs`.
- `docs/`: the build contract, the operations guide, the validation report, and `tutorial-screen-map.json`.
- `research/`: the evidence the copy is checked against (capability matrix, facts with source lines, design notes,
  tutorial screen maps). The screen sources (`research/WindowsHTML-src`, `research/MacHTML-src`) are git-ignored copies
  of `windows/WindowsHTML` and `mac/MacHTML`.

See [docs/OPERATIONS.md](docs/OPERATIONS.md) for deployment, the custom domain, and how to update screens and tutorials.
