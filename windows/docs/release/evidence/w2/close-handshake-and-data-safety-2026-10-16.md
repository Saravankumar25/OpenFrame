# Windows phase W2 evidence: protect writers' work (2026-10-16)

Proof level 2 (debug build on the development computer, ADR-0030). Branch `windows/w1-controls`, commit a28761a.

## Close within 100 ms keeps the typed text

| Run | Build | Result |
|---|---|---|
| Before the fix | debug build of commit 54733f7 | **Failed** in round 1: "Round 1 ends by the close button." was lost when the window closed |
| After the fix | debug build with `close_gate.rs` and `app/pendingSaves.ts` | **Passed** 20 rounds × 2 (close button, then Alt+F4), 40 closes in a row, every sentence found after reopening (`tests/e2e/specs/04-close-keeps-typing.test.mjs`, `E2E_CLOSE_ROUNDS=20`) |

The close was sent within 100 ms of the last key in every round (the spec asserts it). The app log shows the handshake
for each close: "window close requested; held for the page", then the page steps "flushing" and "close", 20 to 40 ms
apart. Two earlier runs failed in later rounds for a reason in the test, not the app: the click put the caret inside
a long line and Enter split it, so the sentence was stored inside another line (the database held every word). The
spec now presses End before Enter and makes each new line an Action (Ctrl+2).

## Other W2 items

| Item | Test | Result |
|---|---|---|
| Cloud-synced folders (fake OneDrive root, Dropbox marker, Google Drive, iCloud names, environment-named OneDrive) | `openframe-platform` `cloud::tests` (4), `platform/project_lifecycle.rs::a_new_project_in_a_fake_onedrive_folder_is_flagged_and_existing_projects_stay`, `NewProjectDialog.test.tsx` | Pass |
| Documents folder unknown: refuse to start, never a temp folder | `project_lifecycle.rs::the_app_refuses_to_start_without_a_documents_folder` | Pass |
| Project delete and locate | `project_lifecycle.rs` (4 tests); `10-home.test.mjs` "Delete Project" step | Pass; a deliberate break (delete skips the move, locate skips the identity check) made 2 of them fail, then restored |
| Pre-migration backup opens with the old reader and equals the fixture | `migration_fixtures.rs::m1_04_every_project_schema_version_upgrades_with_content_backup_search_and_export` | Pass; a deliberate break (backup taken after the migration) failed on v1 |
| A failed replace names the hidden `.replaced-` folder | `packages.rs::a_failed_replace_that_cannot_put_the_project_back_names_the_hidden_folder` | Pass |
| Manual OneDrive typing test | `docs/testing/onedrive-typing-test.md` | Written; **not run yet** (owner, on a laptop with OneDrive) |
