# Chapter 3: commercial distribution on the Microsoft Store and the Mac App Store

> Rewritten 2026-10-15. Chapter 3 starts after Chapter 2. This page keeps what is decided, what is built and unverified, and
> what must be checked against the stores' current rules. The implementation plan with phases is written when Chapter 2
> closes. No Chapter 3 code is written before then. The earlier version is in
> [the archive](../history/2026-10-15-chapter-3-plan-before-restructure.md). Nothing here is legal advice.

## 1. Goal

The same application, packaged for the Microsoft Store (Tauri shell) and the Mac App Store (SwiftUI shell), over one shared
Rust core ([ADR-0027](../adr/ADR-0027-rust-core-two-ui-shells.md)). A 7-day free trial followed by paid activation. Target
prices (not final): 3,000 rupees in India, 89 US dollars in the USA and the rest of the world, 89 euros in Europe.

## 2. Already decided

| Topic | Decision | Record |
|---|---|---|
| Model | Paid, 7-day free trial, no credit card to start | [ADR-0018](../adr/ADR-0018-distribution-licensing-commerce.md) |
| Price in the app | Never shown; the store's purchase window shows it | [ADR-0023](../adr/ADR-0023-store-trial-and-internal-installer.md) |
| Microsoft Store trial | The Store runs the trial; the app reads the Store licence; an ended trial blocks everything except Buy and Restore | ADR-0023 |
| Mac App Store trial | A zero-price "7-day Trial" non-consumable and a paid "Lifetime Access" non-consumable; expiry computed by a licence server from Apple's purchase time; full block at expiry | [ADR-0022](../adr/ADR-0022-mac-trial-end-lock.md), [38-mac-app-store-trial.md](../engineering/38-mac-app-store-trial.md) |
| Internal builds | No trial, no purchase, no price, no lock; team password gate | ADR-0023, [ADR-0024](../adr/ADR-0024-internal-team-builds.md), [ADR-0030](../adr/ADR-0030-verification-ladder-and-release-flow.md) |
| Platforms | Windows 11 and Apple silicon only | [ADR-0017](../adr/ADR-0017-cross-platform-windows-and-macos.md) |
| AI in the packages | The runtime ships in the package; the models download after install on the user's click | [ADR-0026](../adr/ADR-0026-offline-ai-download-after-install.md) |

## 3. Built, but only tested with fakes

- `crates/openframe-entitlement`: eight licence states, the 7-day rules, reminders, the Microsoft Store and Mac adapters
  over fake sources; the store lock inside the Rust dispatcher; the trial chip, reminders and Buy routing in the screens.
- `services/licence-server`: a small Rust server for the Mac channel; the Mac app needs a network call during the trial, so
  the Mac store channel is not fully offline while a trial runs.
- A Swift StoreKit 2 bridge. It has never talked to the real App Store. It moves into the SwiftUI application
  ([ADR-0027](../adr/ADR-0027-rust-core-two-ui-shells.md)) when this chapter starts.

None of this has run against a real store account, product or money.

## 4. To verify before any payment flow is written

The owner's requirement: do not assume the proposed payment or trial approach is permitted. The first search (2026-10-09)
could not reach every official page, so each row is read again from the store's own pages first.

| Question | What the search found | Check before building |
|---|---|---|
| May a non-game Windows app use its own payment system in the Microsoft Store? | Microsoft allows apps (not games) their own or third-party commerce since July 2021 (policy section 10.8) | Read the current Store Policies and the App Developer Agreement; choose store commerce or an own system |
| May a Mac App Store app offer a free trial that unlocks with one payment? | The 2021 guidelines allow a non-subscription trial as a zero-price non-consumable with the duration and later charge disclosed | Read guideline 3.1.1 and the Mac text; confirm a server-computed expiry is accepted |
| Can the price be exactly 3,000 rupees, 89 dollars and 89 euros? | Regional prices come from price tiers | Read the price matrices in both consoles and adjust or set custom prices |
| Must the app show the price? | ADR-0023 says never; Apple wants the trial length and the later charge disclosed | Confirm the first-launch screen satisfies guideline 3.1.1 without a price |
| Does the offline promise survive the licence server? | The Mac channel checks online during the trial | Decide, and say so in the privacy policy |
| May the app download model files after install under each store's rules? | Weights are data; the Mac sandbox forbids downloading executable code | Confirm with each store's current text (ADR-0026) |
| Refunds | Each store decides | The owner decides what to promise; the app follows the store |

## 5. Distribution requirements by platform

| Item | Microsoft Store | Mac App Store |
|---|---|---|
| Package | `.msix` built by `scripts/build-msix.mjs` | `.app` inside a `.pkg` |
| Account | Partner Center, reserved name, identity values in `packaging/msix/store.config.json` (placeholders now) | Apple Developer Program, App Store Connect record, certificates |
| Signing | The Store signs the package | Developer ID and Mac App Store certificates; hardened runtime; App Sandbox with user-selected files |
| Notarization | Not needed for the Store | Needed outside the store |
| Validation | Windows App Certification Kit on the exact package | App Store validation and TestFlight |
| AI | Runtime in the package; models download after install; `internetClient` is justified by that download | Runtime and engine choice per [ADR-0027](../adr/ADR-0027-rust-core-two-ui-shells.md) open item 2; models in the app container |
| Updates | Store updates only | Store updates only |
| Listing | Description, screenshots from the release candidate, age rating, privacy policy URL, support contact | Same, plus the in-app purchase products |

## 6. Legal and licence work

1. The end-user licence agreement replaces `LICENSE-PENDING.md` and passes on the use restrictions of every model that ships
   (for Gemma, the terms at ai.google.dev/gemma/terms; another model, its own terms). Counsel confirms.
2. Third-party notices: `THIRD_PARTY_LICENSES.md` is generated.
3. Privacy policy and terms hosted on the website (Chapter 2) and linked from both stores.
4. Trademark and name checks, company details, support address.

## 7. Decisions moved here from Chapter 1

- Stronger protection for the internal team gate and encryption of project files at rest ([ADR-0030](../adr/ADR-0030-verification-ladder-and-release-flow.md) decision 5).
- "Report AI output" (ADR-0020 decision 4).
- The in-app updater question for the direct builds, and signing and notarization of every shipped file.

## 8. Existing material

[../store/listing.md](../store/listing.md) and [../store/privacy-policy.md](../store/privacy-policy.md) are drafts with
placeholders. [../release/launch-readiness-register.md](../release/launch-readiness-register.md) lists operations, legal,
commerce and support requirements. [../engineering/27-microsoft-store.md](../engineering/27-microsoft-store.md),
[36](../engineering/36-entitlement-and-trial.md) and [38](../engineering/38-mac-app-store-trial.md) hold the engineering
contracts. The build plan boxes tagged "Later: store publishing" are the detailed checklist for this chapter.
