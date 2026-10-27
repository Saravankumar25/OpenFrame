# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. Last rewritten: 2026-10-27 (overnight session:
> Story page formatting, sides, call sheet order, storage and recovery, OpenFrame-5; branch
> `ui/story-page-callsheet-sides`, local, not merged, not pushed).

## State

1. **Branch `ui/story-page-callsheet-sides`** sits on top of `ai/conversational-s0` (the conversational assistant,
   slices S0 to S8 and its fix round). It adds, all tested: the Story page as an A4 sheet with font size, bold, italic,
   underline, highlight and zoom (migration 0019, formatted PDF and Word); Story opens on the Story page; sides PDFs
   without the OpenFrame mark and brand line; the call sheet screen in the PDF's order; the storage specification
   ([ADR-0038](../adr/ADR-0038-library-file-and-durable-identity.md),
   [40-storage-locations-and-recovery.md](../engineering/40-storage-locations-and-recovery.md)); four assistant answer
   gaps found by specs 72 and 73. `main` has none of this yet.
2. **Installer:** `release-out/OpenFrame-5.exe` from this branch (record in `release-out/OpenFrame-5.json`; see the
   first entry of the progress log for the hash and the scans).
3. **Checks:** workspace tests, clippy, fmt, Vitest 715, governance 90 of 90 green; end-to-end: every spec file green
   (full run plus reruns of the seven that needed spec or app fixes). Details in the progress log.

## Next items, in order

1. **Owner hand test of OpenFrame-5:** the Story page toolbar and zoom, sides PDF, call sheet order, Settings > Storage
   (back up, restore, move the vault; move a project), and Home "Found on this PC".
2. **Reinstall drill** for the storage box: uninstall with "delete app data", reinstall, confirm the same identity,
   places and private notes come back; then tick the storage box in `masterplan.md` with that evidence.
3. **Merge to `main`** once the owner agrees (it brings the conversational assistant too), then the AI bars of spec 7.5.

## Open blockers

- Owner: agree to merge the branch; hand test OpenFrame-5.
- Not done: the user manual (.docx) does not show the new screens; the Mac half of storage (security-scoped bookmarks,
  Mac track 1b); a real Microsoft Store install test of the library file.

## Do not

- Do not push without the owner's word. Do not `git pull` origin/main here. Do not edit `OpenFrameMac/`.
- Do not build another installer without the owner's word.
- Do not overwrite `library.json` by hand in tests on this PC; it holds the owner's identity.
- Tool gates in this workspace ask for a short statement of facts before the first write or edit of a file; give it, then retry.
