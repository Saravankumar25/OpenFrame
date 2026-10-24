# Chapter 2: the product website

> Written 2026-10-09. The website is built after the application screens are stable (Chapter 1, streams 1 and 2). It may
> start in parallel for pages that need no screenshots. The earlier plan in [../website/README.md](../website/README.md) is
> kept as source notes; this page is the current plan and wins where they differ.

## 1. Goal

A small, fast, cheap-to-run static website for OpenFrame Studio, hosted on Cloudflare Pages, that tells the truth about the
product: what it does, how to start, how to get help, and what the legal terms are.

## 2. Pages

| Page | Contents | Source of truth |
|---|---|---|
| Home | One clear sentence about the product, three or four real screens, the trial and price once Chapter 3 decides them | [vision-and-scope.md](../product/vision-and-scope.md) |
| Features | One section per Verified feature group: Idea Vault, Story Board, Screenplay, Breakdown, Schedule, Call sheets, Import and export, Offline AI | [feature-status.md](../implementation/feature-status.md), Verified rows only |
| Tutorials | Short step-by-step guides that follow real workflows: write a scene, build a screenplay from the Story Board, import a Final Draft file, make a breakdown, plan a shooting day, make a call sheet, back up and restore | The Verified end-to-end specs in `tests/e2e/specs/` |
| Onboarding | Install, first launch, where files live, how to back up, the SmartScreen or Gatekeeper warning while builds are unsigned | The internal test plan and the data note screens |
| Support and contact | One address, expected reply time, what to send (a diagnostics file the person exports), known issues | Needs the support address (owner task) |
| Privacy Policy | What leaves the computer (nothing automatically), what the AI does (runs locally), logs | [../store/privacy-policy.md](../store/privacy-policy.md) after counsel review |
| Terms and Conditions | The end-user licence terms, including the Gemma use restrictions that must be passed on | Written with counsel (owner task) |
| Updates | Release notes per version | The release record |

## 3. Faithful screens, not invented ones

The owner asked for HTML recreations of the real interface. The existing `HTML_Mockups/` folder (181 screens) is a design
reference, not the product. It was not compared with the built screens by any test, and some of its screens show features
that do not exist: mock-ups 164 to 168 (live LAN collaboration, removed) and 155 (permissions, not in the interface).

Recommended method, in order:

1. Run the real application with the invented demo project "Night Bus" from the end-to-end suite, at one fixed window size.
2. Capture each wanted screen as a static page: the rendered HTML and the compiled stylesheet, with scripts removed and the
   theme tokens kept, so the recreation uses the real design system. This reuses the screen capture step that
   `tests/e2e/specs/90-visual-baseline.test.mjs` already performs.
3. Check every recreation against a screenshot of the same state. A recreation that cannot be matched is not published.
4. Add only light interaction (tabs, a hover, an opened menu) with plain HTML and a few lines of script.
5. Mark any screen that shows a state not yet built as "planned" or leave it out.

This keeps the website honest and cheap to update: when a screen changes, capture it again.

## 4. Technology and cost

- Static files only. Plain HTML and CSS with a very small amount of script is enough for this site. A static site
  generator is allowed if it removes real work (for example shared headers), but no framework is added for fashion.
- Hosting: Cloudflare Pages. The free plan serves static files with no bandwidth charge; limits (builds per month,
  custom domains) are to be checked on Cloudflare's current pricing page before launch.
- Domain and company email (SPF, DKIM, DMARC) are owner tasks.
- No analytics unless disclosed, per [ADR-0021](../adr/ADR-0021-diagnostics-telemetry-analytics-policy.md).
- Quality bars from the earlier plan stay: keyboard use, visible focus, contrast, reduced motion, responsive widths from
  375 to 1440 pixels, good loading speed, security headers, a sitemap.

## 5. Dependencies

1. Stable screens: Chapter 1 stream 2 (the core is proved) and the wording change away from "filmmakers".
2. Owner tasks: domain, support address, counsel review of the policy and terms.
3. Decisions: price display on the site (the site may show a price; the application does not), refund wording, download
   links (Chapter 3 decides what is public; the internal builds are not offered on the public site).

## 6. Verification and completion

- Every claim maps to a Verified row (checked by reading the page text against the feature status file).
- Every recreated screen has a matching screenshot.
- The link check passes, the page passes an accessibility check, and the site loads quickly on a throttled mobile profile.
- The owner approves the text and the design before it is published.

## 7. Reference material

`docs/website/prompts/` holds two supplied prompts. They are method notes only. The tool names in them were never
verified, so nothing is installed from them and no source code is sent to a third party.
