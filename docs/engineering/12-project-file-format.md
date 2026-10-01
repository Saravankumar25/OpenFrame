# 12 — Local Project File Format

> **Status legend:** ✅ implemented · 🚧 in development · 📋 planned.
> Source: `crates/openframe-project-format/src/lib.rs`, `crates/openframe-application/src/modules/project.rs`.
> Decision: ADR-0003.

## 1. Project folder ✅

```text
<Title>.openframe/                 folder name = sanitized title; "<Title> (2).openframe" when taken
├─ openframe.json                  manifest (human-inspectable)
├─ project.sqlite                  canonical structured data (see 11-physical-database.md)
├─ project.sqlite-wal / -shm       present only while open
├─ assets/<aa>/<assetId>[.<ext>]   managed media; <aa> = last two hex chars of the id (reversed)
├─ cache/thumbnails/               rebuildable previews (never canonical)
├─ cache/intelligence.sqlite       derived AI index (documents, vectors, context graph); deletable, rebuilt (doc 18)
├─ recovery/
│  ├─ session.json                 crash marker (exists while a session is open)
│  └─ checkpoint.sqlite            last confirmed saved state (explicit Save / clean close)
├─ backups/                        pre-migration-v<N>-<ms>.sqlite, before-recovery-<ms>.sqlite
└─ .lock                           exclusive OS lock held while open
```

- **Title sanitizing** (`openframe_security::sanitize_file_name`): replaces `<>:"/\|?*` and control characters, trims
  trailing dots and spaces and leading dots, caps the name at 120 characters, prefixes Windows reserved names
  (`CON`, `NUL`, `COM1`…) with `_`, and never returns an empty name (`Untitled`).
- **Default parent:** `Documents\OpenFrame\Projects` (the user may choose another local folder in New Project).

## 2. Manifest `openframe.json` ✅

```json
{
  "format": "openframe-project",
  "formatVersion": 1,
  "projectId": "0192f0c1-…",          // UUIDv7, must equal project.id in project.sqlite
  "title": "Railway",
  "createdAt": 1790000000000,          // epoch ms
  "appVersion": "0.1.0",               // last app version that wrote the project
  "schemaVersion": 9,                  // mirrors PRAGMA user_version
  "aiCompatibilityVersion": 1
}
```

Validation on open (`read_manifest`): the manifest must exist and parse, `format` must equal `openframe-project`,
`formatVersion` must be ≤ supported (otherwise `project_format.too_new`, nothing touched), and `projectId` must be a
valid id. After the database opens, its `project.id` must equal `projectId` (otherwise refused). `schemaVersion` and
`appVersion` are rewritten atomically after a successful open or migration. The manifest is written with
`atomic_write` (temp sibling + fsync + rename).

## 3. Assets ✅

| Mode | Stored as | Notes |
|---|---|---|
| `managed` | `asset.rel_path` = `assets/<aa>/<id>.<ext>` | Copied into the project (`ingest_file` → `*.importing` → fsync → rename; SHA-256 recorded; image dimensions read). Limit 8 GiB per file. Removed on transaction rollback. |
| `external` | `asset.external_path` (absolute, `\\?\` stripped) | Referenced, not copied (FSD §40.4). Availability is re-checked on use. A missing file gives `not_found.file` with a relink path (`files.relink`). |

- Relative paths are always resolved through `openframe_security::confine(root, rel)`. It rejects `..`, absolute
  paths, drive letters, UNC, alternate data streams (`:`), reserved names and symlink escapes.
- The extension is kept only if it is 1–10 ASCII alphanumerics. Otherwise the file has no extension.
- An asset's bytes are deleted only when its purge runs **after commit** and no other row references it
  (`purge_asset_if_unreferenced`).
- Global Idea Vault assets use the same layout under the vault folder. Copies between stores create a new asset id
  and a new file (`copy_asset_between`).

## 4. Lifecycle ✅

| Action | Behavior |
|---|---|
| Create | Folder + DB + migrations + project/member/activity rows → checkpoint → manifest. Any failure removes the half-created folder. |
| Open | exists? → not UNC → manifest → (same project already open: no-op; other project: close it) → **lock** → previous crash marker? → `Store::open` (quick_check, too-new, safety backup + migrate) → identity check → write crash marker → ensure local Owner member → update manifest → search rebuild if empty or migrated → register in recents → `ProjectOpened` |
| Save (explicit) | WAL checkpoint + online backup to `recovery/checkpoint.sqlite` |
| Close / app exit | Checkpoint + backup to `checkpoint.sqlite` + remove crash marker + release lock |
| Recover | If `session.json` existed at open, the user is offered **Keep latest autosave** (default, nothing lost) or **Return to last confirmed save** (current DB kept as `backups/before-recovery-<ms>.sqlite`, then checkpoint restored) |
| Duplicate | Online backup of the DB (works while open) + copy `assets/` → new project id and title "Copy of …", undo history cleared, new manifest |
| Archive | Flag in project + recents (ADR-0012 §1). Nothing moves on disk. |
| Delete | Owner only. Identity re-checked, lock taken to prove it isn't open, then the **whole folder goes to the Recycle Bin** |
| Locate | Re-point a recent entry to a moved folder after verifying the same `projectId` |

## 5. Locations that are refused

- **UNC/network shares** (`\\server\share`, `\\?\UNC\…`, `//…`): `project_format.network_location` (ADR-0003).
- A folder already open in another process: `project_format.in_use`.

## 6. Packages 🚧

Exchange, Review and Response packages (the **only** collaboration mechanism; LAN was removed, ADR-0007), full
project packages and backup packages are ZIP containers produced and read by the package/import-export modules.
The foundation provides safe archive handling in
`openframe_security::archive` ✅:

- `inspect` pre-flights every entry against limits before extracting anything: entry count, total size, per-entry
  size, compression ratio > 200 for entries > 1 MiB, unsafe names (via `validate_relative`) and symlink entries.
- `read_entry` enforces the size limit on the **actual stream**, not the header.
- Limits: `PACKAGE` (200k entries, 64 GiB total, 16 GiB per entry) and `DOCUMENT` (5k entries, 512 MiB, 256 MiB per
  entry; used for DOCX and exchange packages).

Package manifest format, package format version (`PACKAGE_FORMAT_VERSION = 1`) and the "open as copy / replace after
backup / cancel" flow for identity collisions are specified by the package module (📋 to be documented with it).
