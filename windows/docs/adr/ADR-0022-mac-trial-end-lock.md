# ADR-0022: What the Mac App Store build locks when the trial ends

- **Status:** Accepted 2026-10-07 (owner answer in chat: "add the Apple payment gate to buy the full version and stop
  the software access if they have not paid after the 7-day trial"; scope confirmed the same day as the Mac App Store
  build only, with Buy and Restore as the only actions left). **Confirmed 2026-10-08** (owner, in chat): the Mac app is
  a free download, the user buys the full version in the app after the 7-day trial, and the app is fully blocked at
  expiry. Price and refund rules stay open (ADR-0018).
- **Date:** 2026-10-07, confirmed 2026-10-08
- **Related:** [ADR-0018](ADR-0018-distribution-licensing-commerce.md) (decisions 3, 5 and 6);
  `docs/engineering/36-entitlement-and-trial.md`; `docs/engineering/38-mac-app-store-trial.md`;
  `docs/implementation/owner-decisions.md`

## Context

ADR-0018 decision 5 says the user's project stays readable and exportable whatever a store check says. The owner list
still had "What locks when the trial ends" open, with the recommendation to lock editing and keep opening and exporting.
The trial screens branch built a full-screen block on the Mac. A review flagged it as a breach of decision 5. The owner
then decided the Mac case directly.

## Decision

1. **Mac App Store build, trial ended without a purchase (TrialEnded) or purchase revoked or refunded (Revoked):** the
   app is fully blocked. No project can be opened, edited or exported. The lock screen offers only **Buy full version**
   (the Apple payment sheet through StoreKit, product `com.openframe.studio.lifetime`) and **Restore purchase**. The
   screen cannot be closed.
2. The block is enforced in the Rust core, not only in the webview: every project open, edit and export operation
   refuses in these two states. A purchase or a restore lifts the block without a restart.
3. **Trial not started on the Mac:** the screen offers Start free trial, Buy and Restore (unchanged).
4. **Online check needed on the Mac (NeedsOnlineCheck):** unchanged from ADR-0018 decision 6. Editing locks; opening,
   reading and exporting keep working; the screen can be closed and offers Try again.
5. **Windows (Microsoft Store and installer):** unchanged. Nothing is blocked. What locks on Windows when the trial ends
   stays an open owner decision.
6. The user's files are never deleted or changed by the lock. After a purchase or restore everything opens as before.

## Consequences

- ADR-0018 decision 5 is superseded for the Mac App Store build in the TrialEnded and Revoked states only. It still
  governs Windows and every other Mac state.
- A Mac user who does not buy cannot export their own work after day 7. The trial screens and reminders must say this
  plainly before the trial ends, so nobody is surprised.
- Restore purchase must always work from the lock screen, including offline when StoreKit has a cached entitlement.
- Confirm against Apple's current App Review Guidelines before submission, as ADR-0018 already requires for the trial
  method.

## Alternatives rejected

- Lock editing only and keep opening and exporting (the earlier recommendation): rejected by the owner for the Mac.
- Keep Export my projects on the lock screen: offered and rejected by the owner on 2026-10-07.
