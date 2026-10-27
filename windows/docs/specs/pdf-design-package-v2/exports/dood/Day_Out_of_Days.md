# Day Out of Days (DOOD) — PDF design handoff

| | |
|---|---|
| Export | Day Out of Days — op `schedule.export_dood` (PDF · XLSX · CSV; this spec covers the PDF) |
| Variant | Landscape matrix, one trio: `Day_Out_of_Days.html/.pdf/.md` + `stress-test/Day_Out_of_Days_STRESS_TEST.html/.pdf`. Industry vs Simple code style are `data-toggle` variants in the same HTML |
| Status | Design reference — not integrated; not validated against OpenFrame's Rust renderer |
| Repo commit | `2f5a045416bd2125801329533fc47f986ccc8174` (`main`, Windows tree) |
| Date | 2026-10-23 |
| Generator | `tools/build/matrix-finance/build_dood.py` + `dood_lib.py` (ports `derive_codes`, `CODES` and the label/count rules from the Rust source; the primary sample is checked against the 17 totals rows of the supplied PDF) |

---

## A. Purpose and users

- **Production manager / UPM / casting coordinator**: "Who is booked on which day, who is on hold, when does each actor start and finish?" — read at a glance, usually pinned to a wall or handed to an agent.
- **1st AD / scheduler**: spot over-long holds, travel and rehearsal/fitting days, and what was changed by hand (`*`).
- **Production accountant**: per-actor work/hold counts (the basis of "days worked" and "days held").
- **Cast and agents** (shared copy): find their own row fast, with dates and weekdays, in grayscale on a photocopy.

What must be findable in seconds: the actor's row (frozen name column), the day column (shoot number + date + weekday, month band), the code and its meaning (legend printed on every page), totals per actor and per day, and every manual correction.

## B. Verified functionality

**Scope.** The Day Out of Days of the *current* shooting schedule (`current_schedule`), for the cast and day range currently shown in the DOOD tab. It is a derived report: cast confirmed in Breakdown × scenes scheduled on each day; the only stored data is the user's manual corrections (`dood_override`).

**Inputs / args** (`ExportDoodArgs`, `deny_unknown_fields`):

| UI | arg key | Notes |
|---|---|---|
| Format (PDF / Spreadsheet (XLSX) / CSV) | `format`, `path` | `DoodExportButton` uses `[TABLE_FORMATS[0], xlsx "Spreadsheet (XLSX)", csv "CSV"]` |
| "Cast to show" select (All cast or one cast member) | `castKeys` (omitted = everyone) | picks rows only; numbers/codes always come from the whole schedule |
| "First day" / "Last day" selects | `fromDayId`, `toDayId` | inclusive; swapped if reversed |
| (schedule setting "Code style": "Codes: SW, W, WF, H" / "Codes: W and -") | not an export arg: `board.schedule.dood_style` (`"industry"` / `"simple"`) | decides derived codes and which legend items print |

No export-specific options. Dialog note (verbatim): "Corrections are marked with *. Export is a snapshot; the schedule is not changed."

**Formats / data.** PDF = masthead, metrics, per-band matrices, totals table, optional correction details. XLSX = 3 sheets: "Day Out of Days" (columns `#`, `Cast`, `Character`, one column per day `Day 4 (Mon 14 Jun 2027)`, then `Work, Hold, Travel, Other, Start, Finish`), "Codes" (legend), "Corrections" (`Cast, Day, Code, From the schedule, Note`). CSV = first table only. `revised_at` = schedule's last change, never the export clock, so output is reproducible.

**Empty / error behaviour.** No current schedule → error `validation.no_schedule` "Create a shooting schedule first."; no days or no cast rows → `nothing_to_export` "The Day Out of Days has no shooting days or cast members to show yet." (no PDF is produced, so there is no empty-state page to design). Capability `Export` required (not Viewer/Commenter).

**Codes — exactly what the source defines** (`schedule/dood.rs::CODES`, legend order):

| Code | Legend text (verbatim) | How it arises |
|---|---|---|
| SW | Start work | derived (industry style): first work day |
| W | Work | derived: any other work day (also the only work code in the simple style) |
| WF | Work finish | derived (industry): last work day |
| SWF | Start, work and finish on the same day | derived (industry): single work day |
| H | Hold (paid, not working) | derived (industry): non-work, non-off day strictly between first and last work day |
| T | Travel | **manual correction only** |
| R | Rehearsal | manual correction only |
| F | Fitting | manual correction only |
| D | Drop | manual correction only |
| - | Not working | derived in the *simple* style for non-work days; also selectable as a correction |

Off days derive to blank; blank means "no code for that day" (legend line "Blank: no code for that day"). Simple style prints only `W - H T R F D` in the legend. Totals per row: `work_days` = SW+W+WF+SWF, `hold_days` = H, `travel_days` = T, `other_days` = R+F+D; `start_label` / `finish_label` = first/last work day ("Shoot Day 1"). A corrected cell prints `code*` and the schedule's own code is kept in `derived_code` (blank is printed as the word "blank" in the corrections table).

> The UI labels the default style "industry". I did **not** verify these codes against any external DOOD convention (Movie Magic etc.); this spec only claims what `CODES` defines. Everything convention-like below (weekend days, reading guides, count rows, column-group order) is a *configurable design choice*, listed in section F.

### B.1 Current-output assessment (supplied `Day Out of Days (DOOD).pdf`, 8 pages)

**Retain**
- Landscape page; masthead eyebrow/title/"Project · Revised date" and the four facts (Schedule, Production source, Displayed range, Corrections).
- Four metrics (Shooting days, Cast members, Off days, Manual corrections).
- The code vocabulary, legend wording, the `*` correction marker, the "OFF DAY" column treatment, day header content (shoot number, date, weekday, month).
- Cast number tile + cast label; totals (Work, Hold, Travel, Other, Start, Finish); Correction details table and its wording.
- Source colours per code (they are already reinforced by the printed text).

**Improve visually**
- Make the matrix the hero: one dense, even grid, readable at arm's length; day columns equal width in every group.
- Code cells that survive grayscale: text + cell shape (start/finish caps, hollow hold box, ink outline for corrections), not hue alone.
- Month band over the day columns (also carries the year, which the day headers lack).
- Weekday emphasis for weekend shooting days; off days as a shaded column.
- Totals and counts inside the matrix instead of a separate 2-page table.

**Verified functional / layout problems in the current output**
1. A 14-shooting-day, 17-cast schedule needs **8 pages**: three 7-day "bands" (`PDF_DAYS_PER_TABLE = 7`), each band's 17 cast rows split across two pages (pp. 1–2, 3–4, 5–6) because banner + legend + header consume ~40 % of the page. The reader must flip between two pages to read one cast list.
2. Band 3 has only 2 day columns; the cast column stretches across the whole page (p. 5–6) leaving two thin columns at the far right — hard to track across 500 pt of whitespace.
3. The full legend is repeated above each band (pp. 1, 3, 5) but **not on continuation pages** (pp. 2, 4, 6), so those pages show codes with no key and no explanation of `*`; `*` itself is explained only in the masthead facts on p. 1.
4. Totals are in a separate table (pp. 6–7) that repeats all 17 names; to read an actor's days you cross-reference four pages.
5. The totals section says "Totals cover the whole schedule, not only the displayed range", but `dood.rs::build` counts Work/Hold/Travel/Other over `shown` (the displayed range) while only Start/Finish use the whole schedule. With a narrowed range (`fromDayId`/`toDayId`) the sentence is wrong. (Not visible in the supplied PDF, which has the full range; verified from source.)
6. Correction details split across pp. 7–8, leaving **one row alone on p. 8** (p. 8 is otherwise empty).
7. Cast labels are all upper-case bold in the matrix ("NIMISHA RAGHAVAN (ANJALI)") so actor and character are not distinguished; long labels wrap to two lines and make rows uneven.
8. Footer prints "Page 1 / 8" (bespoke) rather than the package-wide "Page X of Y".

**Cannot be determined from the evidence**
- How `PDF_DAYS_PER_TABLE = 7` was chosen beyond the source comment ("Raising it needs a rendered proof at print size").
- Whether the Mac app prints the same document (mac tree not compared).
- Behaviour of the live renderer with 40+ cast / 30+ days (only the design package was exercised, in Chrome).

## C. Source mapping

| Item | Value |
|---|---|
| UI entry point | `windows/apps/desktop/src/features/export/buttons.tsx` → `DoodExportButton({ args })`; mounted in `windows/apps/desktop/src/workspaces/production/tabs/dood.tsx` (toolbar, after the "Codes" select and the "N corrected by hand" chip) with `args = doodArgs(sel)` (`castKeys`, `fromDayId`, `toDayId`) |
| Op / builder | `schedule.export_dood` → `modules/exports/dood.rs::export` (`ExportDoodArgs`); data from `modules/schedule/dood.rs` (`report`, `build`, `derive_codes`, `CODES`, `ScheduleDoodView`, `DoodDay`, `DoodRow`, `DoodCell`, `DoodCode`) |
| Renderer blocks used today | `Masthead` (+ `Side::Facts`), `Metrics { compact: true }`, `Section`, `Banner`, `Legend` / `LegendItem`, `StyledTable` (`grid`, `boxed`, `pad_y`, `header_cells`, `continued`), `Cell` / `CellItem::Tile`, `HeaderStyle::Navy`, `Block::PageBreak`; helpers `style::{day_number, short_day_parts, nb, cell, cell_bold}`; colours `code_colors()` in `dood.rs` |
| Reproducibility | document date = `revised_at(c, schedule.id)`, not the clock |

## D. Field inventory

| Visible element | Source | Kind | Empty treatment |
|---|---|---|---|
| Eyebrow "Production planning · Cast commitments" | literal | fixed | — |
| Title "DAY OUT OF DAYS" | literal (`document = "Day Out of Days"`) | fixed | — |
| "{project} · Revised {date}" | `board.project_title`, `revised_at` | conditional (project non-blank) | omit project part |
| Facts: Schedule | `board.schedule.name` | required | — |
| Facts: Production source | `board.source.info.label` | conditional | "No production source" |
| Facts: Displayed range | first/last of `view.days` `.title` ("Shoot Day 1 · Mon 11 Jan 2027") joined with " to "; single day = its title | derived | empty string |
| Facts: Corrections | count of `overridden` cells | derived | "None; every cell comes from the schedule" else "{n} cell(s) entered by hand (marked *)" |
| Stats (4) | `shoot_days`, `view.rows.len()`, `off_days`, `corrected` | derived | singular/plural labels |
| Table title row + hint | "Cast commitments by shooting day" · day-range text · "Columns group i of n" | derived | group text only when n > 1 |
| Legend | `view.legend` (`DoodCode.code/.meaning`) + "Blank: no code for that day" (+ design additions: "Entered by hand (correction)", "Off day") | repeatable | style `simple` prints fewer items |
| Month band | `DoodDay.date` | derived (design addition; year included) | empty cell for an undated day |
| Day header: shoot no. | `day_number(short_label)` → "01"; off day → "OFF" | derived | falls back to upper-cased `short_label` |
| Day header: date | day of month of `DoodDay.date` | conditional | blank |
| Day header: weekday | `short_day_parts` weekday | conditional | blank |
| `#` tile | `DoodRow.number` | required | — |
| Cast label | `DoodRow.actor`, `.character` → "**Actor** (CHARACTER)" or "**Character**" (`label` rule: `actor (character)` or `character`) | required | no actor → character only |
| Code cell | `DoodCell.code` (+ `*` if `overridden`) | conditional | blank cell; off-day blank is shaded |
| Totals Work/Hold/Travel/Other | `DoodRow.work_days/hold_days/travel_days/other_days` | derived (over displayed days) | 0 |
| Totals Start/Finish | `DoodRow.start_label/finish_label` ("Shoot Day 1") printed as "Day 1" | conditional | blank |
| Per-day counts (3 rows) + column sums | **design addition**: counts of work / `H` / `T R F D` codes of the *displayed* cast per day; sums of the totals columns | derived | "0" printed muted |
| Correction details | `cells` with `overridden`: label, `DoodDay.title`, `code`, `derived_code` ("blank" if empty), `note` | repeatable, conditional (only if any) | section omitted |

## E. Content architecture

Order: Masthead (title, facts, stats) → **Cast commitments by shooting day** (one table per *column group*; frozen `#` + cast columns) → Correction details (only if any).

Changes versus the current PDF, and why:
1. **Bands → column groups.** The current document slices days into 7-column bands, each a separate page-set. Here day columns are sized (24–38 pt) so the *whole* schedule fits one group when possible (the 16-column sample: one group, one page); otherwise columns are split into the fewest groups (e.g. 36 columns → 21 + 15) and every group repeats the frozen `#`/cast columns, the legend and the header. Reason: one page per cast list; matrix readable as one object.
2. **Totals moved into the matrix** as a trailing "Totals" block (Work, Hold, Travel, Other, Start, Finish) on the last group, instead of a second table that repeats all names. Same fields, same order, same values.
3. **Per-day count rows** (cast working / on hold / travel-rehearsal-fitting-drop) and column sums added under the cast rows. Derived from the displayed cells only; no new data.
4. **Legend lives in the repeating table header** so it prints on every page (the current continuation pages have none); `*` and "Off day" explained in it.
5. **Metrics become an inline stat row** (same four numbers, same labels) to buy ~30 pt of matrix height. The renderer's `Metrics` block with `compact: true` may be kept instead if a page has room; the numbers are identical.
6. **Cast label**: actor bold, "(CHARACTER)" regular, not whole-label upper-case; row height stays one line unless the name needs two.
7. Month band and year added to the day header (derived from `DoodDay.date`).

Table columns (landscape content width 700 pt):

| Column | Width | Align | Wrap |
|---|---|---|---|
| `#` (tile) | 15 pt | centre | no |
| Cast member | remainder (≥ 136 pt; 141 pt in the sample) | left | wraps, row grows (never ellipsised) |
| Day columns | one fixed width **D** per document, 24–38 pt (sample: 24 pt) | centre | no (codes ≤ 4 glyphs incl. `*`) |
| Totals block (last group only) | Work 24 · Hold 24 · Travel 28 · Other 26 · Start 28 · Finish 30 = 160 pt | centre | no |

Group planning (deterministic; `plan_groups`): choose the fewest groups `k` and a group size `g` such that every non-last group fits `700 − 15 − 136 = 549 pt` at D ≥ 24 and the last group fits `549 − 160 = 389 pt`; then D = min(38, floor(min(549/g, 389/last))). Sample: 16 columns → one group, D = 24.

Sorting: cast order and numbering exactly as the source (first work day, then character name); days in schedule order including off days.

## F. Visual specification

- **Page**: US Letter landscape 792×612 pt, margins 46 pt left/right/top, 66 pt bottom. Footer from the shared system (logo lockup 17 pt high, "Created using OpenFrame", "Page X of Y") — logo must be the embedded PNG, not the redrawn mark (implementation requirement).
- **Running header**: yes — `@top-right "LOW TIDE RADIO · DAY OUT OF DAYS"` (6.8 pt, no rule, no logo). Reason: a multi-page matrix printed loose needs page identity; verified present on every page (`verify_pdf.py --header`). Single-purpose document identity stays in the masthead.
- **Type** (Helvetica): title 20 pt bold navy; eyebrow 7.6 pt tracked; facts 7.6 pt; stat values 13 pt bold navy / labels 6.6 pt tracked caps; table title 10 pt tracked caps; legend 7 pt (codes 7 pt bold); month band 6.8 pt bold tracked; header shoot no. 7.2 pt bold, date 10 pt bold, weekday 6.6 pt bold; cast 7.6 pt (actor bold navy); code cells 8.2 pt bold; counts 7.8 pt bold; totals 8 pt bold (Start/Finish 7.2 pt).
- **Row height** 16 pt min (cast rows); count rows 13 pt; header 36 pt + month band 11 pt.
- **Code cell encoding** (text + shape + weight + colour; colour is last):

| Code | Fill / text | Shape cue (survives grayscale) |
|---|---|---|
| SW | `#1d78b0` / white | 3 pt dark cap on the **left** edge |
| W | `#b9e0ee` / `#1b4f66` | plain light fill |
| WF | `#62cf9a` / `#14472f` | 3 pt dark cap on the **right** edge |
| SWF | `#16809f` / white | caps on **both** edges |
| H | `#ffd199` / `#7a4300` | hollow 1 pt outline box |
| T / R / F / D | `#d5c4f2` / `#f7e07a` / `#f4b6d0` / `#eab0b0` with source text colours | letter only (distinct letters) |
| - | `#eceff3`, slate, regular weight | — |
| blank | white | — |
| off-day column | `#e9edf3` (header `#dfe4ec`, label "OFF") | shaded column |
| **corrected** | any fill + `*` suffix | 1.4 pt ink outline inside the cell |

  Fills are the source's `code_colors()` values (retained). Caps are 3 pt rects inset in the cell; the outline is a stroked rect — both plain rect primitives.
- **Weekend / off days** (configurable): off day = shaded column + "OFF" label (source `is_off_day`). Weekend shooting days = weekday label in a filled slate chip (SAT / SUN). *Weekend = Sat + Sun is a design default, not a source concept; make the set a setting (some regions use Fri–Sat) or disable.*
- **Reading guides**: 0.5 pt row hairlines; a 0.75 pt darker line every 5th cast row (configurable, 0 = off).
- **Counts block**: tint `--of-panel`, first count row has a 1 pt navy top rule; totals sums in `--of-tint`.
- **Status / grayscale**: verified in `--gray` renders: SW/SWF dark, WF mid with right cap, W light, H outlined, corrected outlined with `*`; every code is also printed as text.
- **Colour tokens** from `openframe-print.css` (`--of-navy`, `--of-slate`, `--of-tint`, `--of-line`, …); no gradients/shadows other than flat inset rects.

## G. Dynamic behaviour

- **Many cast** (40+): rows paginate; the header (title, legend, month band, day header) repeats on every page; the last two cast rows travel with the three count rows (no orphan counts); the first two rows travel with the header.
- **Many days** (30+): new column group = new page; every group repeats `#` + cast and the legend; only the last group carries the totals block; "Columns group i of n" in the title row.
- **Group order** (configurable): default "complete one column group for all cast, then the next group". Alternative "tile order" (rows chunk × column group) suits wall-taping; not rendered in this reference.
- **Long cast labels**: wrap inside the cast column; row grows (see 54-character character name in the stress test).
- **Undated day**: month cell empty, date and weekday blank, header still shows the shoot number.
- **Off day with a correction**: the code cell keeps the shaded column fill only when blank; a corrected code uses its code fill and outline.
- **No corrections**: "Correction details" omitted, Corrections fact reads "None; every cell comes from the schedule".
- **No actor**: label = bold character only. **Range/cast filter**: only the chosen rows/days print; counts and Work/Hold/Travel/Other cover *displayed* days; Start/Finish are whole-schedule (state this in the totals header hint if a range is active).
- **Simple code style**: only the legend items `W - H T R F D`; `-` cells shaded `#eceff3`.

## H. Pagination and print rules

- Landscape Letter, margins as above, content height 500 pt.
- Table header = 4 rows (title row, legend row, month band, day header) inside `<thead>` → repeats on every page of a group.
- Rows `break-inside: avoid`; first 2 rows and last 2 rows + counts are `break-inside: avoid` groups; a column group always starts on a new page; Correction details starts after the last group (`tr.title` row keeps its name across pages).
- Minimum rows per page after the header ≥ 2; no heading separated from its table.
- Footer drawn identically on every page; page numbers real (`Page X of Y`).

## I. HTML / PDF correspondence

- `Day_Out_of_Days.html` is the same content as the PDF. Real data: the supplied PDF's schedule (14 shoot days + 2 off days, 17 cast, 3 corrections); the cell grid was extracted from the PDF and the Work/Hold/Travel/Other/Start/Finish of all 17 rows re-derived by the Python port of `derive_codes` match the supplied totals table (asserted in the generator).
- Toggles (preview bar): *Industry codes (SW, W, WF, H)* ↔ *Simple codes (W and -)* (mutually exclusive; simple codes re-derived from the same work/off data). The PDF captured the **industry** style.
- Stress test `Day_Out_of_Days_STRESS_TEST` (fictional): 42 cast × 36 day columns (30 shoot + 6 off), one undated day, 54-character character name, cast without actor, 13 corrections with a ~190-character note → 5 pages (2 column groups × 2 pages + corrections), counts and totals verified visually.
- Render: Chrome 155 headless, `tools/render_pdf.py`; `tools/verify_pdf.py --size landscape --header "DAY OUT OF DAYS"` passes on both PDFs (2 and 5 pages). Chrome substitutes Arial for Helvetica (metrics are near-identical, not byte-identical).
- Known differences vs the Rust renderer: month band with partial colspans, cap/outline rects, multi-row repeating header and keep-together row groups are new layout features (see K).

## J. Acceptance criteria

1. Landscape Letter; footer + `Page X of Y` on every page; running header text on every page.
2. Day columns have one width in every group; frozen `#` + cast columns repeated at the left of every group; no day cell is split by a page boundary.
3. Every page shows the legend; legend text equals `CODES` meanings; "Blank: no code for that day" present.
4. SW/WF/SWF are distinguishable from W in a grayscale render by the edge caps; H by the outline; corrected cells carry `*` and an outline.
5. Off-day columns are shaded and labelled "OFF"; weekend chips appear on SAT/SUN shooting days only.
6. For every row: Work/Hold/Travel/Other equal the number of cells with SW+W+WF+SWF / H / T / R+F+D in the *displayed* days; Start/Finish = first/last work day in the whole schedule.
7. Per-day "Cast working" counts equal the number of work-code cells in that column; the Work column sum equals the sum of the row counts.
8. A 40+ cast / 30+ day input paginates with repeated headers, no orphan count rows, no clipped cast labels.
9. Correction details lists every `*` cell with the schedule's code ("blank" when empty) and the note; omitted when none.
10. Same data → byte-identical output (date = last schedule change).
11. Simple style prints only its legend items and `-`/`W` cells.

## K. Known issues and unresolved questions

**Verified defects in the current output**: items 1–8 in B.1.

**Design choices (configurable, no standard claimed)**: weekend = Sat/Sun chip; reading-guide interval (5); count rows' definition (work / H / T-R-F-D); column-group order (down-then-over default); totals block on the last group only; Start/Finish shortened to "Day N" (data identical to "Shoot Day N"); metrics as inline stats; month band with year.

**Unverified assumptions**
- Chrome's page breaks stand in for the renderer's measured layout; the Rust renderer needs: column-group layout with frozen columns, a multi-row repeating header, partial-colspan header cells, tbody keep-together, rect-edge caps/outline inside a cell, and chip text on a header cell. None of these exist in `StyledTable` today (it has one `header_cells` row and `Row::spanning` for full-row spans).
- Fit of the 16-column sample on one page depends on D = 24 pt; `SWF*` at 8.2 pt bold is ≈ 22 pt wide in Arial — measure in the real renderer before locking D_MIN.
- The Mac app was not compared.
- The `Cast totals` sentence in the current PDF is wrong for filtered ranges (source-verified); whether that is intended is unresolved. This design prints the corrected scoping in the data, not a new sentence.

**Promote to shared CSS (suggestions)**: `.kc` code swatch; `tr.title` + legend-in-thead pattern; `.m` matrix table with frozen columns and month band; `.cnt` count rows; `.stats` inline stat row; `.chip.navy` weekday chip.
