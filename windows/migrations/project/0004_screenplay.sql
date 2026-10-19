-- Screenplay schema (owned by modules/screenplay). HUB TABLES are a
-- cross-module contract (production, schedule, visual, AI read them).
-- Display scene numbers are NEVER stored: they are derived from position
-- within a draft (Domain §2.2). Production objects reference screenplay_scene.id.

CREATE TABLE screenplay (
    id                  TEXT PRIMARY KEY,
    episode_id          TEXT REFERENCES episode(id),
    title               TEXT NOT NULL,
    format              TEXT NOT NULL DEFAULT 'Feature' CHECK (format IN ('Feature','Short','Episodic')),
    current_draft_id    TEXT,
    title_page_json     TEXT NOT NULL DEFAULT '{}',
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1,
    deleted_at          INTEGER
);

CREATE TABLE screenplay_draft (
    id                  TEXT PRIMARY KEY,
    screenplay_id       TEXT NOT NULL REFERENCES screenplay(id),
    name                TEXT NOT NULL,
    note                TEXT,
    status              TEXT NOT NULL DEFAULT 'Draft' CHECK (status IN ('Draft','Review','Locked','Revision')),
    created_from_draft_id TEXT REFERENCES screenplay_draft(id),
    locked_at           INTEGER,
    revision_label      TEXT,
    revision_color      TEXT,
    -- Added by the screenplay module (FSD §24.6, §95): why the revision exists and
    -- who locked the draft (lock timestamp + user identity are recorded).
    revision_reason     TEXT,
    locked_by           TEXT,
    locked_by_name      TEXT,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1,
    deleted_at          INTEGER
);
CREATE INDEX idx_draft_screenplay ON screenplay_draft(screenplay_id, created_at);

-- A scene belongs to exactly one draft. When a new draft is created from an
-- existing one, scenes are copied with new ids and lineage_id carries the
-- stable cross-draft scene identity used for comparison and production
-- reconciliation (FSD §22.6, §53–56).
CREATE TABLE screenplay_scene (
    id                  TEXT PRIMARY KEY,
    draft_id            TEXT NOT NULL REFERENCES screenplay_draft(id),
    lineage_id          TEXT NOT NULL,
    position            INTEGER NOT NULL,
    heading             TEXT NOT NULL DEFAULT '',
    synopsis            TEXT,
    notes               TEXT,
    story_day           TEXT,
    time_note           TEXT,
    source_scene_card_id TEXT,
    omitted             INTEGER NOT NULL DEFAULT 0,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1,
    deleted_at          INTEGER
);
CREATE INDEX idx_scene_draft ON screenplay_scene(draft_id, position);
CREATE INDEX idx_scene_lineage ON screenplay_scene(lineage_id);

CREATE TABLE screenplay_element (
    id              TEXT PRIMARY KEY,
    scene_id        TEXT NOT NULL REFERENCES screenplay_scene(id),
    position        INTEGER NOT NULL,
    element_type    TEXT NOT NULL CHECK (element_type IN ('scene_heading','action','character','dialogue','parenthetical','transition','shot','note')),
    text            TEXT NOT NULL DEFAULT '',
    dual            INTEGER NOT NULL DEFAULT 0,
    revision_mark   TEXT,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    rev             INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_element_scene ON screenplay_element(scene_id, position);

-- ---------------------------------------------------------------------------
-- Module tables (screenplay module; not hub contracts)
-- ---------------------------------------------------------------------------

-- Review rounds (FSD §23, Domain "Review Round"). Comments reference them via
-- comment.review_round_id. Completing a review never locks the draft.
CREATE TABLE review_round (
    id              TEXT PRIMARY KEY,
    draft_id        TEXT NOT NULL REFERENCES screenplay_draft(id),
    name            TEXT NOT NULL,
    reviewers_json  TEXT NOT NULL DEFAULT '[]',
    deadline        TEXT,
    status          TEXT NOT NULL DEFAULT 'Open' CHECK (status IN ('Open','Complete')),
    created_by      TEXT,
    created_by_name TEXT,
    completed_at    INTEGER,
    completed_by    TEXT,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    rev             INTEGER NOT NULL DEFAULT 1,
    deleted_at      INTEGER
);
CREATE INDEX idx_review_round_draft ON review_round(draft_id, created_at);

-- Automatic History Points (FSD §21.1–21.2, Domain "Automatic History Point"):
-- recoverable snapshots of one draft's written content, recorded periodically
-- while editing and when a draft is created. Restoring one creates a NEW draft;
-- a history point is never a deliverable draft.
CREATE TABLE screenplay_history_point (
    id              TEXT PRIMARY KEY,
    draft_id        TEXT NOT NULL REFERENCES screenplay_draft(id),
    reason          TEXT NOT NULL CHECK (reason IN ('interval','draft_created')),
    scene_count     INTEGER NOT NULL DEFAULT 0,
    element_count   INTEGER NOT NULL DEFAULT 0,
    snapshot_json   TEXT NOT NULL,
    created_by      TEXT,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    rev             INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_history_point_draft ON screenplay_history_point(draft_id, created_at);

-- User corrections to character-cue detection: map a cue (normalized, e.g.
-- "MEERA") to a Story character, or mark it as not a character. character_id
-- is deliberately not a foreign key: the Story module owns character purge.
CREATE TABLE screenplay_character_link (
    id              TEXT PRIMARY KEY,
    screenplay_id   TEXT NOT NULL REFERENCES screenplay(id),
    cue_name        TEXT NOT NULL,
    character_id    TEXT,
    ignored         INTEGER NOT NULL DEFAULT 0,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    rev             INTEGER NOT NULL DEFAULT 1,
    UNIQUE (screenplay_id, cue_name)
);

-- Infrastructure (not undo-tracked): a per-draft edit sequence so the editor can
-- tell a stale document read from a fresh one after its own saves.
CREATE TABLE sys_screenplay_sync (
    draft_id    TEXT PRIMARY KEY,
    seq         INTEGER NOT NULL
);
