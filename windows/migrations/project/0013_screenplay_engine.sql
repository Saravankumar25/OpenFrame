-- Screenplay engine (Screenplay engine and file formats area; contract doc 32 §2.1; ADR-0015).
-- Additive, owned by modules/screenplay:
--   1. custom screenplay elements (definitions per screenplay);
--   2. element rebuild: `page_break` and `custom` become element types, with the custom
--      element reference and an opaque import token kept for round trips;
--   3. production scene number labels (ADR-0015): stored only once frozen or imported;
--   4. page setup, title page behaviour, header and footer per screenplay, and saved presets;
--   5. revision records and the locked pagination baseline used for changed pages.
-- Ids, revs and timestamps of existing elements are kept (doc 23 §4 rule 3).

CREATE TABLE screenplay_custom_element (
    id              TEXT PRIMARY KEY,
    screenplay_id   TEXT NOT NULL REFERENCES screenplay(id),
    name            TEXT NOT NULL,
    -- Keyboard shortcut such as 'Ctrl+Alt+1' (validated by the module), or none.
    shortcut        TEXT,
    -- The built-in element whose position and indent the custom element uses.
    look            TEXT NOT NULL DEFAULT 'action'
                    CHECK (look IN ('action','centered','character','parenthetical','dialogue','transition','shot')),
    uppercase       INTEGER NOT NULL DEFAULT 0,
    italic          INTEGER NOT NULL DEFAULT 0,
    -- Blank lines before the element on the page.
    space_before    INTEGER NOT NULL DEFAULT 1 CHECK (space_before BETWEEN 0 AND 3),
    -- What Enter creates next: a built-in element type or 'custom:<id>'.
    next_element    TEXT NOT NULL DEFAULT 'action',
    -- Import and export identity, for example the Final Draft paragraph type "TELEPHONE".
    export_name     TEXT NOT NULL,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    rev             INTEGER NOT NULL DEFAULT 1,
    deleted_at      INTEGER
);
CREATE INDEX idx_custom_element_screenplay ON screenplay_custom_element(screenplay_id, created_at);

CREATE TABLE screenplay_element_new (
    id                  TEXT PRIMARY KEY,
    scene_id            TEXT NOT NULL REFERENCES screenplay_scene(id),
    position            INTEGER NOT NULL,
    element_type        TEXT NOT NULL CHECK (element_type IN ('scene_heading','action','character','dialogue',
                            'parenthetical','transition','shot','note','page_break','custom')),
    text                TEXT NOT NULL DEFAULT '',
    dual                INTEGER NOT NULL DEFAULT 0,
    revision_mark       TEXT,
    custom_element_id   TEXT REFERENCES screenplay_custom_element(id),
    -- Opaque source token from an imported file (for example the original FDX paragraph type).
    import_source       TEXT,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1,
    CHECK ((element_type = 'custom') = (custom_element_id IS NOT NULL))
);
INSERT INTO screenplay_element_new(id, scene_id, position, element_type, text, dual, revision_mark,
                                   created_at, updated_at, rev)
    SELECT id, scene_id, position, element_type, text, dual, revision_mark, created_at, updated_at, rev
    FROM screenplay_element;
DROP TABLE screenplay_element;
ALTER TABLE screenplay_element_new RENAME TO screenplay_element;
CREATE INDEX idx_element_scene ON screenplay_element(scene_id, position);
CREATE INDEX idx_element_custom ON screenplay_element(custom_element_id) WHERE custom_element_id IS NOT NULL;

-- ADR-0015: a label is never identity. NULL while the number is derived from order.
ALTER TABLE screenplay_scene ADD COLUMN production_number TEXT;
ALTER TABLE screenplay_scene ADD COLUMN production_number_source TEXT
    CHECK (production_number_source IN ('imported','locked','inserted','manual'));

-- Page setup, title page behaviour, header and footer. '{}' means the standard screenplay
-- preset, which reproduces the output from before this migration exactly.
ALTER TABLE screenplay ADD COLUMN page_setup_json TEXT NOT NULL DEFAULT '{}';

-- Project-specific saved page setup presets.
CREATE TABLE screenplay_page_preset (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    settings_json   TEXT NOT NULL,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    rev             INTEGER NOT NULL DEFAULT 1,
    deleted_at      INTEGER
);

-- One record per Start Revision (ADR-0015 decision 4). Label, colour and reason stay on the
-- revision draft row; this record ties the revision to the locked baseline and its date.
CREATE TABLE screenplay_revision (
    id                  TEXT PRIMARY KEY,
    draft_id            TEXT NOT NULL UNIQUE REFERENCES screenplay_draft(id),
    baseline_draft_id   TEXT NOT NULL REFERENCES screenplay_draft(id),
    revision_date       TEXT NOT NULL,
    created_by          TEXT,
    created_by_name     TEXT,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1
);

-- The pagination of a draft at the moment it was locked: page and scene fingerprints that
-- later revisions are compared with to list changed pages and scenes.
CREATE TABLE screenplay_lock_baseline (
    id              TEXT PRIMARY KEY,
    draft_id        TEXT NOT NULL UNIQUE REFERENCES screenplay_draft(id),
    settings_json   TEXT NOT NULL,
    pages_json      TEXT NOT NULL,
    scenes_json     TEXT NOT NULL,
    page_count      INTEGER NOT NULL,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    rev             INTEGER NOT NULL DEFAULT 1
);
