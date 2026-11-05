# ADR-0043: The team password is removed

- **Status:** Accepted 2026-11-05 (owner decision, in the owner's words: "no need of any password for the app bro remove it totally").
- **Date:** 2026-11-05
- **Supersedes:** the password gate of [ADR-0024](ADR-0024-internal-team-builds.md) (decision 6, the hash, the wait after wrong
  tries, the build-time password) and decision 5 of [ADR-0030](ADR-0030-verification-ladder-and-release-flow.md) (the minimum
  length of 8 characters and where the password is kept).
- **Unchanged:** everything else in both ADRs. The internal builds stay unsigned, never sold, with no trial, no Buy button and
  no pop-up. The store builds, the Store-owned trial, the lock and paid activation ([ADR-0039](ADR-0039-store-release-licensing.md))
  never used the team password and do not change.

## Context

The internal `.exe` and `.dmg` asked a team password on every launch (ADR-0024, ADR-0030). It was a door lock for the pilot,
and the owner decided it is not needed. It also cost something: every end-to-end run needed a throwaway password and a
separate gated build, the installer build needed a secret variable on the build computer, and a person who forgot the
password could not open their own work.

## Decision

1. The internal Windows installer opens straight to the app, on the first start and on every start. There is no password
   screen, no password check in the core and no password in the build. The Mac disk image follows the same rule when the
   SwiftUI shell is built.
2. All of the password code is deleted, not switched off: the password screen and its component test, the two operations
   `team_gate.status` and `team_gate.unlock` with their generated types, the Argon2id hashing and checking, the build script
   that embedded a hash, the `STRAFT_TEAM_PASSWORD` variable in the build scripts, the release workflow and the end-to-end
   runner, and the `argon2` and `getrandom` dependencies that only it used.
3. The end-to-end runner has one release mode, `--release=open`. A run that builds with it makes the store-channel
   executable, which starts on "Checking your purchase" and is closed by the helper. An installed internal build is driven
   with `node tests/e2e/run.mjs --release=open --app=<installed straft-desktop.exe>`; it shows no check screen, so the helper
   closes none. `--release=gated` no longer exists.
4. The secret scans lose the rule and the installer check that looked for the password, because no such secret exists.
5. The Straft AI refusal for requests to change the licence or payment stays. Its pattern still lists "team password" so the
   two safety cases in the measured set (`safety-en-024`, `safety-en-038`) keep their fixed refusal until a measured run
   changes that set (ADR-0028).

## Old data

No migration is needed. An old install may hold `entitlement/team-gate.json` in the app data folder (the count of wrong tries).
Nothing reads it any more; it is left in place and harmless, and it is never deleted. No setting key was removed. An old
install opens with no error and loses nothing.

## Consequences

- Anyone who has the internal installer can open the app. This is accepted by the owner for the pilot. The unsigned build
  and the "not for sale" label are unchanged, and project files stay on the person's own computer.
- ADR-0024 decision 6 and ADR-0030 decision 5 are marked superseded in place; their text stays as history.
- The threat model no longer lists the program-file guessing limit that came with the 8-character minimum.
- The 'gate-bypass' step of the clean-machine proof and the password steps of the other steps are gone; the journey step
  remains.
- Chapter 3 (the stores) is not affected: the Store builds never carried the gate.
