# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. It exists so the next session starts from the facts
> instead of from a long history, which is how context rot is avoided. If this page is older than the newest progress log
> entry, trust the log and fix this page first. Last rewritten: 2026-11-04, after the rename to Straft Studio.

## State

1. **The product is Straft Studio, its assistant Straft AI** (ADR-0042): code, screens, exports, packages, data folders, documents. Project folder
   `.straft`, manifest `straft.json`, app data folder `Straft`, library folder `Straft Studio`, Mac bundle id `app.straftstudio.studio`, Windows id
   `studio.straft.desktop`. The `.of*` package extensions stay, owner's decision. The domain, contact addresses and social handles keep their spelling.
2. **Old data carries over on first start:** app data copied (old folder kept), models moved, library and project folders renamed, stored paths rewritten,
   Mac preferences copied. The old spelling exists in code only behind the `LEGACY` marker (`crates/straft-platform/src/legacy_names.rs` and a few lines).
3. **Measured on a copy of real data, headless.** Never run: a real first launch of the installed app, anything on Windows. No AI eval after the name change.
4. **MacHTML:** 758 screens per mode, from commit `53d30d0b` with only the name changed. Layout faults at 1280 points wide remain (see its README).
5. **Release package state:** unchanged from the earlier handoff. Real StoreKit, the signed store build and TestFlight are not proved on this Mac.

## Next items

1. **Owner:** decide on a maximized recapture of MacHTML. A wider window (driver switch `STRAFT_CAPTURE_SIZE`, tried at 1440 x 900) removes the sidebar overlap, but the
   inspector and Files table need app layout fixes at those widths.
2. **Owner:** decide when to delete the old data folder and old app on each machine, and rename the local checkout folder.
3. **Owner:** the store steps in `docs/release/mac-store-test.md`. Then the Windows port and the AI bar decision from earlier.

## Owner decisions open

- AI tool bar missed (58.5% against 90%), and whether the older sets still count.
- Identity in `library.json` is not authentication: keep or change.
- A Straft domain and contact addresses, when bought.

## Do not

- Do not write the old product name anywhere except behind the `LEGACY` marker. `website/` on GitHub still holds it; it is a later task.
- Do not stage `Straft_Release_Package/` or `release-out/`.
- Do not run two E2E runners or two SwiftPM builds at once; build outside the E2E lock.
- Do not leave build caches behind: clear `target/` after each finished build step (owner decision 2026-10-27).
