# Shooting Schedule (stripboard) — PDF design handoff (v2: PORTRAIT)

| | |
|---|---|
| Export | Shooting Schedule — `schedule.export_schedule` |
| Variant | PDF, **portrait US Letter (v2, user decision; v1 was landscape)**, "Entire schedule", unscheduled scenes on, scene notes off (the state captured in `Shooting_Schedule.pdf`) |
| Status | **Design reference — not integrated; not validated against OpenFrame's Rust renderer** |
| Repo commit | `2f5a045416bd2125801329533fc47f986ccc8174` (`main`) |
| Date | 2026-10-23 |
| Files | `Shooting_Schedule.html` / `.pdf` (13 pages; v1 landscape 14) · `stress-test/Shooting_Schedule_STRESS_TEST.html` / `.pdf` (10 pages) · generator `tools/build/scheduling/` |
| Baseline | `exports/Shooting Schedule - full schedule.pdf` (15 pages, **landscape**, same Low Tide Radio data) |
| Package | `OpenFrame_PDF_Design_Package_v2/` (all paths in this file are relative to it) |

---

## A. Purpose and users

The Shooting Schedule is the printed stripboard of the whole shoot: every shooting day in order, each scene as a colour-coded strip, with the breaks (meals, company moves, travel) between them, then the scenes nobody has scheduled yet.

* **1st AD / scheduler** — checks the day order, whether a day is over target, whether a strip changed since the script changed, what is still unscheduled. Must find: day number + date, crew call, estimated wrap against target, INT/EXT + DAY/NIGHT per strip, where the company moves are.
* **Production manager / line producer** — totals per day (scenes, pages, over-target flag), locations per day, call-sheet status per day.
* **Department heads / cast (printed, pinned up)** — which scenes on which day, who is in each scene, start times.
* **Printing reality** — stripboards are often printed in black and white; the strip colours cannot be the only carrier of meaning.

## B. Verified functionality

**Scope** (UI `ScheduleExportButton`, `features/export/buttons.tsx`): radio "Entire schedule" · "Current day only (<day title>)" · "Selected days" (day picker; empty text "Create a shooting day first."). Selected days keep schedule order (`schedule.rs`, `board.days.iter().filter(...)`).

**Options** (checkbox → arg key): "Include unscheduled scenes" → `includeUnscheduled`, default **on**; "Include scene notes" → `includeNotes`, default **off**. Note under the dialog: "Private notes excluded. Export is a snapshot of the schedule; the schedule is not changed." (Server-side default when the key is absent: `includeUnscheduled = dayIds is None`; the UI always sends the checkbox value.)

**Formats**: PDF, Excel (XLSX), CSV. This design covers PDF only. XLSX = two sheets, "Schedule" (one row per scene/break; columns Day, Date, Order, Start, Item, Scene, I/E, D/N, Heading, Location, Synopsis, Cast, Pages, Est. time, + Notes when enabled) and "Days" (Day, Date, Scenes, Pages, Estimated, Crew call, Shooting start, Target wrap, Estimated wrap, Locations, Cast, Call sheet, Day notes). **CSV = the first table only (the "Schedule" sheet)** (`exports/mod.rs`, `Fmt::Csv` → `out.tables.first()`).

**Orientation**: the current builder forces landscape: `Document::new(document).landscape(true)` (`schedule.rs`, "PDF (landscape stripboard)"). **v2 asks for portrait: remove `.landscape(true)`** (the default is portrait, `report.rs` `landscape: false`) and use the portrait column widths in E. The baseline PDF (15 pages) is landscape.

**Args**: `format`, `path`, `dayIds?`, `includeUnscheduled?`, `includeNotes?` (`ExportScheduleArgs`, `deny_unknown_fields`).

**Entry points (both checked)**
1. `workspaces/production/tabs/schedule.tsx:220` — toolbar of the Schedule page (Board / List / Calendar). Passes `days` and `sourceLabel` but **no `currentDayId`**, so "Current day only" is disabled here.
2. `workspaces/call-sheets/shared/ScheduleViews.tsx:273` — the day-detail view (UI capture `583-prod-schedule-day-export-dialog`), bottom row next to "Open Call Sheet". Passes `currentDayId`, so "Current day only (Shoot Day 1 · Mon 9 Nov 2026)" is enabled. Suggested file name in the dialog: `<Project> - Shooting Schedule - <date>.pdf`.

**Data dependencies**: a current shooting schedule (`current_schedule`), its production source (scenes, cast, locations), days, strips, markers, call sheets. **Errors**: no schedule → "Create a shooting schedule first."; `dayIds: []` → "Select at least one shooting day to export."; unknown day → not found; no days and empty pool → "The schedule has no shooting days yet." (`nothing_to_export`). Output is reproducible (document carries the schedule's last-change date, not the export clock).

**Strip colouring — verified** (`schedule.rs::strip_palette`, `board.rs::strip_dto`): class = `x` if `int_ext` is `EXT` **or `INT/EXT`**, else `i`; then `n` if `day_night == "N"` else `d`. Colours (owner decision 2026-10-20 in the source comment): `xd` EXT+Day yellow fill `#fbe98f` / stripe `#d9a400`; `xn` EXT+Night green `#c9e7b8` / `#4c9a3a`; `id` INT+Day white `#ffffff` / `#a9b6c6`; `in` INT+Night light blue `#cfe4f6` / `#4a8cc4`. Ink is always navy `#1a2640`; muted per-class text colours `#5a4a1f`, `#2f5a25`, `#566277`, `#2f4d6b`. The source already prints the meaning as two text chips (`INT`/`EXT`/`INT/EXT` and `Day`/`Night`) in the Type column, so colour is reinforcement only. The scene's day/night field is already normalised (the baseline shows scene 39 "…DUSK" as NIGHT and scenes 5/27/44 "…DAWN" as DAY).

### B.1 Current-output assessment (baseline PDF, 15 pages)

**Retain**
* Stripboard (landscape today) with one table per day; day band (navy) + tinted sub-line + day notes + strip table.
* The four strip colours with navy ink, left stripe and the INT/EXT + DAY/NIGHT text chips.
* Column set Scene / Type / Start / Heading & synopsis / Cast / Pages / Est.; "Start" and "Est." only when any value exists.
* Breaks on the same grid in neutral grey rows; "MANUAL" tag under hand-entered page counts; "Changed / New / Removed" chips; "Day notes:"; off-day band; exact empty sentences ("No scenes scheduled.", "All source scenes are scheduled.").
* Production order, never re-sorted.

**Improve visually**
* Sub-line (p10–12) packs five facts into one 8.6 pt line; hard to scan. Redesigned as a tinted facts band with the key times in bold.
* "SHOOT DAY 05 | Day 5" says the same thing twice; merged to "Shoot Day 5".
* "COMPANY MOVE" chip wraps to two lines in a 64 pt Type column (p5, p10, p12, p14).
* Pages numeral at 11 pt dominates the row; "MANUAL" on every strip is visual noise in this project (every strip is hand-entered).
* Strip legend sits at the end of the last day (p14) instead of before the first strip.

**Verified functional / layout problems**
1. **p15 is an almost empty page**: only "UNSCHEDULED · 0 / All source scenes are scheduled." (verified p15). Cause: the Unscheduled section starts after the last day and the last page cannot hold it with the legend.
2. **Cast names wrap into the Pages column area** in a 150 pt cast column: "STUDIO PRODUCER (Karan Shetty)" wraps to two lines (p12 scene 3, p14 scene 43); in the text layer the second line collides with the "MANUAL" tag line.
3. **Legend is orphaned at the foot of p14** and again a whole page (p15) follows.
4. **Page use**: a 3-strip day cannot share a page with the next day, so 14 days + 2 off days fill 15 pages with a third to a half of the page empty on several (visibly p1, p8, p13).
5. **Break rows are not in time order** (p5: lunch 13:30 and a 18:00 move listed after a 16:45 scene; p10: 11:00 move after a 13:30 scene). The export prints position order (`board.day_items` sorts by `(position, strip-before-marker, id)`), so this reflects the data's positions, not a print bug. Related, derived from `timeline.rs::compute` (inferred, not run): a pinned break placed after the last scene moves the running clock backwards, so Day 1 prints "wrap about 14:00" although scene 29 (12:00 + 2h 30m) ends 14:30.
6. A break time can print as "15:30 (+1)" (p13, travel row) because a pinned time earlier than the shooting start is read as after midnight (`timeline::after`); correct per the source rules but surprising on a printed page.
7. Day banner title "Day 5" + eyebrow "SHOOT DAY 05" duplicate each other (p2–p14).

**Cannot be determined from the evidence**
* How the Rust renderer lays out a day taller than a page (no baseline day is that tall; `StyledTable.continued` code shows a "(CONTINUED)" label bar and repeated table header, see `report/design/table.rs`).
* Output for "Current day only" / "Selected days" (same builder; only the metrics and day list change).
* What an XLSX/CSV looks like (not in scope, no samples).
* Whether `includeNotes` strips ever carry long notes in practice (none in the sample; layout shown only in the stress test).

## C. Source mapping

| Item | Value |
|---|---|
| Op | `schedule.export_schedule` |
| Builder | `windows/crates/openframe-application/src/modules/exports/schedule.rs::export` (args `ExportScheduleArgs`) |
| Data | `schedule::board::{Board, DayRow, StripRow, MarkerRow, Item}`, `Board::strip_dto`, `day_summary`, `day_timeline`, `call_sheet_brief`, `revised_at`; `timeline::ScheduleDayTimeline`; `source::ScheduleCastRef/ScheduleLocRef` |
| Renderer blocks | `Document::new(..).footer(footer(project, document))` (v1/current: `.landscape(true)`; v2: portrait, call dropped), `Masthead` (+ `Side::Badge`), `Metrics`, `Banner` (`keep_next = 110.0`), `Block::Text`, `StyledTable` (`boxed`, `continued`) with `Row.fill/stripe`, `Cell`/`CellItem::{Tile, Chips, Text}`, `Legend`, `Section` |
| UI | `features/export/buttons.tsx::ScheduleExportButton` mounted in `workspaces/production/tabs/schedule.tsx` and `workspaces/call-sheets/shared/ScheduleViews.tsx` |
| Footer text today | `footer()` = "<Project> · Shooting Schedule" right side + "Page X / N" (current renderer redraws a vector mark; the design requires the PNG logo lockup, see F) |

## D. Field inventory

| Element | Source | Kind | Empty treatment |
|---|---|---|---|
| Eyebrow "Production planning / Schedule" | literal | fixed | — |
| Title "SHOOTING SCHEDULE" | literal | fixed | — |
| Project subtitle | `board.project_title` (`nb`) | conditional | omitted |
| "Production source: … · Schedule: …" | `board.source.info.label` (else "No production source"), `board.schedule.name` | derived | "No production source" |
| Badge "Schedule snapshot", "Revised 19 Oct 2026", "Project schedule · PDF export" | literal + `revised_at` (`date_from_ms`) | derived | — |
| Metrics: shooting days / scheduled scenes / script pages | count of non-off days in scope / strips on scope days / sum `day_summary.page_eighths` (zero-padded `{:02}` for the two counts) | derived | "00" |
| Metric "Unscheduled scene(s)" | pool length | conditional (only when unscheduled included) | "00" |
| Strip legend | `strip_palette` | fixed | — |
| Day band title | `DayRow.label()` (`Shoot Day N`; off day "Off day") | derived | — |
| Day band date | `DayRow.date` → `long_date` → "MONDAY · 11 JANUARY 2027" | conditional | omitted |
| Facts: "N scenes, M other items · arranged in production order" | `sum.scene_count`, markers on day | derived | "other items" part omitted when 0 |
| Facts: "<pages> pages total" | `pages(sum.page_eighths)` ("page total" when "1") | derived | prints "0/8" for an empty day (`fmt::pages(0)`, inferred) |
| Facts: crew call / wrap about / target | `tl.summary` parts (`crew_call`, `estimated_wrap`, `target_wrap`); only when `tl.anchored` | conditional | omitted |
| Facts: "over the 11h target" / "past the target wrap" | `sum.over_target`, `tl.over_target_wrap`, `sum.target_minutes` | conditional | omitted |
| Facts: "Call sheet: <status>" | `call_sheet_brief().status` (Draft / Needs refresh / Final) | conditional | omitted |
| **Facts: Locations** (new line) | `day_summary.locations[].name` (already in the XLSX "Days" sheet) | conditional | omitted |
| "Day notes" | `DayRow.notes` (`nonblank`) | conditional | omitted |
| Empty day | — | exact text | "No scenes scheduled." |
| Off day | `DayRow.off` | conditional | "No shooting" + day notes |
| Scene tile | `dto.number` | required | "-" |
| Type chips | `dto.int_ext`, `dto.day_night` ("Day"/"Night"/as-is) | conditional | chip omitted when blank |
| Start | `timeline.rows[].start_label` ("08:00", "05:50 (+1)") | conditional column | "–" in the cell; column hidden when no item has one |
| Heading | `dto.heading` uppercased; "(no longer in the script)" when `missing_from_source` | required | — |
| Location line | `dto.locations[].name` joined ", "; omitted when the heading already contains it | conditional | omitted |
| Synopsis | `full_synopses` (complete, not the 90-char strip text) | conditional | omitted |
| Scene note | `dto.notes` ("Note: …"), only with `includeNotes` | conditional | omitted |
| State chip | `dto.source_state` Changed / New / Removed → "Changed in the script" / "New in the script" / "Removed from the script" | conditional | omitted when Current |
| Cast | `dto.cast` → "CHARACTER (Actor)" or "CHARACTER"; one per line | repeatable | empty cell |
| Pages / MANUAL | `dto.pages_label`; `dto.pages_overridden` | required / conditional | — |
| Est. | `dto.estimated_minutes` via `minutes()` | conditional column | "–" |
| Break row | `MarkerRow`: `marker_type` chip, `label` uppercased, `at_time`, `duration_minutes`, `notes` | repeatable | start "–" when unknown |
| Unscheduled section | `board.unscheduled()` sorted by scene number | conditional (`includeUnscheduled`) | "All source scenes are scheduled." |

**Verified absent from this export** (no field, so not designed): numeric cast IDs (`ScheduleCastRef` has `key`, `character`, `actor`, `initials`; the PDF prints "Character (Actor)"; the numeric IDs 11/14/7 belong to the Call Sheet), pinned-time marking, scene location addresses, scope label (only written to the job log), department/prop info, per-day total estimate (exists in `DaySummary.estimated_minutes` and the XLSX but is not printed).

## E. Content architecture

Order (unchanged from the source except item 3): Masthead → metrics → **strip legend (moved here)** → one block per day in schedule order (off days interleaved) → Unscheduled section.

Day block: band → facts → column header → day-notes row → strips and breaks in position order.

Table columns (**portrait content width 520 pt**, real size; v1 landscape widths in brackets):

| Column | Width | Align | Wrap |
|---|---|---|---|
| Scene | 38 pt [46] | left | tile, no wrap |
| Type | 48 pt [54] | left | chips stacked |
| Start (optional) | 38 pt [46] | left | "(+1)" second line |
| Heading & synopsis | flex: **196 pt** with all columns (288 in v1); 236 without Start; 234 without Est.; 274 without Start and Est. | left | heading, location, synopsis, note wrap; row grows |
| Cast | 124 pt [176] | left | one entry per line; an entry wider than 117 pt wraps to a second line |
| Pages | 36 pt [44] | right | no wrap, MANUAL below |
| Est. (optional) | 40 pt [46] | right | no wrap |

Cell padding 3.5 pt left/right (6 pt in v1). No column is dropped or merged; column order is unchanged. Widest cast entries measured with Helvetica metrics at 8.2 pt: STUDIO PRODUCER (Karan Shetty) 135 pt, PORT OFFICER (Ramesh Kaimal) 127 pt, YOUNG ANJALI (Ameya Suresh) 122 pt, so those three wrap at 124 pt (see K).

Changes versus the current PDF, and why
1. **Band title**: "Shoot Day 5" only (the eyebrow duplicated it). No information lost.
2. **Facts band** replaces the one-line sub-line: left "N scenes, M other items · arranged in production order", right the same facts in the same words with the key values (pages, crew call, wrap) in bold; "over the …" is an amber pill with its text (grayscale-safe). Same strings as the source (`tl.summary`, `over target` wording).
3. **Legend moved from after the last day to under the metrics**, with the text codes (EXT · DAY …) and the statement "INT/EXT counts as exterior". A key is needed before the first strip, and it ended up orphaned on p14. In portrait the legend wraps onto two lines.
4. **Locations line added** to the facts band from `day_summary.locations` (already exported in XLSX "Days"). Rationale: location is the first thing a 1st AD reads for a day; it is the only addition. Easy to drop (one row).
5. **Day notes** become the first row of the day table (orange stripe, "DAY NOTES" label) instead of a loose line under the sub-line, so the band, facts, header and notes read as one block. Not repeated on continuation pages.
6. **Break rows**: the type chip spans the Scene + Type columns, so "COMPANY MOVE" is on one line. Same data, same grey row.
7. **Column widths** (current PDF → v2 portrait): Scene 48 → 38, Type 64 → 48, Start 50 → 38, Cast 150 → 124, Pages 56 → 36, Est. 52 → 40 pt; the heading column is whatever remains (196 pt). The landscape v1 design had Cast 176.
8. **Running header** added (F).
9. **Pagination rules** (H): a whole day is kept together when it fits one page; the empty-state Unscheduled sentence stays on the last day's page.
10. **v2: portrait** (user decision), see the subsection below.

### Changes from v1 (user decision: portrait)

The user looked at the landscape strips scaled onto a portrait page and found them readable, and asked for the structure to stay as it is. v2 is **not** a scaled copy: the same strip structure is re-laid at real size in the 520 pt portrait content area.

* Orientation: US Letter portrait 612 × 792 pt (content 520 × 680 pt) instead of landscape. In the Rust builder: drop `.landscape(true)`.
* Column widths reallocated (table above); same seven columns, same order, same content, same wording.
* Type floor kept: body/cells at least 8.0 pt, chips and labels at least 6.4 pt. Against v1 the heading went 9.2 to 8.8, synopsis and cast 8.4 to 8.2, pages 9.6 to 8.8, start 9 to 8.6, band title 14 to 13 and band date 9 to 8.4; the location line went UP (7.8 to 8.0) and the MANUAL tag went UP (5.8 to 6.4). v1's smallest text was 5.8 pt; v2's smallest is 6.4 pt (MANUAL), chips 6.6 pt.
* The facts band wraps to two lines (counts, then totals and times), the Locations line stays third; metrics boxes show value above label; the legend wraps to two lines.
* Scene notes stay under the description (unchanged).
* Kept: day banner, locations line, metrics and legend, strip colours and text codes, day-notes row, break and company-move rows, Unscheduled section, running header (fits inside the 46 pt top margin), keep-whole-day rule, header repeat, no lone last row, empty-state sentences.
* Result: 13 pages (v1: 14). The only page saved is Days 13 and 14 sharing the last page; every other day still has a page to itself (see H).

## F. Visual specification

* Page: US Letter **portrait 612 × 792 pt** (v1: landscape 792 × 612); margins 46 pt left/right/top, 66 pt bottom (shared system); content box 520 × 680 pt.
* Type (Helvetica): band title 13 pt bold white; band date 8.4 pt, tracking 0.04 em, uppercase, white; facts 8.2 pt (values bold), locations 8 pt `--of-slate` with a 6.6 pt tracked label; column header 6.8 pt bold caps, tracking 0.06 em, `--of-slate`; heading 8.8 pt bold uppercase navy; location 8 pt bold muted; synopsis 8.2 pt muted; cast 8.2 pt navy; pages 8.8 pt (MANUAL 6.4 pt bold tracked); est. 8.2 pt; start 8.6 pt bold with "(+1)" 6.6 pt; chips 6.6 pt; note 8 pt bold; legend text 7.4 pt. **Measured minimum in the content area: 6.4 pt (MANUAL); chips 6.6 pt; smallest body/cell text 8.0 pt** (`tools/build/scheduling/measure.py`, PyMuPDF span sizes).
* Day band: `--of-navy`, radius 7 pt top corners, padding 7 / 10 pt. Off day band `#56627a`. Facts band `--of-tint`. Column header `--of-tint` with hairline top.
* Strips: row fill + 3 pt left stripe per class (E/B above); 0.5 pt `--of-line-strong` row separators; table outer hairline; bottom corners radius 4 pt. Scene tile white with hairline border, 8.4 pt bold. Breaks: fill `#f1f3f6`, stripe `--of-faint`, white chip.
* **Status treatments**: Changed = amber chip + text "Changed in the script"; New = blue chip "New in the script"; Removed = red chip "Removed from the script" + heading suffix "(NO LONGER IN THE SCRIPT)"; over target = amber pill with text; Manual pages = text tag. None depends on colour alone.
* **Grayscale**: yellow, green and blue strips become near-identical greys (viewed in `--gray` thumbnails). The distinction therefore lives in the INT/EXT and DAY/NIGHT chips on every strip, plus the legend lists the text code next to each swatch. A scheduler reading a B/W print can still tell every strip type.
* Footer: shared system (logo PNG 17 pt high lockup, "Created using OpenFrame", "Page X of Y"). **Implementation requirement**: the Rust renderer must embed the PNG logo (it currently redraws a vector mark and prints "<Project> · Shooting Schedule · Page X / N").
* **Running header decision: YES** — `LOW TIDE RADIO · SHOOTING SCHEDULE · REVISED 19 OCT 2026`, top-right, 6.8 pt, no rule, no logo, on every page. Reason: a 13-page document that is pinned and handed out page by page; every page then names the production and the revision (stale copies are common on set). The revision is the schedule's `revised_at`. Shown on page 1 too (page 1 also has the masthead). It sits in the 46 pt top margin box (24 pt padding-top, 6.8 pt text); `verify_pdf.py --header` finds it on all 13 pages and no content collides with it.

## G. Dynamic behaviour

* **Start / Est. columns** disappear when no item in that day has a value; the heading column takes the width (stress p4). Break labels then carry "AT 14:30" / "(30m)" as in the source.
* **Long headings / locations / synopses** wrap, rows grow (stress p2). **Very wide cast**: one name per line, row grows (stress p5, 16 entries, most on two lines, a ~330 pt row); a row is never split. Unresolved: a single row taller than a page (not realistic).
* **Missing number** → tile "-". **No cast / no location / no synopsis** → cell or line omitted.
* **Changed / New / Removed** chips under the synopsis; Removed also appends "(no longer in the script)".
* **Scene notes** (`includeNotes`): "Note: …" bold line under the synopsis; off by default and hidden in the PDF (toggle in HTML).
* **Empty day**: band + facts + a single row "No scenes scheduled." (no column header). **Off day**: grey band, "No shooting", day notes in a bordered box.
* **Day taller than a page**: splits between strips; band + facts + column header repeat on each page; day notes do not (stress p1–3). The production renderer should keep its "(CONTINUED)" marker (`StyledTable.continued`) for this case; the HTML repeats the full band instead (see I).
* **Unscheduled**: section title "Unscheduled · N" (+ hint "Scenes not on a shooting day"), same strip rows without Start; may split anywhere with the header repeating. Empty: the exact sentence.
* **Scope**: Current day / Selected days produce the same blocks for fewer days; metrics recount; the Unscheduled metric and section are absent when the option is off (stress p10).

## H. Pagination and print rules

* **Portrait** US Letter; header/footer as above; content box 520 × 680 pt (v1: 700 × 500).
* Masthead + metrics + legend stay on page 1; the first day may start below them.
* A day block (band + facts + header + rows) is **kept whole** if it fits on one page, otherwise moved to the next page (today's behaviour: `Banner.keep_next = 110`). A day that cannot fit one page may start where the previous content ends and splits **only between rows**.
* A strip/break row never splits. The day band is never left at the bottom: at least the column header and the first two rows must fit with it (`keep_next ≥ band + facts + header + 2 rows`, replacing the fixed 110 pt).
* No lone last row: if a split would leave one row on the next page, move the previous row with it (guard exercised in v1 landscape stress p6 to p7; in v2 it applies whenever a split would leave one row).
* Column header repeats on every page of a day; Unscheduled likewise.
* Off days are small blocks kept whole and flow after the preceding day when there is room (Day 6 + Off day on p6, Day 12 + Off day on p12 in the PDF).
* **v2 pagination numbers (primary PDF)**: 13 pages; page fill per `verify_pdf.py`: 62 52 49 66 62 66 58 48 60 64 56 78 98 %. One day per page for Days 1 to 12 (a typical day is 300 to 450 pt tall, so two never fit in 680 pt); Days 13 and 14 share page 13 together with the "Unscheduled · 0" sentence (98 % full: if it did not fit, the sentence would fall alone onto a 14th page, the v1 p15 defect, so keep the Unscheduled heading and sentence with the last day where possible). No day splits in the primary PDF; the stress test splits a 16-strip day over 3 pages.
* "Page X of Y" are real values (verified by `verify_pdf.py`).

## I. HTML / PDF correspondence

* `Shooting_Schedule.html` (v2, portrait: `@page { size: 8.5in 11in }`, `<div class="doc">` without `landscape`) is the full Low Tide Radio schedule (14 shoot days, 2 off days, 44 strips, 57 5/8 pages) transcribed from the baseline and machine-checked: every heading, synopsis, cast entry, location, break label and day note was matched against the baseline text (0 missing).
* **PDF captured state**: Entire schedule · unscheduled on (0 unscheduled → empty sentence) · scene notes off.
* **Toggles** (preview toolbar): "Include scene notes" (three illustrative notes, marked "(illustrative, fictional)"); "Unscheduled scenes (illustrative)" (swaps the empty sentence for two fictional strips, metric → 02).
* Rendered with `tools/render_pdf.py` (headless Chrome 155); verified with `verify_pdf.py --size letter --header "LOW TIDE RADIO · SHOOTING SCHEDULE · REVISED 19 OCT 2026"`.
* **Stress test** (`stress-test/`): fictional rows on Low Tide Radio names, banner on page 1 says so. Contains a 16-strip day with a long day note, long heading/location, Changed/New/Removed strips, a scene note and a break with a note, "past the target wrap"; a day with no times (columns collapse); a 16-name cast and an unnamed location; an empty day; an off day with a long note; five filler days that push the week over many pages; an Unscheduled list of 6 including Removed (number "-") and a long heading; and a final "Current day only" scope variant using real Shoot Day 10 (metrics without the Unscheduled item).
* **Known differences vs the Rust renderer**: (1) Chrome repeats the whole band + facts on a continuation page, the renderer would draw the compact "(CONTINUED)" bar; (2) Chrome paints each cell's fill separately, leaving faint hairline seams between columns on coloured rows (zoom 220 %); the renderer draws one rectangle per row; (3) `break-before: avoid` on the last row and `break-inside: avoid` on days approximate rules the renderer must implement as measured heights; (4) the running header is CSS `@page` content.

## J. Acceptance criteria

- [ ] Page is US Letter **portrait** 612 × 792 pt; margins 46/46/46/66 pt; nothing outside them (`verify_pdf.py --size letter` passes). The builder no longer calls `.landscape(true)`.
- [ ] Column widths at 520 pt: Scene 38, Type 48, Start 38, Cast 124, Pages 36, Est. 40, heading = remainder (196 pt with all columns); order Scene, Type, Start, Heading & synopsis, Cast, Pages, Est.
- [ ] No text in the content area smaller than 6.4 pt; chips and labels at least 6.6 pt except the "MANUAL" tag (6.4); body and cell text at least 8.0 pt.
- [ ] The running header sits inside the top margin and does not touch content on any page.
- [ ] Every page has the PNG logo lockup, "Created using OpenFrame" and "Page X of Y" with the true total; running header text present on every page.
- [ ] Each day prints band "Shoot Day N" + uppercase weekday · date; off days "Off day" with "No shooting".
- [ ] Facts band wording equals the source: "N scenes, M other items · arranged in production order", "<pages> pages total", "<HH:MM> crew call", "wrap about HH:MM (target HH:MM)", "over the 11h target" or "past the target wrap", "Call sheet: <status>"; omitted parts leave no separator.
- [ ] Strip fill/stripe follow the four classes; INT/EXT counts as exterior; every strip also shows the INT/EXT and DAY/NIGHT chips (grayscale thumbnails remain distinguishable).
- [ ] Strips and breaks keep position order; break rows grey with the type chip on one line; `Start` column appears only when some item has a start; same for `Est.`.
- [ ] Cast shows one "CHARACTER (Actor)" per line; a 14-entry cast makes the row taller, never clipped or overlapping Pages.
- [ ] "MANUAL" appears only when the pages value was entered by hand.
- [ ] Changed / New / Removed chips appear with their exact texts; Removed adds "(no longer in the script)".
- [ ] A day taller than a page repeats band, facts and column header on every page; the day-notes row prints once.
- [ ] A day band never sits at the foot of a page without its header and two rows; no single-row last page for a day.
- [ ] Empty day shows "No scenes scheduled." and no column header; empty pool shows "All source scenes are scheduled." on the last day's page (no page of its own).
- [ ] Unscheduled metric and section are absent when "Include unscheduled scenes" is off.
- [ ] Two exports of the same data are byte-identical (no clock in the layout).

## K. Known issues and unresolved questions

**Verified defects in the current output**: see B.1 items 1–7 (almost-empty p15; wrapping cast names; orphaned legend; poor page fill; non-chronological break rows are data; "(+1)" quirk; duplicated day title).

**Portrait at real size vs landscape: measured comparison (primary data, 44 strips)**

| | v1 landscape (14 p) | v2 portrait (13 p) |
|---|---|---|
| Heading column (all columns present) | 288 pt | 196 pt |
| Cast column | 176 pt | 124 pt |
| Headings wrapping to 2 lines (never 3) | 2 | 14 |
| Synopses on 3+ lines | 0 | 4 (max 3 lines) |
| Cast entries wrapping (of 145) | 0 | 6 (STUDIO PRODUCER x2, PORT OFFICER x2, YOUNG ANJALI x2) |
| Strips with 3+ cast lines | 30 of 44 have 3+ entries in both | 31 (wrapped names add one line to a few) |
| Tallest strip | 124 pt (scene 40, 10 cast) | 122 pt (same scene; the tall cast list, not the width, decides) |
| Mean strip/break row height | 46 pt | 50 pt (+8 %) |
| Smallest text in content | 5.8 pt (MANUAL) | 6.4 pt (MANUAL); chips 6.6 pt |
| Page fill | 56 to 96 % | 48 to 98 % (min 48 %, mean 63 %) |

Row heights are measured from the rendered PDFs (strip left-stripe rectangles); wrap counts use Helvetica metrics at the real widths and agree with the rendered line counts.

* **Where portrait is worse**: (1) names wrap in the 124 pt cast column ("STUDIO PRODUCER (Karan / Shetty)", "PORT OFFICER (Ramesh / Kaimal)", "YOUNG ANJALI (Ameya / Suresh)"); removing every cast wrap needs a cast column of about 143 pt (135 pt text + padding), which costs the heading column a further 19 pt; page count is sensitive to this because days are kept whole (Chrome test: cast 116 and 120 pt gave 12 pages but wrap APPACHAN (K. P. Gopinathan) and others; 124 to 132 pt gave 13); (2) headings wrap in 14 of 44 strips, so the heading block reads as 2 lines of uppercase bold, one more line per strip; (3) the facts band and legend need two lines each; (4) the strip is dense: 7 columns in 520 pt leave only 196 pt for the text a scheduler reads first.
* **Where portrait is not worse**: tallest strip and total pages. Most days fill 49 to 66 % of a 680 pt portrait page (page fill 48 to 98 %) and 56 to 84 % of a 500 pt landscape page; either way, one day per page.
* **No column cannot fit** at real size; none was dropped or merged. The tightest cells are Type (the "INT/EXT" chip is about 42 pt wide in a 48 pt cell) and Pages (an "1 2/8" value plus the MANUAL tag in 36 pt).
* A landscape-looking variant of the same page (long rows) would not be better here: rows are no taller in portrait; the cost is paid in wrapping and density, not in pages.

**Design choices (open to veto)**
* Portrait orientation (user decision, v2); locations line in the facts band (addition); legend moved to the top; day band title merged; day notes inside the table; header repeats on continuation.
* Not added although the data exists: per-day estimated total, shooting-start time, the scope label ("Entire schedule / Current day only / N selected days" is only in the job log, so a one-day print is not identified as a partial schedule — recommended addition), cast initials.

**Unverified assumptions**
* The strip colours and text colours are taken from `strip_palette`; the printed appearance of the Rust renderer for `Row.fill` was only seen through the baseline PDF.
* "Day taller than a page" behaviour in the Rust renderer (see B.1).
* Whether the Rust renderer's measured wrapping reproduces Chrome's line counts at 3.5 pt cell padding (the wrap table above is Chrome plus PyMuPDF metrics); the 13-page count is Chrome's and could differ by a page.
* Page counts for other schedules were not tested; portrait with the keep-whole-day rule leaves large white areas whenever a day is 350 to 500 pt tall.
* The "0/8 pages total" for an empty day is inferred from `fmt::pages(0)`, not seen in a PDF.
* Wrap-time anomaly on Day 1 (B.1 item 5) is inferred from reading `timeline::compute`, not reproduced in the app.
* `mac/` was not examined.
