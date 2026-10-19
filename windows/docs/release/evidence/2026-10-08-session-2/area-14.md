# Area 14 evidence: Shooting schedule and Day Out of Days

Recorded 2026-10-08 on branch `area/14-schedule` (worktree `agent-aae076af1169f70c9`), on top of `session-2` at
`79a660a`. Machine: the owner's Windows 11 development machine (build 26200). Masterplan section "14. Shooting
schedule and Day Out of Days"; contract `docs/engineering/33-production-depth-contract.md` section 4.

The lead ticks the plan boxes and moves the feature rows after running the end-to-end spec. This file only records
what was built, which tests prove it, and what was run.

## 1. Boxes

Result labels follow `docs/release/evidence/README.md` section 2.

| Box | Status | Code | Proof (tests) |
|---|---|---|---|
| **Day timeline** | Finished, end-to-end run pending | Migration `0015_schedule_timeline.sql` (`shooting_day.crew_call`, `shooting_start`, `target_wrap`); `schedule.set_day_times` in `modules/schedule/ops.rs`; `modules/schedule/timeline.rs`; `DayTimes` in `call-sheets/shared/ScheduleViews.tsx` | `schedule_depth.rs::sched_003_day_times_are_stored_undoable_and_validated`, `::night_shoot_wraps_after_midnight`, `::day_times_starts_and_dood_corrections_survive_a_crash`, `::finalized_schedule_refuses_times_starts_and_corrections`, `::commenters_and_viewers_cannot_change_times_starts_or_corrections`; `timeline.rs` unit tests (4); `entitlement.rs` store lock list; `migration_fixtures.rs::v14_to_v15_keeps_an_existing_schedule_and_adds_empty_day_times`; `api/schedule.test.ts` (clock parsing) |
| **Scene start time and computed timeline** | Finished, end-to-end run pending | `schedule_strip.start_minutes`; `schedule.set_strip_start` (never moves a scene); meal, travel, company move and custom markers feed `timeline.rs` and the day total; strip dialog in `ScheduleDialogs.tsx`; start shown on the strip (`Strip.tsx`) | `schedule_depth.rs::sched_004_scene_starts_and_breaks_feed_the_timeline_and_day_total`; `Strip.test.tsx` "shows a pinned start time and fades strips the filter does not match"; `perf.rs` P16 `schedule.set_strip_start` |
| **Availability warnings** | Finished, end-to-end run pending | `modules/schedule/availability.rs` reads dated Unavailable windows through `production::availability::unavailable_for_schedule`; `travel_overflow`, target wrap inside `duration_overflow`, `cast_unavailable`, `location_unavailable` in `board.rs`; advice only (Keep anyway, strict mode only asks) | `schedule_depth.rs::sched_005_over_target_and_travel_warnings_are_advice_only`, `::sched_005_cast_and_location_availability_warn_on_dated_days_only`, `::missing_location_still_warns`; `availability.rs` unit tests (2); `schedule.rs::fsd_sch_003_warnings_are_advisory_with_keep_anyway` |
| **Filters** | Finished, end-to-end run pending | `call-sheets/shared/scheduleFilter.ts` (location, cast, day or night, length, scheduled, breakdown complete, day, revision) and `ScheduleFilterBar.tsx`; view state only, no operation | `scheduleFilter.test.ts` "match on every dimension of spec section 27", "never changes the schedule it filters" |
| **No automatic planner** | Finished | No operation moves a scene except the user's move, drag or confirmed suggestion | `schedule_depth.rs::sched_009_nothing_moves_a_scene_without_a_user_move`; `schedule.rs::fsd_sched_014_grouping_suggestions_need_explicit_confirmation` |
| **Stale and review state** | Finished, end-to-end run pending | Day times, starts and estimates mark the day's call sheet Needs refresh (`sync_call_sheets`); snapshots without times keep their old fingerprint | `schedule_depth.rs::sched_007_call_sheet_stale_flag_follows_day_times_and_starts`, `::sched_007_script_change_review_keeps_new_fields`, `::call_sheet_snapshot_without_times_keeps_its_old_shape`, `::migration_0015_upgrades_an_existing_schedule_without_marking_call_sheets_stale` |
| **Day Out of Days** | Finished, end-to-end run pending | `modules/schedule/dood.rs` (`schedule.dood`, `schedule.set_dood_override`), `dood_override` table, `production/tabs/dood.tsx` (cast and day range pickers, cell menu, correction note dialog), PDF, CSV and XLSX in `modules/exports/dood.rs`; read-only to screenplay and breakdown | `schedule_depth.rs::dood_001_derived_from_cast_and_schedule_with_ranges_and_corrections`, `::dood_corrections_and_day_times_survive_restart`, `::dood_export_needs_days_and_cast`, `::dood_correction_notes_are_searchable_on_their_day`; `dood.rs` unit test; `dood.test.tsx` (4) |
| **Schedule exports** (PDF and spreadsheet) are reproducible | Finished, end-to-end run pending | Exports carry the schedule's last change date (`board::revised_at`), XLSX creation date fixed through `tables_to_xlsx_at` | `schedule_depth.rs::sched_008_schedule_and_dood_exports_are_byte_reproducible`; `exports.rs` schedule tests |
| **Schedule end-to-end test** | Not finished: written, not run (the lead runs end-to-end tests serially) | `tests/e2e/specs/61-schedule.test.mjs` | Covers create, drag (undo, redo), menu move, off day, meal break, day times and timeline, a forced over-target conflict with Keep anyway, grouping that moves nothing, filters, call sheet stale review (undo, redo), Day Out of Days correction (undo, redo) with a note, byte-reproducible PDF and XLSX exports of both documents, quit and relaunch |

## 2. Fixes made in this pass

- **Search after undo.** Undo and redo reindexed only the rows they restored, so a shooting day's search document
  kept an undone Day Out of Days note and kept listing a scene whose move was undone. `Registry::index_parent`
  (registry.rs) and `changed_rows` (store.rs) now reindex the parent day for `dood_override` and `schedule_strip`
  changes on every mutation, undo and redo. Proof: `dood_correction_notes_are_searchable_on_their_day`.
- **Correction notes were unreachable.** The backend stored notes but the screen always sent `note: null`, and
  changing a corrected code wiped its note. The cell menu now keeps the note and offers "Add a note" or "Edit the
  note" (`DoodNoteDialog`). Proof: `dood.test.tsx`, end-to-end step "Day Out of Days".
- **Test files in the production tab bundle.** `production/index.tsx` collected every `./tabs/*.tsx`, which would
  also pull a test file into the app bundle and the tab list. The glob now excludes `*.test.tsx`.
- **Test corrections.** `sched_004` expected a 45 minute gap where the timeline correctly gives 165 (14:00 minus the
  11:15 computed start); `Strip.test.tsx` matched the label pieces in the wrong order; `v14_to_v15` used a column
  name that does not exist (`date` instead of `shoot_date`); `production_depth.rs` asserted that only migration 14
  runs on a schema 13 database although 15 now exists (it now upgrades to exactly 14); the AI schema sampler in
  `ai_toolbox.rs` gave "Ravi" to clock fields described as `HH:MM`. No test was skipped or weakened.
- **End-to-end spec tightened.** The conflict step forces a real over-target warning (day target 30 minutes) and
  always clicks Keep anyway; the correction checks look for "1 corrected by hand" instead of the legend text that is
  always on screen.
- **Docs.** Contract doc 33 section 4 (availability source, search), `11-physical-database.md` (migrations 12 to 15),
  `23-release-migration.md` (schema version 15 and the allocation note), regenerated `ai-tool-coverage.md`.
- **Performance suite.** `perf.rs` measures P16 `schedule.set_day_times`, `schedule.set_strip_start`,
  `schedule.dood` and P18 `schedule.export_dood`.

## 3. AI decision

Exposed through proposals: `schedule.set_day_times` (`propose_update_shooting_day`) and `schedule.set_strip_start`
(`propose_scene_timing`). Read tool: `day_out_of_days` (`schedule.dood`); `schedule_day` reports day times.
Hidden: `schedule.set_dood_override` (manual knowledge about holds, travel and fittings stays the user's), and
`schedule.export_dood` (user-picked path). Regenerated `docs/engineering/ai-tool-coverage.md` from the live registry.

## 4. Commands run

All Rust commands ran with `CARGO_TARGET_DIR=C:\OpenFrame\.build-cache\target-area14` and `RUSTC_WRAPPER=sccache`
(the lead's build folder change of 2026-10-08). Earlier runs against the shared `target` folder are discarded:
cargo reused crates and test binaries compiled from other worktrees, which produced false failures and could
produce false passes.

Code under test: commit `020e9f0` (this file is committed after it). Rust commands also used
`CARGO_BUILD_JOBS=3`, `CARGO_INCREMENTAL=0` and `--test-threads=4` because five builds shared 15 GB of memory.

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` (also inside `npm run lint`) | PASS |
| `cargo test --workspace --no-fail-fast -- --test-threads=4` | 1902 passed, 3 failed, 9 ignored (91 test binaries); the 3 failures are below |
| `cargo test -p openframe-application --test failure_injection -- --test-threads=1` (rerun alone) | PASS, 16 passed |
| `cargo test -p openframe-application --test forced_kill -- --test-threads=1` (rerun alone) | PASS, 5 passed, 1 ignored |
| `cargo test -p openframe-ai --test soak_sidecar -- --test-threads=1` (rerun alone) | FAIL, 2 passed, 1 failed (see below) |
| `npm run typecheck` | PASS |
| `npm run lint` (clippy and `eslint --max-warnings 0`) | PASS |
| `npm run test:web` | PASS, 46 files, 502 tests |
| `npm run check:governance` | PASS, 63 governance tests; writing check OK |
| `vite build` into a scratch folder | PASS (checks the tab glob change) |

A follow-up commit after `020e9f0` changes only comments and test failure messages (em dashes in files this branch
touched) and restores `ProjectManifest.ts`: the copy in `020e9f0` had been generated from another worktree's crate
through the shared build folder, and the clean run regenerated it from this branch's source. After that commit,
`cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings` and `npm run check:writing` ran again and
passed; the full test run was not repeated for a comment-only change.

Writing hits left in touched files on purpose: product-wide strings shared with other areas, each using an em dash: the
"Recoverable" row of every AI change set catalog, the "Call Sheet, Day 1" style titles and the empty-value marker on
call sheets, the moodboard and storyboard export titles, and the test assertions on them. Changing them belongs to a product-wide copy
sweep, not to this area, because it changes call sheet and export output that areas 15 and 9 own.

The three failures in the full run depend on the machine, not on area 14 code, and none of them touches the
schedule:

- `failure_injection::m1_11_external_volume_removed_while_open` failed with "Drive already SUBSTed": another
  worktree's run held the same drive letter. It passes when rerun alone.
- `forced_kill::m1_03_forced_kill_during_migration_leaves_a_consistent_database` failed with "the migration worker
  completed no cycle in 700 ms" while the machine was swapping. It passes when rerun alone.
- `soak_sidecar::m1_08_sidecar_crash_restart_and_cancel_loop` (crate `openframe-ai`, unchanged by this branch)
  counts processes by the test binary's image name across the whole machine. Other worktrees run a binary with
  the same name at the same time, so the count moves (expected 3, found 1). Result: FAIL here; it needs a quiet
  machine to confirm, and its process count should be scoped to the test's own children (a separate fix).

Area 14 test binaries in the full run: `schedule_depth` 19 passed, `schedule` 24, `entitlement` 15, `exports` 20,
`migration_fixtures` 7, `production_depth` 14, `ai_toolbox` 10, `ai_tool_coverage` 4, `perf` 5 (1 ignored).

Not run here: the end-to-end suite (lead), `npm run perf` in the optimised `perf` profile. A debug-build run of
`perf_feature_120_budgets` against the shared folder showed every schedule row inside its budget (P16
`set_day_times` p95 69 ms, `set_strip_start` 54 ms, `schedule.dood` 31 ms, P18 `export_dood` 61 ms); that run is
indicative only.

## 5. Proposed feature status rows (lead to apply after the end-to-end run)

| Row | Now | Proposed | Evidence to cite |
|---|---|---|---|
| Day timeline (SCHED-003) | Specified | Verified when `61-schedule` passes; Reachable now (production/schedule, day view) | section 1 row 1 |
| Scene start time and duration on a strip (SCHED-004) | Designed | Verified when `61-schedule` passes; Reachable now | section 1 row 2 |
| Availability warnings (SCHED-005) | Designed | Verified when `61-schedule` passes; Reachable now | section 1 row 3 |
| Filters (SCHED-006) | Specified | Verified when `61-schedule` passes; Reachable now (production/schedule List and Board) | section 1 row 4 |
| No automatic optimizer ever moves scenes (SCHED-009) | Built | Verified | `sched_009_nothing_moves_a_scene_without_a_user_move` |
| Stale and review state (SCHED-007) | Reachable | Verified when `61-schedule` passes | section 1 row 6 |
| Schedule PDF and spreadsheet export (SCHED-008) | Reachable | Verified when `61-schedule` passes | `sched_008_schedule_and_dood_exports_are_byte_reproducible` |
| Derived Day Out of Days (DOOD-001) | Specified | Verified when `61-schedule` passes; Reachable now (production/dood) | section 1 row 7 |

None is proposed as Release-ready: the optimised performance run, the accessibility pass and the user manual
(`docs/manual` holds only the .docx and .pdf; no manual source was updated) are still open.

## 6. Owner decisions

None of the area 14 boxes waits on an owner decision or a Proposed ADR.

## 7. Notes for the lead

- `npm run check:writing` reports that the app, crates and living-docs baselines can be lowered; the baseline was
  not changed in this branch to avoid a conflict with the other areas.
- The shared build folder problem above also explains the "unresolved import `tables_to_xlsx_at`" and "file is
  being used by another process" failures seen during this session; neither was a code fault.
