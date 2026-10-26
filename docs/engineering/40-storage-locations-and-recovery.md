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
- A file that does not parse, has a user id that is not a UUID, or names a relative or protected path is ignored
  (logged), never trusted and never fatal. `projectsDir` and `vaultDir` are optional; missing means the default.
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
| `project.move` | Moves the open project folder to another parent folder | Copy, verify (integrity check, same project id, every file size), switch Recent Projects, reopen, then delete the source; on any failure the source stays and is reopened |
| `project.orphan_notes` / `project.adopt_orphan_notes` | Lists private-note owners that are not members of the project and not the current person; the Owner may take over one owner's notes | Explicit, one owner at a time, Owner only, writes an activity entry, removes the matching undo entries as package import does |
| `vault.backup` | `.ofvault` backup: the whole vault database (Recently Deleted included) and every asset | Consistent database copy (online backup); manifest with sizes and SHA-256 |
| `vault.export_package` | `.ofvault` package for another computer: as the backup but without Recently Deleted items | Same format, `kind: "package"` |
| `vault.restore` | Replaces the current vault with a backup or package | Validates zip paths, manifest, hashes, database integrity and schema; extracts to a staging folder; keeps the current vault as `Global Idea Vault (before restore <date>)` beside it; rolls back on failure |
| `vault.move` | Moves the vault to another folder | Copy to staging, verify, switch, then delete the source; refuses a non-empty destination, the same folder or a folder inside the vault |
| `vault.locate` | Connects an existing vault folder | Must contain a `vault.sqlite` that passes the integrity check; nothing is moved or overwritten |

### `.ofvault` format 1

A zip archive:

```
manifest.json   { "format": 1, "kind": "backup" | "package", "appVersion": "...", "createdAt": ms,
                  "schemaVersion": n, "files": [{ "path": "vault.sqlite", "size": n, "sha256": "..." }, ...] }
vault.sqlite
assets/...
```

Limits on read: at most 100,000 entries and 20 GB uncompressed; every entry path must be relative, without `..`,
drive letters or absolute roots; every file must be listed in the manifest with the right size and hash.

## 5. Interrupted operations

Staging folders are named `<target>.openframe-staging-<millis>`. A staging folder found at start is from an
interrupted move or restore; it is never used as live data and is removed on the next start (best effort). The live
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
