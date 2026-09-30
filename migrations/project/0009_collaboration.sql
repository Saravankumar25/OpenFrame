-- Portability schema (owned by modules/packages): package log, import sessions,
-- the review queue for imported notes and separate review records.
-- LAN collaboration was removed from scope: collaboration is file-based only
-- (exchange / review / response packages, Import/Export spec §9–§20).
--
-- sys_* tables are infrastructure (NOT undo-tracked, not project content):
--   * the Import Session is a temporary review context (Import/Export §12), and
--     undoing an applied import must never erase the record that it happened.
-- review_queue_item / exchange_review_record are canonical project content:
-- retained review material that the user decides about later (§17, §15).

-- Every package this project exported or imported (for the import/export report).
CREATE TABLE sys_package_log (
    id              TEXT PRIMARY KEY,
    direction       TEXT NOT NULL CHECK (direction IN ('export','import')),
    package_type    TEXT NOT NULL,
    package_id      TEXT NOT NULL,
    file_name       TEXT NOT NULL,
    summary         TEXT NOT NULL,
    session_id      TEXT,
    actor_id        TEXT,
    actor_name      TEXT,
    at              INTEGER NOT NULL
);
CREATE INDEX idx_package_log_at ON sys_package_log(at DESC);

-- Import Session (Import/Export §12): source package, detected type,
-- compatibility, identities, versions, the computed preview (mappings,
-- conflicts, warnings, proposed operations), the user's selections and the
-- final result. content_json keeps the validated package content so the
-- preview can be recomputed against the current project state at apply time.
CREATE TABLE sys_import_session (
    id                  TEXT PRIMARY KEY,
    package_id          TEXT NOT NULL,
    package_type        TEXT NOT NULL,
    file_name           TEXT NOT NULL,
    package_sha256      TEXT NOT NULL,
    source_project_id   TEXT NOT NULL,
    source_project_title TEXT NOT NULL,
    compatibility       TEXT NOT NULL CHECK (compatibility IN ('Valid','Stale','Rejected')),
    manifest_json       TEXT NOT NULL,
    content_json        TEXT NOT NULL,
    preview_json        TEXT NOT NULL,
    status              TEXT NOT NULL DEFAULT 'Previewed'
                        CHECK (status IN ('Previewed','Applied','PartiallyApplied','PendingReview','Rejected','Failed','Cancelled','Undone')),
    apply_mode          TEXT,
    selection_json      TEXT,
    result_json         TEXT,
    apply_actor_id      TEXT,
    apply_actor_name    TEXT,
    undo_from_seq       INTEGER,
    undo_to_seq         INTEGER,
    safety_backup       TEXT,
    created_by          TEXT,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL
);
CREATE INDEX idx_import_session_created ON sys_import_session(created_at DESC);
CREATE INDEX idx_import_session_package ON sys_import_session(package_id);

-- Review Queue (Import/Export §16–§17; mockup 163): imported comments whose
-- context is ambiguous or no longer exists. Never silently discarded; the user
-- attaches them to a scene later.
CREATE TABLE review_queue_item (
    id                  TEXT PRIMARY KEY,
    session_id          TEXT NOT NULL,
    package_id          TEXT NOT NULL,
    source_comment_id   TEXT,
    kind                TEXT NOT NULL CHECK (kind IN ('ambiguous','unmapped')),
    author_user_id      TEXT,
    author_name         TEXT NOT NULL,
    body                TEXT NOT NULL,
    quoted_text         TEXT,
    source_label        TEXT,
    reason              TEXT NOT NULL,
    candidates_json     TEXT NOT NULL DEFAULT '[]',
    status              TEXT NOT NULL DEFAULT 'Pending' CHECK (status IN ('Pending','Attached')),
    attached_comment_id TEXT,
    attached_label      TEXT,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1,
    deleted_at          INTEGER
);
CREATE INDEX idx_review_queue_status ON review_queue_item(status, created_at);

-- A separate review record (Import/Export §15 stale option "Create a separate
-- review record"): the package snapshot and its comments kept for reference
-- without touching the working content.
CREATE TABLE exchange_review_record (
    id                  TEXT PRIMARY KEY,
    session_id          TEXT,
    package_id          TEXT NOT NULL,
    package_type        TEXT NOT NULL,
    source_project_id   TEXT NOT NULL,
    source_project_title TEXT NOT NULL,
    source_label        TEXT,
    title               TEXT NOT NULL,
    content_json        TEXT NOT NULL,
    comment_count       INTEGER NOT NULL DEFAULT 0,
    exported_by         TEXT,
    exported_at         INTEGER,
    created_by          TEXT,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    rev                 INTEGER NOT NULL DEFAULT 1,
    deleted_at          INTEGER
);
CREATE INDEX idx_review_record_created ON exchange_review_record(created_at DESC);
