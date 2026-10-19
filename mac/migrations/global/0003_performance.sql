-- Performance indexes (see migrations/project/0010_performance.sql).
CREATE INDEX idx_undo_actor_state ON sys_undo(actor_id, state, seq);
