-- 0011: durable apply journal for multi-operation AI Change Sets (ADR-0019, masterplan M1.01, AI-005).
--
-- Accepting a Change Set writes one row here in the same transaction that marks the Change Set
-- Accepted, and the final "Applied" transaction (or the live failure path) closes it. Every
-- operation in between commits through the normal pipeline together with its undo step, so the
-- undo log after `undo_seq_before` is the exact list of what was applied. A row still in
-- 'applying' when a project opens means the process stopped mid-apply: recovery undoes those
-- steps in reverse order (compensation, ADR-0019 decision 2) and records the outcome.
--
-- `sys_` prefix: system bookkeeping, not canonical content, so it is not undo-tracked and the
-- compensation's own undo steps can never revert the journal.

CREATE TABLE sys_ai_apply_journal (
    change_set_id    TEXT PRIMARY KEY REFERENCES change_set(id),
    actor_user_id    TEXT NOT NULL,
    undo_seq_before  INTEGER NOT NULL,
    total_operations INTEGER NOT NULL CHECK (total_operations >= 0),
    state            TEXT NOT NULL CHECK (state IN ('applying','applied','compensated','compensation_failed')),
    started_at       INTEGER NOT NULL,
    updated_at       INTEGER NOT NULL
);

CREATE INDEX sys_ai_apply_journal_open ON sys_ai_apply_journal(state) WHERE state = 'applying';
