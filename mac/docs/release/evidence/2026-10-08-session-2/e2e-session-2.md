# Session 2 end-to-end run: evidence of 2026-10-08

Branch `session-2`, starting at `900f245`. Windows 11 machine of the owner, debug build from the shared Cargo
target `C:\OpenFrame\.build-cache\target`, real bundled Offline AI (Gemma 3 1B chat model and BGE embedding model)
with deterministic sampling. No release or installer build was run.

## 1. Result

The first full run of `npm run test:e2e` failed 23 of 30 spec files (01, 02, 15, 16, 21, 31, 40, 41, 42, 43, 50,
51, 61, 62, 70, 72, 73, 74, 75, 76, 90, 91, 95).

Final full run, `npm run test:e2e` once, at `0e6d035`:

| Measure | Value |
|---|---|
| Spec files | 30 passed, 0 failed |
| Suites | 30 |
| Tests | 155 passed, 0 failed, 0 skipped, 0 cancelled |
| Named steps in the specs (`await step(...)` calls) | 166, all inside passing tests |
| Visual baseline | 15 of 15 screens, 0 pixels differ |
| Themes | 16 themes x 2 modes x 15 screens readable |
| Duration | 1221 s for the whole command (build of the debug app included); 1144 s inside the specs |

Every spec also passed on its own (`node run.mjs --no-build --only=<spec>`) before the full run, and the shared-root
group (01, 02, 90, 91, 95) passed together.

No test was skipped, quarantined or weakened, and no timeout was lengthened.

## 2. Failures, root causes and fixes

Product defects first, then specs or harness that no longer matched the product.

### Product defects

| Spec | Root cause | Fix commit | Test that would have caught it |
|---|---|---|---|
| 15 | Under the store lock the core refused `team_gate.status`, so a locked store user saw "could not start the password check" instead of the lock screen | `d9391c0` | `tests/entitlement.rs` (Microsoft Store and Mac locks) |
| 15 | Under the store lock `project.current` is refused; the splash showed whenever the query had no data, unmounted Home, Home asked again, and the window looped on "Opening your local projects..." (about 300 refused calls a second) | `48bc8ef` | `App.test.tsx` |
| 21 | Idea Vault previews, character pictures and call sheet thumbnails built URLs for Tauri's asset protocol, which is switched off; no image, voice note or video could be shown. Recorded voice notes (WebM) had no length, so the player could not seek | `641174e` | `assetSrc.test.ts`, `media.test.ts` |
| 42 | Enter at the end of a new script's scene heading inserted an action in front of the empty one, leaving a stray empty action | `00d0dfd` | `flow.test.ts` |
| 42 | A toast's action was a link to "#": announced as a link, Space did nothing | `86eba05` | `toast.test.tsx` |
| 42, 43 | The screenplay layout was saved 500 ms after the last change and the waiting save was dropped on leaving, so a switched screenplay or chosen navigator columns were lost after a close | `5f8df79` | `layout.test.tsx` |
| 42 | The shortcut recorder took focus and claimed Escape only in effects after paint; an Escape in that moment closed the whole Home dialog (about one run in two) | `2509587` | `Shortcuts.test.tsx` (renders without act) |
| 61 (screenshots) | Stripboard strips used the theme text colour on fixed light papers, unreadable in dark themes | `1a16c7c` | `stripColours.test.ts` |
| 70 (screenshot) | The Import Exchange Package footer crushed its note to one word | `a500d6d` | `dialogFooter.test.ts` |
| 72, 74, 75 | With the real model, "Create a task to ..." on a new project came back as an overview or a search, never a proposal; "How many scenes" failed on a draft name the model invented; a search with few hits did not name them | `df1ba55` | `commands.rs` unit tests, `ai_agent.rs` |
| 72 | "Offline AI settings..." opened the page under the assistant panel, hiding the Technical details card | `772b666` | `AiPanel.test.tsx` |
| 73 | After reading project facts the model called the product guide, whose general text replaced the answer | `14195ce` | `ai_agent.rs` |
| 76 | A clean close dropped the open project before it wrote the checkpoint and removed the crash marker (detaching the AI index can wait up to 5 s); quitting in that window, or a shutdown that found no project, left the marker and the next open offered recovery | `529f7d9` | `core_pipeline.rs` (two tests) |
| 91 | Two counts (Story Outline act row, active folder in Files) were below 4.5:1 contrast in most themes | `6cdf41d` | spec 91 itself |

### Specs and harness that no longer matched the product

| Spec | Cause | Fix commit |
|---|---|---|
| 01, 40, 41 | Typing an unknown character name now opens the suggestion list with "Create New" highlighted; Enter accepted it and the dialogue landed in the cue. The specs close the list with Esc. 01 also expects "Call Sheet: Day 1" | `6eca5bb` |
| 02 | Runs on the shared data root that 01 builds; it failed because 01 failed and passed once 01 did | (01's fixes) |
| 16, 21, 31, 50, 61 | Text drawn in capitals by CSS, the Parking Lot open by default, the built draft opening in the editor, the delete confirmation for built cards, the Quick reuse section, a meal break without a duration | `d9178dc` |
| 21 | The fake microphone flags were overwritten by msedgedriver, so the voice note waited for a real microphone; between spec groups the runner did not wait until the driver ports could be bound again | `0502145` |
| 42, 51 | Ctrl+F pressed before the editor took the new caret; catalog rows, location cards and the "Add to Crew" button | `80c4b48` |
| 62 | Scenes figure, breakdown before the call sheet, the Finalized line, the storyboard panel kind, Notes & Tasks through the project menu | `969e2cf` |
| 70 | The native file dialog helper reported success while nothing reached the dialog (stdin-fed PowerShell, parts exposed only as panes, WM_SETTEXT not used by the dialog) | `df0f241` |
| 72, 73 | The embedding engine may start when a project opens (AI specification section 4); the scope line was read after a fixed sleep | `2c5268e` |
| 76 | The close helper waited for the words "Your projects", which the Offline AI repair message also contains; the ask helper could take a restored answer for the new one; the last step's request is now answered without the model | `9416b14` |
| 90 | Captured whatever Windows mode the computer used; baselines from 2026-10-06 (section 3) | `0e6d035` |
| 95 | Recently Deleted in the Idea Vault replaces the page; Settings gained Advanced and Appearance; Ctrl+J is checked by the panel itself | `dbd55e4` |

## 3. Visual baselines regenerated

Folder `docs/release/evidence/M0/M0.20-visual-baseline/`, captured with `E2E_VISUAL_UPDATE` and then compared in the
shared-root group and the full run (0 pixels differ). The checksum file is refreshed. The spec now pins the default
theme in Light mode. Every screen shows the new logo and header, the contrast-safe accent ink on primary buttons
(`90be753`, area 12) and slightly different navigation icon rendering; the list below gives what else changed.

| Screen | Share of pixels changed | Reason |
|---|---|---|
| 01 Application Home | 1.68 % | Appearance and Restore Backup or Package entries in the header and toolbar, new project cover |
| 02 Project Home | 0.40 % | Logo, header and icon rendering only (the time line is an ignore region) |
| 03 Idea Vault | 2.42 % | The old capture showed a recovery toast after an unclean close, which no longer happens |
| 04 Story Board | 2.50 % | Same recovery toast gone; "No scene numbers; they come from the screenplay" (writing standard) |
| 05 Story Outline | 0.41 % | Row prefixes "Sequence:", "Scene:"; readable act count (`6cdf41d`) |
| 06 Screenplay editor | 0.35 % | Screenplay picker beside the draft picker (several screenplays per project), page count, navigator columns button |
| 07 Breakdown | 0.82 % | Breakdown category colours and icons, scene completion line, punctuation |
| 08 Production overview | 0.56 % | Day Out of Days tab; "Draft 1 from Story Board" without the dash, screenplay name in the status line |
| 09 Production catalog | 0.38 % | Day Out of Days tab; category colours and icons; subtitle punctuation |
| 10 Production schedule | 3.61 % | Schedule filters and the Day Out of Days tab (area 14), "Shoot Day 1 · Mon 14 Jun 2027" |
| 11 Call sheets | 0.26 % | Filter "Final" and "Show replaced revisions" wording |
| 12 Files | 0.14 % | Readable count of the active folder (`6cdf41d`) |
| 13 Notes & Tasks | 0.12 % | Accent ink on the New Task button |
| 14 Recently Deleted | 0.55 % | 30-day retention wording |
| 15 Project settings | 1.43 % | Writing, Keyboard shortcuts, Advanced and Appearance tabs; internal build licence card and About card |

No difference was unexplained.

## 4. Offline AI: CPU and typing measurements

The hypothesis that the bundled AI starves the CPU and causes the failures was tested and is not supported.

- Spec 01 (AI idle): engine processes averaged 0.4 % CPU (max 10 %, about 60 MB); the app 0.3 %.
- Spec 72 (real answers): engines averaged 28.9 % of 18 logical processors (max 71.8 %, up to 1338 MB); the app
  0.1 %. The chat engine uses one thread per physical core (at most 16), the embedding engine half that (at most 4),
  at normal priority. The embedding engine starts at project open, the chat engine on the first question.
- Typing latency in the screenplay editor, keydown to the next frame: idle median 5.9 ms, p95 16.6 ms, max
  18.6 ms; while both engines answer a question median 6.3 ms, p95 19.0 ms, max 20.2 ms. The longest main-thread gap
  was 12.7 ms idle and 17.3 ms while answering. All typed text was kept.
- No product change to priority or threads was made, since the measurements show no starvation.

Timings from the final full run (spec 72, with model load on first use): AI Ready 3.7 s after launch; exact answer
19.7 s; meaning question 23.5 s; navigation command 18.3 s; Stop Offline AI and restart on the next question
26.2 s. Spec 73 questions took 18.3 to 23.5 s; spec 76's real-model answer after the bundle was whole again 32.5 s.

## 5. Other checks after the fixes

`npm run typecheck` passed; `npm --workspace apps/desktop run lint` passed with no warnings; `npm run test:web` 60
files, 552 tests passed; `npm run check:governance` 67 passed;
`node scripts/cargo-queued.mjs clippy --workspace --all-targets -- -D warnings` passed;
`node scripts/cargo-queued.mjs test --workspace` 94 test binaries, 2012 passed, 0 failed, 11 ignored.

## 6. For the owner

- The AI Assistant panel lies over the right 360 px of the page and covers controls there (the specs close it first,
  as a person would). Docking it beside the page instead is a design decision.
- Typing an unknown character name and pressing Enter accepts "Create New"; a second Enter is then needed to reach
  Dialogue (as planned under "Smart suggestions while typing").
- Dashes remain in user-facing text in files this session did not touch, for example the scene notes and Proposed
  Changes titles in `story/dialogs.tsx`, `interchange/model.ts` and the empty-conversation hint in
  `ai/Conversation.tsx`. The writing ratchet counts them.

## 7. Owner decisions of 2026-10-08: docked AI panel, one Enter for a new character, no dashes

Commits on `session-2`: `7e74f07` (dock the AI Assistant), `8d6b53e` (one Enter after a new character name),
`922a7c8` (nothing cut off beside the docked panel; spec fixes), `d5cc2bb` (dash sweep), `4fdd7d3` (baseline screen 08).

### Final full run

`npm run test:e2e` once at `4fdd7d3`, debug build included:

| Measure | Value |
|---|---|
| Spec files | 30 passed, 0 failed |
| Tests | 157 passed, 0 failed, 0 skipped, 0 cancelled (155 before, plus "one Enter after a new character name" in 42 and "the docked panel remembers whether it is open" in 72) |
| Visual baseline | 15 of 15 screens, 0 pixels differ |
| Themes | 16 themes x 2 modes x 15 screens readable |
| Duration | 1212 s for the whole command; 1134 s inside the specs |

### What the earlier runs found and how it was fixed

- The journey now tours every workspace, production tab and project page at the 1200 x 720 minimum window with
  the AI Assistant docked, and checks that the panel covers nothing and no control ends past the page edge outside
  a scroll area. It found the Idea Vault toolbar (List, Folder) and the Project settings tab bar (Appearance) cut
  off. Toolbars now wrap and tab bars scroll sideways; at wider windows nothing changes.
- Each WebDriver session starts with fresh window storage, so a relaunch cannot carry the panel state. Spec 72
  checks open and closed across a reload of the window instead.
- Spec 42 read the new empty Dialogue line without its placeholder text, and reached a Parenthetical with an extra
  Tab; both were spec mistakes. The journey's Story step counted two cards before the second showed its text; it
  now waits for the text.

### Visual baseline changed on purpose

| Screen | Reason |
|---|---|
| 08 Production Overview | The "Needs attention" rows now read "Newer revision available: ... is newer. Production stays on ..." and "Needs casting: ARJUN" (colon instead of a dash). Regenerated alone with `E2E_VISUAL_UPDATE=08-production-overview`. |

No other screen changed. The docked panel is not part of any baseline screen.

### Dash sweep counts (`node scripts/check-writing-standard.mjs`)

| Group | Before | After |
|---|---|---|
| app | 96 | 16 |
| crates | 310 | 202 |
| living docs | 40 | 39 |
| scripts and CI | 26 | 26 |
| Total | 472 | 283 |

The hits left in `app` and `crates` are test fixtures that stand for text people write (scene headings, draft
names, captions), parsers that accept every dash a writer may type, developer comments, migration comments and test
panic messages. No user-visible string OpenFrame builds keeps an em dash or a spaced en dash.

### Other checks at the end

`npm run typecheck` passed; `npm --workspace apps/desktop run lint` passed with no warnings; `npm run test:web`
61 files, 568 tests passed; `npm run check:governance` 67 passed;
`node scripts/cargo-queued.mjs clippy --workspace --all-targets -- -D warnings` passed;
`node scripts/cargo-queued.mjs test --workspace` 94 test binaries, 2013 passed, 0 failed, 11 ignored.

## 8. Owner answers: one Enter on a known name, Tab never accepts

Commit `d75140d` on top of the lead's `df4853c` (rustfmt only). Enter on a known character name chosen from the
suggestion list now fills it and moves on to Dialogue in one press, like a new name; the filled name is saved before
the Dialogue line, so Ctrl+Z takes back the Dialogue step first and the name after. Tab keeps its element meaning and
never accepts a suggestion.

Focused runs before the gates: `suggest.test.ts` and `flow.test.ts` (editor folder 217 tests at the time) passed;
`node run.mjs --only=42-writing` passed (9 tests, including the new "one Enter on a known name" test);
`node run.mjs --no-build --only=40-screenplay,41-screenplay,43-several` passed.

Gates once: `npm run typecheck` passed; desktop lint passed with no warnings; `npm run test:web` 61 files, 573 tests
passed; `npm run check:governance` 67 passed; `cargo fmt --all -- --check` passed; queued clippy with
`-D warnings` passed. No Rust code changed in this step, so the Rust tests were not run again (last run: 2013
passed, 0 failed, 11 ignored).

Final full run, `npm run test:e2e` once at `d75140d`:

| Measure | Value |
|---|---|
| Spec files | 30 passed, 0 failed |
| Tests | 158 passed, 0 failed, 0 skipped, 0 cancelled |
| Visual baseline | 15 of 15 screens, 0 pixels differ (no baseline changed in this step) |
| Duration | 1212 s for the whole command; 1147 s inside the specs |
