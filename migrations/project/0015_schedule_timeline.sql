-- Shooting schedule depth (docs/engineering/33-production-depth-contract.md section 4).
-- Additive only: the 0007 tables stay as they are and gain nullable columns.
--
--   * Day timeline: crew call, shooting start and target wrap per shooting day.
--     Times are stored as minutes after midnight (0 to 1439), never as display
--     strings. A time earlier than the day's crew call is read as after midnight,
--     so a night shoot can wrap the next morning.
--   * Scene start time: an optional start the user pinned on a strip. Without it
--     the start is computed from the day's order and durations.
--   * Day Out of Days: the report is derived from cast assignments and the
--     schedule; only the user's manual corrections are stored (one row per cast
--     member and day). Corrections never change the screenplay, the breakdown or
--     the schedule.

ALTER TABLE shooting_day ADD COLUMN crew_call INTEGER
    CHECK (crew_call IS NULL OR (crew_call >= 0 AND crew_call < 1440));
ALTER TABLE shooting_day ADD COLUMN shooting_start INTEGER
    CHECK (shooting_start IS NULL OR (shooting_start >= 0 AND shooting_start < 1440));
ALTER TABLE shooting_day ADD COLUMN target_wrap INTEGER
    CHECK (target_wrap IS NULL OR (target_wrap >= 0 AND target_wrap < 1440));

ALTER TABLE schedule_strip ADD COLUMN start_minutes INTEGER
    CHECK (start_minutes IS NULL OR (start_minutes >= 0 AND start_minutes < 1440));

-- Day Out of Days code style: 'industry' (SW, W, WF, SWF, H) or 'simple' (W and -).
ALTER TABLE shooting_schedule ADD COLUMN dood_code_style TEXT NOT NULL DEFAULT 'industry'
    CHECK (dood_code_style IN ('industry', 'simple'));

-- Manual Day Out of Days corrections. cast_key is the schedule's stable cast key
-- (`char:<character id>` or `name:<CHARACTER NAME>`); a correction whose cast
-- member or day no longer exists is ignored, never applied to someone else.
CREATE TABLE dood_override (
    id                  TEXT PRIMARY KEY,
    schedule_id         TEXT NOT NULL REFERENCES shooting_schedule(id),
    cast_key            TEXT NOT NULL CHECK (length(cast_key) BETWEEN 1 AND 300),
    day_id              TEXT NOT NULL REFERENCES shooting_day(id),
    code                TEXT NOT NULL CHECK (code IN ('SW','W','WF','SWF','H','T','R','F','D','-')),
    note                TEXT CHECK (note IS NULL OR length(note) <= 500),
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1
);
CREATE UNIQUE INDEX idx_dood_override_key ON dood_override(schedule_id, cast_key, day_id);
