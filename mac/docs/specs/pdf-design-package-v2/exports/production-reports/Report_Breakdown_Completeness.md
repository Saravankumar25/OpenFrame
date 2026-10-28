# Production Report — Breakdown Completeness  (`reports.export_report`, `reportType: "breakdown_completeness"`)

| | |
|---|---|
| Export | Production reports, op `reports.export_report` (PDF; CSV and XLSX are separate projections of the same `ReportData`) |
| Variant | **Report_Breakdown_Completeness** — portrait, 5 columns (status-first emphasis, open-scenes list) |
| Status | Design reference — not integrated; not validated against OpenFrame's Rust renderer |
| Repo commit | `2f5a045416bd2125801329533fc47f986ccc8174` (`main`) |
| Date | 2026-10-23 |
| Files | `exports/production-reports/Report_Breakdown_Completeness.html` / `.pdf` / `.md`; stress tests in `exports/production-reports/stress-test/` |
| Shared shell | Defined once in `Report_Scene.md`, section **F.1 Shared report shell**; the other five specs reference it and add their own column specs |


## A. Purpose and users

**Who:** the script supervisor and the breakdown lead / 1st AD, with the production coordinator tracking readiness before scheduling.

**When:** before locking the schedule and before each tech scout/prep meeting: "is the breakdown finished?".

**What they must find fast:** how many scenes are **Reviewed** versus **In progress / Needs review / Not started**, and — the question the current PDF cannot answer at a glance — **which scenes** those are. Status is a word with a shape (grayscale-safe), open scenes carry a left bar, and an optional list of open scene numbers sits under the metrics strip.

## B. Verified functionality

**Scope.** One production report, generated on demand from the current Production Source / current schedule, or a saved snapshot of one. Six live report types exist (`reportType`): `scene`, `location`, `cast_scene`, `prop`, `schedule`, `breakdown_completeness` (labels in the Reports tab: Scene Report, Location Report, Cast Scene Report, Prop Report, Schedule Report, Breakdown Completeness). This spec covers **Breakdown Completeness** (`breakdown_completeness`). Saved snapshots use the identical `ReportData` schema and print through the same layout (see "Saved snapshot").

**Inputs / args (verified, `ExportReportArgs` in `exports/reports.rs`, `deny_unknown_fields`, camelCase).**

| Arg key | Meaning |
|---|---|
| `format` | `"pdf"` \| `"csv"` \| `"xlsx"` (the UI offers PDF, CSV, Excel (XLSX) from `TABLE_FORMATS`) |
| `path` | destination chosen in the export dialog |
| `reportType` | live report type, optional; used when `reportId` is absent |
| `filter` | live text filter, optional (the Reports tab box "Filter rows (scene, location, name…)", max 200 chars, trimmed, blank = none) |
| `reportId` | a saved snapshot instead of a live report (`production_report` row, not deleted); when present it wins over `reportType`; neither present -> error "A report" required |

**Options.** None beyond format. The export dialog (`ReportExportButton` -> `ExportDialog`, title `Export Breakdown Completeness`, source line `Source: Breakdown Completeness` plus ` · filtered by “…”` when a filter is typed, or `Source: saved report <title>`) has no scope or checkbox options.

**Data dependency.** `reports.generate` -> `build_report` in `modules/schedule/docs.rs`. It needs a Production Source (taken from the current schedule's source, otherwise the active source) and reads non-omitted scenes (`!s.omitted`) and, for day information, the current schedule board.

**Filter semantics (verified).** Case-insensitive substring match: a row is kept when **any cell** contains the text (`report_filter`, `rows.filter(... any(cell.to_lowercase().contains(..)))`).

**Empty / error behaviour (verified).**
- No source selected: `columns = []`, `rows = []`, `note = "Choose a Production Source in Breakdown first."` (`NO_SOURCE_MESSAGE`), `sourceLabel = ""`. The PDF prints masthead (no `Source:` part), zero-value metrics and an info callout with that sentence; portrait (0 columns).
- Source but no rows (e.g. filter matches nothing): columns are still returned, `note = "No matching data."`; the PDF prints masthead, zero metrics and an info callout with the note; **orientation follows the column count, so an empty Breakdown Completeness is portrait**.
- Unknown `reportType`: validation error "Choose one of the available reports." Filter longer than 200 characters: validation error "Keep the filter under 200 characters." Export needs the Export capability (`require_export`).
- A report row cell that the PDF font cannot draw: export is refused with `HINT_TABLES` ("Export it as CSV or Excel (XLSX) to keep them, or replace those characters and export the PDF again.") — behaviour inherited from the shared export pipeline.
**Breakdown Completeness specifics.** Columns (5): `Sc`, `Heading`, `Confirmed`, `Suggested (unreviewed)`, `Status`. One row per non-omitted scene in scene order. `Confirmed` / `Suggested (unreviewed)` = `src.breakdown_counts(scene)` (confirmed elements / suggested elements not yet reviewed). `Status`: (0,0) -> `Not started`; (n,0) -> `Reviewed`; (0,n) -> `Needs review`; (n,n) -> `In progress`. Metrics (exact): `Scenes` = rows, `Reviewed` = rows with `Reviewed`, `In progress or needs review` = rows with `In progress` or `Needs review`, `Not started` = rows with `Not started`.

**Saved snapshot.** `reports.save` stores `ReportData` JSON in `production_report` (`title` defaults to `"Breakdown Completeness: <source label>"`, user-editable up to 160 chars). Exporting with `reportId` prints that stored data: the masthead title is the **snapshot title**, the generated date is the snapshot's, the scope reads "Saved snapshot", and the stored `filter` (if any) is available to print. Rows never change after saving.

### B.1 Current-output assessment

Baseline: `Production Report — Breakdown Completeness.pdf` (3 pages, portrait, 22 Oct 2026, project Low Tide Radio, source `LOW TIDE RADIO: Blue Revision (Locked)`).

**Retain**
- Columns, labels, scene order; scene tile; right-aligned counts; status chips with words; the four metrics (p1-p3).

**Improve visually**
- Four open scenes (23, 24, 37, 40) are only discoverable by scanning three pages for a blue chip (p2-p3); now they carry an orange bar, and an optional line under the metrics lists them.
- `Confirmed` and `Suggested` use 96 pt columns for 1-2 digit numbers while headings wrap at 192 pt (p1: scene 2, 3, 10, 11, 13, 14); Heading is 222 pt, counts 60/90 pt.
- Suggested count `0` is now muted and a non-zero count amber, so the column highlights what needs review.
- Page 1 shows 15 rows in the current PDF (p1) and 22 in the redesign; 2 pages instead of 3.

**Verified functional / layout problems (current output)**
- `Not started` has no tone in `status_tone` (falls to `Neutral`), so the scenes needing the most attention would print as a plain grey chip; the sample has 0 such rows, so p1-p3 cannot show it.
- The metric `In progress or needs review` merges two different states and names no scenes (p1).
- Shell defects shared with the Scene Report.

**Cannot be determined from the evidence**
- How a `Needs review` or `Not started` row looks in the real export — the sample has only `Reviewed` (40) and `In progress` (4).
- Whether suggested-element counts include rejected suggestions (`breakdown_counts` not inspected in detail).


## C. Source mapping

| Item | Verified identifier |
|---|---|
| Commit | `2f5a045416bd2125801329533fc47f986ccc8174` |
| UI entry point | `windows/apps/desktop/src/workspaces/production/tabs/reports.tsx`, component `ReportsTab` (tab id `reports`, label "Reports"); the list entry "Breakdown Completeness" in the left `vpanel` listbox; export button = `ReportExportButton` (from `windows/apps/desktop/src/features/export/buttons.tsx`) placed in the `PageHeader` actions, next to "Save snapshot". |
| Op name | `reports.export_report` (registered in `exports/mod.rs`, `r.query("reports.export_report", reports::export)`, capability Export) |
| Builder | `windows/crates/openframe-application/src/modules/exports/reports.rs` -> `export()` (+ helpers `report_col`, `report_cell`, `status_tone`, `is_numeric_column`) |
| Data | `reports.generate` / `build_report` / `struct ReportData { report_type, title, source_label, generated_at, columns, rows, note, filter }` in `modules/schedule/docs.rs`; saved: `reports.save`, `reports.get`, table `production_report` (`title`, `snapshot_json`) |
| Builder fn for this report's rows | `build_report` default arm (`_`) in `modules/schedule/docs.rs` (`src.breakdown_counts(&sc.id)`; status table `(0,0)/(_,0)/(0,_)/_`) with `status_tone` and `report_cell` (`Status` chips) in `exports/reports.rs`; metrics in `export()` (`count_where`) |
| Renderer blocks used (current) | `Document::new(..).landscape(columns > 5).footer(footer(project, data.title))`; `Block::Masthead` (via `production_masthead`, `title_size = 26`); `Block::Metrics(compact)` of `Metric`s; `Block::Styled(StyledTable{ header: HeaderStyle::Navy, boxed, zebra, continued: Some(data.title) })` with `Row`/`Cell`; cell items `CellItem::Tile` and `CellItem::Chips(Chip::label(.., Tone))`; `Block::Callout{Tone::Info}` for the empty state; `Block::Text` (8.4 pt slate) for `data.note` under the table; plain `Table` pushed to `Output.tables` for CSV/XLSX. |
| Design needs from the renderer (new) | running header (single line, top-right, multi-page only); compact masthead; context banners (snapshot / filter); fit-content metrics strip; status chip with leading shape (disc / ring+dot / ring / square); left stripe on attention rows; two-line "day / date" cell; stacked days list cell; letter-divider rows (Prop only); footer logo PNG embedded (current renderer redraws a vector mark + text — implementation requirement from the shared brief). |


## D. Field inventory

Shared shell rows first, then this report's table. The open-scenes list is a design addition derived from `Status`.

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
| Metrics `Scenes`, `Reviewed`, `In progress or needs review`, `Not started` | counts of `Status` over `data.rows` (see B) | derived | 0; value amber when the third is > 0, red when the fourth is > 0 |
| Open-scenes list (banner `SCENES STILL OPEN`) | scene numbers whose `Status` is `In progress`/`Needs review` (+ separate `Not started` list) | derived (design addition) | Omitted when none are open |
| `Sc` (tile) | `sc.number` | required | — |
| `Heading` | `sc.heading` | required | Empty cell |
| `Confirmed` | confirmed element count | required | `0` muted |
| `Suggested (unreviewed)` | suggested element count | required | `0` muted; non-zero amber |
| `Status` chip | derived word (see B) | required (derived) | — |

## E. Content architecture

**Order and hierarchy.** Shell -> metrics strip -> (optional) open-scenes banner -> table in scene order. Row = tile + bold heading | confirmed | suggested | status chip. Rows that are not `Reviewed` get a left bar.

**Table columns** (design order shown; widths resolved by the generator `tools/build/production-reports/gen.py::widths`, the same rule the Rust renderer should apply):

| Column (exact header) | Width at current column set | Align | Wrap | Treatment |
|---|---|---|---|---|
| `Sc` | 36 pt (fixed) | left | no wrap | tile |
| `Heading` | 222 pt (flexible, weight 222) | left | wraps | bold navy |
| `Confirmed` | 60 pt (fixed) | right | header one line | bold navy; `0` muted |
| `Suggested (unreviewed)` | 90 pt (fixed) | right | header wraps to 2 lines: `SUGGESTED` / `(UNREVIEWED)` | bold amber when > 0; `0` muted regular |
| `Status` | 112 pt (fixed) | left | no wrap (chip) | chip: word + mark; `Reviewed` disc/ok, `In progress` ring+dot/info, `Needs review` ring/warn, `Not started` square/danger |

Sum of widths = 520 pt (content width of portrait Letter minus 2 x 46 pt margins).

**Sorting:** none added (scene order). Grouping by status was considered and rejected: scene order is how the script supervisor reads it; the open-scenes list gives the grouped view.

**Changes versus the current PDF, and why**
- `Not started` tone: neutral -> **danger** (the source's `status_tone` has no entry for it). Why: it is the most urgent state.
- Open-scenes banner under the metrics (derived from `Status`; optional toggle).
- Orange (red for Not started) left bar on open rows; zero counts muted; non-zero suggested amber.
- Widths: Sc 36, Heading 222, Confirmed 60, Suggested 90, Status 112.


## F. Visual specification

The shared shell (running header, masthead, context banners, metrics strip, table styling, status chip, orientation rule) is specified in `Report_Scene.md` **F.1 Shared report shell**; the values below are only what is specific to this report.

**Page:** Letter portrait; 2 pages, running header `LOW TIDE RADIO · BREAKDOWN COMPLETENESS`.

**Status marks (grayscale-safe):** Reviewed = solid disc (green), In progress = ring with centre dot (blue), Needs review = hollow ring (amber), Not started = solid square (red) — word always printed. Open rows: 3 pt left bar (orange; red for Not started). Metric values coloured amber/red only when non-zero, the number and label always print.

**Open-scenes banner:** shared `.banner.warn` (amber fill, orange 3 pt stripe), label `SCENES STILL OPEN`, text `In progress or needs review: 23, 24, 37, 40` (+ ` · Not started: …` when any); 4 pt below the metrics strip.

**Everything else:** F.1.

## G. Dynamic behaviour

Shared rules are in `Report_Scene.md` F.1/G; the items below are this report's specifics.

- All reviewed: no bars, banner omitted, the 3rd and 4th metrics read 0.
- 500 scenes: banner text wraps over several lines (a list of up to ~60 numbers per line); if the list becomes impractical (> 40 numbers) the renderer may truncate to `Scenes still open: 73 scenes` — a design option not implemented here (K).
- Heading long: wraps in 222 pt.
- Filter keeps the metric counts consistent with the printed rows (filtering `coast` changes `Reviewed`, etc.).
- `Needs review` (0 confirmed, N suggested) shows `0` muted and `N` amber, ring chip, bar.

**Always true for every report (see Scene G):** rows grow to fit wrapped text and are never ellipsised; a row is never split across pages; header row repeats on every page; an optional banner/note that is absent leaves no gap; an empty result prints the source's note in an info callout and no table; unknown/extra columns use the generic fallback widths and make the document landscape when `columns > 5`.

## H. Pagination and print rules

- Paper: US Letter, **portrait 612 x 792 pt** (`columns.len() = 5`; rule `> 5` -> landscape). Margins 46 pt left/right/top, 66 pt bottom (footer zone). Content width 520 pt.
- Table header repeats on every page (`thead` navy row). There is **no table title row**; the report's name on continuation pages is carried by the running header (see F.1 item 1).
- Rows `break-inside: avoid`. A row that does not fit moves whole to the next page. Minimum rows: never leave a single row alone on the last page when it can be avoided — move one more row with it (current output violates this, see K).
- Masthead, context banners and metrics strip are `break-after: avoid`: they stay with the first table rows; page 1 must show at least 3 table rows below the metrics, otherwise the table starts on a new page together with its header.
- Header repeats; the banner and metrics stay with the first rows. 44 rows over 2 pages; no row is split.
- Footer and numbering: shared system — official logo (17 pt high) + `Created using OpenFrame` bottom-left, `Page X of Y` bottom-right, on every page. The old footer text `<project> · <report title>` is **not** reproduced in the footer (brand rule); that identity moves to the running header.
- Running header: `@top-right`, `{PROJECT NAME} · BREAKDOWN COMPLETENESS`, only when the report spans more than one page.

## I. HTML/PDF correspondence

- `Report_Breakdown_Completeness.html` is the printable document: one `<div class="doc">`, shared print CSS linked from `../../00_SYSTEM/assets/openframe-print.css`, shell + report CSS in one inline `<style>`, `of-preview.js` for the screen-only toolbar. Generated by `tools/build/production-reports/gen.py` from the real rows in `tools/build/production-reports/data/breakdown_completeness.json` (extracted from the supplied baseline PDF by `extract_real_rows.py`); the HTML and the PDF therefore contain the same content.
- **PDF captured state:** live report, no filter, not a snapshot, as the user would export it on 22 Oct 2026 from `LOW TIDE RADIO: Blue Revision (Locked)`: 44 real rows, **2 pages** (current output: 3). Rendered with `python tools/render_pdf.py Report_Breakdown_Completeness.html Report_Breakdown_Completeness.pdf` (Chrome 155 headless, `@page` margin boxes for the running header, logo footer and `Page X of Y`). `python tools/verify_pdf.py Report_Breakdown_Completeness.pdf --size letter --header "LOW TIDE RADIO · BREAKDOWN COMPLETENESS"` passes.
- **Preview toolbar toggles (`data-toggle`):**
  - `Open-scenes list` (on) — the banner under the metrics. `Filter active` — sample text `coast`; metrics recompute. `Saved snapshot (illustrative)` — snapshot title `Breakdown Completeness: LOW TIDE RADIO: Blue Revision (Locked)`.
- **Stress tests** (`stress-test/`; rows marked fictional are illustrative and are never in the primary PDF):
  - `Report_Breakdown_Completeness_STRESS_TEST` (5 pages): saved-snapshot identity with a 160-char title, a 200-char filter, a very long project and source label, extreme cell text, rows with missing fields, every status variant, then many fictional rows so the table runs several pages with repeated headers; the optional note slot is filled.
  - `Report_Breakdown_Completeness_STRESS_TEST_WIDE` (2 pages): 8 columns (the 5 real ones + 3 fictional: Cast, Props, Notes) -> the **whole document is landscape** (rule `columns > 5`); shows the generic column rules.
  - `Report_Breakdown_Completeness_STRESS_TEST_EMPTY` (1 page): a filter that matches nothing -> metrics at 0 and the source's note `No matching data.` in an info callout; portrait.
  - The no-Production-Source state is identical for all six reports apart from the title and metric labels; it is shown once, in `Report_Scene_STRESS_TEST_NO_SOURCE`.
- **Known differences versus the Rust renderer:** Chrome lays out tables with CSS (`table-layout: fixed` with `<colgroup>` widths); the Rust renderer must measure with the same metrics it paints (existing rule) so wrap points can differ by a word. Letter-spacing, `box-shadow` stripes and the ring/dot shapes are drawn with rects/circles in the renderer (no CSS features are relied on beyond flat fills, hairlines and rounded rects). Chrome keeps a divider/row with its neighbour via `break-after/inside: avoid`; the Rust pager must implement the equivalent. The HTML status/filter toggles are preview-only JavaScript and recompute metrics and zebra from the visible rows; in the app those values come from `build_report`.

## J. Acceptance criteria

- [ ] Page size is Letter portrait; margins 46/46/46/66 pt; nothing drawn in the 66 pt footer band or outside the side margins (`tools/verify_pdf.py --size letter` passes).
- [ ] Footer on every page: logo lockup + `Created using OpenFrame` + `Page X of Y` with the true total.
- [ ] Masthead shows, in this order: eyebrow `PRODUCTION REPORT`; title; `Project: … · Source: …` (a part is omitted when blank); right panel `GENERATED` / date / scope (`All rows` | `Filtered rows` | `Saved snapshot`). There is no `Export reference` line and no second logo.
- [ ] Metrics strip labels and counts equal the source's metrics for this report (`Scenes`, `Reviewed`, `In progress or needs review`, `Not started`) and are recomputed from the rows actually printed (filtered rows -> filtered counts).
- [ ] Column headers are the exact `data.columns` strings, in the order given in section E; header row repeats on every page; header text never splits mid-word.
- [ ] No cell text is truncated or ellipsised; very long unbroken strings wrap by character inside their column; no cell overflows its column.
- [ ] Every status value is readable with colour removed: the word is always printed, and the chip carries its shape (disc / ring+dot / ring / square). `verify_pdf.py --gray` thumbnails reviewed.
- [ ] Zebra restarts correctly and body text is >= 8.4 pt; chips >= 6.6 pt; running header 6.8 pt.
- [ ] Empty result: no table, metrics show 0, info callout shows exactly the note text (`No matching data.` or `Choose a Production Source in Breakdown first.`), no running header, one page.
- [ ] Filter active: a filter banner prints `Filter` + the filter text (wraps if long) and the scope says `Filtered rows`. Saved snapshot: a snapshot banner prints, the title is the snapshot title, the date is the snapshot date; both banners can appear together.
- [ ] Orientation is decided once per document from the column count and never changes between pages.
- [ ] Four metrics with the exact labels; the sum Reviewed + open + Not started = Scenes.
- [ ] Every non-Reviewed row has the left bar and a chip with its word and mark; Reviewed rows have none.
- [ ] In the grayscale thumbnails `Reviewed`, `In progress`, `Needs review` and `Not started` are distinguishable by word and by mark.
- [ ] The open-scenes banner lists exactly the scenes whose status is In progress or Needs review (23, 24, 37, 40 in the sample).

## K. Known issues and unresolved questions

**Verified defects in the current output**
- `Not started` falls to `Neutral` in `status_tone` (code).
- Open scenes are not discoverable except by scanning (p2-p3).
- Shell defects shared with the Scene Report.

**Design choices (made here, reversible)**
- `Not started` = danger.
- Open-scenes banner (derived).
- Left bars and amber suggested counts.
- Scene order kept (no grouping).

**Unverified assumptions / open questions**
- Real appearance of `Needs review`/`Not started` rows (stress file only, fictional).
- Behaviour for a very long open-scenes list.
- Whether the app should emit a per-state breakdown of `In progress or needs review` (metrics merge them).
