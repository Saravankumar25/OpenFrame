# OpenFrame website plan

> Created 2026-10-05. The website is a launch dependency (privacy URL, support URL, download or Store link, policies,
> theme file downloads). It does not exist in this repository yet. Build plan area: Legal, privacy, website and support; launch register GTM-001 to 010.
> The two files in [prompts/](prompts) are the supplied Claude Code prompts. Use them as method, not as authority:
> every tool name in them (Ponytail, OmniRoute, GEO Optimizer and others) is **unverified**; install nothing from a
> name alone, and send no source code to a third-party provider.

## 1. What the site must contain before Store submission

| Page | Needed for | Source of truth |
|---|---|---|
| Home and product overview | Store listing consistency | Features marked Release-ready in the feature status file only |
| Features | same | Feature status file |
| Download links: Microsoft Store, Windows installer, Mac App Store | launch | Release record |
| Support | Store requires a support contact | [owner-decisions.md](../implementation/owner-decisions.md), manual work: support setup |
| Privacy policy | Store requires a policy URL | `docs/store/privacy-policy.md`, ADR-0021 |
| Terms or EULA, third-party notices | legal | Counsel review ([owner-decisions.md](../implementation/owner-decisions.md), manual work) |
| Security disclosure | register SEC-009 | |
| Theme downloads (later) | ADR-0014 | JSON files only; no theme API |

## 2. Rules

- No claim for a feature that is not Release-ready in the feature status file. No fake testimonials, customer counts, logos or metrics.
- Copy: specific, plain, no generic AI marketing phrases (the banned list is in [../engineering/35-writing-standard.md](../engineering/35-writing-standard.md)),
  no em dashes in user-facing text, one idea per section.
- Real product screenshots from the release candidate, with an invented demo project.
- Accessibility: keyboard, visible focus, contrast, reduced motion respected. Responsive at 375, 390, 430, 768, 1024, 1440 px.
- Performance: measure LCP, CLS, INP on a throttled mobile profile; animate transform and opacity; lazy-load heavy media.
- Security: HTTPS, security headers, no secrets in the bundle, no third-party trackers without disclosure.
- SEO: unique title and description, canonical URL, sitemap, robots, social preview, structured data only when factual.
  Search Console and Bing Webmaster setup after launch. Skip GEO tooling until the basics are correct.
- Design: do the design diagnosis first and wait for owner approval before changing visuals (as the supplied prompt requires). Use one visual direction; no cloned layouts.
- Email: company-owned domain, SPF, DKIM, DMARC; transactional and marketing separated if both exist.

## 3. Build order

1. Domain, hosting, email (owner manual work: buy the domain and set up company email). 2. Policy and support pages live (build plan area: Legal, privacy, website and support).
3. Product and feature pages with real screenshots after the release candidate.
4. Store link when the listing is approved. 5. Launch content (trailer, founder story, content bank) belongs to the Certification, publication and launch area, not to the Store submission.

## 4. Open decisions

Hosting provider; static site versus framework; analytics (ADR-0021 default: none or privacy-friendly and disclosed);
who writes the copy; brand and typography beyond the UX specification.
