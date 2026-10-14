# 01: Feature ledger

> Created 2026-10-05. The single list of **product features** and their completion state. Launch, operations,
> legal and commerce requirements live in [../release/launch-readiness-register.md](../release/launch-readiness-register.md)
> and use their own IDs. Milestones are in [00-masterplan.md](00-masterplan.md). Figure numbers refer to the User
> Manual and to [../audits/2026-10-03-manual-vs-repository-audit.md](../audits/2026-10-03-manual-vs-repository-audit.md).

## How to use

State scale (from the supplied completion plan):

| State | Meaning |
|---|---|
| S0 | Specified. Source known, nothing built or only partly. |
| S1 | Contracted. Data, operation, route and UI contract written. |
| S2 | Implemented. Code exists and passes its own tests. |
| S3 | Reachable. A normal user can find and use it in the product. |
| S4 | Verified. Persistence, undo, recovery, permissions, export/re-import, restart, failure paths and E2E proven. |
| S5 | Release-ready. Performance, security, accessibility, installer, migration and docs gates green. |

Rules:

1. A row moves up only with evidence (test name, E2E spec, artifact path) in the Evidence column of its milestone PR.
2. A row may not reach S5 with an empty required cell (UI route, AI decision, search decision, tests, export).
3. Every new row has a stable ID. Wording may change; IDs may not. Superseded rows stay with a pointer.
4. "Current" states what was verified against the repository, tagged **[V date]** or with a file reference. The
   unverified tags [R] (README claim), [A] (audit claim) and [S] were replaced on 2026-10-05 (masterplan M0.19) with
   file and test evidence or a corrected state; `scripts/check-ledger.mjs` refuses them from now on.
5. Columns **UI route** (where a normal user reaches the feature; `none: <reason>` or `pending`) and **Tests** (test
   references, or `pending`) are required. `scripts/check-ledger.mjs` (CI job `governance`, masterplan M0.22) fails
   when an in-scope row lacks source, milestone, UI route, tests or state, when an S2+ row has no tests, or when an
   S3+ row has no route. Each section names its specification sources on a `Source:` line.
6. Test reference shorthand: `file.rs::name` is an integration test in `crates/*/tests/`; `(unit)` is a test inside
   the named source file; `*.test.ts(x)` is a Vitest file under `apps/desktop/src`; `J:` and `R:` are titles in
   `tests/e2e/specs/01-journey.test.mjs` and `02-restart.test.mjs`. Baseline run: `docs/release/evidence/M0/INDEX.md`.

Priorities: P0 must ship in Store v1 (scope D2 = full). P1 should ship. P2 may defer with owner sign-off.

## CORE: durable foundation (figures 1 to 23, 154 to 157, 172, 173)

Source: FSD §§3, 4, 6, 40, 41, 43, 44, 46, 51, 52, 62; UX/UI shell and Home; Security spec (roles, logs); ADR-0003, ADR-0005, ADR-0009, ADR-0021; manual figures 1 to 23, 154 to 157, 172, 173.

| ID | Requirement | Pri | Current | Gap | MS | UI route | Tests | State |
|---|---|---|---|---|---|---|---|---|
| CORE-001 | Project create, open, close, duplicate, archive, delete to Recycle Bin, recents, locate moved | P0 | Implemented (`modules/project.rs`, `app/home/AppHome.tsx`); create, close, reopen E2E-proven [V 2026-10-05] | No test for `project.delete` and `project.locate`; packaged-app E2E on clean machine | M10 | app-home (card menu); project menu Close | core_pipeline.rs::duplicate_creates_independent_identity, ::fsd_4_5_archive_of_closed_project_is_stored_in_the_project; J: close project; R: reopen | S3 |
| CORE-002 | Single-writer lock; UNC/network projects refused | P0 | Implemented (`network_location_error`, `ProjectLock`) [V 2026-10-05] | No E2E; failure injection on Windows; mapped drive letters and links to shares not refused (security baseline E3) | M10 | app-home (refused on open) | core_pipeline.rs::project_is_single_writer, ::closed_project_edits_respect_the_single_writer_lock; security_paths.rs::network_paths_follow_the_policy_including_slash_variants; openframe-project-format (unit) lock_is_exclusive | S3 |
| CORE-003 | Continuous autosave, Save checkpoint, save-state indicator | P0 | Implemented (`app/shell/SaveStatus.tsx`); autosave plus restart E2E-proven [V 2026-10-05] | Forced-kill matrix (save, migration, export, apply); save-error states untested | M1 | shell save status, Ctrl+S | screenplay.rs::fsd_script_021_autosave_persisted_on_restart_and_crash; core_pipeline.rs::crash_offers_recovery_and_keeps_latest_autosave; J: type in the screenplay editor (autosaved); R: reopen | S3 |
| CORE-004 | Crash detection and recovery offer | P0 | Implemented (`app/shell/RecoveryDialogs.tsx`) [V 2026-10-05] | Crash E2E; dialog has no web test; soak and forced-kill in CI; **defect 2026-10-05**: the dialog calls the recovery state "more recent" while showing an older time than the last save (evidence M0 INDEX row M0-28) | M1 | shell recovery dialog | core_pipeline.rs::crash_offers_recovery_and_keeps_latest_autosave, ::fsd_44_recovery_state_reports_checkpoint_and_offer; openframe-project-format (unit) session_marker_detects_unclean_close | S3 |
| CORE-005 | Migrations: backup first, one transaction, refuse newer | P0 | Implemented; safety backup before upgrade (`application/src/store.rs`); no test asserts the backup [V 2026-10-05] | Fixture corpus per schema version, interrupted, disk-full | M1 | none: backend | openframe-persistence migrate.rs (unit) failed_migration_rolls_back_completely, too_new_database_is_refused, upgrade_only_runs_pending; core_pipeline.rs::corrupt_database_is_refused_without_modification | S2 |
| CORE-006 | Generic undo/redo, refuses to overwrite later work | P0 | Implemented; integration and E2E (Story only) [V 2026-10-05] | Property tests across every module | M1 | shell Undo/Redo, Ctrl+Z/Y, toast Undo | openframe-persistence undo.rs (unit) undo_refuses_to_overwrite_later_changes; core_pipeline.rs::undo_never_overwrites_later_work_by_someone_else; e2e_flows.rs::e2e_undo_redo_across_modules; J: Story undo/redo | S4 |
| CORE-007 | Recently Deleted, Activity | P0 | Implemented (`workspaces/trash`, `workspaces/activity`) [V 2026-10-05] | Purge has no E2E; retention window decision (ADR-0012 open) | M0 | trash; activity (project menu) | RecentlyDeleted.test.tsx; core_pipeline.rs::activity_entries_offer_open_only_for_live_objects; e2e_flows.rs::e2e_recently_deleted_across_modules; J: Recently Deleted delete and restore | S3 |
| CORE-008 | Project Files cabinet (managed, external, relink) | P0 | Implemented (`workspaces/files`) [V 2026-10-05] | E2E only opens the page | M10 | files | core_pipeline.rs::file_add_search_undo_redo_and_trash, ::fsd_40_files_export_copy_and_relink; security_review.rs::path_02_file_copies_never_land_in_openframe_folders | S3 |
| CORE-009 | Global search (FTS5), private notes owner-only | P0 | Engine implemented (`app/shell/SearchOverlay.tsx`); coverage grows per module [V 2026-10-05] | Every new entity registers a search doc | all | search-overlay (Ctrl+K) | J: Search (Ctrl+K) finds the scene and navigates to it; R: reopen; comments.rs::fsd_script_033_private_notes_are_invisible_to_other_users_and_their_search; e2e_flows.rs::e2e_search_hits_navigate_to_the_workspace_that_shows_them | S3 |
| CORE-010 | App shell, Home, navigation, command overlay, toasts with Undo | P0 | Implemented; the Ctrl+K overlay searches content only, there is no command palette [V 2026-10-05] | none known | M10 | shell; app-home | AppHome.test.tsx; menu.test.tsx; J: first launch to Project Home; J: every workspace and production tab renders | S3 |
| CORE-011 | Project Home shows a direct production-planning route and next day / call sheet summary | P1 | Partial: `workspaces/project-home/index.tsx`; no next-day or call-sheet summary [V 2026-10-05] | Add route and summary | M10 | home (Quick Access, Production status card) | core_pipeline.rs::fsd_6_3_home_stays_empty_until_real_content_exists | S2 |
| CORE-012 | Roles and capability matrix enforced in Rust (Owner, Editor, Commenter, Viewer, Export-only) | P0 | Enforced in Rust (`openframe-domain/src/auth.rs`, ADR-0009); no role names in the UI [V 2026-10-05] | Role tests for every new operation; user-facing role path | all | none: backend | openframe-domain auth.rs (unit) viewer_cannot_mutate, export_only_can_export_but_not_edit; core_pipeline.rs::permissions_are_enforced_before_any_mutation; story.rs::viewers_and_commenters_cannot_change_the_story | S2 |
| CORE-013 | Offline operation; only downloads show "unavailable offline" | P0 | Implemented (`app/shell/Shell.tsx`); the quoted "unavailable offline" string does not exist in code [V 2026-10-05] | Network-disabled E2E (gate G12) | M10 | shell (status bar "Offline · Local Project Saved") | J: AI panel says Offline AI is not installed; AiPanel.test.tsx | S3 |
| CORE-014 | Local logs, redacted, 14 days | P0 | Daily files, 14 kept (`src-tauri/src/lib.rs` `init_logging`); only panic and startup lines pass through `redact()`; logs not reachable from the UI [V 2026-10-05] | Redaction of ordinary log lines; redaction test as a gate (M1.18) | M1 | none: backend | security_paths.rs::panic_log_lines_are_redacted_and_bounded; openframe-security (unit) redaction_removes_personal_data | S2 |
| CORE-015 | **Export diagnostics** bundle (user-initiated, inspectable, redacted) | P0 | Missing; only AI diagnostics exist (`workspaces/settings/OfflineAi.tsx`). ADR-0021 Accepted 2026-10-05 | Build and test (ADR-0021) | M1 | pending | pending | S1 |
| CORE-016 | Windows High Contrast usable; no colour-only meaning | P0 | QA check only (ADR-0012 §7) | Manual pass, automated contrast checks | M10 | pending | pending | S1 |
| CORE-017 | Keyboard-only walkthrough of every workspace, visible focus, labelled icon buttons | P0 | Planned (doc 22 G10) | Accessibility pass and report | M10 | pending | pending (J: unlabelled-controls check is partial) | S0 |

## VAULT: Idea Vault (figures 24 to 47)

Source: FSD §5; UX/UI Idea Vault; manual figures 24 to 47.

| ID | Requirement | Pri | Current | Gap | MS | UI route | Tests | State |
|---|---|---|---|---|---|---|---|---|
| VAULT-001 | Project vault: views (grid, card, list, folder), empty state | P0 | Implemented (`workspaces/idea-vault`); 24 tests in `idea_vault.rs` [V 2026-10-05] | Reachability E2E, restart/undo/export proof | M10 | vault (Grid, Card, List, Folder) | idea_vault.rs::fsd_idea_005_views_present_the_same_items; IdeaVault.test.tsx; J: Idea Vault Quick Capture | S3 |
| VAULT-002 | Add note, image, URL, file, voice note, video; autosaved text editor | P0 | Implemented (`idea-vault/index.tsx`, `dialogs.tsx`) [V 2026-10-05] | Voice recording on clean machine, antivirus/permission cases | M10 | vault Add menu, Record Note | idea_vault.rs::fsd_idea_002_every_item_type_can_be_added, ::fsd_idea_011_note_autosave_coalesces_and_persists, ::fsd_idea_013_voice_note_saved_and_playable_after_reopen | S3 |
| VAULT-003 | Drag-in files, partial success, external file unavailable | P0 | Implemented (`onDragDropEvent`, Unavailable badge) [V 2026-10-05] | Failure-injection cases | M10 | vault drag-drop | idea_vault.rs::partial_add_keeps_successful_files, ::fsd_idea_020_external_files_are_distinguishable_and_relinkable; perf.rs::vault_list_reports_missing_image_files | S3 |
| VAULT-004 | Selection, context menu, collections, research flags, Recently Deleted | P0 | Selection, collections, Recently Deleted implemented; **research flags not implemented** [V 2026-10-05] | Research flags; E2E | M10 | vault context menu; vault/deleted | idea_vault.rs::fsd_idea_019_multi_select_batch_organisation, ::fsd_idea_007_collection_membership_never_duplicates_or_deletes, ::fsd_idea_018_delete_is_recoverable_and_purge_is_deliberate | S2 |
| VAULT-005 | Global Idea Vault and copy to project | P0 | Implemented [V 2026-10-05] | E2E, vault migration fixtures | M10 | app-home Global Idea Vault; Copy to Project / Global | idea_vault.rs::global_vault_works_with_no_project_open, ::fsd_idea_003_014_global_copy_edited_leaves_global_unchanged; AppHome.test.tsx; IdeaVault.test.tsx | S3 |
| VAULT-006 | Search the vault, Quick Capture | P0 | Implemented [V 2026-10-05] | E2E | M10 | vault search; Quick Capture Ctrl+Shift+N | idea_vault.rs::fsd_idea_009_search_matches_title_text_caption_tags_filenames; AppHome.test.tsx; J: Idea Vault Quick Capture | S3 |
| VAULT-007 | Send to Story Board | P0 | Implemented [V 2026-10-05] | Lineage test with Story | M4 | vault item menu Send to Story | idea_vault.rs::fsd_idea_015_016_017_send_to_story_copies_and_keeps_original, ::global_item_can_be_sent_to_story_of_open_project | S3 |
| VAULT-008 | AI read and proposal tools, retrieval projection | P1 | Covered by generated coverage [V: `ai-tool-coverage.md`] | Eval cases | M9 | ai-panel | ai_tool_coverage.rs::tool_coverage_matches_operation_metadata; ai_toolbox.rs::every_proposal_tool_builds_operations_the_registry_accepts; ai_retrieval.rs::spec_13_every_domain_gets_documents_chunks_vectors_and_graph | S2 |

## STORY: Story Board (figures 48 to 74)

Source: FSD §§7 to 14, 18, 25; Revision 1 §§21, 52; manual figures 48 to 74.

| ID | Requirement | Pri | Current | Gap | MS | UI route | Tests | State |
|---|---|---|---|---|---|---|---|---|
| STORY-001 | Acts, sequences, beats, scene cards; board and outline views; zoom out | P0 | Implemented (`workspaces/story`); 33 tests in `story.rs` [V 2026-10-05] | E2E | M4 | story; story/outline | story.rs::fsd_story_001_acts_contain_sequences_and_scenes; BoardView.test.tsx; J: Story act, sequence, cards, undo/redo; R: reopen | S3 |
| STORY-002 | Card drawer, drag with insertion line, multi-select, parking lot, duplicate, convert beat | P0 | Implemented (`drawers.tsx`, `dnd.tsx`) [V 2026-10-05] | E2E, undo property tests | M4 | story drawer, drag, Parking Lot | story.rs::fsd_story_004_015_drag_changes_order_and_undo_restores_it, ::fsd_story_011_multi_select_move_preserves_relative_order, ::fsd_story_029_convert_beat_keeps_original_as_converted; J: Story | S3 |
| STORY-003 | Delete rules (non-empty act, card with linked scene) | P0 | Implemented (`story/dialogs.tsx`) [V 2026-10-05] | Reference-behaviour tests | M4 | story delete dialogs | story.rs::act_delete_choices_recommend_moving_children, ::deleting_linked_card_keeps_the_screenplay_scene, ::fsd_story_016_delete_is_recoverable_and_undoable; J: Recently Deleted | S3 |
| STORY-004 | Build Screenplay (preview, existing screenplay, reorder warning) | P0 | Implemented (`BuildDialog.tsx`) [V 2026-10-05] | Product decision: block on missing headings (PRD C-1) | M4 | story Build Screenplay | story.rs::fsd_story_020_021_022_build_preview_order_selection_and_missing_headings, ::fsd_story_023_repeat_build_creates_a_new_draft_never_overwrites, ::fsd_story_025_applying_board_order_to_a_written_script_requires_confirmation; J: Build Screenplay | S3 |
| STORY-005 | Characters directory, relationships, timeline, sequence hub | P0 | Implemented (`Characters.tsx`, `Timeline.tsx`) [V 2026-10-05] | E2E | M4 | story/characters; story/timeline; sequence hub | story.rs::characters_create_edit_relationships_links_and_scene_usage, ::story_timeline_groups_by_day_and_never_reorders, ::character_image_is_a_managed_copy | S3 |
| STORY-006 | Series, seasons, episodes, season board | P0 | Implemented (`Episodes.tsx`) [V 2026-10-05] | Episodic vs series types (ADR-0012 §3) | M4 | story/episodes (episodic projects) | story.rs::episodes_scope_story_content_and_season_board_reorders; e2e_flows.rs::e2e_series_scene_links_carry_their_episode; NewProjectDialog.test.tsx | S3 |
| STORY-007 | Story Board / Outline PDF export | P0 | Implemented (`features/export`) [V 2026-10-05] | Golden PDF, branding rule | M7 | story toolbar Export | exports.rs::story_board_outline_exports_in_board_order_with_optional_notes, ::story_board_selected_items_scope; export.test.tsx | S3 |
| STORY-CARD-001 | **Long scene description** (canonical field) | P1 | Only `short_description` and `notes` exist [V: `0003_story.sql`]; `fsd_story_006` stores a long short description and is not evidence for this row | Additive migration, UI, search, AI | M4 | pending | pending | S0 |
| STORY-CARD-002 | Card Display Settings (12 toggles) stored as view config, never as content | P1 | Missing [V] | App/project view-config store, UI | M4 | pending | pending | S0 |
| STORY-CARD-003 | Presets Minimal, Story, Production, Custom | P1 | Missing | With STORY-CARD-002 | M4 | pending | pending | S0 |
| STORY-CARD-004 | Notes and attachments capacity defined and tested | P1 | `story_attachment` table exists [V] | Capacity limits, rendering tests | M4 | story drawer Attachments | story.rs::attachments_on_cards_are_copied_by_duplicate_and_survive_undo | S1 |

## SCRIPT: Screenplay engine (figures 75 to 101 and revised spec §§4 to 20, 45)

Source: FSD §§15 to 17, 21 to 24; Revision 1 §§4 to 20, 45, 48, 51; ADR-0008, ADR-0010, ADR-0011 (ADR-0015, ADR-0016 proposed); manual figures 75 to 101.

| ID | Requirement | Pri | Current | Gap | MS | UI route | Tests | State |
|---|---|---|---|---|---|---|---|---|
| SCRIPT-001 | Editor, eight element types, auto progression, manual override | P0 | Implemented (`editor/rules.ts`, `ScreenplayEditor.tsx`) [V 2026-10-05] | Progression test matrix | M2 | screenplay editor | editor.test.ts (Enter progression); screenplay.rs::fsd_script_005_011_all_element_types_are_supported_and_validated, ::fsd_script_012_manual_element_switching_is_overridable; J: type in the screenplay editor | S3 |
| SCRIPT-002 | Structured model, stable element ids, not rich text | P0 | `screenplay_element` rows with id/type/text [V] | Add metadata columns per ADR-0015 and below | M2 | screenplay (model) | editor.test.ts (round trip); screenplay.rs::stale_reads_are_detectable_by_edit_sequence | S2 |
| SCRIPT-003 | Element rule profiles (margins, spacing, next element, page-break, continuation, mapping) | P0 | Partial in editor and `layout.rs` [V] | Single shared profile table used by editor, PDF, FDX | M2 | none: backend | layout.rs (unit) wrapping_respects_width_and_breaks; pdf.rs::pdf_layout_follows_screenplay_conventions | S1 |
| SCRIPT-004 | Find and replace (case, whole word, next/prev, undoable) and **scope: scene or whole script** | P0 | Find and replace exists (`misc.tsx` FindBar, `editor/find.ts`); scope missing [V 2026-10-05] | Add scope | M2 | screenplay Find bar Ctrl+F | screenplay edit.rs (unit) find_respects_case_and_whole_words; screenplay.rs::fsd_script_015_016_replace_all_is_one_undoable_change; editor.test.ts | S2 |
| SCRIPT-005 | Focus mode, Writing Room, scene and private notes, comments | P0 | Implemented (`ScreenplayView.tsx`, `panels.tsx`) [V 2026-10-05] | E2E | M2 | screenplay View menu (Writing Room, Focus), side panels | comments.rs::fsd_script_030_031_032_scene_comment_reply_resolve_keeps_history, ::fsd_script_033_private_notes_are_invisible_to_other_users_and_their_search; screenplay.rs::fsd_script_020_scene_notes_are_non_printing_scene_metadata, ::layout_is_remembered_per_user | S3 |
| SCRIPT-006 | Drafts, history/recovery, compare, review rounds, delete draft | P0 | Implemented (`drafts.tsx`, `CompareView.tsx`, `ReviewView.tsx`) [V 2026-10-05] | E2E | M2 | screenplay drafts panel; screenplay/compare; screenplay/review | screenplay.rs::fsd_script_023_024_named_drafts_preserve_prior_versions_with_lineage, ::fsd_script_026_automatic_history_is_separate_from_named_drafts, ::fsd_script_027_compare_at_scene_and_text_level, ::fsd_script_028_review_round_linked_to_draft_and_completing_never_locks | S3 |
| SCRIPT-007 | Lock Shooting Draft, Start Revision, revision label/colour/reason | P0 | Lock and revisions implemented (`lock.tsx`; columns on `screenplay_draft`) [V 2026-10-05] | Page-level baseline and changed pages (ADR-0015) | M2 | screenplay toolbar Lock / Revisions | screenplay.rs::fsd_script_034_035_locked_draft_rejects_edits, ::fsd_script_036_037_revision_leaves_locked_draft_unchanged; lock.security.test.ts | S3 |
| SCRIPT-008 | **Production scene numbers**: imported, alphanumeric, frozen after lock | P0 | Not stored by design (ADR-0011) [V] | ADR-0015 migration and operations | M2 | pending | pending | S0 |
| SCRIPT-009 | **Deterministic pagination** shared by preview, print and PDF | P0 | PDF layout in `layout.rs`; the editor does not paginate [V 2026-10-05] | Contract, parity tests, perf budget | M2 | screenplay Export Print Preview | pdf.rs::pdf_layout_follows_screenplay_conventions; interchange.rs::fsd_script_049_050_title_page_and_derived_scene_numbers_in_pdf | S1 |
| SCRIPT-010 | Keep-together rules (heading, cue, parenthetical, orphan) | P0 | Rules implemented in `layout.rs` (heading kept with 2 lines, action split, speech split) [V 2026-10-05] | Per-rule golden tests; editor parity | M2 | none: backend | layout.rs (unit) heading_is_never_orphaned, long_dialogue_splits_with_more_and_contd | S1 |
| SCRIPT-011 | Dialogue continuation (MORE / CONT'D) generated, not typed | P0 | Rendered in `layout.rs`; parsed in `heuristics.rs` [V] | Model distinction "one split block vs two" and editor display | M2 | PDF / Print Preview (generated) | layout.rs (unit) long_dialogue_splits_with_more_and_contd; pdf.rs::split_dialogue_is_rejoined_on_import | S1 |
| SCRIPT-012 | **Explicit page break directive** (insert, remove, undo, export) | P0 | Import/export layer has `ElementKind::PageBreak` (Fountain `===`, honoured by `layout.rs`), but `interchange/draft.rs` drops it on import; no project element type [V 2026-10-05] | New element kind or flag, FDX/Fountain mapping | M2 | pending | pending | S0 |
| SCRIPT-013 | Title page structured fields, preview, export toggle | P0 | Implemented (`misc.tsx` TitlePageDialog; `title_page_json`) [V 2026-10-05] | Field set per spec §13, preview, tests | M2 | screenplay Title Page dialog; export toggle | screenplay.rs::fsd_script_049_title_page_fields; interchange.rs::fsd_script_049_050_title_page_and_derived_scene_numbers_in_pdf | S3 |
| SCRIPT-014 | Header/footer with placeholders, first-page rule | P1 | Missing | Settings and renderer | M2 | pending | pending | S0 |
| SCRIPT-015 | Page setup presets (Letter, A4, saved preset, font, size, line spacing) | P0 | Paper size is an export setting (ADR-0008) | Persisted page setup, UI | M2 | screenplay Export (paper option only) | pending | S1 |
| SCRIPT-016 | **Custom elements** (name, shortcut, preset, next element, import/export identity) | P0 | `element_type` has a fixed CHECK of 8 values [V] | Table rebuild migration, UI behind Advanced | M2 | pending | pending | S0 |
| SCRIPT-017 | Scene navigator with optional columns (Page, Notes, Revision, Production) | P1 | Navigator exists (`SceneNavigator.tsx`); no column configuration [V 2026-10-05] | Column config | M3 | screenplay navigator | screenplay.rs::fsd_script_014_050_navigator_numbers_derive_from_order | S1 |
| SCRIPT-018 | Smart structural suggestions (INT./EXT., characters, locations, time terms) | P0 | Missing (only editor rules) [V] | Deterministic completion list, no model | M3 | pending | pending | S0 |
| SCRIPT-019 | **Custom keyboard shortcuts** with conflict detection and restore | P1 | Shortcuts hard-coded [V] | Settings, command registry | M3 | pending | pending | S0 |
| SCRIPT-020 | Spell check: project language, toggle, personal dictionary, primary + secondary language | P1 | Browser `spellcheck="true"` only [V] | Language and dictionary layer | M3 | screenplay editor | pending | S1 |
| SCRIPT-UNI-001 | **Multilingual text survives** type, save, reload, PDF, FDX, Fountain, DOCX, search, AI | P0 | PDF export fails outside Latin-1 (ADR-0008) [V] | ADR-0016 fonts and shaping, corpus | M2 | pending | pdf.rs::non_latin_text_is_a_clear_export_error_and_nothing_is_written (asserts the refusal) | S0 |
| SCRIPT-021 | Print preview identical to PDF (page count, breaks) | P0 | Source present (`PrintPreview.tsx`) [V] | Parity tests | M2 | screenplay Export Print Preview | interchange.rs::fsd_script_049_050_title_page_and_derived_scene_numbers_in_pdf | S2 |
| SCRIPT-022 | Scene Hub, scene context menu, moving a scene already in production | P0 | Implemented (`misc.tsx` SceneHubDrawer, ReorderWarningDialog) [V 2026-10-05] | Dependency warning tests | M2 | screenplay scene menu Scene Hub | screenplay.rs::scene_hub_reports_context_without_editing; e2e_flows.rs::e2e_script_revision_after_production_source_flags_everything_and_deletes_nothing; J: Schedule strip Open in Screenplay | S3 |
| SCRIPT-023 | Delete draft with strong confirmation | P0 | Implemented (`drafts.tsx`) [V 2026-10-05] | E2E | M2 | screenplay drafts Delete Draft | screenplay.rs::fsd_script_025_exactly_one_current_draft, ::fsd_script_034_035_locked_draft_rejects_edits | S3 |

## XFER: import, export, packages (figures 91 to 96, 158 to 163, 169 to 171)

Source: FSD §§19, 20, 45, 47, 50, 60; Import/Export spec; Offline and Collaboration spec (packages); Revision 1 §§36 to 42; ADR-0007; manual figures 91 to 96, 158 to 163, 169 to 171.

| ID | Requirement | Pri | Current | Gap | MS | UI route | Tests | State |
|---|---|---|---|---|---|---|---|---|
| XFER-001 | FDX import with preview, counts, warnings; never flatten unknown constructs | P0 | `fdx.rs` 607 lines; unknown paragraph types are imported as Action with an info warning, i.e. flattened [V 2026-10-05] | Stop flattening: preserve/map/review custom elements; revisions; numbers | M2 | shell Create Import Screenplay | fdx.rs::fdx_fixture_maps_paragraph_types, ::malformed_and_foreign_fdx_are_rejected_humanely; interchange.rs::fsd_script_038_fdx_import_creates_new_screenplay_with_new_identities; e2e_flows.rs::e2e_import_fountain_and_fdx_then_breakdown_and_ai_counts | S2 |
| XFER-002 | FDX export structurally valid, re-importable, opens in Final Draft | P0 | Implemented (`ExportScreenplayDialog.tsx`) [V 2026-10-05] | Round-trip suite; external-app reopen check | M2 | screenplay Export | fdx.rs::fdx_round_trip_preserves_structure; interchange.rs::fsd_script_045_to_048_every_format_exports_and_reimports, ::fdx_revision_marks_are_kept_on_import_and_exported | S3 |
| XFER-003 | FDX round-trip fixture suite (13 classes) with parsed, exported, re-imported and PDF snapshots | P0 | 7 fixture files, 2 of them FDX [V: `import-export/tests/fixtures/screenplay`] | Complete the 13 classes | M2 | none: backend | fdx.rs::fdx_fixture_maps_paragraph_types | S1 |
| XFER-004 | Fountain import/export with documented fidelity | P0 | `fountain.rs` 660 lines [V] | Fidelity table, round-trip | M2 | Import dialog; screenplay Export | fountain.rs::fountain_writer_matches_golden_file, ::fountain_round_trip_is_stable; interchange.rs::fsd_script_040_fountain_preview_shows_what_would_be_created_and_writes_nothing; J: import pasted Fountain | S3 |
| XFER-005 | DOCX import/export as compatibility format | P0 | `docx.rs` 882 lines [V] | Documented boundary, tests | M2 | Import dialog; screenplay Export | docx.rs::docx_round_trip_keeps_readable_screenplay, ::zip_bomb_is_rejected_without_inflating_it; interchange.rs::zip_bomb_docx_is_rejected_without_writing | S3 |
| XFER-006 | PDF import best effort with preview, detection, page estimate, safe failure | P0 | `pdf_import.rs` [V] | Parser corpus, limits | M2 | shell (Create, Import Screenplay) | pdf.rs::exported_pdf_parses_back_with_same_scene_count, ::scanned_or_broken_pdfs_are_rejected_with_the_standard_message; security_parsers.rs::pdf_decompression_bomb_is_rejected | S3 |
| XFER-007 | TXT and pasted text import | P0 | `text.rs` [V] | none known | M2 | shell (Create, Import Screenplay, paste box) | txt.rs::pasted_text_is_recognised_with_honest_warnings, ::txt_export_reimports; interchange.rs::fsd_script_043_pasted_text_uses_txt_heuristics_with_honest_warnings; J: import pasted Fountain | S3 |
| XFER-008 | Import report useful and exact; export failure leaves source untouched | P0 | Import report in `modules/interchange/import.rs` (not `import-export/src/report.rs`, which writes production reports) [V 2026-10-05] | Failure-injection tests | M2 | shell (Import Screenplay report); screenplay (Export error) | interchange.rs::failed_imports_change_nothing, ::export_destination_is_validated_and_failures_leave_no_file; pdf.rs::pdf_is_written_atomically; Interchange.test.tsx | S3 |
| XFER-009 | Screenplay outputs are clean (no OpenFrame branding) | P0 | PDF metadata and DOCX `dc:creator` say "OpenFrame Studio" (`pdfgen.rs`, `docx.rs`); product decision on metadata [V 2026-10-05] | Branding test | M7 | none: backend | pending | S1 |
| XFER-010 | PDF visual regression (page count, margins, headings, dialogue, title page, numbers, continuation, revision marks, headers) | P0 | No PDF golden-file or byte-identity test exists; only a Fountain golden file [V 2026-10-05] | Render-to-image compare harness | M2 | none: backend | pdf.rs::pdf_layout_follows_screenplay_conventions (geometry only) | S1 |
| XFER-011 | Exchange / Review / Response packages: export, open, response, version mismatch, unmatched notes, preview before apply | P0 | Implemented (`features/packages`); 18 tests in `packages.rs` [V 2026-10-05] | E2E, identity collision tests | M8 | project-menu (Share / Exchange) | packages.rs::fsd_col_001_exchange_package_can_be_inspected_in_another_installation, ::fsd_col_002_stale_response_is_imported_as_comments_only_without_changing_text, ::too_new_package_format_is_refused_clearly; packages.test.ts | S3 |
| XFER-012 | Backup, whole-project package, import with identity collision | P0 | Implemented (`BackupDialogs.tsx`); restore and Open Project Package are only in the project menu, not on Application Home [V 2026-10-05] | Restore entry on Application Home; restore drill; cross-machine test | M8 | project-menu (Backup & Portability) | packages.rs::fsd_44_backup_round_trip_restores_as_copy_with_new_identity, ::fsd_44_interrupted_backup_leaves_no_partial_file, ::fsd_45_project_package_round_trip_across_installations, ::fsd_45_4_identity_collision_open_as_copy_or_replace_after_backup | S3 |
| XFER-013 | Live LAN collaboration | removed | **Removed by ADR-0007**; code residue already gone [V] | Remove LAN text from specs and manual (DOC-003, DOC-007) | M0, M10 | none: removed | none | n/a |

## BRK: breakdown, catalog, locations, people (figures 102 to 120)

Source: FSD §§26 to 30, 53, 54; Revision 1 §§22 to 24, 53; manual figures 102 to 120.

| ID | Requirement | Pri | Current | Gap | MS | UI route | Tests | State |
|---|---|---|---|---|---|---|---|---|
| BRK-001 | Production source choice, breakdown view, tagging, suggestions, accept/reject, catalog reuse | P0 | Implemented (`workspaces/breakdown`); 23 tests in `production.rs` [V 2026-10-05] | E2E | M5 | breakdown | production.rs::fsd_brk_001_breakdown_requires_and_shows_the_selected_source, ::fsd_brk_003_suggestions_never_auto_confirm, ::fsd_cat_001_same_prop_across_two_scenes_reuses_the_catalog_item; J: Breakdown accept a suggestion | S3 |
| BRK-002 | Script change reconciliation, Needs Review, source update | P0 | Implemented (`breakdown/ReviewViews.tsx`) [V 2026-10-05] | Never silently delete confirmed data (test) | M5 | breakdown Needs Review | production.rs::fsd_brk_016_017_018_production_source_update_never_deletes_breakdown_data, ::fsd_brk_019_heading_change_and_in_place_edits_flag_review; e2e_flows.rs::e2e_script_revision_after_production_source_flags_everything_and_deletes_nothing | S3 |
| BRK-003 | Eleven categories with **colour + text + icon + tooltip**, customisable in Advanced | P1 | Category order and short labels only; no colour or icon per category [V 2026-10-05] | Colour model and accessibility test | M5 | breakdown | production.rs::fsd_breakdown_006_categories_follow_the_defined_order | S1 |
| BRK-004 | Efficiency: accept/reject selected, quick reuse, filters (unreviewed, suggested, missing), scene completion state | P1 | Partial: Accept Selected, Accept All Safe, status filters, Mark Complete exist; no Reject Selected, no "missing" filter [V 2026-10-05] | Bulk commands with batch undo | M5 | breakdown | breakdown.test.tsx (filters by derived status); production.rs::fsd_brk_011_batch_accept_asks_for_ambiguous_matches, ::fsd_108_overview_is_derived_and_breakdown_complete_is_manual | S1 |
| BRK-005 | Source-phrase jump, catalog item to all scenes, concise breakdown report | P1 | Partial: breakdown report export exists; catalog opens only the first scene; no source-phrase jump [V 2026-10-05] | Navigation, report | M5 | breakdown; production/catalog | exports.rs::breakdown_report_marks_suggested_and_lists_catalog_usage; catalog.test.tsx | S1 |
| BRK-006 | Large-project performance (filters, open) | P0 | Perf suite exists (doc 21) [V] | Budgets for breakdown filters | M5 | none: backend | perf.rs::perf_feature_120_budgets (ignored; `npm run perf`) | S1 |
| PROD-001 | Catalog: props, equipment, wardrobe, vehicles, effects, VFX, sound, animals; archive/replace | P0 | Implemented (`production/tabs/catalog.tsx`) [V 2026-10-05] | E2E | M5 | production/catalog | production.rs::catalog_crud_archive_delete_and_search, ::catalog_replace_in_selected_scenes_keeps_the_old_item; catalog.test.tsx; J: workspace tour | S3 |
| PROD-002 | Locations: address, access, parking, hospital, availability, notes, photos | P0 | Implemented; parking, access, power, facilities stored as notes keys in `notes_json`, no hospital or availability field [V 2026-10-05] | Field audit against call sheet needs | M5 | production/locations | production.rs::locations_fields_notes_photos_and_trash, ::fsd_prod_004_007_location_status_flow_and_scene_usage | S2 |
| PROD-003 | Cast and crew: performer, character, contact, availability, notes, archive | P0 | Implemented; availability is free text only [V 2026-10-05] | Availability data model | M5 | production/cast-crew | production.rs::fsd_cast_001_cast_connects_actors_to_characters_and_scenes, ::fsd_prod_008_010_cast_availability_is_recorded_as_notes, ::crew_directory_requires_role | S2 |
| PROD-004 | Replace a rejected location for selected scenes | P0 | Implemented (`ReplaceLocationDialog`) [V 2026-10-05] | E2E | M5 | production/locations Replace | production.rs::fsd_loc_001_location_replacement_never_changes_screenplay_text | S3 |

## VIS: visual planning (figures 121 to 127)

Source: FSD §§31 to 34, 55; manual figures 121 to 127.

| ID | Requirement | Pri | Current | Gap | MS | UI route | Tests | State |
|---|---|---|---|---|---|---|---|---|
| VIS-001 | Moodboard canvas and export | P0 | Implemented (`tabs/moodboards.tsx`) [V 2026-10-05] | Branding on export | M7 | production/moodboards | visual.rs::fsd_prod_011_012_moodboard_create_rename_and_add_move_items, ::moodboard_images_from_the_idea_vault_are_guarded_and_copied; exports.rs::moodboard_pdf_embeds_images_and_excludes_internal_notes_unless_selected; visual.test.tsx | S3 |
| VIS-002 | Storyboard panels, import/sketch/empty, sketch tool | P0 | Implemented (`tabs/storyboards.tsx`, `SketchDialog.tsx`) [V 2026-10-05] | E2E | M7 | production/storyboards | visual.rs::fsd_prod_014_015_storyboard_for_a_scene_with_panels, ::fsd_prod_016_panel_drag_reorder_recalculates_numbers_and_linked_shot_numbers; exports.rs::storyboard_sheet_has_panels_shot_numbers_framing_and_optional_notes | S3 |
| VIS-003 | Shot list and shot details | P0 | Implemented (`tabs/shots.tsx`) [V 2026-10-05] | E2E | M7 | production/shots | visual.rs::fsd_shot_001_shots_are_ordered_by_drag_and_tied_to_scenes, ::fsd_prod_018_020_shot_needs_only_a_description_and_fields_are_optional; exports.rs::shot_list_exports_by_scene_selection_or_whole_project; visual.test.tsx | S3 |

## SCHED: schedule and DOOD (figures 128 to 140)

Source: FSD §§35 to 37, 56; Revision 1 §§25 to 28, 54, 55; manual figures 128 to 140.

| ID | Requirement | Pri | Current | Gap | MS | UI route | Tests | State |
|---|---|---|---|---|---|---|---|---|
| SCHED-001 | Schedule creation, stripboard (Unscheduled pool and days), list, calendar, drag a scene into a day | P0 | Implemented (`tabs/schedule.tsx`, board, list, calendar); 24 tests in `schedule.rs` [V 2026-10-05] | E2E | M6 | production/schedule | schedule.rs::fsd_sch_001_all_scenes_enter_unscheduled, ::fsd_sch_002_drag_into_day_is_undoable, ::schedule_and_call_sheets_survive_restart; J: Schedule | S3 |
| SCHED-002 | Break markers: Meal, Travel, Company Move, Custom with time and duration | P0 | `schedule_marker` exists [V] | Extend, do not replace | M6 | production/schedule | schedule.rs::markers_days_duplicate_off_day_and_delete_restore; Strip.test.tsx; exports.rs::schedule_export_has_days_markers_summaries_and_unscheduled | S3 |
| SCHED-003 | Day timeline: crew call, shooting start, target wrap | P1 | `shooting_day` has date, notes, planned minutes only [V] | Additive columns | M6 | production/schedule | pending | S0 |
| SCHED-004 | Scene start time and duration on a strip | P1 | `schedule_strip` has `estimated_minutes` only [V] | Add start time, computed timeline | M6 | production/schedule | pending | S1 |
| SCHED-005 | Availability warnings (cast, location, missing location, over target, travel) | P1 | Warnings exist: actor_conflict, missing_location, duration_overflow, location_conflict, call_sheet_stale, script_removed; no cast availability or travel rules [V 2026-10-05] | Availability data and rules | M6 | production/schedule | schedule.rs::fsd_sch_003_warnings_are_advisory_with_keep_anyway, ::strict_validation_offers_keep_anyway | S1 |
| SCHED-006 | Filters (location, cast, D/N, length, scheduled, breakdown complete, day, revision) | P1 | Only an all / unscheduled / scheduled list filter [V 2026-10-05] | Filter model, no mutation | M6 | production/schedule (list) | pending | S0 |
| SCHED-007 | Stale and review state, grouping suggestion, script-change review | P0 | Implemented [V 2026-10-05] | E2E | M6 | production/schedule | schedule.rs::fsd_56_script_changes_flag_strips_and_never_delete, ::fsd_sched_014_grouping_suggestions_need_explicit_confirmation; e2e_flows.rs::e2e_script_revision_after_production_source_flags_everything_and_deletes_nothing | S3 |
| SCHED-008 | Schedule PDF and spreadsheet export | P0 | Implemented (`modules/exports/schedule.rs`) [V 2026-10-05] | Branding, reproducibility | M7 | production/schedule Export | exports.rs::schedule_export_has_days_markers_summaries_and_unscheduled, ::a_failed_write_leaves_the_previous_file_and_no_partial_file; export.test.tsx | S3 |
| SCHED-009 | No automatic optimizer ever moves scenes | P0 | Design rule with a test asserting no automatic move [V 2026-10-05] | Test asserting no auto-move | M6 | production/schedule | schedule.rs::fsd_sched_014_grouping_suggestions_need_explicit_confirmation (days unchanged before confirmation) | S2 |
| DOOD-001 | Derived Day Out of Days from cast and schedule, date range, manual correction, PDF and spreadsheet | P1 | Missing [V: no matches] | Derived report, manual override model | M6 | pending | pending | S0 |

## CALL: call sheets, sides, reports, budget, notes (figures 141 to 153)

Source: FSD §§38, 39, 57 to 59; Revision 1 §§29 to 35, 43, 56, 57; manual figures 141 to 153.

| ID | Requirement | Pri | Current | Gap | MS | UI route | Tests | State |
|---|---|---|---|---|---|---|---|---|
| CALL-001 | Create from a shooting day, snapshot, editor, optional sections, stale and refresh preview, finalize, issue | P0 | Implemented (`CallSheetEditor.tsx`); statuses Draft, Needs Refresh, Ready, Final, Issued, Superseded [V 2026-10-05] | E2E | M7 | callsheets | schedule.rs::fsd_call_001_call_sheet_generated_from_shooting_day, ::fsd_call_009_schedule_change_marks_call_sheet_needs_refresh, ::fsd_call_011_finalized_call_sheet_is_immutable_and_revised; J: Call sheet from the shooting day; R: reopen | S3 |
| CALL-002 | Production header, day info (general/crew/shooting/meal calls, wrap, weather, sunrise, sunset) | P0 | Fields verified 2026-10-05: dayLabel, date, crewCall, dayNotes, mealBreak, weather; no general or shooting call, wrap, sunrise or sunset | Field audit against spec §29 | M7 | callsheets | pending (field audit) | S1 |
| CALL-003 | Locations block (address, parking, hospital, emergency, access); unknown shows "Needs input" | P0 | Fields verified 2026-10-05: name, address, meetingPoint, parking, emergencyContact, "needs input" placeholder; no hospital or access fields | Fields and placeholder rule | M7 | callsheets | pending | S1 |
| CALL-004 | Scene schedule table with start time, travel and move rows | P0 | Depends on SCHED-004 | After M6 | M7 | callsheets | pending | S1 |
| CALL-005 | Talent, extras, department notes, advance schedule, crew by department, walkie channels, practical notes, attachments | P1 | Verified 2026-10-05: weather, specialNotes, attachments, referenceImages, extraFields; no walkie channels or crew by department | Sections and defaults (six default, rest via + Add Section) | M7 | callsheets | pending | S1 |
| CALL-006 | Branded, stable, printable PDF | P0 | `modules/exports/callsheet.rs`; no branding or visual regression [V 2026-10-05] | Visual regression, branding | M7 | callsheets Export | exports.rs::call_sheet_pdf_has_paper_layout_and_issued_sheet_uses_its_snapshot | S2 |
| CALL-007 | Sides for a day, production reports (6 types) | P0 | Implemented (`tabs/sides.tsx`, `tabs/reports.tsx`, 6 report types) [V 2026-10-05] | Branding, E2E | M7 | production/sides; production/reports | schedule.rs::fsd_prod_024_sides_are_script_snapshots, ::fsd_prod_023_reports_are_read_only_projections; exports.rs::sides_pdf_contains_only_the_days_scenes_in_screenplay_format, ::reports_export_live_and_saved_in_every_format | S3 |
| BUDGET-001 | Lightweight budget with **Planned and Actual**, totals and difference | P1 | Budget tab with snapshot-level planned total and contingency; lines have one `amount` [V 2026-10-05] | Migration (amount becomes planned), UI, report | M7 | production/budget | schedule.rs::fsd_prod_025_budget_is_advisory_and_validated, ::budget_snapshots_can_be_deleted_restored_and_purged; exports.rs::budget_summary_exports_totals_and_lines | S1 |
| NOTES-001 | Project notes and lightweight tasks; Activity history | P0 | Implemented (`workspaces/notes`, `workspaces/activity`) [V 2026-10-05] | E2E | M10 | notes; activity | notes.rs::fsd_106_note_create_edit_pin_and_search, ::fsd_158_task_requires_title_and_validates_owner_and_relation, ::notes_and_tasks_persist_across_restart; J: workspace tour | S3 |
| TEMP-001 | Template browser: scopes, use, save, rename, delete, independent copy | P1 | Engine and UI (`settings/Templates.tsx`), 7 built-ins [V 2026-10-05] | Decide inventory (D12) | M7 | settings/templates | notes.rs::fsd_59_builtin_templates_are_listed_and_read_only, ::fsd_59_global_templates_work_without_a_project_and_persist, ::fsd_113_using_a_template_copies_it_independently, ::fsd_52_project_template_delete_is_recoverable_and_undoable | S3 |
| TEMP-002 | Release inventory of 8 built-in templates incl. three screenplay variants | P1 | 7 exist; screenplay variants missing [V] | Author and test | M7 | settings/templates | notes.rs::fsd_59_builtin_templates_are_listed_and_read_only | S1 |
| TEMP-003 | Sample scripts (OpenFrame-authored) used as fixtures | P1 | Missing | D13, X13 | M7 | none: backend | pending | S0 |
| BRAND-001 | Production exports carry subtle branding; spreadsheets via metadata/footer; user files never modified | P1 | Not verified | Branding layer and tests | M7 | none: backend | pending | S0 |

## THEME: appearance (revised spec §44, §47A; ADR-0014)

Source: Revision 1 §44 and §47A; ADR-0012 §7; ADR-0014 (proposed); `docs/engineering/30-appearance-themes.md`.

| ID | Requirement | Pri | Current | Gap | MS | UI route | Tests | State |
|---|---|---|---|---|---|---|---|---|
| THEME-001 | Light, Dark, System modes; System follows OS live | P1 | Missing [V: no `data-theme`, no `prefers-color-scheme`] | Appearance engine | M3 | pending | pending | S0 |
| THEME-002 | OpenFrame Default equals today's UI; visual baseline captured first | P1 | Baseline of 15 screens captured 2026-10-05 before any tokenization (`docs/release/evidence/M0/M0.20-visual-baseline/`) | Compare harness (M1.17); Default theme equals baseline | M3 | pending | 90-visual-baseline.test.mjs (capture only) | S1 |
| THEME-003 | Tokenize all themeable colours; classify fixed and document colours | P1 | Central palette plus literals remain [V] | One audit pass | M3 | pending | pending | S0 |
| THEME-004 | 15 built-in themes from one registry; add a theme without editing components | P1 | Missing | Registry and seeds | M3 | pending | pending | S0 |
| THEME-005 | Import, validate, preview, save, rename, export, delete custom themes | P1 | Missing | Validator and UI | M3 | pending | pending | S0 |
| THEME-006 | Preferences in `app.sqlite`; invalid or deleted active theme falls back to Default | P1 | Missing | App migration 0002 | M3 | pending | pending | S0 |
| THEME-007 | Contrast checks (AA 4.5:1) light and dark; High Contrast theme | P1 | Missing | Automated contrast tests | M3 | pending | pending | S0 |
| THEME-008 | Theme never changes screenplay or call-sheet output | P1 | Design rule | Test | M3 | pending | pending | S0 |

## AI: Offline AI and assistant (figures 174 to 181)

Source: AI spec; agentic RAG spec; Local AI Runtime spec; FSD §42; ADR-0006, ADR-0013, ADR-0019 (ADR-0020 proposed); Microsoft Store policy for generative AI; manual figures 174 to 181.

| ID | Requirement | Pri | Current | Gap | MS | UI route | Tests | State |
|---|---|---|---|---|---|---|---|---|
| AI-001 | One-click Offline AI install: signed manifest, resumable, verified; Gemma 3 1B and BGE small via managed llama.cpp | P0 | Implemented (`crates/openframe-ai`, `ai/OfflineAiSetup.tsx`); E2E checks only the not-installed state; `real_runtime.rs` is ignored [V 2026-10-05] | Hardware and Vulkan validation | M9 | ai-panel; settings Offline AI | openframe-ai lifecycle.rs::one_click_install_is_verified_combined_healthy_and_usable_without_restart; install.rs::tampered_manifest_is_rejected; download.rs::interrupted_download_resumes_from_partial_with_a_range_request; OfflineAiSetup.test.tsx | S3 |
| AI-002 | Assistant: bounded agent (<= 12 steps), toolbox, hybrid retrieval, one Change Set per request, human-only apply | P0 | Implemented (`ai/AiPanel.tsx`, `HARD_MAX_STEPS = 12`); 423 ops, 298 exposed [V: `ai-tool-coverage.md`]; no real-model E2E | Retrieval quality unmeasured | M9 | ai-panel | ai_agent.rs::default_step_limit_is_eight_and_the_hard_limit_twelve, ::a_multi_step_mutation_ends_in_one_reviewable_change_set, ::no_tool_the_model_can_see_reviews_or_applies_change_sets; AiPanel.test.tsx | S3 |
| AI-003 | **Retrieval gold set and metrics** (Recall@K, MRR, nDCG, grounding, leakage) for FTS, vector, hybrid, hybrid+graph | P0 | Missing: no Recall@K, MRR or nDCG code [V 2026-10-05] | Corpus, harness, thresholds | M9 | none: backend | pending | S0 |
| AI-004 | **Minimum-spec and recommended hardware benchmark**; CPU path; Vulkan discrete GPU | P0 | Not measured; `scripts/ai-benchmark.mjs` exists, no results in evidence [V 2026-10-05] | Run `scripts/ai-benchmark.mjs` and `real_runtime` | M9 | none: platform | pending | S0 |
| AI-005 | **Crash-safe multi-operation Change Set apply** (ADR-0019) | P0 | Implemented 2026-10-05: apply journal (migration 0011), compensation on open, file-argument refusal [V] | Desktop E2E of a real-model apply interrupted by a process kill (AI-006); migration fixture from schema 10 (M1.04) | M1 | none: backend (result visible in the AI panel history and Activity) | ai_apply_recovery.rs::adr_0019_stop_after_every_apply_boundary_reopens_consistent, ::recovery_runs_once_and_a_second_open_changes_nothing, ::a_completed_apply_closes_the_journal_and_survives_a_crash, ::adr_0019_every_change_set_operation_is_database_only, ::duplicating_a_project_with_an_unfinished_apply_never_claims_it_was_undone | S2 |
| AI-006 | Real-model E2E: ask, retrieve, propose, apply or reject, stale, restart, index rebuild | P0 | Missing (doc 22 §6) | E2E | M9 | ai-panel | pending | S0 |
| AI-007 | Automated no-egress test (inference only to 127.0.0.1) | P0 | Missing (threat T33) | Test | M9 | none: backend | pending (openframe-ai supervisor.rs::llama_server_launch_is_loopback_only_and_key_is_not_on_the_command_line checks flags, not egress) | S0 |
| AI-008 | Prompt injection and retrieval poisoning corpus | P0 | Injection tests exist in `ai_agent.rs` [V]; poisoning corpus missing | Corpus | M9 | none: backend | ai.rs::prompt_injection_in_project_text_is_data_not_instructions; ai_agent.rs::injected_text_in_project_content_stays_data_inside_observations; ai_toolbox.rs::read_tools_never_mutate_and_resist_injection_text; security_review.rs::ai_01_prompt_delimiters_survive_unicode_case_folding | S2 |
| AI-009 | Every new operation gets metadata, exposure decision, tool, projection, eval case, regenerated coverage doc | P0 | Process enforced by tests; CI fails on coverage drift (`git diff --exit-code`, added 2026-10-05) | Applies to each feature milestone | all | none: process | ai_tool_coverage.rs::every_registered_operation_has_explicit_metadata, ::tool_coverage_matches_operation_metadata, ::coverage_matrix_document_is_generated_from_the_registry | S2 |
| AI-010 | **Report AI output** action (Store policy for live generative AI) | P0 | Missing; needs the support address (X15, M9.13) | In-app action, support link, no auto-send | M9 | pending (ai-panel) | pending | S0 |
| AI-011 | Store build bundles the runtime; weights downloaded on click (ADR-0020) | P0 | Runtime is downloaded today (lifecycle tests); ADR-0020 Proposed | Packaging change, disclosure | M9 | none: packaging | pending | S0 |
| AI-012 | Production manifest key and model host configured; dev key never trusted | P0 | Gate fails closed [V: release-validate] | X7 | M9 | none: release | openframe-ai install.rs::manifest_signed_by_another_key_is_rejected (runtime trust only) | S1 |
| AI-013 | Model update regression suite | P1 | Benchmark script exists | Thresholds, CI hook | M9 | none: release | pending | S1 |
| AI-014 | Product knowledge covers new features; AI reads and proposes new feature data only via tools | P1 | `knowledge.md` exists [V] | Update per milestone | all | ai-panel | ai.rs::ai_ac_023_product_questions_are_grounded_in_the_product_guide; knowledge.rs (unit) | S2 |

## PLAT: platform and release engineering

Source: Engineering Package Index; `docs/engineering/22` to `28`; launch readiness register; masterplan §1a; ADR-0017 and ADR-0018 (proposed).

| ID | Requirement | Pri | Current | Gap | MS | UI route | Tests | State |
|---|---|---|---|---|---|---|---|---|
| PLAT-001 | Supported Windows versions decided and enforced in manifest and docs | P0 | Conflict: Win11 (README) vs 1809 (manifest) [V] | D3 | M0, M11 | none: platform | pending | S0 |
| PLAT-002 | CI: Windows runner format, clippy, tests, web, bindings, **desktop E2E as a release gate** | P0 | CI jobs rust, web, governance, e2e, supply-chain on windows-latest; release `build` and `store` need the tag E2E job (2026-10-05) [V] | First green CI run on GitHub; E2E specs per workspace (M1.13) | M1 | none: release | 01-journey.test.mjs; 02-restart.test.mjs; 95-reachability.test.mjs (local run 18/18) | S2 |
| PLAT-003 | Supply chain green: `cargo deny`, `npm audit`, licence inventory | P0 | Supply-chain CI job made blocking 2026-10-05 and `cargo audit` added; yanked `yoke-derive` updated (M0.21) | Green CI run on GitHub | M0 | none: release | local 2026-10-05: cargo deny check, cargo audit, npm audit --omit=dev, license-inventory --check (docs/release/evidence/M0/INDEX.md) | S2 |
| PLAT-004 | Architecture lint (forbidden imports, direct DB writes, unregistered operations, binding drift) | P0 | `scripts/architecture-lint.mjs` and blocking CI governance job added 2026-10-05 (M0.23); bindings and AI-coverage drift checked in the rust job | Green CI run on GitHub | M0 | none: release | scripts/tests/governance.test.mjs (architecture lint rules W1-W3, C1-C5, R1-R5) | S2 |
| PLAT-005 | Migration fixture corpus per shipped schema version | P0 | Implemented 2026-10-05: fixtures generated from the frozen SQL for project 1 to 11, global 1 to 3, app 1 [V] | Permanent binary fixtures from the first external build (doc 23 §4) | M1 | none: release | migration_fixtures.rs::m1_04_every_project_schema_version_upgrades_with_content_backup_search_and_export, ::m1_04_every_global_vault_schema_version_upgrades_and_keeps_its_items | S2 |
| PLAT-006 | WebView2 missing/broken handling on clean machines | P0 | `webviewInstallMode: downloadBootstrapper` in `tauri.conf.json`; nothing exercises it [V 2026-10-05] | Clean-VM test | M11 | none: packaging | pending | S1 |
| PLAT-007 | Platform service boundary (no `cfg(windows)` in modules) | P2 | Missing [V] | M16 | M16 | none: platform | pending | S0 |
| PLAT-008 | macOS CI, signing, notarization, sidecar lifecycle, E2E | P2 | Missing [V] | M16 | M16 | none: platform | pending | S0 |
| REL-001 | Licence selected and committed | P0 | Pending [V] | D1 | M0 | none: release | pending | S0 |
| REL-002 | Partner Center identity and name reserved; MSIX identity filled | P0 | `REPLACE_*` placeholders [V] | X2, X3 | M0 | none: release | pending | S0 |
| REL-003 | Store MSIX release candidate built by the real pipeline | P0 | Pipeline written (`release.yml`); never run to a release; only tag is `baseline/pre-masterplan-2026-10` [V 2026-10-05] | RC tag | M11 | none: release | pending | S1 |
| REL-004 | WACK passes on the exact MSIX; report archived | P0 | No evidence (`docs/release/evidence/M11/INDEX.md` empty) | X11 | M11 | none: release | pending | S0 |
| REL-005 | Clean-machine install, upgrade, uninstall, reinstall, offline | P0 | No evidence (`docs/release/evidence/M11/INDEX.md` empty) | Matrix | M11 | none: release | pending | S0 |
| REL-006 | Privacy policy final and hosted on HTTPS | P0 | Draft with placeholders [V] | X9 | M12 | none: release | pending | S0 |
| REL-007 | Store listing accurate to the build; screenshots from the RC; IARC; Notes for Certification | P0 | Draft with `[N]` and `[support link]` [V] | X14 | M13 | none: release | pending | S0 |
| REL-008 | SBOM, checksums, licence inventory, release evidence package | P0 | Release workflow produces them [V] | Run on RC | M11 | none: release | pending | S1 |
| REL-009 | Release notes, known issues, rollback artifact | P0 | Missing | Author | M13 | none: release | pending | S0 |
| REL-010 | Direct channel: Authenticode signing, updater key, NSIS/MSI | P2 | Pipeline written (Authenticode, NSIS, MSI, updater), blocked on D15 and X21 [V 2026-10-05] | D15, X21 | M16 | none: release | pending | S1 |

## DOC: documentation work

Source: `docs/README.md`; `docs/engineering/31-ai-assisted-development-controls.md`; masterplan §1.

| ID | Requirement | MS | State |
|---|---|---|---|
| DOC-001 | Documentation map and governance (this integration) | M0 | S2 |
| DOC-002 | Promote supplied documents into the tree with headers | M0 | S2 |
| DOC-003 | Mark LAN sections in PRD, FSD, UX, Domain, Security, Collaboration, Import/Export specs as superseded (banners now, body rewrite later) | M0, M10 | S1 |
| DOC-004 | Fix stale facts in README, DEVELOPMENT, Engineering Index, ESD, doc 22, 23, 27, LICENSE-PENDING | M0 | S1 |
| DOC-005 | Resolve each ADR proposed here (accept, amend or reject) | M0 | S0 |
| DOC-006 | Write contracts: screenplay engine (doc 32) and production depth (doc 33) | M0 | S1 |
| DOC-007 | Regenerate the User Manual so it matches the shipped UI and drops figures 164 to 168 | M10 | S0 |
| DOC-008 | In-app help and public documentation, knowledge base | M12 | S0 |
| DOC-009 | Keep this ledger, the masterplan and the launch register in step on every merge | all | process |
| DOC-010 | Repository root cleaned: specifications in `docs/specs/`, manual in `docs/manual/`, decision register in `docs/adr/README.md`, superseded LAN sections marked in place (57) | M0 | S2 |
