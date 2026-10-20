# Next session prompt: WindowsHTML, every Windows screen as an HTML page

> Written 2026-10-21 at the owner's request. Paste the block below into a new Claude Code session opened in
> `C:\OpenFrame`. The Mac side already has the same thing (`mac/MacHTML/` on GitHub main); this is its Windows twin.

```text
Read CLAUDE.md, docs/implementation/handoff.md and this file, then build WindowsHTML.

Goal: a folder WindowsHTML/ (in the Windows checkout root) with one standalone HTML page for every screen and
state of the Windows app, replicating the real app to the finest detail: layout, spacing, fonts, colours,
icons, borders, shadows, scroll positions, text, and every control in its real state. Hundreds of pages. It is
the source for the website's screen recreations (Chapter 2), so it must match the shipped app exactly.

How (capture from the real app, never redraw by hand):
1. Write a capture tour as an E2E spec, tests/e2e/specs/zz-capture.test.mjs (skipped unless E2E_CAPTURE=1),
   using the existing WebDriver harness (tests/e2e/lib/*.mjs, run.mjs --only=zz-capture --no-build). It seeds a
   project called Night Bus with the bundled example screenplay plus Beat Board acts and cards, characters,
   Idea Vault items, a breakdown, a schedule with days, call sheets, budget lines, notes, tasks, a Story page
   text, files, comments, two drafts, a lock, and visits every screen it can reach.
2. Coverage: every workspace and tab (Home, Idea Vault, Story: Story page, Board, Outline, Characters,
   Timeline; Screenplay incl. Scenes panel, Find and Replace, Notes, Comments, Drafts, Compare, Lock, Export;
   Breakdown; Production: Overview, Catalog, Locations, Cast & Crew, Moodboards, Storyboards, Shot Lists,
   Schedule, Day Out of Days, Daily View, Sides, Reports, Budget; Call Sheets; Files; Notes & Tasks; Activity;
   Recently Deleted; Templates; Project settings and all Global settings pages; Offline AI states; AI
   Assistant panel with a few answers and See sources open), every dialog, drawer, menu, popover, context
   menu, export dialog, empty state, filled state, error and validation state, and the collapsed navigation
   rail. Use docs/implementation/feature-status.md, the routes in tests/e2e/specs/95-reachability.test.mjs,
   the screen list in 98-layout-geometry.test.mjs and the 181 ids in HTML_Mockups/ as the checklist (at least 400 distinct screens and states in each of Light and Dark; the owner asked for 400 or more); list
   anything not reachable and why.
3. Capture each screen twice: the default theme in Light and in Dark (set in Settings, Appearance), at
   1600x1000 window size. For each screen save a full-window screenshot and a serialized DOM: the page's real
   HTML with every computed style inlined (or the app's CSS bundled once into WindowsHTML/assets/), fonts and
   SVG icons embedded, form values and scroll positions kept, no scripts, nothing loaded from the network.
   The web view IS the app, so the page should be the real markup, not an overlay on a picture.
4. A generator, scripts/windows-html/build-windows-html.mjs, writes WindowsHTML/light/NNN-name.html,
   WindowsHTML/dark/NNN-name.html (same NNN-name in both), and WindowsHTML/index.html with a Light/Dark switch
   and links between screens where a control leads to another captured screen.
5. Verify: render every page in the same WebView2 at the same size and compare with its screenshot using the
   visual diff helper of 90-visual-baseline; fail a page over 0.5 % differing pixels; write the table into
   WindowsHTML/README.md with how it was made, how to regenerate, the commit it came from, and what was not
   captured (system Save/Open dialogs, drag states, anything needing real devices).

Rules: no em dashes; do not change app behaviour to make capture easier (test-only hooks only under the
existing debug switches); commit locally on a branch, merge to main locally after the check passes; never
push unless the owner says so in that session.
```
