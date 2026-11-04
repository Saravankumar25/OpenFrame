# ADR-0039: Store release of the Windows app: Store-owned trial, refresh, no licence server, lock without data loss

- **Status:** Accepted 2026-11-03 (owner's release package, "the ones given in the package override the previous decisions").
- **Date:** 2026-11-03
- **Source:** the owner's release package, copied to [docs/release/package-2026-11-03/](../release/package-2026-11-03/)
  (files 00 to 08). Where this page and the package differ, the package wins; report it.
- **Supersedes:** the licence server decisions of [ADR-0018](ADR-0018-distribution-licensing-commerce.md) and
  [ADR-0022](ADR-0022-mac-trial-end-lock.md) ("server-checked expiry"), and `docs/engineering/38-mac-app-store-trial.md`.
- **Amends:** [ADR-0023](ADR-0023-store-trial-and-internal-installer.md) (when the licence is read, refund wording);
  [ADR-0030](ADR-0030-verification-ladder-and-release-flow.md) and the chapter order (Chapter 3 is started now).
- **Related:** [ADR-0040](ADR-0040-section-scoped-assistant.md) (Track B of the same package);
  `docs/engineering/36-entitlement-and-trial.md`, `docs/engineering/27-microsoft-store.md`.

## Context

The owner has started commercial distribution, and the app must be ready to package as MSIX for the Microsoft Store and
as a sandboxed app for the Mac App Store. Partner Center and Apple Developer enrolment are still under verification, so
every value only those programs can give stays a named placeholder. Reading the code showed four real gaps on Windows:
the real Store calls were not written, the licence was read once and never again while the app stayed open, a refund
showed the "trial ended" words, and the chip was missing on Application Home.

A fifth gap came from reading the lock path: the core refuses every project operation as soon as the state locks, so text
an editor still held in memory could no longer be saved. The package asks for a test that no typed text is lost; the
code had to change as well.

## Decision

1. **Chapter 3 is started by the owner.** Store code may be written now. Nothing is pushed, packaged or built as an
   installer without the owner's word; the rule in `CLAUDE.md` stays.
2. **The Microsoft Store owns the trial clock, the purchase and the expiry.** The app asks and reacts. Local timers only
   decide when to ask again; none of them is a second authoritative clock.
3. **Real Store source in the Windows shell.** `apps/desktop/src-tauri/src/platform/microsoft_store.rs` implements
   `StoreLicenceSource` over `Windows.Services.Store`. The Rust core never depends on WinRT. Each `StoreContext` is given
   the main window's HWND through `IInitializeWithWindow` before use. Reads run off the UI thread with a 20 second timeout;
   a timeout means "could not reach the Store" and never a lock. A licence with an empty or unreadable `SkuStoreId` is not
   a Store licence (an unpackaged or sideloaded process gets an active licence from the platform) and reads as "could not
   reach the Store", never as Purchased.
4. **The purchase call starts on the UI thread.** Microsoft documents that `RequestPurchaseAsync` must be called on the UI
   thread (it fails with `0x80070578` otherwise). The product is fetched off the UI thread with the 20 second timeout, the
   call is started on the main thread, and a worker waits for its result. The wait has no 20 second limit, because the
   person may keep the Store's window open for as long as they like. This narrows package file 03 task W1, which says "all
   calls".
5. **State mapping with memory** (package file 03, section W3). `NoticeRecord` gains optional fields, so old files stay
   valid: `lastState` (`trial`, `purchased`, `ended` or `revoked`), `lastTrialSeconds` and `lastTrialReadAtMs`.

   | Store says | Last known | Result |
   |---|---|---|
   | active, not trial | any | `Purchased` |
   | active, trial | any | `TrialActive`, days rounded up, 1 to 7 |
   | not active | `trial`, `ended` or none | `TrialEnded` (remembered as `ended`) |
   | not active | `purchased` or `revoked` | `Revoked` (remembered as `revoked`) |
   | call failed | `purchased` | stay `Purchased` |
   | call failed | `ended` | `TrialEnded` |
   | call failed | `revoked` | `Revoked` |
   | call failed | `trial` | `TrialActive` from the last good remaining time minus the time since, never more |
   | call failed | `trial`, and the remaining time has run out | `NeedsOnlineCheck` |
   | call failed | none | `NeedsOnlineCheck` |

   A lock the Store confirmed holds, also after a restart, until a good Store answer lifts it: an unreachable Store
   never unlocks a refunded person or one whose trial ended. `ended` and `revoked` are not in the package; they are
   saved in `notices.json` for this reason. The row for a trial whose time has run out is not in the package either.
   Keeping `TrialActive` for ever would extend the trial, and a lock would break "never block the app
   because a read failed", so the ran-out trial takes the closable check screen. Inside one run the time since comes
   from the monotonic clock. After a restart it comes from the wall clock, floored at zero, so a clock turned back gives
   at most what the Store last said.
6. **When the licence is read** (package file 03, section W2): at launch, on Restore, on `OfflineLicensesChanged`, when
   the window comes to the front after more than 5 minutes, every 15 minutes, at local midnight, and at the expiry moment.
   One worker thread does all of it. A changed answer emits `AppEvent::Module { module: "entitlement", name: "changed" }`,
   and the interface asks again at once. This is what makes a purchase or a lock appear within seconds. The first read
   runs on the worker after startup, because the Store call can block; until it answers the state is `Unknown`, which
   shows nothing and blocks nothing.
7. **`trial_ends_at_ms` on the status.** Now plus the remaining time, from the same read. The shell formats it in local
   time for the reminder line "It ends on Thu 5 Nov at 9:42 PM.". The core never reads the time zone.
8. **New state `ClockChanged`, Mac only.** Added to the shared core (`locks_the_app`, `permits`, `summary`, `notices::due`,
   generated types). `Notice::ClockChanged` beats everything except the internal build. Windows never reaches it.
9. **The lock never loses typed text.** When a check moves the state from not locked to locked after startup, the core
   opens a short flush window (20 seconds) in which `check_operation` still lets operations through and `entitlement.notice`
   already names the lock. The lock screen, as it opens, stores what every editor holds (`flushPendingSaves`, the same
   call the window close uses) and then calls `entitlement.lock_ready`, which folds the write-ahead logs of the open
   project and the vault into their files and ends the window at once. If the page never answers, the window ends by
   itself. The dialog is modal, so nothing new can be typed during it. A lock found at launch has no window: nothing is
   unsaved then.
10. **Screens** follow package file 02 exactly: chip text "Trial ends in N days", amber at 3, 2 and 1 days, on Application
    Home and every project screen; reminders on Home and over a project; the lock and check wording; "Check again" on
    Windows where the Mac says "Restore purchase"; "Need help? Contact us". The reminder keeps mock 184's line "Pay once, own
    forever. No subscription.", which amends ADR-0023's "no pricing model" wording; no amount or currency ever appears.
11. **Contact.** One address, `https://straftstudio.com/contact/`, opened in the browser. No form and nothing sent. The
    domain and the official addresses were confirmed by the owner on 2026-11-04 ([ADR-0042](ADR-0042-product-renamed-to-straft-studio.md) decision 5).
    Shown in About, on the lock screen and on the check screen. Windows gets no Help menu.
12. **Social rows.** The eight handles are placeholders. In a release build a row with no address is hidden, and if none
    has an address the welcome popup shows only the thank-you text and OK. Debug builds keep showing them, so mocks and
    tests still work. The core tells the page through `EntitlementNotice.debugBuild`.
13. **No licence server anywhere.** `services/licence-server`, the Tauri Mac shell (`platform/mac_inputs.rs`,
    `macos_storekit.rs`, `storekit.rs`, `revalidation.rs`, the Swift StoreKit bridge folder) and their addresses leave the
    Windows tree. The Mac trial is computed offline from Apple-signed StoreKit data with a local clock guard (package file
    04, built in the Mac track). No device binding, no analytics, no accounts.
14. **Placeholders fail a release.** `scripts/check-release-placeholders.mjs` scans the files named in package file 06
    for `REPLACE_WITH_`. `release-validate` runs it for Store releases; debug, test and internal builds ignore it.
15. **Version and identity.** The package version is `Major.Minor.Build.0`. The version source is
    `[workspace.package].version` in `Cargo.toml`; `release-validate` already fails when the other files differ, and now
    also fails when the version did not rise above the last submitted one in `packaging/msix/submissions.json`. The same
    file records the identity at first submission, and a Store release with a different identity fails.
16. **Capabilities.** `runFullTrust` and `internetClient` (the AI model download). The manifest comment that said nothing
    is downloaded was left over from ADR-0025, which ADR-0026 replaced.
17. **Windows ARM64 is not shipped now.** The lock has no ARM64 engine and no ARM64 test PC is known. The x64 package runs
    on Windows 11 on ARM under emulation; confirm it in Partner Center's package validation. ARM64 stays a follow-up with
    its own acceptance test on real hardware.

## Consequences

- The real Store path cannot be exercised before the app record exists: there is no simulator for `StoreContext`. The
  mapping and every trigger are tested with doubles, and `docs/release/windows-store-test.md` is the manual walk.
- Mac parity: the Rust changes here must reach `mac/crates` in the Mac session. `docs/implementation/handoff.md` lists them.
- A person who deletes `notices.json` loses only the memory: a Store answer still decides, and the worst case is a refund
  shown with the "trial ended" words.
- Accepted limit: the remembered trial time is capped at 7 days when it is read back, so an edited `notices.json` cannot
  extend a trial. A remembered purchase stays: a failed read never locks a buyer (Windows itself refuses to start a
  Store app without a Store licence). A person who edits `notices.json` to say "purchased" and also keeps the Store
  unreachable gets the app open on that one computer; nothing leaves it and nothing is sold from it.
- The channel follows the build as well as the package: an executable built with `STRAFT_DISTRIBUTION=store` on Windows
  is the Microsoft Store channel, packaged or not, so a copy taken out of its package still locks and has no team password.
