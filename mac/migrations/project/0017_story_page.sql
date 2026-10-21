-- The Story page (owner request 2026-10-20): one free page per project to write the story in
-- prose, like a notepad. It is not linked to the Beat Board, the screenplay or anything else;
-- only its own text is stored. Additive only.
CREATE TABLE story_page (
    id          TEXT PRIMARY KEY,
    body        TEXT NOT NULL DEFAULT '' CHECK (length(body) <= 2000000),
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    rev         INTEGER NOT NULL DEFAULT 1
);
