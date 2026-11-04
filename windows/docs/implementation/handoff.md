# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. Last rewritten: 2026-11-04, after merging the local
> Windows work with the windows folder on GitHub.

## State

1. **GitHub and local now hold the same Windows code.** The product is Straft Studio and the assistant is Straft AI
   ([ADR-0042](../adr/ADR-0042-product-renamed-to-straft-studio.md); [ADR-0041](../adr/ADR-0041-product-renamed-straft.md) is a
   pointer). The merge and its decisions are in the [reconciliation log](reconciliation-log.md). The local checkout still has
   the app at its root; on GitHub it is under `windows/`.
2. **Checks on the merged tree:** see the top entry of the [progress log](progress-log.md).
3. **One migration.** `legacy_names`, `legacy_migration` and `legacy_rename` carry old data over at first start. The old app
   data folder is copied once and left in place, the models are hard linked, and the library and project folders take the
   new name. A failed copy uses the old folder for that run. It was tested on temporary folders; the real first start on the
   owner's laptop has not been run.
4. **Chapter 3 is started by the owner** ([ADR-0039](../adr/ADR-0039-store-release-licensing.md)). The package is in
   [docs/release/package-2026-11-03/](../release/package-2026-11-03/00_README_START_HERE.md); it overrides older decisions.
5. **Track A, Windows licensing and store readiness, is built.** Real `StoreContext` calls with a refresh worker, licence
   memory, refund words, lock that stores typed text first, chip and reminders on Home, Contact us, placeholders that fail a
   Store release, version and identity guards, update safety U1 to U11 (table in
   [update-simulation.md](../release/update-simulation.md)), licence server and Tauri Mac shell removed.
6. **Track B, section-scoped Straft AI, is built** ([ADR-0040](../adr/ADR-0040-section-scoped-assistant.md), slices R0 to
   R7). The person picks one to three sections; only those are read.
7. **Straft AI bars missed, not lowered:** invented names 39.7 percent (bar: no worse than 32.1), pointer recall 68.8 percent
   (bar 90), long answers median 37 s (bar 30 s). Answer facts rose from 47.1 to 71.7 percent.
   [Numbers](../release/evidence/ai-section-scoped-2026-11-03/r7-final/README.md). The rename did not rerun the evaluation.
8. **The internal installer on the owner's laptop was built before the rename.** A new installer needs the owner's word.
9. **`StraftMac/`** is untracked here and has its own migration port. It must take the Mac ids and this migration's behaviour.
   No Swift was compiled and no Mac build was run.

## Next items, in order

1. Owner: choose how the product reads in running text. The app, WindowsHTML and the manuals say "Straft could not..." and use
   "Straft Studio" as the full name; the earlier GitHub text said "Straft Studio could not...". The short form stayed. Changing
   it is a scripted edit, and WindowsHTML and the manuals would then be regenerated.
2. Owner: decide on two review findings left as they are. A project folder opened from elsewhere is renamed when it is opened
   (network paths excluded), and the library is renamed even if the old app is running, as long as no project in it is open.
3. Owner: rename the checkout folder to `C:\Straft`, then reset `CARGO_TARGET_DIR`, remove the old team password variable, and
   create the CI variable `STRAFT_AI_ASSETS_DIR` and secret `STRAFT_TEAM_PASSWORD`.
4. Mac session: bring `StraftMac/` into line with this migration, then the Rust parity list below and package file 04.
5. Earlier item, still open: Partner Center app record, identity values, publisher display name, 7-day trial, then the manual
   walk in [windows-store-test.md](../release/windows-store-test.md). Only a Store package can prove the licence read, the
   purchase window and `OfflineLicensesChanged`.

Also waiting on the owner: may Straft AI read the contents of small text files? The Files section stays at 37.5 percent
without it.

## Website

The live website comes from `website/` on GitHub `main`; a push there builds, tests and deploys it. The local folder `StraftWebsite` is
behind GitHub, so start website work from a clean worktree of `origin/main`. The phone nav and pop-up work went live on 2026-11-05.

## Open blockers

- All store values are placeholders ([list](../release/package-2026-11-03/06_PLACEHOLDERS_AND_COMPANY_DETAILS.md)); a Store
  release fails until they are filled. `LICENSE-PENDING.md` and the migration freeze also block `release-validate`.
- Trademark and name clearance for "Straft" is the owner's task before brand spend.
- Not run: release-mode E2E (`--release`), any MSIX or WACK run, the manual update simulation, ARM64 (not shipped,
  ADR-0039 item 17), the live window title, About and icon check.

## Mac parity (the Rust the `StraftMac/` tree must take)

`crates/straft-entitlement` (`ClockChanged`, `Notice::ClockChanged`, `read_store` and `StoreMemory`, `NoticeRecord` fields,
`.bad` copies, refusing saves after a failed load) and `crates/straft-application` (`modules/entitlement.rs` flush window
and `entitlement.lock_ready`, `check_edit` in `registry.rs`, `modules/ai/sections/`, dossier by leaves, `migrations/app/0010`).
The Windows tree still holds `crates/straft-entitlement/src/mac/` with the licence server design: package file 04 M1 and
M2 belong to the Mac session.

## Do not

- Do not push without the owner's word. Never `git pull` origin/main here: GitHub `main` has `mac/`, `website/` and
  `windows/`; sync `windows/` by the subtree commit method only.
- Do not build an installer or a release app executable without the owner's word.
- Do not tick "delete app data" when uninstalling the old app before Straft has started once. The first start copies that
  data. Projects are not affected. Straft's own uninstaller never touches the old folder.
- Do not edit the old-name spellings marked with a `LEGACY` comment (`git grep 'LEGACY:'`). Old data needs them.
- When work is too slow, parallelize with a file ownership map; never drop a step.
