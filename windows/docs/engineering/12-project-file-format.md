# 12. Local Project File Format

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

## 6. Packages

Exchange, Review and Response packages (the **only** collaboration mechanism; LAN was removed, ADR-0007), full
project packages and backup packages are ZIP containers produced and read by
`crates/openframe-application/src/modules/packages/`. Safe archive handling lives in `openframe_security::archive`:

- `inspect` pre-flights every entry against limits before extracting anything: entry count, total size, per-entry
  size, compression ratio > 200 for entries > 1 MiB, unsafe names (via `validate_relative`) and symlink entries.
- `read_entry` enforces the size limit on the **actual stream**, not the header.
- Limits: `PACKAGE` (200k entries, 64 GiB total, 16 GiB per entry) and `DOCUMENT` (5k entries, 512 MiB, 256 MiB per
  entry; used for DOCX and exchange packages).

**Container.** `manifest.json` (format name, package type, format version, identities, scope, privacy flags,
originating user), `checksums.json` (SHA-256 of every other entry), then `content.json` for exchange packages or
`project/` (database copy, project manifest, managed files), `external/` (linked files copied on request) and
`external-references.json` for backup and full project packages. Reading validates structure, format version,
metadata, extracts into a private staging folder and checks every checksum before anything in a project changes.
The reader is bounded: entry count, entry size, total size and compression ratio limits, a 4 MB limit on the
manifest and checksum list, a 64 MB limit on `content.json`, and refusal of unsafe names (parent folders, absolute
and drive paths, alternate data streams, reserved Windows names, 8.3 short names, case collisions, duplicates) and
symbolic links. Damaged entry data is reported as a damaged package. The hostile input tests are in
`packages/format.rs` (module `hostile`) and `openframe-security/src/archive.rs`.
Writing goes to a unique staging file next to the destination and is moved into place only when complete; a
cancelled or failed write, or a read-only destination, leaves no file and keeps an existing one.

**Format versions.** `PACKAGE_FORMAT_VERSION = 2`; the app reads formats 1 and 2 and refuses newer ones with
`import.package_too_new`. Format 2 added `tableCounts` (backup and full project packages: live rows per project
table; restore refuses a database whose counts differ) and `carried` (exchange packages: the `table.column` values
the writer carried). Format 2 exchange content also carries the stored production scene number of each scene
(`productionNumber`, ADR-0015) and, in review and response packages, the custom element types the sent scenes use
(`customElements`, with `customElementId` on each element) so the reviewer sees "12A" and the element laid out as
its author defined it. A format 1 package has none of these fields and reads as before. Committed fixtures of every
format live in `crates/openframe-application/tests/fixtures/packages/format-N/` and
`older_package_formats_still_open` opens them. The format 1 fixtures are current packages rewritten without the
format 2 fields, so they prove the reader, not the old writer; the old database schemas are proven by the migration
fixtures.

**Coverage.** `packages/coverage.rs` lists every project table: how the Full Project Package treats it (kept, only the
exporter's rows, emptied, or dedicated code) and which columns exchange packages carry or leave out, each with a
reason. A Backup Package always carries the whole database. The unit test `package_coverage_follows_the_project_schema`
fails and names the table or column when a migration adds data without a package decision. The Full Project Package
rebuilds the full-text index and compacts the database after removing rows, so removed text is not left in free
pages.

**Identity collisions.** A backup or full project package whose project identity already exists here opens as a copy
with a new identity (default) or replaces the existing project after a verified safety backup. An exchange package
from another project is never applied by identity; it can only be copied with new identities. Restore is reachable
from Application Home (Restore Backup or Package) and from the project menu. The restore drill is in
[39-packages-restore-drill.md](39-packages-restore-drill.md).
