# W9 security and stress mapping

> 2026-10-18. Maps the gate rows S1, S8, S9 and P2 to tests that exist. Every test name below was found with grep under
> `crates/` or `tests/e2e/specs/` before it was written here. Gate: [windows-internal-release-gate.md](../../windows-internal-release-gate.md).

## S1 Threat model current: Pass

Rechecked 2026-10-18 against the code and tests. Every test name below was found with grep. T39 got its test at the end of the session, so S1 is Pass. Threat model: [20-security-threat-model.md](../../../engineering/20-security-threat-model.md).

| Row | Status | Test or reason |
|---|---|---|
| T1 | Pass | `apps/desktop/src/ai/noLiveHtml.test.tsx`: "source tree has no unsafe HTML sinks" (fails on `dangerouslySetInnerHTML`, `.innerHTML =`, `insertAdjacentHTML`, `document.write`, `new Function(`, `eval(`) and "assistant answers are never live HTML" |
| T7 | Pass | `crates/openframe-security/tests/security_paths.rs`: `input_files_must_be_regular_files_within_the_cap` |
| T19 | Not applicable in Chapter 1 | Internal builds have no updater. The store delivers updates in Chapter 3 |
| T24 | Pass | `tests/e2e/specs/05-webview-network.test.mjs` (all WebView2 processes, WebDriver and direct run, net log) and `72-ai-assistant.test.mjs` ("no network: the app and the engines use loopback only while AI works"); passed in the final release run ([e2e-release-runs.md](e2e-release-runs.md)). No source-level test bars HTTP client use outside the downloader modules; the run-time specs are the proof |
| T33 | Pass | Same two specs as T24 |
| T35 | Pass, manual part in W10 pilot | `crates/openframe-platform/src/cloud.rs`: `documents_redirected_to_onedrive_is_found_through_the_environment`, `a_fake_onedrive_root_and_the_other_services_are_found_by_name`, `the_local_alternative_is_not_synced`; `apps/desktop/src/app/home/NewProjectDialog.test.tsx`: "warns about a folder OneDrive syncs and offers a folder on this computer (W2)". The 10-minute typing test on a synced folder is part of the W10 pilot |
| T39 | Pass | `crates/openframe-application/tests/ai/ai_agent.rs`: `the_ai_log_and_report_hold_no_prompt_or_project_text`. Markers planted in a scene, a note, the request and the model reply never appear in any column of `sys_ai_trace` or in the exported AI report |
| T40 | Later chapter | Mac track M2, not Chapter 1 Windows |

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

## S14 Reviews: Pass

A security review of the code changes of this session found no critical or high issue. One medium: the uninstaller removed the whole `%LOCALAPPDATA%OpenFrame` folder when the app data box was ticked. Fixed in 6f9a280: it removes only the app's own items, then the folder only if empty, and a governance test guards it. Lows recorded, not fixed: a future trial record in app data (Chapter 3), bounds in `check-binary`, and shell command strings in the installer script that use only trusted paths. Earlier reviews: [sql-audit.md](sql-audit.md), [binary-hardening.md](binary-hardening.md) (BinSkim), gitleaks in [supply-chain.md](supply-chain.md), and the threat model refresh above.
