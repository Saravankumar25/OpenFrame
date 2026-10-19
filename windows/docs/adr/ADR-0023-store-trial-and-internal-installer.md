# ADR-0023: Microsoft Store trial, internal Windows installer, no price in the app

- **Status:** Accepted 2026-10-07 (owner answers in chat the same day). Decision 3 extended to an internal Mac `.dmg`, with a
  team password on both internal builds, by [ADR-0024](ADR-0024-internal-team-builds.md).
- **Date:** 2026-10-07
- **Related:** [ADR-0017](ADR-0017-cross-platform-windows-and-macos.md) (decision 1, release files);
  [ADR-0018](ADR-0018-distribution-licensing-commerce.md) (decision 3, purchase proof per channel);
  [ADR-0022](ADR-0022-mac-trial-end-lock.md) (Mac trial end lock); `docs/engineering/36-entitlement-and-trial.md`;
  `docs/engineering/27-microsoft-store.md`; `docs/implementation/owner-decisions.md`

## Context

The plan had three release files, two of them public sales channels plus a signed Windows installer for direct sales
with its own licence file, local trial timer and payment provider. The owner decided on 2026-10-07 that the public
market is the two stores only, that the Microsoft Store runs the trial itself, and that the app never shows a price.

## Decision

1. **Microsoft Store build (`.msix`).** The 7-day trial is configured in Partner Center and tracked by the Store. The app
   keeps no trial timer of its own for this channel. At launch and on the Store's licence change event it reads the
   licence through `StoreContext.GetAppLicenseAsync` (`Windows.Services.Store`): `IsActive`, `IsTrial` and
   `TrialTimeRemaining`.
2. **Microsoft Store, trial over and not bought (`IsActive` false):** the same full block as the Mac (ADR-0022). No
   project opens, edits or exports. The lock screen offers only Buy full version (the Store purchase window) and Restore
   purchase. The block is enforced in the Rust core. If the Store cannot be reached and no cached licence exists, the
   NeedsOnlineCheck rule applies: editing locks, opening and exporting stay.
3. **Windows installer (`.exe`) is an internal build** for the owner and the owner's team. It is never sold or listed.
   It has no trial, no payment, no Buy button, no reminders and no lock, and it says "Internal build, not for sale" in
   About and Settings > Licence. It is still signed and still passes the security standard.
4. **No price in the app.** No screen, reminder, error, tutorial or document inside the app mentions a price, currency
   or pricing model. Buy hands off to the store's own purchase window (StoreKit on the Mac, the Store purchase dialog on
   Windows), which shows the price. A test fails if price wording appears in the licensing screens.

## Consequences

- ADR-0017 decision 1 is amended: still three files, but the `.exe` is internal, not a sales channel.
- ADR-0018 decision 3 is superseded for the installer: no signed licence file, local trial or payment provider. The
  installer licence and local trial code is removed once the Microsoft Store adapter lands (clean as you go).
- These owner items close as no longer needed: how the installer proves a purchase, trial after a reinstall on the
  installer, licence expiry, machine binding and revocation on the installer, and what locks on Windows at trial end.
- Refund rules follow each store's own rules; no written rule for direct sales is needed.
- The Partner Center product identity and the 7-day trial setting are owner tasks; the real Store calls cannot be
  tested until they exist.
- Risk: an internal `.exe` copied outside the team is a full, unlocked copy. Share it only inside the team.

## Alternatives rejected

- Keep the installer for direct sales with its own licence file: rejected by the owner.
- Show the price or "pay once" wording in the app: rejected by the owner; the stores show it.
