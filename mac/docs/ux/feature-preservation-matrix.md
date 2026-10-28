# Feature preservation matrix (Windows final polish)

> Written 2026-10-20. Every route and main command that existed before the polish work (baseline 26d9756), what
> happened to it, and the end-to-end spec that exercises it now. Results are the latest run recorded in
> `docs/implementation/progress-log.md`; this page lists coverage, not counts.

## Kept, with the spec that proves it

| Area | Routes and commands | Status | E2E specs |
|---|---|---|---|
| Application Home | project list, new project, open, import, recently deleted, global settings (gear) | kept; Help and Palette buttons replaced by the gear | 10, 15, 16, 25, 60 |
| Idea Vault | notes, folders, links, search | kept | 20, 21 |
| Story | Story page (plain free writing, Export as TXT, PDF and Word), Beat Board, acts, beats, scene cards, Parking Lot, characters, relationships, locations | kept; columns and relationship text improved; Story page tab added first (2026-10-20) | 30, 31, 32 |
| Screenplay | editor, elements, Tab cycle, scene list, find, notes, comments, drafts, compare, export, lock, revisions, title page, page setup, several screenplays | kept; Review section removed | 40, 41, 42, 43, 44 |
| Breakdown | script pane, tagging, suggestions, scene list, side panel | kept; panels can be hidden | 50, 51 |
| Production | Overview, Catalog, Locations, Cast and Crew, Moodboards, Storyboards, Shot Lists, Schedule, Day Out of Days, Daily View, Sides, Reports, Budget | kept; Shot Lists grid and Daily View export added | 61, 62, 63, 86 |
| Call Sheets | per day, exports | kept | 62, 63 |
| Files | project files, details | kept | 76 |
| Packages | Script Review and Story exchange, import of older packages | kept; four package types removed (below) | 70 |
| AI Assistant | ask, scope, proposals, apply, undo, offline model | kept; resizable, cleaner answers | 72, 73, 74, 75, 76, 78 |
| Notes and tasks | project notes, private notes, tasks | kept | 80 |
| Project pages | settings, members, about | kept | 85 |
| Appearance | themes, light and dark, import | kept; one theme added | 60, 91 |
| Safety | restart, close while typing, forced kill, webview network block | kept | 02, 04, 05, 26 |
| Whole app | every route reachable, keyboard and screen reader, performance, visual baseline, layout geometry | kept; geometry spec added | 00, 01, 90, 95, 96, 97, 98 |

## Removed on the owner's instruction

| Removed | Instruction | What remains for existing data |
|---|---|---|
| Screenplay Review section | Owner change list | Review round data stays in the database and in old packages |
| Breakdown, Shot List, Schedule and Call Sheet Review packages | Owner change list | Export refuses with a clear message; import of older files still works |
| Episodic and Series project types for new projects | Owner change list | Existing Episodic and Series projects keep their type and open normally |
| Undo and Redo buttons in the top bar | Owner change list | Ctrl+Z, Ctrl+Y and the project menu Edit section |
