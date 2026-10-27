-- Story page: one plain writing page in the Story workspace (Story > Story tab).
--
-- A single row holds the writer's free text. The page is not linked to any card, scene, screenplay
-- or assistant tool. The row is created by the first save, so a project that never uses the page
-- has no row. `rev` is checked on every save; on a stale rev the page re-reads it and saves the text on screen,
-- so what the writer sees is never discarded. No `deleted_at`: the page is cleared by writing empty text.
CREATE TABLE story_page (
    id          TEXT PRIMARY KEY,
    body        TEXT NOT NULL DEFAULT '',
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    rev         INTEGER NOT NULL DEFAULT 1
);
