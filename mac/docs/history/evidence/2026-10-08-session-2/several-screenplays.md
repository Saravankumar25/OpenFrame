# Several screenplays per project: evidence (2026-10-08, session 2)

Masterplan area 11, box **Several screenplays per project**. Owner decision of 2026-10-08 (ADR-0012 amendment): a
project may hold several screenplays; each keeps its own drafts, locked drafts, production numbers and page setup;
Story Board, Breakdown, Schedule, Call Sheets and exports always say which screenplay they use and never pick one
silently; older projects open with their one screenplay and lose nothing.

Branch `area/11-several-screenplays`, built from `session-2` at `c6d3ee8`. No migration: the schema already allowed
several `screenplay` rows per scope (data check in `reconciliation-log.md`, 2026-10-08). Areas 14 and 15 own 0015 and
0016 on their branches; this work did not need 0017.

## What changed

Every place listed in the 2026-10-08 section of `docs/implementation/reconciliation-log.md`:

| Place | Change |
|---|---|
| `screenplay/mod.rs` | Module comment reworded. `screenplay_for_scope` (`LIMIT 1`) removed; `screenplays_in_scope` lists every screenplay of a project or episode. `ScreenplayOverview` has `screenplays` and takes a `screenplayId` (a stale id falls back to the oldest; the picker always names the open one). `scene_nav`, `draft_nav` and search results carry `screenplayId`; the search context names the screenplay. |
| `screenplay/drafts.rs` | Guard against a second screenplay removed. A default title gets a number ("Black Rain (2)"); a typed title already used in the scope is refused with `validation.duplicate`. New command `screenplay.rename` (Edit, one undo step, sets the title page title, reindexes search). |
| `story/build.rs` | `NewScreenplay` guard removed; the preview's default title is unique. The Apply Story Board order preview lists every screenplay. |
| `interchange/import.rs` | `can_create_screenplay` is true except in a series without episodes; `episodesWithoutScreenplay` became `episodes`; a New Draft needs `screenplayId` when there are several; the report gives the stored title. |
| `packages/exchange.rs` | `current_draft` takes a screenplay id. `only_current_draft` serves packages without a draft and asks for one when there are several. A review package comes back only to its own screenplay. The Production Source label names the screenplay. |
| `ai/queries.rs`, `ai/scope.rs` | Draft labels name the screenplay when there are several ("Draft 2 of “The Short”"); references may name it; ordinals count per screenplay; "the current draft" with several screenplays and no open draft is a question, never a pick. |
| `ai/catalog/screenplay.rs` | `propose_screenplay` no longer refuses; its card lists the screenplays kept as they are and refuses a taken title. New tool `propose_rename_screenplay`. |
| `ai/catalog/story.rs` | Build Screenplay takes a `screenplay` argument; a new draft goes to the named, open or only screenplay, else the tool asks. |
| Other AI reads | Character rename, scene numbers, character cues, Production Source comparison and project overview work per screenplay. |
| Frontend | Screenplay picker (`workspaces/screenplay/picker.tsx`: switch, New Screenplay, Rename Screenplay) at the left of the toolbar, keyboard reachable. The route and saved view keep `screenplayId`. Build dialog: "Build a new screenplay document" enabled with a title field (mock 064). Apply order dialog: screenplay chooser. Import dialog: New Screenplay always offered, New Draft names its target, the open screenplay is offered first. Breakdown, Production overview and Schedule banners name the screenplay of the Production Source. The assistant's Current Screenplay scope sends the open draft. |
| Docs | `docs/engineering/32-screenplay-engine-contract.md` section 1.2; `docs/engineering/ai-tool-coverage.md` regenerated; user manual source (`docs/manual/OpenFrame_Studio_User_Manual.docx`) step 2 in the Screenplay empty state section. |

## Screenplay delete (second slice, same day)

Owner answers later on 2026-10-08 (ADR-0012 amendment): a screenplay can be deleted to Recently Deleted, the
Production Source screenplay cannot be deleted until another source is chosen, and an episode may hold several
screenplays with the same rules. Built after merging `session-2` into this branch (the 30-day purge in
`modules/retention.rs`, area 16 packages, the soak test fix and `.cargo/config.toml`).

| Place | Change |
|---|---|
| `screenplay/delete.rs` (new) | `screenplay.delete` (SoftDelete: Owner and Editor; confirmation; one transaction and one undo step). Marks only the `screenplay` row deleted, so drafts, locked drafts, scenes, production numbers, page setup, custom elements, character links and Story Board links are untouched and come back on restore. Refused with a plain message while one of its drafts is the active Production Source. Trash handler `screenplay`: restore waits for a deleted episode and gives a numbered title when the old one was taken meanwhile; purge removes every draft, scene, comment, history point, custom element and character link, and is refused while production planning refers to the screenplay. |
| `screenplay/mod.rs` | `load_draft` and `drafts_of` require a live screenplay, so a deleted screenplay's drafts can neither be read nor edited. |
| `screenplay/drafts.rs` | Draft purge split into `purge_draft_tx` (reused by the screenplay purge). A deleted draft of a deleted screenplay waits for the screenplay on restore. Rename no longer reindexes by hand. |
| `screenplay/extras.rs` | A deleted scene of a deleted screenplay waits for the screenplay on restore. |
| `registry.rs`, `store.rs` | New `Registry::search_dependents`: a change of a `screenplay` row re-indexes its current draft's scenes in every mutation, undo, redo and package apply, so search drops a deleted screenplay and finds it again after restore or undo. New hidden reason `WHOLE_SCREENPLAY`. A restore that changed a screenplay's title returns a `restoredNote`. |
| `project.rs` | Recently Deleted area "Screenplay" for the `screenplay` table. |
| Other readers | Package draft lookup, Story character scene counts, the AI character cue rename and the Story timeline scene check skip a deleted screenplay (the others already did). |
| Frontend | Picker: "Delete Screenplay…" for Owner and Editor (`permissions.delete`), keyboard reachable. `DeleteScreenplayDialog` names the screenplay, says it goes to Recently Deleted for 30 days, shows the core's reason when refused (Production Source), says when it is the only one, and opens the next screenplay afterwards. Recently Deleted: a draft whose screenplay is also deleted, and a screenplay whose episode is deleted, say to restore that first. |
| Docs | `docs/engineering/32-screenplay-engine-contract.md` section 1.2 (delete and restore, search dependents); `docs/engineering/ai-tool-coverage.md` regenerated (`screenplay.delete` hidden, reason `WHOLE_SCREENPLAY`); user manual source step 2 and a new edge-case bullet in the Screenplay empty state section. |

Choices made:

- **The last screenplay can be deleted.** Nothing in the code needs a screenplay to exist: a new project has none, the
  overview then returns no screenplay and the workspace shows its empty state with New Screenplay. Tested.
- **The assistant cannot delete a screenplay** (hidden, reason `WHOLE_SCREENPLAY`). It can still propose deleting one
  draft (`propose_delete_draft`) and restoring an item from Recently Deleted (`trash.restore`).
- **Permanent removal follows the draft rule.** A screenplay that production planning still refers to (an earlier
  Production Source, breakdown rows, schedule strips, sides) is never removed for good, by hand or by the 30-day
  purge; it stays in Recently Deleted and restorable, and the purge logs it as skipped. This is the same rule
  `purge_draft` already had.
- **A title taken while deleted** is not refused: the restored screenplay gets "Title (2)" and Recently Deleted's
  toast says so.

## Tests

| Test | What it proves |
|---|---|
| `screenplay.rs::fsd_script_001_empty_screenplay_without_story_board` | Flipped: a second `screenplay.create` adds "Black Rain (2)" beside the first. |
| `screenplay.rs::several_screenplays_keep_their_own_drafts_locks_numbers_and_page_setup` | Overview list and choice, stale id fallback, edit isolation, same draft names in both, lock in one leaves the other editable, production number and A4 page setup per screenplay, `locate` names the screenplay. |
| `screenplay.rs::screenplays_are_renamed_with_unique_titles_and_undo` | Duplicate titles refused on create and rename, rename updates the title page, search names the screenplay, undo, Viewer, Commenter and Export-only refused, every role sees both. |
| `story.rs::fsd_story_023_repeat_build_creates_a_new_draft_never_overwrites` | Flipped: a second "new screenplay" build is created beside the first, which keeps its drafts and current draft; each build is one undo step. |
| `interchange.rs::fsd_script_044_import_into_existing_screenplay_never_replaces_current_draft` | Flipped: a second New Screenplay import is allowed and independent; with several screenplays a New Draft must name its target. |
| `interchange.rs::episodic_projects_import_screenplays_into_a_chosen_episode` | An episode can take a second screenplay; an unknown episode is refused. |
| `ai_toolbox.rs::several_screenplays_are_named_and_never_picked_silently` (new) and the flipped step in the screenplay fixture test | AI creates a second screenplay with a "Kept as they are" row, asks which screenplay for "Draft 1", "current" and Build Screenplay new draft, resolves "Draft 1 of Railway (short version)", renames by name. |
| `migration_fixtures.rs::a_project_from_before_several_screenplays_opens_with_its_one_screenplay_intact` (upgrade test) | A project built at every schema version from 0012 to the current one opens with exactly its one screenplay, chosen without asking, text intact, and can then take a second without touching the first. |
| `screenplay_delete.rs` (new, 7 tests) | Delete and restore with drafts, lock, scene number, A4, links and search intact; reads and edits of a deleted screenplay refused; a deleted draft waits for its screenplay; Viewer, Commenter and Export-only refused, Editor allowed; undo and redo, search included; the assistant cannot delete (hidden, not exposed) but can restore; Production Source refusal until `production.apply_update` moves production to the other screenplay; a former source is never purged (by hand or after 31 days) and stays restorable; the last screenplay deleted and a new one started; a title taken meanwhile restores as "Black Rain (2)" with a note; disk full and write failure leave nothing half done; a crash after the delete keeps it; removed for good exactly 30 days after the delete with its drafts, scenes and lock records and no dangling reference; an episode with two screenplays loses one and gets it back, waiting for a deleted episode. |
| `entitlement.rs::plat_009_the_store_lock_refuses_deleting_or_restoring_a_screenplay` (new) | The store lock refuses `screenplay.delete`, `trash.restore` and the Recently Deleted list; after the purchase both screenplays are there and the delete works. |
| `picker.test.tsx` (Vitest, 10 tests; 3 new) | Delete only for roles that may delete, from the keyboard; the confirmation names the screenplay and says Recently Deleted and 30 days, Cancel changes nothing, the delete opens the other screenplay; the Production Source refusal is shown inline; the last screenplay can be deleted. |
| `RecentlyDeleted.test.tsx` (new case) | A deleted screenplay is listed and restored from the keyboard, the restore message carries the title note, a deleted draft of a deleted screenplay says to restore the screenplay first. |
| `picker.test.tsx` (Vitest, 7 tests, first slice) | Picker names the open screenplay and switches from the keyboard, New and Rename only for editors, create with typed title, duplicate refused, episode sent, rename and the core's refusal shown, `useOverview` sends `screenplayId`. |
| `BuildDialog.test.tsx` (new case) | New screenplay document enabled with a unique title. |
| `model.test.ts` (interchange) | Import modes and readiness with several screenplays. |

Gates run on 2026-10-08 in the worktree (build folder `target-area11s`):

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass, no warnings |
| `cargo test --workspace -- --test-threads=4` | Pass: 1,869 passed, 0 failed, 9 ignored (90 test binaries) |
| `npm run typecheck` | Pass |
| `npm --workspace apps/desktop run lint` (the frontend half of `npm run lint`; the Rust half is the clippy row) | Pass |
| `npm run test:web` | Pass: 45 files, 501 tests |
| `npm run check:governance` | Pass (63 governance tests; writing standard counts did not rise) |

Gates run on 2026-10-08 after the screenplay delete slice (same worktree and build folder):

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass, no warnings |
| `cargo test --workspace -- --test-threads=6` | Pass: 1,945 passed, 0 failed, 9 ignored (93 test binaries) |
| `npm run typecheck` | Pass |
| `npm --workspace apps/desktop run lint` | Pass |
| `npm run test:web` | Pass: 50 files, 523 tests |
| `npm run check:governance` | Pass (64 governance tests; writing standard counts did not rise, and the touched files lost 13 em dashes) |

Gates run once more after merging `session-2` with area 14 (Schedule and Day Out of Days, migration 0015). The two
search hooks are kept side by side: area 14's `index_parent` (a strip or Day Out of Days override re-indexes its
shooting day) and this branch's `search_dependents` (a screenplay re-indexes its scenes). The v14 to v15 schedule
upgrade test and the several-screenplays upgrade test (which now also builds a v15 fixture) are both kept.

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | Pass |
| `npm run lint` (clippy `-D warnings` and ESLint) | Pass |
| `cargo test --workspace -- --test-threads=6` | Pass: 1,987 passed, 0 failed, 9 ignored (94 test binaries), package coverage included |
| `npm run typecheck` | Pass |
| `npm run test:web` | Pass: 52 files, 532 tests |
| `npm run check:governance` | Pass (64 governance tests; writing standard counts did not rise) |

End-to-end spec (not run here; the lead runs it): `tests/e2e/specs/43-several-screenplays.test.mjs`. It creates a
second screenplay from the picker, writes in both, switches, renames, quits, relaunches and checks both keep their own
text. A third test (added with screenplay delete, also not run here) deletes the second screenplay from the picker,
checks the confirmation text, restores it from Recently Deleted and finds its text again.

## Proposed feature-status rows (for the lead)

| Feature | Importance | What is built | What is missing | Area | Route | Tests | Status |
|---|---|---|---|---|---|---|---|
| Several screenplays per project | Should ship | Picker (switch, new, rename, delete), per-screenplay drafts, locks, numbers and page setup; delete to Recently Deleted with restore, Production Source refusal and 30-day purge; Story Board, import, packages and AI name the screenplay; upgrade keeps the one screenplay (checked 2026-10-08) | E2E spec 43 run | Screenplay | screenplay (toolbar picker) | screenplay.rs::several_screenplays_keep_their_own_drafts_locks_numbers_and_page_setup, ::screenplays_are_renamed_with_unique_titles_and_undo; screenplay_delete.rs (7 tests); entitlement.rs::plat_009_the_store_lock_refuses_deleting_or_restoring_a_screenplay; migration_fixtures.rs::a_project_from_before_several_screenplays_opens_with_its_one_screenplay_intact; ai_toolbox.rs::several_screenplays_are_named_and_never_picked_silently; picker.test.tsx | Reachable |

Row to update: "Scene Card links stay correct when screenplay scene identities change" still says several screenplays
per project is an open owner decision; it is decided and built.

## Not done

- Deleting a screenplay: built in the second slice (above).
- The search title of a scene still reads "Scene 12 — HEADING" with an em dash. The same pattern is used by the AI
  context line, exports, shots, storyboards and review threads, so it should change everywhere at once in its own
  change, not only in search.
- The PDF copy of the user manual is not regenerated from the edited `.docx`.
- The masterplan box, progress log and feature status are left to the lead.
