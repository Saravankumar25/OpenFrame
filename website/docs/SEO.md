# SEO, GEO and performance

Work of 2026-10-31 by the `seo-sec` agent. Security is in `docs/SECURITY.md`. Everything marked "tested" is checked by
`tests/seo.test.mjs` on each build (`OF_DIST=<folder> node --test tests/seo.test.mjs`).

Where the code is: `src/lib/seo.mjs` (head tags, Open Graph and X cards, JSON-LD, sitemap helpers), `scripts/seo-files.mjs`
(sitemap.xml, robots.txt, llms.txt, llms-full.txt, `_headers`), `scripts/indexnow.mjs`, `scripts/perf.mjs`,
`src/content/lastmod.json` (commit this file).

## 1. Every page

| Item | How | State |
|---|---|---|
| Title | At most 60 characters, unique. Brand `| Straft Studio` is added automatically when the title has none and still fits. Cut at a word, never with a dash or dots. Duplicate tutorial titles get their area added ("Delete a saved snapshot (Budget) on Windows"). | Tested |
| Description | 70 to 165 characters, unique. Tutorial descriptions are the purpose line plus the area and platform, cut at a word. | Tested |
| Canonical | `SITE_URL + path` on every page, always with the trailing slash that Cloudflare Pages serves. | Tested |
| Robots | Indexable: `index, follow, max-image-preview:large, max-snippet:-1, max-video-preview:-1`. Error pages (`noindex` flag): `noindex`, and they are not in the sitemap. `/search/` must be `noindex` (a results shell with no content of its own; the test enforces it once the page exists). | Tested |
| Open Graph | `og:type website`, site name, title, description, url, image (absolute https), image type, width, height, alt text, locale. | Tested, including that `og.png` really is the declared 1200 by 630 and under 600 KB |
| X (Twitter) | `summary_large_image` with title, description, image and image alt. No `twitter:site`: there is no account to name. | Tested |
| Per-page picture | A page can set `ogImage`, `ogWidth`, `ogHeight`, `ogAlt`, `ogType`. Today every page uses `/og.png`. A picture per section (features, tutorials by area) would make shared links more specific; not done. | Optional |

Home title is now "Straft Studio: film production software" (it was 88 characters and would have been cut in results).

## 2. Structured data (JSON-LD)

Each page declares what it needs; `jsonLd()` in `src/lib/seo.mjs` then removes anything the page does not show.

| Type | Where | Notes |
|---|---|---|
| Organization | Home, About | Brand name `Straft Studio`, logo (512 px icon), slogan from the About page. Founders (Saravan Kumar M K, Badrinath Reddy A) only where they are named on the page, so on About only. |
| WebSite | Home | With `SearchAction` to `/search/?q={search_term_string}`. Google stopped showing the sitelinks search box in 2024, so this is kept for other consumers and costs one object. Test: the target page exists. |
| SoftwareApplication | Home, Features, Windows, Mac, Pricing | `operatingSystem`, `applicationCategory` (MultimediaApplication), image. `offers` only on pages that show the price (Home, Pricing, the platform pages): INR 3999 with `eligibleRegion` IN, EUR 89 with `eligibleRegion` Europe, USD 89 with `ineligibleRegion` IN and Europe, in stock. These are the launch prices; the crossed-out originals are not in the data. No `priceValidUntil`: no end date is set, and an invented one would expire. No ratings or reviews: none exist and none are invented, so Google will not show a software rich result. The markup still gives search and answer engines the facts. |
| BreadcrumbList | Every indexable page except Home | Home, then the page's trail, last item is the page itself. Tested: each item resolves. |
| FAQPage | `/faq/` only | Every question is checked against the visible page text. Google shows FAQ rich results only for a few official sites now; the markup is kept because it matches the page and answer engines read it. |
| HowTo | Removed | Google ended HowTo rich results in 2023 and no longer reports them. The markup repeated every step of about 700 tutorial pages in each page's source for no reader. Tutorial steps are already plain, ordered headings in the page. Nothing else uses it. Tested: HowTo is not allowed. |

Decision flagged for the owner: the legal entity (Vighnajith Advisories Private Limited) is **not** in the JSON-LD. The
owner rule is that it appears in the footer only. Search engines and the store listing can read it from the footer. If you
want the entity link in the knowledge graph, add `parentOrganization: { '@type': 'Organization', name: '...' }` to `ORG()`
in `src/lib/seo.mjs` and relax the test line that forbids the company name in JSON-LD. I recommend leaving it out until the
company has its own public web presence to point to.

## 3. Sitemap, robots, languages

* `sitemap.xml` lists indexable pages only. `<lastmod>` is the day the page's own content (title, description, body) last
  changed, not the build day. The hashes are kept in `src/content/lastmod.json`; the file must be committed or dates restart
  at the build day. Search engines ignore lastmod once it has proven wrong, so it is kept honest.
* Image entries: for every local `<img>` in a page body (not inline `data:` pictures, not other sites) the page's sitemap
  entry gets an `<image:image><image:loc>`. It works for any picture a page shows, today the `/shots/` pictures. Google reads
  only `image:loc` now. Pictures that are only linked (the full-size file behind a lightbox link) are not listed.
* `robots.txt`: allow everything, `Disallow: /search-index.json` (a script's data file), the sitemap address. It does not
  block AI crawlers. The `tutorial-screen-map.json` file is no longer published.
* **hreflang does not apply.** hreflang tells a search engine which URL serves which language or region. This is one
  English site on one URL per page. The regional price is chosen in the browser on the same URL (with scripts off both
  prices are listed, and the JSON-LD names both with their regions). The Windows and Mac tutorial pairs are different
  content for different systems, not translations, so they are not alternates either. Add hreflang only when a second
  language or a region-specific URL set exists.

## 4. Answer engines (GEO)

* `llms.txt` (summary, facts, key pages) and `llms-full.txt` (the whole feature catalog and every tutorial with its purpose)
  are generated, served as UTF-8 text, and tested to be free of company and technology details and dashes. Every link in them
  is checked against the build. The facts follow the FAQ and the pricing page word for word.
* Pages state facts in plain sentences with the price, platforms, import and export formats, privacy and trial in the FAQ
  and in `llms.txt`; JSON-LD repeats only what is visible.
* Do not switch on "Block AI bots" in Cloudflare (see the dashboard list in `docs/SECURITY.md`). Whether AI companies may
  train on the site is an owner policy decision; it is not stated in `robots.txt` today.

## 5. IndexNow

* Key file: `src/static/8dff8bfd7101028db60a502780c7c4c7.txt`, published at `/8dff8bfd7101028db60a502780c7c4c7.txt` (its
  content is the key, 32 hex characters, no newline). The key is public by design: it only proves that the site owns the file,
  so it is committed and is not a secret. IndexNow lists Bing, Yandex, Naver, Seznam, Yep and Amazon; a submission to one is
  shared with the others. Google does not take part. Specification: https://www.indexnow.org/documentation and
  https://www.indexnow.org/faq.
* `scripts/indexnow.mjs` runs after a deploy and sends only what changed. Modes: `--since <old-sitemap.xml>` (pages new or
  with a different lastmod, plus pages that left the sitemap, compared with the live sitemap saved before the deploy),
  `--changed` (lastmod is today; the fallback when no old sitemap exists), no flag (every URL; the first run, or to repeat a
  failed one), `--dry-run` (prints, sends nothing). `--soft` turns every failure into a warning with exit code 0, so a
  search-engine outage never fails a deploy.
* It refuses a `pages.dev` address, waits up to 15 seconds for the key file to be live, sends at most 10,000 URLs per request,
  and accepts 200 and 202 as success (400 format, 403 key, 422 wrong host or repeated unchanged URLs, 429 too many).
* IndexNow asks for a resubmission of the same URL only after a real change and not within minutes. Do not run the no-flag
  mode after every deploy. Cloudflare Crawler Hints sends its own IndexNow signal from cache misses once the owner switches it
  on (`docs/SEO-CHECKLIST.md`); the script covers the pages it misses.
* The deploy workflow lines and the full checklist are in `docs/SEO-CHECKLIST.md`. Live check of what search engines see:
  `npm run seo:monitor` (`scripts/seo-monitor.mjs`). Tests without a browser: `tests/seo-live-ready.test.mjs`.

## 6. Moving to the custom domain (SEO side)

1. Rebuild with `OF_SITE_URL=https://<domain>`: canonical tags, Open Graph URLs, JSON-LD, sitemap, robots, llms files and
   `security.txt` all follow it. Deploy.
2. Search Console: add a Domain property (DNS TXT record), submit `/sitemap.xml`. Bing Webmaster Tools: import from Search
   Console. Check "Page indexing" after a week.
3. Run IndexNow with no flag once (section 5), then let the deploy workflow send changes.
4. Keep the `pages.dev` address answering; the canonical tags point search engines to the real domain.
5. If the privacy notice or footer changes in the same release, keep `llms.txt` and the FAQ in step.

## 7. Performance

Measured with `OF_DIST=<folder> node scripts/perf.mjs [paths]`: headless Chromium, Pixel 5 profile, slow 4G (1.6 Mbps, 150
ms), CPU four times slower, cold cache, Brotli, median of 3 to 5 runs. The machine was shared with other work, so absolute
numbers move by a few hundred milliseconds between runs; the comparisons below were run back to back.

Result after the change (sizes are what crosses the wire):

| Page | LCP | CLS | Transfer |
|---|---|---|---|
| `/` | 0.82 s | 0.002 | 212 KB (one picture, 39 KB) |
| `/features/` | 0.95 s | 0.003 | 180 KB |
| `/pricing/` | 0.80 s | 0.002 | 170 KB |
| a Windows tutorial (`create-first-project`) | 0.75 s | 0.001 | 169 KB |
| a Mac tutorial with 9 to 10 pictures | 0.87 to 0.93 s | 0.002 | 284 to 316 KB, pictures 20 to 50 KB each, loaded as they scroll in |

Good thresholds are LCP 2.5 s and CLS 0.1. All pass with room.

**What was changed:** the three font preloads were removed from every page. Fonts are 137 KB of a 170 KB page. Preloaded, they
shared the slow line with the stylesheet, so nothing was painted until about 1.5 to 2.2 s. Without them the page paints at
about 0.75 to 0.95 s and the fonts swap in a moment later (`font-display: swap` is set on all three). The swap costs a
layout shift of 0.002 to 0.004. Measured back to back, same dist, same profile:

| Variant | `/` first paint | `/pricing/` first paint |
|---|---|---|
| Three preloads (before) | 1.82 s | 1.52 s |
| Inter and Newsreader preloaded | 2.22 s | 1.59 s |
| Newsreader only | 2.00 s | 0.94 s |
| No preloads (now) | 0.96 s | 0.74 s |

**Checked and fine:** one render-blocking stylesheet (14 KB compressed, 54 KB raw), one deferred script (12 KB compressed,
including search), `font-display: swap` on every face, every `<img>` has width, height and alt (tested), pictures below the
fold are lazy, the first picture on the home page is eager with high fetch priority, `/assets/*` is cached for a year as
immutable and the content hash is in the file name, pictures for a week.

**Not mine, still open (listed for the owners of those files):**
1. Fonts are the weight of the page. Newsreader (58 KB) is used only for the main heading. Subsetting the three fonts to the
   characters the site uses, or limiting the weight axis to the weights in `site.css` (400 to 800), would cut about half.
   Needs `fonttools` (no Python is installed on this machine). A fallback font with `size-adjust` would remove the 0.002 swap shift.
2. The footer company logo `src/static/company/vighnajith-advisories-logo.webp` is 175 KB at 1774 by 887 and is shown small
   on every page. A copy at about 400 px wide is a few KB. It is lazy, so it does not delay first paint.
3. `site.css` carries the rules for search, legal and pictures on every page. Fine at 14 KB compressed; split only if it
   passes about 30 KB.
4. `/assets/fonts/*` is `immutable` for a year but the font file names have no hash. Rename a font file whenever its
   contents change.
5. Re-run `scripts/perf.mjs` on a Windows tutorial once the Windows pictures exist; only the Mac pictures were in the build
   when this was measured.

## 8. Owner list

* Decide whether the legal entity goes into JSON-LD (section 2): recommended not yet.
* Decide the AI training policy (section 4).
* Everything only the owner can do for search (Search Console, Bing, Cloudflare switches, the analytics decision) is in
  `docs/SEO-CHECKLIST.md`, section "Pending user action". The Cloudflare list in `docs/SECURITY.md` still applies.
* Before switching Cloudflare Web Analytics on, update the privacy notice (it says the site runs no analytics and loads
  nothing from other companies). See `docs/SECURITY.md` section 3.
