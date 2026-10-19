-- Production schema (owned by modules/production). HUB TABLES are a
-- cross-module contract (schedule, call sheets, visual, AI read them).
-- Production planning references screenplay scenes of the selected Production
-- Source draft; it never rewrites screenplay text (FSD §1.4, §53).

CREATE TABLE production_source (
    id              TEXT PRIMARY KEY,
    draft_id        TEXT NOT NULL REFERENCES screenplay_draft(id),
    active          INTEGER NOT NULL DEFAULT 1,
    selection_reason TEXT,
    selected_by     TEXT,
    selected_at     INTEGER NOT NULL,
    -- The source this one replaced through "Update Production Source" (FSD §53, §161).
    previous_source_id TEXT REFERENCES production_source(id),
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    rev             INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_production_source_active ON production_source(active);

CREATE TABLE catalog_item (
    id              TEXT PRIMARY KEY,
    category        TEXT NOT NULL,
    name            TEXT NOT NULL,
    description     TEXT,
    image_asset_id  TEXT REFERENCES asset(id),
    notes           TEXT,
    contact         TEXT,
    status          TEXT NOT NULL DEFAULT 'Required',
    -- Links a Cast catalog item to a character, a Location/Set item to a location record.
    character_id    TEXT REFERENCES story_character(id),
    location_id     TEXT,
    archived        INTEGER NOT NULL DEFAULT 0,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    rev             INTEGER NOT NULL DEFAULT 1,
    deleted_at      INTEGER
);
CREATE INDEX idx_catalog_category ON catalog_item(category, name);
CREATE INDEX idx_catalog_location ON catalog_item(location_id);

-- Alternative names a catalog item is known by (e.g. "gun" for "Arjun's Pistol").
-- Used by duplicate matching and suggestion matching (FSD §28.5, §143).
CREATE TABLE catalog_alias (
    id              TEXT PRIMARY KEY,
    catalog_item_id TEXT NOT NULL REFERENCES catalog_item(id),
    alias           TEXT NOT NULL,
    alias_key       TEXT NOT NULL,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    rev             INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_catalog_alias_item ON catalog_alias(catalog_item_id);
CREATE INDEX idx_catalog_alias_key ON catalog_alias(alias_key);

-- Scene-level production requirement (Domain §4 Breakdown Element).
-- Only Confirmed/Manual rows are production truth; Suggested rows are pending
-- review and never count as production data (FSD §27.8).
CREATE TABLE breakdown_element (
    id                  TEXT PRIMARY KEY,
    source_id           TEXT NOT NULL REFERENCES production_source(id),
    scene_id            TEXT NOT NULL REFERENCES screenplay_scene(id),
    scene_lineage_id    TEXT NOT NULL,
    category            TEXT NOT NULL,
    catalog_item_id     TEXT REFERENCES catalog_item(id),
    display_name        TEXT NOT NULL,
    notes               TEXT,
    source_evidence     TEXT,
    confirmation_state  TEXT NOT NULL CHECK (confirmation_state IN ('Suggested','Confirmed','Rejected','Manual')),
    origin              TEXT NOT NULL DEFAULT 'manual',
    -- Optional tagged text span: screenplay element id + UTF-16 offsets within its text.
    span_element_id     TEXT,
    span_start          INTEGER,
    span_end            INTEGER,
    -- Plain-language confidence for suggestions ("Likely" / "Possible"); never a raw score.
    confidence          TEXT,
    -- Historical elements (scene removed from the current source) may be archived by the user.
    archived            INTEGER NOT NULL DEFAULT 0,
    -- Set when the linked catalog item was permanently removed; the entry keeps a visible status.
    catalog_removed_at  INTEGER,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1,
    deleted_at          INTEGER
);
CREATE INDEX idx_breakdown_scene ON breakdown_element(scene_id, category);
CREATE INDEX idx_breakdown_catalog ON breakdown_element(catalog_item_id);
CREATE INDEX idx_breakdown_lineage ON breakdown_element(scene_lineage_id);

-- Rejected suggestions: not re-suggested for the same scene + production source (FSD §27.5).
-- Rejecting creates no production data.
CREATE TABLE breakdown_dismissed (
    id                  TEXT PRIMARY KEY,
    source_id           TEXT NOT NULL REFERENCES production_source(id),
    scene_lineage_id    TEXT NOT NULL,
    category            TEXT NOT NULL,
    name_key            TEXT NOT NULL,
    display_name        TEXT NOT NULL,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_breakdown_dismissed ON breakdown_dismissed(source_id, scene_lineage_id);

-- Per-scene production planning state, keyed by stable scene identity (lineage).
-- `complete` is the manual "Breakdown Complete" indicator (FSD §96). The baseline
-- is the script text production planning was based on; when the scene's current
-- text differs, the scene is stale / "Needs Review" (FSD §54, §125).
CREATE TABLE production_scene_state (
    id                  TEXT PRIMARY KEY,
    scene_lineage_id    TEXT NOT NULL UNIQUE,
    complete            INTEGER NOT NULL DEFAULT 0,
    needs_breakdown     INTEGER NOT NULL DEFAULT 0,
    baseline_draft_id   TEXT,
    baseline_heading    TEXT,
    baseline_text       TEXT,
    baseline_fingerprint TEXT,
    reviewed_at         INTEGER,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE location (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    address         TEXT,
    contact         TEXT,
    status          TEXT NOT NULL DEFAULT 'Idea' CHECK (status IN ('Idea','Shortlisted','Confirmed','Rejected')),
    notes_json      TEXT NOT NULL DEFAULT '{}',
    replacement_location_id TEXT REFERENCES location(id),
    archived        INTEGER NOT NULL DEFAULT 0,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    rev             INTEGER NOT NULL DEFAULT 1,
    deleted_at      INTEGER
);

-- Location photos (FSD §29.5): managed copies, ordered; the first is the thumbnail.
CREATE TABLE location_photo (
    id              TEXT PRIMARY KEY,
    location_id     TEXT NOT NULL REFERENCES location(id),
    asset_id        TEXT NOT NULL REFERENCES asset(id),
    caption         TEXT,
    position        INTEGER NOT NULL,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    rev             INTEGER NOT NULL DEFAULT 1,
    deleted_at      INTEGER
);
CREATE INDEX idx_location_photo ON location_photo(location_id, position);

CREATE TABLE cast_member (
    id                  TEXT PRIMARY KEY,
    person_name         TEXT NOT NULL,
    character_id        TEXT REFERENCES story_character(id),
    -- Character as named in the screenplay (character cue), used when no Story
    -- character record exists. Matching is case-insensitive.
    character_name      TEXT,
    is_primary          INTEGER NOT NULL DEFAULT 1,
    contact             TEXT,
    photo_asset_id      TEXT REFERENCES asset(id),
    notes               TEXT,
    availability_notes  TEXT,
    archived            INTEGER NOT NULL DEFAULT 0,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1,
    deleted_at          INTEGER
);

CREATE TABLE crew_member (
    id              TEXT PRIMARY KEY,
    person_name     TEXT NOT NULL,
    role            TEXT NOT NULL,
    department      TEXT,
    contact         TEXT,
    notes           TEXT,
    archived        INTEGER NOT NULL DEFAULT 0,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    rev             INTEGER NOT NULL DEFAULT 1,
    deleted_at      INTEGER
);
