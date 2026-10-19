-- Breakdown, catalog, locations, cast and crew depth (owned by modules/production).
-- Contract: docs/engineering/33-production-depth-contract.md sections 2 and 3. Additive only.

-- Advanced customisation of the eleven breakdown categories. Only overrides are stored; the
-- defaults (colour, icon, label and tooltip) live in code. `id` is the category name, so a
-- category has at most one row. Display metadata only: it never changes breakdown data.
CREATE TABLE breakdown_category_style (
    id          TEXT PRIMARY KEY,
    color       TEXT,
    icon        TEXT,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    rev         INTEGER NOT NULL DEFAULT 1
);

-- Call-sheet facts for a location. Access and parking stay in `notes_json` (already there);
-- the emergency contact is contact data and follows the contact privacy rule.
ALTER TABLE location ADD COLUMN nearest_hospital TEXT;
ALTER TABLE location ADD COLUMN emergency_contact TEXT;

-- Crew members get the same free-text availability note cast members already have.
ALTER TABLE crew_member ADD COLUMN availability_notes TEXT;

-- Availability windows for locations, cast and crew. Dates are ISO calendar days (YYYY-MM-DD),
-- inclusive. Advisory data the schedule reads; it never moves a scene by itself.
CREATE TABLE availability_window (
    id          TEXT PRIMARY KEY,
    owner_type  TEXT NOT NULL CHECK (owner_type IN ('location','cast_member','crew_member')),
    owner_id    TEXT NOT NULL,
    start_date  TEXT NOT NULL,
    end_date    TEXT NOT NULL,
    kind        TEXT NOT NULL CHECK (kind IN ('Available','Unavailable')),
    note        TEXT,
    position    INTEGER NOT NULL DEFAULT 0,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    rev         INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_availability_owner ON availability_window(owner_type, owner_id, start_date);
