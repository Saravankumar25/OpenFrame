# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. It exists so the next session starts from the facts
> instead of from a long history, which is how context rot is avoided. If this page is older than the newest progress log
> entry, trust the log and fix this page first. Last rewritten: 2026-11-03, after the release package was applied to the Mac app.

## State

1. **The release package is applied to the Mac app** on `release/trial-offline-and-store-readiness`. **Not on GitHub yet:** the push was
   refused (403), because the login on this Mac, `udarshmarthala`, has read access only. Local branch `mac/sync-2026-11-03` holds the merge
   into `mac/`, ready to push. Offline trial with the Keychain clock guard (ADR-0040), section-scoped assistant with one road (ADR-0041), store
   packaging with placeholders, one product name "OpenFrame AI" on every screen and message.
2. **Installed here:** internal build 225, `release-out/mac/OpenFrame-6.dmg`. The update check from build 183 passed 4 of 4 and the owner's
   real data moved to app schema 11 with recents and models kept (`docs/release/update-safety.md`).
3. **Measured with the real model:** section set on the final code 322 of 336 answer facts (95.8%), length 39 of 40, pointer 59 of 59 and recall 59 of 60. Older gold
   sets through the eval-only all-sections road, on the binary before the rename: tool and arguments 58.5% against the 90% bar (59.0% before this work), end state 89.5%,
   conversation facts down from 64.6% to 58.9%, repetition up from 2 to 11. `docs/release/evidence/ai-reharness/README.md`.
4. **Not proved on this Mac:** real StoreKit, the G4 check in the App Store sandbox, the signed store build, TestFlight, the Windows tree
   (`docs/release/windows-port-2026-11-03.md`). No Xcode here, so `apps/macos/scripts/build-release-app.sh --store` has not produced a package.

## Next items

0. **Owner:** give `udarshmarthala` write access, or push `mac/sync-2026-11-03` from an account that has it, then merge when the checks pass.
1. **Owner:** read the AI numbers above and decide about the tool bar. Spot-check one section case in ten (ADR-0028 decision 9).
2. **Owner:** the store steps in `docs/release/mac-store-test.md`, then the placeholders in `docs/release/mac-store-placeholders.md`.
3. **Windows:** copy the shared Rust and build the picker. Then the full regular E2E on the Mac.

## Owner decisions open

- AI tool bar missed (58.5% against 90%), and whether the older sets still count now that no person can reach that road.
- GitHub reports `Saravankumar25/OpenFrame` as public, while `CLAUDE.md` says the repository stays private. Older tracked files hold this Mac's
  user name in paths (`InterchangeScenarios.swift`, `OnboardingTests.swift`, `tests/gold-project/VERIFY.md`, `docs/release/evidence/mac/m8-eval-en/results.jsonl`).
- Identity in `library.json` is not authentication (doc 40 section 2): keep or change.

## Do not

- Do not push, merge to a remote or build a store package without the owner's word.
- Do not stage `OpenFrame_Release_Package/` or `release-out/`.
- Do not run two E2E runners or two SwiftPM builds at once; build outside the E2E lock.
- Do not leave build caches behind: clear `target/` after each finished build step (owner decision 2026-10-27).
