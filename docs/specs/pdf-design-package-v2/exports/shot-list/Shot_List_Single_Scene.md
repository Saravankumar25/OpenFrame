# Shot List — Single Scene (shot.export_list)

**Variant:** Single Scene: scope "Current scene" (one scene group)
**Version / orientation:** v2, **PORTRAIT** US Letter 612 x 792 pt (user decision; supersedes the landscape v1 design). The current builder calls `.landscape(true)` and must change to portrait.
**Status:** Design reference — not integrated; not validated against OpenFrame's Rust renderer
**Repo commit:** `2f5a045416bd2125801329533fc47f986ccc8174` (windows/ tree)
**Date:** 2026-10-23
**Files:** `Shot_List_Single_Scene.html` (source), `Shot_List_Single_Scene.pdf` (rendered), `stress-test/Shot_List_Single_Scene_STRESS_TEST.html/.pdf`

## A. Purpose and users

**Who.** The 1st AD and the DP (with the camera/grip/G&E department heads) carry this one-page sheet to set for ONE scene; the director and script supervisor tick shots off against it; the 2nd AD pins it to the day's scene board. It is printed from the Shots tab with "Current scene".

**What they must find in seconds.**
1. *Which scene is this?* Scene number and heading, large, top of the page (heading printed once, not twice).
2. *What is the coverage plan?* The ordered list of shot numbers (1A, 1B, 1C ...), each with a thumbnail frame when there is one.
3. *What camera language does each shot need?* Size, movement, angle, lens, scannable as one tinted block; bold shot size first.
4. *What do I need to prepare for the whole scene?* A coverage strip derived from the listed shots: lenses, movement types (crane, drone, handheld ...), characters named in the shots: a lens/grip/rig prep read without scanning every row.
5. *Which shots are done?* A hand-tick circle per shot (the source prints it in every export).
6. *Is the plan still valid?* "Scene changed since planning" / "Scene removed from the script (kept for reference)" directly under the scene band.

The page must stay a single sheet for the typical scene (3-8 shots) and degrade cleanly to several pages (repeated column header, scene named in the running header) for a long scene.

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

**This variant** = scope "Current scene" (`sceneIds` with one id, `scope_label` = "Current scene", `contents_label` "N shots in 1 scene"), **or any export that resolves to exactly one scene group** (e.g. "Selected scenes" with one scene, or a whole-project export where only one scene has shots). The builder switches layout on `groups.len() == 1`, not on the scope. `includeRemoved` does not apply to a one-scene selection.

Data dependencies: `ShotDto` list for the scene lineage; project title; today's date; picture files (managed or linked assets). Empty/error behaviour: no shots -> error "The selected scenes have no shots yet." and no file.

**B.1 Current-output assessment (supplied `Shot List — Single Scene.pdf`, 1 page, landscape; v2 is portrait).**

- *Retain:* scene grouping with a dark band per scene and a shot count; column order Done, Image, Shot, Description, Size, Movement, Angle, Lens (+ Notes: a column in the current output, a block under Description in v2); "column exists only if the data does" rule; one picture per shot with storyboard fallback; Notes label order and wording; the two status sentences; "(CONTINUED)" continuation cue; hand-tick circle; wrapped (never clipped) text; whole-row page moves; Letter paper (current output landscape; v2 portrait by user decision).
- *Improve visually:* shot number is only bold text (make it a tile); the four camera columns are plain text with no grouping; Notes labels indistinguishable from the text; Notes squeezed into a 114-150 pt column (4-6 line cells); 88 x 50 "No image" frames on most rows make single-line shots 60-70 pt tall; the table pad (7 x 6 pt) wastes width; footer text is a long grey string instead of the brand footer.
- *Verified functional/layout problems in the current PDFs:*
  - "Overhead" breaks mid-word ("Overhea" / "d") in the 50 pt Angle column: Single p1 (shot 1C), Multiple p1 (1C), Entire p1 (1C). Cause: `Col::pt("Angle", 50.0)` with 7 pt padding leaves 36 pt, narrower than the word at 8.2 pt.
  - "35mm anamorphic" (shot 25C) is wider than the 44 pt Lens column's 30 pt inner width and runs into the Notes text: Entire p6.
  - Group-wide empty columns: Entire p2-p5 (scenes 8 and 19, Size only) print empty Movement, Angle, Lens and Notes cells (v2: an empty Notes block simply prints nothing) because column presence is decided for the whole export, not per scene. By design, but a blank cell is ambiguous (not decided vs forgotten).
  - A missing picture's placeholder label is drawn on one line without wrapping (`CellItem::Placeholder` in `report/design/table.rs`), so "Image unavailable: <long original file name>" will overflow its 88 pt frame (code evidence; the supplied PDFs only contain "No image").
  - A row taller than the page body cannot be split (`styled_table` moves a row whole and then emits it regardless), so a very long description/notes cell can run past the footer zone (code evidence; no such row in the supplied PDFs).
  - A selected scene that has no shots is silently left out and not counted (see B); no explanatory line.

  - The scene heading is printed twice on the single page: once as the masthead's third line ("Scene 1: EXT. OPEN SEA - NIGHT (1999)") and again as the full-width band directly below (p1). Cause: `mast.lines.push(scene_title(..))` for `single` **and** the per-group `Banner`.
  - Bottom 40 % of the page is empty; nothing on the page uses it to say more about the scene (design opportunity, not a defect).

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

Single-scene specifics in `export_shots`: `single = groups.len() == 1`; `document = format!("Shot List: {}", scene_title(first))`; `mast.lines.push(scene_title(first))`; `Banner::new(scene_title(first))` with `right = plural(n, "shot", "shots")`, `sub_left` per status, `keep_next = 90.0` (60.0 without any image); table `continued = Some(scene_title(first))`.

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

Single-scene additions (design, **derived from the listed shots only, no new source field**):

| Visible element | Derived from | Kind | Empty-value treatment |
|---|---|---|---|
| Coverage strip cell "Lenses" | distinct `ShotDto.lens` values, first-appearance order, joined by " · " | derived, optional | cell omitted when no shot has a lens |
| Coverage strip cell "Movement" | distinct `ShotDto.movement` values, same rule | derived, optional | cell omitted |
| Coverage strip cell "Characters in these shots" | distinct names over `ShotDto.characters` | derived, optional | cell omitted |
| Coverage strip as a whole | the three cells above | derived | whole strip omitted (no empty box) when all three are empty |

The label says "in these shots" on purpose: the data is what the shots list, not the scene's breakdown cast. No scene description, page count, INT/EXT or day/night is available in `ShotDto` and none is parsed from the heading.

## E. Content architecture

**Order of blocks (page 1):** masthead (eyebrow, title, project, summary panel) -> rule -> scene card [band -> status row (conditional) -> coverage strip (conditional)] -> one table (column header, rows). One table, one scene; no repeated band inside the table.


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

**Sorting.** Rows in the scene's coverage order (the builder's order); never re-sorted by the design.

**Changes versus the current PDF, with reasons.**
1. *Scene heading appears once.* The masthead third line is dropped; the band is the scene title. Reason: duplicate on a one-page sheet.
2. *Coverage strip added* (derived Lenses / Movement / Characters). Reason: a DP/grip/1st AD wants the scene's lens and rig list at the top of the sheet. Optional: `data-toggle="Scene coverage strip"`; if the product does not want derived content, drop it and the rest of the page is unchanged.
3. *Band carries "SCENE n" as an eyebrow and the heading as the title* (same information as "Scene n: HEADING"). Reason: the number is the first thing scanned on a multi-page set.
4. *Band, status row, header and rows are one attached card* (current: band, 4 pt gap, table). Reason: a status or heading can never be separated from its rows.
5. *Camera block* tinted, Size bold, empty = "–". *Shot tile.* *Notes labels bold.* Reason: scannability of camera vocabulary and shot numbers.
6. *Notes stacked under Description, no Notes column* (v2, user decision for portrait) and padding 5 pt (current 7 x 6). Reason: on a 520 pt page there is no room for a separate Notes column next to four camera columns; stacking keeps every shot's notes next to the shot text.
7. *Camera columns sized from the longest word.* Reason: fixes "Overhea/d" and "anamorphic" overflow.
Unchanged: column order, which columns exist and when, label/wording of every source string, Done circle, picture fallback.


**Changes from v1 (landscape design, same date)** — two user decisions, nothing else in content, wording, field meaning or column order:
1. *Portrait US Letter* (612 x 792 pt, content 520 x 680 pt) instead of landscape (792 x 612, content 700 x 500). `@page { size: 8.5in 11in }`; the `landscape` class is gone from `.doc`.
2. *Notes under Description.* The Notes column is removed; the same lines (Characters, Storyboard, Sound, Camera, same order and wording) are stacked beneath the description text in the same cell. A shot without notes prints nothing extra.
Consequences re-derived for 520 pt: columns Done 32, Image 88 (frame still 80 x 45), Shot 38+, Description = remainder (160-180 pt), camera columns measured (Size 43-46, Movement 56, Angle 46, Lens 40-52; camera text 8.2 pt instead of 8.4 pt so that MOVEMENT and "anamorphic" fit without splitting); band min height 34 pt (was 36), scene gap 10 pt (was 12), coverage strip columns equal; the multiple-scenes sample is now one page, so its running header is dropped (rule unchanged: header only when the document has more than one page).
Pages, v1 landscape -> v2 portrait: single sample 1 -> 1; multiple sample 2 -> 1; entire sample 7 -> 5 (baseline 8); stress tests single 3 -> 3, multiple 3 -> 2, entire 10 -> 8.
Honest readability notes from the portrait render: rows that carry notes are taller (70-106 pt versus about 55-85 pt in v1) and the description text wraps in a 160-180 pt column (about 32-36 characters per line at 8.8 pt), but there are fewer pages because the page body is 680 pt tall instead of 500 pt; description-only rows stay at 55-56 pt (the picture frame). Camera cells wrap two-word values ("Eye level", "Handheld on the boat", "100mm macro") onto two lines in the 40-56 pt columns; every word still fits on one line.

## F. Visual specification

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

**F.10 Single-scene composition (portrait).** Masthead (about 64 pt incl. rule) + scene card (band 34 pt; status row 22 pt when present; coverage strip about 40-60 pt because its values wrap in a 520 pt card) + table. For the 3-shot sample the page is about 59 % full (3 rows of 94, 73 and 59 pt) and stays one page; the remaining space is intentionally left white (set notes are handwritten there). Coverage strip: three **equal** columns on `--of-panel` (v1: 1.2 : 1.1 : 1), 0.5 pt hairlines (outer `--of-line-strong`, inner `--of-line`), cell padding 6 / 10 / 7 pt, label 6.8 pt bold uppercase tracking .11 em `--of-slate`, value 8.6 pt bold `--of-navy`, wraps over lines; any missing cell collapses (remaining cells share the width).

## G. Dynamic behaviour

- **Optional columns:** present only when some shot has the data (Image, Size, Movement, Angle, Lens, Notes). Done, Shot, Description always. The grid is recomputed for the export; description and notes share the remainder equally; when Notes is absent Description takes it all.
- **Long description / notes / camera values:** wrap, the row grows. Size/Movement/Angle/Lens are up to 200 characters each in the data model; a long value wraps inside its column (words are never split unless a single word exceeds the 84 pt cap, then it breaks at character level). Nothing is ellipsised.
- **Long scene heading:** wraps inside the band (band height grows in 16 pt line steps); the eyebrow and the count stay on the first/centre line.
- **Missing picture:** placeholder frame with the source label, wrapped; frame grows if the label needs it. Odd aspect ratios letterbox.
- **Optional camera fields empty:** "–" in existing camera columns; missing column printed nowhere.
- **Status:** removed wins over changed; one status row.
- **Zero shots / empty scene:** cannot occur in this variant (the builder returns the error "The selected scenes have no shots yet." instead of a document).
- **Many shots (more than a page):** the table continues on the next page, header repeats; the running header then carries "<PROJECT> · SHOT LIST · SCENE n" because the scene card is not repeated (stress test: 14 shots, 3 pages).
- **Coverage strip:** de-duplicated, order of first appearance, wraps; cell dropped when empty; strip dropped when all empty.
- **Very tall single row** (description up to 2 000 characters plus notes up to 4 000): the design allows the row to split between text lines when it exceeds the page body (the stress test's 118B is 270 pt tall and is kept whole); the current renderer does not split rows (see B.1).

## H. Pagination and print rules

- **Paper:** US Letter **portrait**, margins 46 / 46 / 46 / 66 pt, content 520 x 680 pt (v1: landscape 700 x 500).
- **Page 1 keep rules:** masthead + scene card + column header + at least the first row stay together (`keep_next`-style: the card is never left alone at the bottom of a page; if the first row does not fit under it the whole card moves). Rows never split. The final page of the table carries at least two rows (no lone last row).
- **Repeated header:** the column header row repeats on every continuation page. The scene band is not repeated (it is a card above the table) so the scene is named in the running header instead.
- **Running header:** top-right, only when the document has more than one page: `<PROJECT> · SHOT LIST · SCENE <n>` (uppercase, single line, no rule/logo). The primary sample is one page, so it has none (verified with `--no-header`). Decision: not on one-page sheets because the masthead already names project and scene.
- **Footer / numbering:** shared system footer; "Page X of Y" from the real count.
- **Primary sample result:** 1 page (59 % of the content height; rows of 94 / 73 / 59 pt).

## I. HTML/PDF correspondence

`Shot_List_Single_Scene.html` is the Chrome-printed reference. It uses `../../00_SYSTEM/assets/openframe-print.css` + one inline `<style>` (generated). The PDF was rendered with `python tools/render_pdf.py` (headless Chrome 155, `--no-pdf-header-footer`, margins and footer from `@page`) and checked with `python tools/verify_pdf.py <pdf> --size letter --no-header`.

State captured in the PDF: scope Current scene, scene 1 "EXT. OPEN SEA - NIGHT (1999)", 3 real shots (1A with the app-screenshot image `shot-sample-01.png`, which is how the user's real project looks: **that sample picture is a screenshot of the OpenFrame app welcome dialog embedded as shot 1A's reference image in the supplied export; it is kept as real sample data**), coverage strip on, no status row.

Preview toggles (toolbar): *Scene coverage strip* (on), *State: scene changed since planning* (off), *State: scene removed from the script (kept for reference)* (off). Toggling both statuses together is a preview convenience; the builder prints at most one.

Stress test `stress-test/Shot_List_Single_Scene_STRESS_TEST.html/.pdf`: scene 118 with a 3-line-wrapping heading, "changed" status, 14 shots on 3 pages (portrait; rows per page 1 / 4 / 9): a 440 pt row (very long description + long notes + long camera values; the page-1 gap is the tall row moving whole to page 2), missing pictures ("No image", "Image unavailable: <long name>", "Image can't be printed"), 1:1 / 3:4 / 2.39:1 pictures (derived from the sample images, in `stress-test/assets/`), description-only shots, notes-only shots, a lens-only shot, a very long hyphenated character name, more than seven characters, repeated column header, running header with the scene number, no lone last row.

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
- [ ] The scene heading appears exactly once on page 1.
- [ ] Coverage strip lists each distinct lens / movement / character once, in order of first appearance; it is absent when none exist.
- [ ] For a multi-page single scene every page after the first has the repeated column header and the running header with the scene number; page 1 does not repeat the scene card.
- [ ] Determinism: the same snapshot exported twice gives identical page layout (no clock or random value in layout; the "Exported" date is text only).

## K. Known issues and unresolved questions

**Verified defects in the current output:** see B.1 (duplicate heading, "Overhea/d", "anamorphic" overflow, ambiguous blanks, single-line placeholder label, unsplittable tall rows, silent omission of empty scenes).

**Design choices:** coverage strip (derived, optional); "–" for empty camera cells; camera tint; bold Size; notes under description; band eyebrow; running header on multi-page documents only; repeated band is replaced by the running header in this variant.

**Unverified assumptions / open questions:**
1. Product decision: is derived scene coverage (lenses/movement/characters) wanted on the sheet? It is derived from the shots, not from the scene's breakdown.
2. Should a one-page sheet use the leftover space for larger thumbnails when a scene has few shots? Not designed (adaptive row height needs a measuring pass).
3. Shot labels are positional (see B): a printed sheet can disagree with the app after reordering. The only mitigation available without new data is the "Exported <date>" text in the summary panel; a revision/export id would need a new field.
4. The renderer change list in I (mixed bold runs, wrapped placeholder, per-column header tint, 5 pt padding) is not validated.
5. Mac parity not checked.

**v2 portrait concerns:** (1) the page-fit of the entire-project sample is tight (scene 42 sits at the bottom of p5, 99 % full); a metric difference in the Rust renderer could add a sixth page. (2) The 160-180 pt Description column is narrow for long descriptions; a very long description plus long notes makes a row hundreds of points tall (stress test 440 pt) and, if it exceeds the 680 pt body, the current renderer cannot split it. (3) Notes under the description separate the notes from the camera block visually; they are read down the description column instead of across the row. (4) Printing 8 / 8.2 pt text on portrait is as small as v1; nothing below 6.8 pt is used.

