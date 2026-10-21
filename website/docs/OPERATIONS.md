# Operations: deploy, domain, maintenance, validation, open inputs

## Deploy (Cloudflare Pages)

- Project: `openframe-studio` (classic Pages, production branch `main`). Address: https://openframe-studio.pages.dev
- Credentials: `CLOUDFLARE_API_TOKEN` (custom token, permission Account, Cloudflare Pages, Edit) and
  `CLOUDFLARE_ACCOUNT_ID`, saved as Windows user environment variables. `scripts/deploy.mjs` reads them; nothing is
  stored in the repository.
- Release: `npm run build && npm test && npm run deploy`. A preview of a branch: `node scripts/deploy.mjs preview`.
- Do not pass `--force` to wrangler again: it was needed once to create the project on classic Pages.
- Hosting files are generated: `_headers` (strict CSP with the hash of the one boot script, HSTS, frame denial,
  no-sniff, referrer and permissions policy, long cache for hashed assets), `_redirects` (old `.html` URLs from the
  website mock go to the new clean URLs), `sitemap.xml`, `robots.txt`, `llms.txt`, `404.html`.

## Custom domain (later)

1. Buy the domain. In Cloudflare, Workers and Pages, `openframe-studio`, Custom domains, add the domain and follow the DNS
   steps (Cloudflare adds the records itself when the zone is on Cloudflare).
2. Build with the new address: set `OF_SITE_URL=https://<domain>` (or change the default in `site.config.mjs`), then
   `npm run build && npm test && npm run deploy`. Canonical URLs, Open Graph URLs, the sitemap, robots.txt and llms.txt
   all follow it.
3. Optional: a redirect from `openframe-studio.pages.dev` to the domain (Bulk Redirects in Cloudflare) so search
   engines settle on one address.
4. Publish the contact address on `/contact/` once the company email exists (`src/content/copy.mjs`).

## When the app changes

- New or changed screens: recapture WindowsHTML (`windows/`) or MacHTML (`mac/`), copy them to
  `research/WindowsHTML-src` and `research/MacHTML-src` (or point `OF_WIN_HTML` / `OF_MAC_HTML` at them), run
  `npm run screens`. The capture script masks personal names, local paths and machine details before each screenshot
  and refuses to write a screen that still contains them.
- Which tutorials use a screen: `docs/tutorial-screen-map.json` (`screens["win/<screen>"]` lists `win:<tutorial id>`).
- A tutorial step: edit `src/content/tutorials/<win|mac>/<id>.mjs`. Check a target with
  `node research/probe.mjs <win|mac> <screen>.html '<selector>'` (it must match one element). `npm run screens` records
  the click target; `npm test` fails if a target has no rectangle.
- Copy rules: `docs/CONTRACT.md` (truth rules, no em dashes, no invented claims). Facts with source lines:
  `research/facts.json` and `research/capability-matrix.md`.

## Validation (2026-10-21)

- `npm test`: 81 of 81 pass. Static checks on all 96 pages (one h1, unique titles and descriptions, canonical and Open
  Graph tags, alt text and image sizes, no inline styles or scripts, JSON-LD parses, no em or en dashes, every internal
  link and asset resolves, sitemap complete). 62 tutorial modules (fields, 3 to 9 steps, every screen captured, every
  click target measured, related and next ids exist). Browser tests: platform switch and persistence, switching from a
  Windows-only tutorial to macOS shows the explanation, player next, previous, play, pause, keyboard and full screen, no
  horizontal scroll at 375, 768 and 1280 pixels, back and forward, 404 status, axe with no serious or critical issues.
- Live smoke test on https://openframe-studio.pages.dev: key pages 200, unknown page 404, old `.html` URL 301, CSP and
  HSTS headers served, no console errors on 7 pages including tutorials, platform switch works live.
- Lighthouse (live, desktop run): home page performance 100, accessibility 100, best practices 100, SEO 92 before the
  link-text fix (now fixed); Windows tutorial page 100, 100, 100, 100. LCP 1.6 to 1.9 s, CLS 0.

## Open business inputs (owner decisions, not blockers for the site)

- Domain and company email (Chapter 2). The contact page says no public address is decided yet.
- Final prices, trial start and store listings (Chapter 3). The site shows target prices marked as targets and says no
  trial is running.
- Final legal terms and privacy policy for the product. The site carries an interim website notice only.
- Minimum RAM and disk for the app are not published; the platform pages say so.
- Public download: none until release. The Mac internal `.dmg` is not built yet.
- Social accounts: none listed until they exist.
- Mac AI tutorials: the Mac screens of the offline assistant were not captured (the capture build had no AI runtime).
  Recapture MacHTML with the assistant installed, then add steps to `mac/ai-assistant-setup.mjs` and
  `mac/ai-assistant-ask.mjs`.
