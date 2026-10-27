-- 0019: formatting of the Story page (owner request 2026-10-27: font size, bold, italic, underline,
-- highlight).
--
-- `body` stays the plain text and the source of truth. `format` is NULL or a JSON list with one entry
-- per line of `body`, each a list of runs whose texts joined give that line exactly
-- (crates/openframe-import-export/src/prose.rs). The app checks the match on every save and on every
-- read; a format that does not match its text is never applied. Older builds ignore the column.
ALTER TABLE story_page ADD COLUMN format TEXT;
