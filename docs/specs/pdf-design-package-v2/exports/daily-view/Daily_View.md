# Daily View — PDF / TXT design handoff

> ## NO BASELINE PDF WAS AVAILABLE FOR THIS EXPORT
> The Daily View export is new since the earlier audit pack and **no real exported PDF was supplied**. This design is derived only from the source builder (`exports/daily_view.rs`), the Daily View page code (`daily.tsx`), the real UI captures and the visual language of the sibling documents. **It has NOT been visually validated against a real export.** Every value in `Daily_View.pdf` and its stress test is **illustrative (fictional) sample data modelled on the Low Tide Radio project** (names, headings, notes and addresses from the supplied schedule / call-sheet PDFs; location status "Confirmed", the four cast call times, and the Final call-sheet state are invented). The HTML and the PDF carry the same notice at the top of the page; the notice is **not** part of the product design (toggle "Illustrative-sample notice" in the HTML preview hides it).

| | |
|---|---|
| Export | Daily View — `schedule.export_daily_view` ("Shoot Day N - Daily View") |
| Variant | PDF, portrait US Letter (and the TXT twin) |
| Status | **Design reference — not integrated; not validated against OpenFrame's Rust renderer** |
| Repo commit | `2f5a045416bd2125801329533fc47f986ccc8174` (`main`) |
| Date | 2026-10-23 |
| Files | `Daily_View.html` / `.pdf` (1 page) · `stress-test/Daily_View_STRESS_TEST.html` / `.pdf` (6 pages, 4 variants) · generator `tools/build/scheduling/build_daily.py` |

---

## A. Purpose and users

One shooting day, as a page you can hand to someone: *what do I need to know for this day?* (the page subtitle in `daily.tsx`).

* **1st AD / UPM** — which scenes in which order, how long, which location and whether its address is known, who is needed and when, whether the call sheet exists and is current, the day notes and breaks.
* **Department heads (props, wardrobe, art)** — the "important breakdown items" for the day's scenes.
* **Producer / coordinator** — a one-sheet summary for a day, usable on the unit or by email as plain text.
* Day-level means it must normally fit **one page** and must print gaps explicitly ("Address missing", "Location missing.", "Call times: Not entered yet") rather than hide them.

## B. Verified functionality

* **Scope**: no scope choices. The button exports the day the screen shows. `DailyViewExportButton({ dayId, dayLabel })` (`features/export/buttons.tsx`) passes `baseArgs={{ dayId }}`; document name `"<dayLabel> - Daily View"`; dialog title "Export Daily View"; `bare` (no project prefix in the suggested name → `Shoot Day 1 - Daily View.pdf`, UI capture `601-prod-daily-export-dialog`).
* **Options**: none. Note in the dialog: "Private notes excluded. The document shows what the Daily View shows; the schedule is not changed."
* **Formats**: PDF (hint "printable") and "Plain text" (`txt`). `ExportDailyViewArgs { format, path, dayId? }`.
* **Entry point**: `workspaces/production/tabs/daily.tsx:59`, in the `PageHeader` actions next to the day `Select`.
* **Data**: `schedule::ops::daily_view` — the same read the screen uses (`ScheduleDailyDay`: `label`, `title`, `date_label`, `is_off_day`, `notes`, `scenes: Vec<ScheduleStripDto>`, `markers`, `summary`, `cast: Vec<ScheduleDailyCast{key,character,actor,call_time}>`, `crew_call`, `locations`, `call_sheet`, `breakdown_items`, `warnings`). Call times and crew call come from the day's call sheet (`callsheet::call_times`) when one exists.
* **Errors**: no schedule → "Create a shooting schedule first."; no days → "The schedule has no shooting days yet." (`nothing_to_export`); an unknown `dayId` → not found (the screen falls back to another day, the document never does). Off days and empty days **do** export.
* **Title/format mechanics**: PDF built with `Document::new(...)` — the builder never sets `.landscape(true)`, so the default **portrait** applies (`report.rs`: `landscape: false`).
* **Builder blocks**: `Masthead` (eyebrow "Production planning / Daily View" in orange, title = day label, subtitle = date label or "No date yet", line "Project: <title>", "Off day" chip, `Side::Badge` "Daily View snapshot" + "Revised <date>" + "Shooting schedule · PDF export", rule), `Metrics{compact:true}` (Scene(s), Estimated), then `Section` + `KeyValues` / `Paragraph` for seven sections in this order: **Scenes, Location, Cast, Call Sheet, Important Notes, Important Breakdown Items, Scene Order** (same order and wording as the screen).

### B.1 Current-output assessment

There is no current PDF, so this is an assessment of the **builder's output as written plus the screen**.

**Retain**: the section order and names; the masthead and snapshot badge (same masthead as the Shooting Schedule PDF p1); compact metrics; every empty-state sentence (exact strings below); document name; reproducibility (revision date, no export clock).

**Improve visually** (design choices, all within the vocabulary of the renderer): short sections side by side; tiles for scene numbers; call-sheet status as a chip; breaks and breakdown items as chips as on screen; Scene Order as a small table.

**Verified functional / content observations** (source and UI capture evidence; none observed in a PDF)
1. **Scene Order is double-numbered**: the builder writes `"{i+1}. {scene number}: {heading}"` (e.g. "1. 28: INT. …", "2. 29: …"); the screen shows the same through `<ol>` + `"{number}: {heading}"`. Position and scene number look alike and are easy to confuse. The design uses an `#` column (production order) and a `Scene` column.
2. **Call times are not tied to people**: `Call times: 06:30 / 07:00 / 07:30` lists only the times that exist, with no names, in a different order from "Cast: …". With a partly filled call sheet it is impossible to tell whose time is whose. The design pairs them (Cast table). The data is available (`ScheduleDailyCast.call_time` per member).
3. **"Estimated" includes break durations**: `day_summary` adds marker durations to `estimated_minutes`. UI capture 597 shows "3 scenes · 45m estimated · 3 without estimate" where the 45 m is only the DINNER marker. The wording "N estimated" under Scenes therefore overstates scene time.
4. **Warnings are not printed**: the screen's banner ("N things to check on this day: …") is not in the builder although the doc comment says the page "shows exactly what the screen shows".
5. **Off day prints false alarms**: "Location missing." and "No cast confirmed for these scenes." (capture 600 shows the same on screen). Kept in the design because they are the source strings (stress variant 2) — see K.
6. **Cast list drops the character** when an actor exists (`actor.unwrap_or(character)`); the design shows both.
7. **Breakdown items drop their category** (the screen exposes it only as a tooltip); the design follows the builder.
8. `minutes(0)` → "0m" (metric "Estimated" on an empty day).

**Cannot be determined from the evidence**: how the current renderer actually paints `KeyValues` and `Paragraph` (label column width, wrapping of long values); whether a typical day currently fits one page; the exact look of the PDF header on a day with a very long label; how long breakdown-item lists break across pages today.

## C. Source mapping

| Item | Value |
|---|---|
| Op | `schedule.export_daily_view` |
| Builder | `modules/exports/daily_view.rs`: `export`, `sections(&ScheduleDailyDay)`, `text_of` (plain text), enum `Part { Facts, Line, Numbered }` |
| Data | `schedule/ops.rs`: `daily_view`, `ScheduleDailyDay`, `ScheduleDailyCast`, `ScheduleDailyView`; `board.rs`: `ScheduleShootDaySummary`, `ScheduleStripDto`, `ScheduleMarkerDto`, `CallSheetBrief`; `source.rs`: `ScheduleLocRef`, `ScheduleItemRef` |
| UI | `buttons.tsx::DailyViewExportButton` in `workspaces/production/tabs/daily.tsx`; screen captures `597-599` (days), `600` (off day), `684` (no schedule → "Select or create a shooting day."), `601` (dialog) |
| Footer text today | `footer(project, document)` → "<Project> · <Shoot Day N - Daily View>" |

## D. Field inventory

| Element | Source field | Kind | Empty treatment |
|---|---|---|---|
| Eyebrow | literal "Production planning / Daily View" | fixed | — |
| Title | `d.label` ("Shoot Day N" / "Off Day") | required | — |
| Date line | `d.date_label` (`date_short`: "Mon 11 Jan 2027") | conditional | "No date yet" |
| "Project: …" | project title | conditional | omitted |
| Chip "Off day" | `d.is_off_day` | conditional | omitted |
| Badge "Daily View snapshot", "Revised <date>", "Shooting schedule · PDF export" | `revised_at` | derived | — |
| Metric Scene(s) | `summary.scene_count` | derived | "0" |
| Metric Estimated | `minutes(summary.estimated_minutes)` (includes breaks, see B.1) | derived | "0m" |
| Scenes: numbers | `scenes[].number` | repeatable | "Not numbered" |
| Scenes: Total | "N scene(s)" · "<est> estimated" (only > 0) · "<n> without estimate" (only > 0) | derived | whole section: "No scenes scheduled." |
| Location name | `locations[].name` | repeatable | "Location missing." |
| Location details | `status` · `address` | conditional | address → "Address missing" |
| Cast names | `cast[].actor` else `character` | repeatable | "No cast confirmed for these scenes." |
| Cast character (design) | `cast[].character` | conditional | blank when only a character |
| Call times | `cast[].call_time` (builder: joined with " / ") | conditional | "Call times: Not entered yet" |
| Crew call | `d.crew_call` | conditional | omitted |
| Call sheet status | `call_sheet.status` → Final / Needs refresh / else Draft | conditional | "No call sheet yet for this day." / "Off days have no call sheet." |
| Call sheet details | `source_changed` → "Available. The schedule changed after it was created."; Final → "Finalized and matches the schedule."; else "Available and up to date with the schedule." | derived | — |
| Day notes | `d.notes` (split on line breaks) | conditional | "No day notes." |
| Break chips | `markers[]` → "LABEL at HH:MM (dur)" (time and duration only when present) | repeatable | omitted |
| Breakdown items | `breakdown_items[].name` (distinct category+name) | repeatable | "No confirmed props, wardrobe or other items for these scenes." |
| Scene order | `scenes[]`: position, `number`, `heading`, `estimated_minutes` | repeatable | "Nothing scheduled yet." |

Not printed although present in `ScheduleDailyDay`: `warnings`, `is_today`, `summary.cast`, per-scene INT/EXT, D/N, cast, pages, location.

## E. Content architecture

Order (same as the builder/screen): Masthead → metrics → **Scenes | Location** → **Cast** → **Call Sheet | Important Notes** → **Important Breakdown Items** → **Scene Order**. Reading order left-to-right, top-to-bottom is exactly the builder's order, so the TXT twin keeps working unchanged.

Changes versus the builder, and why
1. **Two-up rows** for the four short sections (Scenes | Location, Call Sheet | Important Notes): the builder stacks seven full-width sections; most are one or two lines, which would waste a page. Keeps a typical day on one page.
2. **Cast as a table** — Cast (actor, else character, as the builder) · Character · Call — pairing each call time with its person (B.1 item 2). The Call column appears only if at least one call time exists; otherwise the line "Call times: Not entered yet" is printed. "Crew call: HH:MM" printed under the table when present. Both TXT lines stay valid, but see K: a TXT Cast line should pair times with names too.
3. **Scene Order as a table** — `#` (production order), Scene tile, Heading, Est. Fixes the double numbering.
4. **Chips** for break items and breakdown items (the screen uses chips; the builder joins with " · "). The chip text equals the builder text (e.g. "LUNCH at 13:00 (1h)").
5. **Call-sheet status chip** (Final green, Needs refresh amber, Draft neutral) + the builder's sentence.
6. **Scene number tiles** in Scenes (as the screen's buttons) and "Total · …" line.

## F. Visual specification

* **Portrait vs landscape — decision: portrait.** The builder has no `landscape` flag; the content is a vertical list of short labelled sections, line lengths are short, and a day fits one portrait page. Landscape would only help a wide table, which this document does not have (the Scene Order table is four columns).
* Page: US Letter 612 × 792 pt; margins 46 pt left/right/top, 66 pt bottom; content box 520 × 680 pt.
* Masthead: eyebrow 7.6 pt bold tracked; title 26 pt bold navy ("Shoot Day 1"); date 11 pt bold `--of-slate`; "Project: …" 8 pt; rule 1.2 pt navy below; right badge: "Daily View snapshot" accent chip, "Revised …" 9.4 pt bold, "Shooting schedule · PDF export" 7.8 pt. "Off day" chip under the date.
* Metrics: two compact boxes (value 14 pt bold + label 6.6 pt tracked caps), max 150 pt wide.
* Section headings: shared `.sec` (10 pt bold caps navy, 1.2 pt rule); 11 pt above, 5 pt below. Two-up columns are 253 pt each with a 14 pt gutter.
* Scenes: tiles 9 pt bold; Total line 8 pt `--of-slate`. Location: name 9.6 pt bold; details 8 pt slate; "Address missing" and "Location missing." use the shared red `needs` marker (text + tint, readable in grayscale).
* Cast / Scene Order tables: shared `tbl zebra boxed compact`; header navy 6.9 pt caps; body 8.4 pt; name column bold navy; Call and Est. right-aligned, tabular.
* Call sheet: status chip 7.4 pt + 8.8 pt sentence. Notes: 8.8 pt paragraphs, break chips 8 pt bold. Breakdown items: pill chips 8 pt, wrapping.
* **Status redundancy**: call-sheet status is always the word (FINAL / NEEDS REFRESH / DRAFT) in the chip; gaps are words ("Address missing"), not colour.
* Footer: shared system (PNG logo lockup, "Created using OpenFrame", "Page X of Y").
* **Running header decision**: none on a one-page Daily View (verified `--no-header`). **Rule for the renderer**: print a compact top-right line `<PROJECT> · <SHOOT DAY N> · DAILY VIEW` only when the document has more than one page, because page 2 of a long day otherwise carries no day identity. The multi-page stress test shows this (with a generic text because it contains several days).

## G. Dynamic behaviour

* **Off day**: "Off day" chip in the masthead; Scenes "No scenes scheduled."; Location "Location missing."; Cast "No cast confirmed for these scenes."; Call sheet "Off days have no call sheet."; notes shown; items "No confirmed props, …"; Scene Order "Nothing scheduled yet."; metrics 0 / 0m (stress variant 2).
* **Empty shoot day** (no date etc.): "No date yet" and every empty sentence (variant 3).
* **Heavy day**: tiles wrap onto several lines (any number of scenes); an unnumbered scene prints "Not numbered"; locations stack (long names/addresses wrap; a missing status simply omits it, a missing address prints "Address missing"); the cast table grows; notes keep their line breaks; markers and items wrap as chips (variant 1, 3 pages).
* **Cast without actor**: the Cast cell shows the character name and the Character cell is blank (builder rule `actor ?? character`).
* **Call times**: all missing → no Call column and "Call times: Not entered yet"; some missing → "—" in the cell.
* **Estimates**: "<n> without estimate" and "–" in the Est. cell for scenes without an estimate.
* **Long headings** wrap inside the Scene Order table; rows grow.

## H. Pagination and print rules

* Portrait US Letter; margins as above; typical day = 1 page (the sample uses 89 % of the content height).
* Sections are kept whole (`Scenes|Location` and `Call Sheet|Important Notes` rows never split); a section heading is never the last thing on a page (keep-with-next).
* **Cast table and Scene Order table split between rows; the header row (and for Scene Order the title row "Scene Order · Production order · numbers are scene numbers") repeats on every page**; at least two rows per page fragment (a single trailing row moves with the previous one).
* Chip groups (notes, breakdown items) may split between chips lines only when the group is taller than the space left; otherwise move whole.
* Footer and "Page X of Y" on every page; running header only when more than one page (F).

## I. HTML / PDF correspondence

* `Daily_View.html` = the primary PDF: Shoot Day 1 (Mon 11 Jan 2027), two scenes (28, 29), Coast Radio Station, four cast, crew call 07:00, Final call sheet, day note and LUNCH marker from the real schedule PDF; the 12 breakdown items are the call sheet PDF's props / costumes / SFX lines for scenes 28–29; the address is the call sheet's. Invented: location status "Confirmed", cast call times 06:30 / 07:00 / 07:00 / 07:30, the Final state's wording. Toggle: "Illustrative-sample notice".
* `stress-test/Daily_View_STRESS_TEST.html`: four variants separated by page breaks — (1) heavy fictional day (16 scenes with long headings, one unnumbered, 3 without estimates, 4 locations incl. a long one and missing addresses/status, 16 cast with partial call times and two without actor, 3-line note + 6 break chips, 35 items, Needs refresh call sheet) = 3 pages with repeated table headers; (2) off day; (3) day with nothing entered; (4) cast without call times, scenes without estimates, Draft call sheet. Running header present because the file is multi-page.
* Rendered with `tools/render_pdf.py`; verified with `verify_pdf.py --size letter --no-header` (primary) and `--header "LOW TIDE RADIO · DAILY VIEW · STRESS TEST"` (stress).
* **Known differences vs Rust**: Chrome flex/grid wrapping for chips and the two-up columns must be reproduced with measured text; `tr.title` repeating-header trick is Chrome-specific (renderer repeats its own header).

### TXT twin (builder `text_of`, unchanged by this design)

Plain text for Shoot Day 1 of the sample (what `text_of` produces for the same data):

```
Shoot Day 1
Mon 11 Jan 2027

SCENES
Scenes: 28, 29
Total: 2 scenes · 7h 30m estimated

LOCATION
Location: Coast Radio Station (Old Signal Station, Vypin)
Details: Confirmed · Beach Road, Vypin, Kochi, Kerala 682508

CAST
Cast: Sajan Kuriakose · Aditya Nair · Nimisha Raghavan · Lal Fernandez
Call times: 06:30 / 07:00 / 07:00 / 07:30
Crew call: 07:00

CALL SHEET
Status: Final
Details: Finalized and matches the schedule.

IMPORTANT NOTES
Coast Radio Station interiors, day. Basement water pumped from 05:00.
LUNCH at 13:00 (1h)

IMPORTANT BREAKDOWN ITEMS
Army surplus ammunition box · Station logbook 1999 · … · Water tank and dirty water dressing

SCENE ORDER
1. 28: INT. COAST RADIO STATION - TAPE ROOM - DAY (4h)
2. 29: INT. COAST RADIO STATION - OPERATIONS ROOM - DAY (2h 30m)
```

The TXT has no project line, no revision, no metrics. Recommendation (K): add the project and revision lines, and pair call times with names, so TXT and PDF carry the same facts.

## J. Acceptance criteria

- [ ] Portrait US Letter, margins 46/46/46/66; `verify_pdf.py --size letter` passes; one-page day has **no** running header.
- [ ] Masthead: eyebrow "Production planning / Daily View", title = day label, date line ("No date yet" when absent), "Project: …" when set, "Off day" chip only on off days, snapshot badge with the revision date; no export clock anywhere.
- [ ] Two metrics: scene count (singular "Scene" at 1) and Estimated.
- [ ] Section order is Scenes, Location, Cast, Call Sheet, Important Notes, Important Breakdown Items, Scene Order; headings use the exact names.
- [ ] Every empty state prints its exact source sentence (list in D) and no placeholder is invented.
- [ ] "Address missing" / "Location missing." use the needs marker; a missing location status prints nothing.
- [ ] Cast table pairs each call time with its person; Call column appears only when a time exists; otherwise "Call times: Not entered yet".
- [ ] Scene Order shows `#` and the scene number separately; unnumbered scenes read "Not numbered".
- [ ] Call-sheet chip text is exactly Final / Needs refresh / Draft with the matching builder sentence.
- [ ] A heavy day flows to extra pages with repeated table headers, no clipped text, no section heading alone at a page bottom; a single trailing table row never sits alone.
- [ ] Multi-page documents show the compact running header; single-page ones do not.
- [ ] TXT output equals `text_of` for the same day.
- [ ] The "Illustrative sample" banner is **not** in any real export.

## K. Known issues and unresolved questions

**Verified in source/UI (no PDF)**: B.1 items 1–8 (double numbering; unpaired call times; "estimated" includes breaks; warnings not printed; off-day false alarms; character dropped; category dropped).

**Design choices (open to veto)**: two-up rows; cast and scene-order tables; chips; portrait; header only on multi-page documents.

**Unverified / needs a real export**
* Everything visual: no baseline exists, so spacing, wrapping and page count against the Rust renderer are unvalidated. The HTML page-fill figure (89 %) is Chrome's, not the renderer's.
* Whether the product should print the screen's "things to check" warnings (the data `ScheduleDailyDay.warnings` is already loaded). Not designed because the builder does not print it; a one-line section "Things to check" under the masthead would be the natural place.
* Whether off days should hide "Location missing." / "No cast confirmed…" (recommended; kept only because they are the source strings).
* Whether "Estimated" should exclude break durations (it currently includes them).
* Cast call times and crew call come from the call sheet; with no call sheet both are always empty.
* TXT: proposed to carry the same project/revision lines and name-paired call times; the current `Part` enum has no table variant (implementation detail).
* `mac/` was not examined.
