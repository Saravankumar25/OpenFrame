# How Google shows straftstudio.com (address and icon)

Audit of 2026-11-05. Owner report: Google Search shows the home page as HTTP and shows a generic globe instead of the Straft Studio icon.

Short answer: the site is correct on every signal we can measure. Nothing on the site sends Google to HTTP. The globe and the HTTP address are an old copy in Google's index that has not been refreshed yet. Only Search Console can show what Google holds today.

## What was measured (live site, 2026-11-05)

| Check | Result |
|---|---|
| `http://straftstudio.com/`, `http://www.straftstudio.com/`, `https://www.straftstudio.com/`, `/index.html` | Each answers 301 to `https://straftstudio.com/` in one hop. `http://straftstudio.com/favicon.ico` also 301 to HTTPS |
| Home page | 200, `Cache-Control: public, max-age=0, must-revalidate` |
| `rel="canonical"` | One per page, equal to the page's own https URL. Sample of 100 live pages (every 7th sitemap URL, 300 ms apart): 0 problems |
| `og:url`, JSON-LD `url` and `@id`, sitemap | https only. The sitemap has 752 URLs, none on http or www, no hreflang, no `/search/` (the noindex page) |
| `http://` references | Scanned all 777 text files in `dist/` (757 HTML pages; the screen mockups are skipped). The only hits are licence text: `http://scripts.sil.org/OFL`, `http://github.com/productiontype/Newsreader` (font licences) and the XML namespace URLs in the sitemap. None is a link a crawler follows. `llms.txt`, `site.webmanifest`, CSS and JS: none |
| HSTS | `Strict-Transport-Security: max-age=31536000` on every https response. Not sent on http (correct, browsers ignore it there). No `includeSubDomains` or `preload`, left alone on purpose |
| Cloudflare | No challenge page. Googlebot, Googlebot-Image and a Google Favicon user agent get 200 on the home page and every icon from this machine. Icons are `cf-cache-status: HIT`. A user agent sent from this machine does not prove what real Google addresses receive |
| robots.txt | Allows everything except `/search-index.json` and `/mockups/`. The home page and every icon are allowed |
| `https://www.google.com/s2/favicons?domain=straftstudio.com&sz=64` | Returns a 64 px PNG of the Straft film-frame mark (orange gradient on the dark tile), redirected through `t1.gstatic.com/faviconV2?...url=http://straftstudio.com`. I viewed the image. Google's favicon service already has our icon. The `http://` in that URL is how that service names a host, not a link from our site |
| Google's index | Not readable without a login: a plain fetch of `google.com/search?q=site:straftstudio.com` returns a page that requires JavaScript. I did not try to get around it. Nothing here is a claim about Google's current results or its chosen canonical |

## Google's favicon requirements, one by one

Source: https://developers.google.com/search/docs/appearance/favicon-in-search

| Requirement | Status | Evidence |
|---|---|---|
| Declared on the home page, one favicon per hostname | Pass | Icons are in `<head>` of `https://straftstudio.com/`. Host is `straftstudio.com`; `www` redirects to it |
| Googlebot can crawl the home page and follow redirects | Pass | 200 for Googlebot user agents, not blocked by robots.txt, no noindex |
| Googlebot-Image can crawl the icon file | Pass | `/favicon.ico` and the PNGs answer 200 to a Googlebot-Image user agent, not blocked by robots.txt |
| Stable URL | Pass | `/favicon.ico` and the other paths have not changed |
| Square | Pass | Sizes 16, 32, 48 inside the ICO, PNGs 16, 32, 180, 192, 512. The SVG viewBox is 1024 by 1024 |
| At least 8x8, preferably larger than 48x48 | Gap fixed | Largest icon declared in `<head>` was the 48 px entry of the ICO. The 192 px PNG existed only in the manifest. Now declared in `<head>` |
| Supported format | Pass | ICO and PNG. SVG is not on Google's list (BMP, GIF, ICO, PNG, JPEG, PPM, TIFF). The SVG stays for browsers, and the ICO and PNGs cover Google |
| MIME type | Pass | `image/vnd.microsoft.icon`, `image/png`, `image/svg+xml`. Google's page names no MIME rule |
| Time | Not in our hands | Google says a change needs a recrawl and "can take" several days to several weeks. A favicon may not appear even when every rule is met |

## What was fixed

- `src/lib/layout.mjs`: added `<link rel="icon" href="/icon-192.png" type="image/png" sizes="192x192">` to the head of every page. It reuses the existing file, so there is no new art. Every page now offers Google an icon larger than 48 px.
- `tests/seo.test.mjs`: one test. Every indexable page links `/favicon.ico`, every `rel="icon"` target exists, and `icon-192.png` is square and larger than 48 px. It has not been run yet (see below).
- Not changed: `favicon.svg` cache (`max-age=14400, must-revalidate`) costs one cheap revalidation every four hours and has no effect on Google. No `http://` link needed fixing. `scripts/make-icons.mjs` and `src/static` icons are untouched.

The fix takes effect only after the next deploy (a push of `website/` to GitHub main) and a rebuild of `dist/`.

## Why Google can still show http and a globe when the site is right

Only as Google documents it:

- Canonical choice is a weighing of signals: redirects, `rel="canonical"`, the sitemap, hreflang. Google says these are suggestions and that HTTPS is preferred by default (https://developers.google.com/search/docs/crawling-indexing/consolidate-duplicate-urls). Google lists HTTPS-to-HTTP redirects, HTTP links and an invalid certificate as reasons it may keep HTTP. We have none of them now.
- Google must recrawl the home page before anything shown changes. Google says a crawl takes "a few days to a few weeks" and that asking again and again does not make it faster (https://developers.google.com/search/docs/crawling-indexing/ask-google-to-recrawl).
- The favicon is picked up from the home page when Google recrawls it, and Google says a favicon may not appear even when the guidelines are met. A globe is Google's default when it has no icon stored for that host.
- If the site was reachable on HTTP or without an icon at first crawl, Google can keep that record until it recrawls. I cannot see when Google last crawled or what it stored.

## What the owner does in Search Console

Search Console is the only place that shows the canonical Google chose. Home: https://search.google.com/search-console

1. Add a Domain property for `straftstudio.com`. Help: https://support.google.com/webmasters/answer/34592. It asks for a DNS TXT record, which the owner adds in Cloudflare (we do not touch DNS). A URL-prefix property for `https://straftstudio.com/` is the alternative and verifies with an HTML file or tag.
2. Open URL Inspection on `https://straftstudio.com/`. Help: https://support.google.com/webmasters/answer/9012289. Read two lines under Page indexing: "User-declared canonical" (expect `https://straftstudio.com/`) and "Google-selected canonical". If the selected one is `http://straftstudio.com/`, the index is stale or Google is choosing differently. Write down what it shows.
3. Click "Test live URL", then "Request indexing" (after the deploy of this fix). Quota is small and one request is enough.
4. Sitemaps: submit `https://straftstudio.com/sitemap.xml`. Help: https://support.google.com/webmasters/answer/7451001. Expect "Success" and about 752 discovered pages.
5. Favicon: in the live test, open "View tested page", then the screenshot and "More info". Also open `https://straftstudio.com/favicon.ico` in a browser. If Google still shows the globe after about three weeks, use the favicon feedback form linked from the Google favicon page.

## What to expect and how to tell it worked

- Within days to a few weeks, URL Inspection shows a Google-selected canonical of `https://straftstudio.com/` and a recent crawl date.
- Search results show the https address and, after Google refreshes its icon store, the Straft frame mark in place of the globe. The icon often lags the address.
- Check from a normal browser: search `straftstudio.com` and look at the result. Or open `https://www.google.com/s2/favicons?domain=straftstudio.com&sz=64`, which already shows our mark.
- If after several weeks the selected canonical is still http, send the URL Inspection screenshot. Then the cause is in Google's data, not on our site.

## Not verified

- Anything inside Google's index: the stored canonical, the stored icon, the last crawl date.
- How real Google IP addresses are treated by Cloudflare (user agents were tested from this machine only).
- The new test and the new `<head>` line were not built or run, because a build and a test run were not allowed on this machine at the time. Run `npm run build` and `npm test` before the deploy.
