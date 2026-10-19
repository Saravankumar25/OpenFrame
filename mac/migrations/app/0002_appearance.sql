-- Appearance preferences and My Themes (ADR-0014; docs/engineering/30-appearance-themes.md section 4).
-- Personal preferences of the person at this computer. Never stored in a project.

-- 'mode' is light, dark or system; 'theme' is the active theme id. A missing row means the default
-- (System mode, OpenFrame Default), so an installation that never opens Appearance keeps today's look.
CREATE TABLE sys_appearance_pref (
    key         TEXT PRIMARY KEY CHECK (key IN ('mode', 'theme')),
    value       TEXT NOT NULL,
    updated_at  INTEGER NOT NULL
);

-- Imported themes. Only validated values are stored (seeds and overrides rebuilt from numbers);
-- every read validates them again, and a row that fails is shown as unusable and never applied.
CREATE TABLE sys_user_theme (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    description     TEXT NOT NULL DEFAULT '',
    author          TEXT NOT NULL DEFAULT '',
    seeds_json      TEXT NOT NULL,
    overrides_json  TEXT NOT NULL DEFAULT '{}',
    format_version  INTEGER NOT NULL,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);
