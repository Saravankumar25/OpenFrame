# Website security

Audit of 2026-10-31 by the `seo-sec` agent. Scope: the static site in this repository and its Cloudflare Pages deployment
(project `openframe-studio`). The product apps are out of scope. The checks below are repeated by `tests/seo.test.mjs` on every
build, so this page is the explanation and the test is the guard.

## 1. What the site is, and what that means for risk

A static site: HTML, one CSS file, one script file, fonts and pictures, served by Cloudflare Pages. There is no server code
(no `functions/` folder, no `_worker.js`, no `wrangler.toml`), no forms, no accounts, no cookies, no API endpoints and no
database. The only script on the pages runs in the visitor's browser. Search runs in the browser over a static JSON file.
So the attack surface is: what the browser is told to trust (headers, CSP), what the build accidentally publishes, the
deployment credentials, and the supply chain of the build tools.

## 2. Findings

Severity is the effect if it were exploited. "Fixed" means changed in this work and covered by a test.

| # | Finding | Severity | Status |
|---|---|---|---|
| 1 | **The GitHub repository `Saravankumar25/Straft` is public** (checked with `gh api`, `private: false`). It holds `mac/`, `windows/` and `website/`. `website/research/facts.json`, `capability-matrix.md` and the brief name the technologies, and `CLAUDE.md` says the repository stays private. Public history cannot be recalled by making it private later, but making it private stops further exposure. | High (owner rule broken, source exposed) | **Accepted by the owner on 2026-10-31: the repository stays public.** Keep secrets, tokens and keys out of every commit. |
| 2 | `dist/tutorial-screen-map.json` was published. It lists internal screen file names for every tutorial. Nothing used it (the copy in `docs/` is the one the docs cite). | Low | Fixed: no longer written to the build. |
| 3 | CSP did not allow Cloudflare Web Analytics, and HSTS used `includeSubDomains`. | Low | Fixed, section 3. |
| 4 | `scripts/deploy.mjs` passed the branch name from the command line into a shell command (`shell: true` on Windows). | Medium (local only: the person running the deploy) | Fixed: only `A-Z a-z 0-9 . _ -` accepted. |
| 5 | Nothing stopped a deploy from carrying a private folder, a key or a saved token. | Medium | Fixed: `scripts/dist-guard.mjs` runs inside `deploy.mjs` and blocks it, including a check that the saved Cloudflare token text is nowhere in `dist/`. |
| 6 | `npm audit`: `sharp` (high, three advisories in the image libraries it bundles: libvips, libheif, librsvg). `sharp` is a build-time tool that only reads our own pictures. It never runs for visitors and never ships. | Low in practice | Open: upgrade to 0.35.5 (a breaking release) once the picture pipeline owner has finished, then run the picture build again. |
| 7 | Dependencies: the three `@fontsource-variable/*` packages are not imported by any script. They are the origin of the font files copied into `src/assets/fonts/`. | Info | Keep as provenance or remove after noting the version in `src/assets/fonts/`. Not changed. |
| 8 | `security.txt` names `/contact/` as the contact, and that page says the site has no contact form. There is no real security mailbox yet. | Low | Open, needs the company email (Chapter 2). Replace `Contact:` with `mailto:` then. |
| 9 | `deploy.mjs` runs `npx -y wrangler@4.151.3`. The version is pinned, but npx fetches it at deploy time without a lockfile hash. | Low | Open: add `wrangler` as a devDependency so `package-lock.json` pins its integrity hash, then call it from `node_modules`. |
| 10 | Secrets: the 139 tracked files and everything in the build were scanned for private keys, AWS, GitHub, Slack and generic bearer tokens, the Cloudflare variable names, `.env` files and source maps. None found. `.gitignore` covers `dist/`, `.cache/`, `.wrangler/`, `manuals/`, `.tmp/`. | none | Clean. Test: "repository: nothing secret or private is tracked". |
| 11 | Private material never reaches `dist/`: the build copies only `src/static/`, the built assets and the generated pages. `research/`, `manuals/`, `.cache/` and `docs/` are not copied. The PDF manuals are internal and `dist-guard` rejects `.pdf` and `.docx` anywhere in the build. | none | Clean, enforced. |
| 12 | `_redirects`: every rule is a fixed path to a fixed path on this site. No `:splat`, no host, no query forwarding, so it cannot be used as an open redirect. | none | Clean, enforced. |
| 13 | Mixed content: no `http://` resource or link; `upgrade-insecure-requests` is in the CSP. No resource comes from another site. | none | Clean, enforced. |
| 14 | `/search/?q=` puts the visitor's text into the page. The search script escapes it before inserting it, and the CSP (no `unsafe-inline`, no `unsafe-eval`) would stop an injected handler anyway. Tested by loading a hostile query. | none | Clean. |
| 15 | Preview deployments of other branches get public `*.pages.dev` addresses. Cloudflare marks them noindex, but anyone with the link can read them. | Low | Dashboard setting, section 6. |
| 16 | The footer company logo is a 175 KB picture (1774 by 887) used at a small size on every page. | Performance, not security | Listed in `docs/SEO.md`. |

## 3. Response headers (`_headers`, built by `scripts/seo-files.mjs`)

| Header | Value | Why |
|---|---|---|
| Strict-Transport-Security | `max-age=31536000` | One year. No `preload` (the preload list cannot be left quickly). No `includeSubDomains` until every subdomain of the final domain is known to serve HTTPS only: add it then. |
| Content-Security-Policy | see below | Scripts only from this site plus the one hashed boot script. |
| X-Content-Type-Options | `nosniff` | No MIME guessing. |
| X-Frame-Options | `DENY` | Old browsers. No page is framed any more, so `frame-ancestors 'none'` is safe everywhere. |
| Referrer-Policy | `strict-origin-when-cross-origin` | Other sites see only the origin. |
| Permissions-Policy | camera, microphone, geolocation, payment, usb, bluetooth, serial, hid, midi, sensors, display capture, clipboard read, wake lock, idle detection and browsing topics all off | The site uses none of them. |
| Cross-Origin-Opener-Policy | `same-origin` | Isolates the browsing context group. |
| Cross-Origin-Resource-Policy | `same-origin`; `cross-origin` for `/og.png` only | Other sites cannot embed our files, but link previews can still show the Open Graph picture. |
| X-Permitted-Cross-Domain-Policies | `none` | No Flash or PDF policy files. |

Not set on purpose: Cross-Origin-Embedder-Policy (it gives nothing without cross-origin isolation and could block the
Cloudflare Analytics script).

CSP, one line in the file:
`default-src 'self'; script-src 'self' 'sha256-<boot script> ' https://static.cloudflareinsights.com; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self' https://cloudflareinsights.com; manifest-src 'self'; frame-src 'none'; object-src 'none'; worker-src 'none'; base-uri 'self'; form-action 'none'; frame-ancestors 'none'; upgrade-insecure-requests`

* The boot script hash is computed from `BOOT` in `src/lib/layout.mjs` at build time, so the hash cannot drift. A test checks
  that every page has exactly one inline script and that its hash is the one allowed.
* `img-src data:` is needed for one small icon drawn by the stylesheet.
* The two Cloudflare hosts are allowed so that Web Analytics works when the owner switches it on. Nothing loads from them
  until then.

### Verifying Web Analytics after the owner switches it on

Do this before relying on it. The privacy notice currently says the site runs no analytics and loads nothing from other
companies: update `src/pages/legal.mjs` (and `docs/SEO.md` owner list) in the same change, or do not switch it on.

1. In the Cloudflare dashboard, Pages, `openframe-studio`, Settings, Web Analytics, enable it. Cloudflare then adds
   `<script defer src="https://static.cloudflareinsights.com/beacon.min.js" data-cf-beacon='{...}'>` to each page when it is
   served. The page needs no change.
2. Safe check with Report-Only. In `scripts/seo-files.mjs` add one more line under `/*`:
   `Content-Security-Policy-Report-Only: <the same policy>` (copy of the enforced one, so it reports without blocking
   anything new). Deploy to a preview branch (`node scripts/deploy.mjs check-analytics`).
3. Open the preview in Chrome, DevTools, Console. A policy problem prints `Refused to load the script ...` or
   `Refused to connect to ...` with the directive name. No such line means the CSP is right. In the Network tab, the
   request to `cloudflareinsights.com/cdn-cgi/rum` should answer 204 or 200.
4. Remove the Report-Only line and deploy to production.

If a blocked host appears that is not listed above, add exactly that host to the matching directive in
`headersFile()` and update the assertion in `tests/seo.test.mjs` ("security headers").

## 4. Not applicable now, and when it starts to apply

The site has no forms, no endpoints and no Functions, so these are not needed and are deliberately absent:

| Control | Why not now | Add it when |
|---|---|---|
| Cloudflare Turnstile | Nothing to protect: no form, no sign-in, no download that costs money. | A contact or sign-up form, a waitlist, a feedback box, any page with a POST. |
| Rate limiting rules | Static files served from the edge cache; no expensive work per request. | Any Pages Function or Worker, any API (activation, licence check, download tokens in Chapter 3), a form endpoint. |
| Custom WAF rules | The free managed ruleset already covers generic web attacks on a static site. | A dynamic endpoint exists, or logs show a pattern of abuse (scrapers, credential stuffing, path scanning) that the managed rules miss. |
| CORS headers | Nothing is fetched from another origin. | A public API or font and image hosting for other sites. |
| CSRF protection, session cookies, input validation | No state, no cookies, no input that leaves the browser. | Accounts or forms. |

The test "pages are strict-CSP clean" fails if a `<form>` appears, and the repository test fails if `functions/`,
`_worker.js` or `wrangler.toml` appear, so adding any of them forces a look at this table first.

Abuse risk of the deployment itself: Cloudflare Pages static requests are not billed, so traffic spikes cost nothing, but the
daily 100,000 request limit applies to Functions only. There is nothing to run up a bill with.

## 5. Cloudflare Pages limits against the build

| Limit | Value | This build |
|---|---|---|
| Files per deployment | 20,000 | see "Size" below |
| Size of one file | 25 MiB | the largest file is far below it |
| `_headers` | 100 rules, 2,000 characters per line | 16 rules, the longest line (the CSP) about 430 characters |
| `_redirects` | 2,000 static and 100 dynamic | about 100 static, 0 dynamic |

Size: the picture change grows the build from about 800 files to a few thousand (each screen picture exists in several sizes,
two themes and two platforms). The tests fail the build at 20,000 files and `deploy.mjs` blocks it too; a warning appears
from 15,000. If a later change needs more, cut the number of picture sizes first (each screen's largest size is only needed
when the picture is opened), not the pages.

## 6. Cloudflare dashboard changes (owner or whoever has the account)

None of these can be done from the repository. Order matters only where stated.

**Account**
1. Turn on two-factor sign-in for the Cloudflare account. Use an authenticator app or a hardware key.
2. The deploy token (`CLOUDFLARE_API_TOKEN`) should be an API token limited to "Cloudflare Pages: Edit" on this one
   account, with an expiry date (for example 6 months), never the Global API Key. Rotate it if it was ever pasted in a chat
   or a file. It lives only in the Windows user environment variables.
3. Turn on notifications: deployment failed, and certificate expiry (once the custom domain exists).

**Pages project `openframe-studio`**
4. Settings, General, Access policy: enable it for preview deployments, so non-production branches are not public.
5. Settings, Builds, Branch control: keep `main` as the production branch.
6. Settings, Web Analytics: leave off until the privacy notice is updated and the check in section 3 is done.

**When the custom domain is added**
7. Add it under Custom domains (the DNS record is created by Cloudflare if the domain is in the account).
8. SSL/TLS: mode **Full (strict)**; Always Use HTTPS **on**; Minimum TLS **1.2**; TLS 1.3 **on**; Automatic HTTPS Rewrites **on**.
9. DNS: turn on **DNSSEC** and add the DS record at the registrar. Add a **CAA** record only after checking the issuers that
   Cloudflare's certificate list names, or leave CAA alone.
10. HSTS: the header in `_headers` is enough. Do not also enable HSTS in the SSL/TLS dashboard (two sources can disagree).
    Decide on `includeSubDomains` only when every subdomain, including mail hosts that serve web pages, is HTTPS only.
11. Security: **Bot Fight Mode on**. Under Security, Settings, leave **Browser Integrity Check on**. Keep the free managed
    WAF ruleset on.
12. AI crawlers: **"Block AI bots" must stay off** if answer engines should be able to read the site (the GEO goal in
    `docs/SEO.md`). Cloudflare can also add a content-signals block to `robots.txt` for the zone. Decide the policy on
    training use, then see `docs/SEO.md`. Check the live `robots.txt` after the domain is attached: Cloudflare may prepend
    lines to ours.
13. Speed: Brotli on (default), HTTP/3 on, **Early Hints on**. Leave Rocket Loader and Auto Minify off (they rewrite scripts
    and break the CSP hash).
14. Email (when the company email is set up): SPF, DKIM and a DMARC record (start with `p=quarantine`, move to `p=reject`),
    MTA-STS, and a mailbox for `security@` that replaces the `Contact:` line in `security.txt`.
15. Redirect the `pages.dev` address to the custom domain if Cloudflare offers a switch for it; otherwise the canonical
    tags already tell search engines which address counts. Rebuild with `OF_SITE_URL=https://<domain>` first
    (see `docs/SEO.md`, "Moving to the custom domain").

## 7. Deployment hygiene

* `npm run deploy` runs `scripts/deploy.mjs`. It reads the token from the environment (or, on Windows, from the saved
  user variable through PowerShell), passes it only to the wrangler process, and never prints it. It refuses to deploy if
  `dist/` holds a private folder, key, `.env`, source map, PDF, a secret-shaped string, a file over 25 MiB or more than 20,000
  files.
* Run the build with no `OF_DIST` for a deploy, so `dist/` is the full build.
* Never add `functions/`, `_worker.js` or `wrangler.toml` to this repository without updating sections 4 and 5.
* Repeat `npm audit` when dependencies change. `npm test` runs `tests/seo.test.mjs` against the build.
