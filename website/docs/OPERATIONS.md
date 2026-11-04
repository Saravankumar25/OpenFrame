# Operations: build, test, deploy, roll back, and the Cloudflare plan

Written 2026-10-31. This file is internal. It never ships on the website. Facts about Cloudflare were checked against
Cloudflare's current documentation on this date; each section lists its pages. Cloudflare changes its dashboard often,
so when a menu name below does not match what you see, search the dashboard for the setting name.

Names of secrets appear here. Values never do, and they must never be pasted into this repository.

## 1. Build, test, deploy

Run these from the website folder.

| Step | Command | What it does |
|---|---|---|
| Build | `npm run build` | Renders every page into `dist/`: HTML, CSS, JS, `_headers`, `_redirects`, `sitemap.xml`, `robots.txt`, `llms.txt`, `404.html`. |
| Test | `npm test` | Runs every file in `tests/` against `dist/`: static checks, content checks, browser checks, accessibility. Build first. |
| Deploy | `npm run deploy` | Uploads `dist/` to the Cloudflare Pages project `straft-studio`, production branch `main`. A push to `main` on GitHub does this by itself (section 9). |
| Preview | `node scripts/deploy.mjs preview` | Uploads the same folder as a preview deployment on a branch named `preview`. Not the live site. |

Release order: `npm run build`, then `npm test`, then `npm run deploy`. Never deploy with a red test.

### Credentials

`scripts/deploy.mjs` needs two values and reads them from the environment. On Windows, when they are not set in the
current shell, it reads them from the Windows user environment variables:

- `CLOUDFLARE_API_TOKEN`: a custom API token with one permission, Account, Cloudflare Pages, Edit.
- `CLOUDFLARE_ACCOUNT_ID`: the account id shown on the Cloudflare dashboard.

To check that they exist without printing them: `[Environment]::GetEnvironmentVariable('CLOUDFLARE_API_TOKEN','User') -ne $null`
in PowerShell. To change one: Windows Settings, search "environment variables", edit the user variable, open a new
terminal. If a token leaks, delete it in the Cloudflare dashboard (My Profile, API Tokens), create a new one with the
same single permission, and replace the user variable. Keep the token this narrow. DNS, domain and security settings
are changed by hand in the dashboard (section 3), so the token never needs those permissions.

### Notes on the deploy script

- It runs a pinned Wrangler version (`wrangler@4.151.3`) through `npx`, so a new Wrangler release cannot change behaviour
  under us. Update the pin on purpose, after a preview deploy works.
- It passes `--commit-dirty=true` because the site folder has uncommitted work during development. Direct upload does
  not use Cloudflare's own build servers, so the monthly build limit on the Free plan does not apply to us.
- Do not add `--force`. It was needed once, to create the project, and is not needed again.
- Pages sends the files named `_headers` and `_redirects` from the root of `dist/` as rules. The build generates both.

### Building for the custom domain

The public address lives in `site.config.mjs` and can be overridden with `OF_SITE_URL`. Canonical links, Open Graph
URLs, the sitemap, `robots.txt` and `llms.txt` all follow it. After the domain is live:

```
$env:OF_SITE_URL = 'https://<domain>'
npm run build; npm test; npm run deploy
```

Then change the default in `site.config.mjs` so every later build uses the domain without the variable.

Store links work the same way: `OF_STORE_WIN` and `OF_STORE_MAC`. Until they are set the store buttons stay enabled and
do nothing.

## 2. Roll back a bad deployment

Pages keeps every deployment. Production points at one of them, and you can move the pointer.

1. Cloudflare dashboard, Workers & Pages, `straft-studio`, Deployments.
2. In "All deployments", find the last good production deployment. Open its three-dot menu.
3. Choose "Rollback to this deployment" and confirm. Production switches at once.
4. Open the live site in a private window and check the home page, `/features/`, and one tutorial.

Rules from Cloudflare: only production deployments that built successfully can be rollback targets, and preview
deployments cannot. After a rollback you can still roll forward to a newer deployment from the same menu.

A rollback does not change the repository. Afterwards, fix the cause in the repository, build, test and deploy again,
or the next normal deploy will bring the bad change back.

Second way, from a known good commit: check out the commit, run the full build and test, and `npm run deploy`. This is
slower but leaves a clean record. To list deployments from the command line, run
`npx wrangler@4.151.3 pages deployment list --project-name straft-studio`. Wrangler's documentation does not list a
rollback command, so use the dashboard for the actual rollback.

Source: Cloudflare Pages, Rollbacks (https://developers.cloudflare.com/pages/configuration/rollbacks/) and Wrangler
Pages commands (https://developers.cloudflare.com/workers/wrangler/commands/pages/).

## 3. Regenerate pictures and icons

### Screen pictures

`scripts/shots.mjs` is being written by another work stream and did not exist when this file was written. When it
lands, read the comment at the top of the script for its options and fill this paragraph in. Until then the older
chain still works: `npm run screens` runs `scripts/extract-screens.mjs` and `scripts/render-screens.mjs`, which read the
captured app screens in `research/` and write the screen catalogs and thumbnails.

Capture sources: recapture WindowsHTML or MacHTML in the app project, copy them to `research/WindowsHTML-src` and
`research/MacHTML-src`, then run the screens step. A screen name that disappears makes `npm run build` and
`npm run check:content` fail and name it.

Pictures are optimised at build time. Do not turn on any paid image service in Cloudflare to compensate for large
pictures; shrink them in the script instead. Watch the file count too: the Free plan allows 20,000 files per Pages
site, and `dist/` held 8,621 files on 2026-10-31. Every new picture size or format adds files.

### Icons and the link preview picture

- `node scripts/make-icons.mjs` redraws the favicon set, the Apple touch icon, the 192 and 512 app icons and the
  maskable icon from the frame mark into `src/static/`. Run it only when the mark changes, then rebuild.
- `node scripts/make-og.mjs` redraws `src/static/og.png` (1200 by 630) from the brand line and one real screen. It needs
  `.cache/win-full`, which `scripts/render-screens.mjs` creates. Run it when the brand line changes.
- After either, build, run `npm test`, deploy, and check the link preview in a messaging app. Chat apps cache preview
  pictures for days, so a stale preview right after a deploy is normal.

## 4. The Cloudflare plan

Cloudflare is the primary infrastructure: hosting, domain, DNS, certificates, security rules and analytics. The goal is
a fast site that costs nothing to run, sets no cookies of its own, and has one canonical address.

### 4.1 Pages (live now)

- Project `straft-studio`, live at https://straftstudio.com, direct upload by the workflow in section 9 or from this folder.
- Static assets are free and unlimited, on Free and paid plans. Cost appears only if a Pages Function runs; there are
  none, and the site has no `functions/` folder. Keep it that way until a feature needs one.
- Pages caches by itself. For cacheable requests it sends `public, max-age=0, must-revalidate` with an ETag, so
  browsers revalidate and get a 304 when nothing changed. Our `_headers` overrides this for `/assets/*` with a one-year
  `immutable` cache, which is safe because those file names carry a content hash. Brotli and Gzip are on by default.
- Clean URLs are automatic: `/page.html` redirects to `/page`, and `/about/index.html` to `/about/`. The closest
  `404.html` is used for unknown paths.
- Preview deployments get `X-Robots-Tag: noindex` from Pages, so previews do not enter search results.

Source: https://developers.cloudflare.com/pages/configuration/serving-pages/ and
https://developers.cloudflare.com/pages/functions/pricing/.

### 4.2 Registrar and DNS (when the domain is bought)

Where to buy: Cloudflare Registrar sells at cost with no markup, includes redacted WHOIS, and turns DNSSEC on with one
click. Its list of supported endings is on https://www.cloudflare.com/tld-policies/. On 2026-10-31 that page lists
`.com`, `.app`, `.studio` and `.io`, and does not list `.in`, `.film` or `.co.in`. If the owner wants a `.in` name, buy it
at another registrar and then point its nameservers to Cloudflare. A domain that is not on Cloudflare nameservers
cannot use an apex domain with Pages.

One canonical host. Pick it once and never change it. Recommendation: the apex (`example.com`), with `www` redirecting
to it. Search engines then see one address, links and the sitemap agree, and the earlier work on canonical URLs stays
simple.

Steps:

1. Buy the domain on Cloudflare Registrar (or add an existing domain as a zone and switch its nameservers).
2. Workers & Pages, `straft-studio`, Custom domains, Set up a domain. Add the apex first. Cloudflare creates the
   CNAME to `straft-studio.pages.dev` itself. Then add `www` the same way.
   Add the domain in the Pages screen before touching DNS: a hand-made CNAME that Pages does not know about gives a
   522 error.
3. Check CAA records. If the zone has any, they must allow Cloudflare's certificate authorities (Let's Encrypt,
   Google Trust Services, SSL.com). A new Cloudflare zone has none, which is fine.
4. Redirect `www` to the apex: Rules, Redirect Rules, Create rule. Match requests whose hostname is `www.<domain>`,
   target `https://<domain>/${1}` style dynamic URL (or the wildcard pattern `https://www.<domain>/*` to
   `https://<domain>/${1}`), status 301, keep the query string. Both hostnames must be proxied (orange cloud), which
   they are when Pages created them. The Free plan allows 10 single redirect rules per zone, with wildcards but no
   regular expressions. Use the redirect simulator before deploying the rule.
5. Redirect the old `straft-studio.pages.dev` address to the domain with an account-level Bulk Redirect (Free plan:
   15 rules, 5 lists, 10,000 redirects per account). Cloudflare names this as the way to retire `*.pages.dev`. Single
   Redirects run before Bulk Redirects, and Bulk Redirects run after the WAF.
6. Rebuild with `OF_SITE_URL` (section 1), test, deploy.
7. Do not change a custom domain's DNS record away from Pages, even briefly. It deactivates the domain, and restoring it
   shows errors until it reactivates. For a temporary redirect use a redirect rule instead.

Sources: https://developers.cloudflare.com/pages/configuration/custom-domains/,
https://developers.cloudflare.com/rules/url-forwarding/ and
https://developers.cloudflare.com/rules/url-forwarding/single-redirects/create-dashboard/.

### 4.3 Encryption settings (after the domain is on Cloudflare)

- Always Use HTTPS: SSL/TLS, Edge Certificates, turn on Always Use HTTPS. It redirects HTTP to HTTPS for every
  hostname in the zone. It needs the SSL/TLS mode to be anything other than Off. Use Full (strict) mode where it
  is offered; Pages hostnames already have valid certificates.
- HSTS: the site already sends `Strict-Transport-Security: max-age=31536000; includeSubDomains` from `_headers`, so the
  dashboard switch is optional. If it is turned on as well, keep the same twelve months. `includeSubDomains` makes
  every subdomain unreachable over plain HTTP, including the app domain and any email-tracking subdomains, so confirm
  they all serve HTTPS first. Leave Preload off until the owner is certain, because removal from browser preload lists
  takes months. Never move the zone away from Cloudflare or turn proxying off while HSTS is active.
- Minimum TLS version: SSL/TLS, Edge Certificates, Minimum TLS Version. Recommendation: TLS 1.2, with TLS 1.3 left on.
  This is our recommendation, not a Cloudflare default. Raise it to 1.3 only if analytics later show no visitors on 1.2.
- DNSSEC: Registrar or DNS, Settings, turn on DNSSEC. If the domain is registered elsewhere, copy the DS record to that
  registrar.
- Rocket Loader: keep it off. It rewrites script tags, and our CSP only allows our own scripts and one hashed boot
  script. This is reasoning from how our policy works; test it first if anyone proposes enabling it.

Sources: https://developers.cloudflare.com/ssl/edge-certificates/additional-options/always-use-https/,
https://developers.cloudflare.com/ssl/edge-certificates/additional-options/http-strict-transport-security/ and
https://developers.cloudflare.com/ssl/edge-certificates/additional-options/minimum-tls/.

### 4.4 Web Analytics (cookie-free)

What it is: Cloudflare's free analytics for page views, visits, countries, pages, referrers and Core Web Vitals. Cloudflare
states that it does not use client-side state such as cookies or localStorage to track users, and does not identify
individuals by IP address, user agent or other data. Cloudflare's own Web Analytics pages give no statement about
cookie banners, so whether a banner is needed is a legal question for the owner's adviser. The site's privacy page
(owned by the legal work) must describe whatever is switched on.

How to switch it on for a Pages project:

1. Workers & Pages, `straft-studio`, Metrics, Web Analytics, Enable.
2. Cloudflare adds its snippet on the next deployment, so run `npm run deploy` once afterwards.
3. Read the reports under Web Analytics in the dashboard. Cloudflare keeps six months of data.

Content Security Policy: the live policy on 2026-10-31 allows scripts only from this site (`script-src 'self'` plus
the hash of the boot script) and `connect-src 'self'`. That does not allow the Cloudflare beacon. The instruction from
the brief that "the CSP already allows it" did not match the build output when checked. Cloudflare's documentation says:
add `https://static.cloudflareinsights.com/beacon.min.js` (the host is enough) to `script-src`; for the automatic setup
that reports to the site's own `/cdn-cgi/rum` path, `connect-src 'self'` already covers it; for the manual setup add
`cloudflareinsights.com` to `connect-src`. The CSP lives in `scripts/build.mjs` and belongs to the seo-sec work. The
change is to be made there and checked first with a `Content-Security-Policy-Report-Only` header, as Cloudflare
advises, and then confirmed in the browser console with no violation on the live site.

Free-plan notes: there is a soft limit of ten sites per account. Real-user metrics in Speed Observatory leave out
traffic from the EEA, the UK and Switzerland on the Free plan.

Sources: https://developers.cloudflare.com/web-analytics/get-started/,
https://developers.cloudflare.com/web-analytics/faq/ and
https://blog.cloudflare.com/free-privacy-first-analytics-for-a-better-web/.

### 4.5 Speed Observatory

Dashboard, Speed, Observatory (still marked beta). It runs a Lighthouse test in a headless browser against a page and
lists suggestions, and it shows real-user data including Interaction to Next Paint. Run it for the home page, a feature
page and a tutorial page after each large change and keep the numbers in the release notes. Cloudflare's page does not
say how many tests the Free plan allows or whether scheduled tests are available, so check what the dashboard offers.
Do not follow suggestions that turn on paid features or script-rewriting features (Rocket Loader).

Source: https://developers.cloudflare.com/speed/observatory/.

### 4.6 CDN and caching

Nothing to configure. Pages already serves from Cloudflare's network with Brotli, ETag revalidation and the immutable
cache we set for hashed assets. Do not add Cache Rules for HTML: stale pages after a deploy are the usual harm. Cache
Rules suit only immutable, hash-named files, which `_headers` already covers. If an old file lingers, purge the cache
from the dashboard.

No paid image services. Pictures are made the right size at build time with `sharp`, in the screen and icon scripts. If
a page feels heavy, fix the source file, not the delivery.

### 4.7 Bot Fight Mode and free managed WAF rules

- Free Managed Ruleset: the only managed ruleset on the Free plan. Cloudflare's page says it is available on all plans
  and does not say whether it is on by default, so open Security, WAF, Managed rules and make sure it is deployed.
  The Cloudflare Managed Ruleset and OWASP ruleset need a paid plan.
- Bot Fight Mode: Security, Settings, filter by Bot traffic, turn Bot fight mode on. It challenges traffic that matches
  known bot patterns. Limits to know: it applies to the whole domain, cannot be customised, and WAF Skip or Allow
  rules and Page Rules cannot bypass it. Cloudflare's page does not say how it treats search engine crawlers, so
  after enabling it, check the Search Console crawl stats and the Security Events log for blocked Googlebot or
  Bingbot requests during the first week. If crawlers are challenged, turn it off.
- Cookies: Bot Fight Mode adds JavaScript detections that Cloudflare always runs, and its cookies (`cf_clearance`,
  `__cf_bm`) are listed by Cloudflare as strictly necessary. The scripts load from `/cdn-cgi/challenge-platform/` on
  our own host, which `script-src 'self'` already covers. Because cookies can appear, the privacy page and any "we set
  no cookies" claim on the public site must be reviewed before Bot Fight Mode goes on. Say so to the legal work
  stream.

Sources: https://developers.cloudflare.com/waf/managed-rules/,
https://developers.cloudflare.com/bots/get-started/free/,
https://developers.cloudflare.com/bots/additional-configurations/javascript-detections/ and
https://developers.cloudflare.com/fundamentals/reference/policies-compliances/cloudflare-cookies/.

### 4.8 Turnstile and Workers: only when needed

Neither is used today. The site has no form (the CSP says `form-action 'none'`) and no server code.

- Turnstile is Cloudflare's CAPTCHA alternative. Add it only when a form exists, such as a contact form or a waitlist.
  On the Free plan: 20 widgets per account, 10 hostnames per widget, unlimited challenges. The page's script and
  frame hosts must then be allowed in the CSP, and `form-action` must name the receiving endpoint.
- Workers or Pages Functions are needed only for server work: receiving a form, sending a transactional email, an
  activation check. On the Free plan: 100,000 requests a day shared across Workers and Functions, 10 ms CPU per
  request, 50 subrequests. Over the daily limit, requests fail with error 1027 unless the route fails open. Plan any
  such feature with those limits in mind, and prefer a separate Worker on a separate subdomain over adding a
  `functions/` folder to the marketing site.

Sources: https://developers.cloudflare.com/turnstile/, https://developers.cloudflare.com/turnstile/plans/ and
https://developers.cloudflare.com/workers/platform/limits/.

### 4.9 Free plan limits, and what could cost money

Limits that apply to this site on the Free plan (https://developers.cloudflare.com/pages/platform/limits/):

| Limit | Free plan | Where we stand |
|---|---|---|
| Files per Pages site | 20,000 | 8,621 on 2026-10-31. Watch it when screen pictures are added. |
| Size of one file | 25 MiB | No file is near it. Keep video out. |
| `_headers` rules | 100 rules, 2,000 characters per header | About 10 rules. The CSP line is long, so watch its length. |
| `_redirects` | 2,000 static and 100 dynamic | A few dozen. |
| Custom domains per project | 100 | 2 planned. |
| Builds on Cloudflare's servers | 500 a month | Not used (direct upload). |
| Single Redirect rules | 10 per zone | 1 planned. |
| Bulk Redirect rules | 15 per account, 5 lists | 1 planned. |
| Turnstile widgets | 20 | 0. |
| Workers and Functions | 100,000 requests a day | 0. |

Things that could cost money. None is on by default; do not switch any of them on without the owner's word:

- A domain: the yearly registration at cost, renewing automatically unless auto-renew is switched off.
- Pages Functions or Workers beyond the free daily quota (paid Workers plan).
- Any Cloudflare image product, Advanced Certificate Manager, a Pro or Business plan, Super Bot Fight Mode, and the
  WAF managed rulesets that the Free plan does not include (the Cloudflare Managed Ruleset, the OWASP ruleset). Read
  the price shown in the dashboard before saying yes, and check Cloudflare's pricing page for anything not listed here.
- Email sending services (section 2 of `docs/ROADMAP.md`): the free tier of a provider has a cap, then charges.

### 4.10 Dashboard changes checklist (owner, tick as done)

Before the domain:

- [ ] Workers & Pages, `straft-studio`, Metrics: Web Analytics enabled (needs the CSP change in 4.4, then a deploy).
- [ ] Billing: confirm the account is on the Free plan and a payment method is attached only if the domain is bought here.

When the domain is bought:

- [x] Domain: straftstudio.com is an active zone on this Cloudflare account (owner, 2026-11-04). Auto-renew and WHOIS redaction: not checked.
- [ ] DNS: DNSSEC on.
- [ ] Pages custom domains: apex added, then `www` added, both show Active.
- [ ] Redirect Rule: `www` to apex, 301, query string kept. Tested with the simulator.
- [ ] Bulk Redirect: `straft-studio.pages.dev` to the domain.
- [ ] SSL/TLS: mode Full (strict) where offered; Always Use HTTPS on; Minimum TLS 1.2; TLS 1.3 on.
- [ ] HSTS: decision made (the site already sends it); Preload still off.
- [ ] Security: Free Managed Ruleset deployed; Bot Fight Mode on only after the privacy page covers its cookies.
- [ ] Speed, Observatory: first test of three pages saved.
- [x] Site rebuilt for https://straftstudio.com and deployed; `site.config.mjs` default changed.
- [ ] Search setup (section 5) started.

### 4.11 Domain automation and token permissions

Do not run `npm run domain:ensure` on `straftstudio.com` without the owner's word. It changes SSL/TLS settings and adds redirect rules. It never touches MX or TXT records, and it stops if a name already has other records.

`npm run domain:ensure` makes the Cloudflare side match this document. It only looks, then changes what differs, so it is
safe to run again. `npm run domain:verify` needs no credentials and checks the public site. `npm run deploy` runs the
verify step after every deploy.

What `ensure` keeps in place for `straftstudio.com`:

| Item | Setting |
|---|---|
| DNS | `@` and `www`, CNAME to `straft-studio.pages.dev`, proxied |
| Pages custom domains | apex and `www` on project `straft-studio`, both Active |
| `www` redirect | Single Redirect rule `www_to_apex`, 301, path kept, query string kept |
| SSL/TLS | mode Full (strict), Always Use HTTPS on, minimum TLS 1.2, TLS 1.3 on; DNSSEC on |
| Old address | account Bulk Redirect list `pages_dev_to_domain`: `straft-studio.pages.dev` to the apex, 301, path and query kept (official method, Pages docs "Disable access to `*.pages.dev` subdomain"). Preview addresses (`*.straft-studio.pages.dev`) are not touched |

Token permissions (token `straft-website`, kept in the user variables `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID`,
never in the repository):

| Scope | Permission | Used for |
|---|---|---|
| Account | Cloudflare Pages: Edit | deploy, custom domains |
| Zone | Zone: Read, DNS: Edit | find the zone, CNAME records |
| Zone | Single Redirect: Edit | `www` redirect |
| Zone | Zone Settings: Edit | SSL/TLS settings |
| Account | Account Filter Lists: Edit | the `pages_dev_to_domain` list |
| Account | Mass URL Redirects: Edit | the Bulk Redirect rule |

Limit the zone permissions to the one zone `straftstudio.com` and the account permissions to this account. The other
permissions on the token (analytics, cache, WAF, email routing and so on) are not used by these scripts.

A failed deploy or a failed live check ends with a non-zero exit code and a FAIL line. Automatic deploys: section 9.

Troubleshooting:

- A name shows `ENOTFOUND` right after DNS records were created: the resolver cached the empty answer. Wait about five
  minutes, or test with `curl --resolve <host>:443:<cloudflare ip>`.
- Pages domain stays Pending: check the CNAME is proxied and points to `straft-studio.pages.dev`; run `domain:ensure`.
- Error 522 on a custom hostname: the hostname is not added in Pages, or its record was changed away from Pages.
- Redirect loop on `www`: the rule must match only `www`; it must never redirect the apex.
- Recovery after a bad change: rerun `npm run domain:ensure`. A bad deploy: section 2.
## 5. Search engine setup, once the custom domain is live

Do these in order. Nothing here works well on the `pages.dev` address, because the canonical host is meant to be the
custom domain. Expect weeks, not days, before pages appear.

Before you start, open `https://<domain>/robots.txt` and `https://<domain>/sitemap.xml` in a browser. robots.txt must
allow crawling and name the sitemap on the custom domain. The sitemap must list only pages that return 200 on the
canonical host, with no `pages.dev` address in it.

1. Google Search Console (https://search.google.com/search-console).
   - Add a Domain property (not a URL prefix property). DNS is the only way to verify a Domain property. Choose the TXT
     method, copy the value that starts with `google-site-verification=`.
   - In Cloudflare DNS, add a TXT record with name `@` and that value. Leave the record in place forever; removing it
     un-verifies the site. Verification can take a day or two to be served; retry later if it fails.
   - Sitemaps, add `sitemap.xml`. Status must read Success.
   - URL Inspection for the key pages (home, features, pricing, download, a Windows and a Mac tutorial index): run the
     live test, then Request indexing. There is a daily quota and a request does not guarantee indexing. Do not
     repeat a request for the same page; Google says it does not speed anything up.
   - Pages report: check Indexed against Not indexed and read each reason once a week for the first month.
   - Experience, Core Web Vitals: this report fills only after enough real visits. Until then use Speed Observatory.
2. Bing Webmaster Tools (https://www.bing.com/webmasters). My Sites, Import, sign in with the Google account, pick the
   verified domain and its sitemap. Bing re-checks ownership through Search Console from time to time, so keep that
   access, or verify by DNS as a second method.
3. IndexNow. A script (`scripts/indexnow.mjs`) is being added by the SEO work. It needs a key file at the site root
   (`<key>.txt`, containing the key) and posts changed URLs to `https://api.indexnow.org/indexnow` or a participating
   engine. Run it after each deploy that changes pages. The IndexNow documentation does not mention Google, so treat it
   as a faster signal for Bing and the other participating engines; Google relies on the sitemap.
4. Check the basics by hand: view source on the live home page and confirm one `<title>`, one description, a canonical
   link on the custom domain, and the Open Graph tags. Fetch a missing page and confirm it returns the 404 page with
   a real 404 status.
5. Structured data. Paste a live URL into the Rich Results Test (https://search.google.com/test/rich-results) to see what
   Google reads and what it would show. Paste the same URL into the Schema Markup Validator (https://validator.schema.org)
   to check the markup against schema.org even where Google shows no rich result. Use both: a page can pass one and
   fail the other. Fix errors; treat warnings about optional fields as optional.
6. Performance. Run Speed Observatory and Lighthouse on three pages. Target: good Core Web Vitals on a mid-range phone
   (the hero animation decision in `docs/decisions/hero-animation-style.md` was made with this in mind).
7. Analytics choice: use Cloudflare Web Analytics, not Google Analytics 4, and do not install Google Tag Manager.
   Reasons: it sets no cookies of its own and needs no consent banner decision from us; it adds one small script
   from one host instead of a tag layer that can load anything; the privacy page can stay short and true, which
   matches a product whose pitch is that your work stays on your computer; and the page-view data is all this
   marketing site needs. What it does not do: funnels, custom events and long history (six months). If the owner later
   needs to see store-button clicks, add one measured link redirect at that time. Search Console already gives the
   search side.
8. Google Business Profile. Google's rule: a profile is for a business with a physical location customers can visit,
   or one that travels to customers in a service area. Virtual offices, mailbox addresses and P.O. boxes do not
   qualify. Straft Studio is sold online through two app stores, so unless the company has a staffed public-facing
   office, it probably does not qualify. This is the owner's decision; do not create a profile with an address that
   is not a real, staffed location. Source: https://support.google.com/business/answer/3038177.
9. Review and directory sites: G2, Capterra, Product Hunt, AlternativeTo, SaaSHub. Create a listing on each only when
   the product can be downloaded. Ask for reviews only from real users who have used the app, never write or buy
   them, and never list a review the site cannot show a source for. Listings without reviews are fine. Use the same
   name, description and prices everywhere, and link to the canonical domain.
10. After one month: compare Search Console impressions, Bing's report and Web Analytics. Write down what moved in the
    release notes.

Sources: https://support.google.com/webmasters/answer/9008080 (DNS verification),
https://developers.google.com/search/docs/crawling-indexing/ask-google-to-recrawl (indexing requests),
https://blogs.bing.com/webmaster/september-2019/Import-sites-from-Search-Console-to-Bing-Webmaster-Tools (Bing import) and
https://www.indexnow.org/documentation (IndexNow).

## 6. When the app changes

- New or changed screens: recapture, copy to `research/`, run `npm run screens`.
- Tutorials: edit the workflow in `src/content/manual/<win|mac>/<area>.mjs` (schema in `docs/CONTENT-BRIEF.md`), then
  `npm run check:content`. Which tutorials use a screen: `docs/tutorial-screen-map.json`.
- Features: `src/content/catalog.mjs` (every capability with its screens) and `src/content/features.mjs` (page copy).
- User manuals: `npm run manuals` writes two PDFs to `manuals/` (git-ignored). They are internal and never published
  or linked (owner decision 2026-10-31).

## 7. Open business inputs

- Microsoft Store and Mac App Store listing links.
- Custom domain and company email; a public contact address.
- Mac screens of Straft AI at work (the Mac capture had no engine installed).
- Owner direction on the hero animation and on real photography (see `docs/ROADMAP.md`).

## 8. The domain, email and the old project

The public address is one line: the `SITE_URL` default in `site.config.mjs`. To move again, change that line, change `SUPPORT_EMAIL`
in `src/content/nav.mjs`, and push. Canonical links, Open Graph URLs, the sitemap, `robots.txt` and `llms.txt` all follow.

`straftstudio.com` and `www.straftstudio.com` are custom domains on the Pages project, with one CNAME record each to
`straft-studio.pages.dev`. Google Workspace mail lives in the same zone. Never add, edit or delete its MX, SPF, DKIM or
verification records when working on the website. Official addresses: admin@, support@, contact@, developer@ and hello@straftstudio.com.

The old Cloudflare project `openframe-studio` and its domain `openframestudio.app` stay as they were and are no longer deployed
to. Removing them or redirecting them is the owner's decision.

## 9. Automatic deploys

A push to `main` that changes `website/` runs `.github/workflows/website-deploy.yml` at the root of the GitHub repository: it
builds, runs the tests and deploys. Only pushes to `main` and manual runs trigger it, so pull requests, including those from
forks, never see the secrets. The repository secrets are `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID`. Cloudflare's own
Git builds are not used, because the Cloudflare GitHub app is not authorised on this account, and a project made for direct
upload cannot switch to them later.

The build needs the screen pages in `.cache/shots` (about 675 MB), which are made on this machine and are not in Git. They are
kept as one file on a GitHub release, and `scripts/screens-bundle.json` names the tag, the file and its SHA-256. The workflow
downloads that file and refuses it if the checksum differs. After the screens change:

1. `npm run screens`, then `node scripts/sim-mac.mjs`.
2. `node scripts/pack-screens.mjs <next number>`. It replaces developer paths in the captured text, and refuses other file types, the
   old product name or a leftover developer path.
3. `gh release create website-screens-<number> .cache/website-screens-<number>.tar.gz --latest=false --title "Website screens <number>" --notes "Screen pages for the website build."`
4. Commit `scripts/screens-bundle.json` and push through a pull request.

To deploy again without a change, use Run workflow on the Actions page. To deploy from this machine, use `npm run build`, `npm test`, `npm run deploy`.
