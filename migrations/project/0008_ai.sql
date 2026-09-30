-- AI schema: AI requests, results, tool invocations, Change Sets
-- (owned by modules/ai). Domain §4 AI Request / AI Result / AI Tool Invocation /
-- Change Set; AI spec §50.
--
-- Rules:
--   * No hidden model chain-of-thought and no full-context copies are stored:
--     results keep the user-facing content, deterministic structured values and
--     provenance references only (Local AI Runtime spec §13).
--   * AI records are personal: queries filter by user_id / requesting_user_id.
--   * The AI never writes canonical tables. An accepted Change Set is applied
--     through normal registry operations (see modules/ai/change_set.rs).

CREATE TABLE ai_request (
    id                          TEXT PRIMARY KEY,
    user_id                     TEXT NOT NULL,
    project_id                  TEXT,
    -- Conversation / session context (follow-ups reuse scope, never permissions).
    session_id                  TEXT NOT NULL,
    scope_kind                  TEXT NOT NULL,
    -- Resolved scope references (ids + human label), never content copies.
    scope_json                  TEXT NOT NULL DEFAULT '{}',
    scope_label                 TEXT NOT NULL,
    request_text                TEXT NOT NULL,
    -- Interpreted operation (tool name) once known.
    intent                      TEXT,
    operation_class             TEXT CHECK (operation_class IN ('Read','Compute','Navigate','Suggest','Mutate')),
    target_objects_json         TEXT NOT NULL DEFAULT '[]',
    authorization_state         TEXT NOT NULL DEFAULT 'Pending' CHECK (authorization_state IN ('Pending','Allowed','Denied')),
    -- Local AI only: always 'Local' (or 'Not Sent' when no model ran).
    external_processing_state   TEXT NOT NULL DEFAULT 'Local' CHECK (external_processing_state IN ('Local','Not Sent')),
    model_reference             TEXT,
    status                      TEXT NOT NULL CHECK (status IN ('Created','Resolved','Running','Completed','Failed','Canceled')),
    completed_at                INTEGER,
    created_at                  INTEGER NOT NULL,
    updated_at                  INTEGER NOT NULL,
    rev                         INTEGER NOT NULL DEFAULT 1,
    deleted_at                  INTEGER
);
CREATE INDEX idx_ai_request_session ON ai_request(user_id, session_id, created_at);

CREATE TABLE ai_result (
    id                  TEXT PRIMARY KEY,
    request_id          TEXT NOT NULL REFERENCES ai_request(id),
    -- Answer | Navigate | Suggestion | Proposal | Denied | Private | Clarify | Unavailable | Failed
    result_kind         TEXT NOT NULL,
    content             TEXT NOT NULL,
    details_json        TEXT NOT NULL DEFAULT '[]',
    structured_json     TEXT,
    provenance_json     TEXT NOT NULL DEFAULT '[]',
    nav_json            TEXT,
    change_set_id       TEXT,
    confidence_state    TEXT CHECK (confidence_state IN ('Exact','Inferred','Unavailable')),
    status              TEXT NOT NULL CHECK (status IN ('Informational','Pending Approval','Accepted','Rejected','Applied','Stale','Conflict','Failed')),
    error_code          TEXT,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1,
    deleted_at          INTEGER
);
CREATE INDEX idx_ai_result_request ON ai_result(request_id);

CREATE TABLE ai_tool_invocation (
    id                      TEXT PRIMARY KEY,
    request_id              TEXT NOT NULL REFERENCES ai_request(id),
    tool_name               TEXT NOT NULL,
    parameters_json         TEXT NOT NULL DEFAULT '{}',
    target_objects_json     TEXT NOT NULL DEFAULT '[]',
    authorization_state     TEXT NOT NULL CHECK (authorization_state IN ('Allowed','Denied')),
    execution_state         TEXT NOT NULL CHECK (execution_state IN ('Pending','Running','Succeeded','Failed','Blocked')),
    result_reference        TEXT,
    error_code              TEXT,
    created_at              INTEGER NOT NULL,
    updated_at              INTEGER NOT NULL,
    rev                     INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_ai_tool_invocation_request ON ai_tool_invocation(request_id);

-- A bounded, previewable set of registry operations (Domain §4 Change Set).
CREATE TABLE change_set (
    id                      TEXT PRIMARY KEY,
    origin                  TEXT NOT NULL DEFAULT 'AI' CHECK (origin IN ('User','Import','Review','AI')),
    ai_request_id           TEXT REFERENCES ai_request(id),
    ai_result_id            TEXT,
    requesting_user_id      TEXT NOT NULL,
    -- Role at preparation time; a permission change makes the proposal stale.
    requesting_role         TEXT NOT NULL,
    approver_user_id        TEXT,
    approved_at             INTEGER,
    approval_scope          TEXT,
    title                   TEXT NOT NULL,
    summary                 TEXT NOT NULL DEFAULT '',
    target_objects_json     TEXT NOT NULL DEFAULT '[]',
    affected_modules_json   TEXT NOT NULL DEFAULT '[]',
    -- Ordered registry operation calls: [{"op": "module.action", "args": {...}, "label": "..."}]
    operations_json         TEXT NOT NULL,
    -- Proposal tool + validated arguments, so "Re-check & Review" can rebuild it.
    source_json             TEXT NOT NULL DEFAULT '{}',
    -- Preview rows shown before acceptance (counts, exclusions, locked targets).
    preview_json            TEXT NOT NULL DEFAULT '[]',
    exclusions_json         TEXT NOT NULL DEFAULT '[]',
    -- Base version: {"projectId": "...", "rows": [{"table","id","rev"}], "absent": [{"table","id"}]}
    base_version            TEXT NOT NULL,
    review_state            TEXT NOT NULL DEFAULT 'Pending' CHECK (review_state IN ('Pending','Accepted','Rejected','Applied','Stale','Conflict','Failed')),
    validation_state        TEXT NOT NULL DEFAULT 'Not Checked' CHECK (validation_state IN ('Not Checked','Valid','Invalid','Needs Review')),
    stale_reason            TEXT,
    error_message           TEXT,
    applied_at              INTEGER,
    applied_operations      INTEGER NOT NULL DEFAULT 0,
    created_at              INTEGER NOT NULL,
    updated_at              INTEGER NOT NULL,
    rev                     INTEGER NOT NULL DEFAULT 1,
    deleted_at              INTEGER
);
CREATE INDEX idx_change_set_request ON change_set(ai_request_id);
CREATE INDEX idx_change_set_user ON change_set(requesting_user_id, review_state);
