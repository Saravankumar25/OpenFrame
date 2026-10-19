-- The local AI trace (ADR-0028 decision 8; Windows phase W5): one row per assistant request, numbers
-- and ids only. Never the request text, the answer, project text or a file path. It stays on this
-- computer; the person can write it to a file with Export AI report. Settings shows the AI health
-- card from it.

CREATE TABLE sys_ai_trace (
    id                  TEXT PRIMARY KEY,
    request_id          TEXT NOT NULL,
    project_id          TEXT,
    created_at          INTEGER NOT NULL,
    -- The scope kind (WholeProject, CurrentScene, ...), the provider's model reference and its window.
    scope_kind          TEXT NOT NULL,
    model_reference     TEXT,
    context_tokens      INTEGER,
    -- Tools offered to the model and the tools it ran, by name (comma separated).
    tools_offered       INTEGER NOT NULL DEFAULT 0,
    tools_chosen        TEXT NOT NULL DEFAULT '',
    steps               INTEGER NOT NULL DEFAULT 0,
    sources             INTEGER NOT NULL DEFAULT 0,
    -- Engine numbers summed over the request's model calls.
    model_calls         INTEGER NOT NULL DEFAULT 0,
    tokens_in           INTEGER NOT NULL DEFAULT 0,
    tokens_out          INTEGER NOT NULL DEFAULT 0,
    -- The largest prompt of the request against the window, in percent.
    context_fill_pct    INTEGER NOT NULL DEFAULT 0,
    prompt_ms           INTEGER NOT NULL DEFAULT 0,
    generation_ms       INTEGER NOT NULL DEFAULT 0,
    total_ms            INTEGER NOT NULL DEFAULT 0,
    -- Verifier passes (W6); 1 until the loop exists. Failed checks by name.
    passes              INTEGER NOT NULL DEFAULT 1,
    checks_failed       TEXT NOT NULL DEFAULT '',
    -- Answer, Proposal, Navigate, Clarification, Error ... and the error code, if any.
    result_kind         TEXT NOT NULL,
    error_code          TEXT,
    proposed            INTEGER NOT NULL DEFAULT 0,
    -- What the person did with a proposal: applied, rejected or undone (NULL until they act).
    outcome             TEXT CHECK (outcome IN ('applied', 'rejected', 'undone')),
    outcome_at          INTEGER
);

CREATE INDEX sys_ai_trace_created ON sys_ai_trace(created_at);
CREATE INDEX sys_ai_trace_request ON sys_ai_trace(request_id);
