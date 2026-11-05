# SEO checklist and progress report

The one list that stays. Started 2026-11-05 by the automation and monitoring agent. Each row has a state: **done** (with the
evidence), **open** (work for an agent, with the agent named), or **owner** (only the owner can do it, details in
"Pending user action" below). Tick a row by changing its state and writing the evidence next to it. Never tick without evidence.
How the pieces work: `docs/SEO.md`. Dashboard settings: `docs/SECURITY.md` and `docs/OPERATIONS.md`.

Agents: **automation** (this file, IndexNow, monitor, tests), **crawl**, **metadata**, **geo**, **performance**.

## A. Getting pages into search engines

| # | Item | State | Evidence or next step |
|---|---|---|---|
| A1 | `sitemap.xml` live, lists indexable pages only, honest `lastmod`, image entries | done | Live fetch 2026-11-05: 200, XML. `scripts/seo-monitor.mjs` checks it. `tests/seo.test.mjs` checks content. |
| A2 | `robots.txt` allows crawling, names the sitemap, blocks only `/search-index.json` and `/mockups/` | done | Live fetch 2026-11-05. `tests/seo-live-ready.test.mjs` checks no sitemap URL is blocked. |
| A3 | IndexNow key file at the site root | done | `https://straftstudio.com/8dff8bfd7101028db60a502780c7c4c7.txt` answers 200 with the key (monitor, 2026-11-05). |
| A4 | `scripts/indexnow.mjs`: changed URLs only, soft failure, dry run, host refusal, retries, cap | done | Unit tests in `tests/indexnow.test.mjs` and `tests/seo-live-ready.test.mjs`. A mode flag is required; `--since` and `--changed` refuse more than 100 URLs. |
| A5 | First IndexNow submission of all URLs (`--all`) | done | 2026-11-05 05:03 UTC, from the live sitemap: 752 URLs, 1 request, HTTP 200. No earlier full submission was recorded (the 04:53 UTC deploy sent 3 URLs, answer 202). Do not repeat. |
| A6 | IndexNow after every deploy, wired into the deploy workflow | done | Live since the 2026-11-05 04:53 UTC run: step runs after the deploy step succeeds, `--soft --since`. Suggested hardening below. |
| A7 | Google Search Console Domain property, verified | owner | Pending user action 1. |
| A8 | Sitemap submitted in Search Console | owner | Pending user action 1. |
| A9 | Bing Webmaster Tools site added (import from Search Console) | owner | Pending user action 2. |
| A10 | Cloudflare Crawler Hints on | owner | Pending user action 4. Sends IndexNow signals from cache misses. |
| A11 | Google indexing requests | n/a | Google has no push API for ordinary pages. The URL Inspection API only reads status (2,000 queries a day per site) and cannot request indexing. The sitemap and Search Console are the route. |

## B. Automated checks (automation)

| # | Item | State | Evidence or next step |
|---|---|---|---|
| B1 | Build-time checks: one h1, JSON-LD parses, nothing in the sitemap is redirected, blocked, queried or without trailing slash, sitemap limits, `lastmod` not in the future, image entries exist, no site-wide `X-Robots-Tag`, key file exact | done | `tests/seo-live-ready.test.mjs`, 7 tests, 7 pass against the existing `dist/` on 2026-11-05. Runs with `npm test`. Titles, canonicals, noindex pages, structured-data rules stay in `tests/seo.test.mjs`. |
| B2 | Live monitor: robots, sitemap, a spread of pages (200, own canonical, no noindex, one h1), real 404, llms.txt, key file | done | `npm run seo:monitor`. Run against the live site 2026-11-05: all passed (20 pages). `--all` checks every URL. Exit code 1 on failure. |
| B3 | Monitor runs after each deploy | open | Lead applies the workflow lines below. |
| B4 | Monitor runs weekly and on failure someone is told | open | Optional: a scheduled workflow (lines below). A red run sends GitHub's own failure email; no extra service. |
| B5 | Search Console and Bing reports read on a schedule | owner | Section D, "Routine". |
| B6 | Lighthouse SEO score and axe run in CI | open | performance agent (local run was held back while the desktop test ran). |

## C. Crawl and indexing (crawl agent): all open

| # | Item | State |
|---|---|---|
| C1 | Every internal link resolves; no orphan pages; every indexable page reachable in 3 clicks from Home | open (crawl) |
| C2 | No redirect chains; `_redirects` targets are final URLs; old paths from the previous site redirect once | open (crawl) |
| C3 | One canonical host; `www` and `pages.dev` redirect to the apex with path kept | open (crawl); `npm run domain:verify` checks the first two |
| C4 | Trailing-slash policy identical in links, canonicals, sitemap and redirects | open (crawl); sitemap side is tested in B1 |
| C5 | Error pages: real 404, `noindex`, helpful links; no soft 404 | open (crawl); the monitor checks the 404 status |
| C6 | `/search/` stays `noindex`; no internal search or filter URLs in the sitemap | open (crawl); `tests/seo.test.mjs` enforces it |
| C7 | Pagination, tag or duplicate-content pages: none, or canonicalised | open (crawl) |
| C8 | `robots.txt` after Cloudflare changes: no prepended block that changes crawling | open (crawl); see owner action 5 |
| C9 | Crawl budget: sitemap size, `lastmod` honest (hash file committed) | open (crawl) |

## D. Page metadata and structured data (metadata agent): all open

| # | Item | State |
|---|---|---|
| D1 | Unique title and description on every page, within length | open (metadata); `tests/seo.test.mjs` enforces it |
| D2 | Open Graph and X cards, image size and alt, per-section pictures | open (metadata) |
| D3 | JSON-LD types, values match the visible page, validator pass (Rich Results Test, Schema Markup Validator) | open (metadata); rules tested, validators not run |
| D4 | `lang`, viewport, charset, favicon set, manifest | open (metadata) |
| D5 | Image alt text, descriptive file names, dimensions | open (metadata) |
| D6 | Heading outline: one h1, no skipped levels | open (metadata); the one-h1 rule is tested in B1 |
| D7 | Breadcrumbs on every deep page | open (metadata); tested |
| D8 | hreflang | n/a: one English site on one URL set (`docs/SEO.md` section 3) |

## E. Answer engines (geo agent): all open

| # | Item | State |
|---|---|---|
| E1 | `llms.txt` and `llms-full.txt` accurate, linked pages exist | open (geo); tested in `tests/seo.test.mjs` |
| E2 | AI crawler policy decided (allow search and answer bots, training yes or no) and written into `robots.txt` | owner decision, then geo |
| E3 | Facts stated in plain sentences: price, platforms, privacy, import and export, trial | open (geo) |
| E4 | Brand entity: consistent name, About page, same facts everywhere | open (geo) |
| E5 | FAQ answers match the questions people ask; markup matches the page | open (geo) |
| E6 | Check that Bing and answer engines can fetch pages (no challenge page for their bots) | open (geo), with owner action 5 |

## F. Speed and experience (performance agent): all open

| # | Item | State |
|---|---|---|
| F1 | Core Web Vitals on mobile: LCP 2.5 s, INP 200 ms, CLS 0.1, field data once traffic exists | open (performance); lab numbers in `docs/SEO.md` section 7 |
| F2 | Fonts subset, footer logo resized, picture sizes | open (performance) |
| F3 | Cache headers: immutable assets, short HTML cache, no stale canonical | open (performance) |
| F4 | CSP console error from the Cloudflare beacon gone | owner (action 3), then performance confirms |
| F5 | Accessibility pass (axe) on key pages | open (performance) |

## G. Off-site and launch (owner)

| # | Item | State |
|---|---|---|
| G1 | Store listings link to the site and the site links to the store pages when the listings exist (`STORE` in `site.config.mjs`) | owner |
| G2 | Directory listings (Product Hunt, AlternativeTo, G2, Capterra, SaaSHub) once the app can be downloaded; same name, description, prices | owner |
| G3 | Google Business Profile: only with a staffed public address | owner decision (`docs/OPERATIONS.md` section 10, item 8) |
| G4 | Company email on the domain, `security@` mailbox for `security.txt` | owner |
| G5 | Real reviews only from real users; none written or bought | owner |

## Deploy workflow lines (the lead applies; the file is outside this repository)

File: `.github/workflows/website-deploy.yml` on GitHub `main`. Replace the last step ("Deploy and check the live site") with these
three steps, in this order. Nothing else changes.

```yaml
      - name: Save the live sitemap, to find the pages this deploy changes
        continue-on-error: true
        run: curl -fsS --max-time 30 https://straftstudio.com/sitemap.xml -o "$RUNNER_TEMP/sitemap-before.xml"

      - name: Deploy and check the live site
        env:
          CLOUDFLARE_API_TOKEN: ${{ secrets.CLOUDFLARE_API_TOKEN }}
          CLOUDFLARE_ACCOUNT_ID: ${{ secrets.CLOUDFLARE_ACCOUNT_ID }}
        run: npm run deploy

      - name: Tell IndexNow engines which pages changed (not Google)
        continue-on-error: true
        run: node scripts/indexnow.mjs --soft --since "$RUNNER_TEMP/sitemap-before.xml"

      - name: Check what search engines see on the live site
        run: node scripts/seo-monitor.mjs
```

Notes. `--soft` and `continue-on-error` mean an IndexNow outage never fails the deploy. The monitor has no `continue-on-error`:
the site is already live when it runs, so a red run is the alarm. It needs no secrets. A failed IndexNow run loses that
deploy's list (the next diff starts from the new sitemap); repeat by hand with `node scripts/indexnow.mjs --since <saved old sitemap>`, or wait for the next change. Never run `--all` again
unless the owner asks. The checkout is sparse (`website` only), which is enough: both scripts read
`dist/sitemap.xml` and `src/static/` from there.

Suggested hardening (not applied; the lead edits the workflow): add `if: github.ref == 'refs/heads/main'` to the IndexNow step, so a
manual run of the workflow from another branch can never submit URLs. The step already runs only when the deploy step succeeded
(GitHub's default `success()`), and the workflow itself starts only from a push of `website/**` to `main` or a manual run.

Optional weekly check, a new file `.github/workflows/website-seo-monitor.yml`:

```yaml
name: Website SEO monitor
on:
  schedule:
    - cron: '30 4 * * 1'
  workflow_dispatch:
permissions:
  contents: read
jobs:
  monitor:
    runs-on: ubuntu-24.04
    timeout-minutes: 15
    defaults:
      run:
        working-directory: website
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          sparse-checkout: website
          persist-credentials: false
      - uses: actions/setup-node@949feb2413d6458794dcd2491c4babbbce0c15c1 # v7.1.0
        with:
          node-version: 24
      - run: node scripts/seo-monitor.mjs --sample 100
```

## PENDING USER ACTION

Only the owner can do these. Each step names the page to open. None needs a paid service. None touches MX, SPF, DKIM or DMARC.

### 1. Google Search Console: Domain property and sitemap

Google does not take part in IndexNow. Search Console is how Google learns about the sitemap and how you read indexing errors.

1. Open https://search.google.com/search-console and choose Add property, then **Domain**. Enter `straftstudio.com`.
   Help: https://support.google.com/webmasters/answer/34592
2. Google shows a TXT record. In the Cloudflare dashboard open the `straftstudio.com` zone, DNS, Records, Add record: type TXT,
   name `@`, content the value Google shows. Add it as a new record; do not edit or delete any existing TXT record (they hold
   the mail settings). Help: https://support.google.com/webmasters/answer/9008080
3. Back in Search Console press Verify. Cloudflare answers DNS within minutes. Keep the TXT record for good: Google
   re-checks it, and removing it removes the verification.
4. In Search Console, Sitemaps, enter `sitemap.xml` and Submit. The status should read Success and show the discovered page
   count. Help: https://support.google.com/webmasters/answer/7451001
5. After about a week open Pages (Page indexing) and read the reasons listed. "Discovered, currently not indexed" on a new
   site is normal and settles over weeks. Do not use the Request Indexing button on many pages; it is for single urgent pages.

### 2. Bing Webmaster Tools: import from Search Console

1. Open https://www.bing.com/webmasters and sign in with a Microsoft, Google or Facebook account.
2. Choose **Import** on My Sites and allow access to the Google account that owns the Search Console property. Pick
   `straftstudio.com`. Bing copies the verification and the sitemap. Details:
   https://blogs.bing.com/webmaster/september-2019/Import-sites-from-Search-Console-to-Bing-Webmaster-Tools
3. Check that the sitemap shows under Sitemaps. If it does not, add `https://straftstudio.com/sitemap.xml` by hand.
4. Bing re-checks ownership through Search Console from time to time. If Search Console access is removed, verify Bing again
   with an XML file or a DNS record.
5. IndexNow needs nothing from you here: the key file is already live, and Bing reads submissions without an account.

### 3. Cloudflare Web Analytics: switch off, or switch on

Today the zone injects the Web Analytics beacon, the site's content security policy blocks it, and every page logs one console
error (`docs/OPERATIONS.md` section 4.4). Pick one:

* **Off (the quiet choice, matches the privacy page today).** Cloudflare dashboard, Analytics & Logs, Web Analytics,
  `straftstudio.com`, Manage site, Disable. If the site is listed under Workers & Pages, `straft-studio`, Metrics, switch it
  off there too. Docs: https://developers.cloudflare.com/web-analytics/get-started/
* **On.** First change the privacy page, which now says the site runs no analytics and loads nothing from other companies.
  Then allow the beacon in the content security policy (`scripts/seo-files.mjs`, a task for the lead) and check the console on
  the live site. Cloudflare states that Web Analytics sets no cookies and does not use localStorage to track people. Whether a
  consent banner is needed in your markets is a question for your legal adviser. FAQ: https://developers.cloudflare.com/web-analytics/faq/

### 4. Cloudflare Crawler Hints

Dashboard, the `straftstudio.com` zone, Caching, Configuration, switch on **Crawler Hints**. It is free on every plan. Cloudflare
tells search engines through IndexNow when a cached page has changed. Docs:
https://developers.cloudflare.com/cache/advanced-configuration/crawler-hints/ and
https://blogs.bing.com/webmaster/november-2021/Cloudflare-Supports-IndexNow-via-one-click-Integration. The deploy script
sends the same signal for the pages it detects, and the two together do no harm: IndexNow only asks that the same URL is
not sent again within minutes.

### 5. AI crawler settings in Cloudflare

Decision first (`docs/SEO.md` section 4): may AI companies train on the site, and should answer engines read it? The GEO goal
needs search and answer bots allowed.

1. Security, Settings (or Bots): leave **Block AI bots** off if you want answer engines to read the site. The Cloudflare
   Bot Fight Mode can challenge crawlers; read Bing Webmaster Tools, Crawl information, in the first week.
2. "Managed robots.txt" (Cloudflare adds a block to `robots.txt` that asks AI training crawlers to stay away and states
   content signals such as `ai-train=no`) is on or off at the zone. If you switch it on, Cloudflare puts its block **before**
   our lines. Open `https://straftstudio.com/robots.txt` afterwards and confirm that `Allow: /` and the `Sitemap:` line are still
   there. Docs: https://developers.cloudflare.com/bots/additional-configurations/managed-robots-txt/
3. AI Crawl Control lets you allow or block each named AI crawler. Docs: https://developers.cloudflare.com/ai-crawl-control/
   Robots rules are requests, not locks; blocking in Cloudflare is the enforced version.
4. Tell the lead the decision, so `robots.txt` and `llms.txt` can state it.

### 6. DNSSEC

Cloudflare dashboard, `straftstudio.com`, DNS, Settings, DNSSEC, **Enable DNSSEC**. If the domain is registered with Cloudflare
Registrar the DS record is added for you. If it is registered elsewhere, copy the DS record values shown by Cloudflare into
the registrar's DNSSEC page. Wait until the status reads Active. Docs: https://developers.cloudflare.com/dns/dnssec/
Do not enable it while a registrar transfer is in progress.

### 7. Analytics decision (Google Analytics 4, conversion tracking)

Nothing was added. The privacy page promises no analytics, and the owner has not authorised one. The decision:

| Choice | What it gives | What it costs |
|---|---|---|
| No analytics, Search Console and Bing only | Search impressions, clicks, queries, indexing errors. Matches the privacy page. | No visits, no store-button clicks. |
| Cloudflare Web Analytics (action 3, On) | Page views, countries, referrers, Core Web Vitals, no cookies. Recommended in `docs/OPERATIONS.md` section 10. | Privacy page change, CSP change, six months of history. |
| Google Analytics 4 | Funnels, custom events, conversion events. | Cookies and a Google script, a consent decision in the EU and UK, a longer privacy page, CSP loosened for Google hosts. Against the product's pitch. |

Conversion tracking needs a store listing to count clicks to. Until the listings exist there is nothing to convert. Revisit at
Chapter 3 launch.

### Routine after the first week

Weekly: Search Console Pages report and Bing Webmaster Tools Site Explorer for new errors; `npm run seo:monitor` (or the
scheduled workflow). Monthly: compare impressions and clicks with the last month and write down what moved.

## Sources

IndexNow protocol https://www.indexnow.org/documentation and FAQ https://www.indexnow.org/faq (checked 2026-11-05: Google not listed;
up to 10,000 URLs per request; 200 and 202 accepted; resubmit only after a change). Google URL Inspection API
https://developers.google.com/webmaster-tools/v1/urlInspection.index/inspect and limits https://developers.google.com/webmaster-tools/limits.
Sitemap submission API https://developers.google.com/webmaster-tools/v1/sitemaps/submit. Cloudflare Crawler Hints
https://developers.cloudflare.com/cache/advanced-configuration/crawler-hints/. Cloudflare managed robots.txt
https://developers.cloudflare.com/bots/additional-configurations/managed-robots-txt/ (read through search results only). Bing import
https://blogs.bing.com/webmaster/september-2019/Import-sites-from-Search-Console-to-Bing-Webmaster-Tools. Not fetched in full: the Search
Console help pages, the Web Analytics pages and the DNSSEC and AI Crawl Control pages linked above; the menu names come from
those pages and from `docs/OPERATIONS.md`, and may have moved.
