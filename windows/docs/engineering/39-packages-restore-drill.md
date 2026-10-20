# 39. Packages and backup: restore drill

This is the written restore drill for the build plan area "Packages and backup". It says how a user gets a project
back after losing a computer, how we prove it works, and what the last run showed. Run it before every release
candidate and record the result at the end.

## 1. What a user does

1. On the old computer: project menu, **Create Backup…**, choose a folder on another disk, optionally tick **Copy
   linked external files into the backup** (portable backup), **Create Backup**. The result names the file, its size,
   the files included and any linked file that was not copied or is missing.
2. On the new computer: install OpenFrame, open it. With no project yet, Application Home shows **Restore Backup or
   Package…** next to Create Project and Open Existing Project (it stays next to Open Project once projects exist).
3. **Choose Package…**, pick the `.ofbackup` file. OpenFrame validates it before anything is created and shows the
   package type, project, date, size and linked-file report. When the package holds private notes, the screen says
   before the restore that they will become yours (ADR-0012 amendment of 2026-10-08).
4. **Restore**. The project opens with its own identity. If a project with the same identity already exists on this
   computer, the choice is **Open as Copy** (default, new identity, existing project untouched) or **Replace Existing
   After Backup** (a verified safety backup of the existing project is written first, then it is replaced).

A **Full Project Package** (`.ofproject`, project menu, Export Full Project…) restores the same way. It carries the
exporting member's own private notes only and no undo history; a Backup Package carries everything.

Private notes in a restored package belong to the person who restores it (ADR-0012 amendment of 2026-10-08): whoever
holds the file already holds the notes. In the same transaction as the rest of the restore, on the staged copy before
it is moved into place, every private note and its search entry get the restorer as owner, and undo entries that hold
an old owner's copy of a note are dropped so an undo can never hand the note back. The notes stay private: other
members of the restored project do not see them in lists, search or the assistant and cannot change them. The result
message says how many notes are now yours.

## 2. What must hold

| Check | How it is proven |
|---|---|
| Backup created, no partial file on cancel or failure | `packages.rs::fsd_44_interrupted_backup_leaves_no_partial_file`, `failure_injection.rs::m1_11_read_only_destination_keeps_the_old_file`, `format.rs::failed_write_leaves_no_partial_package` |
| Restore on a new computer (fresh data root) keeps identity, text in Telugu and Hindi, files and private notes (now owned by the person restoring), and survives restart | `packages.rs::backup_restores_on_a_new_computer_with_a_fresh_data_root` |
| Restored private notes are visible, searchable and editable by the restorer and stay private from every other member, for backups and Full Project Packages; the restore screen says so first | `packages.rs::restored_private_notes_belong_to_the_restorer_and_stay_private`, `packages.test.ts` ("restore screen"), `70-packages.test.mjs` |
| Damaged backups are refused and create nothing: truncated file, flipped bytes, damaged database pages, rows removed with recomputed checksums, a foreign trigger planted | `packages.rs::corrupt_backups_are_detected_and_leave_nothing_behind` |
| An interrupted restore leaves no project and no staging folder at any checkpoint | `packages.rs::interrupted_restore_leaves_nothing_behind` |
| Identity collision: open as copy or replace after a verified safety backup | `packages.rs::fsd_45_4_identity_collision_open_as_copy_or_replace_after_backup` |
| Packages from every committed format still open, and an old backup migrates to the current schema | `packages.rs::older_package_formats_still_open` (fixtures `crates/openframe-application/tests/fixtures/packages/format-1/`, `format-2/`) |
| Production numbers and custom elements travel in review, response, backup and full project packages |  `packages.rs::production_numbers_and_custom_elements_travel_in_every_package_class` |
| Hostile packages are refused with a stable code and leave nothing behind | `format.rs` module `hostile` (5 tests), `archive.rs::damaged_entry_data_is_a_rejected_archive_not_a_storage_error` |
| Every project table and column has a package decision | `coverage.rs::package_coverage_follows_the_project_schema` |
| The whole flow through the real app, including the Windows Save and Open dialogs | `tests/e2e/specs/70-packages.test.mjs` |

## 3. Manual drill (two computers or two Windows accounts)

1. Account A: create a project, write two scenes (one in Telugu), add a Beat Board card, a comment, a private note,
   a stored image and a linked PDF. Create a portable backup on a USB drive.
2. Account B (nothing of A's app data): open OpenFrame, **Restore Backup or Package…**, pick the backup. Expect
   **Restore** (no collision), then the project opens with the same text, image, PDF (now stored in the project)
   and comment. Before the restore, the screen says the backup's private note will become yours. After it, the note
   is in Private Notes for account B and can be edited.
3. Restore the same backup again on B. Expect the collision choice; **Open as Copy** gives "<title> (copy)".
4. Copy the backup, cut its size in half with any tool, try to restore it. Expect a message that the package is
   damaged or incomplete and no new project in the list.
5. Quit and reopen OpenFrame on B. Both projects are listed and open.

## 4. Result of the last run

| Date | Build | Automated checks | Manual drill | Notes |
|---|---|---|---|---|
| 2026-10-07 | branch `area/16-packages` | see the area report: the Rust tests above and the E2E spec | not run (needs a second Windows account) | |
