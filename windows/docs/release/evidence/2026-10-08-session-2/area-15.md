# Area 15 evidence: Call sheets, budget, templates, samples and exports

Recorded 2026-10-08 on branch `area/15-callsheets-exports` (worktree `agent-aebb0069ec54c9b41`), after merging
`session-2` at `6b4f596`. Machine: the owner's Windows 11 development machine. Masterplan section "15. Call sheets,
budget, templates, samples and exports"; contract `docs/engineering/33-production-depth-contract.md` sections 5 to 8.

The lead ticks the plan boxes and moves the feature rows after running the end-to-end spec. This file records what was
built, which tests prove it, and what was run. Result labels follow `docs/release/evidence/README.md` section 2.

## 1. Boxes

| Box | Status | Code | Proof (tests) |
|---|---|---|---|
| **Call sheet content** | Finished, end-to-end run pending | `modules/schedule/callsheet_source.rs` (one definition of the source: scenes with I/E, D/N, cast ID numbers and set, the day's rows with start times and lengths from `Board::day_timeline`, meal, travel and company move rows, cast, locations with parking, access and contact, extras and department items from the breakdown, the next shooting day); `modules/schedule/callsheet.rs` (document version 2: header, day times, weather with sunrise and sunset, locations with hospital, scene schedule, talent with status, pickup, makeup and contact, extras, department notes, advance schedule, crew by department, walkie channels and contacts, practical notes, attachments, reference images, special notes); `exports/callsheet.rs` (PDF); `workspaces/call-sheets/CallSheetEditor.tsx`. Unknown values read "Needs input" on screen and in the PDF; weather, sunrise and sunset are typed, never looked up | `callsheet.rs` unit tests (prefill, merge, estimated starts, `merge_keeps_a_typed_actor_and_follows_the_cast_record_otherwise`); `callsheet_source.rs` unit tests (2); `schedule.rs::fsd_call_001_call_sheet_generated_from_shooting_day`; `exports.rs::call_sheet_pdf_has_paper_layout_and_final_sheet_uses_its_snapshot`; `pdf_regression.rs::call_sheet_pdf_matches_its_golden`; `CallSheetEditor.test.tsx` (7) |
| **Six default sections;** the rest through "Add Section"; empty optional sections stay hidden | Finished, end-to-end run pending | `DEFAULT_SECTIONS`, `OPTIONAL_SECTIONS`, `callsheets.add_section` (prefills from crew records, the day's extras and department items, the next shooting day); optional sections are `None` until added or filled from project data; the PDF leaves out an added section that is still empty | `schedule.rs::add_section_is_searchable_undoable_and_needs_edit_rights`; `CallSheetEditor.test.tsx` "shows the six default sections and hides optional ones that were not added", "shows an added optional section and lets the user remove it" |
| **Update preview** lists changed scenes, cast calls and locations; finalized sheets stay unchanged snapshots; status words follow the owner's answer | Finished, end-to-end run pending | Migration `0016_callsheets_budget.sql` maps Ready to Draft, Needs Refresh to a flagged Draft, Issued and Superseded to Final; `callsheets.set_ready` and `callsheets.issue` removed; `callsheets.refresh_preview` (added, removed and changed scenes, cast calls, locations, date, other areas, side by side rows); Final exports from its `snapshot` row; a replaced revision stays Final and keeps `superseded_by`; a source stored before 0016 is compared in its old shape (`CallSheetSource::fingerprint_as`) so an upgrade never marks a draft stale on its own | `schedule.rs::fsd_call_009_schedule_change_marks_call_sheet_needs_refresh`, `::fsd_call_011_finalized_call_sheet_is_immutable_and_revised`, `::stale_call_sheet_needs_explicit_choice_to_finalize`; `schedule_depth.rs::sched_007_call_sheet_stale_flag_follows_day_times_and_starts`, `::call_sheet_snapshot_without_times_keeps_its_old_fingerprint`; `migration_fixtures.rs::v15_to_v16_keeps_amounts_as_planned_and_maps_call_sheet_status_words`; `api/schedule.test.ts` "uses the owner's three call sheet status words" |
| **Editing a call sheet never changes** the screenplay, breakdown or schedule (test) | Finished | Call sheet operations write only `call_sheet`, `snapshot` and assets | `schedule.rs::call_sheet_operations_never_change_screenplay_breakdown_or_schedule` (every row of 14 screenplay, breakdown, catalog, location, cast, crew and schedule tables compared before and after edit, every Add Section, refresh, finalize, new revision and delete), `::fsd_call_002_call_sheet_edits_never_change_the_schedule` |
| **Lightweight budget** with Planned and Actual, category totals, difference and report; existing amounts become Planned | Finished, end-to-end run pending | `budget_line.amount` is Planned, `budget_line.actual` added by 0016 (NULL until entered); totals and difference derived in `modules/schedule/budget.rs` (difference only over lines with an Actual); `exports/budget.rs` (PDF, CSV, XLSX with Planned, Actual and Difference per line and category); `production/tabs/budget.tsx` | `migration_fixtures.rs::v15_to_v16_keeps_amounts_as_planned_and_maps_call_sheet_status_words`; `exports.rs::budget_report_exports_planned_actual_and_difference`; `pdf_regression.rs::budget_pdf_matches_its_golden`; `budget.test.tsx` (3); `api/schedule.test.ts` (text without a number is refused) |
| **Sides, six production reports, moodboard, storyboard sheet and shot list exports** | Finished, end-to-end run pending | Existing exports in `modules/exports/` (sides, reports, visual), now branded and free of em dashes in app-made labels; the sides cover says "Needs input" for an unknown actor or call | `exports.rs::sides_pdf_contains_only_the_days_scenes_in_screenplay_format`, `::reports_export_live_and_saved_in_every_format`, `::moodboard_pdf_embeds_images_and_excludes_internal_notes_unless_selected`, `::storyboard_sheet_has_panels_shot_numbers_framing_and_optional_notes`, `::shot_list_exports_by_scene_selection_or_whole_project`, `::every_production_export_is_branded_and_user_files_are_never_modified` |
| **Subtle OpenFrame branding** on every non-screenplay export; spreadsheets use metadata or a footer; user files are never modified (test per export) | Finished | `exports/mod.rs::deliver` brands in one place: PDF mark and line on every page (Sides: the line in each page's bottom margin), XLSX document properties and first-sheet footer (`report.rs::tables_to_xlsx_branded`), TXT last line. CSV stays pure data (no added row). Screenplay exports never pass through `deliver` | `exports.rs::every_production_export_is_branded_and_user_files_are_never_modified` (18 export and format cases, the brand line once per PDF page, workbook properties and footer, CSV without a brand row, an unbranded screenplay PDF, the picked image and the managed assets byte-for-byte unchanged); `report.rs::branded_pdfs_and_workbooks_name_openframe_and_plain_ones_do_not`; `samples.rs` (sample screenplay PDFs unbranded) |
| **PDF regression tests** for call sheet, schedule, Day Out of Days, breakdown, budget and sides | Finished | `tests/pdf_regression.rs` with goldens in `tests/fixtures/pdf-regression/`: one fixed project, two exports must be byte-identical, then every page's paper size and every text run with its position and size is compared with the golden (clock dates masked) | `pdf_regression.rs` (7 tests). Note: these goldens compare text layout, not rendered pixels; the raster image comparison of `openframe-import-export/tests/pdf_golden.rs` covers screenplay PDFs only |
| **Built-in templates** per the owner's answer; using a template makes an independent copy and later template edits never change existing documents (test) | Finished, end-to-end run pending | Eight built-ins in `modules/notes/templates.rs`; `templates.apply` runs the owning module's operations, all or nothing, as one undo step; plain lines or objects; "Create from template" in `settings/Templates.tsx` | `notes.rs::the_eight_built_in_templates_follow_the_owner_decision`, `::using_a_template_makes_independent_documents_in_one_undo_step`, `::a_template_use_that_fails_leaves_nothing_behind`, `::fsd_113_using_a_template_copies_it_independently` |
| **Sample scripts** (Example Feature, Example Short, Example Scene, FDX Interchange Test) shipped as assets and used as regression fixtures; copyright note per file | Finished, end-to-end run pending | `openframe-import-export/src/samples.rs` compiles the four files into the app; `screenplay.samples`; `screenplay.import_preview` and `import_apply` take a `sample` id; the import dialog lists the samples | `openframe-import-export/tests/samples.rs` (6: embedded bytes match the files and no stray file, copyright note and no em or en dash in every file, scene counts match the README, the feature keeps characters, more than 20 sets, Telugu and Hindi and four times of day, the FDX keeps 12A, 12B and three Indian scripts, every sample round-trips through Fountain and FDX and prints an unbranded PDF); `entitlement.rs` (samples behind the store lock) |
| **End-to-end test** for call sheets, sides, reports, visual planning, notes and tasks | Not finished: written, not run (the lead runs end-to-end tests serially) | `tests/e2e/specs/62-call-sheets-exports.test.mjs` | Covers opening the Example Short sample, two planned days, a call sheet with the six default sections, Needs input and Add Section, a schedule change with the update review, finalize and a new revision, a budget line with Planned and Actual and undo, a call sheet from the Indie Call Sheet template, sides, a scene report, schedule, Day Out of Days, breakdown, shot list, moodboard and storyboard exports with the brand line, an unbranded screenplay PDF, a note and a task, quit and relaunch |

## 2. Every new or changed operation

| Operation | Permissions | Store lock | Undo | Search | AI decision | Keyboard |
|---|---|---|---|---|---|---|
| `callsheets.add_section` | Owner, Editor; Viewer and Commenter refused (`add_section_is_searchable_undoable_and_needs_edit_rights`) | refused (`entitlement.rs` lock list) | one step (same test) | added sections are indexed (same test) | proposal tool `propose_call_sheet_section` | "Add Section" menu (Radix menu: Enter, arrows, Escape) |
| `callsheets.update`, `refresh`, `finalize`, `new_revision`, `delete` (document version 2) | unchanged rules (`schedule.rs` permission tests) | refused (`refresh_preview` in the lock list; the gate is on the one dispatch path) | unchanged | `index_call_sheet` covers every new field | existing tools; `propose_update_call_sheet` edits the new header and day time fields | native inputs and buttons |
| `callsheets.set_ready`, `callsheets.issue` | removed | | | | removed from the catalog | buttons removed |
| `budget.add_line`, `budget.update_line` (`planned`, `actual`) | Owner, Editor | refused (lock list) | unchanged | unchanged | existing proposal tools take `planned` and `actual` | Add Line dialog fields |
| `templates.apply` | Edit capability; Viewer and Commenter refused (`using_a_template_makes_independent_documents_in_one_undo_step`) | refused (lock list) | one grouped step (same test) | records are indexed by their own modules | hidden: `DUPLICATE` (the assistant proposes the same records through the owning modules) | "Create from template" menu and dialog |
| `screenplay.samples` | Import capability | refused (lock list) | read only | not content | hidden: `UI_FLOW` | sample buttons in the import dialog |
| `screenplay.import_preview` / `import_apply` with `sample` | Import capability | refused (lock list) | one step (existing import) | existing import | existing decision (user path) | sample buttons |

`docs/engineering/ai-tool-coverage.md` was regenerated by `ai_tool_coverage.rs`. Package coverage: `budget_line` and
`call_sheet` are whole tables already covered in `packages/coverage.rs` (Kept in full project packages); no new table or
column needs its own rule. Area 16 owns "Packages carry the new data".

## 3. Fixes made in this pass

- **Merge.** `session-2` merged; the four placeholder migrations 0012 to 0015 removed; `0016_callsheets_budget.sql`
  follows `0015_schedule_timeline.sql` and is recorded in `migrations/frozen.json`. The interim `callsheet_times.rs`
  (times guessed from marker clock times only) is deleted: the call sheet reads every time from the schedule timeline.
  Report branding is merged with reproducible workbooks (`tables_to_xlsx_branded` takes the creation date).
- **No false "Needs refresh" after an upgrade.** Without the source version, every draft made before 0016 would have
  read Needs refresh. Stored sources without a version are now compared in their old shape.
- **Refresh lost typed values.** A refresh replaced an address or actor typed on the sheet with the record's value. The
  merge now keeps a typed value and follows the record only where the sheet still holds the record's value.
- **Template use was not atomic.** `templates.apply` ran several operations with no rollback. It now undoes what it
  made when one step fails, and groups a success into one undo step. Templates created in the app (plain lines) now
  create documents; before, only the built-in object form did.
- **Empty optional extras call** printed blank; it now reads "Needs input".
- **Money parsing.** Text with no number in it ("abc") was read as an empty amount; it is now refused.
- **Writing standard.** App-made labels in exports, the sides cover and the touched screens no longer use em dashes;
  the writing baseline was lowered with `--update`.
- **Duplicate helper removed.** `board::call_sheet_status_word` duplicated `effective_status`; the schedule export now
  prints the effective word directly.

## 4. Commands run

Rust commands ran through `node C:/OpenFrame/scripts/cargo-queued.mjs` with
`CARGO_TARGET_DIR=C:/OpenFrame/.build-cache/target-area15` and `CARGO_INCREMENTAL=0` (sccache and rust-lld from
`.cargo/config.toml`).

While building, only the changed targets ran. The full gates ran once at the end:

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | OK |
| `cargo clippy --workspace --all-targets -- -D warnings` (through the queue) | OK, no warnings |
| `npm --workspace apps/desktop run lint` (ESLint, max 0 warnings) | OK |
| `cargo test --workspace --no-fail-fast` | 95 test binaries: 2,011 passed, 6 failed, 9 ignored. The 6 failures were the migration downgrade harnesses in `forced_kill.rs` and `migration_failures.rs`, which pin the latest schema (15) and had no step back from 16. Fixed (drop `budget_line.actual` and record version 16) and the two targets rerun: `forced_kill` 5 passed, 1 ignored; `migration_failures` 5 passed. Net: 2,017 passed, 0 failed, 9 ignored |
| `npm run typecheck` | OK |
| `npm run test:web` | 53 files, 530 tests passed |
| `npm run check:governance` | OK: architecture lint (189 allow-listed sites), migrations (project 16, global 4, app 2), feature status (171 rows), reachability (36 routes), writing standard (baseline lowered to 562 hits with `--update`), links (102 documents), plan status, 64 governance tests |
| `cargo deny check` | advisories, bans, licenses and sources OK |
| End-to-end (`62-call-sheets-exports.test.mjs`) | Not run (lead runs end-to-end tests serially); `node --check` passes |

PDF goldens were generated with `OPENFRAME_UPDATE_GOLDEN=1` and reviewed line by line before committing (call
sheet, schedule, Day Out of Days, breakdown, budget, sides).

## 5. Proposed feature-status rows (the lead moves them after the end-to-end run)

| Row (key) | Proposed status | Evidence to record |
|---|---|---|
| Create from a shooting day, snapshot, editor, optional sections, stale and refresh preview, finalize, issue (CALL-001) | Reachable, Verified after the end-to-end run; rename "issue" out of the row (Draft, Needs refresh, Final) | `schedule.rs` call sheet tests; `CallSheetEditor.test.tsx`; `62-call-sheets-exports.test.mjs` |
| Production header, day info (CALL-002) | Reachable | `fsd_call_001_*`; `pdf_regression.rs::call_sheet_pdf_matches_its_golden` |
| Locations block with "Needs input" (CALL-003) | Reachable | `CallSheetEditor.test.tsx`; call sheet golden ("Nearest hospital Needs input") |
| Scene schedule table with start time, travel and move rows (CALL-004) | Reachable | call sheet golden (19:00, MEAL, TRAVEL rows); `fsd_call_001_*` |
| Talent, extras, department notes, advance schedule, crew by department, walkie channels, practical notes, attachments (CALL-005) | Reachable | `add_section_is_searchable_undoable_and_needs_edit_rights`; call sheet golden |
| Branded, stable, printable PDF (CALL-006) | Reachable | `pdf_regression.rs`; `every_production_export_is_branded_and_user_files_are_never_modified` |
| Sides for a day, production reports (CALL-007) | Reachable (branding done) | `exports.rs` sides and reports tests; sides golden |
| Lightweight budget with Planned and Actual (BUDGET-001) | Reachable | `budget.test.tsx`; `budget_report_exports_planned_actual_and_difference`; v15 to v16 fixture |
| Template browser (TEMP-001) | Reachable | `notes.rs` template tests |
| Release inventory of 8 built-in templates (TEMP-002) | Reachable | `the_eight_built_in_templates_follow_the_owner_decision` |
| Sample scripts used as fixtures (TEMP-003) | Reachable (import dialog) | `openframe-import-export/tests/samples.rs` |
| Production exports carry subtle branding (BRAND-001) | Built (no screen of its own; route `none: backend`) | `every_production_export_is_branded_and_user_files_are_never_modified` |
| Moodboard, storyboard, shot list, schedule export rows (VIS-001, VIS-002, VIS-003, SCHED-008, STORY-007) | unchanged status; "Branding on export" no longer missing | same test |

## 6. Owner questions and limits

- No box here needs an owner decision. Nothing touches price, refunds or listings.
- **CSV exports carry no brand line.** A row added to a CSV would break imports into other tools; spreadsheets carry
  the brand in the XLSX properties and footer. If the owner wants a line in CSV too, say so.
- **Sides are branded** (one line in each page's bottom margin, never in the script text). Spec §43 lists only
  screenplay PDF, FDX, Fountain and DOCX as clean; sides are read as a production deliverable.
- **Package and backup manifests** with the brand belong to the packages area (16).
- **Manual source.** `docs/manual` holds only the .docx and .pdf; there is no manual source to update yet (masterplan
  area 17 box "User manual rebuilt"). The new screens follow the manual's chapter on call sheets, sides, reports,
  budget and notes (figures 141 to 153).
- **Screenplay templates** still refuse a project that already has a screenplay; the "several screenplays per project"
  branch (area 11) changes that rule when it joins.
- `graphify update .` was not run in this worktree (no `graphify-out` here); the lead runs it after the merge.
