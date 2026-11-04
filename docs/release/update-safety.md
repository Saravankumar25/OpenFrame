# Update safety on the Mac App Store

Owner package 05, section B (`Straft_Release_Package/05_UPDATE_SAFETY_AND_STANDARDS.md`). The question: when a new version
reaches the Mac App Store and a person updates, does the app stay intact and does their data stay intact? Status as of
2026-11-03. "Gate" means a script or test that fails the build; "manual" means a written step a person runs.

| Item | What it protects | Mac status | Evidence |
|---|---|---|---|
| U1 One version source | The version number rises on every upload | Done | Public version is the workspace version in `Cargo.toml`. `CFBundleVersion` is the commit count. `build-release-app.sh --store` refuses a number that did not rise (`scripts/check-release-placeholders.mjs --build-number`, last number in `apps/macos/packaging/store-identity.json`) |
| U2 Update simulation | A new build opens what the last release wrote | Done for release 1; manual run below | `tests/fixtures/release-1/` (frozen), test `update_simulation_opens_release_1_project`. Each release adds a fixture folder |
| U3 Names that never change | Bundle id and product ids stay the same, so the container survives | Done | Golden test `scripts/tests/store-identity.test.mjs` (bundle id, product ids, Keychain names, sandbox rights) |
| U4 Config files survive bad input | A cut-short or garbage settings file never stops the app | Done | `tests/platform/config_files.rs` (7 files, 3 corruptions each, plus the recovery marker). `NoticeStore::load` keeps an unusable record as `notices.json.bad`. The project manifest tolerates missing version fields (test `a_manifest_without_version_fields_still_reads`). `library.json` is copied to `library.json.unreadable-*` |
| U5 Safety copies live with the data | Deleting the app does not delete the copy that protects the work | Done | `Store::open` is called with `<project>/backups` (project) and `<vault>/backups` (Global Idea Vault). Tests `migration_fixtures.rs` read the `pre-migration-v*` copy from `root/backups` and `vault_dir/backups` for every old version |
| U6 Update while the app runs | The store closes the app; the close keeps every edit | Done in the core; Mac end to end test in progress | `applicationShouldTerminate` answers `.terminateLater`, runs `flushAll`, then `close_current` (checkpoint, marker cleared). `forced_kill.rs` `m1_03_*` proves a kill during a save loses no committed edit and offers recovery. A Mac end to end variant is being added separately |
| U7 AI engine and models | Update never removes downloaded models or breaks the AI | Done in code; real signed build not yet run | The engine ships in the app. Models live under app data (`models/`). Free space is checked before a download (`model_download.rs`, `free_bytes`). Downloads resume (`fetch.rs`, test `an_interrupted_download_resumes_where_it_stopped`). No code deletes models on update: `sweep_sets` removes only inactive sets. The store build checks the engine by code signature (`code_signature.rs`, `engines.rs` tests) |
| U8 Derived data never blocks opening | A changed search index never delays opening a project | Done | `project.rs` `start_search_rebuild`: the rebuild runs on a background thread after open. `SEARCH_INDEX_VERSION` (kept per project in `sys_settings`) triggers a rebuild when the indexer changes without a migration. Test `the_search_index_rebuilds_after_open_when_the_indexer_version_changes`. The thread emits the module event `search` / `indexed` when it ends |
| U9 Uninstall wording | People know to back up the root folder | Done | `LicenceRules.dataPoints` tells the person to back up the root folder, and before deleting Straft Studio from the Mac. Nothing is promised about the container after deletion |
| U10 Staged rollout | A bad update reaches few people first | Owner step | Turn on phased release in App Store Connect if it is offered for the Mac. A store update cannot be downgraded: a bad update is fixed forward. Keep `check-migrations` and the update simulation as release gates |
| U11 No time bombs | Nothing expires because of a build date or a year | Done | Scan of 2026-11-03 found no behaviour that depends on the build date, install date or a fixed year |

## Manual run: install over the previous build

Run before each submission, on a clean Mac (or a new user account), with build N from the store and build N+1 as the candidate.

1. Install build N. Open it, finish the first-run steps (library folder, name). Note the folder chosen.
2. Create a project with real content: a screenplay with a few scenes, two story characters, a budget line, a note, one
   attached file. Change the appearance. Close one project and keep a second one open.
3. Download the AI models (Settings, Straft AI) and ask one question. Confirm it answers.
4. Let the trial run, or buy with a StoreKit test account. Note the licence state on the About screen.
5. Quit the app while a project is open and an edit was typed a second ago.
6. Install build N+1 over build N (TestFlight update, or the Mac App Store update). Do not delete anything.
7. Open the app. Confirm: the library and projects are where they were; the open project reopens; no recovery prompt appears
   after a normal quit; appearance, recents and the notice flags (welcome, data note, tutorial) are unchanged; the licence state
   is the same; search finds the note within a few seconds; the AI answers without a new download; the models folder is intact.
8. If the new build changed the project schema, look in the project's `backups/` folder for one `pre-migration-v*` copy, and
   confirm opening the project took no longer than before.
9. Kill the app (Activity Monitor, Force Quit) during an edit, reopen, and confirm the recovery offer returns the edit.
10. Record the build numbers, the date and the result in `docs/release/evidence/`.

## Last run

2026-11-03, internal builds on the owner's M1: `apps/macos/E2E/update-check.sh` from build 183 (commit of 2026-10-28) to the installed build
225 (`a46790ac`), 4 of 4: Story page text, saved text after a kill, Command-Q with a sheet open, every piece of work after a kill. The same
install on the owner's real data: `app.sqlite` went from version 9 to 11, integrity check ok, 2 of 2 recents and their folders kept, models
kept, process up, no warnings or errors logged. The bundle identifier changed from `studio.straft.mac` to `app.straftstudio.studio`,
so preferences macOS keeps per identifier start fresh once. The manual run above on a store build has not been done.

## Adding a release fixture

After a release ships, copy `generate_release_1_fixture` to `generate_release_N_fixture`, run it once on that release's build,
commit `tests/fixtures/release-N/`, and add a test that opens it. Never change an older fixture. The README in
`crates/straft-application/tests/fixtures/release-1/` explains the layout.
