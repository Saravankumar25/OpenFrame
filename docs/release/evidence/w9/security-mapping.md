# W9 security and stress mapping

> 2026-10-18. Maps the gate rows S1, S8, S9 and P2 to tests that exist. Every test name below was found with grep under
> `crates/` or `tests/e2e/specs/` before it was written here. Gate: [windows-internal-release-gate.md](../../windows-internal-release-gate.md).

## S1 Threat model current: Open

The threat model ([20-security-threat-model.md](../../../engineering/20-security-threat-model.md)) was refreshed on
2026-10-18. More than two rows still carry a yellow or planned mark instead of a named test, so S1 stays Open:

| Row | Gap listed in the threat model |
|---|---|
| T1 | CI grep for `dangerouslySetInnerHTML` is planned (the no-live-HTML tests cover the web source) |
| T7 | File-size cap is marked yellow in the table, but `input_files_must_be_regular_files_within_the_cap` exists (see S8) |
| T19 | Updater plugin not integrated |
| T24 | No test that bars HTTP client use outside the downloader modules |
| T33 | The web view is not covered by the loopback-only check |
| T35 | Manual test pending |
| T39 | No test names the numbers-only rule of the AI log |
| T40 | Planned for the Mac track |

## S8 Files: Pass, images noted

| Check | Tests cited (file: test name) | Status |
|---|---|---|
| Traversal, absolute, UNC, drive, reserved names (T3, T4a) | `crates/openframe-security/src/lib.rs`: `relative_paths_cannot_escape`, `confine_joins_inside_root`, `sanitize_handles_hostile_names`; `crates/openframe-security/tests/security_paths.rs` | Pass |
| Links and junctions (T4a) | `crates/openframe-security/tests/confinement_and_network.rs`: `new_files_below_a_junctioned_assets_folder_are_refused`, `a_junction_deeper_inside_assets_is_refused_too`, `a_dangling_symlink_is_refused_when_symlinks_can_be_created`, `unc_project_locations_are_network_locations` | Pass |
| Zip slip, absolute names, oversized entries, entry count (T6) | `crates/openframe-security/src/archive.rs`: `zip_slip_rejected_and_nothing_written`, `absolute_and_drive_paths_rejected`, `oversized_entry_rejected`; `crates/openframe-security/tests/security_paths.rs`: `archives_declaring_too_many_entries_are_refused_before_parsing` | Pass |
| XML floods and entity expansion (T6a) | `crates/openframe-import-export/tests/security_parsers.rs`: `xml_attribute_flood_fails_fast`, `fdx_with_attribute_flood_is_rejected_not_hung`; `crates/openframe-import-export/tests/hostile_corpus.rs`: `no_entity_expansion` check used by `m1_15_fdx_hostile_corpus` | Pass |
| PDF bombs, nesting, malformed PDF (T6a) | `security_parsers.rs`: `pdf_deep_nesting_is_rejected_before_lopdf`, `pdf_decompression_bomb_is_rejected`, `pdf_unbounded_filters_are_refused`, `pdf_object_stream_nesting_is_screened_after_inflating`, `pdf_page_content_amplification_is_bounded`; `crates/openframe-import-export/tests/pdf.rs` (garbage bytes after `%PDF-1.7` give a clear error); `hostile_corpus.rs`: `m1_15_pdf_hostile_corpus` | Pass |
| Huge files (T7) | `security_paths.rs`: `input_files_must_be_regular_files_within_the_cap`; `hostile_corpus.rs`: `m1_15_huge_files_are_refused_before_reading`, `m1_15_huge_text_patterns_parse_in_bounded_time` | Pass |
| Crafted documents crash a parser (T41) | W3 fuzz evidence: [fuzz-2026-10-16.md](../w3/fuzz-2026-10-16.md); `hostile_corpus.rs` `m1_15_*_hostile_corpus` for FDX, Fountain, DOCX and PDF | Pass |
| Malformed images | Limits in `crates/openframe-import-export/src/report.rs` (`ReportImage::decode`: 20,000 pixels an edge, 512 MiB allocation, 64 MiB source, decoder panic caught). `crates/openframe-import-export/tests/report.rs` checks that bytes that are not an image give `export.image_unreadable`. Image import in `modules/visual/common.rs` sniffs the file type (`sniffs_images`) and has no malformed-image test | Pass, images: limits and one unreadable-image test; no malformed-image import test |

## S9 Fuzzing: Exception (owner approved 2026-10-18)

Six fuzz targets ran in W3 and are recorded in [fuzz-2026-10-16.md](../w3/fuzz-2026-10-16.md). On 2026-10-18 the owner
decided not to re-fuzz in the endgame session (owner decisions, row "Scope of the endgame session"). The hostile corpus
tests above still run in the normal suite.

## P2 Crash and power loss: Pass

| Check | Tests cited | Status |
|---|---|---|
| Kill while typing or saving | Specs `tests/e2e/specs/26-forced-kill-keeps-work.test.mjs` and `04-close-keeps-typing.test.mjs`, passed in the final run ([e2e-release-runs.md](e2e-release-runs.md)) | Pass |
| Disk full or device failure at commit | `crates/openframe-application/tests/platform/failure_injection.rs`: `m1_11_disk_full_and_device_failure_at_commit_lose_nothing`, `m1_11_export_to_a_full_drive_leaves_the_previous_file`, `m1_11_read_only_project_database_is_never_modified`, `m1_11_antivirus_lock_on_the_checkpoint_during_save` | Pass |
| Full disk before the AI download | `crates/openframe-ai/src/install.rs`: `preflight_counts_what_is_left_and_refuses_a_full_or_unknown_disk` (code `ai.disk_full`) | Pass |
| Network pulled during the download | `crates/openframe-ai/src/fetch.rs`: `a_file_downloads_then_an_interrupted_one_resumes_where_it_stopped`, `a_host_without_ranges_restarts_cleanly_and_never_doubles_the_file`, `more_than_the_pinned_size_a_redirect_away_and_a_pause_are_all_stopped`; spec `tests/e2e/specs/78-ai-download.test.mjs` (interrupted download resumes with a range request, pause and resume) | Pass |
| Virtual machine powered off while typing | Covered by the W10 pilot decision ([vm-blocked.md](vm-blocked.md)) | Covered by W10 |
