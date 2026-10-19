-- OpenFrame project schema — core (project, members/roles, episodic structure,
-- recoverable deletes, assets, project files, snapshots, notes, tasks, comments,
-- private notes, templates, activity, undo, view state, search).
-- Conventions: docs/engineering/11-physical-database.md
--   * canonical tables: id TEXT PRIMARY KEY (UUIDv7), created_at/updated_at INTEGER (epoch ms),
--     rev INTEGER (optimistic concurrency), deleted_at INTEGER NULL (recoverable delete)
--   * no ON DELETE CASCADE on canonical tables (generic undo must never lose rows silently)
--   * no BLOB columns in canonical tables (binary data lives in files under assets/)
--   * sys_* / search_* / _* tables are infrastructure and are NOT undo-tracked

CREATE TABLE project (
    id              TEXT PRIMARY KEY,
    title           TEXT NOT NULL CHECK (length(trim(title)) > 0),
    project_type    TEXT NOT NULL CHECK (project_type IN ('Feature Film','Short Film','Episodic','Series')),
    status          TEXT NOT NULL DEFAULT 'Idea',
    status_user_selected INTEGER NOT NULL DEFAULT 0,
    language        TEXT,
    genre           TEXT,
    creator         TEXT,
    logline         TEXT,
    archived        INTEGER NOT NULL DEFAULT 0,
    owner_user_id   TEXT NOT NULL,
    settings_json   TEXT NOT NULL DEFAULT '{}',
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    rev             INTEGER NOT NULL DEFAULT 1
);

-- Project members and their security role (Security spec §6).
CREATE TABLE project_member (
    id              TEXT PRIMARY KEY,
    user_id         TEXT NOT NULL UNIQUE,
    display_name    TEXT NOT NULL,
    role            TEXT NOT NULL CHECK (role IN ('Owner','Editor','Commenter','Viewer','ExportOnly')),
    professional_label TEXT,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    rev             INTEGER NOT NULL DEFAULT 1,
    deleted_at      INTEGER
);

-- Episodic structure (Domain §2.4). Episode-scoped content references episode_id.
CREATE TABLE season (
    id          TEXT PRIMARY KEY,
    title       TEXT NOT NULL,
    note        TEXT,
    position    INTEGER NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    rev         INTEGER NOT NULL DEFAULT 1,
    deleted_at  INTEGER
);

CREATE TABLE episode (
    id          TEXT PRIMARY KEY,
    season_id   TEXT REFERENCES season(id),
    title       TEXT NOT NULL,
    summary     TEXT,
    status      TEXT,
    position    INTEGER NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    rev         INTEGER NOT NULL DEFAULT 1,
    deleted_at  INTEGER
);
CREATE INDEX idx_episode_season ON episode(season_id, position);

-- Recoverable deleted state (Domain §12, FSD §52). Restore uses parent/position.
CREATE TABLE deleted_item (
    id              TEXT PRIMARY KEY,
    object_type     TEXT NOT NULL,
    object_id       TEXT NOT NULL,
    table_name      TEXT NOT NULL,
    title           TEXT,
    parent_type     TEXT,
    parent_id       TEXT,
    position        INTEGER,
    deleted_by      TEXT,
    deleted_at      INTEGER NOT NULL,
    restore_until   INTEGER,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    rev             INTEGER NOT NULL DEFAULT 1
);
CREATE UNIQUE INDEX idx_deleted_item_object ON deleted_item(table_name, object_id);

-- Managed or externally referenced binary assets (ESD §9).
CREATE TABLE asset (
    id              TEXT PRIMARY KEY,
    storage_mode    TEXT NOT NULL CHECK (storage_mode IN ('managed','external')),
    rel_path        TEXT,
    external_path   TEXT,
    original_name   TEXT NOT NULL,
    media_type      TEXT NOT NULL,
    byte_size       INTEGER,
    sha256          TEXT,
    width           INTEGER,
    height          INTEGER,
    duration_ms     INTEGER,
    last_seen_at    INTEGER,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    rev             INTEGER NOT NULL DEFAULT 1,
    CHECK ((storage_mode = 'managed' AND rel_path IS NOT NULL) OR (storage_mode = 'external' AND external_path IS NOT NULL))
);

-- Project Files cabinet (FSD §40).
CREATE TABLE project_file_folder (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    parent_id   TEXT REFERENCES project_file_folder(id),
    position    INTEGER NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    rev         INTEGER NOT NULL DEFAULT 1,
    deleted_at  INTEGER
);

CREATE TABLE project_file (
    id          TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    folder_id   TEXT REFERENCES project_file_folder(id),
    asset_id    TEXT NOT NULL REFERENCES asset(id),
    notes       TEXT,
    source      TEXT,
    position    INTEGER NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    rev         INTEGER NOT NULL DEFAULT 1,
    deleted_at  INTEGER
);
CREATE INDEX idx_project_file_folder ON project_file(folder_id, position);

-- Immutable point-in-time snapshots (Domain §4 Snapshot, FSD §126).
CREATE TABLE snapshot (
    id              TEXT PRIMARY KEY,
    snapshot_type   TEXT NOT NULL,
    source_type     TEXT,
    source_id       TEXT,
    source_version  TEXT,
    label           TEXT,
    content_json    TEXT NOT NULL,
    created_by      TEXT,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    rev             INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_snapshot_source ON snapshot(source_type, source_id, created_at);

-- Project notes and lightweight tasks (FSD §106).
CREATE TABLE project_note (
    id          TEXT PRIMARY KEY,
    title       TEXT,
    body        TEXT NOT NULL DEFAULT '',
    pinned      INTEGER NOT NULL DEFAULT 0,
    created_by  TEXT,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    rev         INTEGER NOT NULL DEFAULT 1,
    deleted_at  INTEGER
);

CREATE TABLE task (
    id              TEXT PRIMARY KEY,
    title           TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'Open' CHECK (status IN ('Open','Done')),
    owner_user_id   TEXT,
    due_at          INTEGER,
    notes           TEXT,
    target_type     TEXT,
    target_id       TEXT,
    position        INTEGER NOT NULL,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    rev             INTEGER NOT NULL DEFAULT 1,
    deleted_at      INTEGER
);

-- Comments on any supported object (FSD §23, §94). Replies reference parent_id.
CREATE TABLE comment (
    id              TEXT PRIMARY KEY,
    review_round_id TEXT,
    parent_id       TEXT REFERENCES comment(id),
    target_type     TEXT NOT NULL,
    target_id       TEXT NOT NULL,
    scene_id        TEXT,
    anchor_json     TEXT,
    quoted_text     TEXT,
    body            TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'Open' CHECK (status IN ('Open','In Discussion','Resolved')),
    context_moved   INTEGER NOT NULL DEFAULT 0,
    author_user_id  TEXT NOT NULL,
    author_name     TEXT NOT NULL,
    resolved_by     TEXT,
    resolved_at     INTEGER,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    rev             INTEGER NOT NULL DEFAULT 1,
    deleted_at      INTEGER
);
CREATE INDEX idx_comment_target ON comment(target_type, target_id);
CREATE INDEX idx_comment_round ON comment(review_round_id);

-- Private notes: a distinct authorization boundary (Security §8). Every query
-- MUST filter by owner_user_id = current actor.
CREATE TABLE private_note (
    id              TEXT PRIMARY KEY,
    owner_user_id   TEXT NOT NULL,
    target_type     TEXT,
    target_id       TEXT,
    body            TEXT NOT NULL,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    rev             INTEGER NOT NULL DEFAULT 1,
    deleted_at      INTEGER
);
CREATE INDEX idx_private_note_owner ON private_note(owner_user_id, target_type, target_id);

-- Templates (Domain §20).
CREATE TABLE template (
    id                  TEXT PRIMARY KEY,
    template_type       TEXT NOT NULL,
    name                TEXT NOT NULL,
    content_json        TEXT NOT NULL,
    created_by          TEXT,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1,
    deleted_at          INTEGER
);

-- ---------------------------------------------------------------------------
-- Infrastructure (not undo-tracked)
-- ---------------------------------------------------------------------------

CREATE TABLE sys_activity (
    id          TEXT PRIMARY KEY,
    at          INTEGER NOT NULL,
    actor_id    TEXT,
    actor_name  TEXT,
    action      TEXT NOT NULL,
    summary     TEXT NOT NULL,
    target_type TEXT,
    target_id   TEXT,
    origin      TEXT NOT NULL DEFAULT 'local'
);
CREATE INDEX idx_activity_at ON sys_activity(at DESC);

CREATE TABLE sys_undo (
    seq             INTEGER PRIMARY KEY AUTOINCREMENT,
    label           TEXT NOT NULL,
    actor_id        TEXT,
    coalesce_key    TEXT,
    changes_json    TEXT NOT NULL,
    state           TEXT NOT NULL CHECK (state IN ('done','undone')),
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

CREATE TABLE sys_view_state (
    user_id     TEXT NOT NULL,
    key         TEXT NOT NULL,
    value_json  TEXT NOT NULL,
    updated_at  INTEGER NOT NULL,
    PRIMARY KEY (user_id, key)
);

CREATE TABLE search_doc (
    rowid       INTEGER PRIMARY KEY,
    source_table TEXT NOT NULL,
    entity_id   TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    title       TEXT NOT NULL DEFAULT '',
    body        TEXT NOT NULL DEFAULT '',
    context     TEXT NOT NULL DEFAULT '',
    nav_json    TEXT NOT NULL DEFAULT '{}',
    owner_user_id TEXT,
    updated_at  INTEGER NOT NULL,
    UNIQUE (source_table, entity_id)
);

CREATE VIRTUAL TABLE search_fts USING fts5(
    title, body,
    content='search_doc', content_rowid='rowid',
    tokenize = "unicode61 remove_diacritics 2",
    prefix = '2 3'
);

CREATE TRIGGER search_doc_ai AFTER INSERT ON search_doc BEGIN
    INSERT INTO search_fts(rowid, title, body) VALUES (new.rowid, new.title, new.body);
END;
CREATE TRIGGER search_doc_ad AFTER DELETE ON search_doc BEGIN
    INSERT INTO search_fts(search_fts, rowid, title, body) VALUES ('delete', old.rowid, old.title, old.body);
END;
CREATE TRIGGER search_doc_au AFTER UPDATE ON search_doc BEGIN
    INSERT INTO search_fts(search_fts, rowid, title, body) VALUES ('delete', old.rowid, old.title, old.body);
    INSERT INTO search_fts(rowid, title, body) VALUES (new.rowid, new.title, new.body);
END;
