# Mac-only changes to keep when the Windows core comes in (2026-10-27)

Written before the merge, as the AI port orders ask (section 4, step 2). It lists every file the Mac tree changed in the
shared code since the last Windows sync, and what happens to each change when the Windows code of `114dff50` is merged.

## The three trees

| Name | What it is |
|---|---|
| Base | The Windows tree at the last Mac sync: GitHub `324cd963`, folder `windows/` (tree `11081707`) |
| Mac | This branch before the merge (`mac/screening-room-theme` plus the dead code cleanup of 2026-10-27) |
| Windows | Local Windows `main` at `114dff50`, the same tree as GitHub `main` `windows/` at `85d8574a` |

Paths compared: `crates/`, `migrations/`, `scripts/`, `apps/desktop/`, `packaging/`, `tests/`, `fuzz/`, `templates/`,
`services/` and the root `Cargo.toml`, `Cargo.lock`, `package.json`, `deny.toml`. The Swift shell (`apps/macos/`) has no
Windows copy and is not part of the merge.

| Group | Files |
|---|---:|
| Changed on the Mac only | 205 |
| Changed on Windows only | 370 |
| Changed on both | 141 (51 now identical, 43 merge without a conflict, 47 conflict) |

## Kept as they are (Mac wins)

- **Mac-only crates.** `openframe-ffi` (UniFFI, `invoke`, events, `cancel_task`, `power_event`) and
  `openframe-gold-project` with its `tests/gold-project` data.
- **Metal engine placement (ADR-0033).** `crates/openframe-ai/src/bundle.rs`, `gpu.rs`, `manager.rs`: `-ngl 999
  --device MTL0`, the fall back to the processor and the remembered failure. The Windows S8 engine settings
  (`cache_prompt`, `--parallel 1`, `--cache-ram 512`, the answer slot, `timings.cache_n`) are added on top.
- **macOS runtime lock and fetch.** `packaging/ai/ai-assets.lock.json` (entry `llama-cpp-b11259-macos-arm64-metal`),
  `packaging/ai/fetch-macos-runtime.sh`, `scripts/prepare-ai-assets.mjs`.
- **macOS process watcher.** `crates/openframe-platform/src/process.rs`: the child engine's group ends within a second of
  OpenFrame ending, also after a crash (macOS has no job objects).
- **StoreKit revalidation.** `crates/openframe-entitlement/src/adapters.rs`, `mac/adapter.rs`.
- **Mac tooling.** `scripts/gen-swift-types.mjs`, `gen-mac-parity.mjs`, `gen-theme-reference.mjs`, `mac-html/*`,
  `license-inventory.mjs`, `scan-installer.mjs`, `package.json` scripts, `deny.toml` entries for the Mac crates.

## Mac changes that are improvements for both platforms (to repeat on Windows, ADR-0034)

Kept on the Mac. Written down here and in the progress log so the Windows session makes the same change; `windows/` is
not edited from the Mac.

| File | Change |
|---|---|
| `crates/openframe-security/src/lib.rs` | `MAC_EXECUTABLE_EXTENSIONS`: received `.app`, `.command`, `.webloc`, `.pkg`, `.dmg`, `.mobileconfig` and similar files are shown, never opened |
| `crates/openframe-application/src/modules/ai/intelligence/indexer.rs` | The index snapshot says why meaning search failed (`embed_error`) instead of only "updating" |
| Dead code cleanup of 2026-10-27 | Removed with no callers: `openframe-platform::keys`, `openframe_search::get_document`, the unused exchange package fields (`callSheetId`, `productionSource`, `schedule`, `callSheets`), 8 unused crate dependencies, test-only helpers |
| Library move (new in this sync) | Finder metadata files (`.DS_Store`, `._*`) are skipped in the copy and in the check after it |

## Replaced by the Windows version (Windows wins)

| Mac change | Why it goes |
|---|---|
| Eight deterministic AI routes in `modules/ai/commands.rs`, `tools.rs`, `catalog/story.rs`, `toolbox/reads.rs`, `knowledge.md` and their tests | Windows replaced the whole answer path with the conversational assistant (ADR-0037). The port orders forbid keeping an old path next to the new one |
| `schedule.export_daily` in `modules/exports/daily.rs` (PDF, CSV, XLSX) | Windows built the same screen's export as `schedule.export_daily_view` in `daily_view.rs` (PDF, TXT). One operation name on both platforms; the Mac Daily View calls the Windows one. The Mac loses CSV and XLSX for the Daily View |
| Story page in `modules/story/page.rs`, `exports/story.rs`, `docx.rs` | Both trees built it on 2026-10-20; Windows has since added formatting, A4 and migration 0019. Windows version taken |
| `migrations/project/0017_story_page.sql` | Owner decision 2026-10-27: the Mac copy (with a length check) is replaced by the Windows bytes so both platforms ship one schema. A Mac test project made with the old file still opens; the migration runner only logs the checksum difference |
| `apps/desktop/` and `tests/e2e/` | The Mac ships the SwiftUI shell. Its copy of the web app is kept only for the generated types and as the Windows reference, so it follows Windows file for file |

## Conflicts to resolve by hand (47 files)

Rust (20): `modules/ai/commands.rs`, `ai/tools.rs`, `ai/knowledge.md`, `exports/mod.rs`, `exports/story.rs`,
`story/page.rs`, `packages/coverage.rs`, `schedule/ops.rs`, `schema.rs`, `openframe-import-export/src/docx.rs`,
`openframe-ai-eval/src/main.rs`, and the tests `migration_failures.rs`, `exports.rs`, `project_lifecycle.rs`,
`forced_kill.rs`, `ai_toolbox.rs`, `ai_agent.rs`, `openframe-import-export/tests/report.rs`; plus `migrations/frozen.json`
and `0017_story_page.sql` (rules above).

Scripts (4): `check-links.mjs`, `check-writing-standard.mjs`, `writing-standard-baseline.json`,
`architecture-lint.mjs`: Windows logic, with the Mac-only skip paths (`MacHTML/`, `apps/macos/`) kept.

Web app and end-to-end specs (23): Windows version, then the dead code cleanup re-applied where it still applies.

## Pre-existing on the Mac, fixed in the merge

`cargo clippy --all-targets -D warnings` has never been clean on macOS: three test files hold helpers and imports that
only Windows tests use (`openframe-security/tests/security_paths.rs`, `confinement_and_network.rs`,
`openframe-application/tests/platform/failure_injection.rs`). They get a `cfg_attr(not(windows), allow(...))` line,
a change to repeat on Windows.

## Made during the merge (all to repeat on Windows where they apply, ADR-0034)

| File | Change | Windows |
|---|---|---|
| `crates/openframe-platform/src/dirs.rs` | `library_pointer_dir()`: on macOS `~/Library/Application Support/OpenFrame Library`, a folder of its own outside the app data (on Windows it is the Roaming folder, as before) | No change needed |
| `modules/storage/library_ops.rs` | The pointer uses `library_pointer_dir()`; the OneDrive earlier-library folder is looked at on Windows only | Same behaviour |
| `modules/storage/tree.rs`, `relocate.rs`, `library_ops.rs` | Finder files (`.DS_Store`, `._*`) are not copied, checked or counted, and do not stop an empty folder from being removed | Harmless on Windows; repeat for one core |
| `modules/story/page.rs` | A Story page saved by a Mac build of 2026-10-20 (random row id) is read as an unsaved page, so the first save keeps its text | Repeat for one core (no effect on Windows data) |
| `crates/openframe-persistence/src/lib.rs` | A writable database gets the owner's write permission back on its own `-wal` and `-shm` files before it opens (macOS copies the database's permissions to them, so a reader of a read-only file left them read-only) | Unix only; repeat for one core |
| `crates/openframe-ffi` | Reveal kinds follow the moved vault, `dataRoot` waits for the library setup, new `appData`; test switches `test_library_setup` and `test_ai_deterministic` (debug build on a test folder only) | Mac only |
| `tests/ai/ai.rs` | The `\\server\share` network path check runs on Windows only | Repeat |
| `crates/openframe-ai-eval/src/main.rs` | The Mac fix lost in the merge is back: the engine and model folders are made absolute before the engine starts in its own folder (with relative folders every case measured "AI unavailable"), and the run says at once whether the search model is ready | Repeat: the Windows eval has the same fault when given relative folders |
| Three test files | `cfg_attr(not(windows), allow(...))` for helpers only Windows tests use | Repeat |

Windows names kept everywhere: the Mac Daily View now calls `schedule.export_daily_view` and the Story page calls
`story.page.get`, `story.page.save` and `story.page.export`.

## Security review fixes of 2026-10-27 (shared core: repeat on Windows, ADR-0034)

| File | Change |
|---|---|
| `openframe-platform/src/drives.rs`, `openframe-security/src/lib.rs` | macOS network volumes (`statfs` without `MNT_LOCAL`) are refused where network folders are refused |
| `storage/tree.rs` | A `._*` or `.DS_Store` file is Finder metadata only when it is a regular file of at most 1 MB with the AppleDouble or `Bud1` magic; any other such file is user data and moves with the rest; names from the app's own walk may start with a dot |
| `ai/conversation_export.rs`, `ai/trace.rs`, `util.rs` | AI exports go through `check_output_file`; the pointer folder is protected; `library.json` and the pointer are never overwritten by a save |
| `ai/answer/prompt.rs` | Screenplay titles in the system message are escaped, one line, at most 120 characters |
| `ai/dossier/guard.rs` | Path stripping covers `/Volumes/`, `~/`, `/Applications/`, `/Library/`; without contact rights e-mails and phone numbers are hidden in every field |
| `ai/memory/rules.rs` | An answer that looks like an instruction is never saved to memory |
| `openframe-persistence/src/lib.rs` | Side files are changed through a descriptor opened with `O_NOFOLLOW` (regular files only) |
| `storage/library_ops.rs`, `storage/vault_archive.rs`, `openframe-security/src/archive.rs` | A remembered library inside the app data is ignored; a vault restore checks free space against the declared unpacked size first |
