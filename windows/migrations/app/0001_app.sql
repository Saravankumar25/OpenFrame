-- OpenFrame application database (%LOCALAPPDATA%/OpenFrame/app.sqlite).
-- Holds application-level, non-project state only. Project content never lives here.

CREATE TABLE sys_settings (
    key         TEXT PRIMARY KEY,
    value_json  TEXT NOT NULL,
    updated_at  INTEGER NOT NULL
);

-- The local Application User (Security §5): stable identity, no account.
CREATE TABLE sys_local_profile (
    user_id         TEXT PRIMARY KEY,
    display_name    TEXT NOT NULL,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

-- Recent/known projects shown on the Home screen. A view registry, not truth:
-- the project folder remains authoritative for its own title/type/status.
CREATE TABLE sys_recent_project (
    project_id      TEXT PRIMARY KEY,
    path            TEXT NOT NULL,
    title           TEXT NOT NULL,
    project_type    TEXT NOT NULL,
    status          TEXT NOT NULL,
    archived        INTEGER NOT NULL DEFAULT 0,
    pinned          INTEGER NOT NULL DEFAULT 0,
    last_opened_at  INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);
CREATE INDEX idx_recent_opened ON sys_recent_project(archived, last_opened_at DESC);

-- Global templates usable by any project (Domain §20).
CREATE TABLE sys_global_template (
    id              TEXT PRIMARY KEY,
    template_type   TEXT NOT NULL,
    name            TEXT NOT NULL,
    content_json    TEXT NOT NULL,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);
