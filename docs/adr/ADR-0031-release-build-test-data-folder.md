# ADR-0031: The release build honours the test data folder, for file locations only

- **Status:** Accepted 2026-10-16 (owner, in chat, at the start of Windows phase W4).
- **Date:** 2026-10-16
- **Amends:** the "debug builds only" rule for `OPENFRAME_DATA_ROOT` in `apps/desktop/src-tauri/src/lib.rs` and
  [DEVELOPMENT.md](../../DEVELOPMENT.md). Every other test switch keeps that rule.
- **Related:** [ADR-0030](ADR-0030-verification-ladder-and-release-flow.md) (proof level 3 needs the suite on the release
  build), [Windows phase 4](../roadmap/chapter-1a-windows-phases.md), [threat model](../engineering/20-security-threat-model.md).

## Context

Proof level 3 means the end-to-end suite passes on the release build that goes into the installer. The suite runs every
spec in its own data folder through `OPENFRAME_DATA_ROOT`, but the release build ignored that variable, so a release run
would have read and written the tester's real Documents folder, settings and Recycle Bin. The owner was offered three
ways out: let the release build honour the test folder, use a separate Windows test user, or something else.

## Decision

1. The release build honours `OPENFRAME_DATA_ROOT=<absolute folder>` for **file locations only**: app data, projects, the
   Global Idea Vault and deleted projects (which go to `<folder>\recycle-bin` instead of the Recycle Bin). As in the debug
   build, such a run does not take the single-instance lock, so it never hands over to the person's own OpenFrame.
2. Every other test switch stays debug-only and is refused by the release build whatever the environment says:
   deterministic AI sampling (`OPENFRAME_AI_DETERMINISTIC`) and the hand-set licence state (`entitlement.debug_set`). A
   unit test in `lib.rs` holds this. (Amended 2026-10-16: the switch for another AI folder, `OPENFRAME_AI_BUNDLE_DIR`, was
   removed. The models live in the test folder's app data since ADR-0026, and the engine-damage spec runs its own copy
   of the release build, so no test needs it.)
3. *(Superseded 2026-10-27: the team password gate was removed, so there is nothing to ask.)* The team password gate is not affected: a release build made with a team password asks for it on a test folder too.

## Consequences

- The suite can drive the release executable without touching the tester's data.
- Anyone who can set an environment variable for a person's session can point OpenFrame at another folder. That person
  can already read and write the same files directly, so no privilege boundary is crossed; the threat model records it.
- Release specs that need a debug-only switch are listed in the release run with the reason they are skipped.
