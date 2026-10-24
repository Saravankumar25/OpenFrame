# Shot List — Entire Project (shot.export_list)

**Variant:** Entire Project: scope "Whole project", option includeRemoved
**Version / orientation:** v2, **PORTRAIT** US Letter 612 x 792 pt (user decision; supersedes the landscape v1 design). The current builder calls `.landscape(true)` and must change to portrait.
**Status:** Design reference — not integrated; not validated against OpenFrame's Rust renderer
**Repo commit:** `2f5a045416bd2125801329533fc47f986ccc8174` (windows/ tree)
**Date:** 2026-10-23
**Files:** `Shot_List_Entire_Project.html` (source), `Shot_List_Entire_Project.pdf` (rendered), `stress-test/Shot_List_Entire_Project_STRESS_TEST.html/.pdf`

## A. Purpose and users

**Who.** Production office (1st AD, UPM, script supervisor), department heads and the DP for the *whole film*: the master shot list printed from the Shots tab with "Whole project" (pre-production binder, tech recce, coverage audit, hand-off to a second unit). It must be navigable at 40+ shots: scene number in a fixed place, scenes in script order, per-scene counts, and a clear marker for scenes whose plan is stale or whose scene left the script.

**What they must find fast.** Scene n (band at the start of each group, repeated when a scene continues), shot tile, camera block, per-scene shot count, the overall count ("36 shots in 10 scenes") and the export date, "Scene changed since planning", and (when the option is on) the shots of scenes removed from the script, clearly set apart at the end.

## B. Verified functionality

**Orientation.** The current builder sets `.landscape(true)` and produces landscape pages. **v2 (user decision) is portrait US Letter for all three scopes; the builder must drop `.landscape(true)`.**

**Entry point and dialog.** Production workspace, Shots tab -> Export button -> "Export Shot List". Formats (`TABLE_FORMATS`): PDF ("printable"), CSV ("spreadsheet-friendly text"), Excel (XLSX) ("one sheet per table"). Scopes offered by `ShotListExportButton`: **"Current scene (<number> · <heading>)"** (only when a scene is selected; sends `sceneIds: [id]`), **"Selected scenes"** (picker, sends `sceneIds`), **"Whole project"** (no `sceneIds`). Option (checkbox, default off): **"Whole project: also shots of scenes removed from the script"** -> `includeRemoved`.

**Args** (`ExportShotListArgs`, camelCase, `deny_unknown_fields`): `format` ("pdf" | "csv" | "xlsx"), `path` (absolute, extension forced), `sceneIds?: string[]`, `includeRemoved?: boolean`.

**Resolution rules (`export_shots`).**
- `sceneIds` present but empty -> validation error on `sceneIds`: "Select at least one scene to export."
- `sceneIds` absent -> `shots::list` for the whole project: active scenes in script order, then (only if `includeRemoved`) shots of removed scenes ordered by `scene_lineage_id` (SQL `ORDER BY scene_lineage_id`, i.e. by id, not by former script position).
- `sceneIds` present -> one `shots::list` per id **in the order received**, de-duplicated by shot id; `includeRemoved` is **ignored** in this mode (the builder passes `None`).
- No shots at all -> `AppError::export("nothing_to_export", ...)` with "The selected scenes have no shots yet." (selection) or "There are no shots to export yet." (whole project). No PDF is produced.
- Groups are built from the shots (`scene_lineage_id`, first-appearance order). **A scene with zero shots is therefore never printed**, and is not counted in "N shots in M scenes".
- The layout is "single" when exactly one group results (`groups.len() == 1`), whatever the scope: PDF document title becomes "Shot List: Scene N: HEADING" and the masthead gets an extra line with the scene title.
- Shot order inside a scene = the scene's coverage order (`ORDER BY position`); the label (`12B`) is **derived** from the scene's current number plus the position (A..Z, AA..) and is never stored: reordering, inserting or deleting shots, or renumbering the scene, changes labels in the next export.
- Pictures: the shot's own reference image, otherwise the first attached storyboard panel image. Image problems become placeholders; the builder does **not** report them as export warnings (it counts them in `Pics` but never calls `image_warnings`).
- No private notes exist in this export (the result summary says "Private notes excluded" generically).

**This variant** = scope "Whole project" (no `sceneIds`; `scope_label` "Whole project"). Option **"Whole project: also shots of scenes removed from the script"** (`includeRemoved`, default **off**): when on, shots whose scene no longer exists in the script are appended after the active scenes, each such group having `scene_removed = true`, no scene number, letter-only labels (A, B, C) and the status sentence "Scene removed from the script (kept for reference)". When off (the supplied PDF) removed scenes do not appear. Active scenes keep script order; scenes with no shots are not printed.

Sample = the user's real export: 36 shots in 10 scenes (1, 2, 8, 13, 16, 19, 25, 34, 40, 42), 8 pages in the current PDF, scene 40 flagged "Scene changed since planning", option off.

**B.1 Current-output assessment (supplied `Shot List — Entire Project.pdf`, 8 pages, landscape; v2 portrait: 5 pages).**

- *Retain:* scene grouping with a dark band per scene and a shot count; column order Done, Image, Shot, Description, Size, Movement, Angle, Lens (+ Notes: a column in the current output, a block under Description in v2); "column exists only if the data does" rule; one picture per shot with storyboard fallback; Notes label order and wording; the two status sentences; "(CONTINUED)" continuation cue; hand-tick circle; wrapped (never clipped) text; whole-row page moves; Letter paper (current output landscape; v2 portrait by user decision).
- *Improve visually:* shot number is only bold text (make it a tile); the four camera columns are plain text with no grouping; Notes labels indistinguishable from the text; Notes squeezed into a 114-150 pt column (4-6 line cells); 88 x 50 "No image" frames on most rows make single-line shots 60-70 pt tall; the table pad (7 x 6 pt) wastes width; footer text is a long grey string instead of the brand footer.
- *Verified functional/layout problems in the current PDFs:*
  - "Overhead" breaks mid-word ("Overhea" / "d") in the 50 pt Angle column: Single p1 (shot 1C), Multiple p1 (1C), Entire p1 (1C). Cause: `Col::pt("Angle", 50.0)` with 7 pt padding leaves 36 pt, narrower than the word at 8.2 pt.
  - "35mm anamorphic" (shot 25C) is wider than the 44 pt Lens column's 30 pt inner width and runs into the Notes text: Entire p6.
  - Group-wide empty columns: Entire p2-p5 (scenes 8 and 19, Size only) print empty Movement, Angle, Lens and Notes cells (v2: an empty Notes block simply prints nothing) because column presence is decided for the whole export, not per scene. By design, but a blank cell is ambiguous (not decided vs forgotten).
  - A missing picture's placeholder label is drawn on one line without wrapping (`CellItem::Placeholder` in `report/design/table.rs`), so "Image unavailable: <long original file name>" will overflow its 88 pt frame (code evidence; the supplied PDFs only contain "No image").
  - A row taller than the page body cannot be split (`styled_table` moves a row whole and then emits it regardless), so a very long description/notes cell can run past the footer zone (code evidence; no such row in the supplied PDFs).
  - A selected scene that has no shots is silently left out and not counted (see B); no explanatory line.

  - 33 of 36 rows show a "No image" frame, so a one-line shot such as 19A occupies about 62 pt and the project needs 8 pages (p2-p5, p8).
  - Continuation pages (p3 "SCENE 8 (CONTINUED)", p4, p7) show a slim bar with no shot count and no status, the `continued` bar (`StyledTable.continued`) carries neither; the "changed"/"removed" sentence is a `Banner.sub_left` and exists only on the page where the scene starts (code: `table.rs` draws only "<TITLE> (CONTINUED)").
  - Removed-scene output could not be checked in a supplied PDF (option off in the sample): behaviour taken from source only.

- *Cannot be determined from the evidence:* how the Mac app exports shot lists (`mac/` has shot-list mock-ups under `mac/HTML_Mockups/126-shotlist.html` but its renderer was not compared); the on-screen order in the "Selected scenes" picker (so whether the printed order is script order or click order); whether a removed scene can lack a snapshot heading in real data; maximum realistic shots per scene; whether `size`/`movement`/`angle`/`lens` can be stored as empty strings (the builder tests `is_some()` for the column but prints `opt()`, so an empty string would create a column of blanks).
## C. Source mapping

| Item | Verified identifier (repo `main` @ `2f5a045416bd2125801329533fc47f986ccc8174`) |
|---|---|
| UI entry point | `windows/apps/desktop/src/features/export/buttons.tsx`, component `ShotListExportButton({ scene, scenes, ...trigger })` (line 216). Opens `ExportDialog` titled **"Export Shot List"**, `documentName="Shot List"`, `op="shot.export_list"`, `formats={TABLE_FORMATS}` (`features/export/model.ts`). |
| Where the button appears | `windows/apps/desktop/src/workspaces/production/tabs/shots.tsx` (import line 71, mounted as the `exportButton` prop of `<Header>` around line 180). `scene` = the scene selected in that tab's Scene selector (`{ id: scene.sceneId, label: "<number> · <heading>" }`), or `null`; `scenes` = every scene of the current script in the same label format. |
| Operation | `shot.export_list`, registered in `windows/crates/openframe-application/src/modules/exports/mod.rs` (line 133; capability `Cap::Export`, "Export a shot list to a location the user picked."). |
| Builder | `windows/crates/openframe-application/src/modules/exports/visual.rs`: `ExportShotListArgs` (l.538), `scene_title`, `asset_of`, `panel_refs`, `pub(super) fn export_shots` (l.581). |
| Data | `windows/crates/openframe-application/src/modules/visual/shots.rs`: `ShotDto` (l.63), `ShotPanelRef` (l.49), `shots::list` (l.427, args `ShotListArgs { scene_id, scene_lineage_id, include_removed }`), `load_lineage_shots`, `shot_from_row`, `make_label`; `visual/common.rs::shot_letters` (A..Z, AA..); `visual/scenes.rs::SceneCtx::review_state` (l.253) produces `(scene_removed, needs_review)`. `util::AssetInfo` for pictures. |
| Renderer blocks used | `Document::new(..).landscape(true).footer(footer(&project, "Shot List"))` (**v2 requirement: drop `.landscape(true)`, the document is portrait**), `Block::Masthead` (`eyebrow`, `subtitle`, `lines`, `Side::Panel`, `rule`), `Block::Banner` (`title`, `right`, `sub_left`, `keep_next`, `fill`), `Block::Styled(StyledTable)` (`Col::pt` / `Col::weight`, `boxed`, `HeaderStyle::Tint`, `continued`, `Row`, `Cell::text`, `Cell::new().item(..)` (v2: the Description cell stacks the Notes lines as additional `CellItem::Text` items; the `Col::weight("Notes", 1.5)` column is removed), `CellItem::Check`, `CellItem::Image { w: 88, h: 50 }`, `CellItem::Placeholder { w: 88, h: 50, label }`, `CellItem::Text`), plus a flat `Table` ("Shot List", 14 columns) for CSV/XLSX. Renderer: `openframe-import-export/src/report/design/{blocks,table}.rs`. |
| Delivery | `deliver(core, actor, &dest, fmt, Job { action: "shot.export_list", document, source_label: "Shot Lists", scope_label, contents_label, target: None }, out)`. |

Whole-project specifics: `scene_ids = None` -> `shots::list(ShotListArgs { scene_id: None, scene_lineage_id: None, include_removed: a.include_removed })`: active scenes first (`ctx.scenes`, one `load_lineage_shots` per unique lineage), then, if enabled, `SELECT DISTINCT scene_lineage_id FROM shot WHERE deleted_at IS NULL ORDER BY scene_lineage_id` filtered to lineages with no current scene. `Banner.sub_left` = `("Scene removed from the script (kept for reference)", true)` when `first.scene_removed`, else `("Scene changed since planning", true)` when any shot `needs_review`.

## D. Field inventory

| Visible element | Source field / struct | Kind | Empty-value treatment |
|---|---|---|---|
| Eyebrow "Production · Shot list" | literal in `export_shots` (`Masthead::new("Shot List").eyebrow("Production · Shot list", ..)`) | constant | n/a |
| Title "Shot List" | literal | constant | n/a |
| Project name (subtitle) | `project(core)` = `project_title`, via `nb()` | conditional | omitted when blank |
| Summary panel line 1 "N shots in M scenes" | `plural(shots.len())`, `plural(groups.len())` (groups = distinct `scene_lineage_id` that have at least one shot) | derived | always present |
| Summary panel line 2 "Exported 22 Oct 2026" | `super::today_label()` (day, 3-letter month, year, local clock) | derived | always present |
| Scene band eyebrow "SCENE 25" (design) | `ShotDto.scene_number` (`Option<String>`; `SceneCtx::number_label`) | conditional | omitted for a removed scene (`None`) |
| Scene band title | `ShotDto.scene_heading` (current heading; snapshot heading if the scene was removed) | required (may be an empty string for a removed scene without snapshot: see K) | band shows no title text; nothing invented |
| Scene band count "3 SHOTS" | `plural(list.len(), "shot", "shots")` uppercased by the renderer | derived | always present |
| Status row "Scene removed from the script (kept for reference)" | `ShotDto.scene_removed` of the group's first shot | conditional | row omitted when false |
| Status row "Scene changed since planning" | `ShotDto.needs_review` of **any** shot of the group, shown only when the scene is not removed (`else if`) | conditional | row omitted when false |
| Done circle | `CellItem::Check`, no data | constant per row | always present (hand-tick) |
| Image | `ShotDto.reference_asset`, else the first storyboard panel image (`panels.iter().find_map(|p| p.asset)`) loaded by `load_image` | conditional per row; the **column** exists only if some shot of the export has a picture (`any_image`) | placeholder "No image"; missing file "Image unavailable: {original_name}"; unreadable/too large "Image can't be printed" |
| Shot tile "25C" | `ShotDto.label` = scene number + `shot_letters(position)`; letters only for a removed scene ("A", "B") | derived, required | n/a |
| Description | `ShotDto.description` (required, max 2 000 chars, `DESCRIPTION_MAX`) | required | n/a |
| Size / Movement / Angle / Lens | `ShotDto.size / movement / angle / lens` (`Option<String>`, each max 200 chars, `SHORT_MAX`) | optional; the **column** exists only if some shot has that field (`has_size` ...) | design: "–" in `--of-muted`; current output: blank |
| Notes block under the description: `Characters: A, B` | `ShotDto.characters` (`Vec<String>`) joined by ", " | optional line | line omitted |
| Notes: `Storyboard: <board> #<n>[, ...]` | `ShotDto.panels` -> `panel_refs` = `"{storyboard_name} #{number}"` joined by ", " | optional line | line omitted |
| Notes: `Sound: ...` | `ShotDto.sound_note` (trimmed by `nonblank`) | optional line | line omitted |
| Notes: `Camera: ...` | `ShotDto.camera_notes` (trimmed by `nonblank`) | optional line | line omitted |
| Notes block (v2: stacked under the Description text, same cell) | present for a shot only if it has characters, panels, sound note or camera notes; the builder's `has_notes` no longer creates a column | conditional per shot | nothing printed; the row is just as tall as the description |
| Footer | shared system footer (logo, "Created using OpenFrame", "Page X of Y") | constant | n/a |
| Running header (design) | project name uppercased + " · SHOT LIST" (+ " · SCENE N" in a one-scene document) | derived | omitted when the document is one page |

Not printed although present in `ShotDto` / available: `order` (CSV/XLSX only), `created_at`, `updated_at`, `rev`, `storyboard_panel_id`, `visual_kind`, `scene_id`, `scene_lineage_id`. CSV/XLSX (one flat `Table "Shot List"`, same rows) carry 14 columns: Scene, Scene heading, Shot, Order, Description, Size, Movement, Angle, Lens, Characters, Sound note, Camera notes, Storyboard, Status ("Scene removed" / "Scene changed since planning" / blank). The PDF is the only format designed here.

Whole-project additions: removed-scene groups (conditional on `includeRemoved`) have `scene_number = None` (no "SCENE n" eyebrow, labels A/B/C), `scene_heading` from the stored snapshot, band fill slate.

## E. Content architecture

**Order of blocks:** masthead (first page) -> rule -> active scenes in script order -> (option on) removed scenes in `scene_lineage_id` order. Each scene = table [band -> status row (conditional) -> column header -> rows]. No separator or heading is printed between the active and the removed scenes (none exists in the source); the slate band, the missing number and the red status row mark them.

Columns, widths, wrapping: **identical to `Shot_List_Single_Scene.md` section E**:

Content width 520 pt (612 - 2 x 46). Widths include cell padding.

| # | Column | Present when | Width (pt) | Align | Wrap | Content |
|---|---|---|---|---|---|---|
| 1 | Done | always | 32 (measured: header "DONE" + 9, min 32); 3 pt side padding | centre | no | 12 pt circle, 0.9 pt `--of-slate` stroke, top of the row |
| 2 | Image | some shot in the export has a picture | 88 (frame 80 x 45, 16:9, 4 pt inset each side) | left | n/a | thumbnail, `contain`, never cropped |
| 3 | Shot | always | 38 (min; measured as longest label at 10 pt bold + 13, e.g. 39 for "118A"); 3 pt side padding | centre | no | label tile (10 pt bold) |
| 4 | Description | always | flexible: **all that the fixed columns leave**: 177 pt in the single and multiple samples, 162 pt in the entire-project sample | left | yes, row grows | description 8.8 pt / 1.35, then the **Notes block** (below) |
| 5 | Size | some shot has a size | measured, min 40 (46 / 43 in the samples) | left | at spaces only | 8.2 pt **bold** navy |
| 6 | Movement | some shot has a movement | measured, min 44 (56 in the samples: the header word MOVEMENT sets it) | left | yes | 8.2 pt |
| 7 | Angle | some shot has an angle | measured, min 40 (46 in the samples) | left | yes | 8.2 pt |
| 8 | Lens | some shot has a lens | measured, min 40 (40 for 24mm / 100mm; 52 when "anamorphic" is present) | left | yes | 8.2 pt |

**The Notes block** is no longer a column. Inside the Description cell, after the description text and a 3.5 pt gap, up to four lines (8 pt / 1.3, `--of-slate`, label bold `--of-navy`, 2 pt between lines) in the fixed order and wording of the builder: `Characters: ...`, `Storyboard: ...`, `Sound: ...`, `Camera: ...`. A shot without notes prints no block and no gap. Effect on row height: a row is `max(image 45 pt, description + notes stack) + 10 pt`; a description-only row stays at 55-56 pt (the picture frame sets it), a row with 2-4 note lines is 70-106 pt in the sample (1A 94 pt, 2A 83 pt, 25A 106 pt, 25C 82 pt), a very long note stack grows the row (stress test: 440 pt). Description is 160-180 pt wide (v1: about 156-166 pt each for Description and Notes), so the notes are no longer squeezed into a narrow column but every row with notes is taller than in v1.

*measured* (camera columns) = max( longest unbreakable word of that column over the **whole export** at 8.2 pt (Helvetica Bold for Size) + 2 pt safety, header label at 6.9 pt bold with .09 em tracking + 1.5 pt ) + 9 pt (4.5 pt padding each side), capped at 84 pt. One column grid is computed per document and shared by every scene table, so columns line up from scene to scene, no word is split mid-word ("Overhead", "anamorphic", "Handheld") and no header wraps ("MOVEMENT"). The +2 pt safety is needed: with 0 pt "anamorphic" broke at its last letter in Chrome.

**Grouping/sorting.** By scene lineage; rows in coverage order. The deterministic but arbitrary order of removed scenes (by id) is kept; the design does not re-sort.

**Differences from the other variants.** Running header on every page; no coverage strip; removed-scene treatment (slate band + red status row) is part of this scope only; the document typically spans many pages, so pagination rules (H) matter most.

**Changes versus the current PDF, with reasons.** Items 3-7 of the Single variant (band eyebrow/attached card, tile and camera block, widths, column sizing) and: *continuation pages repeat the band and the status row* (current: slim bar without count/status); *scenes of <= 3 shots are never split*; *page count 5 instead of 8* (v1 landscape design: 7) in portrait: the page is 680 pt tall instead of 500, rows are 55-106 pt, and the notes no longer sit in a narrow column.


**Changes from v1 (landscape design, same date)** — two user decisions, nothing else in content, wording, field meaning or column order:
1. *Portrait US Letter* (612 x 792 pt, content 520 x 680 pt) instead of landscape (792 x 612, content 700 x 500). `@page { size: 8.5in 11in }`; the `landscape` class is gone from `.doc`.
2. *Notes under Description.* The Notes column is removed; the same lines (Characters, Storyboard, Sound, Camera, same order and wording) are stacked beneath the description text in the same cell. A shot without notes prints nothing extra.
Consequences re-derived for 520 pt: columns Done 32, Image 88 (frame still 80 x 45), Shot 38+, Description = remainder (160-180 pt), camera columns measured (Size 43-46, Movement 56, Angle 46, Lens 40-52; camera text 8.2 pt instead of 8.4 pt so that MOVEMENT and "anamorphic" fit without splitting); band min height 34 pt (was 36), scene gap 10 pt (was 12), coverage strip columns equal; the multiple-scenes sample is now one page, so its running header is dropped (rule unchanged: header only when the document has more than one page).
Pages, v1 landscape -> v2 portrait: single sample 1 -> 1; multiple sample 2 -> 1; entire sample 7 -> 5 (baseline 8); stress tests single 3 -> 3, multiple 3 -> 2, entire 10 -> 8.
Honest readability notes from the portrait render: rows that carry notes are taller (70-106 pt versus about 55-85 pt in v1) and the description text wraps in a 160-180 pt column (about 32-36 characters per line at 8.8 pt), but there are fewer pages because the page body is 680 pt tall instead of 500 pt; description-only rows stay at 55-56 pt (the picture frame). Camera cells wrap two-word values ("Eye level", "Handheld on the boat", "100mm macro") onto two lines in the 40-56 pt columns; every word still fits on one line.

## F. Visual specification

Visual specification = **identical to `Shot_List_Single_Scene.md` section F.1-F.9**, restated:

**F.1 Page.** US Letter **portrait** 612 x 792 pt (`@page { size: 8.5in 11in }`, no `landscape` class on `.doc`). Margins 46 left/right, 46 top, 66 bottom: content box **520 x 680 pt**. Helvetica / Helvetica-Bold only. Smallest text anywhere: 6.8 pt (labels) / 6.9 pt (column headers); body 8-8.8 pt.

**F.2 Masthead (first page only, compact).** Eyebrow "Production · Shot list" 7.6 pt bold, tracking .14 em, uppercase, `--of-slate`. Title "Shot List" 20 pt bold `--of-navy` (the shared `.masthead.compact` size; the current PDF uses 28 pt). Project name 11 pt regular `--of-slate`. Right: summary panel, rounded 6 pt, fill `--of-panel`, 0.5 pt `--of-line` stroke, min width 140 pt, padding 7/13/8 pt, right-aligned: line 1 10 pt bold navy "N shots in M scenes", line 2 8.4 pt slate "Exported D Mon YYYY". Full-width 1.2 pt navy rule 12 pt below the masthead block, then 12 pt gap.

**F.3 Scene group header (the band).** One component for every scope. Navy (`--of-navy`) rounded band (6 pt top corners), min height 34 pt, padding 6 / 14 pt: left an eyebrow "SCENE 25" (8 pt bold, tracking .14 em, colour `#f5c27a`, the colour the renderer already uses for `Banner.eyebrow`), 10 pt gap, the scene heading 14 pt bold white (line-height 1.15, wraps over several lines, never ellipsised), right "N SHOTS" 9 pt regular, tracking .06 em, white. A removed scene uses `--of-slate` fill instead of navy and has no "SCENE n" eyebrow (it has no number). The band is attached to its table: band, optional status row, column header and rows form one rounded card (outer 0.5 pt `--of-line-strong` stroke on the sides and bottom).

**F.4 Status row (under the band, full width).** Text only, 8.6 pt bold, padding 5 / 14 / 5 / 11, 3 pt left stripe. *Scene changed since planning*: fill `--of-amber-fill` (#fdf0d9), text `--of-amber-text` (#8a4f00), stripe `--of-orange`. *Scene removed from the script (kept for reference)*: fill `--of-red-fill` (#fde9e4), text `--of-red-text` (#9a3822), stripe `--of-red-text`. Grayscale-safe: the status is carried by the exact source sentence, the stripe and (for removed) the lighter slate band; colour is redundant. Only one status row per group (removed wins, as in the builder).

**F.5 Column header.** Fill `--of-tint` (#e9eff5), 6.9 pt bold uppercase, tracking .09 em, `--of-slate`, padding 5 pt, 0.5 pt `--of-line-strong` bottom rule. The four camera headers sit on `#e2e9f1` (slightly darker) to mark the camera group.

**F.6 Rows.** Padding 5 pt top/bottom; side padding 5 pt (Description), 4.5 pt (camera), 4 pt (Image), 3 pt (Done, Shot); 0.5 pt `--of-line` rule between rows, `--of-line-strong` under the last row. Rows are never split and never truncated. Minimum row height = 55 pt (45 pt frame + 2 x 5 pt) when the Image column exists, otherwise text height + 10 pt. Vertical alignment top for every cell (checkbox, tile, text and frame share one top edge).
- Shot tile: 10 pt bold `--of-navy`, fill `--of-tint`, 0.5 pt `--of-line-strong` stroke, radius 3.5 pt, min width 26 pt, padding 2 / 3 pt, centred in the column. Same primitive as the existing `CellItem::Tile`.
- Camera cells (Size, Movement, Angle, Lens): **8.2 pt** / 1.3 (v1: 8.4 pt), ink, **cell fill `--of-panel` (#f5f7fa)** across the four columns so the camera vocabulary reads as one block (the renderer already supports a per-cell fill, `Cell.fill`). Size is bold navy because it is the first word a DP and 1st AD scan for (Wide, Medium, Close-up ...). Empty value: an en dash "–" in `--of-muted`, regular weight; a column that does not exist for the whole export prints no dash.
- Notes (v2: a block inside the Description cell, under the description text, 3.5 pt gap): 8 pt / 1.3 `--of-slate`; each line starts with its label in bold `--of-navy` ("Characters:", "Storyboard:", "Sound:", "Camera:"), 2 pt between lines, fixed order Characters, Storyboard, Sound, Camera (as the builder), no separator rule. Implementation note: the label and the text are two runs of one wrapping paragraph. The current `Txt` is one style per item; if mixed runs are not available, print the whole line in one style (current behaviour) and keep the order.
- Description: 8.8 pt / 1.35 `--of-ink`.

**F.7 Pictures.** Frame 80 x 45 pt (16:9), fill `#eef1f5`, 0.5 pt `--of-line` stroke, radius 2 pt, image scaled to fit (`contain`), centred, never cropped or stretched; portrait, square and 2.39:1 references letterbox inside the frame. Missing picture: frame fill `#f1f3f6`, label 7 pt `--of-muted` (the current label uses the 3.1:1 faint grey; `--of-muted` is the legible choice), centred, **wrapped inside the frame**; the frame grows in height (never in width) when the label needs more than 45 pt. Labels exactly as the builder: "No image", "Image unavailable: {original_name}", "Image can't be printed".

**F.8 Footer and running header.** Footer = shared system (logo lockup 17 pt high, "Created using OpenFrame", "Page X of Y"). The current footer text "PROJECT · SHOT LIST · Page N / M" is replaced; the project name moves to the running header. Running header (top-right, 6.8 pt, `--of-slate`, tracking .04 em, no rule, no logo) only when the document has more than one page; decision per document is stated in section H.

**F.9 Colour use.** Navy for structure, orange only as the eyebrow tint on navy and the "changed" stripe, no colour carries meaning alone.

**F.10 Removed-scene groups.** Band fill `--of-slate` (#566277, white text 6.2 : 1), no eyebrow, status row red as F.4. In grayscale the slate band (about 40 % grey) stays distinct from the navy band (about 15 %), and the status sentence is printed, so the removed state never relies on colour.

## G. Dynamic behaviour

Same rules as the other variants plus:
- **Large datasets:** 40+ shots over many pages: every scene is its own table; the band, status row and column header repeat when a scene continues; scenes of <= 3 shots stay whole; a scene with more shots may split with >= 1 row under the header and >= 2 rows on the last fragment. Stress test: 50 shots in 14 scenes on 8 pages (portrait).
- **includeRemoved on:** removed scenes append after the active ones; group order by id; each has the removed status row; a removed scene's shots keep letter-only labels. A removed scene with an empty stored heading prints a band without title text (nothing invented; see K).
- **Scenes without shots:** not printed (verified); optional empty-scene row proposed in the stress test only.
- **No pictures at all:** the Image column is omitted everywhere.
- **Changed + removed:** removed wins (one status row).
- **Page-end whitespace:** a scene that does not fit moves whole when it has <= 3 shots; otherwise it splits. Page 4 of the sample is 83 % full and the stress test's page 1 is 61 % full for this reason.
- **Summary panel:** counts shots and scenes that have shots; for 36/10 the panel reads "36 shots in 10 scenes" as in the supplied PDF.

## H. Pagination and print rules

- **Paper:** US Letter **portrait**, 46 / 46 / 46 / 66 pt margins (content 520 x 680; v1: landscape 700 x 500).
- **Keep rules:** header content (band, status row, column header) always with >= 1 row; <= 3-row scenes unsplit; no lone last row; rows never split.
- **Repeated header:** the band (right label `CONTINUED` in production, count in the Chrome reference) + status row + column header on every continuation page of a scene.
- **Running header:** `LOW TIDE RADIO · SHOT LIST` top-right on every page (document has more than one page); the current footer text carried this string at the bottom, it moves up so the footer stays the shared brand footer.
- **Footer / numbering:** shared system footer, "Page X of Y".
- **Primary sample result:** 5 pages, rows (shots) per page 6 / 8 / 8 / 7 / 7 = 36; content fill 96 / 85 / 85 / 83 / 99 %. p1: scenes 1-2; p2: 8, 13; p3: 16, 19; p4: 25; p5: 34, 40, 42. No scene splits in the sample; the stress test (8 pages, shots per page 3 / 8 / 8 / 8 / 7 / 8 / 6 / 2) shows splits of scenes 19 and 25. The sample is tight: scene 42 fits at the bottom of p5 with a few points to spare; a slightly different font metric could push it to a 6th page.

## I. HTML/PDF correspondence

`Shot_List_Entire_Project.html` generated by `tools/build/shot-list/build.py`, rendered with `tools/render_pdf.py`, checked with `verify_pdf.py --size letter --header "LOW TIDE RADIO · SHOT LIST"` and `--gray`. PDF state: the real 36 shots / 10 scenes, option "include removed" OFF, scene 40 flagged changed. Pictures: 1A uses `shot-sample-01.png` (an app screenshot that is the real embedded reference image of shot 1A in the supplied export; kept as real sample data), 25A and 25G use `shot-sample-02/03.png` (storyboard panels).

Preview toggle: *Option on: shots of removed scenes* shows one **illustrative (fictional)** removed scene "INT. HARBOUR CAFE - DAY" (shots A, B) appended at the end; the summary panel text is static and does not recount when toggled.

Stress test `stress-test/Shot_List_Entire_Project_STRESS_TEST.html/.pdf`: 50 shots in 14 scenes, 8 pages (portrait): the 36 real shots + fictional scenes 44 (six shots with pictures of 16:9, 3:4, 2.39:1, 1:1), 50 (long heading), 51 (no shots, proposed empty row) and two removed scenes (option ON). Shows repeated bands/headers, no orphan bands, removed-scene banners, and the empty-scene row.

Known differences between the Chrome reference and the Rust renderer: (1) Chrome repeats the whole `<thead>` verbatim on a continuation page, so the band repeats with its count; the production renderer must print `CONTINUED` instead of the count (and may keep the slim "(CONTINUED)" bar of the current renderer if the product prefers; the repeated band is the specified design). (2) Chrome centres glyphs with its own Helvetica substitute (Arial on the render machine); widths are metric-compatible but not byte-identical to `pdf-writer` Helvetica. (3) Mixed bold/regular runs in the Notes lines (now under the description) need a small run-wrapping addition to `Txt` in the Rust renderer. (4) Per-cell fills (`Cell.fill`) exist in the renderer; the darker camera header fill needs `header_cells` (or a header tint per column). (5) The thumbnail frame and placeholder wrap are drawn by `CellItem::Image` / `CellItem::Placeholder`, which currently take fixed w/h and a single-line label; the wrapped, growing placeholder is a renderer change. (6) The footer logo is a PNG the production renderer must embed (the current renderer redraws a vector mark and text): implementation requirement.
v2 note: the portrait HTML uses no `landscape` class and `@page { size: 8.5in 11in }`; in the Rust builder remove `.landscape(true)`. Scripts: the three HTML files and their stress tests are generated by `tools/build/shot-list/build.py` from one data model and one CSS block (`python build.py`, then `tools/render_pdf.py`); the specs by `build_md.py`.

## J. Acceptance criteria

Run on the rendered PDF (every page is **612 x 792 pt portrait** Letter; `verify_pdf.py --size letter`):
- [ ] Footer on every page: logo image 17 pt high, aspect 699:163 undistorted, "Created using OpenFrame", "Page X of Y" with the true total; no content inside the bottom 66 pt (`tools/verify_pdf.py --size landscape`).
- [ ] No content outside the 46 pt side margins; no text overlaps its neighbour column; no word is split mid-word in Size, Movement, Angle, Lens or Shot (search the extracted text for "Overhea", "anamorphi", "MOVEMEN" immediately followed by a lone letter on the next line).
- [ ] Column order is Done, Image, Shot, Description, Size, Movement, Angle, Lens (no Notes column); a column is absent exactly when no shot of the export has that data; the grid is identical in every scene table of the document; content width 520 pt.
- [ ] Notes lines appear inside the Description cell, below the description text, in the order Characters, Storyboard, Sound, Camera, with the builder's wording; a shot without notes has no gap or empty line; no text smaller than 6.8 pt.
- [ ] No column header wraps (MOVEMENT, SHOT, DONE stay on one line).
- [ ] Each scene band shows "SCENE n" + heading + "N SHOTS" with N equal to the rows of that scene; a removed scene shows no number, a slate band and the removed sentence.
- [ ] Shot labels equal scene number + letters in order (1A, 1B, 1C ...), removed scenes letters only.
- [ ] Empty optional camera cell prints "–"; empty Notes cell prints nothing; Notes line order is Characters, Storyboard, Sound, Camera.
- [ ] Picture: never cropped (aspect preserved), centred in the 80 x 45 pt frame; missing = "No image" / "Image unavailable: <name>" / "Image can't be printed", label fully visible.
- [ ] No scene band is the last thing on a page; a scene header always has at least one row under it on the same page; the last fragment of a split scene has at least two rows; a scene with three or fewer shots is never split (unless taller than the page).
- [ ] On every continuation page the band and column header repeat; the right-hand label reads CONTINUED instead of the count.
- [ ] Grayscale print (`--gray`): the two status sentences, the band fills and the shot tiles remain distinguishable without colour.
- [ ] With `includeRemoved` off, no group has the removed status; with it on, removed groups come after every active group, have a slate band, no "SCENE n" eyebrow, letter-only labels and the removed sentence.
- [ ] Running header on every page of a multi-page document.
- [ ] Active scenes appear in script order; the summary panel equals the number of printed rows and bands.
- [ ] 40+ shots: no band is the last element of a page; no scene fragment ends with a single row.
- [ ] Determinism: the same snapshot exported twice gives identical page layout (no clock or random value in layout; the "Exported" date is text only).

## K. Known issues and unresolved questions

**Verified defects in the current output:** see B.1 (including continuation bars without count/status and the 33/36 placeholder rows).

**Design choices:** one shared layout for all scopes; band repeated with its status; <= 3-row scenes unsplit; removed scenes slate + red status; empty-scene row only as a proposal; running header on multi-page documents.

**Unverified assumptions / open questions:**
1. Removed-scene output is from source only (no supplied PDF with the option on); the illustrative sample is fictional.
2. Removed scenes are ordered by `scene_lineage_id`, which is not their former script position; whether the product wants them in last-known script order is a product question.
3. A removed scene with an empty snapshot heading would print a titleless band.
4. A scene index / page references are not designed (page numbers are not known when the document is laid out in a single pass).
5. Renderer changes listed in I not validated; Mac parity not checked.

**v2 portrait concerns:** (1) the page-fit of the entire-project sample is tight (scene 42 sits at the bottom of p5, 99 % full); a metric difference in the Rust renderer could add a sixth page. (2) The 160-180 pt Description column is narrow for long descriptions; a very long description plus long notes makes a row hundreds of points tall (stress test 440 pt) and, if it exceeds the 680 pt body, the current renderer cannot split it. (3) Notes under the description separate the notes from the camera block visually; they are read down the description column instead of across the row. (4) Printing 8 / 8.2 pt text on portrait is as small as v1; nothing below 6.8 pt is used.

