# Production Report — Prop Report  (`reports.export_report`, `reportType: "prop"`)

| | |
|---|---|
| Export | Production reports, op `reports.export_report` (PDF; CSV and XLSX are separate projections of the same `ReportData`) |
| Variant | **Report_Prop** — portrait, 3 columns (alphabetical letter dividers; dense rows) |
| Status | Design reference — not integrated; not validated against OpenFrame's Rust renderer |
| Repo commit | `2f5a045416bd2125801329533fc47f986ccc8174` (`main`) |
| Date | 2026-10-23 |
| Files | `exports/production-reports/Report_Prop.html` / `.pdf` / `.md`; stress tests in `exports/production-reports/stress-test/` |
| Shared shell | Defined once in `Report_Scene.md`, section **F.1 Shared report shell**; the other five specs reference it and add their own column specs |


## A. Purpose and users

**Who:** the props master and art department coordinator, set decorator, and the 1st AD for prop-heavy scenes; script supervisor for continuity.

**When:** during breakdown review and prop sourcing/hire planning.

**What they must find fast:** a prop by name (alphabetical), **in which scenes it appears** and **how many scenes use it** (hero props with many scenes need tracking: `Nagra tape recorder` 7, `Coir string of station keys` 7, `Mariamma's wristwatch` 8, `Quarter-inch reel 14-11-1999` 8). 148 rows must scan fast, so rows are one line when possible and alphabetical letters divide the list.

## B. Verified functionality

**Scope.** One production report, generated on demand from the current Production Source / current schedule, or a saved snapshot of one. Six live report types exist (`reportType`): `scene`, `location`, `cast_scene`, `prop`, `schedule`, `breakdown_completeness` (labels in the Reports tab: Scene Report, Location Report, Cast Scene Report, Prop Report, Schedule Report, Breakdown Completeness). This spec covers **Prop Report** (`prop`). Saved snapshots use the identical `ReportData` schema and print through the same layout (see "Saved snapshot").

**Inputs / args (verified, `ExportReportArgs` in `exports/reports.rs`, `deny_unknown_fields`, camelCase).**

| Arg key | Meaning |
|---|---|
| `format` | `"pdf"` \| `"csv"` \| `"xlsx"` (the UI offers PDF, CSV, Excel (XLSX) from `TABLE_FORMATS`) |
| `path` | destination chosen in the export dialog |
| `reportType` | live report type, optional; used when `reportId` is absent |
| `filter` | live text filter, optional (the Reports tab box "Filter rows (scene, location, name…)", max 200 chars, trimmed, blank = none) |
| `reportId` | a saved snapshot instead of a live report (`production_report` row, not deleted); when present it wins over `reportType`; neither present -> error "A report" required |

**Options.** None beyond format. The export dialog (`ReportExportButton` -> `ExportDialog`, title `Export Prop Report`, source line `Source: Prop Report` plus ` · filtered by “…”` when a filter is typed, or `Source: saved report <title>`) has no scope or checkbox options.

**Data dependency.** `reports.generate` -> `build_report` in `modules/schedule/docs.rs`. It needs a Production Source (taken from the current schedule's source, otherwise the active source) and reads non-omitted scenes (`!s.omitted`) and, for day information, the current schedule board.

**Filter semantics (verified).** Case-insensitive substring match: a row is kept when **any cell** contains the text (`report_filter`, `rows.filter(... any(cell.to_lowercase().contains(..)))`).

**Empty / error behaviour (verified).**
- No source selected: `columns = []`, `rows = []`, `note = "Choose a Production Source in Breakdown first."` (`NO_SOURCE_MESSAGE`), `sourceLabel = ""`. The PDF prints masthead (no `Source:` part), zero-value metrics and an info callout with that sentence; portrait (0 columns).
- Source but no rows (e.g. filter matches nothing): columns are still returned, `note = "No matching data."`; the PDF prints masthead, zero metrics and an info callout with the note; **orientation follows the column count, so an empty Prop Report is portrait**.
- Unknown `reportType`: validation error "Choose one of the available reports." Filter longer than 200 characters: validation error "Keep the filter under 200 characters." Export needs the Export capability (`require_export`).
- A report row cell that the PDF font cannot draw: export is refused with `HINT_TABLES` ("Export it as CSV or Excel (XLSX) to keep them, or replace those characters and export the PDF again.") — behaviour inherited from the shared export pipeline.
**Prop Report specifics.** Columns (3): `Prop`, `Scenes`, `Scene count`. Only breakdown elements in category `Props` (`i.category == "Props"`) of non-omitted scenes; one row per distinct name (case-insensitive), alphabetical by lowercase name; `Scenes` = scene numbers `, `-joined; `Scene count` = number of scenes. Metric: `Props` = row count (148 in the sample).

**Saved snapshot.** `reports.save` stores `ReportData` JSON in `production_report` (`title` defaults to `"Prop Report: <source label>"`, user-editable up to 160 chars). Exporting with `reportId` prints that stored data: the masthead title is the **snapshot title**, the generated date is the snapshot's, the scope reads "Saved snapshot", and the stored `filter` (if any) is available to print. Rows never change after saving.

### B.1 Current-output assessment

Baseline: `Production Report — Prop.pdf` (7 pages, portrait, 22 Oct 2026, project Low Tide Radio, source `LOW TIDE RADIO: Blue Revision (Locked)`).

**Retain**
- Three columns, labels, alphabetical order, right-aligned bold `Scene count`, metric `Props` = 148 (p1-p7).

**Improve visually**
- Column weights 1.2 / 1.5 / 1.2 give Prop 160 pt, Scenes 200 pt and Scene count 160 pt, so prop names wrap at 160 pt while `Scene count` (a one-digit number) takes nearly a third of the page (p2: `Edit suite monitors and audio / workstation`; p3: `Fishing-net kite with copper wire / antenna`, `Framed station photograph (turned to / wall)`); Prop is now 250 pt and `Scene count` 70 pt.
- 7 pages for a three-column list; rows are tightened (padding 2.5 pt) and the masthead is compact: 5 pages with letter dividers.
- No way to see where `D` starts in a 148-row list; letter dividers added.

**Verified functional / layout problems (current output)**
- The last page holds two rows (`Wireless set`, `Wooden toothpick`) (p7).
- A 160 pt Prop column wraps long prop names unnecessarily (p2, p3, p5, p6 — e.g. `Twin station microphones and / handsets`, `Walkman microphone on wooden / spoon`).
- Shell defects shared with the Scene Report.

**Cannot be determined from the evidence**
- Whether prop names with different spelling but the same prop (`Biscuit box (reel case)` vs `(reel container)`) should merge — out of scope, data issue.
- Whether props master wants Cast/Wardrobe/Vehicle categories — the source emits only `Props`.


## C. Source mapping

| Item | Verified identifier |
|---|---|
| Commit | `2f5a045416bd2125801329533fc47f986ccc8174` |
| UI entry point | `windows/apps/desktop/src/workspaces/production/tabs/reports.tsx`, component `ReportsTab` (tab id `reports`, label "Reports"); the list entry "Prop Report" in the left `vpanel` listbox; export button = `ReportExportButton` (from `windows/apps/desktop/src/features/export/buttons.tsx`) placed in the `PageHeader` actions, next to "Save snapshot". |
| Op name | `reports.export_report` (registered in `exports/mod.rs`, `r.query("reports.export_report", reports::export)`, capability Export) |
| Builder | `windows/crates/openframe-application/src/modules/exports/reports.rs` -> `export()` (+ helpers `report_col`, `report_cell`, `status_tone`, `is_numeric_column`) |
| Data | `reports.generate` / `build_report` / `struct ReportData { report_type, title, source_label, generated_at, columns, rows, note, filter }` in `modules/schedule/docs.rs`; saved: `reports.save`, `reports.get`, table `production_report` (`title`, `snapshot_json`) |
| Builder fn for this report's rows | `build_report` arm `"prop"` in `modules/schedule/docs.rs` (`src.items(&sc.id)` filtered by `category == "Props"`, `BTreeMap` by lowercase name) with `report_col`/`report_cell` (`Scene count` numeric -> right-aligned) in `exports/reports.rs` |
| Renderer blocks used (current) | `Document::new(..).landscape(columns > 5).footer(footer(project, data.title))`; `Block::Masthead` (via `production_masthead`, `title_size = 26`); `Block::Metrics(compact)` of `Metric`s; `Block::Styled(StyledTable{ header: HeaderStyle::Navy, boxed, zebra, continued: Some(data.title) })` with `Row`/`Cell`; cell items `CellItem::Tile` and `CellItem::Chips(Chip::label(.., Tone))`; `Block::Callout{Tone::Info}` for the empty state; `Block::Text` (8.4 pt slate) for `data.note` under the table; plain `Table` pushed to `Output.tables` for CSV/XLSX. |
| Design needs from the renderer (new) | running header (single line, top-right, multi-page only); compact masthead; context banners (snapshot / filter); fit-content metrics strip; status chip with leading shape (disc / ring+dot / ring / square); left stripe on attention rows; two-line "day / date" cell; stacked days list cell; letter-divider rows (Prop only); footer logo PNG embedded (current renderer redraws a vector mark + text — implementation requirement from the shared brief). |


## D. Field inventory

Shared shell rows first, then this report's table. Letter divider rows are a design addition derived from the first letter of the sorted names.

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
| Metric `Props` | `rows.len()` | derived | 0 |
| Letter divider (A, B, C … or `#`) | first character of `Prop`, upper-cased; `#` for a non-letter | derived (design addition) | —; hidden when a letter has no visible row |
| `Prop` | `it.name` (display name as first seen) | required | — |
| `Scenes` | scene numbers joined `, ` | required | — |
| `Scene count` | number of scenes | required | — |

## E. Content architecture

**Order and hierarchy.** Shell, then one table, alphabetical, with a thin letter divider row before each new first letter. Row = bold prop name | scene numbers | count.

**Table columns** (design order shown; widths resolved by the generator `tools/build/production-reports/gen.py::widths`, the same rule the Rust renderer should apply):

| Column (exact header) | Width at current column set | Align | Wrap | Treatment |
|---|---|---|---|---|
| `Prop` | 250 pt (flexible, weight 250) | left | wraps | bold navy |
| `Scenes` | 200 pt (flexible, weight 200) | left | wraps at `, ` | regular ink |
| `Scene count` | 70 pt (fixed) | right | no wrap; header `SCENE COUNT` on one line at 70 pt | bold navy |

Sum of widths = 520 pt (content width of portrait Letter minus 2 x 46 pt margins).

**Sorting:** unchanged (alphabetical, case-insensitive).

**Changes versus the current PDF, and why**
- Letter divider rows (6.8 pt bold caps on `--of-tint`, padding 2/6 pt) — optional (`data-toggle`), default on. Why: 148 rows; dividers cost about 20 x 15 pt and make alphabetic jumps fast.
- Dense rows (padding 2.5 pt) and widths 250 / 200 / 70 pt instead of 160 / 200 / 160.
- Zebra restarts after each divider.


## F. Visual specification

The shared shell (running header, masthead, context banners, metrics strip, table styling, status chip, orientation rule) is specified in `Report_Scene.md` **F.1 Shared report shell**; the values below are only what is specific to this report.

**Page:** Letter portrait, content 520 x 680 pt, running header `LOW TIDE RADIO · PROP REPORT` (5 pages).

**Report-specific:** table class `dense`: body padding 2.5/6 pt, single-line row about 15.9 pt. A letter divider is a table row spanning all columns (`tr.group`: 6.8 pt bold caps tracking 0.12 em navy on `--of-tint`, padding 2/6 pt) kept with the first row after it (`break-after: avoid`). Letters printed on a page's first row are not repeated: a letter group continuing over a page break shows no divider on the new page (the running header and alphabetical order carry the context). No chips, tiles or status marks in this report.

**Everything else:** F.1.

## G. Dynamic behaviour

Shared rules are in `Report_Scene.md` F.1/G; the items below are this report's specifics.

- 148 rows: 5 pages in the sample; the stress file (171 rows including 3 extreme ones) runs 6 pages.
- A prop used in 60 scenes (stress): the `Scenes` list wraps inside 200 pt and grows the row.
- A prop name of 130+ characters wraps; a long unbroken name wraps by character.
- A name starting with a digit or symbol goes under `#`.
- Letter divider with no visible row (filter active) disappears.

**Always true for every report (see Scene G):** rows grow to fit wrapped text and are never ellipsised; a row is never split across pages; header row repeats on every page; an optional banner/note that is absent leaves no gap; an empty result prints the source's note in an info callout and no table; unknown/extra columns use the generic fallback widths and make the document landscape when `columns > 5`.

## H. Pagination and print rules

- Paper: US Letter, **portrait 612 x 792 pt** (`columns.len() = 3`; rule `> 5` -> landscape). Margins 46 pt left/right/top, 66 pt bottom (footer zone). Content width 520 pt.
- Table header repeats on every page (`thead` navy row). There is **no table title row**; the report's name on continuation pages is carried by the running header (see F.1 item 1).
- Rows `break-inside: avoid`. A row that does not fit moves whole to the next page. Minimum rows: never leave a single row alone on the last page when it can be avoided — move one more row with it (current output violates this, see K).
- Masthead, context banners and metrics strip are `break-after: avoid`: they stay with the first table rows; page 1 must show at least 3 table rows below the metrics, otherwise the table starts on a new page together with its header.
- A divider is never the last row on a page.
- Dense rows: about 36 rows per page, 5 pages for 148 rows + 20 dividers.
- Footer and numbering: shared system — official logo (17 pt high) + `Created using OpenFrame` bottom-left, `Page X of Y` bottom-right, on every page. The old footer text `<project> · <report title>` is **not** reproduced in the footer (brand rule); that identity moves to the running header.
- Running header: `@top-right`, `{PROJECT NAME} · PROP REPORT`, only when the report spans more than one page.

## I. HTML/PDF correspondence

- `Report_Prop.html` is the printable document: one `<div class="doc">`, shared print CSS linked from `../../00_SYSTEM/assets/openframe-print.css`, shell + report CSS in one inline `<style>`, `of-preview.js` for the screen-only toolbar. Generated by `tools/build/production-reports/gen.py` from the real rows in `tools/build/production-reports/data/prop.json` (extracted from the supplied baseline PDF by `extract_real_rows.py`); the HTML and the PDF therefore contain the same content.
- **PDF captured state:** live report, no filter, not a snapshot, as the user would export it on 22 Oct 2026 from `LOW TIDE RADIO: Blue Revision (Locked)`: 148 real rows, **5 pages** (current output: 7). Rendered with `python tools/render_pdf.py Report_Prop.html Report_Prop.pdf` (Chrome 155 headless, `@page` margin boxes for the running header, logo footer and `Page X of Y`). `python tools/verify_pdf.py Report_Prop.pdf --size letter --header "LOW TIDE RADIO · PROP REPORT"` passes.
- **Preview toolbar toggles (`data-toggle`):**
  - `Letter dividers` (on by default) — hides/shows the divider rows.
  - `Filter active` — sample text `tea`; filters rows and removes empty letter groups. `Saved snapshot (illustrative)` — snapshot title `Prop Report: LOW TIDE RADIO: Blue Revision (Locked)`.
- **Stress tests** (`stress-test/`; rows marked fictional are illustrative and are never in the primary PDF):
  - `Report_Prop_STRESS_TEST` (6 pages): saved-snapshot identity with a 160-char title, a 200-char filter, a very long project and source label, extreme cell text, rows with missing fields, every status variant, then many fictional rows so the table runs several pages with repeated headers; the optional note slot is filled.
  - `Report_Prop_STRESS_TEST_WIDE` (2 pages): 7 columns (the 3 real ones + 4 fictional: Department, Source, Status, Notes) -> the **whole document is landscape** (rule `columns > 5`); shows the generic column rules.
  - `Report_Prop_STRESS_TEST_EMPTY` (1 page): a filter that matches nothing -> metrics at 0 and the source's note `No matching data.` in an info callout; portrait.
  - The no-Production-Source state is identical for all six reports apart from the title and metric labels; it is shown once, in `Report_Scene_STRESS_TEST_NO_SOURCE`.
- **Known differences versus the Rust renderer:** Chrome lays out tables with CSS (`table-layout: fixed` with `<colgroup>` widths); the Rust renderer must measure with the same metrics it paints (existing rule) so wrap points can differ by a word. Letter-spacing, `box-shadow` stripes and the ring/dot shapes are drawn with rects/circles in the renderer (no CSS features are relied on beyond flat fills, hairlines and rounded rects). Chrome keeps a divider/row with its neighbour via `break-after/inside: avoid`; the Rust pager must implement the equivalent. The HTML status/filter toggles are preview-only JavaScript and recompute metrics and zebra from the visible rows; in the app those values come from `build_report`.

## J. Acceptance criteria

- [ ] Page size is Letter portrait; margins 46/46/46/66 pt; nothing drawn in the 66 pt footer band or outside the side margins (`tools/verify_pdf.py --size letter` passes).
- [ ] Footer on every page: logo lockup + `Created using OpenFrame` + `Page X of Y` with the true total.
- [ ] Masthead shows, in this order: eyebrow `PRODUCTION REPORT`; title; `Project: … · Source: …` (a part is omitted when blank); right panel `GENERATED` / date / scope (`All rows` | `Filtered rows` | `Saved snapshot`). There is no `Export reference` line and no second logo.
- [ ] Metrics strip labels and counts equal the source's metrics for this report (`Props`) and are recomputed from the rows actually printed (filtered rows -> filtered counts).
- [ ] Column headers are the exact `data.columns` strings, in the order given in section E; header row repeats on every page; header text never splits mid-word.
- [ ] No cell text is truncated or ellipsised; very long unbroken strings wrap by character inside their column; no cell overflows its column.
- [ ] Every status value is readable with colour removed: the word is always printed, and the chip carries its shape (disc / ring+dot / ring / square). `verify_pdf.py --gray` thumbnails reviewed.
- [ ] Zebra restarts correctly and body text is >= 8.4 pt; chips >= 6.6 pt; running header 6.8 pt.
- [ ] Empty result: no table, metrics show 0, info callout shows exactly the note text (`No matching data.` or `Choose a Production Source in Breakdown first.`), no running header, one page.
- [ ] Filter active: a filter banner prints `Filter` + the filter text (wraps if long) and the scope says `Filtered rows`. Saved snapshot: a snapshot banner prints, the title is the snapshot title, the date is the snapshot date; both banners can appear together.
- [ ] Orientation is decided once per document from the column count and never changes between pages.
- [ ] Rows are alphabetical (case-insensitive); each new first letter has a divider row; no divider is the last item on a page.
- [ ] `Scene count` is right-aligned and narrow (70 pt); the Prop column is wide enough that names under 45 characters stay on one line.
- [ ] Page count for the 148-row sample is 5 (current: 7); the last page has more than 2 rows.
- [ ] With dividers toggled off the row set is unchanged.

## K. Known issues and unresolved questions

**Verified defects in the current output**
- Last page has 2 rows (p7).
- 160 / 200 / 160 pt column widths wrap names (p2-p6).
- Shell defects shared with the Scene Report.

**Design choices (made here, reversible)**
- Letter dividers (derived; optional).
- Dense row padding for this list only.
- Running header on all pages.

**Unverified assumptions / open questions**
- Whether letter dividers are wanted by props masters (not user-validated).
- Whether the Rust pager can keep a divider with its next row (must implement `break-after: avoid` equivalent).
