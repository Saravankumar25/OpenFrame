-- Story schema (owned by modules/story). HUB TABLES below are a cross-module
-- contract: other modules may read these columns; the story module may add
-- columns/tables but must not rename or remove contract columns.
-- Hierarchy (Domain §4, FSD §7.3): Act → Sequence → {Beat, Scene Card};
-- Scene Cards/Beats may sit directly under an Act; parent_type 'parking' =
-- Parking Lot; parent_type 'unassigned' = restored items whose container is gone.
-- episode_id scopes story content to an episode in episodic projects (NULL otherwise).
--
-- Ordering: children of one container share ONE position space across tables
-- (an Act's sequences, beats and cards interleave; a Sequence's beats and cards
-- interleave). Positions are user order only; nothing is ever numbered.
-- A sequence whose act is gone (act_id NULL) lives in the "Unassigned" area.
-- Deleting a container "with everything inside" soft-deletes the children with
-- the SAME deleted_at as the container and no deleted_item row of their own, so
-- restoring/purging the container restores/purges them together.

CREATE TABLE story_act (
    id          TEXT PRIMARY KEY,
    episode_id  TEXT REFERENCES episode(id),
    title       TEXT NOT NULL,
    note        TEXT,
    collapsed   INTEGER NOT NULL DEFAULT 0,
    position    INTEGER NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    rev         INTEGER NOT NULL DEFAULT 1,
    deleted_at  INTEGER
);
CREATE INDEX idx_story_act_order ON story_act(episode_id, position);

CREATE TABLE story_sequence (
    id          TEXT PRIMARY KEY,
    act_id      TEXT REFERENCES story_act(id),
    title       TEXT NOT NULL,
    note        TEXT,
    collapsed   INTEGER NOT NULL DEFAULT 0,
    position    INTEGER NOT NULL,
    -- Scope of a sequence that currently has no act (Unassigned area).
    episode_id  TEXT REFERENCES episode(id),
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    rev         INTEGER NOT NULL DEFAULT 1,
    deleted_at  INTEGER
);
CREATE INDEX idx_story_sequence_act ON story_sequence(act_id, position);

CREATE TABLE story_beat (
    id              TEXT PRIMARY KEY,
    episode_id      TEXT REFERENCES episode(id),
    parent_type     TEXT NOT NULL CHECK (parent_type IN ('act','sequence','parking','unassigned')),
    parent_id       TEXT,
    text            TEXT NOT NULL DEFAULT '',
    note            TEXT,
    color           TEXT,
    state           TEXT NOT NULL DEFAULT 'active' CHECK (state IN ('active','converted')),
    converted_scene_card_id TEXT,
    source_vault_item_id TEXT,
    -- Where a parked beat came from, so "Restore to Story" can return it (FSD §89.9).
    parked_from_type TEXT CHECK (parked_from_type IS NULL OR parked_from_type IN ('act','sequence')),
    parked_from_id   TEXT,
    parked_from_index INTEGER,
    parked_at        INTEGER,
    position        INTEGER NOT NULL,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    rev             INTEGER NOT NULL DEFAULT 1,
    deleted_at      INTEGER
);
CREATE INDEX idx_story_beat_parent ON story_beat(parent_type, parent_id, position);

CREATE TABLE story_scene_card (
    id                  TEXT PRIMARY KEY,
    episode_id          TEXT REFERENCES episode(id),
    parent_type         TEXT NOT NULL CHECK (parent_type IN ('act','sequence','parking','unassigned')),
    parent_id           TEXT,
    short_description   TEXT NOT NULL DEFAULT '',
    scene_heading       TEXT,
    notes               TEXT,
    color               TEXT,
    -- "used to create screenplay" reference (informational; no live sync, FSD §11.10).
    screenplay_scene_id TEXT,
    source_beat_id      TEXT,
    source_vault_item_id TEXT,
    parked_from_type    TEXT CHECK (parked_from_type IS NULL OR parked_from_type IN ('act','sequence')),
    parked_from_id      TEXT,
    parked_from_index   INTEGER,
    parked_at           INTEGER,
    position            INTEGER NOT NULL,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1,
    deleted_at          INTEGER
);
CREATE INDEX idx_story_card_parent ON story_scene_card(parent_type, parent_id, position);
CREATE INDEX idx_story_card_scope ON story_scene_card(episode_id, parent_type);

-- Characters: project story/production directory (FSD §13). Series-level
-- characters have episode_id NULL.
CREATE TABLE story_character (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    role_label  TEXT,
    description TEXT,
    image_asset_id TEXT REFERENCES asset(id),
    notes       TEXT,
    archived    INTEGER NOT NULL DEFAULT 0,
    episode_id  TEXT REFERENCES episode(id),
    position    INTEGER NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    rev         INTEGER NOT NULL DEFAULT 1,
    deleted_at  INTEGER
);
CREATE INDEX idx_story_character_order ON story_character(position);

-- Lightweight narrative relationship between two characters (Domain §4
-- Character Relationship). relationship_type is freeform, e.g. "father of".
CREATE TABLE story_character_relationship (
    id                  TEXT PRIMARY KEY,
    from_character_id   TEXT NOT NULL REFERENCES story_character(id),
    to_character_id     TEXT NOT NULL REFERENCES story_character(id),
    relationship_type   TEXT NOT NULL,
    note                TEXT,
    position            INTEGER NOT NULL,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1,
    CHECK (from_character_id <> to_character_id)
);
CREATE INDEX idx_story_rel_from ON story_character_relationship(from_character_id);
CREATE INDEX idx_story_rel_to ON story_character_relationship(to_character_id);

-- Optional manual association of a character with a Scene Card (FSD §13.4).
-- Reference only: never required for a card to exist.
CREATE TABLE story_character_card_link (
    id              TEXT PRIMARY KEY,
    character_id    TEXT NOT NULL REFERENCES story_character(id),
    scene_card_id   TEXT NOT NULL REFERENCES story_scene_card(id),
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    rev             INTEGER NOT NULL DEFAULT 1,
    UNIQUE (character_id, scene_card_id)
);
CREATE INDEX idx_story_link_card ON story_character_card_link(scene_card_id);

-- Reference attachments on Scene Cards, Beats and Sequences (FSD §11.4, §90.3).
-- They never become production assets automatically.
CREATE TABLE story_attachment (
    id          TEXT PRIMARY KEY,
    owner_type  TEXT NOT NULL CHECK (owner_type IN ('scene_card','beat','sequence')),
    owner_id    TEXT NOT NULL,
    asset_id    TEXT NOT NULL REFERENCES asset(id),
    position    INTEGER NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    rev         INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_story_attachment_owner ON story_attachment(owner_type, owner_id, position);
