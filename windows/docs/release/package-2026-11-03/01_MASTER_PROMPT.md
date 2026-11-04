# Master prompt (paste this first, then point the agent at the other files)

You are working on Straft Studio, published by Vighnajith Advisories Private Limited, an offline desktop app for screenwriters and production teams. Stack: one Rust core over SQLite; Tauri v2 with React and TypeScript on Windows 11; a native SwiftUI shell on the Mac (macOS 14 or later, Apple silicon only). This is the final application before packaging as MSIX for the Microsoft Store and as a sandboxed `.app` for the Mac App Store. It is not a prototype. Treat every decision as a release decision.

## Your job

Make the app release-ready for both stores, with these outcomes. The detail is in the numbered files; this page is the contract.

1. **7-day free trial, handled the right way on each platform.**
   - Windows: the Microsoft Store owns the trial clock, the purchase and the expiry. The app asks the Store for the licence, shows the chip and reminders, and reacts. No custom timer is the source of truth.
   - Mac: fully offline trial with StoreKit 2. A free in-app purchase starts the trial, a paid non-consumable unlocks the app for life, and Apple handles the payment. No licence server. No device binding. A local clock guard stops simple clock tricks.
2. **A "Trial ends in N days" chip** (N from 7 down to 1) on Application Home and on every project screen, on both platforms, with the Buy full version button beside it.
3. **Reminders** on trial days 5, 6 and 7 only, once per calendar day, on Application Home or over a project.
4. **Buyers** (after trial or on day 1) never see the chip, the button or a reminder again. They get a lifetime licence, and then once, in order: the data note, the file safety tutorial, the welcome popup.
5. **Lock**: when the trial ends or a purchase is refunded, the app locks with a screen that cannot be closed, never loses unsaved work, and offers Buy, Restore and Contact us.
6. **Contact us**: one in-app link that opens `https://straftstudio.com/contact/`. No form in the app.
7. **Update safety**: a Store update must keep the app and every user's data intact. Audit, fix gaps and prove it with tests (file 05).
8. **Store packaging readiness**: placeholders for what only the stores can give, hard failures if any remain in a release build, ARM64 plan for Windows, sandbox, privacy manifest and pipeline for the Mac (files 03, 04, 06).
9. **Track B** after the above: the AI re-harness in `ai-reharness.md`.

## How to work

- Start with the repo's `CLAUDE.md` protocol and record the owner decision (file 00, rule 1).
- Read the real code for each area before changing it. Names to start from: `straft-entitlement` (`lib.rs`, `adapters.rs`, `notices.rs`, `mac/`), `straft-application` (`modules/entitlement.rs`), `apps/desktop/src/features/licensing/`, `apps/desktop/src-tauri/src/{lib.rs,distribution.rs}`, `mac/apps/macos/Sources/StraftUI/Store/`, `packaging/`, `scripts/build-msix.mjs`, `.github/workflows/release.yml`.
- Work in the order: (1) shared spec items that touch the Rust core; (2) your platform file; (3) update safety; (4) placeholders and release guards; (5) contact link; (6) tests and gates; (7) handoff.
- Add tests for every rule you add. A rule without a test is not done.
- Keep the diff small and in the existing style. Do not refactor what you do not need to touch.
- Remove what the new design makes dead (the Mac licence server path, listed in file 04, section M1). Do not leave half-removed code.

## Hard constraints (never break)

- No licence server, no cloud database, no device binding, no analytics, no telemetry, no accounts.
- No payment handled by the app. Windows: Microsoft Store purchase window. Mac: `product.purchase()`, Apple's own sheet. No web payment, no licence keys.
- No type-in prices anywhere. The Mac shows the StoreKit-supplied price only (file 04, M5).
- Store builds never self-update. Never ship an in-app updater in a store build (already enforced in `distribution.rs`; keep it).
- Never change a shipped migration file. New schema changes append a new migration.
- Never change the Mac bundle ID or the Keychain access group after the first release. Never change the MSIX identity name or publisher after the first release.
- Never lose user data: autosave before any lock, keep the pre-migration safety backup, refuse a project that is newer than the app instead of opening it.
- No personal data, bank, tax or registration numbers in the repo, the app or CI logs.

## Definition of done

- All gates pass in both trees, or the failure is explained in the handoff file.
- The test matrix in file 02 (section 10) is automated where it can be, and the manual remainder is written as a checklist in `docs/release/`.
- A release build fails while any `REPLACE_WITH_...` placeholder remains, and a debug or internal build still works.
- The mocks in `Trial_Licensing_Mocks/` match what the screens now say. Where the code differs from a mock, the mock or the code is fixed, and the handoff names which.
- A short handoff note lists: what was done, what is placeholder, what could not be verified without a store account, and the next three items.

## Report back (use headings and bullets, no long paragraphs)

1. Work items completed, with the files changed.
2. Gates run and results.
3. Placeholders still open (copy the table from file 06 with status).
4. Anything you found that contradicts this package or the code (state which is right).
5. The manual store-side steps the owner must do next, in order.
