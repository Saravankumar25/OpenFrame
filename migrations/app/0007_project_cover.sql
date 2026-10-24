-- Optional cover photo for a project card on Application Home. The picture is kept by the app, not
-- inside the project: Home lists projects that are closed. `cover_file` is the file name inside
-- `<app data>/covers/assets/` (NULL means the generated cover is shown) and `cover_rev` counts the
-- changes so every new picture has a new name.

ALTER TABLE sys_recent_project ADD COLUMN cover_file TEXT;
ALTER TABLE sys_recent_project ADD COLUMN cover_rev INTEGER NOT NULL DEFAULT 0;
