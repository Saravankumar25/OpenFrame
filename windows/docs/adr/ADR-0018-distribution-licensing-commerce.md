# ADR-0018: Distribution, licensing and commerce model

- **Status:** Accepted in part 2026-10-06 by the owner: paid app with a 7-day free trial, no credit card to start, and the Mac App Store trial method (decision 3 and decision 6). Price, refund rules and the purchase proof in the installer channel are still open (see "Still open").
- **Date:** 2026-10-05, accepted in part 2026-10-06
- **Related:** `LICENSE-PENDING.md`; [ADR-0017](ADR-0017-cross-platform-windows-and-macos.md); Engineering Package Index locked decision "Product model"; PRD "Product position"; `docs/release/launch-readiness-register.md`; `docs/engineering/27-microsoft-store.md`

## Context

The repository said the product is free and open source with the licence not yet selected. The Store listing draft said "no
subscription to a cloud service". The supplied launch register assumed a commercial product with a 7-day trial and no credit card,
pricing, discounts, refunds, entitlement refresh and restore purchase. The two were incompatible until the model was stated.

## Decision

1. **The app is paid and closed source.** Anyone can try it free for 7 days without entering a card. After the trial the full app
   needs a purchase.
2. **The "free and open source" statement is superseded** in the Engineering Package Index, the README and `LICENSE-PENDING.md`. An
   end-user licence agreement replaces the open-source licence. Dependency licences stay inventoried in `THIRD_PARTY_LICENSES.md`.
3. **One entitlement interface in the Rust core** answers three questions: is the trial running, how many days are left, is the app
   unlocked. Each release file has its own adapter behind that interface:
   - Microsoft Store: the Store licence API (trial and purchase).
   - Mac App Store: StoreKit 2 with Apple's non-subscription trial model (owner decision 2026-10-06). A zero-price Non-Consumable
     "7-day Trial" (`com.openframe.studio.trial7`) starts the trial and a paid Non-Consumable "Lifetime Access"
     (`com.openframe.studio.lifetime`) unlocks the app. There is no auto-renewing subscription and Family Sharing is off. Only
     cryptographically verified StoreKit 2 transactions count; the system clock, installation date, files, Keychain and frontend
     state are never the source of truth. A small licensing server validates the Apple-signed trial transaction and computes
     `trialEndsAt` as Apple's purchase time plus 168 hours in server UTC. Lifetime comes only from the verified paid
     non-consumable in StoreKit `currentEntitlements`, restored across reinstall and new Macs with the same Apple Account, and a
     refund or revocation is honoured. Details: [38-mac-app-store-trial.md](../engineering/38-mac-app-store-trial.md).
   - Windows installer: a signed offline licence file, with the trial start stored locally and checked for tampering.
     **Superseded 2026-10-07 by [ADR-0023](ADR-0023-store-trial-and-internal-installer.md):** the installer is an internal
     build with no trial, licence or payment; the Microsoft Store trial is run by the Store; the app shows no price.
4. **Offline use must work.** A purchased app keeps working without a network. A trial keeps counting without a network.
5. **The core never depends on a store.** If a store check fails, the user's project stays readable and exportable.
   **Superseded 2026-10-07 for the Mac App Store build in the trial-ended and revoked states only:** the app is fully
   blocked there, with Buy and Restore as the only actions ([ADR-0022](ADR-0022-mac-trial-end-lock.md)). It still governs
   Windows and every other Mac state.
6. **Offline use for the Mac trial:** the Mac App Store trial needs an online check at launch and again whenever the lease of
   the last check runs out (6 hours of use by default, never beyond the trial's end), after the Mac wakes and when StoreKit
   reports a change (owner decision 2026-10-06 and its corrections the same day); without one, editing locks and opening and
   exporting stay. This is the one exception to decision 4. A lifetime purchase works offline through StoreKit entitlements.
   Windows channels are unchanged.

## Still open (owner)

Price and currency; refund rules; the exact way the installer proves a purchase; publisher legal entity and name; the text of the
end-user licence agreement. See `docs/implementation/owner-decisions.md`.

Note: the Mac trial method is decided (decision 3 and decision 6). Confirm it against Apple's current review rules before
submission.

## Consequences

- Delete `LICENSE-PENDING.md` and add the end-user licence agreement once the owner approves its text. Update manifests,
  `deny.toml`, README, CONTRIBUTING and installer text.
- Update `docs/store/listing.md` and `privacy-policy.md` to the paid model.
- Build the entitlement interface, the three adapters, the trial rules and the failure cases from the launch readiness register.

## Superseded alternatives

| Option | Why it is not used |
|---|---|
| Free and open source | The owner chose a paid product. |
| Free proprietary | Same. |
| Free app with paid add-ons | Not in any specification, and the owner chose a paid app with a trial. |

## Implementation notes

- The entitlement interface, the trial rules and the three adapters are in the crate `crates/openframe-entitlement`. Rules, file formats and limits: [36-entitlement-and-trial.md](../engineering/36-entitlement-and-trial.md).
- Built and tested at crate level: five states, 7-day trial record, offline licence file check. The two store adapters are stubs that answer Unknown. The Mac policy and adapter (with a fake StoreKit and a fake licensing server) are being built in `crates/openframe-entitlement/src/mac/`; the real StoreKit bridge and the licensing server are not built.
- Not reachable in the app yet: nothing calls the crate. The open owner decisions above still block the purchase flows and the reinstall rule.
