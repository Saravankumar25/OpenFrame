# 33: Production depth contract (proposed)

> Status: **proposed**. Approval comes first (Project controls and baseline), then the build (Beat Board and Idea Vault; Breakdown, catalog, locations, cast and crew; Shooting schedule and Day Out of Days; Call sheets, budget, templates, samples and exports). Product behaviour: Revision 1 §§21 to 35, 43, 52 to 57.
> Features in the feature status file: STORY-CARD-*, BRK-*, PROD-*, SCHED-*, DOOD-*, CALL-*, BUDGET-*, TEMP-*, BRAND-*.
> Principle: **extend what exists.** Each section lists what the schema already has and the smallest additive change.

## 1. Beat Board card (Beat Board and Idea Vault area)

Status: **built** (2026-10-07, branch `area/09-story-board`). Tests: `crates/openframe-application/tests/story_cards.rs`.

Has: `story_scene_card.short_description`, `color`; `story_attachment`.
Added: `long_description` (canonical, searchable, AI-readable) in project migration `0012_story_card_and_vault_research.sql`.
It is edited in the expanded card (the Scene Card drawer), carried by Duplicate and by exchange packages
(`ExCard.long_description`, with `long_description_included` so older packages never clear it), indexed in the card's
search document (and through it the assistant's retrieval projection), shown by the `story_card` read tool and editable
only through the `propose_update_scene_card` proposal tool.

Card Display Settings are a **view setting**, not content: `sys_view_state` key `story.card_display` per user (no new
table). Operations `story.card_display` and `story.set_card_display` (Capability View, not undoable, quiet, hidden from
the assistant as view state). 12 toggles: scene heading, short description, long description preview, notes,
attachments, character count, location, day/night, comments, revision/change, breakdown status, shot/storyboard status.
Presets: Minimal (short description and comments; the original compact card, the default), Story (adds heading, long
description preview, notes, character count), Production (adds heading, location, day/night, attachments,
revision/change, breakdown status, shot/storyboard status), Custom (any combination). The card read model carries what
the toggles need: `setName` and `timeOfDay` from the card heading (or the linked scene's heading), and `sceneStatus`
(revised, confirmed breakdown items, breakdown complete, shots, storyboards) of the linked scene, all derived on read.
Invariant: changing display never changes scene or screenplay data and never enters undo or the activity log
(test `changing_card_display_never_changes_scene_or_screenplay_data`).

**Capacity limits** (enforced in Rust, constants in `modules/story/mod.rs`):

| Field | Limit | Refusal |
|---|---|---|
| Short description | 20,000 bytes | `validation.invalid_input` |
| Long description | 100,000 characters (counted as characters, so Telugu and Hindi get the same room as English) | `validation.invalid_input`, message names the limit |
| Notes (card, beat, act, sequence) | 500,000 bytes | `validation.invalid_input` |
| Attachments | 50 per Scene Card, beat or sequence; each file within the project's managed file limit (`MAX_MANAGED_FILE_BYTES`) | `validation.invalid_input`; nothing is copied for the refused file |

The minimized card truncates visually only (CSS line clamps); the stored text is never shortened.

**Links when scene identities change.** A card stores the id of the scene Build Screenplay created. New drafts copy
scenes with new ids and the same `lineage_id`, so every read resolves the link through `modules/story/links.rs`: the
same lineage in the screenplay's current draft first, then the stored scene, then the newest other draft that still has
the lineage, otherwise no link. The stored id is never rewritten. Apply Board Order already matched by lineage.

Sections 2 to 8 below each take the next free project number when their build area starts; numbers are never reserved
in this document.

## 2. Breakdown and catalog (Breakdown, catalog, locations, cast and crew area)

Has: eleven `BreakdownCategory` values with canonical order (ADR-0012 §2), suggestions and review state, catalog.
Add: category display metadata (colour token, icon, label) stored as UI metadata; bulk operations as single
transactional commands with one undo step; persistent filters as view state; source-phrase anchors resolved from
screenplay structure. Rule kept: suggestions never become truth until accepted; a script change never silently deletes confirmed data.

Built (area "Breakdown, catalog, locations, cast and crew", project migration `0014_production_depth.sql`):
- `breakdown_category_style` stores Advanced overrides only (`id` = category, colour token, icon key). Defaults, the
  12-colour palette (each at least 3:1 against the light surfaces, unit test) and the 16 icon keys live in
  `modules/production/categories.rs`. In dark modes the desktop lifts the stored colour toward white so the dot,
  icon and underline keep 3:1 on dark surfaces. The label and tooltip never change. Ops: `breakdown.categories`,
  `breakdown.set_category_style`, `breakdown.reset_category_styles` (AI: hidden, appearance only).
- Filters run in Rust: `breakdown.scenes { filter, category, catalogItemId }` with `filter` one of all, unreviewed
  (suggestions waiting or script changed), suggested, missing (no production element and not complete), needsReview,
  complete, incomplete; the view returns `total` and per-filter `counts`. The chosen filter is per-user view state
  (local storage), never project data.
- Bulk: `breakdown.accept_many` and `breakdown.reject` are one mutation each, so one undo step per batch;
  `breakdown.reuse { sceneIds, catalogItemIds }` adds existing catalog items to many scenes as one step;
  `breakdown.quick_reuse` offers items of the neighbouring scenes first, then the most recently used.
- Source phrase: `breakdown.source_anchor` checks the stored span first, then searches the phrase in the scene
  (status exact, moved, missing or none). It reads only; it never changes the element.
- Short breakdown report: `breakdown.export_report { content: "summary" }` (PDF, CSV or XLSX).

## 3. Locations, cast, crew (Breakdown, catalog, locations, cast and crew area)

Add or verify fields needed by call sheets: location address, access instructions, parking, nearest hospital,
emergency contact, availability windows, notes, photos; person contact, availability, department, role.
Availability is data the schedule reads; it is advisory. Contact data is privacy-classified (owner and role scoped; excluded from AI unless the actor may view it).

Built (same area and migration):
- `location.nearest_hospital`, `location.emergency_contact` (contact data); access and parking stay in `notes_json`.
- `crew_member.availability_notes` (cast already had it).
- `availability_window(owner_type, owner_id, start_date, end_date, kind, note)`: inclusive ISO calendar days,
  Available or Unavailable, at most 200 per owner, replaced as one undo step by `availability.set`. Dates must be
  ASCII digits in the form YYYY-MM-DD and real calendar days. Saving also touches the owner row, so the owner's
  search document (which lists the windows) is rebuilt on save, undo and redo, and other windows refresh it. The schedule reads
  it through the typed read model `production::availability::availability_on` (Unavailable wins over an overlapping
  Available window). Windows are removed with their owner when it is purged from Recently Deleted.
- Contact privacy rule (`modules/production/privacy.rs`): `catalog_item.contact`, `location.contact`,
  `location.emergency_contact`, `cast_member.contact` and `crew_member.contact` are shown only to Owner and Editor
  (other roles get `contactHidden: true` and no value). The assistant never receives them: AI-origin requests are
  treated as not allowed, the AI read tools leave them out and AI documents never include them. They are not put
  in the project search index, and production exports never contain them. A Full Project Package or backup still
  carries them because it is a copy of the whole project for its owner.
- Stable identity: renames keep links (catalog rename keeps the old name as an alias; a Location / Set item and its
  location share one name); `production.references { kind, id }` lists the scenes and linked records before a
  delete and says what the delete does.

## 4. Schedule and DOOD (Shooting schedule and Day Out of Days area)

Has (`0007_schedule.sql`): `shooting_schedule` (day duration default), `shooting_day` (date, notes, planned minutes, off day),
`schedule_strip` (scene, estimated minutes, source state), `schedule_marker` (Meal, Travel, Company Move, Custom with
`at_time`, `duration_minutes`), `schedule_warning_ack`.
Add (additive): on `shooting_day`: `crew_call`, `shooting_start`, `target_wrap` (minutes from midnight, nullable);
on `schedule_strip`: `start_minutes` (nullable, otherwise computed from order and durations). Times are stored as
integers, never display strings. The day timeline is derived: crew call, shooting start, each strip and marker in
position order with duration, estimated wrap; total versus target.
Do **not** introduce a parallel "ScheduleItem" table; strips and markers already share a position space.
Warnings (advisory, acknowledgeable): cast unavailable, location unavailable, location missing, day exceeds target,
travel pushes past target, call sheet stale, source changed. No optimizer; nothing moves a scene without a user action.
DOOD: a **derived report** (cast × shooting days, codes configurable) with a manual-correction layer stored as
overrides keyed by cast and day; export PDF and spreadsheet; read-only to screenplay and breakdown.

Built (migration `0015_schedule_timeline.sql`, 2026-10-07):
- `shooting_day.crew_call`, `shooting_start`, `target_wrap` and `schedule_strip.start_minutes` are clock minutes
  (0 to 1439). A time earlier than the crew call (or shooting start) is read as after midnight, so night shoots
  wrap the next morning. Marker times stay in the existing `schedule_marker.at_time` ("HH:MM", validated).
- The timeline is computed in `modules/schedule/timeline.rs` and returned on every shooting day
  (`ScheduleShootDayDto.timeline`). Operations: `schedule.set_day_times`, `schedule.set_strip_start` (never moves a scene).
- Warnings added: `travel_overflow` (the day fits without its travel and company moves), the target-wrap check inside
  `duration_overflow`, and `cast_unavailable` / `location_unavailable`. Availability is read through
  `modules/schedule/availability.rs` from the dated Unavailable windows of migration 0014 (`availability_window`,
  through `production::availability::unavailable_for_schedule`). Undated days, Available windows and free-text
  notes never warn. In strict mode these warnings ask for Keep anyway like the other conflicts; they never move a scene.
- Search: a shooting day's search document includes its scene headings and its Day Out of Days correction notes.
  `Registry::index_parent` reindexes the day when a strip or a correction changes, including through undo and redo.
- Call sheet source snapshots gain `crewCall`, `shootingStart`, `targetWrap` and a per-scene `start`, left out of the
  JSON when unknown so sheets made earlier keep their fingerprint (no false "Needs refresh").
- DOOD: `shooting_schedule.dood_code_style` ('industry' or 'simple') and `dood_override(schedule_id, cast_key, day_id,
  code, note)`. Codes: SW, W, WF, SWF, H derived; T, R, F, D and "-" by hand. Operations `schedule.dood`,
  `schedule.set_dood_override`, export `schedule.export_dood`.
- Daily View export `schedule.export_daily_view` (PDF or plain text): one shooting day, built from the same read as the
  screen (`schedule::ops::daily_view`), named "Shoot Day N - Daily View", with the schedule's last-change date.
- Schedule and DOOD exports carry the date of the schedule's last change (never the export clock), and the XLSX
  creation date is set from it, so the same data gives byte-identical PDF, CSV and XLSX files.
- Filters are view state in the desktop app (`scheduleFilter.ts`); they never call an operation.

## 5. Call sheet (Call sheets, budget, templates, samples and exports area)

Has: `call_sheet` with lifecycle (Draft, Needs Refresh, Ready, Final, Issued, Superseded), `source_snapshot_json`,
`source_fingerprint`, `document_json`, finalized snapshot link.
Add: a versioned `document_json` schema covering Revision 1 §29 sections; six default sections, the rest through
**+ Add Section**; unknown values render "Needs input", never invented text; update preview lists changed scenes,
cast calls and locations; finalized sheets are immutable snapshots; editing never changes screenplay, breakdown or schedule.
~~Open owner question: status vocabulary and the "needs refresh" trigger (ADR-0012 list).~~ Answered 2026-10-07: Draft,
Needs refresh, Final (ADR-0012 amendment).

**Built (area 15, 2026-10-08).**
- Status: migration `0016_callsheets_budget.sql` maps the stored words (Ready to Draft, Needs Refresh to a flagged Draft,
  Issued and Superseded to Final). Only Draft and Final are stored; "Needs refresh" is derived from the source
  fingerprint and the `needs_refresh` flag. `callsheets.set_ready` and `callsheets.issue` are removed. A replaced
  revision stays Final and keeps `superseded_by`.
- Source: `modules/schedule/callsheet_source.rs` is the one definition of what a sheet is built from (day, scenes with
  I/E, D/N, cast ID numbers and location, the day's rows with start times and lengths from `Board::day_timeline`, meal
  and travel rows, cast, locations with parking, access and contact, extras, department items, the next shooting day).
  It carries `version` (1); a source stored before 0016 is compared in its old shape (`fingerprint_as(0)`), so an
  upgrade alone never marks a draft stale.
- Document: `CallSheetDocument` version 2 (`modules/schedule/callsheet.rs`). Default sections: production header, day
  and times, locations, scene schedule, talent, practical notes. Optional (hidden until added with
  `callsheets.add_section`, which prefills from the project): weather with sunrise and sunset (typed by the user, no
  network lookup), extras, department notes, advance schedule, crew by department, walkie channels and contacts,
  reference images, attachments, special notes. Missing values read "Needs input" on screen and in the PDF.
- Update preview: `callsheets.refresh_preview` lists added, removed and changed scenes, cast calls and locations, the
  date and the other changed areas; nothing applies until `callsheets.refresh` (Draft) or `callsheets.new_revision`
  (Final). The merge keeps typed call times, contacts, notes and added sections.
- Final sheets export from their `snapshot` row, never from later state.

## 6. Budget (Call sheets, budget, templates, samples and exports area)

Has: `budget_snapshot`, `budget_line.amount` (integer minor units), nine categories, contingency.
Add: `planned` and `actual` per line (migration keeps existing `amount` as `planned`; `actual` defaults to null), totals
and difference derived, report export. Out of scope: payroll, invoices, purchase orders, tax, ERP.

**Built (area 15, 2026-10-08).** `budget_line.amount` keeps its name and means Planned; `budget_line.actual` (NULL until
entered) was added by 0016. Category totals, Planned and Actual totals and the difference (Planned minus Actual over
lines that have an Actual) are derived at read time (`modules/schedule/budget.rs`); the report export
(`budget.export_summary`, PDF, CSV and XLSX) shows Planned, Actual and Difference per line and category.

## 7. Templates and samples (Call sheets, budget, templates, samples and exports area)

Has: template engine with scopes and 7 built-ins. Add per the owner decision "Template list": three screenplay templates, indie call sheet, simple
Beat Board, shot list, moodboard, lightweight budget. Use creates an independent copy. Samples: four
OpenFrame-authored scripts used as onboarding assets and fixtures; no third-party text.

**Built (area 15, 2026-10-08).** The eight built-ins live in `modules/notes/templates.rs` (Feature Film Screenplay,
Short Film Screenplay, Episodic / Series, Simple Beat Board, Indie Call Sheet, Shot List, Moodboard, Lightweight
Budget). `templates.copy` copies a template; `templates.apply` creates the document a template describes through the
owning module's operations, all or nothing, as one undo step. Nothing refers back to the template. The four samples
are compiled into the app (`openframe-import-export/src/samples.rs`), listed by `screenplay.samples` and opened through
the regular import preview (`screenplay.import_preview` with `sample`); `tests/samples.rs` uses them as fixtures.

## 8. Branding (Call sheets, budget, templates, samples and exports area)

Screenplay outputs clean. Production outputs carry a small logo and "Created using OpenFrame"; spreadsheets
use workbook metadata or first-sheet footer. User-owned source files are never modified for branding. Packages
and backups carry a branded manifest or README, not edited user files.

**Built (area 15, 2026-10-08).** `modules/exports/mod.rs::deliver` brands every production export in one place: PDFs
print the OpenFrame mark and the brand line at the bottom of each page (Sides print the line in each page's bottom
margin), workbooks name OpenFrame Studio in their document properties and the first sheet's footer, plain text ends
with the line. CSV stays pure data (no added row) so it imports cleanly elsewhere. Screenplay exports
(`screenplay.export`) never pass through `deliver` and stay unbranded. Package and backup manifests belong to the
packages area.

## 9. Cross-cutting rules for every item above

Stable ids only; no cross-module SQL (typed read models); snapshot versus live-source distinction explicit; stale
detection instead of silent mutation; every mutation through Registry then `Store::mutate`; search document and AI
decision per entity; undo and recovery tested; packages carry the new data with a format-version bump.
Migrations are additive, linear on `master` and checked by `scripts/check-migrations.mjs` (doc 23 §4).
