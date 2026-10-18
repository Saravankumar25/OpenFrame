# Mac phases M1 and M2: evidence

Recorded 2026-10-17 on the development Mac. Proof level 2 (proved on development, ADR-0030).

## Machine

| Item | Value |
|---|---|
| Computer | Apple M1, 16 GB memory, Apple silicon (`uname -m`: arm64) |
| macOS | 26.5.2 (build 25F84) |
| Swift | 6.3.3, from the Command Line Tools (no full Xcode installed) |
| Rust | 1.98.1, host `aarch64-apple-darwin`, installed with rustup |
| Node | 22.21.0 |

The Mac in use is Apple silicon, so the plan's "proved on Intel" level does not apply here: every row
below is proved on Apple silicon. Full Xcode is missing; it is needed for the UI testing framework,
`xcodebuild`, signing and the disk image (phase M9). Owner task: install Xcode.

## M1: the core on macOS

`RUSTC_WRAPPER= cargo test --workspace --exclude openframe-desktop --no-fail-fast`

- First run: 2,034 passed, 3 failed, 12 ignored. The three failures were test assumptions:
  1. `ai::the_download_is_offered_only_through_its_own_operations` counted the Windows engine and
     treated a UNC path as a network location. Fixed to be platform-aware; the Apple silicon engine
     is pinned in M8.
  2. `migration_failures::m1_05_read_only_database_is_refused_untouched_and_opens_once_writable`:
     SQLite on Unix creates `-wal` and `-shm` with the database file's permissions. The test guard
     now clears the read-only setting on all three files.
  3. `thirteen_fdx_classes_parse_export_reimport_and_print_as_expected`: the working tree came from a
     Windows checkout with CRLF line endings; it now matches the LF files git stores.
- After the fixes every one of those tests passes (`cargo test -p openframe-application --test ai`,
  `--test platform m1_05`, `cargo test -p openframe-import-export --test fdx_suite`).

The Tauri Mac shell is retired: `.github/workflows/mac.yml` removed, notes in
`docs/archive/2026-10-17-tauri-mac-shell-notes.md`, `.github/workflows/mac-core.yml` added. The parity
matrix is `docs/roadmap/mac-parity-matrix.md` (119 Must-ship features, 41 scenario files).

## M2: the core as a Swift library

- `cargo test -p openframe-ffi`: 10 unit tests and 6 interface tests pass (journey, coded errors,
  restart, allow-list against the inventory, team password gate), 1 helper ignored.
- `cargo clippy -p openframe-ffi --all-targets -- -D warnings`: clean.
- `apps/macos/scripts/build-core.sh`: static library, UniFFI Swift interface, XCFramework (assembled
  by hand without Xcode), 819 generated Swift types, 492 typed operations, 0 fallbacks.
- `apps/macos/scripts/test.sh`: the Swift journey tests (open a project, create a scene, undo, redo,
  events, reopen after restart, 20 parallel calls, coded errors) pass.
