# Windows final polish: work plan

Written 2026-10-19 at the owner's request ("OpenFrame Windows: final refinement, UI unification and release build").
Sources: `changes/` (the owner's change list `all_changes.docx`, change specs 01 to 05 with their HTML references, two
reference pictures), `OpenFrameAI.docx` (AI repair brief with 12 screenshots), and the Mac app in `OpenFrameMac/` (its
SwiftUI source and the 179 captured screens in `OpenFrameMac/release-out/MacHTML/`). These three folders are reference
material and are not committed.

## Findings before work started

- Baseline on `windows/final-polish` (from 26d9756): typecheck passes, 601 of 601 web tests pass.
- The Mac app is a reference for AI behaviour only (owner decision above).
- The Mac and Windows apps share one Rust core. The Mac copy differs in the AI area only by the Metal engine placement
  (ADR-0033: the chat model always runs on the Apple GPU, no timing probe) and one tool fix (a search for the word
  "locations" answers with the location statistics). Same model (Gemma 3 1B), same prompts, same themes file. The Mac
  feels better mostly because Metal makes every step several times faster; the wrong answers in the AI brief come from
  shared core routing and from the Windows chat view, so they are fixed here.
- OpenFrame Default has no dark palette of its own, so its dark mode is generated from the same seeds as Studio Amber.

## Work items

### A. Shell (owner items 1, 4, 5, 6, 10, 13, 14, 16, 17, 18; AI brief section 6)

**Owner decision 2026-10-19 (during this work): the Windows UI keeps its own design and is not unified with the Mac
app.** A Mac-style shell was started and fully reverted the same evening. Only the owner's listed changes are made, in
the existing Windows design:

1. Top bar: the logo opens all projects; Back and Forward page buttons replace the Undo and Redo buttons (Ctrl+Z and
   Ctrl+Y stay; Undo and Redo are also in the project menu); a gear opens Settings (global tabs on Home, project and
   global tabs inside a project) in place of the shortcuts button. Alt+Left and Alt+Right also move between pages.
2. Ctrl+Plus, Ctrl+Minus and Ctrl+0 zoom the window; the screenplay and Breakdown pages zoom with Ctrl+wheel and pinch.
3. Focus mode hides everything except the screenplay page.
4. A page reload keeps the current page (the route is remembered for the session).
5. OpenFrame Default gets its own dark palette.
6. Dark mode contrast pass (screenplay suggestions and similar spots).
7. The page frame can no longer be scrolled by script (the "stuck top, white bottom" bug).

### B. Feature fixes and removals (owner items 2, 3, 5, 7, 8, 9, 11, 12, 15, 19, 20, 21)

Custom relationship text, non-overlapping relationship graph, screenplay scroll alignment, Tab cycles every element,
the Scene-only toggle in Compare, removal of the Breakdown, Shot List, Schedule and Call Sheet Review packages, removal
of the Episodic and Series project types for new projects (existing projects still open), removal of the screenplay
Review section, Daily View export, +Beat/+Scene overflow, long draft names, the Breakdown sidebar overflow, and a grid
view in Shot Lists.

### C. AI (OpenFrameAI.docx)

Unqualified and non-English scene and character counts; "describe a character" answered from screenplay evidence;
product help only for product questions; search for a category word answers with its statistics; clean chat replies with
details behind a disclosure; complete long lists; no claim that a change was applied unless it was; friendly replies to
greetings. Measured with the evaluation harness (ADR-0028) before and after.

### D. PDF exports (change specs 01 to 05 and the owner's note)

Screenplay and Sides typography (spec 01), call sheet (02), shooting schedule (03), Day Out of Days (04), breakdown and
catalog (05), then the same design system for moodboard, storyboard, shot list, the six reports and the budget. Sample
PDFs with sample data go to `release-out/pdf-samples/` for the owner's review.

### E. Verification and release

Rust and web tests, governance checks, end-to-end specs, a real-app walkthrough, the internal installer
(`npm run installer:internal`, built at the owner's request of 2026-10-19), then install and check the artifact.

## Overnight checklist (owner instruction 2026-10-20: finish everything, build and verify the .exe)

Sources added 2026-10-20 (reference only, not committed): `Downloads/OPENFRAME_WINDOWS_UX_AUDIT_AND_REDESIGN_CLAUDE_CODE_MASTER_PROMPT.md`,
`Downloads/OPENFRAME_WINDOWS_AI_CONTEXT_ENGINEERING_CLAUDE_CODE_MASTER_PROMPT.md`, `Downloads/openframe-chnages.docx`.

### Change list and change specs
- all_changes.docx items 1 to 11, 13 to 21: done. Item 12 Daily View export: open.
- Change specs 01 to 05 and read_this.txt (all PDF exports, samples in release-out/pdf-samples): in progress (subagent).
- openframe-chnages.docx 1, stripboard colours (EXT Night green, EXT Day yellow, INT Day white, INT Night light blue):
  done in the app; the schedule PDF follows (subagent brief).
- openframe-chnages.docx 2, Story Board act columns that scroll on their own with whole card text and a Parking Lot
  column: done in story.css; check in the running app.

### AI (OpenFrameAI.docx and the context engineering brief)
- Done: plain and Indian-language counts, character description from evidence, category search, no false applied
  claims, clean chat UI, greetings, product help only for app questions (its tests pending).
- Open: scene-level facts (summary from the full scene, who speaks, speaker count, where and when, first and last line,
  mention check), dialogue facts per character, drafts and omitted counts, follow-ups ("What about Scene 4?", "And
  Ravi?", "Just give me the number"), gold cases, real-model evaluation after the changes, owner walkthrough.

### UX audit brief
- Open: measured geometry pass (AI panel, nav, panels, window sizes) with an E2E geometry spec; scroll ownership;
  clipped text; additive Black on White and White on Black themes; docs/ux deliverables.

### Release
- Open: fmt, clippy, cargo test, web tests, governance, writing; E2E release suite; real-app walkthrough; installer
  build, install, smoke; cleanup of worktrees and stale documents; handoff, progress log, local merge to main.
