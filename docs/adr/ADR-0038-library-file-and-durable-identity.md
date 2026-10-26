# ADR-0038: A library file in the user's folder keeps the identity and the chosen places

- **Status:** Accepted 2026-10-27 (owner request in chat: implement the "Storage, Relocation, Backup, and Identity"
  specification end to end, with every approval granted for the night's work).
- **Date:** 2026-10-27
- **Design and operations:** [40-storage-locations-and-recovery.md](../engineering/40-storage-locations-and-recovery.md).
- **Amends:** [ADR-0011](ADR-0011-uuidv7-identities.md) (where the local user id lives) and
  [ADR-0003](ADR-0003-project-folder-format.md) (projects may be moved by the app, not only by hand).

## Context

The local user id lived only in `app.sqlite` under the app data folder. The uninstall box "delete app data" and any
troubleshooting reset removed it, the next start made a new id, and private notes in projects (owned by the old id)
vanished from view although the projects were intact. The projects folder and the vault folder were fixed at
`Documents\OpenFrame\...`; nothing could move them, and nothing remembered a different place.

## Decision

1. A small JSON file, `library.json`, in the library folder (`Documents\OpenFrame` by default) holds the local user id,
   the display name, and the chosen projects and vault folders. It is user-owned data, never removed by an uninstall.
2. When `app.sqlite` has no profile, the id in a valid library file is reused. `app.sqlite` wins when both exist; the
   file is rewritten from it. An invalid file is ignored, never trusted.
3. Locations are run-time values in the core, changed only by operations that verify the data first (move, locate,
   restore) or that never move data (default folder for new projects).
4. The vault is opened on first use, never created at start, and a configured vault folder that is missing is never
   recreated empty.
5. Private notes of a former owner are re-owned only by an explicit Owner action, one owner at a time, recorded in the
   activity log. Nothing reassigns notes silently.

## Consequences

- A reinstall or an app-data reset keeps the person's identity, places and private notes, with no account and no cloud.
- Anyone who can write the Documents folder can change `library.json`; the same person can already read every project
  there, so this adds no exposure. Paths in the file are validated like any user-chosen path.
- On the Mac store build the file lives in the app container (the sandbox's Documents) and goes with it. Folders
  outside the container need security-scoped bookmarks, which are not built yet (Mac track 1b).
