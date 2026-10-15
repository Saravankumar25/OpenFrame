# ADR-0024: Four release files, internal team builds behind a password

- **Amended 2026-10-15 by [ADR-0030](ADR-0030-verification-ladder-and-release-flow.md):** the minimum team password length the build accepts is 8 characters, not 12. Wherever this record says 12, read 8. The code enforces 8 since Windows phase W1 (`MIN_PASSWORD_CHARS` in `build_hash.rs`). The gate stays a door lock for the pilot; stronger protection is a Chapter 3 decision.
- **Status:** Accepted 2026-10-07 (owner answers in chat the same day).
- **Amended 2026-10-08 (owner, in chat):** the internal `.exe` and `.dmg` stay unsigned for now; Developer ID signing
  and notarization of the `.dmg` and code signing of the `.exe` move to the store and public step. The team password
  gate stays on. The owner sets `OPENFRAME_TEAM_PASSWORD` on the build PC; nobody else sees or stores it.
- **Date:** 2026-10-07
- **Related:** [ADR-0017](ADR-0017-cross-platform-windows-and-macos.md) (decision 1, release files);
  [ADR-0023](ADR-0023-store-trial-and-internal-installer.md) (internal Windows installer, no price in the app);
  [ADR-0022](ADR-0022-mac-trial-end-lock.md); `docs/implementation/owner-decisions.md`

## Context

ADR-0023 made the Windows installer an internal build. The owner then asked for an internal Mac build as well, and for
both internal builds to be protected by one team password, while the two store packages serve the public market.

## Decision

1. **Every release produces four files from one codebase:**
   - Microsoft Store package `.msix` (public; trial and lock per ADR-0023),
   - Mac App Store package `.pkg` (public; trial and lock per ADR-0022),
   - internal Windows installer `.exe` (team only),
   - internal Mac disk image `.dmg` (team only; outside the App Store). *Developer ID signing and notarization are
     deferred to the store and public step by the 2026-10-08 amendment; until then the `.dmg` is unsigned.*
2. **Internal builds** have no trial, payment, Buy button, reminders or store lock, and say "Internal build, not for
   sale" in About and Settings > Licence.
3. **Team password on every launch** of an internal build. Nothing in the app opens until the password is right.
4. **Wrong tries:** after 5 wrong passwords in a row the app waits 1 minute before the next try; each further run of 5
   wrong tries doubles the wait, up to 30 minutes. The count survives a restart. A right password resets it.
5. **The password is never stored in the source, the repository, documents, logs or the built app.** The build reads
   it from an environment variable on the build machine and embeds only an Argon2id hash with a random salt and strong
   cost settings. The check runs in the Rust core with a constant-time comparison. A build without the variable fails
   instead of shipping an unprotected internal file. Changing the password means setting the variable and rebuilding.
6. Store packages never contain the team password check or its hash.

## Consequences

- ADR-0017 decision 1 is amended from three files to four. ADR-0023 decision 3 is extended to the Mac.
- The internal `.dmg` needs an Apple Developer ID Application certificate and notarization (owner task; deferred to
  the store and public step on 2026-10-08), and the
  security standard still applies to it (hardened runtime, signed, no new network use).
- A shared password slows casual use; it does not stop a determined person who holds a copy, because the hash can be
  guessed offline. Share the files only inside the team, use a long password, and change it if it leaks.
- The password was first given in a chat session. The owner should set a new one before builds are shared.

## Alternatives rejected

- Ask the password once per computer: rejected by the owner in favour of every launch.
- No limit on wrong tries: rejected by the owner.
- Storing the password in the code or a config file: refused by the project's security standard.
