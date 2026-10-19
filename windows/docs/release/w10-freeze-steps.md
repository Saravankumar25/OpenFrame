# W10 freeze steps

Done after the team pilot ([pilot kit](../testing/pilot-kit/README.md)). Phase: W10 in
[the Windows phases](../roadmap/chapter-1a-windows-phases.md). Nothing is pushed without the owner's word.

## 1. Before freezing

1. No open report in any of the five blocking classes (loses data, corrupts a project, fails migration or recovery, AI
   changes a project on its own, exposes private notes). Check the [triage table](../testing/pilot-kit/triage.md).
2. Every fixed report was re-checked by the person who found it.
3. The team has signed off. Record the names and date in `docs/implementation/progress-log.md` and a summary under
   `docs/release/evidence/w10/`.

## 2. Tag

Tag the commit of the final tested build. Its short hash is in the installer name.

`git tag windows-core-freeze-<date>` (date as YYYY-MM-DD, local tag only; push only on the owner's word)

## 3. Freeze the lists

- **Operation list.** Operations are registered in `crates/openframe-application/src/registry.rs`; `op_names()` returns
  them. No command prints the list yet. Write the names with one run of a small test or example that calls
  `op_names()` (as `tests/platform/permission_matrix.rs` does), save the output as
  `docs/release/evidence/w10/operations.txt` and commit it. Later changes to the list need an owner decision.
- **Scenario list.** The file names in `tests/e2e/specs/` (40 files at the time of writing, from `00-release-smoke` to
  `97-performance`). Save the output of `ls tests/e2e/specs` as `docs/release/evidence/w10/scenarios.txt`.
- The Rust core and the migrations stay as tagged (`migrations/frozen.json` set to frozen, owner's word).

## 4. Proof level

For every Must-ship row in `docs/implementation/feature-status.md`, record proof level 5 with evidence (the pilot summary
and the final suite run). Run `node scripts/check-feature-status.mjs` and `npm run plan:status` for the numbers.

## 5. After

- Update `docs/implementation/handoff.md` and the progress log.
- The Mac track ([chapter 1b](../roadmap/chapter-1b-mac-phases.md)) starts from this tag.
- Run `npm run check:writing` and `npm run check:links`.
