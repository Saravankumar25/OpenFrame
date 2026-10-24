# Production Report — Schedule Report  (`reports.export_report`, `reportType: "schedule"`)

| | |
|---|---|
| Export | Production reports, op `reports.export_report` (PDF; CSV and XLSX are separate projections of the same `ReportData`) |
| Variant | **Report_Schedule** — portrait, 5 columns |
| Status | Design reference — not integrated; not validated against OpenFrame's Rust renderer |
| Repo commit | `2f5a045416bd2125801329533fc47f986ccc8174` (`main`) |
| Date | 2026-10-23 |
| Files | `exports/production-reports/Report_Schedule.html` / `.pdf` / `.md`; stress tests in `exports/production-reports/stress-test/` |
| Shared shell | Defined once in `Report_Scene.md`, section **F.1 Shared report shell**; the other five specs reference it and add their own column specs |


## A. Purpose and users

**Who:** the production office / production manager, the 1st AD, transport and catering; anyone who needs the shoot calendar on one sheet.

**When:** after the stripboard is built and after each schedule change.

**What they must find fast:** which **shoot day** is which date, which **scenes** are on it **in shooting order**, which **locations** it visits, how long the day is **estimated** to run, and which days are **off days**. Unscheduled scenes (if any) appear as a final row.

## B. Verified functionality

**Scope.** One production report, generated on demand from the current Production Source / current schedule, or a saved snapshot of one. Six live report types exist (`reportType`): `scene`, `location`, `cast_scene`, `prop`, `schedule`, `breakdown_completeness` (labels in the Reports tab: Scene Report, Location Report, Cast Scene Report, Prop Report, Schedule Report, Breakdown Completeness). This spec covers **Schedule Report** (`schedule`). Saved snapshots use the identical `ReportData` schema and print through the same layout (see "Saved snapshot").

**Inputs / args (verified, `ExportReportArgs` in `exports/reports.rs`, `deny_unknown_fields`, camelCase).**

| Arg key | Meaning |
|---|---|
| `format` | `"pdf"` \| `"csv"` \| `"xlsx"` (the UI offers PDF, CSV, Excel (XLSX) from `TABLE_FORMATS`) |
| `path` | destination chosen in the export dialog |
| `reportType` | live report type, optional; used when `reportId` is absent |
| `filter` | live text filter, optional (the Reports tab box "Filter rows (scene, location, name…)", max 200 chars, trimmed, blank = none) |
| `reportId` | a saved snapshot instead of a live report (`production_report` row, not deleted); when present it wins over `reportType`; neither present -> error "A report" required |

**Options.** None beyond format. The export dialog (`ReportExportButton` -> `ExportDialog`, title `Export Schedule Report`, source line `Source: Schedule Report` plus ` · filtered by “…”` when a filter is typed, or `Source: saved report <title>`) has no scope or checkbox options.

**Data dependency.** `reports.generate` -> `build_report` in `modules/schedule/docs.rs`. It needs a Production Source (taken from the current schedule's source, otherwise the active source) and reads non-omitted scenes (`!s.omitted`) and, for day information, the current schedule board.

**Filter semantics (verified).** Case-insensitive substring match: a row is kept when **any cell** contains the text (`report_filter`, `rows.filter(... any(cell.to_lowercase().contains(..)))`).

**Empty / error behaviour (verified).**
- No source selected: `columns = []`, `rows = []`, `note = "Choose a Production Source in Breakdown first."` (`NO_SOURCE_MESSAGE`), `sourceLabel = ""`. The PDF prints masthead (no `Source:` part), zero-value metrics and an info callout with that sentence; portrait (0 columns).
- Source but no rows (e.g. filter matches nothing): columns are still returned, `note = "No matching data."`; the PDF prints masthead, zero metrics and an info callout with the note; **orientation follows the column count, so an empty Schedule Report is portrait**.
- Unknown `reportType`: validation error "Choose one of the available reports." Filter longer than 200 characters: validation error "Keep the filter under 200 characters." Export needs the Export capability (`require_export`).
- A report row cell that the PDF font cannot draw: export is refused with `HINT_TABLES` ("Export it as CSV or Excel (XLSX) to keep them, or replace those characters and export the PDF again.") — behaviour inherited from the shared export pipeline.
**Schedule Report specifics.** Columns (5): `Day`, `Date`, `Scenes`, `Locations`, `Est. time`. One row per schedule day in board order (`DayRow::label()` = `Shoot Day N` or `Off Day`), `Date` = `date_short` or `No date`, `Scenes` = scene numbers in the day's strip order `, `-joined (shooting order, not numeric), `Locations` = distinct location names of the day `, `-joined, `Est. time` = `minutes(sum.estimated_minutes)` or `Missing` when the estimate is 0. If scenes remain unscheduled a last row is added: `Unscheduled`, empty date, `N scene(s)`, empty, empty. Requires a current schedule board; with no schedule the rows are empty. Metric: `Schedule rows` = row count (off days and the Unscheduled row included).

**Saved snapshot.** `reports.save` stores `ReportData` JSON in `production_report` (`title` defaults to `"Schedule Report: <source label>"`, user-editable up to 160 chars). Exporting with `reportId` prints that stored data: the masthead title is the **snapshot title**, the generated date is the snapshot's, the scope reads "Saved snapshot", and the stored `filter` (if any) is available to print. Rows never change after saving.

### B.1 Current-output assessment

Baseline: `Production Report — Schedule.pdf` (2 pages, portrait, 22 Oct 2026, project Low Tide Radio, source `LOW TIDE RADIO: Blue Revision (Locked)`).

**Retain**
- Columns, labels, order (board order), shooting-order scene lists, `Est. time` values (p1-p2).
- Metric `Schedule rows` = 16 (as emitted).

**Improve visually**
- Day is now bold and the Locations column is 178 pt wide (current: 116 pt) so locations wrap less (p1: `Shoot Day 10` had 5 location lines; Scenes is 96 pt instead of 144 pt, Day and Date 78 / 92 pt).
- The whole 16-row schedule fits on one page (current: 2 pages).
- Off days are visibly quieter (panel fill, slate text).

**Verified functional / layout problems (current output)**
- **`Off Day` rows print a red `MISSING` chip in `Est. time` (p1: Sun 17 Jan 2027; p2: Sun 24 Jan 2027)** — `build_report` sets `Missing` whenever `estimated_minutes == 0`, and `report_cell` turns `Missing` into a danger chip. An off day has no scenes, so a missing estimate is not an error; the alarm is false.
- The metric is labelled `Schedule rows` and counts off days (16 = 14 shoot days + 2 off days), which reads like 16 shoot days (p1).
- Scenes are in shooting order, which looks unsorted next to the other reports (`2, 33, 11`), and the page does not say so (p1).

**Cannot be determined from the evidence**
- How an `Unscheduled` row looks in the real export — the sample has no unscheduled scenes (shown in the stress file only).
- Whether the `Day` of a day with no number but scenes can occur (`Off Day` with strips) — the design shows `—` only when the Scenes cell is empty.


## C. Source mapping

| Item | Verified identifier |
|---|---|
| Commit | `2f5a045416bd2125801329533fc47f986ccc8174` |
| UI entry point | `windows/apps/desktop/src/workspaces/production/tabs/reports.tsx`, component `ReportsTab` (tab id `reports`, label "Reports"); the list entry "Schedule Report" in the left `vpanel` listbox; export button = `ReportExportButton` (from `windows/apps/desktop/src/features/export/buttons.tsx`) placed in the `PageHeader` actions, next to "Save snapshot". |
| Op name | `reports.export_report` (registered in `exports/mod.rs`, `r.query("reports.export_report", reports::export)`, capability Export) |
| Builder | `windows/crates/openframe-application/src/modules/exports/reports.rs` -> `export()` (+ helpers `report_col`, `report_cell`, `status_tone`, `is_numeric_column`) |
| Data | `reports.generate` / `build_report` / `struct ReportData { report_type, title, source_label, generated_at, columns, rows, note, filter }` in `modules/schedule/docs.rs`; saved: `reports.save`, `reports.get`, table `production_report` (`title`, `snapshot_json`) |
| Builder fn for this report's rows | `build_report` arm `"schedule"` in `modules/schedule/docs.rs` (`Board::days`, `day_summary`, `day_strips`, `scene_number`, `unscheduled`, `minutes`, `date_short`) with `report_cell` (`Est. time == "Missing"` -> danger chip) in `exports/reports.rs` |
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
| Metric `Schedule rows` | `rows.len()` | derived | 0 |
| `Day` | `DayRow::label()` | required | `Off Day` printed slate regular; the extra row `Unscheduled` = warn chip |
| `Date` | `date_short(d.date)` or `No date` | required (derived) | Empty for the Unscheduled row |
| `Scenes` | scene numbers of the day's strips in order, `, `-joined; for the Unscheduled row `N scene(s)` | optional | Empty for an off day |
| `Locations` | `day_summary(d).locations` names `, `-joined | optional | Empty for an off day |
| `Est. time` | `minutes(estimated_minutes)` or `Missing` | required (derived) | `Missing` -> danger chip (word + square) on a shoot day; **`—` on an off day** (design change, see E) |

## E. Content architecture

**Order and hierarchy.** Shell, then one table in board order (calendar order of days), off-day rows inline, the Unscheduled row last. Row = bold day label | date | scenes (shooting order) | locations | estimate.

**Table columns** (design order shown; widths resolved by the generator `tools/build/production-reports/gen.py::widths`, the same rule the Rust renderer should apply):

| Column (exact header) | Width at current column set | Align | Wrap | Treatment |
|---|---|---|---|---|
| `Day` | 78 pt (fixed) | left | no wrap normally | bold navy; `Off Day` slate regular; `Unscheduled` = warn chip |
| `Date` | 92 pt (fixed) | left | no wrap | regular ink |
| `Scenes` | 96 pt (flexible, weight 96) | left | wraps at `, ` | regular ink; shooting order (not sorted) |
| `Locations` | 178 pt (flexible, weight 178) | left | wraps | regular ink |
| `Est. time` | 76 pt (fixed) | left | no wrap | text; `Missing` = danger chip; off day = `—` |

Sum of widths = 520 pt (content width of portrait Letter minus 2 x 46 pt margins).

**Sorting:** none added (board order). The `Scenes` order is the strip order, i.e. shooting order — a deliberate property of this report that the stress file and K call out.

**Changes versus the current PDF, and why**
- **Off-day estimate:** when `Day` is `Off Day` and `Scenes` is empty, a `Missing` estimate prints as a muted `—` instead of the red chip. Why: an off day has no scenes; the current chip is a false alarm (p1, p2). A shoot day with `Missing` keeps the danger chip (word + square). Proper fix is in the builder (emit an empty string for days without strips) — listed in K.
- Off Day rows get a quiet treatment (panel fill, slate text) so shoot days read as the primary rows.
- `Unscheduled` row (not in the sample) = warn chip in `Day` + orange stripe.
- `Est. time` stays left-aligned (the source treats it as text).


## F. Visual specification

The shared shell (running header, masthead, context banners, metrics strip, table styling, status chip, orientation rule) is specified in `Report_Scene.md` **F.1 Shared report shell**; the values below are only what is specific to this report.

**Page:** Letter portrait; the sample is one page, so **no running header** (`--no-header` verified); the 4-page stress file has it.

**Report-specific:** `tr.quiet` for off days (fill `--of-panel`, text `--of-slate`, no zebra); `Missing` = danger chip with square mark; `Unscheduled` = warn chip (ring) + 3 pt orange stripe. Day label bold navy 8.4 pt.

**Everything else:** F.1.

## G. Dynamic behaviour

Shared rules are in `Report_Scene.md` F.1/G; the items below are this report's specifics.

- Day with no date: `No date` (source text).
- A day with 40 scenes: the Scenes cell wraps; the row grows.
- Several long locations: wrap in the 178 pt column (stress rows with two 120-char locations).
- `Est. time` `Missing` on a shoot day: danger chip.
- Unscheduled row: `Day = Unscheduled` chip, `Scenes = N scenes`, other cells empty (stress).
- Filter `coast` keeps rows where any cell contains it (day, date, scenes, locations, time).

**Always true for every report (see Scene G):** rows grow to fit wrapped text and are never ellipsised; a row is never split across pages; header row repeats on every page; an optional banner/note that is absent leaves no gap; an empty result prints the source's note in an info callout and no table; unknown/extra columns use the generic fallback widths and make the document landscape when `columns > 5`.

## H. Pagination and print rules

- Paper: US Letter, **portrait 612 x 792 pt** (`columns.len() = 5`; rule `> 5` -> landscape). Margins 46 pt left/right/top, 66 pt bottom (footer zone). Content width 520 pt.
- Table header repeats on every page (`thead` navy row). There is **no table title row**; the report's name on continuation pages is carried by the running header (see F.1 item 1).
- Rows `break-inside: avoid`. A row that does not fit moves whole to the next page. Minimum rows: never leave a single row alone on the last page when it can be avoided — move one more row with it (current output violates this, see K).
- Masthead, context banners and metrics strip are `break-after: avoid`: they stay with the first table rows; page 1 must show at least 3 table rows below the metrics, otherwise the table starts on a new page together with its header.
- One page for 16 rows; no header. Rows never split; the tallest sample row (`Shoot Day 12`, 4 location lines) is about 50 pt.
- Footer and numbering: shared system — official logo (17 pt high) + `Created using OpenFrame` bottom-left, `Page X of Y` bottom-right, on every page. The old footer text `<project> · <report title>` is **not** reproduced in the footer (brand rule); that identity moves to the running header.
- Running header: `@top-right`, `{PROJECT NAME} · SCHEDULE REPORT`, only when the report spans more than one page.

## I. HTML/PDF correspondence

- `Report_Schedule.html` is the printable document: one `<div class="doc">`, shared print CSS linked from `../../00_SYSTEM/assets/openframe-print.css`, shell + report CSS in one inline `<style>`, `of-preview.js` for the screen-only toolbar. Generated by `tools/build/production-reports/gen.py` from the real rows in `tools/build/production-reports/data/schedule.json` (extracted from the supplied baseline PDF by `extract_real_rows.py`); the HTML and the PDF therefore contain the same content.
- **PDF captured state:** live report, no filter, not a snapshot, as the user would export it on 22 Oct 2026 from `LOW TIDE RADIO: Blue Revision (Locked)`: 16 real rows, **1 page** (current output: 2). Rendered with `python tools/render_pdf.py Report_Schedule.html Report_Schedule.pdf` (Chrome 155 headless, `@page` margin boxes for the running header, logo footer and `Page X of Y`). `python tools/verify_pdf.py Report_Schedule.pdf --size letter --no-header` passes.
- **Preview toolbar toggles (`data-toggle`):**
  - `Filter active` — sample text `coast`. `Saved snapshot (illustrative)` — snapshot title `Schedule Report: LOW TIDE RADIO: Blue Revision (Locked)`.
- **Stress tests** (`stress-test/`; rows marked fictional are illustrative and are never in the primary PDF):
  - `Report_Schedule_STRESS_TEST` (4 pages): saved-snapshot identity with a 160-char title, a 200-char filter, a very long project and source label, extreme cell text, rows with missing fields, every status variant, then many fictional rows so the table runs several pages with repeated headers; the optional note slot is filled.
  - `Report_Schedule_STRESS_TEST_WIDE` (2 pages): 9 columns (the 5 real ones + 4 fictional: Unit, Call, Wrap, Notes) -> the **whole document is landscape** (rule `columns > 5`); shows the generic column rules.
  - `Report_Schedule_STRESS_TEST_EMPTY` (1 page): a filter that matches nothing -> metrics at 0 and the source's note `No matching data.` in an info callout; portrait.
  - The no-Production-Source state is identical for all six reports apart from the title and metric labels; it is shown once, in `Report_Scene_STRESS_TEST_NO_SOURCE`.
- **Known differences versus the Rust renderer:** Chrome lays out tables with CSS (`table-layout: fixed` with `<colgroup>` widths); the Rust renderer must measure with the same metrics it paints (existing rule) so wrap points can differ by a word. Letter-spacing, `box-shadow` stripes and the ring/dot shapes are drawn with rects/circles in the renderer (no CSS features are relied on beyond flat fills, hairlines and rounded rects). Chrome keeps a divider/row with its neighbour via `break-after/inside: avoid`; the Rust pager must implement the equivalent. The HTML status/filter toggles are preview-only JavaScript and recompute metrics and zebra from the visible rows; in the app those values come from `build_report`.

## J. Acceptance criteria

- [ ] Page size is Letter portrait; margins 46/46/46/66 pt; nothing drawn in the 66 pt footer band or outside the side margins (`tools/verify_pdf.py --size letter` passes).
- [ ] Footer on every page: logo lockup + `Created using OpenFrame` + `Page X of Y` with the true total.
- [ ] Masthead shows, in this order: eyebrow `PRODUCTION REPORT`; title; `Project: … · Source: …` (a part is omitted when blank); right panel `GENERATED` / date / scope (`All rows` | `Filtered rows` | `Saved snapshot`). There is no `Export reference` line and no second logo.
- [ ] Metrics strip labels and counts equal the source's metrics for this report (`Schedule rows`) and are recomputed from the rows actually printed (filtered rows -> filtered counts).
- [ ] Column headers are the exact `data.columns` strings, in the order given in section E; header row repeats on every page; header text never splits mid-word.
- [ ] No cell text is truncated or ellipsised; very long unbroken strings wrap by character inside their column; no cell overflows its column.
- [ ] Every status value is readable with colour removed: the word is always printed, and the chip carries its shape (disc / ring+dot / ring / square). `verify_pdf.py --gray` thumbnails reviewed.
- [ ] Zebra restarts correctly and body text is >= 8.4 pt; chips >= 6.6 pt; running header 6.8 pt.
- [ ] Empty result: no table, metrics show 0, info callout shows exactly the note text (`No matching data.` or `Choose a Production Source in Breakdown first.`), no running header, one page.
- [ ] Filter active: a filter banner prints `Filter` + the filter text (wraps if long) and the scope says `Filtered rows`. Saved snapshot: a snapshot banner prints, the title is the snapshot title, the date is the snapshot date; both banners can appear together.
- [ ] Orientation is decided once per document from the column count and never changes between pages.
- [ ] Off Day rows show `—` in `Est. time`, quiet fill, no red chip; shoot days with `Missing` show the danger chip with word and square.
- [ ] Scenes print in the order the source gives (not sorted).
- [ ] The sample prints on one page with no running header; a longer schedule prints the header on every page.
- [ ] An `Unscheduled` row prints the warn chip and the orange stripe.

## K. Known issues and unresolved questions

**Verified defects in the current output**
- `MISSING` chip on off days (p1, p2) — code: `"Est. time" if v == "Missing"` in `report_cell`; `estimated_minutes > 0` test in `build_report`.
- `Schedule rows` metric counts off days.
- Shell defects shared with the Scene Report.

**Design choices (made here, reversible)**
- `—` instead of the chip for off days (a presentation workaround).
- Quiet off-day rows.
- Running header omitted on one page.

**Unverified assumptions / open questions**
- That every `Off Day` has no strips (assumed from `Scenes` being empty; the design keys on `Day == "Off Day"` and an empty `Scenes` cell).
- Whether `Schedule rows` should be renamed (`Days`) — source label kept.
- The Unscheduled row has never been seen in a real export.
