# Production Report — Scene Report  (`reports.export_report`, `reportType: "scene"`)

| | |
|---|---|
| Export | Production reports, op `reports.export_report` (PDF; CSV and XLSX are separate projections of the same `ReportData`) |
| Variant | **Report_Scene** — landscape, 7 columns (the only landscape report) |
| Status | Design reference — not integrated; not validated against OpenFrame's Rust renderer |
| Repo commit | `2f5a045416bd2125801329533fc47f986ccc8174` (`main`) |
| Date | 2026-10-23 |
| Files | `exports/production-reports/Report_Scene.html` / `.pdf` / `.md`; stress tests in `exports/production-reports/stress-test/` |
| Shared shell | Defined once in `Report_Scene.md`, section **F.1 Shared report shell**; the other five specs reference it and add their own column specs |


## A. Purpose and users

**Who:** the 1st AD and assistant directors, the production coordinator and the production office; script supervisor and department heads for a quick "where and when is scene N" lookup.

**When:** after the breakdown and stripboard exist; printed or sent as the one-page-per-15-scenes "master scene list", re-run after schedule changes.

**What they must find fast:** (1) the scene number and heading; (2) interior/exterior and day/night; (3) page length in eighths; (4) location and cast for the scene; (5) **which shoot day it is on, or that it is Unscheduled** — the last column answers the AD's first question and the "Unscheduled" count in the metrics strip answers the coordinator's. The reader scans by scene number, so the Sc tile and the heading stay the left-most, boldest elements; the day is scanned as a two-line "Shoot Day 12 / Sat 23 Jan 2027" block that never wraps mid-date.

## B. Verified functionality

**Scope.** One production report, generated on demand from the current Production Source / current schedule, or a saved snapshot of one. Six live report types exist (`reportType`): `scene`, `location`, `cast_scene`, `prop`, `schedule`, `breakdown_completeness` (labels in the Reports tab: Scene Report, Location Report, Cast Scene Report, Prop Report, Schedule Report, Breakdown Completeness). This spec covers **Scene Report** (`scene`). Saved snapshots use the identical `ReportData` schema and print through the same layout (see "Saved snapshot").

**Inputs / args (verified, `ExportReportArgs` in `exports/reports.rs`, `deny_unknown_fields`, camelCase).**

| Arg key | Meaning |
|---|---|
| `format` | `"pdf"` \| `"csv"` \| `"xlsx"` (the UI offers PDF, CSV, Excel (XLSX) from `TABLE_FORMATS`) |
| `path` | destination chosen in the export dialog |
| `reportType` | live report type, optional; used when `reportId` is absent |
| `filter` | live text filter, optional (the Reports tab box "Filter rows (scene, location, name…)", max 200 chars, trimmed, blank = none) |
| `reportId` | a saved snapshot instead of a live report (`production_report` row, not deleted); when present it wins over `reportType`; neither present -> error "A report" required |

**Options.** None beyond format. The export dialog (`ReportExportButton` -> `ExportDialog`, title `Export Scene Report`, source line `Source: Scene Report` plus ` · filtered by “…”` when a filter is typed, or `Source: saved report <title>`) has no scope or checkbox options.

**Data dependency.** `reports.generate` -> `build_report` in `modules/schedule/docs.rs`. It needs a Production Source (taken from the current schedule's source, otherwise the active source) and reads non-omitted scenes (`!s.omitted`) and, for day information, the current schedule board.

**Filter semantics (verified).** Case-insensitive substring match: a row is kept when **any cell** contains the text (`report_filter`, `rows.filter(... any(cell.to_lowercase().contains(..)))`).

**Empty / error behaviour (verified).**
- No source selected: `columns = []`, `rows = []`, `note = "Choose a Production Source in Breakdown first."` (`NO_SOURCE_MESSAGE`), `sourceLabel = ""`. The PDF prints masthead (no `Source:` part), zero-value metrics and an info callout with that sentence; portrait (0 columns).
- Source but no rows (e.g. filter matches nothing): columns are still returned, `note = "No matching data."`; the PDF prints masthead, zero metrics and an info callout with the note; **orientation follows the column count, so an empty Scene Report is landscape**.
- Unknown `reportType`: validation error "Choose one of the available reports." Filter longer than 200 characters: validation error "Keep the filter under 200 characters." Export needs the Export capability (`require_export`).
- A report row cell that the PDF font cannot draw: export is refused with `HINT_TABLES` ("Export it as CSV or Excel (XLSX) to keep them, or replace those characters and export the PDF again.") — behaviour inherited from the shared export pipeline.
**Scene Report specifics.** Columns (7, exact strings): `Sc`, `Heading`, `I/E · D/N`, `Pages`, `Location`, `Cast`, `Scheduled`. One row per non-omitted scene in source scene order (`src.scenes`, scene 22 is omitted in the sample, so numbers are not contiguous). `I/E · D/N` is `"<int_ext> · <day_night>"` with `Not set` for an empty half; `Pages` = `pages(sc.page_eighths)` (e.g. `1 2/8`, `5/8`); `Location` = linked location names joined by `, `; `Cast` = character names joined by `, ` (not actors); `Scheduled` = the scene's shoot-day title `Shoot Day N · Mon 25 Jan 2027` (a day without a date prints only `Shoot Day N`) or `Unscheduled`. Metrics: `Scenes` = row count, `Unscheduled` = rows whose `Scheduled` is `Unscheduled`.

**Saved snapshot.** `reports.save` stores `ReportData` JSON in `production_report` (`title` defaults to `"Scene Report: <source label>"`, user-editable up to 160 chars). Exporting with `reportId` prints that stored data: the masthead title is the **snapshot title**, the generated date is the snapshot's, the scope reads "Saved snapshot", and the stored `filter` (if any) is available to print. Rows never change after saving.

### B.1 Current-output assessment

Baseline: `Production Report — Scene.pdf` (6 pages, landscape, 22 Oct 2026, project Low Tide Radio, source `LOW TIDE RADIO: Blue Revision (Locked)`).

**Retain**
- The seven columns, their labels and order (Sc, Heading, I/E · D/N, Pages, Location, Cast, Scheduled) — page 1.
- Scene number as a tile, INT/EXT and D/N as small chips, headings bold navy, navy table header with zebra rows, `Unscheduled` as a warn chip, landscape orientation for this report (p1-6).
- Metrics `Scenes` and `Unscheduled`; the source's own empty text.

**Improve visually**
- Masthead and metrics take about 200 pt of page 1, so page 1 shows only 7 of 44 rows; the compact shell shows 13 (p1).
- `Scheduled` wraps into `Shoot Day 13 · Mon 25 / Jan 2027` on every row (p1-p6, column fixed at 120 pt); it is now a bold `Shoot Day 13` over a slate `Mon 25 Jan 2027`, never wrapping.
- `I/E · D/N` is 100 pt wide for two small chips that need about 52 pt; the width is returned to Heading/Location/Cast, which wrap less (p1: Cast wraps to 3 lines on rows 2 and 3).
- Page identity (`LOW TIDE RADIO · SCENE REPORT`) lived in the footer text; it moves to a running header so the shared footer can stay constant.

**Verified functional / layout problems (current output)**
- Page 6 of 6 holds a single row (scene 45) — a lone-row last page (p6).
- Page 1 holds only 7 rows while pages 2-4 hold 9-10 (p1-p4): the oversized masthead (30/26 pt title, top bar, 4-line side panel, tall metric tiles) costs about two rows.
- The side panel prints the placeholder line `Export reference` on every export (p1).
- For a saved snapshot the panel prints only `Saved snapshot`; the stored filter text is not shown (`scope` match arm `(Some(_), _)` in `reports.rs`), although the Reports tab's snapshot banner shows it.
- A live filter is printed inside the 150 pt side panel as `Filtered by “…”`; up to 200 characters cannot fit (`reports.rs` `scope`).

**Cannot be determined from the evidence**
- The order of `src.scenes` is assumed to be script order (the sample ascends 1-45); the sort is not stated in `build_report`.
- Whether the Rust pager can keep the last two rows together (the current pager did not on p6).
- How the app wants an `Unscheduled` scene ranked (the report does not sort by schedule).


## C. Source mapping

| Item | Verified identifier |
|---|---|
| Commit | `2f5a045416bd2125801329533fc47f986ccc8174` |
| UI entry point | `windows/apps/desktop/src/workspaces/production/tabs/reports.tsx`, component `ReportsTab` (tab id `reports`, label "Reports"); the list entry "Scene Report" in the left `vpanel` listbox; export button = `ReportExportButton` (from `windows/apps/desktop/src/features/export/buttons.tsx`) placed in the `PageHeader` actions, next to "Save snapshot". |
| Op name | `reports.export_report` (registered in `exports/mod.rs`, `r.query("reports.export_report", reports::export)`, capability Export) |
| Builder | `windows/crates/openframe-application/src/modules/exports/reports.rs` -> `export()` (+ helpers `report_col`, `report_cell`, `status_tone`, `is_numeric_column`) |
| Data | `reports.generate` / `build_report` / `struct ReportData { report_type, title, source_label, generated_at, columns, rows, note, filter }` in `modules/schedule/docs.rs`; saved: `reports.save`, `reports.get`, table `production_report` (`title`, `snapshot_json`) |
| Builder fn for this report's rows | `build_report` arm `"scene"` in `modules/schedule/docs.rs` (headers `Sc, Heading, I/E · D/N, Pages, Location, Cast, Scheduled`; `day_of` looks up the strip's day via `Board::strip_day`) and `report_cell` / `report_col` in `exports/reports.rs` |
| Renderer blocks used (current) | `Document::new(..).landscape(columns > 5).footer(footer(project, data.title))`; `Block::Masthead` (via `production_masthead`, `title_size = 26`); `Block::Metrics(compact)` of `Metric`s; `Block::Styled(StyledTable{ header: HeaderStyle::Navy, boxed, zebra, continued: Some(data.title) })` with `Row`/`Cell`; cell items `CellItem::Tile` and `CellItem::Chips(Chip::label(.., Tone))`; `Block::Callout{Tone::Info}` for the empty state; `Block::Text` (8.4 pt slate) for `data.note` under the table; plain `Table` pushed to `Output.tables` for CSV/XLSX. |
| Design needs from the renderer (new) | running header (single line, top-right, multi-page only); compact masthead; context banners (snapshot / filter); fit-content metrics strip; status chip with leading shape (disc / ring+dot / ring / square); left stripe on attention rows; two-line "day / date" cell; stacked days list cell; letter-divider rows (Prop only); footer logo PNG embedded (current renderer redraws a vector mark + text — implementation requirement from the shared brief). |


## D. Field inventory

Every visible element, with its source. Shared shell rows first, then this report's table.

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
| Metric `Scenes` | `rows.len()` | derived | 0 |
| Metric `Unscheduled` | count of rows with `Scheduled == "Unscheduled"` | derived | 0 (value stays navy) |
| `Sc` (tile) | `sc.number` | required | — |
| `Heading` | `sc.heading` | required | Empty cell |
| `I/E · D/N` (two chips) | `sc.int_ext` · `sc.day_night` | required (derived string) | `Not set` per missing half (printed as a chip word) |
| `Pages` | `pages(sc.page_eighths)` | required | Empty cell |
| `Location` | `src.locations(sc)` names joined `, ` | optional | Empty cell |
| `Cast` | `src.cast(&sc.id)` character names joined `, ` | optional, repeatable | Empty cell |
| `Scheduled` (day / date lines or warn chip) | `Board::strip_day` -> `DayRow::title()` (`Shoot Day N · Mon 25 Jan 2027`) or `Unscheduled` | required (derived) | `Unscheduled` chip + left stripe on the row |

## E. Content architecture

**Order and hierarchy.** Masthead -> (optional snapshot banner) -> (optional filter banner) -> metrics strip -> table -> (optional note). Table rows in source scene order, no grouping. Within a row the hierarchy is: tile + bold heading (key) / chips / pages / location / cast / schedule block.

**Table columns** (design order shown; widths resolved by the generator `tools/build/production-reports/gen.py::widths`, the same rule the Rust renderer should apply):

| Column (exact header) | Width at current column set | Align | Wrap | Treatment |
|---|---|---|---|---|
| `Sc` | 34 pt (fixed) | left | no wrap | navy-tint tile, 8.2 pt bold, min 18 pt wide |
| `Heading` | 186 pt (flexible, weight 186) | left | wraps | bold navy |
| `I/E · D/N` | 60 pt (fixed) | left | chips wrap to 2 lines if both read `Not set` | two small neutral chips (`INT`/`EXT`, `D`/`N`/`Not set`) |
| `Pages` | 40 pt (fixed) | left | no wrap | text, e.g. `1 2/8` |
| `Location` | 150 pt (flexible, weight 150) | left | wraps | text |
| `Cast` | 140 pt (flexible, weight 140) | left | wraps | text, characters joined by `, ` |
| `Scheduled` | 90 pt (fixed) | left | day and date lines never wrap (nowrap) | line 1 `Shoot Day N` bold navy; line 2 date slate; `Unscheduled` = warn chip |

Sum of widths = 700 pt (content width of landscape Letter minus 2 x 46 pt margins).

**Sorting:** none added; rows follow the source scene order.

**Changes versus the current PDF, and why**
- `Scheduled` split into a bold day line and a slate date line (the string is `label · date`, so the split is on `" · "`; a value without it prints one line). Why: it wrapped on every row in the current PDF.
- Fixed widths re-balanced (Sc 34, I/E · D/N 60, Pages 40, Scheduled 90; Heading/Location/Cast share the remaining 476 pt as 186/150/140) instead of Sc 40 / I/E 100 / Pages 48 / Scheduled 120 with weights 2.4 / 1.2 / 1.2.
- `Unscheduled` rows get an orange left stripe in addition to the chip.
- Masthead, panel, banners and metrics replaced by the shared shell (F.1); `continued` title line replaced by the running header; footer text moved to the running header.
- No change to columns, labels, order, sorting or cell content.


## F. Visual specification


### F.1 Shared report shell (defined once here; the other five reports reference it)

All six reports (and every saved snapshot) are drawn inside the same shell. Everything below is flat fills, hairlines, rounded rects, circles and Helvetica text, i.e. inside the Rust renderer's vocabulary. Values are in pt and tokens refer to `00_SYSTEM/assets/openframe-print.css`.

**1. Running header (multi-page reports only).** One line, `@top-right`, 6.8 pt regular, `--of-slate`, tracking 0.04 em, 24 pt below the page top, no rule, no logo. Text = `<PROJECT NAME UPPERCASE> · <data.title UPPERCASE>`, e.g. `LOW TIDE RADIO · SCENE REPORT` (project part omitted when the project title is blank; for a snapshot the report-type title is used, never the long snapshot title). *Decision:* the current footer printed `<project> · <report>` on every page (`footer(&project, &data.title)`); the shared brand footer has fixed content, so that page identity moves to this header. A report that fits on one page prints no header (the Location and Schedule PDFs are single-page and verify with `--no-header`; the empty and no-source stress files likewise). The renderer already computes `Page X of Y`, so it knows the page count when it decides.

**2. Masthead (`.rpt-mast`, compact).** Two columns, 14 pt gap, `break-inside/after: avoid`, 7 pt below.
- Left: eyebrow `PRODUCTION REPORT` — 7 pt bold, caps, tracking 0.14 em, `--of-slate`, preceded by a 14 x 2 pt `--of-orange` bar (fill only; orange is never used for small text). Title — 20 pt / 1.1 bold `--of-navy`, tracking -0.01 em, 3 pt above and 3.5 pt below; it is `document` (the snapshot title for a saved snapshot, otherwise `data.title`), wraps to as many lines as needed, never truncated. Meta line — 8 pt `--of-slate`: `Project: <project> · Source: <data.source_label>`; either part is dropped when blank (`nb()`), the whole line when both are.
- Right: info panel, rounded 6 pt, fill `--of-panel`, hairline `--of-line`, padding 6/11/7 pt, right-aligned, min 104 pt / max 150 pt wide: `GENERATED` (6.8 pt bold caps, tracking 0.14 em, slate) / date (10.5 pt bold navy, `date_from_ms(data.generated_at)`, e.g. `22 Oct 2026`) / scope (8 pt slate): `All rows`, `Filtered rows` or `Saved snapshot`.
- Removed versus the current masthead: the 30/26 pt title, the `PRODUCTION / SCENE REPORT` top bar, the uppercase date, and the placeholder line `Export reference`. The masthead + metrics now occupy about 137 pt from the page top to the table header on page 1 versus about 200 pt before, which is what lets page 1 carry two more rows.

**3. Context banners (`.rpt-ctx`, optional, 6 pt below).** Shared `.banner` (flex, padding 5/9 pt, 3 pt left stripe `--of-teal`, fill `--of-blue-fill`, label 7 pt bold caps nowrap).
- Saved snapshot (only when exporting a `reportId`): `SAVED SNAPSHOT  <report title> · generated <date>. It does not change when production data changes.` (the last sentence is the Reports tab's own banner wording).
- Filter (when `data.filter` is non-blank, live or stored in a snapshot): `FILTER  “<filter>” · rows with this text in any cell (not case-sensitive)`. The filter text is 8 pt bold navy and wraps (up to 200 chars -> up to 3 lines at portrait width). *Decision:* the current PDF puts the whole filter into the 150 pt side panel as `Filtered by “…”`, which cannot hold 200 characters, and drops the stored filter for snapshots; the panel now carries only the short scope word and the banner carries the text.

**4. Metrics strip (`.rpt-metrics`).** One rounded 4 pt rectangle, fill `--of-panel`, hairline, width = its content (not full width), max = content width, 9 pt below. Each metric: value 13 pt bold navy (tabular figures) + label 6.8 pt bold caps tracking 0.1 em slate, baseline-aligned, 6 pt gap, padding 5.5/10/6 pt, hairline divider between metrics, labels may wrap to two lines. Exact metric labels per report come from `reports.rs` (Scene: `Scenes`, `Unscheduled`; Location: `Locations`; Cast Scene: `Cast members`; Prop: `Props`; Schedule: `Schedule rows`; Breakdown: `Scenes`, `Reviewed`, `In progress or needs review`, `Not started`). Tone: value turns `--of-amber-text` when `Unscheduled` or `In progress or needs review` is > 0 and `--of-red-text` when `Not started` is > 0; a zero stays navy. The number and label always print, so colour is never the only carrier. Counts are of the printed rows (so a filtered report shows filtered counts).

**5. Table.** `StyledTable`: navy header (`HeaderStyle::Navy`), boxed (outer hairline `--of-line-strong`), zebra, header repeated on every page.
- Header cells: 6.9 pt bold caps, tracking 0.09 em, white on `--of-navy`, padding 5/6 pt, vertically centred; numeric columns right-aligned; header words never split mid-word (floor widths below).
- Body: 8.4 pt / 1.28, `--of-ink`, padding 3.6/6 pt (`dense` variant 2.5/6 pt for the Prop list), top-aligned, hairline `--of-line` between rows, `--of-line-strong` under the last row, zebra fill `--of-panel` on every second row (restarts after a divider row). Rows are `break-inside: avoid`.
- Text roles: row key (heading / location / character / prop / day) = 8.4 pt bold navy; numeric column (all non-empty cells parse as a number, the same `is_numeric_column` rule as the source) = bold navy right-aligned (a `0` in "Suggested (unreviewed)" is regular `--of-muted`); everything else regular ink; secondary parts (dates) regular slate.
- No table title row: the table's name on continuation pages is the running header; the current `continued: Some(data.title)` line (`SCENE REPORT (CONTINUED)`) is dropped.
- Column widths: fixed columns keep their pt value; flexible columns share the rest by weight (the weights in each report's section E equal the pt widths at the current column set); a flexible column never goes below 44 pt (known column) or 76 pt (a column the spec does not know). If the minimums cannot fit the document is still laid out at the minimums (see G, wide column sets).

**6. Status chip (`.st`).** Rounded 3 pt, 0.5 pt border, 6.6 pt bold caps (tracking 0.06 em), padding 2.4/4.6/2.6/3.8 pt, with a 5 pt mark before the word. The word is always printed; tone + mark carry the meaning without colour: `ok` (green) = solid disc, `info` (blue) = ring with centre dot, `warn` (amber) = hollow ring, `danger` (red) = solid square, `neutral` = no mark. Tone map (source `status_tone`, plus one design change): Reviewed / Confirmed / Required -> ok; Needs review / Shortlisted / Searching / Unscheduled / Not cast yet -> warn; In progress / From heading -> info; Missing / Rejected -> danger; **Not started -> danger (design change; the source leaves it neutral)**; anything else (e.g. `Idea`) -> neutral. Scene numbers stay the existing navy-tint tile (`CellItem::Tile`), INT/EXT and D/N stay small neutral chips (`CellItem::Chips`).

**7. Attention stripe.** A row that needs action (Scene: Unscheduled; Cast Scene: Not cast yet; Schedule: Unscheduled row; Breakdown: In progress / Needs review) gets a 3 pt `--of-orange` bar on its left edge, a `Not started` row a 3 pt `--of-red-text` bar. It is redundant with the chip word, never the only signal, and prints as a dark vertical mark in grayscale.

**8. Note and empty state.** `data.note` when rows exist: 8.4 pt `--of-slate`, 5 pt below the table (the source's `Txt size 8.4 SLATE`). No rows: no table; the info callout (`Tone::Info`, 9 pt, `--of-blue-text` on `--of-blue-fill`, radius 6 pt, padding 8/10) prints `data.note`, or `No matching data.` when absent. The metrics strip stays (zeros).

**9. Orientation rule (verified) and the deliberate behaviour.** `reports.rs`: `Document::new(&document).landscape(data.columns.len() > 5)`. The audit pack's wording ("landscape when more than 5 columns") is correct for this commit. Column counts today: Scene 7 -> landscape; Location 5, Cast Scene 5, Prop 3, Schedule 5, Breakdown Completeness 5 -> portrait. Behaviour specified here:
- Orientation is a property of the whole document, decided once from `data.columns.len()`; it never changes between pages and is not influenced by row count, filter, or row content.
- A filter that returns no rows keeps the report's columns, so an empty Scene Report is still landscape and an empty Location Report still portrait (stress `_EMPTY`).
- No Production Source -> 0 columns -> portrait for every type, including the Scene Report (stress `Report_Scene_STRESS_TEST_NO_SOURCE`).
- A live report or an older saved snapshot with 6 or more columns turns the document landscape (content width 700 pt); known columns keep their fixed widths, unknown columns get weight 76 / floor 76 pt, known flexible columns shrink toward their floor (stress `_WIDE`, 8-10 columns). With 5 or fewer columns the same column rules resolve at 520 pt. The shell (masthead, panel, banners, metrics) stretches to the new content width; nothing else changes.
- The design does not move a 5-column report to landscape for readability (Cast Scene would benefit); that would change the `> 5` rule and is listed as an option in each spec's section K.
**Page:** US Letter landscape, 792 x 612 pt, margins 46/46/46/66 pt, content 700 x 500 pt. `@page { size: 11in 8.5in; @top-right { content: "LOW TIDE RADIO · SCENE REPORT"; } }`.

**Typography:** shell sizes as F.1; table 8.4 pt body, 6.9 pt header; heading bold navy; date line 8.4 pt slate.

**Scene-specific treatments:** Sc tile (`.tile`, 8.2 pt bold, navy on `--of-tint`, hairline `--of-line-strong`, radius 3.5 pt); I/E-D/N chips (`.chip`, 6.6 pt bold caps, neutral); `Unscheduled` = warn `.st` chip (hollow-ring mark) plus 3 pt orange row stripe, both readable in grayscale as a dark word-chip and a dark bar; metric `Unscheduled` value in amber text when > 0.

**Colours:** `--of-navy` header and keys, `--of-panel` zebra, `--of-amber-*` for the warn chip, `--of-orange` stripe. **Footer:** shared (logo lockup, `Created using OpenFrame`, `Page X of Y`). **Running header:** yes, `LOW TIDE RADIO · SCENE REPORT` (the report spans 3 pages).

## G. Dynamic behaviour


Dynamic behaviour shared by all six reports (defined once here).

| Situation | Rule |
|---|---|
| Optional parts absent | Filter banner, snapshot banner, note, attention list: removed with no gap or placeholder. Blank `Project` or `Source`: that half of the meta line is dropped. |
| Long title (snapshot title up to 160 chars) | Wraps to extra lines in the masthead; the right panel keeps its width; never truncated. |
| Long filter (up to 200 chars) | Wraps inside its banner; the banner grows. The panel shows only the scope word. |
| Long cell text | Wraps inside the cell, row grows, no ellipsis. A string with no break opportunity wraps by character. A key such as a heading is bold and may take 6+ lines before the page is affected. |
| Missing cell | Empty cell (nothing printed) for text, except the cases named per report (`—` for an off day's missing estimate). Required-looking cells such as Heading are never blank in real data. |
| Many rows | Table flows across pages, header repeats, running header on every page, footer `Page X of Y`. No limit in the design; the stress files run 4-8 pages. |
| Row taller than a page | Not expected; the Cast Scene "Shooting days" list is the tallest cell (14 stacked days = about 150 pt). If a row would exceed a page it must be allowed to split between cell lines (renderer fallback; unverified). |
| Empty result | See F.1 item 8. Filtered-empty keeps columns (orientation unchanged); no-source prints zero columns. |
| Wide column sets (> 5 columns) | Landscape, generic column rules, see F.1 items 5 and 9. Verified in the stress files up to 10 columns. Beyond ~9 unknown columns the 76 pt floor for unknown columns cannot fit 700 pt; behaviour is unspecified (open question in K). |
| Unscheduled / not cast / missing | Warn or danger chip with word + shape, left stripe on the row; counts feed the metrics strip where the source defines a metric. |
| Snapshot | Title, date and stored filter come from the snapshot; rows never change; the banner says so. |

**Scene Report specifics**

- Long `Heading` (stress row 46, about 200 characters): wraps to 6 lines in the 186 pt column, the row grows, chips and day block stay top-aligned.
- Heading without break opportunities (stress row 47): wraps by character, never overflows into the I/E column.
- `Not set · Not set`: two `NOT SET` chips stack in the 60 pt column.
- Empty `Cast`/`Location` cell: blank, row height unaffected.
- Day with no date: only `Shoot Day N` prints (one line).
- Filter matches only scenes on one day -> table shortens, metrics recount, no empty page.

## H. Pagination and print rules

- Paper: US Letter, **landscape 792 x 612 pt** (`columns.len() = 7`; rule `> 5` -> landscape). Margins 46 pt left/right/top, 66 pt bottom (footer zone). Content width 700 pt.
- Table header repeats on every page (`thead` navy row). There is **no table title row**; the report's name on continuation pages is carried by the running header (see F.1 item 1).
- Rows `break-inside: avoid`. A row that does not fit moves whole to the next page. Minimum rows: never leave a single row alone on the last page when it can be avoided — move one more row with it (current output violates this, see K).
- Masthead, context banners and metrics strip are `break-after: avoid`: they stay with the first table rows; page 1 must show at least 3 table rows below the metrics, otherwise the table starts on a new page together with its header.
- Landscape page body is 500 pt high; in the sample page 1 carries 13 rows and pages 2-3 carry 17 and 15 (current output: 7 / 9 / 10 / 9 / 8 / 1 over 6 pages), depending on wraps.
- Header row (7 columns) repeats on each page.
- Footer and numbering: shared system — official logo (17 pt high) + `Created using OpenFrame` bottom-left, `Page X of Y` bottom-right, on every page. The old footer text `<project> · <report title>` is **not** reproduced in the footer (brand rule); that identity moves to the running header.
- Running header: `@top-right`, `{PROJECT NAME} · SCENE REPORT`, only when the report spans more than one page.

## I. HTML/PDF correspondence

- `Report_Scene.html` is the printable document: one `<div class="doc landscape">`, shared print CSS linked from `../../00_SYSTEM/assets/openframe-print.css`, shell + report CSS in one inline `<style>`, `of-preview.js` for the screen-only toolbar. Generated by `tools/build/production-reports/gen.py` from the real rows in `tools/build/production-reports/data/scene.json` (extracted from the supplied baseline PDF by `extract_real_rows.py`); the HTML and the PDF therefore contain the same content.
- **PDF captured state:** live report, no filter, not a snapshot, as the user would export it on 22 Oct 2026 from `LOW TIDE RADIO: Blue Revision (Locked)`: 44 real rows, **3 pages** (current output: 6). Rendered with `python tools/render_pdf.py Report_Scene.html Report_Scene.pdf` (Chrome 155 headless, `@page` margin boxes for the running header, logo footer and `Page X of Y`). `python tools/verify_pdf.py Report_Scene.pdf --size landscape --header "LOW TIDE RADIO · SCENE REPORT"` passes.
- **Preview toolbar toggles (`data-toggle`):**
  - `Filter active` — shows the filter banner (sample text `coast radio`), filters the rows by substring in any cell, recounts the metrics, restarts zebra, scope reads `Filtered rows`; if nothing matches it shows the `No matching data.` callout instead of the table.
  - `Saved snapshot (illustrative)` — shows the snapshot banner, swaps the title to the default snapshot title `Scene Report: LOW TIDE RADIO: Blue Revision (Locked)`, scope `Saved snapshot` (the 20 Oct 2026 date in the banner is fictional).
- **Stress tests** (`stress-test/`; rows marked fictional are illustrative and are never in the primary PDF):
  - `Report_Scene_STRESS_TEST` (8 pages): saved-snapshot identity with a 160-char title, a 200-char filter, a very long project and source label, extreme cell text, rows with missing fields, every status variant, then many fictional rows so the table runs several pages with repeated headers; the optional note slot is filled.
  - `Report_Scene_STRESS_TEST_WIDE` (2 pages): 10 columns (the 7 real ones + 3 fictional: Stunts, Vehicles, Notes) -> the **whole document is landscape** (rule `columns > 5`) (it already is); shows the generic column rules.
  - `Report_Scene_STRESS_TEST_EMPTY` (1 page): a filter that matches nothing -> metrics at 0 and the source's note `No matching data.` in an info callout; landscape because the 7 columns are still returned.
  - `Report_Scene_STRESS_TEST_NO_SOURCE` (1 page): no Production Source -> `columns = []` -> portrait even for the Scene Report; note `Choose a Production Source in Breakdown first.`; masthead without `Source:`.
- **Known differences versus the Rust renderer:** Chrome lays out tables with CSS (`table-layout: fixed` with `<colgroup>` widths); the Rust renderer must measure with the same metrics it paints (existing rule) so wrap points can differ by a word. Letter-spacing, `box-shadow` stripes and the ring/dot shapes are drawn with rects/circles in the renderer (no CSS features are relied on beyond flat fills, hairlines and rounded rects). Chrome keeps a divider/row with its neighbour via `break-after/inside: avoid`; the Rust pager must implement the equivalent. The HTML status/filter toggles are preview-only JavaScript and recompute metrics and zebra from the visible rows; in the app those values come from `build_report`.

## J. Acceptance criteria

- [ ] Page size is Letter landscape; margins 46/46/46/66 pt; nothing drawn in the 66 pt footer band or outside the side margins (`tools/verify_pdf.py --size landscape` passes).
- [ ] Footer on every page: logo lockup + `Created using OpenFrame` + `Page X of Y` with the true total.
- [ ] Masthead shows, in this order: eyebrow `PRODUCTION REPORT`; title; `Project: … · Source: …` (a part is omitted when blank); right panel `GENERATED` / date / scope (`All rows` | `Filtered rows` | `Saved snapshot`). There is no `Export reference` line and no second logo.
- [ ] Metrics strip labels and counts equal the source's metrics for this report (`Scenes`, `Unscheduled`) and are recomputed from the rows actually printed (filtered rows -> filtered counts).
- [ ] Column headers are the exact `data.columns` strings, in the order given in section E; header row repeats on every page; header text never splits mid-word.
- [ ] No cell text is truncated or ellipsised; very long unbroken strings wrap by character inside their column; no cell overflows its column.
- [ ] Every status value is readable with colour removed: the word is always printed, and the chip carries its shape (disc / ring+dot / ring / square). `verify_pdf.py --gray` thumbnails reviewed.
- [ ] Zebra restarts correctly and body text is >= 8.4 pt; chips >= 6.6 pt; running header 6.8 pt.
- [ ] Empty result: no table, metrics show 0, info callout shows exactly the note text (`No matching data.` or `Choose a Production Source in Breakdown first.`), no running header, one page.
- [ ] Filter active: a filter banner prints `Filter` + the filter text (wraps if long) and the scope says `Filtered rows`. Saved snapshot: a snapshot banner prints, the title is the snapshot title, the date is the snapshot date; both banners can appear together.
- [ ] Orientation is decided once per document from the column count and never changes between pages.
- [ ] Page 1 shows at least 12 rows; the last page holds at least 2 rows (pagination of the sample: 3 pages vs 6 in the current output).
- [ ] Every `Scheduled` cell is two lines (`Shoot Day N` / date) or the `Unscheduled` chip; no date breaks across lines.
- [ ] `Unscheduled` rows carry the chip and the orange stripe; the `Unscheduled` metric equals their count.
- [ ] `I/E · D/N` prints two chips; an empty half prints `Not set`.
- [ ] A snapshot export shows the snapshot title, `Saved snapshot`, and the stored filter in a banner.

## K. Known issues and unresolved questions

**Verified defects in the current output**
- Lone-row last page (p6 of the current PDF).
- Oversized masthead leaves only 7 rows on page 1 (p1).
- `Scheduled` wraps on every row (p1-p6).
- `Export reference` placeholder line (p1).
- Snapshot exports drop the stored filter (code, `reports.rs` `scope`); live filter text can overflow the side panel.

**Design choices (made here, reversible)**
- Compact 20 pt title and fit-content metrics strip (the current 26 pt title is `mast.title_size = 26.0`).
- Day / date split in `Scheduled`; narrower `I/E · D/N`.
- Running header instead of footer text; `continued` line dropped.
- Orange stripe on Unscheduled rows (not in the current output).
- `Export reference` dropped; `Filtered by “…”` moved to a banner.

**Unverified assumptions / open questions**
- Rust renderer parity: wrap points, chip metrics and stripe drawing are Chrome-based here.
- Whether splitting `label · date` on `" · "` is acceptable in Rust or the builder should pass two values (a day label can in principle contain ` · `; the sample does not).
- CSV/XLSX projections are out of scope; they use the plain `Table` and are unaffected.
