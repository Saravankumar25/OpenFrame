# Storage locations, relocation, backup and identity recovery

Owner request 2026-10-27 (the "Storage, Relocation, Backup, and Identity" specification, kept with the request).
Decision record: [ADR-0038](../adr/ADR-0038-library-file-and-durable-identity.md). This page is the
design and the contract the code follows; the code is the ground truth.

## 1. What lives where

| Kind | Default place (Windows) | Removed by uninstall? |
|---|---|---|
| Application binaries | `%LOCALAPPDATA%\Programs\OpenFrame Studio` | Yes |
| Technical app data (settings, recents, AI records, logs, models) | `%LOCALAPPDATA%\OpenFrame` (MSIX: the package `LocalState\OpenFrame`) | Only with the "delete app data" box (direct installer); always for MSIX |
| Library folder (the one folder to back up) | chosen on the first start; suggested `C:\Users\<name>\OpenFrame` | Never |
| Library pointer (where the library is) | `%APPDATA%\OpenFrame\library-pointer.json` (Roaming) | Never (the uninstaller only removes listed items under `%LOCALAPPDATA%\OpenFrame`) |
| Library file (identity and chosen places) | `<library>\library.json` | Never |
| Default folder for new projects | `<library>\Projects` (changeable) | Never |
| Global Idea Vault | `<library>\Global Idea Vault` (movable) | Never |
| Each project | wherever it was created or moved (`<Title>.openframe` folder) | Never |
| Backups and packages | wherever the person saves them | Never |

Earlier versions kept the library in `Documents\OpenFrame`. On the Mac the suggested library folder is `~/OpenFrame`
(inside the app container under the App Sandbox of the store build, so it follows the container's lifecycle there;
see section 7).

### The library and the first start

The library is one folder that holds `library.json`, `Projects\` and `Global Idea Vault\`. A projects folder or a
vault elsewhere stays supported (the library file names them). **Nothing is created before the person confirms a
choice**: no folder, no vault, no projects folder, no library file, and no saved profile. Until then the core runs
only `library.*`, `team_gate.*`, `entitlement.*`, `appearance.*`, `app.info` and `app.save_state`; every other
operation answers `library.setup_required`, the vault does not open, start maintenance does nothing, and the shell
refuses to reveal the data folder.

The chosen folder is remembered in `app.sqlite` (`sys_settings` key `storage.library_root`) and in the pointer file,
both written only after the choice worked. At start the app uses `app.sqlite`, else the pointer (a reinstall); when
one of the two is missing it is written again. The library is:

- **Ready** when the remembered folder exists. Normal starts never ask again.
- **NeedsSetup** when nothing is remembered. `found` lists existing libraries at the suggested folder and at
  `Documents\OpenFrame` (the Known Folder, which may be redirected to OneDrive, plus `<home>\Documents\OpenFrame` and
  `<home>\OneDrive\Documents\OpenFrame`), each with real counts. An install from before this change that already has
  `Documents\OpenFrame` lands here once, with that library listed first.
- **Missing** when the remembered folder is not there (a drive that is not connected, a moved folder). It is never
  recreated empty; the screen says where it was expected and offers Try Again, Locate, Choose new and Use default.

The screen "Set up your OpenFrame library" shows at app start, before Home, whenever the state is not
Ready. It cannot be dismissed; closing the window closes the app. Three choices: **Use the default location** (shows
`<home>\OpenFrame` and why it is outside OneDrive; Create Library Here makes it), **Choose a new location** (a folder
picker; the library is `<picked>\OpenFrame`, shown before Create Library, with a warning when the folder is synced),
**Locate existing OpenFrame data** (a folder picker, then a preview of what was found; a wrong level, a single project
or a lone vault is explained; Connect Library only when a library was identified; a failure stays on the screen).

| Operation | What it does | Safety |
|---|---|---|
| `library.status` → `LibraryStatus` | `state` (`ready`, `needsSetup`, `missing`), `root` (in use, or expected), `suggested`, `cloudService` of the library in use, `found: LibraryInspection[]` | Read only; works before setup |
| `library.inspect {dir}` → `LibraryInspection` | `kind` (`libraryRoot`, `libraryParent`, `insideLibrary`, `project`, `vault`, `empty`, `other`), `libraryRoot` when it can be identified safely, `hasLibraryFile`, `ownerName`, `projectCount`, `vaultFound`, `ideaCount`, `cloudService`, `message` | Never writes: the vault is opened read-only and immutable, so no `-wal` or `-shm` appears (changes still in a crashed vault's `-wal` are not counted) |
| `library.use_default` | Sets up `<home>\OpenFrame`: connects it when it already is a library, else creates it | Refuses when a library is set up (Settings moves or locates instead), and a folder that already holds something |
| `library.create_at {parent}` | New library at `<parent>\OpenFrame` | Parent must be absolute, local, writable, outside the app data and outside any project or library. Refuses a destination that holds a library (`conflict.exists`, use Locate) or anything else. A synced folder is allowed; `cloudService` in the answer and the inspect preview warn. What it created is removed again if setup fails |
| `library.connect {dir}` | Connects the library `library.inspect` identifies for `dir` (any level) | Never creates or replaces anything but `library.json`; a folder with no library is `not_found.library` with the inspect message; a vault in it must pass the vault check; takes the storage slot |
| `library.move {destParent}` → task `storage.library_move`, `LibraryMoveReport` | Moves the whole library to `<destParent>\<name>` | See below |

**Identity when a library is set up or connected.** `app.sqlite` keeps a saved profile (ADR-0038). A person with no
saved profile yet (a new computer, or a reset app data: before setup the id lives in memory only) takes over the id
and name in the connected library's file, so their private notes stay theirs. When a saved profile meets the file of
another person, that file is first copied to `library.json.other-person-<ms>`, so their id is not lost, and the file
is rewritten with the profile in use. Nothing in a project is reassigned.

**Moving the library.** Refuses the same place, a destination inside the library or the library inside it, a
non-empty destination, the app data, a project, a folder that is not writable and not enough space. While it runs no
project can be opened, created or closed. It closes the open project when that lives in the library, keeps the vault
closed, copies the whole tree (every project inside, the vault with its safety backups, anything else; a project's
`.lock` and `cache\` are left out as in a project move) into a staging folder, checks every file (size and SHA-256),
every project database and identity and the vault database, renames it into place, then switches: the pointer and
`app.sqlite`, then `library.json` and the places, then Recent Projects for projects that were inside. The folder for
new projects becomes the moved library's `Projects` (always, owner decision 2026-10-27, also when another folder had
been chosen); a vault inside the library follows it, a vault or project outside stays where it is, and a folder picked
in New Project is still used for that project. The open project is reopened from its new place and the vault opened;
if either fails every record is switched back and the copy deleted. Only then is the old folder removed (only what
was copied; `sourceLeftAt` names it when something else was in it). Any failure before the switch keeps the old
library live and removes the staging folder.

Settings > Storage, "Your OpenFrame library": Open Folder, Move Library… (the task dialog of the vault move, with
both paths and what moves) and Locate Existing Library… (the inspect preview, then Connect).

## 2. The library file and the durable identity

The local user id used to live only in `app.sqlite` (`sys_local_profile`). Deleting the app data (uninstall box,
troubleshooting) made a new id, and private notes in projects, owned by the old id, disappeared from view.

Now the library file holds the identity and the chosen places:

```json
{ "format": 1, "userId": "0192...", "displayName": "Meera", "projectsDir": "D:\\Films", "vaultDir": "D:\\Films\\Global Idea Vault", "updatedAt": 1793000000000 }
```

- Written atomically (`openframe_platform::atomic`) at start (once the library is set up), after a name change, after
  every location change and when a library is set up, connected or moved.
- Read at start before the profile is created, and again when a library is set up or connected (section 1). When
  `app.sqlite` has no profile, the user id and name of a valid library file are reused (the same person after a
  reinstall or an app-data reset), so their private notes stay theirs. When `app.sqlite` already has a profile, it
  wins and the file is rewritten.
- Reading has three outcomes: absent (the file is created), valid, and unusable. A file that does not parse (a
  half-written file), has a newer `format`, a user id that is not a UUID, is too large or cannot be read (a read
  error that may pass) is unusable. An unusable file is never trusted and never fatal, and it is never overwritten
  by the start: it may still hold the person's chosen folders. Start copies it aside to
  `library.json.unreadable-<ms>` (best effort, once per distinct content), logs it and runs with the defaults. The
  file is written again only when the person changes a place on purpose (the copy is made first if it is missing),
  after which it is the normal file. A folder inside a valid file that is relative or protected is dropped on its own.
  `projectsDir` and `vaultDir` are optional; missing means the default.
- The id is never derived from a display name, an e-mail address or the operating system account.
- The id is not authentication. Anyone who can write to the library folder can edit `library.json` and take on
  another person's id, and with it their private notes and role in projects on this computer. The only access control
  is the folder permissions of the operating system (security review 2026-10-27; the owner decides whether this limit
  stays). The team password, once a second lock on the internal builds, was removed on 2026-10-27.
- Tests and the end-to-end data root (`OPENFRAME_DATA_ROOT`) count as set up from the start, with the library at
  their own root (`AppConfig::isolated`). `AppConfig::isolated_needing_setup` and, for a debug build on a test data
  root, `OPENFRAME_LIBRARY_SETUP=1` start like a new computer, with every folder inside the test root.

## 3. Locations at run time

`AppCore` keeps the library in use, the projects folder and the vault folder behind one lock, starting from the
remembered library and its file (else the defaults). Every caller reads them through `core.projects_dir()`, `core.vault_dir()` and `core.data_root()`, so a
change applies at once without a restart. The library's own `Projects` and vault folders are
`core.default_projects_dir()` and `core.default_vault_dir()`; `AppConfig` keeps only the suggestion.

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

**One storage task at a time.** `vault.backup`, `vault.export_package`, `vault.restore_backup`, `vault.move`, `vault.locate`, `project.move`, `library.connect` and `library.move` take one slot in the application core when they start (a guard that is released when the task ends, on success, error or panic). A second one is refused at once with `storage.busy` and a plain message, so a restore can never run against a vault path a move has just left. A move or a restore reads the vault folder again inside the closed-vault section and stops (`storage.busy`) if it is not the one it planned for.

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

- First start: "Set up your OpenFrame library" (section 1).
- Settings > Storage (global): library folder (Open, Move Library, Locate Existing Library), default folder for new
  projects (Change), Global Idea Vault folder with
  Open, Back Up, Export Package, Restore, Move, Locate, and the app data folder. Plain words say that changing the
  default folder does not move existing projects.
- Project Settings: the project folder with Open and Move Project, and the private notes of former owners when there
  are any.
- Home: "Found on this PC" lists projects in the projects folder that are not in Recent Projects.

## 7. Platforms

- Direct installer (NSIS): the uninstall box deletes only listed items under `%LOCALAPPDATA%\OpenFrame`; the library
  folder, projects, vault and backups are never touched (`installer-hooks.nsh`), and neither is the library pointer
  in `%APPDATA%\OpenFrame` (Roaming), so a reinstall opens the same library without asking.
- Microsoft Store (MSIX): Windows removes the package app data on uninstall, and `%APPDATA%` writes are redirected
  into the package, so the pointer goes too; the library file in the library folder survives, and the first start
  finds a library at the suggested folder or in Documents,
  so identity and places come back after a reinstall. Not tested on a real Store install yet (Chapter 3).
- Mac (internal disk image): the first start suggests the library at `~/OpenFrame`, in the home folder and outside
  the iCloud Drive "Desktop & Documents" folders, so iCloud does not sync or offload it. The library pointer is
  `~/Library/Application Support/OpenFrame Library/library-pointer.json`, a folder of its own outside the app data
  folder `~/Library/Application Support/OpenFrame` (`library_pointer_dir()` in `openframe-platform`), so resetting the
  app data keeps the pointer, and the library file in the library keeps the identity even if both go. Dragging the app
  to the Trash removes neither. Finder files (`.DS_Store`, `._*`) are skipped in the copy of a move and in the check
  after it, and do not stop an emptied folder from being removed.
- Mac App Store: the home folder resolves inside the app container; the library and its pointer live there and are
  removed with the container. Folders outside the container need security-scoped bookmarks, which are not built. The
  Mac screens therefore hide Choose new location, Locate, Move and Change Folder on the store build and say why
  (`StorageRules.canRelocate` in `apps/macos/Sources/OpenFrameUI/Storage/`). Not tested on a sandboxed store build.
