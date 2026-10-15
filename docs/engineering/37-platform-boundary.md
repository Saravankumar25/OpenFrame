# Platform boundary

> **Current decisions (2026-10-15).** The boundary rules stand. The Mac side is the platform adapter inside the Rust core plus a SwiftUI shell ([ADR-0027](../adr/ADR-0027-rust-core-two-ui-shells.md)); statements about the Tauri shell on the Mac are history. The Mac adapter first runs in Mac phase M3.

> Written 2026-10-06 from `crates/openframe-platform/src/` (`lib.rs`, `atomic.rs`, `dirs.rs`, `exists.rs`, `identity.rs`,
> `keys.rs`, `process.rs`; `spelling.rs` added 2026-10-07 for proofing) and from rule R6 and the `OS_ALLOW` list in `scripts/architecture-lint.mjs`.
> Decisions behind it: [ADR-0017](../adr/ADR-0017-cross-platform-windows-and-macos.md). Plan area: "Platform adapter layer for
> Windows and Mac" in [masterplan.md](../implementation/masterplan.md). State of the feature:
> [feature-status.md](../implementation/feature-status.md) (row PLAT-007). The broader compatibility plan is
> [28-cross-platform-compatibility-security.md](../archive/2026-10-05-cross-platform-compatibility-security-plan.md).

## 1. The rule

Feature code never knows which operating system it runs on. It does not write `cfg(windows)`, `cfg(target_os = ...)`,
`std::os::windows`, `std::os::unix`, `std::os::macos`, or use the `windows` and `libc` crates. It calls a function in
`openframe-platform`, and that crate picks the Windows adapter or the macOS adapter at compile time.

Architecture lint rule R6 enforces this. It scans every Rust file under `crates/` and `apps/desktop/src-tauri/src/` (comments
removed, `#[cfg(test)]` ranges skipped) for those patterns and fails on any match outside the allow-list. Cargo integration tests,
benches and the `openframe-test-support` crate are not scanned. A new allow-list entry is a reviewed security change and must carry
its reason.

### Allow-listed files and why

| File | Why it may know the operating system |
|---|---|
| `crates/openframe-platform/` (the whole crate) | It is the boundary. The Windows adapter and the macOS adapter live here. |
| `apps/desktop/src-tauri/src/distribution.rs` | Desktop shell: MSIX package data folder redirection and the WebView2 missing-runtime dialog (Windows installer channel). |
| `crates/openframe-ai/src/hardware.rs` | DXGI adapter enumeration and the Vulkan library check for the hardware check. To move behind `openframe-platform` when the Mac (Metal) check is written. |
| `crates/openframe-security/src/lib.rs` | Drive-letter and UNC classification for network drive refusal. Windows path prefixes exist only on Windows. |
| `crates/openframe-ai/src/manifest.rs` | Picks the runtime manifest entry for this machine by reading `std::env::consts::OS`. Data only, no OS branch. To call `current_os()` instead. |
| `crates/openframe-application/src/modules/diagnostics.rs` | Fallback OS label in the diagnostics report. Data only, no OS branch. To call `current_os()` instead. |
| `crates/openframe-application/src/modules/project.rs` | Reports the platform name to the interface. Data only, no OS branch. To call `current_os()` instead. |
| `crates/openframe-persistence/src/lib.rs` | Opens SQLite through the `win32-longpath` VFS with a `\\?\` path so projects in deep folders work. The VFS name is Windows-only. |
| `apps/desktop/src-tauri/src/platform/` | Desktop shell macOS adapter: the StoreKit 2 bridge and the licence server client (section 8). `lib.rs` calls it without any `cfg`; Windows builds get no-op twins. |

The hardware, security, persistence and the three label entries are exceptions. Each one is listed in section 6 as work that still has to move.

## 2. Modules

| Module | One line |
|---|---|
| `lib.rs` | The `Os` enum (`Windows`, `MacOs`, `Other`), `current_os()` and `Os::name()`. For diagnostics, logs and the few labels that differ. Feature code should not branch on it. |
| `atomic` | The one shared atomic file replace: write to a temp file in the destination folder, sync, rename over the destination, clean up on failure. |
| `dirs` | App data folder, Documents folder, `Documents/OpenFrame`, and the base local data folder. |
| `exists` | `file_exists`: `Path::is_file`, answered with one attribute query on Windows where that is safe. |
| `identity` | The MSIX package family name of this process, and a validator for it. |
| `keys` | Keyboard labels: `Ctrl` and `Alt` on Windows, Command and Option symbols on macOS. |
| `process` | Managed child processes: a guard that ties a child to the app, and a command setup that hides the console window. |
| `spelling` | The operating system's spelling dictionaries for proofing (SCRIPT-020): installed languages, unknown words in a text (UTF-16 offsets), suggestions for a word. A `SpellEngine` trait so the application layer can be tested with a fake. |

## 3. Adapter state per module

"Verified on Windows" means the crate's own tests run and pass on a Windows 11 machine. "Compiles for macOS, unverified" means
the code is written and gated for macOS, no macOS build has been run, and it stays unverified until Apple silicon hardware or an
Apple silicon CI runner is available.

| Module | Windows adapter | macOS adapter |
|---|---|---|
| `lib.rs` | Verified on Windows. `current_os()` is a `const fn` over `cfg!`. | Same code. Compiles for macOS, unverified. |
| `atomic` | Verified on Windows by 10 tests (section 5). One test, the read-only destination, expects the rename to fail on Windows. | The same source, with no OS-specific code. The test expects the rename to succeed on POSIX, because the folder and not the file mode decides. Unverified on macOS and APFS. |
| `dirs` | `%LOCALAPPDATA%\OpenFrame` and `Documents\OpenFrame`. The test `windows_data_folder_is_under_local_app_data` checks the first. | `~/Library/Application Support/OpenFrame` and `~/Documents/OpenFrame`, through the `directories` crate. The test `mac_data_folder_is_under_application_support` is written, never run. Inside the App Sandbox the home folder is the container, so the same calls land there. Not checked in a sandbox. |
| `exists` | `GetFileAttributesW` for paths shorter than 240 UTF-16 units that contain no NUL and are not links. Links, long paths and everything else fall back to `Path::is_file`. Verified by two tests, including a path longer than 240 units. | `Path::is_file`. The shared tests apply. Unverified on macOS. |
| `identity` | `GetCurrentPackageFamilyName`, with one retry for a larger buffer. The unit test runs unpackaged and expects `None`. The packaged case (a real MSIX install) has no automated test. | Always `None`. The App Store receipt check belongs to the entitlement adapter and does not exist yet. The bundle identifier is read by the desktop shell, not here. |
| `keys` | `Ctrl`, `Alt`, `+` between keys. Verified. | Command, Option, Shift, Return and Delete symbols, no separator. The mapping functions take an `Os` argument, so the Windows test run checks the macOS labels as pure text. Real shortcut behaviour in WebKit is untested. |
| `process` | A Job Object with kill-on-close and die-on-unhandled-exception limits, and `CREATE_NO_WINDOW`. `ChildGuard::kernel_enforced()` is true: the OS ends the child when OpenFrame dies, even in a crash. Verified by `a_guarded_child_dies_when_the_guard_is_dropped`. | The child starts in its own process group and the guard sends SIGTERM to the group when dropped. `kernel_enforced()` is false. If OpenFrame crashes, the drop does not run and nothing ends the child. The test is written, never run. |

| `spelling` | The Windows Spell Checking API (`ISpellCheckerFactory`, `ISpellChecker`) on one worker thread that owns its COM apartment, so callers never initialise COM and COM objects never cross threads. Every call waits at most 5 seconds; input is bounded (64 K UTF-16 units per check, 64 characters per word). Verified by three tests, one against the real English (United States) dictionary when it is installed. No dictionary files ship with OpenFrame and nothing is downloaded. | Not built. Reports no installed languages and `adapter_available() == false`; the editor then falls back to the web view's own spell checking. The Mac adapter needs `NSSpellChecker` on the main thread and Mac hardware to build and verify. |

### Process control on macOS: a gap

The comment in `process.rs` says the parent-death guarantee on macOS comes from "the sidecar supervisor's stop-on-drop plus a
startup sweep of stray engines". The supervisor (`crates/openframe-ai/src/supervisor.rs`) has the stop-on-drop (`impl Drop for
Supervisor`) and attaches the `ChildGuard`. A search of `crates/openframe-ai/src` for a startup sweep of stray engines found none.
Treat the macOS crash case as open until that sweep exists and has a test. The comment describes a sweep that does not exist, and is to be
corrected with the code.

## 4. Where the crate is used today

The move onto the crate is in progress, so this list is a snapshot of 2026-10-06. Re-run
`grep -rn "openframe_platform" crates apps/desktop/src-tauri/src` for the current picture.

| Module | Callers in the snapshot |
|---|---|
| `atomic` | AI model manager and AI store; application core (stale temp cleanup at start), file operations, backup, package format and project operations; persistence; project format; the entitlement trial record. |
| `dirs` | Project format (its app data, Documents and `Documents/OpenFrame` functions call the crate); the desktop shell's package folder redirection (`base_data_dir`). |
| `exists` | `crates/openframe-application/src/util.rs`. |
| `identity` | The desktop shell (`distribution.rs` and `lib.rs`). |
| `process` | The AI sidecar supervisor (`ChildGuard` and `prepare_background`). |
| `current_os` | The desktop shell, to pick the release channel on macOS. |
| `keys` | No caller yet. No screen shows a platform-specific shortcut label through the crate. |

## 5. The shared atomic replace contract

`atomic.rs` is written once and has no `cfg`. The same five rules hold on both systems:

1. **Same-volume temp file.** The new content is written to a temp file in the destination's folder, under a unique name, so the
   final step is a same-volume rename and never a cross-volume copy.
2. **Flush and sync first.** The temp file is flushed and synced (`sync_all`) before the rename.
3. **The old file is never deleted before the rename.** A rename over an existing file replaces it in one step. If the rename
   fails, the old file is untouched.
4. **No leftovers.** On any failure the temp file is removed. Nothing with the temp suffix is left behind.
5. **A read-only destination fails with the original error and keeps the old file.**

Supporting facts:

- The temp name is `.<destination name>.<16 hex digits>.openframe-tmp`. The suffix constant is `TEMP_SUFFIX`.
- Functions: `replace_with` (a closure writes into the open temp file), `replace_bytes`, `replace_from_staged` (renames a file the
  caller wrote in the same folder, and removes it when the rename fails or the folder differs), `unique_temp_path` and
  `remove_stale_temps` (removes only dot-prefixed regular files that end in the suffix).
- A destination with no folder, or a folder that does not exist, fails without creating anything.
- The sync covers the file. The crate does not sync the folder entry after the rename.

The 10 tests are `replaces_content_and_leaves_no_temp_file`, `creates_a_missing_destination`,
`a_failing_writer_keeps_the_old_file_and_cleans_up`, `read_only_destination_keeps_the_old_file_and_reports_the_error`,
`missing_folder_fails_without_creating_anything`, `staged_file_must_be_in_the_destination_folder`,
`staged_file_in_place_is_renamed_over_the_destination`, `temp_names_are_unique_hidden_and_in_the_folder`,
`stale_temps_are_removed_and_user_files_are_not` and `large_write_is_complete_or_absent`.

## 6. What still has to move behind the boundary

| Item | Where it is now | What the move needs |
|---|---|---|
| Open a file and reveal it in the file manager | The desktop shell calls `tauri_plugin_opener` in `apps/desktop/src-tauri/src/lib.rs` (`open_path`, `of_open_url`). The plugin handles both systems. | Decide whether the shell stays the only caller (the plugin is already cross-platform) or the call goes behind a `platform` function. Either way, record the decision. On macOS, check reveal and open inside the App Sandbox. |
| Power events | No power-event code found in the crates or the desktop shell. | A platform function for sleep and wake, so a running export, backup or AI job pauses and recovers on both systems. |
| Window integration | Not in the crate. | Menu bar on macOS, window chrome and title bar rules on both systems. |
| DXGI hardware check | `crates/openframe-ai/src/hardware.rs` (allow-listed). Also the Vulkan library check there. | A `platform` hardware query with a Windows adapter (DXGI) and a macOS adapter (Metal). |
| Drive classification | `crates/openframe-security/src/lib.rs` (allow-listed), `drive_kind` and drive-letter checks. | A `platform` path classification for network and removable locations on both systems, so the network-drive refusal also covers macOS mounts. |
| Long-path SQLite VFS | `crates/openframe-persistence/src/lib.rs` (allow-listed). | A `platform` function that returns how to open a database path. macOS needs no long-path VFS, and the function must say so. |
| Operating-system label reads | The three `std::env::consts::OS` entries in the allow-list (section 1). | Call `openframe_platform::current_os()` and delete the three entries. |
| Any other writer of project or user files | Not in the section 4 list. | Check each remaining file write against the five rules in section 5. |
| Startup sweep of stray engines on macOS | Does not exist (section 3). | Write it, with a test, then correct the comment in `process.rs`. |
| Secure storage and the updater | Not in the crate. | ADR-0017 lists both behind the boundary. The Mac App Store build has no self-updater. |

Each item that is done ends with the entry removed from `OS_ALLOW` or from this table. The lint then proves the move.

## 7. Next steps for Mac continuous integration

`ci.yml` runs on `windows-latest` only. `.github/workflows/mac.yml` (section 8) is a first, narrow macOS job: it builds the
StoreKit bridge and the desktop shell and runs the `openframe-platform`, `openframe-entitlement` and desktop unit tests on Apple
silicon. It has not run yet. Nothing in section 3 marked unverified can be called verified until it has. Still to do, in order:

1. Add a macOS job on an Apple silicon runner (check the runner label) that runs `cargo fmt --check`, `cargo clippy`,
   `cargo test -p openframe-platform` and `cargo test -p openframe-entitlement`. These crates need no web view, so they are the first
   to run and the quickest to check.
2. Widen the job to the whole Rust workspace, then the web tests and the bindings drift check. Fix what fails on macOS in the
   crates, never by adding `cfg` to feature code.
3. Run the three macOS tests that exist but have never run: `mac_data_folder_is_under_application_support`,
   `a_guarded_child_gets_sigterm_when_the_guard_is_dropped` and the shared atomic tests on APFS.
4. Add the Mac-only checks the plan names: App Sandbox entitlements, security-scoped access to folders the user picks, and an
   outgoing network limited to the model download.
5. Only after steps 1 to 4, move PLAT-008 up in [feature-status.md](../implementation/feature-status.md), with the run as evidence.

Mac hardware or a Mac runner and an Apple Developer account are owner tasks in
[owner-decisions.md](../implementation/owner-decisions.md).

## 8. macOS shell adapter: StoreKit bridge

The Mac App Store build connects StoreKit 2 and the licence server to the entitlement service through the desktop shell. Written
on 2026-10-06 on Windows. **Unverified: none of it has been compiled on a Mac, linked or run** until the Mac workflow
(`.github/workflows/mac.yml`) passes. The product rules are in [38-mac-app-store-trial.md](38-mac-app-store-trial.md).

| File | What it does |
|---|---|
| `apps/desktop/src-tauri/macos/StoreKitBridge/` | Swift package (StoreKit 2 only, macOS 13 or later). Exports C functions that return `strdup` JSON, and listeners on `Transaction.updates` and `Storefront.updates` that call back into Rust. |
| `apps/desktop/src-tauri/build.rs` | For a macOS target only: `swift build -c release --product StoreKitBridge` into `OUT_DIR`, then links `libStoreKitBridge.a`, the `StoreKit` and `Foundation` frameworks and the Swift runtime paths from `swift -print-target-info`. Allow-listed for rule R5: it starts `swift` at build time, never in the app. |
| `src/platform/storekit.rs` | The bridge's JSON contract and its mapping onto the `openframe-entitlement` types (`VerifiedTransaction`, `BridgeError` and the rest). Pure Rust, compiled on macOS and in tests on every system. |
| `src/platform/macos_storekit.rs` | The C ABI calls (bounded reads, each string freed once) and `StoreKitChannel`, the crate's `StoreKitBridge` implementation. Allow-listed for rule R4. |
| `src/platform/mac_inputs.rs` | Builds `MacInputs`: the bridge, an HTTPS licence server client (`reqwest` blocking over rustls, 10 second timeout, 64 KiB answer cap, no redirects), nonces from `OsRng` and the monotonic `InstantClock`. The two server addresses and the Ed25519 response key are placeholders the owner replaces. With the placeholder key no request is ever sent and the state stays NeedsOnlineCheck. |
| `src/platform/revalidation.rs` | One background thread runs every check: at launch, on resume or reopen, after sleep (wall clock ahead of the monotonic clock), when focus returns after more than 30 minutes, on a StoreKit transaction or storefront change, and when the lease ends. |

**How it is wired.** `lib.rs` sets `AppConfig::mac_inputs` from `platform::mac_inputs()`, which is `None` on Windows. The
`EntitlementService` keeps the inputs and builds the Mac adapter on the first check, off the main thread, because building it
asks StoreKit for the environment. Until that check answers, the state is NeedsOnlineCheck. `EntitlementService::revalidate`
runs the channel's restore check today, and `EntitlementService::lease` uses fixed intervals by state. Both switch to the Mac
adapter's `revalidate` and `time_until_check` once `ChannelAdapter` exposes them. The log carries the reason and the state name,
never an identifier. No Tauri command is added.

**Known limits.** The adapter takes one response key for both server deployments. The environment comes from the verified
`AppTransaction`, with a build default (Production for release builds) when StoreKit cannot answer. The App Sandbox
`com.apple.security.network.client` entitlement is not added yet; it belongs to the Mac App Store package area.
