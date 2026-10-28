-- 0018: the assistant's long-term memory (spec ai-conversational-harness, section 5.7; slice S6).
--
-- One row is one thing the person stated outright ("I'm the assistant director", "always answer in
-- short sentences") or confirmed ("yes, that's right"). Rules, all in code (modules/ai/memory):
--   * Personal: every query filters by user_id. Another member of the project never sees the row.
--   * Stays on this computer: a Full Project Package leaves the table empty.
--   * Deleting is a flag plus wiping the text. It is not Recently Deleted: the person asked to be
--     forgotten, so the words must not wait in a bin. The row (id, kind, times) is a harmless tombstone.
--   * `ai_` prefix: an AI bookkeeping table, never undo history or an activity entry.

CREATE TABLE ai_memory (
    id                TEXT PRIMARY KEY,
    user_id           TEXT NOT NULL,
    text              TEXT NOT NULL,
    kind              TEXT NOT NULL CHECK (kind IN ('fact', 'preference')),
    -- Ids of the things the entry is about (screenplay, character, person ...); a JSON array of strings.
    entity_refs_json  TEXT NOT NULL DEFAULT '[]',
    source_request_id TEXT,
    created_at        INTEGER NOT NULL,
    deleted_at        INTEGER
);

CREATE INDEX idx_ai_memory_user ON ai_memory(user_id, deleted_at, created_at);
