-- Scheduling schema: shooting schedules, days, strips, markers, call sheets,
-- sides, production reports, budget snapshots (owned by modules/schedule).
--
-- Source-of-truth rules (FSD §63, Domain §1240):
--   * A schedule references screenplay scenes of one Production Source; it never
--     rewrites screenplay text. Scene numbers are derived (never stored).
--   * Strips are never deleted: a scene removed from the script is flagged and
--     leaves the active schedule only after user confirmation (archived = 1).
--   * Call sheets are documents: prefilled from a shooting day, then edited
--     locally. Edits never flow back into the schedule. Final/Issued call sheets
--     are immutable snapshots (a row in `snapshot`); changes create a new revision.
--   * Budget values are advisory and never change automatically (FSD §39.6).

CREATE TABLE shooting_schedule (
    id                  TEXT PRIMARY KEY,
    source_id           TEXT NOT NULL REFERENCES production_source(id),
    name                TEXT NOT NULL DEFAULT 'Shooting Schedule',
    status              TEXT NOT NULL DEFAULT 'Draft' CHECK (status IN ('Draft','Active','Finalized')),
    -- FSD §36.5: warnings block only when the user explicitly enables strict validation.
    strict_validation   INTEGER NOT NULL DEFAULT 0,
    -- Default target length of a shooting day (FSD §36.4 "entered day duration").
    day_duration_minutes INTEGER NOT NULL DEFAULT 600 CHECK (day_duration_minutes > 0),
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1,
    deleted_at          INTEGER
);

-- Day number is derived from position (off days are not numbered).
CREATE TABLE shooting_day (
    id                  TEXT PRIMARY KEY,
    schedule_id         TEXT NOT NULL REFERENCES shooting_schedule(id),
    position            INTEGER NOT NULL,
    shoot_date          TEXT CHECK (shoot_date IS NULL OR shoot_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
    notes               TEXT,
    -- Per-day target overriding the schedule default.
    planned_minutes     INTEGER CHECK (planned_minutes IS NULL OR planned_minutes > 0),
    is_off_day          INTEGER NOT NULL DEFAULT 0,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1,
    deleted_at          INTEGER
);
CREATE INDEX idx_shooting_day_schedule ON shooting_day(schedule_id, position);

-- A scheduled (or unscheduled) scene. day_id NULL = Unscheduled pool. A strip
-- whose day was deleted also appears in Unscheduled and returns with the day.
CREATE TABLE schedule_strip (
    id                  TEXT PRIMARY KEY,
    schedule_id         TEXT NOT NULL REFERENCES shooting_schedule(id),
    day_id              TEXT REFERENCES shooting_day(id),
    scene_id            TEXT NOT NULL REFERENCES screenplay_scene(id),
    scene_lineage_id    TEXT NOT NULL,
    position            INTEGER NOT NULL DEFAULT 0,
    estimated_minutes   INTEGER CHECK (estimated_minutes IS NULL OR estimated_minutes >= 0),
    -- FSD §145: optional manual page-count override, visibly marked.
    page_eighths_override INTEGER CHECK (page_eighths_override IS NULL OR page_eighths_override > 0),
    -- Source state last reviewed by the user (script-change reconciliation, FSD §56).
    source_heading      TEXT NOT NULL DEFAULT '',
    source_text_hash    TEXT NOT NULL DEFAULT '',
    source_page_eighths INTEGER NOT NULL DEFAULT 0,
    source_state        TEXT NOT NULL DEFAULT 'Current' CHECK (source_state IN ('Current','Changed','Removed','New')),
    change_kinds        TEXT NOT NULL DEFAULT '[]',
    -- 1 = left the active schedule after the user confirmed a script removal (kept for history).
    archived            INTEGER NOT NULL DEFAULT 0,
    notes               TEXT,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1,
    deleted_at          INTEGER
);
CREATE INDEX idx_strip_day ON schedule_strip(schedule_id, day_id, position);
CREATE INDEX idx_strip_lineage ON schedule_strip(scene_lineage_id);

-- Day break markers (FSD §35.9, FSD-SCHED-008). Share the position space of the
-- day's strips so they can sit between scenes.
CREATE TABLE schedule_marker (
    id                  TEXT PRIMARY KEY,
    day_id              TEXT NOT NULL REFERENCES shooting_day(id),
    marker_type         TEXT NOT NULL CHECK (marker_type IN ('Meal','Travel','Company Move','Custom')),
    label               TEXT NOT NULL,
    at_time             TEXT,
    duration_minutes    INTEGER CHECK (duration_minutes IS NULL OR duration_minutes >= 0),
    notes               TEXT,
    position            INTEGER NOT NULL,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1,
    deleted_at          INTEGER
);
CREATE INDEX idx_marker_day ON schedule_marker(day_id, position);

-- "Keep Anyway" / "Dismiss" decisions for advisory warnings (FSD §36.6, §146).
-- detail_hash makes a warning resurface when its content changes.
CREATE TABLE schedule_warning_ack (
    id                  TEXT PRIMARY KEY,
    schedule_id         TEXT NOT NULL REFERENCES shooting_schedule(id),
    warning_key         TEXT NOT NULL,
    detail_hash         TEXT NOT NULL,
    decision            TEXT NOT NULL CHECK (decision IN ('Kept','Dismissed')),
    decided_by          TEXT,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1
);
CREATE UNIQUE INDEX idx_warning_ack_key ON schedule_warning_ack(schedule_id, warning_key);

-- Call sheet document (Domain §4 Call Sheet). shoot_day_id deliberately has no
-- foreign key: issued documents are historical and outlive a purged day.
-- `status` holds the document lifecycle; "Needs Refresh" is also derived live
-- by comparing source_fingerprint with the day's current data (FSD §38.7, §147).
CREATE TABLE call_sheet (
    id                  TEXT PRIMARY KEY,
    schedule_id         TEXT NOT NULL,
    shoot_day_id        TEXT NOT NULL,
    revision            INTEGER NOT NULL DEFAULT 1,
    title               TEXT NOT NULL,
    status              TEXT NOT NULL DEFAULT 'Draft' CHECK (status IN ('Draft','Needs Refresh','Ready','Final','Issued','Superseded')),
    needs_refresh       INTEGER NOT NULL DEFAULT 0,
    source_snapshot_json TEXT NOT NULL,
    source_fingerprint  TEXT NOT NULL,
    source_captured_at  INTEGER NOT NULL,
    document_json       TEXT NOT NULL,
    snapshot_id         TEXT REFERENCES snapshot(id),
    finalized_at        INTEGER,
    finalized_by        TEXT,
    issued_at           INTEGER,
    superseded_by       TEXT REFERENCES call_sheet(id),
    previous_id         TEXT REFERENCES call_sheet(id),
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1,
    deleted_at          INTEGER
);
CREATE INDEX idx_call_sheet_day ON call_sheet(shoot_day_id, revision);

-- Sides: snapshot of selected scenes' screenplay text (FSD §58, §111).
CREATE TABLE side (
    id                  TEXT PRIMARY KEY,
    schedule_id         TEXT,
    shoot_day_id        TEXT,
    title               TEXT NOT NULL,
    scope_json          TEXT NOT NULL,
    source_draft_id     TEXT NOT NULL REFERENCES screenplay_draft(id),
    source_label        TEXT NOT NULL,
    content_json        TEXT NOT NULL,
    created_by          TEXT,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1,
    deleted_at          INTEGER
);

-- Saved report snapshots (FSD §110: nothing persists unless saved).
CREATE TABLE production_report (
    id                  TEXT PRIMARY KEY,
    report_type         TEXT NOT NULL CHECK (report_type IN ('scene','location','cast_scene','prop','schedule','breakdown_completeness')),
    title               TEXT NOT NULL,
    source_scope_json   TEXT NOT NULL,
    snapshot_json       TEXT NOT NULL,
    generated_at        INTEGER NOT NULL,
    created_by          TEXT,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1,
    deleted_at          INTEGER
);

-- Lightweight budget (FSD §39, §109). Amounts are integer minor units
-- (cents/paise). Contingency: percent in basis points (1000 = 10%) or an amount.
-- is_current = 1 for the working budget; saved snapshots are frozen copies.
CREATE TABLE budget_snapshot (
    id                  TEXT PRIMARY KEY,
    label               TEXT,
    currency            TEXT NOT NULL DEFAULT 'USD',
    planned_total       INTEGER CHECK (planned_total IS NULL OR planned_total >= 0),
    contingency_mode    TEXT NOT NULL DEFAULT 'percent' CHECK (contingency_mode IN ('percent','amount')),
    contingency_value   INTEGER NOT NULL DEFAULT 0 CHECK (contingency_value >= 0),
    notes               TEXT,
    is_current          INTEGER NOT NULL DEFAULT 1,
    frozen_at           INTEGER,
    -- Production source the user last reviewed this budget against (FSD §39.6:
    -- a script/source change only prompts a manual "Review budget" reminder).
    reviewed_source_id  TEXT,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1,
    deleted_at          INTEGER
);

CREATE TABLE budget_line (
    id                  TEXT PRIMARY KEY,
    budget_id           TEXT NOT NULL REFERENCES budget_snapshot(id),
    category            TEXT NOT NULL CHECK (category IN ('Cast','Crew','Locations','Equipment','Art/Props','Travel/Transport','Food','Post/Other','Contingency')),
    description         TEXT NOT NULL CHECK (length(trim(description)) > 0),
    amount              INTEGER NOT NULL CHECK (amount >= 0),
    notes               TEXT,
    position            INTEGER NOT NULL,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1,
    deleted_at          INTEGER
);
CREATE INDEX idx_budget_line_budget ON budget_line(budget_id, category, position);
