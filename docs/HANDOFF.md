# Website handoff (2026-10-31)

State (deployed 2026-10-31, commit after b351c46): the site follows the approved Website Share Package design
with three independent controls in the header (platform, website theme, app screen theme), plus site search (Ctrl K,
Cmd K or /). Every screen is a live HTML page of the whole app window from `scripts/shots.mjs` (`npm run shots`, output
in the git-ignored `.cache/shots/`), framed in its card and opened full size in the viewer on click. Owner decision
2026-11-01: no crops, no collapsed navigation, no blank sidebars (undrawn Mac sidebars are painted in from the nearest
screen of the same area); only the Windows screenplay writing screens hide the Scenes list so the script is the centre.
A screen is a card only once per feature page.

- 742 pages: home, 15 feature pages with a capability catalog, 390 Windows and 310 Mac written tutorials (with previous
  and next links), platform pages, pricing (₹2,999 in India, US$89 elsewhere, bought separately per platform), download,
  about, FAQ, Themes (custom theme files and six starters), /search/, and the legal hub (Privacy, Terms, Refunds, Cookies
  and browser storage, Notices). Missing legal facts are hidden markers listed in `.cache/agents/legal-report.md`.
- Content source: `src/content/manual/<win|mac>/`. The two PDF manuals built from it are internal only.
- Tests: `npm test` (112 at this point). Docs: `docs/OPERATIONS.md` (deploy, Cloudflare checklist), `docs/SEO.md`,
  `docs/SECURITY.md`, `docs/ROADMAP.md`, `docs/decisions/hero-animation-style.md`.
- Waiting on the owner: contact email, Cloudflare dashboard items, custom domain, legal facts, AI model terms wording,
  euro pricing decision, GitHub repository stays public (owner decision 2026-10-31).
- Store buttons look normal and do nothing until listing links exist.

Next:
1. Owner review of the live site and the tutorial wording.
2. Store listing links when they exist (`OF_STORE_WIN`, `OF_STORE_MAC`).
3. Recapture the Mac assistant screens with the AI engine installed, then add Mac assistant tutorial steps.
