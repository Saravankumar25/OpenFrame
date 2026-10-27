# Storage locations, relocation, backup and identity recovery

Owner request 2026-10-27 (the "Storage, Relocation, Backup, and Identity" specification, kept with the request).
Decision record: [ADR-0038](../adr/ADR-0038-library-file-and-durable-identity.md). This page is the
design and the contract the code follows; the code is the ground truth.

## 1. What lives where

| Kind | Default place (Windows) | Removed by uninstall? |
|---|---|---|
| Application binaries | `%LOCALAPPDATA%\Programs\OpenFrame Studio` | Yes |
| Technical app data (settings, recents, AI records, logs, models) | `%LOCALAPPDATA%\OpenFrame` (MSIX: the package `LocalState\OpenFrame`) | Only with the "delete app data" box (direct installer); always for MSIX |
| Library folder (the one folder to back up) | `Documents\OpenFrame` | Never |
| Library file (identity and chosen places) | `Documents\OpenFrame\library.json` | Never |
| Default folder for new projects | `Documents\OpenFrame\Projects` (changeable) | Never |
| Global Idea Vault | `Documents\OpenFrame\Global Idea Vault` (movable) | Never |
| Each project | wherever it was created or moved (`<Title>.openframe` folder) | Never |
| Backups and packages | wherever the person saves them | Never |

On the Mac the library folder is `~/Documents/OpenFrame` (inside the app container under the App Sandbox of the
store build, so it follows the container's lifecycle there; see section 7).

## 2. The library file and the durable identity

The local user id used to live only in `app.sqlite` (`sys_local_profile`). Deleting the app data (uninstall box,
troubleshooting) made a new id, and private notes in projects, owned by the old id, disappeared from view.

Now the library file holds the identity and the chosen places:

```json
{ "format": 1, "userId": "0192...", "displayName": "Meera", "projectsDir": "D:\\Films", "vaultDir": "D:\\Films\\Global Idea Vault", "updatedAt": 1793000000000 }
```

- Written atomically (`openframe_platform::atomic`) at start, after a name change and after every location change.
- Read at start before the profile is created. When `app.sqlite` has no profile, the user id and name of a valid
  library file are reused (the same person after a reinstall or an app-data reset), so their private notes stay
  theirs. When `app.sqlite` already has a profile, it wins and the file is rewritten.
- Reading has three outcomes: absent (the file is created), valid, and unusable. A file that does not parse (a
  half-written file), has a newer `format`, a user id that is not a UUID, is too large or cannot be read (a read
  error that may pass) is unusable. An unusable file is never trusted and never fatal, and it is never overwritten
  by the start: it may still hold the person's chosen folders. Start copies it aside to
  `library.json.unreadable-<ms>` (best effort, once per distinct content), logs it and runs with the defaults. The
  file is written again only when the person changes a place on purpose (the copy is made first if it is missing),
  after which it is the normal file. A folder inside a valid file that is relative or protected is dropped on its own.
  `projectsDir` and `vaultDir` are optional; missing means the default.
- The id is never derived from a display name, an e-mail address or the operating system account.
- Tests and the end-to-end data root (`OPENFRAME_DATA_ROOT`) use their own library file inside their own root.

## 3. Locations at run time

`AppCore` keeps the projects folder and the vault folder behind a lock, starting from the library file (else the
defaults). Every caller reads them through `core.projects_dir()`, `core.vault_dir()` and `core.data_root()`, so a
change applies at once without a restart. `AppConfig` keeps only the defaults.

### Lazy vault

The vault is opened on first use, not created at start: the start maintenance (purge of Recently Deleted) runs only
when `vault.sqlite` already exists. A vault folder that the library file names but that is missing (a drive that is
not connected) is never recreated empty: opening it fails with `storage.vault_missing` and Settings > Storage offers
Locate and Change. Only the default vault folder is created on first use.

## 4. Operations

| Operation | What it does | Safety |
|---|---|---|
| `storage.info` | All places above, whether each exists, the library file, the current project folder | read only |
| `storage.set_projects_dir` | Changes the default folder for **new** projects | Never moves a project; folder must be writable and outside OpenFrame's own folders |
| `project.discover` | Projects in the projects folder (two levels) that are not in Recent Projects | read only |
| `project.move` | Moves the open project folder to another parent folder. A background task (`storage.project_move`) | Needs Manage Project. While it runs no project can be opened, created or closed (`storage.busy`). Copies everything except `.lock` and `cache/` into `<name>.openframe-staging-<ms>` beside the destination, checks every file (size and SHA-256), the database integrity check and the project id, renames it into place, switches Recent Projects, reopens the project there, checks that the open project really is the one at the new place (otherwise the old folder is kept and the copy is removed), then removes the source. On any failure before the switch the staging folder is removed and the project is reopened at its old path. If the project will not open in its new place, the copy is deleted only after Recent Projects name the old folder again; if that fails too, both folders are kept and the error names them (`storage.rollback_failed`). Linked files (absolute paths) are not touched; the result counts them (`linkedFiles`). Refuses the same parent, an existing folder of that name (`conflict.exists`), a folder inside the project, the app data or the vault, a folder that is not writable, and a drive without enough free space |
| `project.orphan_notes` / `project.adopt_orphan_notes` | Lists private-note owners that are not members of the project and not the current person; the Owner may take over one owner's notes | Explicit, one owner at a time, Owner only, writes an activity entry, removes the matching undo entries as package import does |
| `vault.backup` | `.ofvault` backup: the whole vault database (Recently Deleted included) and every file under `assets/`. A background task (`storage.vault_archive`) | Consistent database copy (online backup); never `backups/`; refuses when there is no vault yet (`not_found.vault`) |
| `vault.export_package` | `.ofvault` package for another computer. Same task | Same format with `kind: "package"`. Drops everything Recently Deleted holds: the deleted items, folders and collections, the files only they used, the undo history and the activity log; a linked file keeps only its file name (its full path names folders on this computer); the search index is rebuilt and the copy vacuumed so the removed text is not left in the file |
| `vault.inspect` | What a `.ofvault` file holds, for the screen before a restore (kind, date, app version, idea count, file count, size) | Read only; checks the structure, the manifest and that the checksum list matches the entries. Restore checks every hash |
| `vault.restore_backup` | Replaces the current vault with a backup or package. A background task (`storage.vault_restore`). Named `vault.restore_backup` because `vault.restore` already brings one item back from Recently Deleted | Everything is validated in `<vault>.openframe-staging-<ms>` beside the vault first (see the format below). Then the vault is closed, the live folder is renamed to `<vault name> (before restore YYYY-MM-DD HHMM)` (made unique when it exists), the staging folder is renamed to the vault path and the vault is opened (and upgraded when older). If it does not open, the restored folder is renamed aside, the old folder is put back, and only then is the restored one deleted; a step that fails stops there, deletes nothing, and the error (`storage.rollback_failed`) names the folder that holds the earlier vault. Before the vault is opened the unpacked files must be exactly the ones `checksums.json` lists plus the manifest and the list itself. With no vault yet it is simply installed. Returns `previousKeptAt` |
| `vault.move` | Moves the vault to another folder (the folder keeps its name). A background task (`storage.vault_move`) | The vault is closed and kept closed for the whole copy, so nothing can be written to the old files meanwhile. Copies everything except `backups/` to staging, checks every file and the database, renames it into place, writes the library file, opens the vault there, then removes the old files (only what was copied). The safety backups in `backups/` are never copied and never deleted by a move: they stay in the old folder, which is reported as `sourceLeftAt` (so is a folder that still holds anything else). If the vault will not open in its new place, the places are switched back first and the copy is deleted only after that worked (otherwise both folders are kept and the error names them). Refuses the same place, a non-empty destination (`conflict.exists`), a folder inside the vault, the app data or the open project, a folder that is not writable and not enough free space. On failure before the switch the old vault stays live |
| `vault.locate` | Connects a folder that already holds a vault. Immediate, not a task | Needs `vault.sqlite` that passes the integrity check, has the vault tables, is not newer than this version and carries no code OpenFrame does not create. Refuses a project folder. Nothing is moved, copied or created; only the library file changes. If the vault does not open there, the old place comes back |

**One storage task at a time.** `vault.backup`, `vault.export_package`, `vault.restore_backup`, `vault.move`, `vault.locate` and `project.move` take one slot in the application core when they start (a guard that is released when the task ends, on success, error or panic). A second one is refused at once with `storage.busy` and a plain message, so a restore can never run against a vault path a move has just left. A move or a restore reads the vault folder again inside the closed-vault section and stops (`storage.busy`) if it is not the one it planned for.

**While the vault is closed for a move or a restore,** the vault lock is held for the whole copy. A call that needs the vault (a screen query) waits at most two seconds and then fails with `storage.busy` ("The Global Idea Vault is being moved or restored. Try again in a moment.", retryable) instead of freezing. Nothing reopens the vault while it is closed.

Errors a person can meet: `not_found.vault` (no vault to back up or move yet), `storage.vault_missing` (the chosen folder has no vault: drive not connected), `conflict.exists`, `conflict.same_place`, `not_found.folder`, `storage.not_enough_space`, `storage.busy` (the vault or project is still in use after ten seconds, another storage task or a project move is running, or the vault is closed for a move; nothing changed), `storage.rollback_failed` (an undo did not work; the message says which folder holds the data), `storage.copy_mismatch`, `storage.not_a_vault`, `storage.vault_damaged`, `storage.vault_too_new`, `storage.untrusted_schema`, and for `.ofvault` files `import.not_vault`, `import.vault_incomplete`, `import.vault_checksum_mismatch`, `import.vault_too_new`, `import.unsafe_archive`.

Every long operation returns `{ taskId }`; progress and the result (`VaultArchiveReport`, `VaultRestoreReport`, `VaultMoveReport`, `ProjectMoveReport`) arrive as task events of kind `storage.*`, and the task can be cancelled until the switch. The desktop shell follows the vault and project places for the asset protocol when a `storage.*` task ends and whenever a project opens or closes.

### `.ofvault` format 1

A zip archive written through the same safe container code as packages (`openframe_security::archive`, atomic write):

```
manifest.json    { "format": "openframe-vault", "formatVersion": 1, "kind": "backup" | "package",
                   "appVersion": "...", "createdAt": ms, "schemaVersion": n,
                   "ideaCount": n, "assetCount": n, "missingAssets": ["<asset id>", ...] }
checksums.json   { "algorithm": "sha256", "files": { "vault.sqlite": "<hex>", "assets/...": "<hex>", ... } }
vault.sqlite
assets/...
```

`checksums.json` lists every entry except itself (the manifest included). `ideaCount` and `assetCount` are for the confirmation screen; `missingAssets` names managed files the vault's database listed but the folder no longer had when the file was made, so a backup of a damaged vault can still be restored.

A restore refuses the file when: the archive limits are broken (packages' limits: at most 200,000 entries and 64 GB, 16 GB for one entry, a compression ratio of 200), an entry path is not relative or has `..`, a drive letter or a reserved name, a link entry, any entry other than `manifest.json`, `checksums.json`, `vault.sqlite` and `assets/...` (a `vault.sqlite-wal` would be replayed by SQLite), the format or the schema is newer than this version, the entries do not match `checksums.json`, a hash differs, the database fails `PRAGMA integrity_check`, lacks the vault tables or carries triggers or views OpenFrame does not create, or a managed file the database names is not in the archive (other than `missingAssets`). Linked files (absolute paths to files elsewhere) are never part of the archive.

## 5. Interrupted operations

Staging folders are named `<target>.openframe-staging-<millis>` (the vault restore and the vault and project moves all use them). A staging folder found at start is from an
interrupted move or restore; it is never used as live data and is removed on the next start (best effort). Start looks
in the projects folder and the folder that holds the vault, and also at the one folder named in
`storage-staging.txt` in the app data folder: a running move or restore writes its staging path there before the first
byte is copied and deletes the file when it ends, so a move to any other folder leaves nothing that is not found. Only
a real folder (never a link) with a staging name is removed that way. The live
location changes only after the copy is verified, and the library file is written after the switch, so a crash at
any point leaves the app pointing at a complete vault or project: the old one until the switch, the new one after.

## 6. Settings and screens

- Settings > Storage (global): library folder, default folder for new projects (Change), Global Idea Vault folder with
  Open, Back Up, Export Package, Restore, Move, Locate, and the app data folder. Plain words say that changing the
  default folder does not move existing projects.
- Project Settings: the project folder with Open and Move Project, and the private notes of former owners when there
  are any.
- Home: "Found on this PC" lists projects in the projects folder that are not in Recent Projects.

## 7. Platforms

- Direct installer (NSIS): the uninstall box deletes only listed items under `%LOCALAPPDATA%\OpenFrame`; the library
  folder, projects, vault and backups are never touched (`installer-hooks.nsh`).
- Microsoft Store (MSIX): Windows removes the package app data on uninstall; the library file in Documents survives,
  so identity and places come back after a reinstall. Not tested on a real Store install yet (Chapter 3).
- Mac App Store: Documents resolves inside the app container; the library file lives with the vault there and is
  removed with the container. Folders outside the container need security-scoped bookmarks, which the Mac shell does
  not have yet: moving the vault or a project outside the container is not offered on the Mac until it does. Not
  built or tested in this slice (Mac track 1b).
