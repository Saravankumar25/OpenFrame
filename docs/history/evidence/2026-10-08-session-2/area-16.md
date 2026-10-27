# Area 16, Packages and backup: evidence of 2026-10-08

Branch `area/16-packages` in the worktree `agent-a9f39a60d18bcbf69`, with `session-2` merged in (first `79a660a`,
then `c6d3ee8`). Windows 11 machine of the owner. The first run used the shared Cargo target
`C:\OpenFrame\.build-cache\target`; the follow-up run (section 6) used this worktree's own build folder
`C:\OpenFrame\.build-cache\target-area16` with sccache, and its results replace those of section 4. The E2E spec was
written and extended but not run here; the lead runs E2E one spec at a time.

## 1. Boxes

| Box (masterplan area 16) | State | Proving code | Proving tests |
|---|---|---|---|
| **Exchange, Review and Response packages** | Finished in code and Rust and web tests; E2E written, not run | `packages/exchange.rs`, `import.rs`, `review.rs`, `features/packages/*` | `packages.rs`: `fsd_col_001_exchange_package_can_be_inspected_in_another_installation`, `fsd_47_7_response_comments_attach_to_matching_scenes`, `fsd_col_002_stale_response_is_imported_as_comments_only_without_changing_text` (version mismatch), `fsd_col_015_016_ambiguous_and_unmatched_notes_go_to_the_review_queue`, `fsd_121_cancel_leaves_the_project_unchanged_and_permissions_apply` (preview before apply), `exchange_identity_collision_never_overwrites_another_project`, `package_roles_scope_what_each_member_can_send_and_apply`, `fsd_46_7_private_notes_are_never_in_exchange_packages`, `too_new_package_format_is_refused_clearly`; `ReviewViewer.test.tsx` (2); `packages.test.ts`; E2E `70-packages.test.mjs` test "review round trip" |
| **Backup and whole-project package** | Finished in code and tests, including restored private notes (sections 3 and 6); E2E written, not run | `packages/backup.rs`, `format.rs`, `coverage.rs`, `BackupDialogs.tsx`, `OpenProjectDialog.tsx`, Home entry in `AppHome.tsx` | `packages.rs`: `fsd_44_backup_round_trip_restores_as_copy_with_new_identity`, `fsd_44_interrupted_backup_leaves_no_partial_file`, `backup_restores_on_a_new_computer_with_a_fresh_data_root`, `corrupt_backups_are_detected_and_leave_nothing_behind`, `interrupted_restore_leaves_nothing_behind`, `fsd_45_project_package_round_trip_across_installations`, `fsd_45_4_identity_collision_open_as_copy_or_replace_after_backup`, `full_project_package_holds_no_trace_of_other_members_private_notes`, `restored_private_notes_belong_to_the_restorer_and_stay_private`; `format.rs` module `hostile` (5 tests); `archive.rs::damaged_entry_data_is_a_rejected_archive_not_a_storage_error`; `AppHome.test.tsx` (Restore Backup or Package entry); `packages.test.ts` (restore screen) |
| **Packages carry the new data** | Partly finished: production numbers and custom elements done with format 2 and compatibility fixtures; schedule timeline and budget Planned and Actual do not exist in the schema yet (areas 14 and 15) | `exchange.rs` (`productionNumber`, `customElements`, `customElementId`), `review.rs`, `coverage.rs` (entries for migrations 12 to 14), `PACKAGE_FORMAT_VERSION = 2` | `packages.rs::production_numbers_and_custom_elements_travel_in_every_package_class`, `::older_package_formats_still_open` (fixtures `tests/fixtures/packages/format-1/`, `format-2/`), `::current_format_manifests_carry_counts_and_coverage`; `coverage.rs::package_coverage_follows_the_project_schema` fails any later migration (the day timeline, budget Planned and Actual) until its package behaviour is decided |
| **No network listener anywhere** | Finished | ADR-0007; no server crate | `packages.rs::the_core_workflow_opens_no_network_listener`, `::the_desktop_app_depends_on_no_network_server_crate`, `::no_lan_collaboration_wording_in_user_facing_text`, `::the_lan_wording_check_catches_offers` |
| **Packages end-to-end test and restore drill** | Written; E2E not run; manual drill not run (needs a second Windows account) | `tests/e2e/specs/70-packages.test.mjs`, `tests/e2e/lib/native-dialog.mjs`, `docs/engineering/39-packages-restore-drill.md` | to be run by the lead |

## 2. What changed in this session

1. Line endings: the three earlier in-progress commits had rewritten 21 files with CRLF. They are back to LF, so
   the branch diff shows only real changes.
2. Merge of `session-2`: one conflict in `AppHome.tsx`, resolved by keeping both the Appearance button and keyboard
   shortcut titles from `session-2` and the Restore Backup or Package entry from this area.
3. Real defects fixed:
   - Every Full Project Package export failed after `session-2` hardened connections (attach limit 0): SQLite runs
     VACUUM through an internal ATTACH. The limit is lifted for that statement only and restored afterwards.
   - A package with damaged entry data (bad CRC) was reported as "could not save this change" instead of a damaged
     package. The archive reader now maps read errors to `import.unsafe_archive`.
   - `content.json` had no size bound beyond the archive limit (256 MB); it is now refused above 64 MB before parsing.
   - The coverage list did not know the tables and columns of migrations 12 to 14 or the migration record table.
   - The network crate test read `Cargo.lock`, which lists `quinn` for a weak HTTP/3 feature of reqwest 0.12 that is
     never compiled. It now reads the resolved graph from `cargo tree`.
   - Two tests assumed three scenes; `screenplay.create` in `session-2` makes a first empty scene.
4. New: production numbers and custom elements in exchange packages and the reviewer view; hostile input tests; a
   review round trip in the E2E spec; package format 1 and 2 fixtures; viewer component tests.
5. Architecture lint: `packages/coverage.rs` added to the reviewed write allow-list (the scrub moved there from
   `backup.rs`; it writes only to the private staging copy).

Definition of done notes: no operation was added or changed in permission, so the store-lock refusal on the single
dispatch path (`entitlement.rs` test "the assistant and packages are refused too") still covers every package
operation. The AI decision is unchanged: package operations stay hidden from the assistant except the read-only
`package_history` tool; `docs/engineering/ai-tool-coverage.md` was regenerated by the workspace test run with no
change. Undo: applied imports have Undo Import (`packages.undo_import`); backup and export write files and are not
undoable by design. Keyboard: Ctrl+Enter adds a reviewer comment (web test and E2E).

## 3. Owner question, answered 2026-10-08

Private notes in a restored backup. The first run found that restored notes kept the old installation's owner, so a
person restoring their own backup on a new computer could not see them. The owner answered on 2026-10-08 (ADR-0012
amendment "private notes in a restored backup"): restoring a backup or whole-project package makes the person
restoring it the owner of the private notes it contains. Implemented in the follow-up run, section 6.

## 4. Commands and results (HEAD `9c7b9f9`, before this evidence file)

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS, no warnings |
| `cargo test --workspace --no-fail-fast` | PASS: 1891 passed, 0 failed, 10 ignored, 90 test binaries. The first attempt could not link `openframe-ai` test `lifecycle` (LNK1104) because another agent was running the same binary in the shared target; the retry passed. |
| `cargo test -p openframe-application --test packages` | PASS: 32 passed, 1 ignored (the fixture writer) |
| `cargo test -p openframe-application --lib packages` | PASS: 69 passed |
| `npm run typecheck` | PASS |
| `npm run lint` (ESLint part, `--max-warnings 0`) | PASS |
| `npm run test:web` | PASS: 45 files, 497 tests |
| `npm run check:governance` | PASS: architecture lint, migrations, feature status, reachability, writing standard, links, plan status, 63 governance tests |
| `npm run test:e2e` for `70-packages.test.mjs` | NOT RUN here |

Shared target warning for the lead: crates of different worktrees share one Cargo unit in the shared target (the
unit hash does not include the worktree path), and Cargo decides freshness by file times. A build in one worktree can
reuse a crate compiled from another worktree's source. This session saw `openframe-domain` from another worktree
(without `OLDEST_PACKAGE_FORMAT_VERSION`) and worked around it by touching its own `.rs` files before each build.

## 5. Proposed feature status changes (for the lead to apply)

| Feature row | Now | Proposed | Condition |
|---|---|---|---|
| Exchange / Review / Response packages (XFER-011) | Reachable | Verified | after `70-packages.test.mjs` passes; add `ReviewViewer.test.tsx`, `production_numbers_and_custom_elements_travel_in_every_package_class` and the hostile tests to its evidence; the gap "identity collision tests" is closed by `exchange_identity_collision_never_overwrites_another_project` |
| Backup, whole-project package, import with identity collision (XFER-012) | Reachable | Verified | after `70-packages.test.mjs` passes; the gap "Restore entry on Application Home" is closed (Home has Restore Backup or Package); the restore drill is written; the manual drill stays open (the private note question was answered, section 6) |
| Live LAN collaboration (XFER-013) | Removed | unchanged | |

Plan boxes this session can tick after the E2E run: "No network listener anywhere" now; "Exchange, Review and
Response packages" and "Backup and whole-project package" after the E2E passes (the owner's
answer for the second came on 2026-10-08 and is built, section 6). "Packages carry the new data" stays open until areas 14 and 15 add the schedule timeline and the
budget Planned and Actual columns and decide their package coverage. "Packages end-to-end test and restore drill"
needs the E2E run and the manual drill.

## 6. Follow-up run of 2026-10-08: restored private notes, own build folder

### What changed

1. `session-2` at `c6d3ee8` merged in (ADR-0012 amendment, build folder decision), no conflicts.
2. Restore path (`packages/backup.rs`, `adopt_private_notes`): inside the same transaction as the rest of the
   restore, on the staged database before it is moved into place, every private note gets the restoring user as
   owner, its search entry gets the same owner, and undo entries that hold a note's old row image (with the old
   owner) are dropped so an undo can never hand the note back. A crash before the commit leaves no project at all
   (`interrupted_restore_leaves_nothing_behind` still passes). It applies to Backup Packages and Full Project
   Packages in every open mode (open, copy, replace). The result message adds "N private notes from the package are
   now yours." when owners changed.
3. Restore screen (`OpenProjectDialog.tsx`, helper `restoredNotesNotice` in `features/packages/api.ts`): before the
   restore runs, a notice says the package's private notes will become yours and that other members of the project
   cannot see them. It uses the manifest's private note count; format 1 packages, which carry no counts, get the
   notice without a number; it is hidden when the package holds none.
4. Tests: `backup_restores_on_a_new_computer_with_a_fresh_data_root` now asserts the new behaviour (the note belongs
   to the restorer, is listed for them, and the report says so). New
   `restored_private_notes_belong_to_the_restorer_and_stay_private`: for a backup (two notes, one from a colleague)
   and a Full Project Package (the exporter's note only), the restorer owns every note in the table and in the
   search index, lists and finds them, no undo entry with the old owner remains, edits one, and the edit survives a
   restart; another member of the restored project lists nothing, finds nothing in search (old or new text) and gets
   `not_found.private_note` on update and delete. `older_package_formats_still_open` also checks that the restorer
   sees the note for the format 1 and format 2 fixtures. Web: `packages.test.ts` "restore screen". E2E:
   `70-packages.test.mjs` adds a private note before the backup and waits for the notice on the restore screen (not
   run here).
5. Docs: `docs/engineering/39-packages-restore-drill.md` (user steps, what must hold, manual drill step 2) and
   `docs/engineering/20-security-threat-model.md` row T10.

### Commands and results

Every cargo command ran with `CARGO_TARGET_DIR=C:\OpenFrame\.build-cache\target-area16` and
`RUSTC_WRAPPER=sccache`; the workspace tests and the second lint run also had `CARGO_BUILD_JOBS=3`,
`CARGO_INCREMENTAL=0` and `--test-threads=4` because the machine was short of memory.

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS, no warnings |
| `cargo test -p openframe-application --test packages` | PASS: 33 passed, 1 ignored (the fixture writer) |
| `cargo test --workspace --no-fail-fast -- --test-threads=4` | 1891 passed, 1 failed, 10 ignored, 90 test binaries. The failure is `openframe-ai` `soak_sidecar::m1_08_sidecar_crash_restart_and_cancel_loop` ("no fake sidecar process survives the loop"), which counts live processes of its own image right after stopping the supervisor. It failed again when run alone. Area 16 does not change `openframe-ai` (no diff against `c6d3ee8`), no process of that binary was left afterwards, and the machine was swapping with five workspace builds at once; it should be rerun on a quiet machine. |
| `npm run typecheck` | PASS |
| `npm run lint` (clippy, then ESLint `--max-warnings 0`) | PASS. A first run picked up the user-level `CARGO_TARGET_DIR` (the shared target) for its clippy step by mistake; it was rerun in `target-area16` and only that run counts. |
| `npm run test:web` | PASS: 45 files, 498 tests |
| `npm run check:governance` | PASS: architecture lint, migrations, feature status, reachability, writing standard, links, plan status, 63 governance tests |
| `tests/e2e/specs/70-packages.test.mjs` | NOT RUN here (the lead runs it) |
