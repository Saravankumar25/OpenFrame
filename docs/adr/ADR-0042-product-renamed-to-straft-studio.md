# ADR-0042: Product renamed to Straft Studio

- **Status:** Accepted 2026-11-04 (owner decision).
- **Date:** 2026-11-04
- **Amends:** every earlier ADR and document that names the product, an identifier, a data folder or a file extension. Their
  decisions stay as written; only the names changed, in place, in the same change.
- **Unchanged:** the website domain `openframestudio.app` and the Instagram account name (open for the owner), and the
  package extensions beginning `.of` (see Consequences).

## Context

The owner renamed the product before the first store release; it was called OpenFrame until 2026-11-04, and this is the one
place that says so. Nothing has shipped to a customer, so every identifier can still change at no customer cost. After the
first store upload a bundle id, a package identity and a Keychain group can never change (`docs/release/mac-store-placeholders.md`),
so the window to do this closes at the first release.

## Decision

1. The product is **Straft Studio** and its assistant is **Straft AI**. Both appear in every screen, document, listing and
   test.
2. Code identifiers follow: crates, packages, Swift modules and scripts use the lowercase form `straft`.
3. Identifiers:
   - Mac bundle id `app.straftstudio.studio`, with the StoreKit products `app.straftstudio.studio.trial7` and
     `app.straftstudio.studio.lifetime`.
   - Windows application id `studio.straft.desktop`.
   - MSIX identity name `Straft.Studio`.
   - Project folder extension `.straft` and manifest file `straft.json`.
   - App data folder `Straft` (`%LOCALAPPDATA%\Straft`, `~/Library/Application Support/Straft`), because the per-user
     program folder takes the product name, as before. Default library folder `Straft Studio` (`<home>\Straft Studio`).
   - Default theme id `straft-default`.
4. The repository is renamed to `Saravankumar25/StraftStudio` when it is next pushed. The local Mac and Windows checkout
   folders are renamed by the owner when convenient.

## Consequences

- Data written by internal builds under the earlier name is copied to the new folders on the first launch. Nothing is
  deleted from the old place.
- Project folders with the earlier extension keep opening. New projects get `.straft`.
- An earlier Windows internal install does not upgrade in place, because its application id differs. Testers install the new
  build next to it and remove the old one when they are ready.
- On the Mac, settings the system keeps per application come across. The Keychain trial guard starts fresh, because no store
  build ever shipped, so no customer loses a trial day.
- The domain stays for now. Moving it needs a purchase and a redirect and is the owner's call.
- The package extensions `.ofproject`, `.ofbackup` and the other `.of*` ones do not spell the old name, and changing them
  would break files people already hold. They stay. Whether to change them is open for the owner in
  [owner-decisions.md](../implementation/owner-decisions.md).
- `docs/history` records were edited in place for the name only, by the owner's order of 2026-11-04 (see the reconciliation
  log).
