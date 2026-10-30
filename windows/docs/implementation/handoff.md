# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. Last rewritten: 2026-10-27 (afternoon: owner fixes
> after testing OpenFrame-5; merged to `main` and pushed to GitHub `main` `windows/` on the owner's word).

## State

1. **`main`** holds the conversational assistant (ADR-0037, slices S0 to S8 and fixes), the overnight work of
   2026-10-27 and the afternoon fixes the owner asked for after testing OpenFrame-5:
   - Call sheet screen in the exact heading and field order of the exported PDF.
   - Story page: Dark page switch; highlight and underline in a choice of colours (PDF and Word keep them).
   - Beat Board: Vertical and Horizontal layouts.
   - Global Idea Vault: no stray file after a failed add, Add Them Back for files already stranded, Locate shows what
     it found with real counts.
   - Library: first-start setup screen (default `<home>\OpenFrame` suggested only, choose, or locate existing data),
     Move Library and Locate Existing Library in Settings > Storage, recovery screen when the library is missing
     ([40-storage-locations-and-recovery.md](../engineering/40-storage-locations-and-recovery.md), ADR-0038).
2. **Checks run on the final tree:** Rust test binaries ai, platform and writing green; clippy `-D warnings` clean;
   Vitest 740 green; typecheck and lint clean; governance 90 of 90; targeted end-to-end specs (30, 31, 62, 71, 00, 10)
   green on the debug build. Not run: the full end-to-end suite (the owner said targeted tests are enough).
3. No installer was built for these changes (owner: do not build OpenFrame-6 yet).
4. **WindowsHTML refreshed on 2026-10-31** from commit 114dff50: 816 screens in Light and Dark, 1632 of 1632
   pages pass the pixel check. New: library setup (3), Settings > Storage, Story page colour menus and Dark page,
   Beat Board Horizontal, whole-film relationship map, Screenplay Dark page. Pushed to GitHub `main` `windows/` on
   the owner's word. Two app defects found by the capture, not fixed (owner to decide):
   - Under the licence lock (trial ended, purchase revoked) the core refuses `library.status`
     (`ALLOWED_WHILE_LOCKED` in `entitlement.rs`), so `LibrarySetupGate` shows "could not check where your work is
     kept" and the lock dialogs never show. Store and Mac channels only; licence code, needs a security review.
     The three licence-locked screens are missing from WindowsHTML until this is fixed.
   - The Story page Highlight and Underline colour menus show empty boxes instead of colour swatches (the menu
     opens outside the Story page, where the colour variables live).

## Next items, in order

1. Owner hand test of the new screens; the cofounder's log `%LOCALAPPDATA%\OpenFrame\logs\openframe.2026-10-27.log`
   to confirm the vault cause.
2. The next internal installer on the owner's word.
3. Reinstall drill for the storage box in `masterplan.md`, then tick it.

## Open blockers

- Mac: no setup screen and no security-scoped bookmarks yet (Mac track 1b). MSIX and Store installs not tested.

## Do not

- Do not push without the owner's word. Never `git pull` origin/main here: GitHub `main` has `mac/`, `website/` and
  `windows/`; sync `windows/` by the subtree commit method only.
- Do not build an installer without the owner's word. Do not edit `OpenFrameMac/`.
- Tool gates in this workspace ask for a short statement of facts before the first write or edit of a file.
