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
  `research/WindowsHTML-src` and `research/MacHTML-src`, then run `npm run screens` (screen catalogs and Windows
  thumbnails). A screen name that disappears makes `npm run build` and `npm run check:content` fail and name it.
- Tutorials: edit the workflow in `src/content/manual/<win|mac>/<area>.mjs` (schema in `docs/CONTENT-BRIEF.md`), run
  `npm run check:content`. Which tutorials use a screen: `docs/tutorial-screen-map.json`.
- Features: `src/content/catalog.mjs` (every capability with its screens) and `src/content/features.mjs` (page copy).
- User manuals: `npm run manuals` writes the two PDFs to `manuals/` (git-ignored). They are internal reference for
  writing tutorials and are never published on the website (owner decision 2026-10-31).
- Store links: set `OF_STORE_WIN` and `OF_STORE_MAC` (or `STORE` in `site.config.mjs`); the store buttons then
  become links. Until then they look normal and do nothing when clicked.

## Validation (2026-10-31)

- `npm test`: 28 of 28 pass. Static: titles, descriptions, canonicals, one h1 per page, no em or en dashes, no internal
  release wording, every internal link, image and app screen resolves, company and founders, store buttons, prices,
  sitemap and screen framing headers. Content: every Windows (819) and Mac (760) screen described in a manual area,
  feature catalog screens exist, no manual page or PDF on the site. Browser: website theme and app screen theme are
  independent and remembered, platform switch in place and across platform pages, screens scale, India and US prices by
  time zone and by the region control, store buttons clickable without going anywhere, no sideways scroll at 375 px,
  axe with no serious or critical issues.

## Open business inputs

- Microsoft Store and Mac App Store listing links (the buttons do nothing until they exist).
- Custom domain and company email; a public contact address.
- Mac screens of the offline assistant at work (the Mac capture had no AI engine); the catalog marks those capabilities
  as Mac without a Mac screen.
