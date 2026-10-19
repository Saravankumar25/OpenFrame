# Chapter 2: the product website

> Rewritten 2026-10-15. Chapter 2 starts when Chapter 1 has closed (both the Windows and the Mac track). This page records
> what is decided and what the plan must cover. The implementation plan with phases is written when Chapter 1 closes; no
> website work is started before then. The earlier plan is in [the archive](../archive/2026-10-15-chapter-2-plan-before-restructure.md).

## 1. What the owner has said

- A website for OpenFrame Studio, hosted publicly on a **purchased domain**, with **company email** on that domain.
- The owner wrote that the website is built **manually**. What this means (hand-written pages, no site builder, or no AI
  generation of the pages) is confirmed with the owner when the plan is written. Until then no tool or method is assumed.
- The screens shown on the site are faithful recreations of the real screens of both the Windows and the Mac application.
- Hosting was proposed as Cloudflare Pages (static, near-zero cost). It stays the proposal unless the owner changes it.

## 2. Goal

A small, fast, cheap-to-run static website that tells the truth about the product: what it does, how to start, how to get
help, and what the legal terms are.

## 3. Pages to plan

| Page | Contents | Source of truth |
|---|---|---|
| Home | One clear sentence, three or four real screens | [vision-and-scope.md](../product/vision-and-scope.md) |
| Features | One section per Verified feature group | [feature-status.md](../implementation/feature-status.md), rows proved on the release build |
| Tutorials | Step-by-step guides that follow real workflows | The shared end-to-end scenario list ([ADR-0027](../adr/ADR-0027-rust-core-two-ui-shells.md)) |
| Onboarding | Install, first launch, the AI download, where files live, backup, the SmartScreen and Gatekeeper steps while builds are unsigned | The internal test plan and the data note screens |
| Support and contact | One address, expected reply time, what to send (the exported diagnostics file and the AI report) | The support address (owner task) |
| Privacy Policy | What leaves the computer (nothing automatically; the one AI download), what the AI does, logs | [../store/privacy-policy.md](../store/privacy-policy.md) after counsel review |
| Terms and Conditions | The end-user licence terms, including the use restrictions of each model that ships | Written with counsel |
| Updates | Release notes per version | The release record |

## 4. Rules that already hold

- **Faithful screens.** Recreations come from the running application with the invented demo project, one fixed window size
  per platform, and are checked against a screenshot of the same state. The 181 screens of `HTML_Mockups/` are design history
  and some show removed features (mock-ups 164 to 168 and 155); they are not used.
- **The claims rule.** Nothing is advertised unless it is Verified on the exact build people download. GPU wording follows
  [ADR-0029](../adr/ADR-0029-gpu-acceleration.md) decision 8; language wording follows [ADR-0028](../adr/ADR-0028-ai-quality-program-and-harness.md) decision 9.
- **No analytics unless disclosed**, per [ADR-0021](../adr/ADR-0021-diagnostics-telemetry-analytics-policy.md).
- **Quality bars:** keyboard use, visible focus, contrast, reduced motion, widths from 375 to 1440 pixels, fast loading,
  security headers, a sitemap, and a link check with no broken link ([ADR-0030](../adr/ADR-0030-verification-ladder-and-release-flow.md) decision 6).
- The internal builds are not offered on the public site.

## 5. Owner tasks the plan will schedule

Buy the domain; company email with SPF, DKIM and DMARC; HTTPS hosting; the support address; counsel review of the policy and
terms; trademark and name clearance. Lead times are in [owner-decisions.md](../implementation/owner-decisions.md) section 3.

## 6. Reference material

`docs/website/prompts/` holds two supplied prompts. They are method notes only; the tool names in them were never verified,
so nothing is installed from them and no source code is sent to a third party.
