# Next session prompt: MacHTML, every Mac screen as an HTML page

> Written 2026-10-21 at the owner's request, after WindowsHTML was built on Windows. Paste the block below into a new
> Claude Code session on the Mac, opened in the Mac checkout (the folder that holds `apps/macos` and `MacHTML/`).
> Time box set by the owner: 1.5 hours.

```text
Read CLAUDE.md and docs/implementation/handoff.md, then rebuild MacHTML. Time box: 1.5 hours. Use subagents in
parallel to go fast (one file each, with a file-ownership map written before they start), and keep quality: every
page is checked against a screenshot of the real app.

Goal: delete the old MacHTML/ (179 screens, screenshot plus overlay) and make a new MacHTML/ with one standalone
HTML page for every screen and state of the Mac app (SwiftUI shell over the Rust core), in Light and Dark mode,
at least 400 distinct states. It is the source for the website's Mac screen recreations (Chapter 2), so it must
match the shipped app exactly. Capture from the real running app, never draw by hand.

Important difference from Windows: the Windows app is a web view, so WindowsHTML saved the app's own DOM and CSS.
The Mac app is native SwiftUI and has no DOM. The most exact HTML possible is built from two things captured at
the same moment: (1) the window screenshot at 2x, and (2) the full macOS accessibility tree (role, subrole,
identifier, label, value, enabled, selected, focused, frame) of the app window and of every open sheet, popover,
menu and panel. Each page then has:
  - the screenshot as pixel-exact background, cut into the regions of the main containers (not one flat image),
  - real HTML for every accessibility element at its exact frame: headings and static text as real text
    (transparent over the picture, selectable), buttons as <button>, text fields and text views as <input> or
    <textarea> with their value, checkboxes, switches, radios, pop-up buttons as <select> with the chosen value,
    tabs, lists, tables and outline rows as their HTML equivalents, menus as <menu>, with the accessibility label
    as aria-label,
  - a "HTML only" switch made in CSS (no script) that hides the picture to show the HTML layer alone,
  - no script, nothing loaded from the network, window chrome, shadow and corner radius as the app shows them.
Say this difference plainly in MacHTML/README.md.

How:
1. Start from what exists: apps/macos/Sources/OpenFrameE2EDriver/Scenarios/CaptureTour.swift (the old tour),
   apps/macos/E2E/capture.sh and scripts/mac-html/build-mac-html.mjs. Extend them; do not start over. Delete
   the old MacHTML/ first (git rm; history keeps it) and fix every document that names it.
2. Seed the invented demo project Night Bus the same way the Windows tour does: first-run screens through the UI
   (name dialog, empty project list, New Project dialog with its error), an empty project "Empty Short" for empty
   states, then Night Bus with the bundled example screenplay imported through Import Screenplay, and the rest
   through the core's own operations: Beat Board acts, sequences, cards, beats and a parked card; four characters
   with relationships; Story page text; Idea Vault notes, quote, URL, two images, a folder, a collection, a pinned
   and a deleted item; a second draft with two changed lines; comments (one resolved, one with a reply); a private
   note; notes and tasks; the production source with a breakdown; catalog items; four locations in four statuses;
   cast linked to characters; crew; shots; a storyboard; a moodboard; a schedule with three dated days and a meal
   break; two call sheets; a budget; project files. Every name invented.
3. Split the tour into section files, one per area, each owned by one subagent writing in parallel: shell and
   project home; Application Home and app settings; Idea Vault; Story (Story page, Beat Board, Outline,
   Characters, Timeline); Screenplay (editor, scenes, find, view modes, panels, drafts, compare, lock and
   revisions, title page, page setup, export and print preview); Breakdown; Production plan (Overview, Catalog,
   Locations, Cast and Crew, Moodboards, Storyboards, Shot Lists); Production schedule (Schedule, Day Out of Days,
   Daily View, Sides, Reports, Budget) and Call Sheets; Files, Notes and Tasks, Activity, Recently Deleted,
   Templates, project and app settings tabs; AI Assistant (a few real answers with See sources open), packages
   and licence screens. Cover every dialog, sheet, popover, menu, context menu, drawer or inspector, empty state,
   filled state, error and validation state, toast, and collapsed sidebar. A failed state is recorded with a
   screenshot and the tour goes on. Mark what cannot be reached and why (system Open and Save panels, real drag
   and drop, microphone, print panel, forced crashes, disk failures). Each section must leave the app as it
   found it.
4. Run the parts in parallel if the driver can target one app instance by process id with its own data root
   (check first; OPENFRAME_DATA_ROOT or the Mac equivalent); otherwise the subagents write their sections in
   parallel and one runner runs them. Capture each state in Light and Dark (set through the app's own Appearance
   setting or the core operation it uses), at one fixed window size, recorded in the README.
5. Generate MacHTML/light/NNN-name.html and MacHTML/dark/NNN-name.html (same NNN-name in both), and
   MacHTML/index.html with a Light and Dark switch in CSS and links between screens where a control leads to
   another captured screen.
6. Verify: render every page in WebKit at the same size and scale, compare it with its screenshot, and fail a page
   over 0.5 % differing pixels (a channel difference over 24 of 255). Also check the HTML layer: every
   accessibility element with a label or value appears in the page as text or a control. Write the table, how the
   pages were made, how to regenerate, the commit, and what was not captured into MacHTML/README.md.

Rules: no em dashes and no AI-sounding writing; do not change app behaviour to make capture easier (test-only hooks
only behind the existing debug switches); never build a release .dmg or installer; commit locally on a branch and
merge to main locally after the check passes; never push unless the owner says so in that session. At the end:
handoff, progress log entry, and a short status saying how many states, how many pages passed, and what was not
captured.
```
