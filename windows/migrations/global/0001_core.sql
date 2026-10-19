-- Global Idea Vault store (Documents/OpenFrame/Global Idea Vault/vault.sqlite).
-- Infrastructure shared with the project store so the same command pipeline
-- (undo, activity, recoverable delete, search, assets) works unchanged.
-- Vault item tables are added by migration 0002.

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
