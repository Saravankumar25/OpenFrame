-- Project Idea Vault schema (owned by crates/openframe-application/src/modules/idea_vault).
-- IDENTICAL to migrations/global/0002_idea_vault.sql: the same module code serves
-- both stores (FSD §5.8). Keep the two files byte-for-byte equal below this header.
--
-- Rules (FSD §5, §88; Domain §4 Global/Project Idea Vault Item, Vault Collection):
-- * Every metadata field is optional; an untitled item is valid (FSD §5.4).
--   `title` holds ONLY a user-entered title — generated display names are never stored.
-- * One folder per item (organization only); collections are N:M; tags are free text.
-- * Files live in the shared `asset` table (managed copy or external reference).
-- * Copies between Global and Project stores are independent rows with new ids;
--   `source_global_item_id` is informational only (no live sync, FSD §5.8/§88.9).

CREATE TABLE vault_folder (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    parent_id   TEXT REFERENCES vault_folder(id),
    position    INTEGER NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    rev         INTEGER NOT NULL DEFAULT 1,
    deleted_at  INTEGER
);
CREATE INDEX idx_vault_folder_parent ON vault_folder(parent_id, position);

CREATE TABLE vault_collection (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    note        TEXT,
    position    INTEGER NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    rev         INTEGER NOT NULL DEFAULT 1,
    deleted_at  INTEGER
);

CREATE TABLE vault_item (
    id              TEXT PRIMARY KEY,
    item_type       TEXT NOT NULL CHECK (item_type IN
                        ('note','image','url','pdf','document','audio','voice','video',
                         'sketch','quote','screenshot','file')),
    title           TEXT,
    body            TEXT,
    caption         TEXT,
    url             TEXT,
    source_text     TEXT,
    asset_id        TEXT REFERENCES asset(id),
    folder_id       TEXT REFERENCES vault_folder(id),
    pinned          INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0,1)),
    pinned_at       INTEGER,
    source_global_item_id TEXT,
    created_by      TEXT,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    rev             INTEGER NOT NULL DEFAULT 1,
    deleted_at      INTEGER
);
CREATE INDEX idx_vault_item_created ON vault_item(deleted_at, created_at DESC);
CREATE INDEX idx_vault_item_folder ON vault_item(folder_id);
CREATE INDEX idx_vault_item_asset ON vault_item(asset_id);

CREATE TABLE vault_item_collection (
    id              TEXT PRIMARY KEY,
    item_id         TEXT NOT NULL REFERENCES vault_item(id),
    collection_id   TEXT NOT NULL REFERENCES vault_collection(id),
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    rev             INTEGER NOT NULL DEFAULT 1,
    UNIQUE (item_id, collection_id)
);
CREATE INDEX idx_vault_item_collection_coll ON vault_item_collection(collection_id);

CREATE TABLE vault_item_tag (
    id          TEXT PRIMARY KEY,
    item_id     TEXT NOT NULL REFERENCES vault_item(id),
    tag         TEXT NOT NULL COLLATE NOCASE,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    rev         INTEGER NOT NULL DEFAULT 1,
    UNIQUE (item_id, tag)
);
CREATE INDEX idx_vault_item_tag_tag ON vault_item_tag(tag);
