# ADR-0019: Crash-safe application of multi-operation AI Change Sets

- **Status:** Accepted 2026-10-05 (owner decision: crash-safe AI changes; the owner delegated the choice to the recorded recommendation in the first governance session). Amends ADR-0013 (Change Set apply).
- **Date:** 2026-10-05
- **Related:** ADR-0005, ADR-0013; `crates/openframe-application/src/modules/ai/change_set.rs`;
  `docs/history/audits/2026-10-02-production-and-store-readiness-audit.md` §10

## Context

A Change Set holds several ordinary Registry commands. Apply marks the Change Set `Accepted`, dispatches each
command in its own transaction, and on a failure while the process is alive runs Undo for the commands already
applied, then records the terminal state. If the process is killed or power fails after operation 2 of 3,
operations 1 and 2 stay committed, the Change Set stays `Accepted` and no compensation runs. A repository search of
`change_set.rs` finds no apply journal or startup recovery path, so the gap is real in the current source.

## Decision

1. Add a durable **apply journal** in the project database: a batch id plus one row per operation boundary
   (`pending`, `applied`, `compensated`), written in the same transaction as each operation where possible.
2. On project open, detect Change Sets stuck in `Accepted` with an unfinished journal and resolve them
   deterministically: **compensate** (undo applied operations in reverse) as the default, because partial AI edits
   are the worse outcome. Show the user what happened in Activity.
3. Preserve existing behaviour for the live-process failure path and keep "applied proposal is one coherent undo step".
4. Operations with filesystem effects are listed and tested separately because they cannot share one transaction.
5. Test by killing the process after every operation boundary and asserting the project reopens consistent.

## Consequences

- Needs a new additive project migration and recovery code on open.
- Adds a forced-kill test matrix to CI (also useful for the save and migration paths).

## Implementation (2026-10-05, build plan items "crash-safe AI changes" and "forced stop at every apply step")

- Migration `0011_ai_apply_journal.sql`: table `sys_ai_apply_journal` (untracked `sys_` prefix, so compensation's
  own undo steps can never revert it).
- `modules/ai/change_set.rs`: the journal row (`undo_seq_before`, `total_operations`, state `applying`) is written in
  the same transaction that marks the Change Set `Accepted`; it closes as `applied` in the transaction that marks it
  `Applied`, or as `compensated` / `compensation_failed` on the live failure path (decision 3 unchanged).
- `recover_interrupted_applies` runs in `project::open_at` before the session starts: every journal still `applying`
  is compensated by undoing the applier's steps after `undo_seq_before` in reverse, the resulting redo entries are
  discarded, the Change Set ends `Failed` with a plain message, and Activity records "Undid an unfinished AI change".
  A recovery error is logged and never blocks opening; the journal stays open and is retried on the next open.
- Decision 4: every operation a proposal may use is checked against its registry `filesystem_effect`; the two that
  declare one (`story.create_character`, `storyboard.add_panel`) are reviewed exceptions because no proposal schema can
  produce their file arguments, and apply refuses any Change Set operation with a declared filesystem effect that
  carries a file argument (`FILE_ARGUMENTS`).
- Decision 5: `crates/openframe-application/tests/ai_apply_recovery.rs` stops the apply at every boundary (after
  acceptance, after each operation, after grouping) through a debug-only fault-injection point and reopens the project
  as after a crash; 8 tests.
- Review (2026-10-05, Rust review of this change) led to: the undo seal, the `undo_seq_before` read and the journal
  insert share the accept transaction; the live failure state and the journal close share one transaction; only an
  undo conflict ends compensation as `compensation_failed`, other errors leave the journal `applying` for the next
  open; `abandon_open_journals` closes open journals as `compensation_failed` wherever a project copy loses its undo
  history (duplicate, backup scrub, package restore as copy), so recovery never reports a compensation it could not do.
- Known limits, recorded rather than hidden: a crash after every operation committed but before `Applied` is still
  compensated (the default of decision 2; partial and complete are not distinguished); an undo conflict during
  compensation follows the general undo rule and drops that user's older undo history; an edit by the same user from a
  second window between acceptance and the first operation would be counted as part of the apply (single window today).
