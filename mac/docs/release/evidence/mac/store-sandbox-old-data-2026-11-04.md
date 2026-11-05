# The store build and data from the earlier name

2026-11-04, macOS 26.5.2, Apple M1. What a sandboxed Mac App Store build can and cannot read of the data internal builds
wrote under the earlier name.

## What was run

A small program, signed ad hoc with the two file entitlements of `apps/macos/packaging/StraftStudio-store.entitlements`
(`com.apple.security.app-sandbox`, `com.apple.security.files.user-selected.read-write`), under the bundle identifier
`app.straftstudio.studio.sandboxprobe`. No full store app was built: that needs the owner's Apple Distribution identity and
a provisioning profile (`apps/macos/scripts/build-release-app.sh --store`), which this Mac does not have.

The program tried to list, read and write two throwaway folders made for the test, one in `~/Library/Application Support`
and one in `~/Documents`, addressed by the real home folder from `getpwuid`, not by `$HOME`.

## Result

```text
HOME env=<home>/Library/Containers/app.straftstudio.studio.sandboxprobe/Data
<home>/Library/Application Support/StraftSandboxProbeOld: DENIED list; DENIED read; DENIED write
<home>/Documents/StraftSandboxProbeOld: DENIED list; DENIED read; DENIED write
```

The test folders were unchanged afterwards. The real data folders were not touched.

## What it means

- A store build reads only its own container. It cannot see the app data folder of the earlier name,
  `~/Library/Application Support/Straft`, `~/Documents/Straft Studio` or an old library, and no entitlement in the
  package asks for it. Asking for it (`temporary-exception`) is an App Review risk, so it is not done.
- `cfg(straft_store)` selects `for_store_build` in `crates/straft-ffi/src/lib.rs`: no migration runs and folders are not renamed.
  This is deliberate: there is nothing for it to reach.
- A team member who moves from the internal `.dmg` to the store build starts with an empty library, a new Keychain trial
  guard and no downloaded AI. The internal data stays where it is, untouched, and the internal app keeps opening it.
- The way across is the person's choice, through a panel: **Open Project** on a `.straft` or earlier project folder, or a
  package restore (`.ofproject`, `.ofbackup`). A folder picked in a panel is usable for that session; it does not reopen by
  itself after a restart, because folder bookmarks are not stored. Making a package in the internal build and restoring
  it in the store build is the route that keeps working.
- No customer is affected: nothing has shipped on the store (ADR-0042).
