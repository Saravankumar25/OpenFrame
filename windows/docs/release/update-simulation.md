# Update simulation (Windows)

Status: written 2026-11-03. The manual run below is not yet done: it needs two real builds with the same identity. The automated
half of the same item (package file 05, U2) exists and runs with the normal test suite: `update_from_release_baseline` opens the
first release's frozen data with the current build and checks the content (see "What the tests prove" at the end of this page).
This page is the manual half: install version N, use it, install N+1 over it, and confirm that nothing a person owns was lost.

Why it matters: the Microsoft Store closes the app, replaces the program files and starts it again. Projects live in Documents,
the private data lives in the package's `LocalState` folder, and both must survive. Names that must never change (item U3 of
[package file 05](package-2026-11-03/05_UPDATE_SAFETY_AND_STANDARDS.md)): the MSIX identity name and publisher, the package
family name, and the `Straft` folder name inside `LocalState`. A different identity is a different app with an
empty data folder.

## What you need

- Two packages built by the real pipeline, same identity, `N` and a higher `N+1` (`Major.Minor.Build.0`). Both are built with
  `node scripts/build-msix.mjs --store`. For a local run without the Store, the same two versions can be built with `--dev`
  (test identity `StraftStudio.LocalTest`, trusted test certificate or Developer Mode, see
  [27-microsoft-store.md](../engineering/27-microsoft-store.md) section 3). The dev route proves the data survives; only the
  Store route proves the licence state survives.
- A clean Windows 11 PC or a PC where the app is not installed and `Documents\Straft Studio` does not exist.
- This page open, and a place to note results.

## Steps

1. Install version N. `Get-AppxPackage *Straft*` shows its version and package family name. Write them down.
2. Start the app. Do the following so there is something to lose:
   - Create a project with real content: a few ideas, a story beat, two screenplay pages, a schedule, one image added to the Idea
     Vault, one comment.
   - Add something to the Global Idea Vault.
   - Change the appearance: another theme, another density, and any setting that is not the default.
   - Open three projects so the recent list has entries.
   - Click `Download Straft AI`, let it finish, and send one AI request so the engine has run once.
   - Walk the licence path you want to test: stay in the trial (chip shows), or buy, so the data note, tutorial and welcome have
     been shown once each.
3. Note the state: recent list order, theme, the chip text or the absence of it, the size of `models` under
   `%LOCALAPPDATA%\Packages\<family>\LocalState\Straft`, and the contents of `entitlement\notices.json` (flags only; do not
   paste it anywhere public).
4. Update to N+1 in one of these ways:
   - Store route: publish N+1 as an update in the limited audience submission and let the Store deliver it, with the app open once
     and closed once (the Store closes a running app to update it).
   - Local route:
     ```powershell
     Add-AppxPackage -Path .\Straft-Studio-<N+1>-x64.msix
     ```
     with the same identity and a higher version. If Windows says the package is older or has a different identity, stop: that
     is a finding.
5. While the app is running with unsaved typing, repeat the update once (the Store closes the app). Reopen the project. The
   typed text is there or the crash recovery dialog restores it (package file 05, U6).
6. Start N+1 and confirm each of these, tick only what you saw:

| Check | Expected |
|---|---|
| Version | `Get-AppxPackage` shows N+1 and the same package family name |
| Projects | Every project opens with all content, including the image and the comment. No "newer than this app" message |
| Global Idea Vault | The entries and the image are there |
| Settings and appearance | Theme, density and the other settings are what you set |
| Recent projects | Same entries, same order |
| Notices and licence flags | The data note, tutorial and welcome do not show again. The reminder does not reappear on a day it already showed. A buyer sees no chip. A trial person sees the chip with the correct days |
| Licence state | Same as before the update: trial days not reset, purchase not lost, no lock for a buyer |
| Downloaded AI | The models are still there (same size in `models`), `Download Straft AI` is not asked for again unless the new version names other models, and an AI request works. A model is never deleted by an update. On Windows the AI engine is part of the package and arrives with the new version, so there is no engine to download; only the models live in app data |
| Safety copies | After a schema change, `pre-migration-v*` copies sit inside the project folder (item U5). If they are in the package data folder, an uninstall would remove them: that is a finding |
| Derived data | The search and AI index may rebuild in the background with the "Preparing project context" notice. Opening and editing never wait for it |
| Log | `%LOCALAPPDATA%\Packages\<family>\LocalState\Straft\logs` has today's log and no startup error |

7. Uninstall N+1 and check item U9 for Windows: the projects and the Global Idea Vault remain in Documents, the package data
   folder is gone (settings and downloaded AI files go with it).

## Staged rollout and fixing forward (item U10, owner step)

- Each update goes out with Partner Center's gradual rollout: publish to a small share of people first, watch support mail and the
  Store health data, then raise the share. Use the option names Partner Center shows on the day.
- A person cannot downgrade a Store update. A bad update is fixed forward: build N+2 with a higher version and submit it. Do not
  try to roll N back.
- The gates before any update is uploaded: `node scripts/check-migrations.mjs`, `node scripts/release-validate.mjs --store`
  (the version must rise, the identity must match `packaging/msix/submissions.json`), and this simulation on the release
  candidate.
- After each submission the owner (or the session that submits) records the uploaded version in
  `packaging/msix/submissions.json` and, after the first one, the identity.

## What the tests prove (items U1 to U11)

Each row names the test or the manual step. "Done" means it runs in the normal suite or script check. Paths are under
`crates/straft-application/tests/platform/` unless another place is given.

| Item | What proves it | State |
|---|---|---|
| U1 One version source | `scripts/tests/release-placeholders.test.mjs` ("the version the package is built with equals Cargo.toml"); `release-validate` check 1; `build-msix` stops when `tauri.conf.json` differs from `Cargo.toml`. The source is `Cargo.toml [workspace.package].version` ([23-release-migration.md](../engineering/23-release-migration.md) section 1) | Done |
| U2 Update simulation | Automated: `update_from_release_baseline.rs` (`update_from_release_baseline`) over `tests/fixtures/release-baseline-1/` (project 19, vault 4, app 9, old `notices.json`). Made by the ignored `regenerate_release_baseline_1`. Manual: the steps above | Automated half done; manual run waits for two real builds |
| U3 Names that never change | Earlier slice: `packaged_data_dir_golden_path` (`apps/desktop/src-tauri/src/distribution.rs`), identity record and compare in `packaging/msix/submissions.json` with `check-release-placeholders` | Done (earlier slice) |
| U4 Small files survive bad input | The table below; `config_files.rs`, `straft-ai/tests/bundled.rs`, `straft-ai` unit tests, notices and library tests | Done |
| U5 Safety copies beside the data | `migration_fixtures.rs` `safety_copies_sit_beside_the_data_and_never_in_app_data`; `update_from_release_baseline` checks the copy when a later build migrates the baseline | Done |
| U6 Update while the app runs | Clean close: `close_gate.rs` unit tests, end-to-end `04-close-keeps-typing`. Cut process: `forced_kill.rs` `m1_03_*` and `u6_after_a_forced_kill_both_recovery_answers_work_on_the_killed_files`, end-to-end `26-forced-kill-keeps-work` (run by the lead, not by this slice) | Rust done; end-to-end specs exist, not run here |
| U7 Downloaded AI files | `install.rs` unit tests (`an_app_update_with_other_pins_removes_no_model_until_the_new_set_is_healthy`, preflight and activation tests), `fetch.rs` resume tests, `bundled.rs` (`damaged_small_records_never_stop_ai_and_are_kept_as_bad`, `a_missing_file_turns_ai_off_with_a_plain_message`); notes below | Done |
| U8 Derived data never blocks opening | `tests/ai/ai_retrieval.rs` `u8_opening_and_editing_never_wait_for_a_missing_or_stale_index`. The search index (`search_doc`) is rebuilt inside the open after a migration, see the note below | Done for the assistant index; search index measured, not moved |
| U9 Uninstall wording | Earlier slice (the uninstall sentence) | Done (earlier slice) |
| U10 Staged rollout | The section "Staged rollout and fixing forward" above (owner step) | Written; owner step |
| U11 Time bombs | The list below | Done, nothing in the Windows code expires |

### Small files the app keeps (U4)

For each file: missing fields take defaults, unknown fields are ignored, a damaged copy never stops the app, and the bad copy is kept
(`*.bad` or a named copy) before anything replaces it. A truncated copy and a garbage copy each have a test.

| File | Behaviour on bad input | Test |
|---|---|---|
| `app.sqlite` (settings, appearance, recents) | Not a database or failing its quick check: kept as `app.sqlite.bad`, a new one is made, the app starts with default look and an empty recent list. A newer version is refused, not reset | `config_files.rs` (truncated and garbage; newer version) |
| `entitlement/notices.json` | Empty record, screens may show again, copy kept as `notices.json.bad`; unknown fields ignored, missing fields default, `version` field | `notices.rs` unit tests (truncated, garbage), `entitlement.rs`, and the old-shape file in `update_from_release_baseline` |
| `entitlement/team-gate.json` (written by builds before 2026-11-05 only) | Nothing reads it any more (ADR-0043). It is left in place and does no harm, whatever it holds | none needed |
| `library.json` | Unreadable or newer file is never overwritten; copy kept as `library.json.unreadable-<time>`; a bad folder in it is dropped, the identity stays | `storage.rs` (truncated and newer) |
| Library pointer file | Ignored, the app falls back to `app.sqlite`; copy kept as `<file>.bad` | `config_files.rs` (truncated and garbage) |
| `ai/bundle-check.json` | Reads as missing, the AI files are checked again; kept as `.bad` | `bundled.rs` |
| `ai/graphics.json` | Reads as the default (Automatic); kept as `.bad`; missing fields default | `bundled.rs` |
| `engines.json` | Reads as empty; kept as `.bad` | `engines.rs` unit test, `bundled.rs` |
| `models/active.json` | A pointer that does not parse is reported as damaged AI files with plain words; nothing is deleted until the person removes Straft AI and downloads again. A pointer naming an unsafe folder is ignored | `install.rs` unit tests |
| Mac cache of the Store answer | Mac only, outside this slice | Mac track |

### Downloaded AI on Windows (U7)

- The engine (llama.cpp) is part of the package and is replaced with the app. The package file's step "download the new engine beside
  the old one" does not apply on Windows. The models are the only downloaded part and live in `models` under app data.
- An update never removes a model. A new app version that pins other models sees the old set as "not installed" and asks for the
  download; the old set stays until the new set is downloaded, checked against the new pins and has passed its health check.
- The free space is checked before a download (`preflight`, the rest of the download plus 512 MB), and a download resumes where it
  stopped (`.partial` files, range requests; a host without ranges restarts cleanly). A failed or missing download leaves AI off
  with a plain message and everything else works.
- Watch item: the download uses the certificate roots compiled into the app (`rustls` with built-in roots). Roots in a build do not
  change by themselves; a release built long after its dependencies were pinned may lack a newer certificate authority of the
  model host. Refresh the dependencies before each release. Nothing else in the app uses the network.

### Search index on open (U8)

The assistant's index (`cache/intelligence.sqlite`) is rebuilt in the background and never blocks opening or editing (tested). The
text search index (`search_doc`, inside the project database) is different: after a migration, or when it is empty, the open
rebuilds it before it returns (`project.rs`). The package file says this should be background work. Measured on the large stress
project of the performance suite (3,743 documents, debug build): 0.5 seconds. It is kept as it is because search must be complete
the moment the project shows; revisit if a project ever needs more than a second.

### Time bombs (U11)

Searched the Rust and TypeScript source outside tests for build dates, install dates, hard-coded years, expiry logic and
certificate dates.

- No licence file dates, no install date and no build date are read. The trial and the purchase come from the Store.
- `CalendarDay` accepts years 2000 to 9999. The 30-day Recently Deleted rule, the staging-file age and the project-lock age are
  relative to now.
- The MSIX test certificate made by `build-msix --dev` is reused only while more than a day of validity is left; otherwise a new one is made.
- `Copyright 2026` in About and in the sample files is a fixed year text, not logic. It will read old in 2027; change it when the year changes.
- `APP_STORE_RELEASE_PLACEHOLDER_MS` exists only in the Mac track (`StraftMac/services/licence-server`, to be retired with the
  Mac server). The Windows code has nothing like it.
- Watch item: the certificate roots for the model download (see above).

## Run record

| Date | N | N+1 | Route (Store or local) | Tester | Result and notes |
|---|---|---|---|---|---|
| (none yet) | | | | | |
