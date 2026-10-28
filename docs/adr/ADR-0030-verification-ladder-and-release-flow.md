# ADR-0030: Verification ladder, release flow and the internal team gate

- **Status:** Accepted 2026-10-15 (owner, in chat). **Decision 5 superseded 2026-10-27** (owner, in chat): the team
  password gate is removed and internal builds open without a password. The rest of this record stays in force. Record:
  `docs/implementation/owner-decisions.md`, row "Team password removed".
- **Date:** 2026-10-15
- **Amends:** [ADR-0024](ADR-0024-internal-team-builds.md) (minimum team password length), and the three-chapter order in
  `docs/product/vision-and-scope.md` (Chapter 1 now runs the Windows track and then the Mac track).
- **Related:** [ADR-0027](ADR-0027-rust-core-two-ui-shells.md), [ADR-0021](ADR-0021-diagnostics-telemetry-analytics-policy.md),
  [testing strategy](../testing/README.md), [internal test plan](../testing/internal-test-plan.md).

## Context

More than 150 end-to-end tests exist and most features are still "unverified". The tests drive a debug build on one
machine with a fake AI. They prove that features are wired and consistent, not that a person can install the product and
trust it. The owner wants each platform finished and tested end to end, built, and tested by the team before the next step.

## Decision

1. **Five levels of proof.** A feature row claims only the level it has evidence for.
   1. Built: the code exists.
   2. Proved on development: unit, integration and end-to-end tests pass on the debug build.
   3. Proved on the artifact: the same end-to-end scenarios pass against the release build that goes into the installer.
   4. Proved on a clean computer: installed and driven by a script on a computer with no developer tools.
   5. Proved by people: the team used it for 2 to 3 days and the findings were fixed and checked again.
   "Verified" in the feature status file means level 3 or higher from the day the Windows track reaches phase 9; before
   that it means level 2 and is labelled so.
2. **Release flow.** Chapter 1, Windows track: finish and prove the Windows application, build the internal `.exe`, the team
   tests it by hand. Chapter 1, Mac track: migrate to the SwiftUI shell, prove it, build the internal `.dmg`, the team
   tests it by hand. Chapter 2: the website. Chapter 3: Microsoft Store and Mac App Store packaging. A stage starts only when
   the one before it has closed. No Chapter 2 or 3 implementation code is written during Chapter 1.
3. **Clean-machine proof.** A virtual machine on the owner's computer (which runs Windows 11 Home, with no Sandbox) and a
   borrowed laptop, both with no developer tools. The borrowed laptops also give the graphics-card measurements.
4. **Internal builds stay unsigned** and carry no trial or licence. SmartScreen and Gatekeeper warnings are documented
   installation restrictions.
5. *(Superseded 2026-10-27: the gate is removed; internal builds open without a password.)* **Team password.** The internal `.exe` and `.dmg` keep the password gate of ADR-0024 (hash of the password, asked on every
   launch, waiting after wrong tries). The minimum password length the build accepts changes from 12 to **8 characters**.
   The password is held by the owner and the team, set only in the build computer's environment, and never written to the
   repository, a log or a document. The gate is a door lock for the pilot. Stronger protection (decrypting the app content
   with a key derived from the password, encrypting project files at rest) is a later decision for Chapter 3.
6. **Secrets and links.** Each phase that ends in a build runs a secret scan of the working tree and the history, a scan of
   the built installer for secrets and personal paths, and a link check of every document and every address the app can
   open. A failure blocks the build.
7. **No installer without the owner's word.** An installer is built only when the owner says so in that conversation.
8. **Hardware tested.** The owner will test on several laptops, with different graphics cards ([ADR-0029](ADR-0029-gpu-acceleration.md)).

## Consequences

- The feature status file gains a column for the proof level; its checker script is extended in Windows phase 1.
- The shared end-to-end scenario list ([ADR-0027](ADR-0027-rust-core-two-ui-shells.md)) is created in Windows phase 4.
- *(Superseded 2026-10-27.)* Lowering the minimum to 8 lowers the cost of guessing the password from the program file. This is accepted for friendly
  testers and is recorded as a known limit in the threat model.
