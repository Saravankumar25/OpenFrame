-- Visual planning schema: moodboards, storyboards, panels, shot lists, shots
-- (owned by modules/visual; FSD §31–34, §55, §101–103).
--
-- Scene links: visual planning follows SCENE IDENTITY (screenplay_scene.lineage_id),
-- never line positions or display numbers (FSD §34.3, §55). `scene_id` records the
-- concrete scene row the object was created from; `scene_lineage_id` is the stable
-- identity used to resolve the scene in the active Production Source draft (or the
-- current draft when production has not started). There is deliberately NO foreign
-- key to screenplay_scene: when a scene disappears from the script its shots and
-- storyboards are kept historically (FSD §55), so they must survive a scene purge.
--
-- `scene_hash` / `scene_heading` are a snapshot of the scene taken when the object
-- was planned (or last marked reviewed). A different current hash flags the scene
-- "Scene changed since planning" — visual planning is never deleted or rewritten.
--
-- Display numbers are NEVER stored: shot labels (12A, 12B…) derive from the scene's
-- display number plus the shot's order in that scene; panel numbers from panel order.

CREATE TABLE moodboard (
    id                  TEXT PRIMARY KEY,
    name                TEXT NOT NULL,
    -- Internal board notes (excluded from standard exports).
    notes               TEXT,
    -- Optional "Moodboard reference" for a screenplay scene (FSD §34.1).
    scene_id            TEXT,
    scene_lineage_id    TEXT,
    position            INTEGER NOT NULL,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1,
    deleted_at          INTEGER
);
CREATE INDEX idx_moodboard_order ON moodboard(position);

-- Freeform canvas item (image | note | link). x/y/w/h are canvas pixels; z is
-- the stacking order (higher = in front).
CREATE TABLE moodboard_item (
    id                  TEXT PRIMARY KEY,
    moodboard_id        TEXT NOT NULL REFERENCES moodboard(id),
    kind                TEXT NOT NULL CHECK (kind IN ('image','note','link')),
    x                   INTEGER NOT NULL DEFAULT 0,
    y                   INTEGER NOT NULL DEFAULT 0,
    w                   INTEGER NOT NULL DEFAULT 200,
    h                   INTEGER NOT NULL DEFAULT 150,
    z                   INTEGER NOT NULL DEFAULT 1,
    caption             TEXT,
    -- Note text (kind = note).
    body                TEXT,
    -- Link target and optional label (kind = link).
    url                 TEXT,
    link_title          TEXT,
    -- Image bytes (kind = image). Shared asset rows; purged only when unreferenced.
    asset_id            TEXT REFERENCES asset(id),
    -- Informational provenance when copied from an Idea Vault item (no live sync).
    source_vault_item_id TEXT,
    -- Internal/private note: excluded from standard exports unless selected (FSD §31.5, §101).
    is_private          INTEGER NOT NULL DEFAULT 0,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1,
    deleted_at          INTEGER
);
CREATE INDEX idx_moodboard_item_board ON moodboard_item(moodboard_id, z);

CREATE TABLE storyboard (
    id                  TEXT PRIMARY KEY,
    name                TEXT NOT NULL,
    -- Optional scene association (FSD §32.6). NULL = standalone storyboard.
    scene_id            TEXT,
    scene_lineage_id    TEXT,
    scene_heading       TEXT,
    scene_hash          TEXT,
    needs_review        INTEGER NOT NULL DEFAULT 0,
    position            INTEGER NOT NULL,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1,
    deleted_at          INTEGER
);
CREATE INDEX idx_storyboard_scene ON storyboard(scene_lineage_id);

CREATE TABLE storyboard_panel (
    id                  TEXT PRIMARY KEY,
    storyboard_id       TEXT NOT NULL REFERENCES storyboard(id),
    -- Visual: 'placeholder' (blank), 'image' (imported) or 'sketch' (drawn in OpenFrame).
    visual_kind         TEXT NOT NULL DEFAULT 'placeholder' CHECK (visual_kind IN ('placeholder','image','sketch')),
    asset_id            TEXT REFERENCES asset(id),
    description         TEXT NOT NULL DEFAULT '',
    framing             TEXT,
    movement            TEXT,
    angle               TEXT,
    -- Dialogue / sound note.
    sound_note          TEXT,
    -- Planned screen time in milliseconds.
    duration_ms         INTEGER,
    note                TEXT,
    -- Optional link to a shot (FSD-STB-001). Several panels may show options for one shot.
    shot_id             TEXT,
    position            INTEGER NOT NULL,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1,
    deleted_at          INTEGER
);
CREATE INDEX idx_panel_board ON storyboard_panel(storyboard_id, position);
CREATE INDEX idx_panel_shot ON storyboard_panel(shot_id);

CREATE TABLE shot (
    id                  TEXT PRIMARY KEY,
    scene_id            TEXT NOT NULL,
    scene_lineage_id    TEXT NOT NULL,
    scene_heading       TEXT,
    scene_hash          TEXT,
    -- Set explicitly (e.g. by a production-source update) or derived from scene_hash;
    -- cleared only when the user marks the scene reviewed.
    needs_review        INTEGER NOT NULL DEFAULT 0,
    -- Order within the scene's coverage → derived label (12A, 12B…).
    position            INTEGER NOT NULL,
    description         TEXT NOT NULL CHECK (length(trim(description)) > 0),
    size                TEXT,
    movement            TEXT,
    angle               TEXT,
    lens                TEXT,
    camera_notes        TEXT,
    -- JSON array of character names, e.g. ["MEERA","ARJUN"].
    characters_json     TEXT NOT NULL DEFAULT '[]',
    sound_note          TEXT,
    -- Visual aid only; not an asset-management relationship (FSD §33.5).
    reference_asset_id  TEXT REFERENCES asset(id),
    -- Primary storyboard panel for this shot (panels also point back via shot_id).
    storyboard_panel_id TEXT,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1,
    deleted_at          INTEGER
);
CREATE INDEX idx_shot_scene ON shot(scene_lineage_id, position);
