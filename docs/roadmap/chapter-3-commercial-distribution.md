# Chapter 3: commercial distribution on Windows and macOS

> Written 2026-10-09. Chapter 3 begins when Chapter 1 is finished. This page lists what is already decided, what is built
> and unverified, what must be checked against the stores' current rules before any payment flow is built, and what the
> existing documents say. The store rules below came from a quick web search on 2026-10-09 and are marked as "verify"
> where the search could not reach the official page. Nothing here is legal advice.

## 1. Goal

The same application, packaged for the Microsoft Store and the Mac App Store, with a 7-day free trial followed by paid
activation, target prices of 3,000 rupees in India, 89 US dollars in the USA and the rest of the world, and 89 euros in
Europe. One shared code base. Platform-specific code stays in the platform crate and the entitlement adapters.

## 2. Already decided (and where)

| Topic | Decision | Record |
|---|---|---|
| Model | Paid, 7-day free trial, no credit card to start | [ADR-0018](../adr/ADR-0018-distribution-licensing-commerce.md) |
| Price in the app | Never shown; the store's purchase window shows it | [ADR-0023](../adr/ADR-0023-store-trial-and-internal-installer.md) |
| Microsoft Store trial | The Store runs the trial; the app reads the Store licence; an ended trial blocks everything except Buy and Restore | ADR-0023 |
| Mac App Store trial | A zero-price "7-day Trial" non-consumable plus a paid "Lifetime Access" non-consumable; expiry computed by a licence server from Apple's purchase time; full block at expiry | [ADR-0022](../adr/ADR-0022-mac-trial-end-lock.md), [38-mac-app-store-trial.md](../engineering/38-mac-app-store-trial.md) |
| Internal builds | No trial, no purchase, no price, no lock | ADR-0023, [ADR-0024](../adr/ADR-0024-internal-team-builds.md) |
| Platforms | Windows 11 and Apple silicon only | [ADR-0017](../adr/ADR-0017-cross-platform-windows-and-macos.md) |

## 3. Built, but only tested with fakes

- `crates/openframe-entitlement`: eight licence states, the 7-day rules, reminders, the Microsoft Store and Mac adapters
  over fake store sources. 76 unit tests in the crate.
- The store lock inside the Rust dispatcher (`Registry::dispatch` calls `entitlement.check_operation`).
- The trial chip, reminders, Buy routing and first-run safety screens in the interface.
- `services/licence-server`: a small Rust server for the Mac channel that checks Apple's signed transactions and answers
  with a signed trial end time. It needs a network call from the Mac app, so the application is not fully offline for the
  Mac store channel while a trial runs.
- A Swift StoreKit 2 bridge compiled on Mac builds. It has never talked to the real App Store.

None of this has run against a real store account, a real product or real money.

## 4. To verify before any payment flow is written

These are the owner's own requirement: "do not assume our proposed payment or trial approach is automatically permitted".

| Question | What the search found | Check before building |
|---|---|---|
| May a non-game Windows app use its own or a third-party payment system in the Microsoft Store? | Since July 2021 Microsoft allows apps (not games) to use their own or third-party commerce and keep 100 percent of that revenue; the policy text is section 10.8 of the Store policies | Read the current Store Policies on Microsoft Learn and the App Developer Agreement in Partner Center. Decide whether to use the Store's trial and purchase, or an own system |
| May a Mac App Store app offer a free time-limited trial that unlocks with one payment? | The 2021 App Review Guidelines revision allows a non-subscription app to offer a trial through a non-consumable in-app purchase at price tier zero, named like "7-day Trial", with the duration and the later charge shown before the trial starts | Read the current guideline 3.1.1 and the Mac-specific text; confirm that a server-computed expiry is accepted. ADR-0018 already asks for this check |
| Can the price be exactly 3,000 rupees, 89 dollars and 89 euros? | Apple sets regional prices from price tiers, so exact figures may not exist; Microsoft Partner Center has its own price tiers | Look at the price matrices in App Store Connect and Partner Center; adjust the targets to the nearest allowed price or set custom prices per market |
| Must the app show the price? | ADR-0023 says never; Apple wants the trial length, the features that stop, and the later charge disclosed before a trial starts | Confirm the first-launch screen satisfies guideline 3.1.1 without a price (already a listed task) |
| Does the offline-first promise survive the licence server? | The Mac channel checks online at launch during the trial and then uses a lease | Decide whether this is acceptable, and say so in the privacy policy |
| Refunds | Each store decides its refund rules | The owner decides what to promise; the app follows the store |

## 5. Distribution requirements by platform

| Item | Microsoft Store | Mac App Store |
|---|---|---|
| Package | `.msix` built by `scripts/build-msix.mjs` | `.app` inside a `.pkg` |
| Account | Partner Center with a reserved name and the identity values in `packaging/msix/store.config.json` (still placeholders) | Apple Developer Program, App Store Connect record, certificates |
| Signing | The Store signs the package | Developer ID and Mac App Store certificates; hardened runtime; App Sandbox with user-selected files |
| Notarization | Not needed for the Store | Needed outside the App Store (the internal `.dmg` later) |
| Validation | Windows App Certification Kit on the exact package | App Store validation and TestFlight |
| AI engine and models | Bundled, loads under MSIX; the manifest drops the `internetClient` capability | Bundled in the app resources, must load under the sandbox |
| Updates | Store updates only; no in-app updater | Store updates only |
| Listing | Description, screenshots from the release candidate, age rating, privacy policy URL, support contact | Same, plus the in-app purchase products |

## 6. Legal and licence work

1. The end-user licence agreement must exist. It replaces `LICENSE-PENDING.md`, and it must pass on the Gemma use
   restrictions to every user. The Gemma Terms of Use (ai.google.dev/gemma/terms) require recipients to get a copy of the
   terms, a "Notice" text file with a fixed sentence (present in `packaging/ai/notices/gemma-NOTICE.txt`), and the use
   restrictions as an enforceable clause in the agreement between the distributor and the user. Counsel confirms this.
2. Third-party notices (llama.cpp, BGE small, fonts, Rust and npm packages): `THIRD_PARTY_LICENSES.md` exists and is
   generated.
3. Privacy policy and terms hosted on the website (Chapter 2) and linked from both stores.
4. Trademark and name checks, company details, support address.

## 7. Steps, in order

1. Owner decides the model per store (own commerce or store commerce on Windows) after reading the current rules.
2. Update the decision register with the result.
3. Wire the real calls behind the existing interfaces: Microsoft Store licence and purchase, StoreKit bridge and licence
   server.
4. Build the store packages from the shared code and run the clean-machine checks and the stores' validators.
5. Submit, answer reviewers, publish, watch the first 90 days.

## 8. Existing material

[../store/listing.md](../store/listing.md) and [../store/privacy-policy.md](../store/privacy-policy.md) are drafts with
placeholders. [../release/launch-readiness-register.md](../release/launch-readiness-register.md) lists operations, legal,
commerce and support requirements by key. [../engineering/27-microsoft-store.md](../engineering/27-microsoft-store.md),
[36](../engineering/36-entitlement-and-trial.md) and [38](../engineering/38-mac-app-store-trial.md) hold the engineering
contracts. The 83 build plan boxes tagged "Later: store publishing" are the detailed checklist for this chapter.
