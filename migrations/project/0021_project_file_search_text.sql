-- 0021: the words of a small text file copied into the project, so search and the assistant can read
-- them without opening a file (ADR-0041).
--
-- Filled once, when the file is added (`files.add` in copy mode): plain UTF-8 `.txt`, `.md` or `.csv` of at
-- most 16 KiB, read from the confined copy in the project's assets. NULL for anything else, for a linked
-- file, and for a file added before this column existed. It is never read back from a path afterwards.
-- Older builds ignore the column.
ALTER TABLE project_file ADD COLUMN search_text TEXT;
