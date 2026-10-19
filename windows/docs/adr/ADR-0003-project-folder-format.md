# ADR-0003: `.openframe` project folder format and single-writer lock

- **Status:** Accepted (implemented)
- **Date:** 2026-09-30
- **Related:** Engineering Index locked decision "One `.openframe` project concept"; FSD §4, §44–45; Security §2;
  `crates/openframe-project-format`; [12-project-file-format.md](../engineering/12-project-file-format.md)

## Context

Filmmakers must experience a project as **one thing** they own, can copy and can back up. Internally the project
holds structured data (SQLite with WAL), large media and recovery artifacts. Two OpenFrame windows or processes
writing the same SQLite file would corrupt state. SQLite WAL is unsafe on network file systems (SMB), yet the PRD
mentions "network drive where supported" (PRD S-3).

## Decision

1. **A project is a folder named `<Title>.openframe`** (title sanitized with `sanitize_file_name`, de-duplicated
   as `<Title> (2).openframe`). It contains:
   `openframe.json` (manifest: format, formatVersion, projectId, title, createdAt, appVersion, schemaVersion,
   aiCompatibilityVersion), `project.sqlite` (+ `-wal`/`-shm` while open), `assets/<shard>/<assetId>.<ext>`,
   `cache/` (rebuildable), `recovery/` (`session.json` crash marker, `checkpoint.sqlite`), `backups/` (safety
   backups) and `.lock`. The internal layout is an implementation detail and is never shown as a set of files to
   manage.
2. **Single writer.** On open, OpenFrame takes an exclusive OS file lock on `.lock` (`fs4::try_lock_exclusive`) and
   holds it for the session. A second opener gets `project_format.in_use`. The OS releases the lock automatically
   when the process exits, including after a crash, so no stale lock can block the user. The Tauri single-instance
   plugin also focuses the existing window instead of starting a second process.
3. **No network-share editing.** UNC paths (`\\server\share`, `\\?\UNC\…`) are refused with
   `project_format.network_location`. The supported routes are copying the project locally or exchanging packages.
   LAN collaboration was removed (ADR-0007). The current message text still mentions a "local-network collaboration
   session" and must be updated by a foundation change. Mapped drive letters cannot be detected reliably and are
   documented as unsupported.
4. **Identity check.** `openframe.json` `projectId` must equal the `project` row id. A mismatch is refused.
   `formatVersion` newer than supported is refused with `project_format.too_new`, and nothing is modified.
5. **Crash-safe writes:** `atomic_write` (temp sibling + fsync + rename) for the manifest and marker files; managed
   assets are copied to `*.importing` and then renamed.
6. **Default locations:** projects in `Documents\OpenFrame\Projects`, the Global Idea Vault in
   `Documents\OpenFrame\Global Idea Vault` (user content belongs in Documents), and app-private state in
   `%LOCALAPPDATA%\OpenFrame`.

## Consequences

- Users can copy, zip or back up a closed project folder with Explorer. Duplicate Project uses the online backup
  API, so it also works while the project is open.
- Explorer shows the project as a folder. Open Project accepts either the folder or `openframe.json`.
- Media stays as normal files (no BLOBs), which keeps the database small and backups incremental-friendly.
- Deleting a project moves the whole folder to the Recycle Bin (`trash` crate) after re-checking the identity and
  the lock.
- Cloud-sync folders (OneDrive/Dropbox) are local paths and are allowed. Sync clients may still interfere with
  WAL files while a project is open. Documented as a known risk and not blocked in v1.

## Alternatives rejected

| Alternative | Reason |
|---|---|
| Single-file ZIP container | No random-access writes or WAL. Every save would rewrite media. Crash-unsafe. |
| Single SQLite file with media BLOBs | Large database, slow backups and checkpoints, and BLOBs break generic undo (ADR-0005). |
| PID-based lock file | Becomes stale after crashes and needs heuristics to break it |
| Allow UNC with SQLite `locking_mode=EXCLUSIVE` | Still unsafe on SMB (WAL shared memory, oplocks). Invariant 9 forbids opening SQLite over a network share. |
