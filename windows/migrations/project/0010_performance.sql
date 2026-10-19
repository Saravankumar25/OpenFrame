-- Performance indexes found by the Feature-120 perf suite
-- (crates/openframe-application/tests/perf.rs, docs/engineering/21-performance-budgets.md).
--
-- sys_undo is read and trimmed on every mutation and after every data change
-- (history.info). Without an index each of those statements scanned the table,
-- and because `state` is stored after the large `changes_json` column every
-- scanned row also read that row's overflow pages.
CREATE INDEX idx_undo_actor_state ON sys_undo(actor_id, state, seq);
