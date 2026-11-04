# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. It exists so the next session starts from the facts
> instead of from a long history, which is how context rot is avoided. If this page is older than the newest progress log
> entry, trust the log and fix this page first. Last rewritten: 2026-11-04, after the release package reached `mac/` on GitHub.

## State

1. **The release package is applied to the Mac app** on `release/trial-offline-and-store-readiness` and **is in `mac/` on GitHub**: pull
   request 6, merged 2026-11-04 as `fb4be22e`, pushed from `Saravankumar25`. The login `udarshmarthala` has read access only, so pushes need
   `Saravankumar25` (`gh auth switch --user Saravankumar25`). This handoff commit and any later one are local until the next sync. Offline trial
   with the Keychain clock guard (ADR-0040), section-scoped assistant with one road (ADR-0041), store packaging with placeholders, one product
   name "OpenFrame AI" on every screen and message.
2. **Installed here:** internal build 225, `release-out/mac/OpenFrame-6.dmg`. The update check from build 183 passed 4 of 4 and the owner's
   real data moved to app schema 11 with recents and models kept (`docs/release/update-safety.md`).
3. **Measured with the real model:** section set on the final code 322 of 336 answer facts (95.8%), length 39 of 40, pointer 59 of 59 and recall 59 of 60. Older gold
   sets through the eval-only all-sections road, on the binary before the rename: tool and arguments 58.5% against the 90% bar (59.0% before this work), end state 89.5%,
   conversation facts down from 64.6% to 58.9%, repetition up from 2 to 11. `docs/release/evidence/ai-reharness/README.md`.
4. **Not proved on this Mac:** real StoreKit, the G4 check in the App Store sandbox, the signed store build, TestFlight, the Windows tree
   (`docs/release/windows-port-2026-11-03.md`). No Xcode here, so `apps/macos/scripts/build-release-app.sh --store` has not produced a package.

## Next items

1. **Owner:** read the AI numbers above and decide about the tool bar. Spot-check one section case in ten (ADR-0028 decision 9).
2. **Owner:** the store steps in `docs/release/mac-store-test.md`, then the placeholders in `docs/release/mac-store-placeholders.md`.
3. **Windows:** copy the shared Rust and build the picker. Then the full regular E2E on the Mac.

## Owner decisions open

- AI tool bar missed (58.5% against 90%), and whether the older sets still count now that no person can reach that road.
- `Saravankumar25/OpenFrame` stays public (owner decision 2026-11-04). Whether to clean six tracked files that hold this Mac's
  user name in paths (`InterchangeScenarios.swift`, `OnboardingTests.swift`, `SettingsTests.swift`, `ShellScenarioTests.swift`, `tests/gold-project/VERIFY.md`, `docs/release/evidence/mac/m8-eval-en/results.jsonl`).
- Identity in `library.json` is not authentication (doc 40 section 2): keep or change.

## Do not

- Do not push, merge to a remote or build a store package without the owner's word.
- Do not stage `OpenFrame_Release_Package/` or `release-out/`.
- Do not run two E2E runners or two SwiftPM builds at once; build outside the E2E lock.
- Do not leave build caches behind: clear `target/` after each finished build step (owner decision 2026-10-27).
