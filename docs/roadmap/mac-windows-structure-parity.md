# Mac and Windows: one structure, two native presentations

Owner request of 2026-10-20: the Mac app takes the structure, navigation, workflows and terminology of the
latest Windows app, keeps its own Apple design, and takes the owner's change lists, the AI brief and the UX
brief. This page is the requirement inventory and its status. Code is the truth; where this page and the code
disagree, the code wins and this page is fixed.

- **Windows baseline:** GitHub `Saravankumar25/OpenFrame` commit `324cd963` (folder `windows/`), imported as
  commit `e9f2619` on top of the W9 merge `973a8ee` and merged into the Mac branch (`af5e41f`). The shared Rust
  core is therefore one core again (ADR-0034).
- **Mac branch:** `mac/windows-parity` (from `mac/m1-toolchain`).
- **Inputs read:** `changes.zip` (all_changes.docx items 1 to 22, five PDF change specs with references,
  read_this.txt), `new changes.docx` (items 1 to 5), `openframe-chnages.docx` (items 1 and 2), `OpenFrameAI.docx`
  (sections 1 to 12, exhibits E01 to E12), the Windows AI context-engineering brief and the Windows UX brief.

Status words: **Done** (built on the Mac and tested), **Core** (done in the shared core, so the Mac has it with
the merge), **Same** (the Mac already matched), **Exception** (a deliberate platform difference, reason given),
**Open** (not done yet, next step given).

## 1. Structure

| Item | Windows | Mac now | Status |
|---|---|---|---|
| Sidebar | Flat: Home, Idea Vault, Story, Screenplay, Breakdown, Production, Call Sheets, Files | Same list and order, SF Symbols, "Project stored locally on this Mac" and the save state at the foot | Done |
| Production | One workspace, 13 tabs: Overview, Catalog, Locations, Cast & Crew, Moodboards, Storyboards, Shot Lists, Schedule, Day Out of Days, Daily View, Sides, Reports, Budget | Same tabs in the same order; the strip wraps in a narrow window | Done |
| Production Overview | Source, breakdown progress, unresolved locations and cast, catalog, needs attention | Ported (production.overview) | Done |
| Breakdown | Breakdown only; categories in Settings > Advanced | Breakdown only; the category looks are in Project Settings > Advanced (the same editor as before) | Done |
| Call Sheets | Call sheet list and editor | Same; Daily View, Sides and Reports moved to Production | Done |
| Notes and Tasks, Activity, Recently Deleted | Project menu | Project menu and the Go menu | Done |
| Back and Forward | Toolbar arrows, Alt+Left and Alt+Right, 50 pages | Toolbar Back and Forward, Go > Back and Forward, Command+[ and Command+], 50 pages | Done (Mac shortcuts) |
| Logo to all projects (item 17) | Logo button | Toolbar button "All Projects" with its name | Exception: the Mac toolbar has no logo; the same action is labelled |
| Settings gear (item 6) | Home: Appearance, Keyboard shortcuts, About; project: Project settings, Writing, Keyboard shortcuts, Templates, Offline AI, Advanced, Appearance | Project Settings sheet with the same seven pages in the same order; at Home the Mac Settings window (Command+Comma) | Done; the Home window keeps General and Diagnostics (Exception: macOS Settings window) |
| Writing settings | Spell check on or off, main and second language, project dictionary | New page; the editor follows the on/off choice and language; words checked by the macOS spelling service | Done |
| Focus mode (item 13) | Only the page | Hides the toolbar, the sidebar and the AI panel | Done |
| Reload keeps the page (item 1) | Route kept in session storage | Window keeps the project's page (scene storage) | Same |
| Search field | Top bar "Search this project" | Toolbar button "Search" (opens the search palette, also Command+K and the Edit menu) | Done (Mac palette) |
| Create menu "+" | 13 quick creates | Toolbar menu "New" with the same quick creates | Done |
| Status bar | Save, breadcrumb, offline line | Save state in the sidebar foot; the window subtitle names the page | Exception |
| AI panel width | Drag the edge, 300 to 640 points or 45 percent, remembered, double-click resets | Same rule, drag handle, arrow keys and VoiceOver adjust, remembered on this Mac | Done |

## 2. Owner change list (all_changes.docx)

| # | Request | Status on the Mac |
|---|---|---|
| 1 | Refresh keeps the page | Same |
| 2 | Custom relationship types | Same (free text with 11 suggestions) |
| 3 | Relationship map overlaps | Done (fewest crossings, bent parallel lines, labels moved clear, on pills) |
| 4 | Global Back and Forward | Done |
| 5 | Scene navigation alignment glitch | Core and Windows; Mac layout fixes of 2026-10-19 (window fit) stand |
| 6 | Settings gear with local and global tabs | Done |
| 7 | Tab goes round every element | Done |
| 8 | Scene-only toggle not working | Done: Compare views carry the Windows names and order, Scene-only and Text-only, plus the Mac's Text inline; a scene chosen in Scene-only opens in Text-only |
| 9 | Remove Breakdown, Schedule, Shot List and Call Sheet Review packages; remove Episodic and Series | Done (old files still open; old projects keep their type) |
| 10 | Right sidebar toggle and pinch zoom in the screenplay | Inspector toggle Same; zoom Done (50 to 200 percent: toolbar, Command+Plus, Minus and 0, trackpad pinch; only the view scales, text, caret, undo and exports unchanged) |
| 11 | Remove the screenplay Review section | Done |
| 12 | Export for the Daily View | Done, as its own document (see "new changes" item 1) |
| 13 | Focus mode shows only the screenplay | Done |
| 14 | Screenplay zoom with the mouse | Done (pinch; see item 10) |
| 15 | +Beat, +Scene overflow | Done (the rows wrap) |
| 16 | OpenFrame Default dark | Core (themes.json); the Mac engine matches the reference for all 17 themes |
| 17 | Logo or name to all projects | Done (labelled button) |
| 18 | Long draft names | Done (the toolbar draft menu shortens a long name in the middle; the full name is in the tooltip and the menu) |
| 19 | Breakdown sidebar breaking the page | Same (Mac split view); hide scenes and panel buttons Open |
| (screenplay toolbar) | Scenes toggle and Find button | Done (Scenes hides the scene list; Find opens the find bar on the page) |
| 20 | Dark mode contrast | Core themes; Mac uses theme tokens |
| 21 | Shot list grid view | Done (List and Grid; a card shows the reference image or the first storyboard panel, then size, angle, movement, lens, description and characters; reordering stays in List as on Windows) |
| 22 | PDF exports redesign | Core (the Windows PDF redesign is in the merged core; Mac exports use it) |

## 3. New changes and openframe-changes

| Item | Status on the Mac |
|---|---|
| Daily View export named "Shoot Day N - Daily view", with what the page shows | Done in the core (`schedule.export_daily`, PDF, CSV, XLSX) and on both apps (owner decision 2026-10-20: the new document only; the schedule export's day-first variant is removed) |
| Area not scrollable (Story outline) | Mac outline scrolls; Same |
| Compare top glitch | Mac compare is its own sheet; Same |
| Offline AI technical details overlapping | Mac shows it as a list in the AI settings; Same |
| Find navigates the scene list but not the screenplay | To check on the Mac (Open) |
| Stripboard colour codes | Same (INT Day white, EXT Day yellow, INT Night blue, EXT Night green) |
| Story Board columns and Parking Lot like the reference | Done (whole card text, columns scroll on their own) |

## 3a. Owner requests of 2026-10-20, afternoon (both apps)

| Item | Status |
|---|---|
| "Story Board" is called "Beat Board" everywhere (it was confused with the production Storyboards) | Done in the core (messages, exports, AI answers, templates), the Mac app, the Windows app and both end-to-end suites. AI questions that still say "story board" are understood. Internal names (`story.*` operations, tables) are unchanged, so projects and packages are unaffected. Specifications keep the old name as history |
| A free "Story" page, left of the Board tab, to write the story like a notepad, with Export as TXT, PDF and DOCX, linked to nothing | Done: new table `story_page` (migration 0017, additive), queries `story.page`, `story.save_page` (refuses a save from a stale window, undoable) and `story.export_page` (TXT without a brand line, PDF, Word). Mac: Story tab first in the Story workspace, saves a second after typing stops and when left. Windows: the same tab and export dialog (TypeScript not compiled on this Mac) |

## 3b. Owner feedback of 2026-10-20, evening: labels, Home, room on small screens

| Item | Status |
|---|---|
| Every button says what it does | Done: toolbar and page buttons carry their names (Close, Remove, Delete, Actions, More, Back, Forward, Search, New, Assistant, Settings, Scenes, Find, Zoom In, Zoom Out). Kept as symbols on purpose: disclosure arrows, checkboxes and the category icon picker |
| Home page as on Windows | Done: "Your projects" with search, Open Project, Restore Backup or Package, New Project; Pinned and Recent projects; Archived as its own page; empty state "Create your first project." Project Home: status menu, Project settings, the three ways to start for a new project, then Continue, Quick Access, Recent, Production status, Project Files and Tasks |
| Toolbar buttons hidden in the overflow menu on a 1280 by 720 screen | Fixed: the toolbar no longer draws the window title (the Window menu and VoiceOver keep it); the project name is at the top of the sidebar |
| Beat Board tool row past the window edge | Fixed: the actions move to a second row in a narrow window |
| Catalog and Locations filter rows past the page with the assistant open | Fixed: the row of filter buttons becomes a pop-up menu when the page is narrow |
| Screenplay scene list over the sidebar at 900 points with the assistant open | Fixed: the scene list steps aside when the page has no room for it (the Scenes button says why) |
| Smallest window taller than a 720-point screen | Fixed: the minimum is checked again when the toolbar changes height |

## 4. AI (OpenFrameAI.docx and the Windows AI brief)

| Item | Status |
|---|---|
| Exact counts from project data ("How many scenes are there?") | Core (Windows AI repair merged) |
| Project questions never answered with product help | Core |
| Plain answers, no Exact or Inferred badges, no step lists | Done on the Mac |
| Sources behind a closed "See sources" | Done on both apps (owner decision 2026-10-20: "See sources", sources only; steps and how sure stay out of the conversation; Windows keeps the confidence as `data-confidence` for its tests) |
| Long lists: preview and Show all, never "and N more" | Done (8 and Show all) |
| Never claim a change was applied without Apply | Core (Change Sets) and Same on the Mac (Proposed Changes card) |
| Follow-up references | Core |
| Local only | Same |
| Starting questions | Done on the Mac: an empty conversation offers "Try asking" questions for the page you are on |
| Copy an answer | Done on the Mac: Copy under each answer (the answer text only) |
| The Story page stays private to the writer | Done: the assistant can neither read nor change it (`story.page` and `story.save_page` hidden from the AI, reason `STANDALONE_PAGE`) |
| Requests the model planned wrongly | Seven deterministic routes (task due date, set dressing, characters for a scene, a shot for scene N, product terms, parking lot cards). English evaluation on the Mac: tool and arguments 94.4 to 98.1 percent, end state 87.3 to 89.9, safety 46 of 46 |
| Meaning search measured on the Mac | Fixed in the evaluation tool: a relative engine folder made the search engine fail to start, so every retrieval case measured word matching and waited ten minutes. The tool now resolves the folders, checks the search model before the first case and stops waiting when the index reports an error. The index now records why embedding failed (the snapshot's `error`) instead of only "updating" |

## 5. Verification

- Rust: `cargo test -p openframe-application -p openframe-ffi -p openframe-import-export`, 30 suites pass (976 core
  tests), plus the Daily View export tests.
- Swift: `apps/macos/scripts/test.sh`, 606 tests; failures after the merge were stale expectations (fixed) and one
  Trash test that passes alone.
- Real app (injected E2E driver): see the progress log entry of 2026-10-20 for the scenarios run and their results.
- Not verified by a person: drags, VoiceOver, the look in every theme at every window size.
