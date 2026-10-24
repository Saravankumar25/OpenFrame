# Production Report — Cast Scene Report  (`reports.export_report`, `reportType: "cast_scene"`)

| | |
|---|---|
| Export | Production reports, op `reports.export_report` (PDF; CSV and XLSX are separate projections of the same `ReportData`) |
| Variant | **Report_Cast_Scene** — portrait, 5 columns (column order changed: Character first) |
| Status | Design reference — not integrated; not validated against OpenFrame's Rust renderer |
| Version | v2 — Shooting days in shoot-day order (see K.4 Changes from v1) |
| Repo commit | `2f5a045416bd2125801329533fc47f986ccc8174` (`main`) |
| Date | 2026-10-23 |
| Files | `exports/production-reports/Report_Cast_Scene.html` / `.pdf` / `.md`; stress tests in `exports/production-reports/stress-test/` |
| Shared shell | Defined once in `Report_Scene.md`, section **F.1 Shared report shell**; the other five specs reference it and add their own column specs |


## A. Purpose and users

**Who:** the casting director/coordinator, the 1st AD and the production office (call sheets, travel, contracts).

**When:** during casting and again when building the shooting schedule and cast availability.

**What they must find fast:** per character, **who plays them** (or that the role is **Not cast yet**), in how many scenes and which, and **on which shoot days they work**. Casting scans for `NOT CAST YET`; the AD scans by character name; both need the list alphabetical by character with the character as the left-most key.

## B. Verified functionality

**Scope.** One production report, generated on demand from the current Production Source / current schedule, or a saved snapshot of one. Six live report types exist (`reportType`): `scene`, `location`, `cast_scene`, `prop`, `schedule`, `breakdown_completeness` (labels in the Reports tab: Scene Report, Location Report, Cast Scene Report, Prop Report, Schedule Report, Breakdown Completeness). This spec covers **Cast Scene Report** (`cast_scene`). Saved snapshots use the identical `ReportData` schema and print through the same layout (see "Saved snapshot").

**Inputs / args (verified, `ExportReportArgs` in `exports/reports.rs`, `deny_unknown_fields`, camelCase).**

| Arg key | Meaning |
|---|---|
| `format` | `"pdf"` \| `"csv"` \| `"xlsx"` (the UI offers PDF, CSV, Excel (XLSX) from `TABLE_FORMATS`) |
| `path` | destination chosen in the export dialog |
| `reportType` | live report type, optional; used when `reportId` is absent |
| `filter` | live text filter, optional (the Reports tab box "Filter rows (scene, location, name…)", max 200 chars, trimmed, blank = none) |
| `reportId` | a saved snapshot instead of a live report (`production_report` row, not deleted); when present it wins over `reportType`; neither present -> error "A report" required |

**Options.** None beyond format. The export dialog (`ReportExportButton` -> `ExportDialog`, title `Export Cast Scene Report`, source line `Source: Cast Scene Report` plus ` · filtered by “…”` when a filter is typed, or `Source: saved report <title>`) has no scope or checkbox options.

**Data dependency.** `reports.generate` -> `build_report` in `modules/schedule/docs.rs`. It needs a Production Source (taken from the current schedule's source, otherwise the active source) and reads non-omitted scenes (`!s.omitted`) and, for day information, the current schedule board.

**Filter semantics (verified).** Case-insensitive substring match: a row is kept when **any cell** contains the text (`report_filter`, `rows.filter(... any(cell.to_lowercase().contains(..)))`).

**Empty / error behaviour (verified).**
- No source selected: `columns = []`, `rows = []`, `note = "Choose a Production Source in Breakdown first."` (`NO_SOURCE_MESSAGE`), `sourceLabel = ""`. The PDF prints masthead (no `Source:` part), zero-value metrics and an info callout with that sentence; portrait (0 columns).
- Source but no rows (e.g. filter matches nothing): columns are still returned, `note = "No matching data."`; the PDF prints masthead, zero metrics and an info callout with the note; **orientation follows the column count, so an empty Cast Scene Report is portrait**.
- Unknown `reportType`: validation error "Choose one of the available reports." Filter longer than 200 characters: validation error "Keep the filter under 200 characters." Export needs the Export capability (`require_export`).
- A report row cell that the PDF font cannot draw: export is refused with `HINT_TABLES` ("Export it as CSV or Excel (XLSX) to keep them, or replace those characters and export the PDF again.") — behaviour inherited from the shared export pipeline.
**Cast Scene Report specifics.** Source columns (5, exact strings, source order): `Actor`, `Character`, `Scenes`, `Scene count`, `Shooting days`. One row per cast element key (`c.key`) across non-omitted scenes; `Actor` = assigned actor or `Not cast yet`; rows are sorted by `Character` (plain string order: upper-case before lower-case). `Scenes` = scene numbers `, `-joined; `Scene count` = how many scenes; `Shooting days` = distinct day titles `; `-joined. **The builder emits them in first-seen (scene) order; this design and v2 require shooting-day order (see B.1 and K.4).** Metric: `Cast members` = row count.

**Saved snapshot.** `reports.save` stores `ReportData` JSON in `production_report` (`title` defaults to `"Cast Scene Report: <source label>"`, user-editable up to 160 chars). Exporting with `reportId` prints that stored data: the masthead title is the **snapshot title**, the generated date is the snapshot's, the scope reads "Saved snapshot", and the stored `filter` (if any) is available to print. Rows never change after saving.

### B.1 Current-output assessment

Baseline: `Production Report — Cast & Scene.pdf` (4 pages, portrait, 22 Oct 2026, project Low Tide Radio, source `LOW TIDE RADIO: Blue Revision (Locked)`).

**Retain**
- Columns/labels, sort by character, `NOT CAST YET` warn chip, right-aligned bold `Scene count` (p1-p4).
- Metric `Cast members` = 17.

**Improve visually**
- `Shooting days` is a `; `-joined paragraph in a 99 pt column that wraps into 22 lines for ANJALI (p1: that row is about 264 pt tall), so page 1 holds only 2 rows and the report runs 4 pages; days are now one per line in a 168 pt column and the report runs 2 pages.
- The list is sorted by character but the first column is Actor, so the sort looks random (p1-p4); Character now leads.
- `STUDIO PRODUCER` wraps in the 99 pt Character column (p4); Character is 108 pt so a bold caps name of about 15 characters stays on one line.

**Verified functional / layout problems (current output)**
- Page 1 carries only 2 of 17 rows (ANJALI, APPACHAN) because the ANJALI row is about 264 pt tall (p1).
- Sort by `Character` uses byte order (`rows.sort_by(|a,b| a[1].cmp(&b[1]))`), so mixed-case names sort upper-case first; the sample happens to be consistent (p2: `Guard`, `Harbour Clerk` between `FATHIMA` and `JOE`).
- **Shooting days are jumbled, not in shooting-day order** (user feedback; baseline `Production Report — Cast & Scene.pdf` p1, row ANJALI / Nimisha Raghavan: `Shoot Day 12, 3, 4, 7, 10, 5, 2, 6, 11, 8, 1, 9, 14`; 9 of the 17 rows are out of order, e.g. p1 APPACHAN `3, 4, 8, 12, 9, 14`). Cause (verified, `modules/schedule/docs.rs`, `build_report`, arm `"cast_scene"`): `for sc in &scenes { for c in src.cast(&sc.id) { … if let Some(d) = day_of(&sc.id) && !e.3.contains(&d) { e.3.push(d); } } }` — days are appended in the order scenes are visited and joined with `; `; the only sort in the arm is `rows.sort_by(|a,b| a[1].cmp(&b[1]))` (by character). `reports.rs::export` prints the cell as given. `Scenes` (`e.2`) is built the same way and is ascending only because scenes are visited in script order (checked: all 17 rows ascending).
- Shell defects shared with the Scene Report.

**Cannot be determined from the evidence**
- Whether one actor playing two characters yields two rows (the grouping key is `c.key`, not the actor) — the sample has no such case.
- Whether `Scenes` should show scene numbers or headings to casting — design keeps numbers (source).


## C. Source mapping

| Item | Verified identifier |
|---|---|
| Commit | `2f5a045416bd2125801329533fc47f986ccc8174` |
| UI entry point | `windows/apps/desktop/src/workspaces/production/tabs/reports.tsx`, component `ReportsTab` (tab id `reports`, label "Reports"); the list entry "Cast Scene Report" in the left `vpanel` listbox; export button = `ReportExportButton` (from `windows/apps/desktop/src/features/export/buttons.tsx`) placed in the `PageHeader` actions, next to "Save snapshot". |
| Op name | `reports.export_report` (registered in `exports/mod.rs`, `r.query("reports.export_report", reports::export)`, capability Export) |
| Builder | `windows/crates/openframe-application/src/modules/exports/reports.rs` -> `export()` (+ helpers `report_col`, `report_cell`, `status_tone`, `is_numeric_column`) |
| Data | `reports.generate` / `build_report` / `struct ReportData { report_type, title, source_label, generated_at, columns, rows, note, filter }` in `modules/schedule/docs.rs`; saved: `reports.save`, `reports.get`, table `production_report` (`title`, `snapshot_json`) |
| Builder fn for this report's rows | `build_report` arm `"cast_scene"` in `modules/schedule/docs.rs` (`src.cast(&sc.id)`, `c.actor`, `c.character`, `c.key`) with `report_cell` (`Actor == "Not cast yet"` -> warn chip; numeric `Scene count` right-aligned) in `exports/reports.rs` |
| Renderer blocks used (current) | `Document::new(..).landscape(columns > 5).footer(footer(project, data.title))`; `Block::Masthead` (via `production_masthead`, `title_size = 26`); `Block::Metrics(compact)` of `Metric`s; `Block::Styled(StyledTable{ header: HeaderStyle::Navy, boxed, zebra, continued: Some(data.title) })` with `Row`/`Cell`; cell items `CellItem::Tile` and `CellItem::Chips(Chip::label(.., Tone))`; `Block::Callout{Tone::Info}` for the empty state; `Block::Text` (8.4 pt slate) for `data.note` under the table; plain `Table` pushed to `Output.tables` for CSV/XLSX. |
| Design needs from the renderer (new) | running header (single line, top-right, multi-page only); compact masthead; context banners (snapshot / filter); fit-content metrics strip; status chip with leading shape (disc / ring+dot / ring / square); left stripe on attention rows; two-line "day / date" cell; stacked days list cell; letter-divider rows (Prop only); footer logo PNG embedded (current renderer redraws a vector mark + text — implementation requirement from the shared brief). |


## D. Field inventory

Shared shell rows first, then this report's table. Note: the table prints Character before Actor.

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
| Metric `Cast members` | `rows.len()` | derived | 0 |
| `Character` (first column) | `c.character` | required | — |
| `Actor` | `c.actor` or `Not cast yet` | required (derived default) | `Not cast yet` warn chip + orange row stripe |
| `Scenes` | scene numbers joined `, ` | required | — |
| `Scene count` | number of scenes | required | — |
| `Shooting days` | day titles joined `; `, one printed per line | optional | Empty cell |

## E. Content architecture

**Order and hierarchy.** Shell, then one table sorted by character. Row = bold character (key) | actor or warn chip | scene numbers | count | stacked shooting days.

**Table columns** (design order shown; widths resolved by the generator `tools/build/production-reports/gen.py::widths`, the same rule the Rust renderer should apply):

| Column (exact header) | Width at current column set | Align | Wrap | Treatment |
|---|---|---|---|---|
| `Character` | 108 pt (flexible, weight 108) | left | wraps | bold navy |
| `Actor` | 96 pt (flexible, weight 96) | left | wraps | regular ink; `Not cast yet` = warn chip |
| `Scenes` | 104 pt (flexible, weight 104) | left | wraps at `, ` | regular ink |
| `Scene count` | 44 pt (fixed) | right | no wrap; header wraps to 2 lines (SCENE / COUNT) | bold navy |
| `Shooting days` | 168 pt (flexible, weight 168) | left | one day per line | `<b>Shoot Day N</b> · date` |

Sum of widths = 520 pt (content width of portrait Letter minus 2 x 46 pt margins).

**Sorting:** rows unchanged (by character). **Within a row, `Shooting days` are sorted: stable ascending by shoot-day number, tie-break by date, unscheduled / undated days last (requirement for the builder or renderer; the builder currently emits first-seen order).** `Scenes` is already ascending and is not changed.

**Changes versus the current PDF, and why**
- **v2: `Shooting days` listed in shooting-day order** (ANJALI: Shoot Day 1, 2, 3 … 12, 14), see K.4.
- **Column order: `Character` first, `Actor` second** (source order is Actor, Character). Why: the rows are sorted by character; the sorted column should be the left-most so the list reads alphabetically. Headers and cell content are unchanged. Reverse by swapping the two entries in the column list.
- `Shooting days` split into one line per day; widths Character 108, Actor 96, Scenes 104, Scene count 44, Shooting days 168 pt (current: 99 / 99 / 124 / 99 / 99).
- `Not cast yet` rows also get an orange stripe.


## F. Visual specification

The shared shell (running header, masthead, context banners, metrics strip, table styling, status chip, orientation rule) is specified in `Report_Scene.md` **F.1 Shared report shell**; the values below are only what is specific to this report.

**Page:** Letter portrait, content 520 x 680 pt; the 17-row sample is 2 pages, so the running header `LOW TIDE RADIO · CAST SCENE REPORT` is used.

**Report-specific:** `Not cast yet` = warn chip (ring) + 3 pt orange stripe on the row (word + shape + bar survive grayscale). Days block: 8.4 pt / 1.25, 1.4 pt between lines; `Shoot Day N` bold navy, rest regular ink. The tallest sample row (ANJALI, 14 days) is about 150 pt.

**Everything else:** F.1.

## G. Dynamic behaviour

Shared rules are in `Report_Scene.md` F.1/G; the items below are this report's specifics.

- 13 stacked days (ANJALI) make the tallest sample row (about 150 pt); a cast member working 60 days would need about 650 pt, close to a full page: the renderer must be able to split such a row between lines (open question, K). The stress rows show 14 days and 60 scene numbers.
- Character name very long (stress): wraps in the 108 pt column; actor name very long: wraps in 96 pt.
- Actor field absent -> `Not cast yet` chip (the source substitutes it).
- No scheduled days: `Shooting days` empty.
- A filter on an actor name (`suresh`) matches both the actor and any `Ameya Suresh` row, since any cell counts.

**Always true for every report (see Scene G):** rows grow to fit wrapped text and are never ellipsised; a row is never split across pages; header row repeats on every page; an optional banner/note that is absent leaves no gap; an empty result prints the source's note in an info callout and no table; unknown/extra columns use the generic fallback widths and make the document landscape when `columns > 5`.

## H. Pagination and print rules

- Paper: US Letter, **portrait 612 x 792 pt** (`columns.len() = 5`; rule `> 5` -> landscape). Margins 46 pt left/right/top, 66 pt bottom (footer zone). Content width 520 pt.
- Table header repeats on every page (`thead` navy row). There is **no table title row**; the report's name on continuation pages is carried by the running header (see F.1 item 1).
- Rows `break-inside: avoid`. A row that does not fit moves whole to the next page. Minimum rows: never leave a single row alone on the last page when it can be avoided — move one more row with it (current output violates this, see K).
- Masthead, context banners and metrics strip are `break-after: avoid`: they stay with the first table rows; page 1 must show at least 3 table rows below the metrics, otherwise the table starts on a new page together with its header.
- Tall rows move whole to the next page, which can leave white space at the foot of page 1 (about 80 pt in the sample); acceptable and consistent with the rest of the package.
- Header repeats; header cell `Scene count` wraps to two lines (SCENE / COUNT) at 44 pt.
- Footer and numbering: shared system — official logo (17 pt high) + `Created using OpenFrame` bottom-left, `Page X of Y` bottom-right, on every page. The old footer text `<project> · <report title>` is **not** reproduced in the footer (brand rule); that identity moves to the running header.
- Running header: `@top-right`, `{PROJECT NAME} · CAST SCENE REPORT`, only when the report spans more than one page.

## I. HTML/PDF correspondence

- `Report_Cast_Scene.html` is the printable document: one `<div class="doc">`, shared print CSS linked from `../../00_SYSTEM/assets/openframe-print.css`, shell + report CSS in one inline `<style>`, `of-preview.js` for the screen-only toolbar. Generated by `tools/build/production-reports/gen.py` from the real rows in `tools/build/production-reports/data/cast_scene.json` (extracted from the supplied baseline PDF by `extract_real_rows.py`); the HTML and the PDF therefore contain the same content.
- **PDF captured state:** live report, no filter, not a snapshot, as the user would export it on 22 Oct 2026 from `LOW TIDE RADIO: Blue Revision (Locked)`: 17 real rows, **2 pages** (current output: 4). Rendered with `python tools/render_pdf.py Report_Cast_Scene.html Report_Cast_Scene.pdf` (Chrome 155 headless, `@page` margin boxes for the running header, logo footer and `Page X of Y`). `python tools/verify_pdf.py Report_Cast_Scene.pdf --size letter --header "LOW TIDE RADIO · CAST SCENE REPORT"` passes.
- **Preview toolbar toggles (`data-toggle`):**
  - `Filter active` — sample text `suresh`; `Saved snapshot (illustrative)` — snapshot title `Cast Scene Report: LOW TIDE RADIO: Blue Revision (Locked)`.
- **Stress tests** (`stress-test/`; rows marked fictional are illustrative and are never in the primary PDF):
  - `Report_Cast_Scene_STRESS_TEST` (7 pages): saved-snapshot identity with a 160-char title, a 200-char filter, a very long project and source label, extreme cell text, rows with missing fields, every status variant, then many fictional rows so the table runs several pages with repeated headers; the optional note slot is filled.
  - `Report_Cast_Scene_STRESS_TEST_WIDE` (2 pages): 8 columns (the 5 real ones + 3 fictional: Costume, Call time, Contact) -> the **whole document is landscape** (rule `columns > 5`); shows the generic column rules.
  - `Report_Cast_Scene_STRESS_TEST_EMPTY` (1 page): a filter that matches nothing -> metrics at 0 and the source's note `No matching data.` in an info callout; portrait.
  - The no-Production-Source state is identical for all six reports apart from the title and metric labels; it is shown once, in `Report_Scene_STRESS_TEST_NO_SOURCE`.
- **Known differences versus the Rust renderer:** Chrome lays out tables with CSS (`table-layout: fixed` with `<colgroup>` widths); the Rust renderer must measure with the same metrics it paints (existing rule) so wrap points can differ by a word. Letter-spacing, `box-shadow` stripes and the ring/dot shapes are drawn with rects/circles in the renderer (no CSS features are relied on beyond flat fills, hairlines and rounded rects). Chrome keeps a divider/row with its neighbour via `break-after/inside: avoid`; the Rust pager must implement the equivalent. The HTML status/filter toggles are preview-only JavaScript and recompute metrics and zebra from the visible rows; in the app those values come from `build_report`.

## J. Acceptance criteria

- [ ] Page size is Letter portrait; margins 46/46/46/66 pt; nothing drawn in the 66 pt footer band or outside the side margins (`tools/verify_pdf.py --size letter` passes).
- [ ] Footer on every page: logo lockup + `Created using OpenFrame` + `Page X of Y` with the true total.
- [ ] Masthead shows, in this order: eyebrow `PRODUCTION REPORT`; title; `Project: … · Source: …` (a part is omitted when blank); right panel `GENERATED` / date / scope (`All rows` | `Filtered rows` | `Saved snapshot`). There is no `Export reference` line and no second logo.
- [ ] Metrics strip labels and counts equal the source's metrics for this report (`Cast members`) and are recomputed from the rows actually printed (filtered rows -> filtered counts).
- [ ] Column headers are the exact `data.columns` strings, in the order given in section E; header row repeats on every page; header text never splits mid-word.
- [ ] No cell text is truncated or ellipsised; very long unbroken strings wrap by character inside their column; no cell overflows its column.
- [ ] Every status value is readable with colour removed: the word is always printed, and the chip carries its shape (disc / ring+dot / ring / square). `verify_pdf.py --gray` thumbnails reviewed.
- [ ] Zebra restarts correctly and body text is >= 8.4 pt; chips >= 6.6 pt; running header 6.8 pt.
- [ ] Empty result: no table, metrics show 0, info callout shows exactly the note text (`No matching data.` or `Choose a Production Source in Breakdown first.`), no running header, one page.
- [ ] Filter active: a filter banner prints `Filter` + the filter text (wraps if long) and the scope says `Filtered rows`. Saved snapshot: a snapshot banner prints, the title is the snapshot title, the date is the snapshot date; both banners can appear together.
- [ ] Orientation is decided once per document from the column count and never changes between pages.
- [ ] Character is the first column and the list is in the source's character order.
- [ ] Every `Not cast yet` row shows the warn chip, its word, its ring mark and the orange stripe.
- [ ] Shooting days print one per line, `Shoot Day N` bold, in ascending shoot-day order in every row: the ANJALI row lists `Shoot Day 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 14` (13 days, not `12, 3, 4, 7 …`). Automatic check: for every row parse `Shoot Day (\d+)` in each `Shooting days` cell and compare with the sorted list.
- [ ] Sorting is stable, ties broken by date; a day without a number or date is last. `Scenes` stays ascending.
- [ ] `Scene count` is right-aligned bold; its header wraps to two lines without splitting a word.

## K. Known issues and unresolved questions

**Verified defects in the current output**
- Page 1 holds 2 of 17 rows (p1).
- Days not in shooting-day order (9 of 17 rows, p1-p4; user feedback) — see B.1.
- Byte-order sort of `Character`.
- Shell defects shared with the Scene Report.

**Design choices (made here, reversible)**
- Character/Actor swap (reversible).
- Days stacked one per line.
- Stripe on Not cast yet rows.
- Orientation kept portrait (5 columns); landscape would shorten rows but changes the `> 5` rule (option: set landscape for this report type).

**Unverified assumptions / open questions**
- Whether casting prefers actor-first order for contract work; the swap is a usability judgement, not user-validated.
- Splitting `Shooting days` on `; `.
- Renderer behaviour for a row taller than the page.

### K.4 Changes from v1

- **Fixed: `Shooting days` order.** v1 printed the days in first-seen (scene) order, e.g. ANJALI `Shoot Day 12, 3, 4, 7, 10, 5, 2, 6, 11, 8, 1, 9, 14`. v2 sorts them ascending: `Shoot Day 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 14`. Rule (requirement for the builder / renderer): stable ascending by shoot-day number, tie-break by date, unscheduled / undated days last. Origin: `modules/schedule/docs.rs::build_report`, arm `"cast_scene"`, vector `e.3` (appended while visiting scenes, joined with `; `); the recommended fix is to sort `e.3` by (day number, date) before the join, which also fixes CSV/XLSX and the Reports tab.
- Affected files: `Report_Cast_Scene.html/.pdf/.md` and the Cast Scene stress tests (`_STRESS_TEST`, `_STRESS_TEST_WIDE`; `_EMPTY` has no rows). Rows, columns (including the v1 Character-first order), wording and widths are unchanged; page count unchanged (2).
- Checked and unchanged: `Scenes` is ascending in all 17 rows.
