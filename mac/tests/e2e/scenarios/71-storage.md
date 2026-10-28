# Storage

Spec: `tests/e2e/specs/71-storage.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own, with an empty `vault-out` folder and an empty `moved-projects` folder inside it. The app starts for the first time.

The backup file is `Global Idea Vault backup.ofvault` in `vault-out`. "Wait for the task" means: the operation returns a task, the spec listens to the application's task events (the same ones the screen follows), and the task must end as completed within 60 seconds. The long storage operations are asked of the core with folders inside the data folder, so no system folder dialog is needed.

## Settings > Storage names the five places and the vault actions

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Harbour Ledger' as the title, click Create.
2. Open the project menu and choose Project settings. Click the Storage tab.
3. Check: the screen shows the sections "Your OpenFrame library", "Folder for new projects", "Global Idea Vault", "This project" and "App data".
4. Check: the buttons of the Global Idea Vault section are, in order, "Open Folder", "Back Up…", "Export Package…", "Restore…", "Move…", "Locate Existing Vault…".
5. Check: "Restore…" and "Locate Existing Vault…" can be clicked (the other four need a vault that exists).
6. Check: the buttons of the "This project" section include "Open Folder" and "Move Project…".
7. Check: the screen is quiet.

## An idea in the Global Idea Vault is backed up, and the backup is inspected

1. Through the core: add the note 'Tide tables for the harbour wall' to the Global Idea Vault.
2. Check: the places report says the Global Idea Vault exists.
3. Through the core: back up the vault to the backup file. Wait for the task.
4. Check: the report counts at least one idea.
5. Check: the backup file exists and its first two bytes are "PK".
6. Through the core: inspect the backup file. Check: its kind is "backup" and it holds at least one idea.
7. Click the Appearance tab, then the Storage tab.
8. Check: within 15 seconds every button of the Global Idea Vault section can be clicked.
9. Check: the screen is quiet.

## Move the open project to another folder: it keeps its content

1. Through the core: add the note 'Moved along with the project' to the project's Idea Vault.
2. Through the core: read the places and note the project folder. Check: the folder exists.
3. Through the core: move the project to the folder `moved-projects`. Wait for the task.
4. Check: the report names the project folder inside `moved-projects`, with the same folder name as before.
5. Check: that folder exists, and the old project folder is gone.
6. Check: the places report names the new folder as the open project.
7. Check: the Workspaces navigation still shows, and the current project is "Harbour Ledger".
8. Check: the project's Idea Vault still holds 'Moved along with the project'.
9. Check: the screen is quiet.

## Home lists the moved project under Recent projects, and it opens again

1. Open the project menu and choose Close project.
2. Check: the screen shows "Recent projects" and a card starting with "Open Harbour Ledger".
3. Check: after 1 second, the screen has no "Found on this PC" section.
4. Through the core: ask which projects the projects folder holds that Recent projects does not list. Check: "Harbour Ledger" is not among them.
5. Click Settings, then the Storage tab.
6. Check: the Global Idea Vault section shows, and there is no "This project" section.
7. Press Esc.
8. Click the project row "Open Harbour Ledger". Check: no recovery question shows.
9. Check: the project's Idea Vault still holds 'Moved along with the project'.
10. Open the project menu and choose Close project.
11. Check: the screen is quiet.

## Platform notes

- The system folder dialogs behind Change Folder…, Back Up…, Restore…, Move…, Locate Existing Vault… and Move Project… are skipped: the spec asks the core with folders in the data folder. Windows file dialogs are filled through UI Automation in `70-packages`; the same way can be added here.
