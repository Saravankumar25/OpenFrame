# ADR-0042: Product renamed to Straft Studio

- **Status:** Accepted 2026-11-04 (owner decision).
- **Date:** 2026-11-04
- **Amends:** every earlier ADR and document that names the product, an identifier, a data folder or a file extension. Their
  decisions stay as written; only the names changed, in place, in the same change.
- **Unchanged:** the package extensions beginning `.of` and the `ofasset:` link scheme (see Consequences). The official
  website domain is `straftstudio.com` and the placeholder social handle is `@straftstudio` (owner decision 2026-11-04).
- **Merged in:** [ADR-0041](ADR-0041-product-renamed-straft.md), the same decision recorded by another session the same day.

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
   - App data folder `Straft` (`%LOCALAPPDATA%\Straft`, `%APPDATA%\Straft`), because the per-user program folder is
     `%LOCALAPPDATA%\Straft Studio`. Library folder name `Straft Studio` (`Documents\Straft Studio`, `~/Straft Studio` on the Mac).
   - Default theme id `straft-default`.
   - Format names `straft-package`, `straft-project` and `straft-vault`.
   - Environment variables `STRAFT_*`, and Rust crates `crates/straft-*`.
4. The repository is renamed to `Saravankumar25/StraftStudio` when it is next pushed. The local Mac and Windows checkout
   folders are renamed by the owner when convenient.
5. The official domain is `https://straftstudio.com`, with Google Workspace on it. The official addresses are `admin@`,
   `support@`, `contact@`, `developer@` and `hello@` at `straftstudio.com`. Public pages and store listings use
   `support@straftstudio.com` for support and `https://straftstudio.com/contact/` for the contact page. The `admin@` address
   is an account contact for the store portals and never appears in the app, the website or a store listing. The legal
   company stays **Vighnajith Advisories Private Limited**.

## Consequences

- Data written by internal builds under the earlier name is copied to the new folders on the first launch. Nothing is
  deleted from the old place. The downloaded AI models are hard linked, not copied, so the first start does not wait on
  gigabytes and they take no extra disk. If that cannot work the person downloads them again. A models folder that is a
  link to another drive moves across as the link, so that setup keeps working. The uninstaller never touches the earlier
  folder: the earlier app may still be in use, and the folder may be the only copy after a failed first start. Removing
  it is the person's choice, through the earlier app's own uninstaller.
- Every reader of an earlier format carries a `LEGACY` marker, and `straft-platform::legacy_names` holds the old strings.
  The frozen upgrade fixture and the old-package test binaries stay as old installs wrote them.
- Folders a person already has under the earlier name take the new name at start: a library folder with the earlier name
  becomes a sibling `Straft Studio` (left alone if that name is taken), and the project folders directly in its `Projects`
  folder become `<title>.straft`. Any other project is renamed when it is opened. Every stored path follows. A rename that
  cannot happen (a project open in another process, a held file, no permission) keeps the old path and is tried at the next
  start. Packages made under the earlier name keep opening.
- An earlier Windows internal install does not upgrade in place, because its application id differs. Testers install the new
  build next to it and remove the old one when they are ready.
- On the Mac, settings the system keeps per application come across. The Keychain trial guard starts fresh, because no store
  build ever shipped, so no customer loses a trial day.
- The website moved in text to `straftstudio.com`. Connecting that domain in Cloudflare is a manual owner action. The earlier
  domain is not deleted: the owner repoints or retires it. Anything already registered with it (a Partner Center or Apple
  contact, for example) needs updating by hand.
- CI needs the repository variable `STRAFT_AI_ASSETS_DIR` and (until 2026-11-05, ADR-0043) the secret `STRAFT_TEAM_PASSWORD` before it can run
  ([owner-decisions.md](../implementation/owner-decisions.md), manual work).
- Trademark and name clearance for "Straft" is still the owner's open task before brand spend.
- The package extensions `.ofproject`, `.ofbackup` and the other `.of*` ones do not spell the old name, and changing them
  would break files people already hold. They stay. Whether to change them is open for the owner in
  [owner-decisions.md](../implementation/owner-decisions.md).
- `docs/history` records were edited in place for the name only, by the owner's order of 2026-11-04 (see the reconciliation
  log).
