# Production Report — Location Report  (`reports.export_report`, `reportType: "location"`)

| | |
|---|---|
| Export | Production reports, op `reports.export_report` (PDF; CSV and XLSX are separate projections of the same `ReportData`) |
| Variant | **Report_Location** — portrait, 5 columns |
| Status | Design reference — not integrated; not validated against OpenFrame's Rust renderer |
| Version | v2 — Shooting days in shoot-day order (see K.4 Changes from v1) |
| Repo commit | `2f5a045416bd2125801329533fc47f986ccc8174` (`main`) |
| Date | 2026-10-23 |
| Files | `exports/production-reports/Report_Location.html` / `.pdf` / `.md`; stress tests in `exports/production-reports/stress-test/` |
| Shared shell | Defined once in `Report_Scene.md`, section **F.1 Shared report shell**; the other five specs reference it and add their own column specs |


## A. Purpose and users

**Who:** the locations manager and scouts; also the 1st AD, transport/unit manager and production coordinator when they plan base camps and permits.

**When:** during scout and booking, and again before each shooting week.

**What they must find fast:** the location name (with the script's own descriptor), its **status** (Idea, Shortlisted, Confirmed, Rejected, or From heading when no location record exists yet), the **address** to give transport and permits, which **scenes** happen there, and on **which shoot days**. A location that is not yet Confirmed must stand out in a list of 16-40 rows without colour: the status chip carries a word and a shape, and the list is alphabetical so the manager finds a place by name.

## B. Verified functionality

**Scope.** One production report, generated on demand from the current Production Source / current schedule, or a saved snapshot of one. Six live report types exist (`reportType`): `scene`, `location`, `cast_scene`, `prop`, `schedule`, `breakdown_completeness` (labels in the Reports tab: Scene Report, Location Report, Cast Scene Report, Prop Report, Schedule Report, Breakdown Completeness). This spec covers **Location Report** (`location`). Saved snapshots use the identical `ReportData` schema and print through the same layout (see "Saved snapshot").

**Inputs / args (verified, `ExportReportArgs` in `exports/reports.rs`, `deny_unknown_fields`, camelCase).**

| Arg key | Meaning |
|---|---|
| `format` | `"pdf"` \| `"csv"` \| `"xlsx"` (the UI offers PDF, CSV, Excel (XLSX) from `TABLE_FORMATS`) |
| `path` | destination chosen in the export dialog |
| `reportType` | live report type, optional; used when `reportId` is absent |
| `filter` | live text filter, optional (the Reports tab box "Filter rows (scene, location, name…)", max 200 chars, trimmed, blank = none) |
| `reportId` | a saved snapshot instead of a live report (`production_report` row, not deleted); when present it wins over `reportType`; neither present -> error "A report" required |

**Options.** None beyond format. The export dialog (`ReportExportButton` -> `ExportDialog`, title `Export Location Report`, source line `Source: Location Report` plus ` · filtered by “…”` when a filter is typed, or `Source: saved report <title>`) has no scope or checkbox options.

**Data dependency.** `reports.generate` -> `build_report` in `modules/schedule/docs.rs`. It needs a Production Source (taken from the current schedule's source, otherwise the active source) and reads non-omitted scenes (`!s.omitted`) and, for day information, the current schedule board.

**Filter semantics (verified).** Case-insensitive substring match: a row is kept when **any cell** contains the text (`report_filter`, `rows.filter(... any(cell.to_lowercase().contains(..)))`).

**Empty / error behaviour (verified).**
- No source selected: `columns = []`, `rows = []`, `note = "Choose a Production Source in Breakdown first."` (`NO_SOURCE_MESSAGE`), `sourceLabel = ""`. The PDF prints masthead (no `Source:` part), zero-value metrics and an info callout with that sentence; portrait (0 columns).
- Source but no rows (e.g. filter matches nothing): columns are still returned, `note = "No matching data."`; the PDF prints masthead, zero metrics and an info callout with the note; **orientation follows the column count, so an empty Location Report is portrait**.
- Unknown `reportType`: validation error "Choose one of the available reports." Filter longer than 200 characters: validation error "Keep the filter under 200 characters." Export needs the Export capability (`require_export`).
- A report row cell that the PDF font cannot draw: export is refused with `HINT_TABLES` ("Export it as CSV or Excel (XLSX) to keep them, or replace those characters and export the PDF again.") — behaviour inherited from the shared export pipeline.
**Location Report specifics.** Columns (5): `Location`, `Status`, `Address`, `Scenes`, `Shooting days`. One row per distinct location name (case-insensitive key) used by non-omitted scenes, alphabetical by lowercase name (`BTreeMap`). `Status` = the location record's status or `From heading` when the name only appears in scene headings; known values: `Idea`, `Shortlisted`, `Confirmed`, `Rejected` (`LOCATION_STATUS` in the AI catalog) and `From heading`. `Address` = record address or empty. `Scenes` = scene numbers joined `, ` in scene order. `Shooting days` = distinct day titles (`Shoot Day N · Mon 18 Jan 2027`) joined by `; `. **The builder emits them in first-seen (scene) order; this design and v2 require shooting-day order (see B.1 and K.4).** Metric: `Locations` = row count.

**Saved snapshot.** `reports.save` stores `ReportData` JSON in `production_report` (`title` defaults to `"Location Report: <source label>"`, user-editable up to 160 chars). Exporting with `reportId` prints that stored data: the masthead title is the **snapshot title**, the generated date is the snapshot's, the scope reads "Saved snapshot", and the stored `filter` (if any) is available to print. Rows never change after saving.

### B.1 Current-output assessment

Baseline: `Production Report — Location.pdf` (2 pages, portrait, 22 Oct 2026, project Low Tide Radio, source `LOW TIDE RADIO: Blue Revision (Locked)`).

**Retain**
- Columns, labels and order; alphabetical order (p1-p2).
- `Status` as a chip (`CONFIRMED` green) and location names bold navy (p1).
- Metric `Locations` = 16.

**Improve visually**
- `Shooting days` is one run-on paragraph of `; `-joined dates in a 100 pt column and wraps into up to 7 lines (p1: Coast Radio Station, `Shoot Day 2 · Tue 12 / Jan 2027; Shoot Day / 3 · Wed 13 Jan …`); each day is now its own line with the day number in bold.
- The current widths (Location, Address and Shooting days 100 pt each by weight 1.2, Scenes 125 pt by weight 1.5, Status 96 pt) wrap names and addresses early and give the narrow Scenes column too much room; widths are re-balanced (Location 120, Status 84, Address 108, Scenes 58, Shooting days 150 pt).
- Masthead height: page 1 shows 8 rows, the whole list fits on 1 page (current: 2 pages).

**Verified functional / layout problems (current output)**
- Nothing functionally wrong in the sample; all 16 statuses are `Confirmed`, so the chip column is uniform (p1-p2). A non-Confirmed row is the case the report exists for and has not been exercised by the real data (see stress file).
- **Shooting days are jumbled, not in shooting-day order** (user feedback; baseline `Production Report — Location.pdf` p1, row Coast Radio Station: `Shoot Day 2 · Tue 12 Jan 2027; Shoot Day 3 · Wed 13 Jan 2027; Shoot Day 1 · Mon 11 Jan 2027; Shoot Day 14 · Tue 26 Jan 2027`; it is the only row of the 16 that is out of order, the others happen to be ascending). Cause (verified, `modules/schedule/docs.rs`, `build_report`, arm `"location"`): `for sc in &scenes { for l in src.locations(sc) { … if let Some(d) = day_of(&sc.id) && !e.4.contains(&d) { e.4.push(d); } } }` — days are appended in the order scenes are visited (script order) and joined with `; `; nothing sorts them, and `reports.rs::export` prints the cell as given. The `Scenes` list in the same row is built the same way and is ascending only because scenes are visited in script order.
- Same shell defects as the Scene Report (side panel placeholder `Export reference`, filter text in a 150 pt panel, snapshot filter dropped).

**Cannot be determined from the evidence**
- Whether `From heading` locations (no record, therefore no address) occur in the user's data — the sample has none.
- Whether the app will sort in the builder (recommended) or in the renderer; either must follow the sort rule in K.4.


## C. Source mapping

| Item | Verified identifier |
|---|---|
| Commit | `2f5a045416bd2125801329533fc47f986ccc8174` |
| UI entry point | `windows/apps/desktop/src/workspaces/production/tabs/reports.tsx`, component `ReportsTab` (tab id `reports`, label "Reports"); the list entry "Location Report" in the left `vpanel` listbox; export button = `ReportExportButton` (from `windows/apps/desktop/src/features/export/buttons.tsx`) placed in the `PageHeader` actions, next to "Save snapshot". |
| Op name | `reports.export_report` (registered in `exports/mod.rs`, `r.query("reports.export_report", reports::export)`, capability Export) |
| Builder | `windows/crates/openframe-application/src/modules/exports/reports.rs` -> `export()` (+ helpers `report_col`, `report_cell`, `status_tone`, `is_numeric_column`) |
| Data | `reports.generate` / `build_report` / `struct ReportData { report_type, title, source_label, generated_at, columns, rows, note, filter }` in `modules/schedule/docs.rs`; saved: `reports.save`, `reports.get`, table `production_report` (`title`, `snapshot_json`) |
| Builder fn for this report's rows | `build_report` arm `"location"` in `modules/schedule/docs.rs` (`BTreeMap` keyed by lowercase name; `l.status`, `l.address`; `day_of`) with `report_cell` (`Status` -> chip via `status_tone`) in `exports/reports.rs` |
| Renderer blocks used (current) | `Document::new(..).landscape(columns > 5).footer(footer(project, data.title))`; `Block::Masthead` (via `production_masthead`, `title_size = 26`); `Block::Metrics(compact)` of `Metric`s; `Block::Styled(StyledTable{ header: HeaderStyle::Navy, boxed, zebra, continued: Some(data.title) })` with `Row`/`Cell`; cell items `CellItem::Tile` and `CellItem::Chips(Chip::label(.., Tone))`; `Block::Callout{Tone::Info}` for the empty state; `Block::Text` (8.4 pt slate) for `data.note` under the table; plain `Table` pushed to `Output.tables` for CSV/XLSX. |
| Design needs from the renderer (new) | running header (single line, top-right, multi-page only); compact masthead; context banners (snapshot / filter); fit-content metrics strip; status chip with leading shape (disc / ring+dot / ring / square); left stripe on attention rows; two-line "day / date" cell; stacked days list cell; letter-divider rows (Prop only); footer logo PNG embedded (current renderer redraws a vector mark + text — implementation requirement from the shared brief). |


## D. Field inventory

Shared shell rows first, then this report's table.

| Visible element | Source field / derivation | Required / optional / derived | Empty-value treatment |
|---|---|---|---|
| Running header (multi-page only) | `session` project title (`project_title`) + `data.title` | derived | Omitted when single-page; project part omitted if blank. |
| Eyebrow `PRODUCTION REPORT` | constant passed to `production_masthead` (`"Production report"`) | constant | — |
| Title (h1) | `document` = saved snapshot title (`production_report.title`) or `data.title` | required | — |
| `Project: …` | `project_title` | optional | Part of the meta line omitted when blank. |
| `Source: …` | `data.source_label` = `<screenplay_title>: <label>` | optional | Omitted when blank (no Production Source). |
| `GENERATED` date | `data.generated_at` -> `date_from_ms` (snapshot: stored `generated_at`) | required (derived) | — |
| Scope word | snapshot -> `Saved snapshot`; filter non-blank -> `Filtered rows`; else `All rows` (source panel text for a filter is `Filtered by “…”`; see F.1 item 3) | derived | — |
| Snapshot banner | `saved_title.is_some()` (export by `reportId`) | conditional | Omitted for live reports. |
| Filter banner | `data.filter` (`nb()`), live or stored in the snapshot | conditional | Omitted when blank. |
| Metrics strip | counts over `data.rows` (see below) | derived | Zeros when no rows; `Rows` fallback label for an unknown `report_type`. |
| Table header | `data.columns` | required | No table when `rows` is empty. |
| Note under table | `data.note` (`nb()`) | optional | Omitted when absent. |
| Empty-state callout | `data.note` or `No matching data.` | conditional (rows empty) | — |
| Metric `Locations` | `rows.len()` | derived | 0 |
| `Location` | `l.name` (display name as first seen) | required | — |
| `Status` chip | `l.status` or `From heading` | required (derived default) | chip word; `Idea` is neutral (no mark) |
| `Address` | `l.address` | optional | Empty cell |
| `Scenes` | scene numbers joined `, ` | required (>= 1 scene) | — |
| `Shooting days` (one line per day) | day titles (`DayRow::title`) joined `; ` | optional | Empty cell for a location with no scheduled scene |

## E. Content architecture

**Order and hierarchy.** Shell, then one table, alphabetical by location. Row = bold location name (key) | status chip | address | scene numbers | stacked shooting days (bold `Shoot Day N`, then ` · Mon 18 Jan 2027`).

**Table columns** (design order shown; widths resolved by the generator `tools/build/production-reports/gen.py::widths`, the same rule the Rust renderer should apply):

| Column (exact header) | Width at current column set | Align | Wrap | Treatment |
|---|---|---|---|---|
| `Location` | 120 pt (flexible, weight 120) | left | wraps | bold navy; the script descriptor in parentheses stays part of the name |
| `Status` | 84 pt (fixed) | left | no wrap (chip) | `.st` chip, tone from `status_tone` |
| `Address` | 108 pt (flexible, weight 108) | left | wraps | regular ink |
| `Scenes` | 58 pt (flexible, weight 58) | left | wraps at `, ` | regular ink, scene order |
| `Shooting days` | 150 pt (flexible, weight 150) | left | one day per line; a day wraps only if it is longer than the column | `<b>Shoot Day N</b> · date` |

Sum of widths = 520 pt (content width of portrait Letter minus 2 x 46 pt margins).

**Sorting:** rows: none added (alphabetical by lowercase name from `build_report`). **Within a row, `Shooting days` are sorted: stable ascending by shoot-day number, tie-break by date, unscheduled / undated days last (requirement for the builder or renderer; the builder currently emits first-seen order).** `Scenes` is already ascending and is not changed.

**Changes versus the current PDF, and why**
- **v2: `Shooting days` listed in shooting-day order** (Coast Radio Station: Shoot Day 1, 2, 3, 14), see K.4.
- `Shooting days` is split on `; ` into one line per day with the label bold. Why: the joined string wraps into an unreadable paragraph (p1).
- Widths re-balanced; `Status` 96 -> 84 pt (`CONFIRMED` needs about 56 pt, the longest value `FROM HEADING` about 76 pt).
- Shared shell replaces the old masthead; no running header on the 1-page sample.


## F. Visual specification

The shared shell (running header, masthead, context banners, metrics strip, table styling, status chip, orientation rule) is specified in `Report_Scene.md` **F.1 Shared report shell**; the values below are only what is specific to this report.

**Page:** Letter portrait, content 520 x 680 pt. `@page { size: 8.5in 11in; }` plus `@top-right` only when the report is more than one page (the sample is one page, so none; the stress file, 6 pages, has it).

**Report-specific treatment:** `Confirmed` = ok chip (disc); `Shortlisted` = warn (ring); `Rejected` = danger (square); `Idea` = neutral, no mark; `From heading` = info (ring+dot). No row stripe is used in this report (status chip only) to keep a uniformly `Confirmed` list calm; a non-confirmed row is found by its chip word and shape.

**Typography/colours/footer:** as F.1.

## G. Dynamic behaviour

Shared rules are in `Report_Scene.md` F.1/G; the items below are this report's specifics.

- A location with no scheduled scenes: `Shooting days` cell empty.
- Very long `Shooting days` (14 day lines) and 60 scene numbers (stress row 1): the row grows tall (about 200 pt); a row taller than a page is not expected, but if it happens the renderer must allow a split between cell lines (open, K).
- Long address (200+ chars) wraps in the 112 pt column.
- Long unbroken name: wraps by character.
- Filter `vypin` keeps rows where any cell matches (name, address or day text).

**Always true for every report (see Scene G):** rows grow to fit wrapped text and are never ellipsised; a row is never split across pages; header row repeats on every page; an optional banner/note that is absent leaves no gap; an empty result prints the source's note in an info callout and no table; unknown/extra columns use the generic fallback widths and make the document landscape when `columns > 5`.

## H. Pagination and print rules

- Paper: US Letter, **portrait 612 x 792 pt** (`columns.len() = 5`; rule `> 5` -> landscape). Margins 46 pt left/right/top, 66 pt bottom (footer zone). Content width 520 pt.
- Table header repeats on every page (`thead` navy row). There is **no table title row**; the report's name on continuation pages is carried by the running header (see F.1 item 1).
- Rows `break-inside: avoid`. A row that does not fit moves whole to the next page. Minimum rows: never leave a single row alone on the last page when it can be avoided — move one more row with it (current output violates this, see K).
- Masthead, context banners and metrics strip are `break-after: avoid`: they stay with the first table rows; page 1 must show at least 3 table rows below the metrics, otherwise the table starts on a new page together with its header.
- A row is never split; the tallest sample row (Coast Radio Station: 4 day lines) is about 55 pt.
- 1 page for the 16-row sample; `Page 1 of 1`, no running header.
- Footer and numbering: shared system — official logo (17 pt high) + `Created using OpenFrame` bottom-left, `Page X of Y` bottom-right, on every page. The old footer text `<project> · <report title>` is **not** reproduced in the footer (brand rule); that identity moves to the running header.
- Running header: `@top-right`, `{PROJECT NAME} · LOCATION REPORT`, only when the report spans more than one page.

## I. HTML/PDF correspondence

- `Report_Location.html` is the printable document: one `<div class="doc">`, shared print CSS linked from `../../00_SYSTEM/assets/openframe-print.css`, shell + report CSS in one inline `<style>`, `of-preview.js` for the screen-only toolbar. Generated by `tools/build/production-reports/gen.py` from the real rows in `tools/build/production-reports/data/location.json` (extracted from the supplied baseline PDF by `extract_real_rows.py`); the HTML and the PDF therefore contain the same content.
- **PDF captured state:** live report, no filter, not a snapshot, as the user would export it on 22 Oct 2026 from `LOW TIDE RADIO: Blue Revision (Locked)`: 16 real rows, **1 page** (current output: 2). Rendered with `python tools/render_pdf.py Report_Location.html Report_Location.pdf` (Chrome 155 headless, `@page` margin boxes for the running header, logo footer and `Page X of Y`). `python tools/verify_pdf.py Report_Location.pdf --size letter --no-header` passes.
- **Preview toolbar toggles (`data-toggle`):**
  - `Filter active` — banner with sample text `vypin`, filters rows, recounts `Locations`.
  - `Saved snapshot (illustrative)` — snapshot banner and snapshot title (`Location Report: LOW TIDE RADIO: Blue Revision (Locked)`).
- **Stress tests** (`stress-test/`; rows marked fictional are illustrative and are never in the primary PDF):
  - `Report_Location_STRESS_TEST` (6 pages): saved-snapshot identity with a 160-char title, a 200-char filter, a very long project and source label, extreme cell text, rows with missing fields, every status variant, then many fictional rows so the table runs several pages with repeated headers; the optional note slot is filled.
  - `Report_Location_STRESS_TEST_WIDE` (2 pages): 9 columns (the 5 real ones + 4 fictional: Contact, Parking, Permit, Notes) -> the **whole document is landscape** (rule `columns > 5`); shows the generic column rules.
  - `Report_Location_STRESS_TEST_EMPTY` (1 page): a filter that matches nothing -> metrics at 0 and the source's note `No matching data.` in an info callout; portrait.
  - The no-Production-Source state is identical for all six reports apart from the title and metric labels; it is shown once, in `Report_Scene_STRESS_TEST_NO_SOURCE`.
- **Known differences versus the Rust renderer:** Chrome lays out tables with CSS (`table-layout: fixed` with `<colgroup>` widths); the Rust renderer must measure with the same metrics it paints (existing rule) so wrap points can differ by a word. Letter-spacing, `box-shadow` stripes and the ring/dot shapes are drawn with rects/circles in the renderer (no CSS features are relied on beyond flat fills, hairlines and rounded rects). Chrome keeps a divider/row with its neighbour via `break-after/inside: avoid`; the Rust pager must implement the equivalent. The HTML status/filter toggles are preview-only JavaScript and recompute metrics and zebra from the visible rows; in the app those values come from `build_report`.

## J. Acceptance criteria

- [ ] Page size is Letter portrait; margins 46/46/46/66 pt; nothing drawn in the 66 pt footer band or outside the side margins (`tools/verify_pdf.py --size letter` passes).
- [ ] Footer on every page: logo lockup + `Created using OpenFrame` + `Page X of Y` with the true total.
- [ ] Masthead shows, in this order: eyebrow `PRODUCTION REPORT`; title; `Project: … · Source: …` (a part is omitted when blank); right panel `GENERATED` / date / scope (`All rows` | `Filtered rows` | `Saved snapshot`). There is no `Export reference` line and no second logo.
- [ ] Metrics strip labels and counts equal the source's metrics for this report (`Locations`) and are recomputed from the rows actually printed (filtered rows -> filtered counts).
- [ ] Column headers are the exact `data.columns` strings, in the order given in section E; header row repeats on every page; header text never splits mid-word.
- [ ] No cell text is truncated or ellipsised; very long unbroken strings wrap by character inside their column; no cell overflows its column.
- [ ] Every status value is readable with colour removed: the word is always printed, and the chip carries its shape (disc / ring+dot / ring / square). `verify_pdf.py --gray` thumbnails reviewed.
- [ ] Zebra restarts correctly and body text is >= 8.4 pt; chips >= 6.6 pt; running header 6.8 pt.
- [ ] Empty result: no table, metrics show 0, info callout shows exactly the note text (`No matching data.` or `Choose a Production Source in Breakdown first.`), no running header, one page.
- [ ] Filter active: a filter banner prints `Filter` + the filter text (wraps if long) and the scope says `Filtered rows`. Saved snapshot: a snapshot banner prints, the title is the snapshot title, the date is the snapshot date; both banners can appear together.
- [ ] Orientation is decided once per document from the column count and never changes between pages.
- [ ] The whole 16-row sample fits on one page without a running header.
- [ ] Each shooting day is on its own line with `Shoot Day N` bold, in ascending shoot-day order in every row: the Coast Radio Station row lists `Shoot Day 1, 2, 3, 14` (not `2, 3, 1, 14`). Automatic check: for every row parse `Shoot Day (\d+)` in each `Shooting days` cell and compare with the sorted list.
- [ ] Sorting is stable, ties broken by date; a day without a number or date is last. `Scenes` stays ascending.
- [ ] Every `Status` value prints its word plus the mark for its tone; a `Rejected` row is distinguishable in a grayscale print.
- [ ] Location rows remain alphabetical (case-insensitive).

## K. Known issues and unresolved questions

**Verified defects in the current output**
- `Shooting days` run-on paragraph (p1-p2).
- Days not in shooting-day order (p1, Coast Radio Station; user feedback) — see B.1.
- Shell defects shared with the Scene Report (placeholder line, filter in the side panel, snapshot filter dropped).

**Design choices (made here, reversible)**
- Stacked days with bold day label.
- No row stripe; status chip only.
- Running header omitted for a one-page report.

**Unverified assumptions / open questions**
- Chronological sorting of the day list (would be a source change in `build_report`).
- Whether the renderer can split an over-tall row; none occurs in the sample.
- Splitting the joined `Shooting days` string on `; ` relies on day titles not containing `; ` (labels are `Shoot Day N` / `Off Day`).

### K.4 Changes from v1

- **Fixed: `Shooting days` order.** v1 printed the days in the order the builder emits them (first-seen, scene order), e.g. Coast Radio Station `Shoot Day 2, 3, 1, 14`. v2 sorts them ascending: `Shoot Day 1, 2, 3, 14`. Rule (requirement for the builder / renderer): stable ascending by shoot-day number, tie-break by date, unscheduled / undated days last. Origin: `modules/schedule/docs.rs::build_report`, arm `"location"`, vector `e.4` (appended while visiting scenes, joined with `; `); the recommended fix is to sort `e.4` by (day number, date) before the join, which also fixes CSV/XLSX and the Reports tab.
- Affected files: `Report_Location.html/.pdf/.md` and the Location stress tests (`_STRESS_TEST`, `_STRESS_TEST_WIDE`; `_EMPTY` has no rows). Rows, columns, wording, widths and pagination are otherwise unchanged (still 1 page).
- Checked and unchanged: `Scenes` is ascending in all 16 rows; row order stays alphabetical.
