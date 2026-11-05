# Website handoff (2026-10-31)

State (deployed 2026-10-31, commit after b351c46): the site follows the approved Website Share Package design
with three independent controls in the header (platform, website theme, app screen theme), plus site search (Ctrl K,
Cmd K or /). Every screen is a live HTML page of the whole app window from `scripts/shots.mjs` (`npm run shots`, output
in the git-ignored `.cache/shots/`), framed in its card and opened full size in the viewer on click. Owner decision
2026-11-01: no crops, no collapsed navigation, no blank sidebars (the capture of 2026-11-05 draws them all); only the Windows screenplay writing screens hide the Scenes list so the script is the centre.
A screen is a card only once per feature page. Feature pages show no "More screens" lists.
Screens were replaced on 2026-11-05 by the captures in `windows/WindowsHTML` and `mac/MacHTML` (bundle `website-screens-2`); Mac windows are now 1600 by about 792 at 1x and are shown whole. Defects found then: `docs/screen-defects-2026-11-05.md`.
The Mac captures never ran the AI assistant (no model in that build), so the Mac AI chat, proposals and Straft AI settings
details are simulated screens (28, `scripts/sim-mac.mjs` with `scripts/sim-mac-screens.mjs`, run after `shots.mjs`): a real Mac
capture whose AI panel or Offline AI pane is redrawn as live HTML with the Windows wording, listed in
`research/screens-mac.json` with `sim: true` (`extract-screens.mjs` drops them, so re-run `sim-mac.mjs` after it).

- 742 pages: home, 15 feature pages with a capability catalog, 390 Windows and 310 Mac written tutorials (with previous
  and next links), platform pages, pricing (launch price ₹3,999 in India, €89 in Europe, US$89 elsewhere, bought separately per platform), download,
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
3. Recapture the Mac assistant screens with the AI engine installed, then replace the simulated ones.
4. Capabilities with no Mac capture (files that moved, import errors, Needs Review, removed scenes, changed after final, deleted day, trial)
   have simulated Mac dialogs and notices (`scripts/sim-mac-free.mjs`), with wording from the Mac source. Capture them for real. No page says "Not on Mac".
