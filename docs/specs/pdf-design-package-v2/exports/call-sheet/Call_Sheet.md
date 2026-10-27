# Call Sheet — PDF design specification (v2: regrouped)

| | |
|---|---|
| Export | **Call Sheet** (`callsheets.export_pdf`) — one shooting day, one revision |
| Variants covered | Final (frozen snapshot) · Draft · Draft with missing crew call · Replaced revision · optional sections on/off |
| Status | **Design reference — not integrated into OpenFrame; not validated against OpenFrame's Rust renderer** |
| Repository | `Saravankumar25/OpenFrame`, branch `main`, commit `2f5a045416bd2125801329533fc47f986ccc8174` (Windows tree) |
| Files | `Call_Sheet.html` · `Call_Sheet.pdf` (4 pp, US Letter portrait) · `stress-test/Call_Sheet_STRESS_TEST_long.{html,pdf}` (9 pp) · `stress-test/Call_Sheet_STRESS_TEST_minimal_draft.{html,pdf}` (1 p) |
| Design system | `../../00_SYSTEM/PDF_Design_System.md`, `../../00_SYSTEM/assets/openframe-print.css` |

---

## A. Purpose and users

The call sheet is the **operational** document of a shooting day. It is printed, emailed and pinned up. Readers scan it in seconds, often at 05:00, often on a phone-photo of a printout.

| Reader | What they look for first |
|---|---|
| Everyone | Crew call time, shoot day/date, unit base address |
| 1st AD / 2nd AD / production office | Day times (general, shooting, breakfast, lunch, wrap), scene order with start times, cast calls |
| Cast, drivers | Call time, pickup, hair/make-up time, contact |
| Camera, art, props, costume, sound | Scenes and the department notes for the day |
| Medic, safety, any emergency | Nearest hospital, emergency contact, safety notes — must be unmistakable but calm |

Design priorities: (1) first-glance facts on page 1 (day, date, crew call, location, first scenes); (2) the scene schedule and the talent table are the working core; (3) reference material (department notes, crew, attachments, images) follows and may be omitted; (4) "Needs input" gaps are visible so nobody issues an incomplete sheet by accident.

## B. Verified functionality

* **Scope:** exactly one call sheet (one `call_sheet` row = one shooting day, one revision). No scope picker.
* **Format:** PDF only (`Fmt::parse(&a.format, &[Fmt::Pdf])`).
* **UI entry points (verified):** `CallSheetExportButton` in `windows/apps/desktop/src/features/export/buttons.tsx`, mounted (a) per row in `workspaces/call-sheets/CallSheetList.tsx` (button "Export") and (b) in the editor `workspaces/call-sheets/CallSheetEditor.tsx` (button "Export PDF"). The dialog title is "Export Call Sheet", sub text "Creates the PDF snapshot to send."
* **Option:** one checkbox, arg `includeOptional` (default **on**): "Include the added sections (weather, extras, department notes, advance schedule, crew, contacts, attachments, reference images)". Off → only the six default sections are printed.
* **Dialog notes (verbatim):** Final: "This call sheet is final: the PDF is made from its frozen snapshot and never changes it. Private notes excluded." Otherwise: "Private notes excluded. Export does not change the shooting schedule. Finalize the call sheet before sending it."
* **Final vs Draft:** a *Final* sheet is exported from its immutable snapshot (`snapshot.content_json`, keys `document` and `source`); otherwise from the live `document_json`. Draft sheets print a "Draft · Not final" badge and an in-document review callout. A Final sheet that was superseded prints a "Replaced by a newer revision" badge.
* **Warnings returned in the export result (not printed):** reference-image placeholders; "This call sheet is not final yet, so the PDF is marked Draft."; "The crew call time is empty."; "A newer revision of this call sheet exists. The PDF shows this final version."
* **Missing values:** anything required but empty prints **"Needs input"** (`NEEDS_INPUT` in `modules/schedule/callsheet.rs`). OpenFrame never invents values (weather, sunrise, sunset are typed by the user).
* **Branding:** `deliver()` sets `out.doc.brand = BRAND_LINE` ("Created using OpenFrame") for all non-Story-page exports.

### B.1 Current-output assessment (supplied `Call Sheet — One Shooting Day.pdf`, 5 pp, Letter portrait)

**Retain**
* Section order and grouping (masthead → crew-call band → Production → Locations → Scene schedule → Talent → Practical/Safety → Practical notes → optional sections). It matches how a call sheet is read.
* Navy/orange identity, dark "Shoot day" block, orange crew-call band, navy table headers, teal location tile, tinted meal/travel rows with a left stripe.
* "Needs input" chips, ESTIMATED start markers, INT/D chips, cast-ID pills, department cards with navy headers.
* The "Final · date" badge and "Script: …" note.

**Improve visually**
* The footer carries project + document + "Page n / N" on the right and a redrawn mark with typeset "OpenFrame STUDIO" on the left (`design::draw_footer`, `lockup()`); replace with the **official logo image**, the brand line beneath it, and `Page X of Y` only (see F).
* Table type is small (5.8 pt "ESTIMATED"; labels 7 pt): raise the floor to 6.4–6.6 pt.
* Large unused areas at the bottom of pages 1–2.

**Verified functional / layout problems (all from the supplied PDF)**
1. **Orphaned section heading (p.1):** "SCENE SCHEDULE" and its hint are the last element on page 1; the table starts on page 2.
2. **Wrong continuation label (p.2):** the table's first page is labelled "SCENE SCHEDULE (CONTINUED)" although no row was printed on p.1 (the `tbl.continued` band is emitted whenever a styled table starts a page).
3. **Cramped cast columns (p.2):** *Pickup* (52 pt) and *Hair / MU* (56 pt) wrap to five lines ("Unit / hotel, 60 / minutes / before / call") and *Call* wraps "NEEDS INPUT"/"minutes before call" in a narrow column, making each cast row ~60 pt tall.
4. **Reference images alone on the last page (p.5)** with ≈80 % of the page empty, because the image card section follows a section that ends on p.4.
5. **Inconsistent date formats:** *Advance schedule → Day* prints the stored text "Next: 2027-01-12" (ISO) while the masthead prints "11 January 2027". This is the stored data (`CallSheetAdvance.day`), not a layout bug; flagged for the data owner.
6. **Footer collision risk:** the right footer text is bold, tracked and long ("LOW TIDE RADIO · CALL SHEET: DAY 1 (V2) · Page 1 / 5"); with a long production title or call-sheet title it competes with the logo lockup (not observed to collide in the supplied file; unverified for long titles).

**Cannot be determined from the evidence**
* How the Rust renderer breaks a `Cards` grid whose row is taller than the remaining space (the design below assumes "whole card moves to next page").
* Behaviour for non-Latin scripts (embedded fonts pack) — out of scope of this design.
* The Mac build's call-sheet layout (`mac/…/exports/callsheet.rs` exists; not compared).

## C. Source mapping (verified at commit `2f5a045…`)

| Item | Location |
|---|---|
| Op registration | `windows/crates/openframe-application/src/modules/exports/mod.rs` → `r.query("callsheets.export_pdf", callsheet::export)` (Capability `Export`) |
| Builder | `…/modules/exports/callsheet.rs` → `pub(super) fn export(core, actor, a: ExportCallSheetArgs)`; args `{ id, format, path, includeOptional? }` |
| Document model | `…/modules/schedule/callsheet.rs` → `CallSheetDocument` (+ `CallSheetHeader`, `CallSheetTimes`, `CallSheetDocLocation`, `CallSheetDocScene`, `CallSheetDocCast`, `CallSheetPractical`, `CallSheetOptional`, `CallSheetExtra`, `CallSheetDepartmentNote`, `CallSheetAdvance`, `CallSheetCrewMember`, `CallSheetCommunication`, `CallSheetAttachment`, `CallSheetExtraField`); `estimated_starts()`, `is_final()`, `NEEDS_INPUT`, `OPTIONAL_SECTIONS` |
| Helpers | `…/exports/style.rs` (`cell`, `cell_bold`, `cell_or_missing`, `long_date`, `day_number`, `number_pills`, `split_list`, `nb`) |
| Renderer | `windows/crates/openframe-import-export/src/report.rs` (`Document`, `Block`, `to_pdf_strict`, `MARGIN = 46`, `FOOTER_SPACE = 34`, `BRAND_LINE`) and `report/design.rs` + `design/{blocks,cards,table}.rs` (`Masthead`, `Side::Dark`, `Callout`, `Section`, `Cards`/`Card`/`CardHead`/`CardPart`, `StyledTable`/`Col`/`Row`/`Cell`/`CellItem`, `Chip`, `Tile`, `Field`, `pal::*`, `draw_footer`, `lockup`) |
| Block use in this export | `Masthead` (+`Side::Dark`), `Callout`, `Section`, `Cards`, `StyledTable` (`HeaderStyle::Navy`, `boxed`, `zebra`, `continued`), `Text` |
| Page | `Document::new(&doc_title).footer(footer(project, sheet.title))` — portrait (no `landscape()` call) |
| Image loading | `exports/mod.rs::load_image` → `ReportImage::decode` (re-encodes to JPEG q85, max edge `ReportImage::MAX_EDGE`); `image_warnings` |
| Date helpers | `exports/mod.rs::date_from_ms` ("30 Sep 2026"), `style.rs::long_date` ("Monday", "19 October 2026") |

## D. Field inventory

`d` = `CallSheetDocument`. "Needs input" = print the chip `NEEDS INPUT` (red-tone, text label). "Omit" = element not printed.

| Visible element | Source | Kind | Empty treatment |
|---|---|---|---|
| Running header | `d.day_label` (digits via `day_number`, zero-padded to 2) · `sheet.revision` · production name | derived | segment omitted if unknown (see F.8) |
| Eyebrow "CALL SHEET · REVISION n" | `sheet.revision` | required | — |
| Title | `d.title` → else `source.productionTitle` → else project title; **upper-cased**; "CALL SHEET" if all blank | derived | fallback |
| Status badge | `is_final(status)`; `frozen_at` date (`date_from_ms`); `superseded_by` | derived | Final → green `FINAL · date`; + red "Replaced by a newer revision"; else amber "Draft · Not final" |
| "Script: …" | `source.sourceLabel` | optional | omit |
| Counts line | #scene rows (`is_scene`), `cast.len()`, `locations.len()` | derived | "0 scenes · …" printed as numbers |
| Shoot-day block: label / big number / weekday / date / foot | `d.day_label` (label "SHOOT DAY" if it has digits or is empty, else the label text; big = zero-padded digits), `d.date` via `long_date` ("Monday", "11 January 2027"), `d.day_of` ("Day 1 of 14") | conditional | no date → "Date needs input"; no `day_of` → omit foot |
| Crew call | `d.crew_call` | **required** | blank → label "CREW CALL · REQUIRED", `NEEDS INPUT` chip, tone Warn, extra column "Not ready to issue / Complete the missing call-sheet details, then finalize." (drafts only) |
| General call / Shooting call / Breakfast / Lunch / Est. wrap | `d.times.*` | optional each | blank columns omitted |
| Production card(s) | `d.header.{company,address,producer,director,first_ad,contact}` | optional each | blank fields omitted; whole section omitted if none; fields split into two cards (`ceil(n/2)` first) |
| Location card | `d.locations[]`: `name` (header, upper-case, teal index tile `01…`), `address` (lead line, bold) | repeatable | no address → field "Address: Needs input" |
| — fields | `meeting_point`, `access`, `parking` (optional); `hospital`, `emergency_contact` (**always printed**, "Needs input" when blank); `notes` (optional) | | |
| "No location for this day." | — | empty state | text printed when `locations` is empty |
| Scene schedule rows | `d.scenes[]`: `start` (else `estimated_starts()` shown with ESTIMATED label, else "Not set"), `number` (tile), `heading` (+ ` (location)` if location not already in heading; upper-case), `description`, `int_ext`/`day_night` chips, `cast_ids` (pills), `pages`, `minutes` (`minutes()` → "2h 30m"; "–" if none) | repeatable | "No scenes on this day." |
| Schedule rows (`kind` = Meal / Travel / Company Move / Custom) | same struct, `kind` chip + heading + description | repeatable | tinted row + left stripe: Meal = amber `#fff6e3` + orange; Travel/Company Move = blue `#eaf3fa` + teal; Custom = panel + slate |
| Section hint | " · Start times from the schedule" if any `start` entered, else " · Start times not entered" | derived | — |
| Talent rows | `d.cast[]`: `cast_id`, `actor` (or Needs input), `character`, `status`, `pickup`, `makeup`, `call_time` (or Needs input), `contact`, `notes` | repeatable | optional columns appear only if **any** row has a value; "No cast called." |
| Safety cards | `practical.emergency_contact` (always, label "Production emergency contact"), then one card per location: "`<name>` · Nearest hospital" | derived | "Needs input" |
| Draft review callout | static text (Draft only) | conditional | hidden when Final |
| Practical notes card | `practical.{parking, meeting_point, travel_notes→"Transport", meal_break→"Meals", toilets, access, safety, special_instructions, production_notes}`, `d.day_notes`→"Day notes", `d.extra_fields[]` (label/value) | optional each | omit blanks; omit section when none |
| Weather | `optional.weather` (+`sunrise`,`sunset`) | optional | omit section if no weather text or no fields |
| Extras | `optional.extras[]`: `description`, `quantity`, `call_time`, `on_set`, `notes` | repeatable | omit section if none has description/quantity |
| Department notes | `optional.departments[]` (`department`, `notes` split on newline and "; ") | repeatable | cards with empty notes dropped |
| Advance schedule | `optional.advance.{day,location,cast,props,wardrobe,preparation}` | optional | omit blanks/section |
| Crew | `optional.crew[]` sorted by department (case-insensitive); department shown on the first row of each group; call = own call → else crew call → else Needs input | repeatable | omit section when empty; Contact column only if any contact |
| Walkie channels and contacts | `optional.communication.{walkie_channels,emergency_contact,production_contact,note}` | optional | omit blanks |
| Special notes | `optional.special_notes` | optional | omit |
| Attachments | `optional.attachments[].name`, joined by ", " ("Sent separately") | repeatable | omit |
| Reference images | `optional.reference_images[]` → `load_image(asset)`; caption = `name` | repeatable | missing → "Image unavailable"; unreadable/too large → "Image can't be printed" (exact source strings) |

Nothing in this document is read from private notes.

## E. Content architecture

**v2 order.** Page 1 is unchanged from v1. The rest of the document is regrouped into two groups, following the information grouping of the user's reference call sheet (`Call_Sheet_Design_Reference_Preview.pdf`: "Cast & Crew" page, "Production Notes" page). Content, wording and styling are unchanged; only the order/grouping of sections changed.

**Group 1 — the day (page 1; unchanged)**
1. **Masthead** (content area, page 1 only): eyebrow → title → badges + "Script" → counts line; right-aligned navy **Shoot day** block.
2. **Crew-call band** (one callout): CREW CALL (hero) + the day's other times as equal columns.
3. **Production** — one or two cards of label/value rows.
4. **Locations** — cards (see G for the column rule).
5. **Scene schedule** — table: Start (58 pt) · Sc. (34) · Heading/description (flex) · Cast (62–70) · Pages (44) · Est. (46, right).

**Group 2 — People on the day** (kept together — see H; title block "PEOPLE ON THE DAY / Cast & Crew")
6. **Talent / Cast** — table: ID (28) · Actor (flex) · Character (flex) · [Status 50] · [Pickup 96–112] · [Hair/MU 62–104] · Call (64–78).
7. **Extras** *(optional; not in the reference — placed here because extras are people on the day)*.
8. **Crew** *(optional)*.
9. **Department notes** *(optional)*.

**Group 3 — Production notes** (kept together — see H; title block "DAY-OF LOGISTICS / Production Notes")
10. **Practical / Safety information** — emergency-contact card + one hospital card per location (2-up grid), Draft callout, **then the Practical notes card in the same section** (the separate "Practical notes" heading no longer exists).
11. **Weather** *(optional)*.
12. **Advance schedule** *(optional)*.
13. **Walkie channels and contacts** *(optional)*.
14. **Special notes** *(optional)*.
15. **Attachments & reference images** *(optional; one section: attachment names, then the images; the section is kept together)*.

A group that has no content at all (every section empty/omitted) is omitted with its title block. Group title blocks are compact (eyebrow 7.2 pt + title 17 pt, ≈ 36 pt tall); they were added because each group normally occupies its own page and a reader paging through a printed sheet finds "Cast & Crew" / "Production Notes" faster; the descriptive sentences of the reference ("Call times, department assignments …") are deliberately not reproduced (not application content). The running header remains the only repeated header.

### Changes from v1 (user decision after review)
| v1 | v2 |
|---|---|
| Talent followed the schedule on page 1/2 | Talent, Extras, Crew, Department notes form group 2 on their own page |
| Practical/Safety and Practical notes were two sections | One section, notes card inside it |
| Weather → Extras → Department notes → Advance → Crew → Walkie → Special notes → Attachments → Reference images | Group 3 order: Safety+Practical → Weather → Advance → Walkie → Special notes → Attachments & reference images |
| No group titles | Compact group title blocks; groups 2 and 3 are **kept together** (move to a new page only when they do not fit the space left) |
| Page 1 | **unchanged** |
Page-start rule: groups are *kept together*, not force-broken — see H. On a full sheet this still gives page 2 = people and page 3 = notes; on a sparse sheet the groups simply follow one another (the minimal-draft stress test is 1 page, not 3).

**Changes vs the current PDF (and why)**
* **Contact and Notes fold into the Talent table** (Contact becomes a muted sub-line under the Actor name; Notes a sub-line under the Character) instead of two extra columns. Reason: with all five optional columns the current layout leaves ~110 pt for Actor + Character and wraps header words mid-word (verified in the long stress test). Contact belongs with the person; notes belong with the role. Same data, better grouping.
* **Pickup and Hair/MU widened** (≥ 96 / 62 pt, flexible) — fixes the 5-line wraps.
* **Locations: one card per row** when there is more than one location (the current renderer uses 2 columns). Reason: a tall pair of cards that cannot split jumped to the next page and left page 1 58 % empty in the stress test. Single-column cards move independently. Single location was already one column.
* **Section titles live inside the table header** (`tr.title` row), so a table can never be separated from its name and the name repeats on continuation pages.
* **No "(continued)" band on the first page of a table.** On continuation pages the renderer should print the table title with "· continued" (the HTML repeats the title row without the suffix — see I).
* The footer no longer carries the project/document string (the running header and masthead do).

## F. Visual specification

**Paper:** US Letter portrait, 612 × 792 pt. **Margins:** 46 pt left/right/top, 66 pt bottom (footer zone). Content width 520 pt.

**Type** (Helvetica / Helvetica-Bold only): title 28 pt bold navy (24 pt in the stress test with a 4-line title; the renderer should shrink to fit up to 4 lines, never clip); eyebrow 7.6 pt bold caps, tracking 0.14 em, slate; section heading 10 pt bold caps, tracking 0.1 em, navy, 1.2 pt navy rule beneath, hint 7.4 pt slate right-aligned; body 8.8 pt; table cell 8.4 pt; lead address 9.6 pt bold; field label 7.6 pt slate / value 8 pt bold ink; labels/eyebrows 6.8 pt bold caps (tracking 0.1 em); chips 6.6 pt bold caps (floor — nothing is smaller, `ESTIMATED` raised from 5.8 to 6.4–6.6 pt); crew call hero 24–27 pt bold; day numeral 26 pt bold.

**Colour** (tokens in `openframe-print.css`): navy `#1A2640`, ink `#1A2438`, slate `#566277` (6.2:1), muted `#66728A` (4.8:1, smallest readable metadata), orange `#E07B1F` (fills/stripes only — 3.0:1 on white, never small text), orange-deep `#8A4F00` for orange text, orange-soft `#FFF1E0`, panel `#F5F7FA`, tint `#E9EFF5`, line `#D8DEE8`, teal `#2A6A8C`; status fill/line/text triplets for amber/red/green/blue as in `design.rs::pal`.

**Components**
* *Shoot-day block:* navy fill, radius 6 pt, padding 7/13/8 pt, label 6.8 pt caps `#A9B6CB`, numeral 26 pt white, weekday 9 pt bold, date 8.4 pt, "Day n of N" 7.6 pt muted.
* *Crew-call band:* callout, `accent` tone (orange-soft fill, orange-line border, radius 6 pt), columns 1.5 : 1 : 1 : 1 : 1 : 1, 1 pt-hairline dividers; first column label in orange-deep. Draft with no crew call: tone Warn (amber) and the "Not ready to issue" column.
* *Cards:* 0.5 pt `#B9C2D1` border, radius 6 pt, padding 6/9/7 pt; label/value rows are two-column (34 % / 66 %, value right-aligned, bold) with a 0.5 pt separator; **long values** (≥ ~60 characters or containing wrap) switch to label-above/value-left. Location card header: tint fill, teal index tile `01`, name bold caps.
* *Tables:* navy header 6.9 pt bold caps white; rows padding 4.2/6 pt, 0.5 pt row rules; zebra `#F5F7FA` on even rows; boxed (0.5 pt side borders); Cast tile = tint tile 8.6 pt bold; cast ID in teal bold; schedule rows keep the stripe colours above.
* *Needs input:* red-tone chip (`#FDE9E4` fill, `#F0B3A4` line, `#9A3822` text), text label always present → grayscale-safe.
* *INT/D chips:* neutral chips; `ESTIMATED` is a 6.4 pt muted caps label under the time.
* *Department cards:* navy head (white caps text 8.6 pt), body bullets with 3 pt orange dot, hairline separators.
* *Special notes:* amber callout (`#FDF0D9` / `#EFC57E` / `#8A4F00`).
* *Reference images:* 3-up grid, card padding 6 pt, frame ratio 100 : 66 (as source `ratio: 0.66`), `contain` fit on `#F5F7FA`, caption 8 pt slate centred; missing → flat panel placeholder (dashed hairline) with source wording.

**Footer (every page; identical):** official logo lockup `assets/openframe-logo.png` (699 × 163 px, ratio 4.289 : 1) at **17 pt high (≈ 72.9 pt wide)**, left edge on the left margin, its baseline-bottom 33 pt above the page bottom edge; directly beneath it `Created using OpenFrame` 6.8 pt regular slate; `Page X of Y` 7.4 pt ink at the lower-right margin, aligned to the brand-line baseline (≈ 22 pt above the bottom edge). No rule, no project text. Never recolour, outline, crop, or restyle the logo; never typeset "OpenFrame Studio".

**Running header (Call Sheet only):** single line top-right, 6.8 pt regular slate, tracking 0.04 em, baseline ≈ 24 pt below the top edge, no rule, no logo, no second masthead: `LOW TIDE RADIO · SHOOT DAY 01 · REVISION 2`.
**Derivation (F.8).** `<production title, upper-case> · SHOOT DAY <NN> · REVISION <n>`. `NN` = digits of `day_label` zero-padded to 2 (same `day_number()` helper as the masthead); if `day_label` has no digits, use the day label in upper case; if empty, omit the segment (and its separator). If the title is blank use "CALL SHEET".
**Pages.** Printed on **every page including page 1** (consistent identification when pages are separated; it is small and does not collide with the masthead). If the product owner prefers it on continuation pages only, suppress it on page 1 — documented as design choice, not a verified requirement.
**Overflow.** If the line would exceed the page width, wrap onto a second line within the 46 pt top margin; never overlap content.

## G. Dynamic behaviour

* **Optional sections** (and all blank fields): omitted entirely — no heading, no placeholder, no gap. Next content moves up. The only exceptions with printed empty states are Locations ("No location for this day."), Scene schedule ("No scenes on this day."), Talent ("No cast called.") — exact source strings.
* **Long text:** every value wraps; rows/cards grow; nothing is truncated or ellipsised. Titles wrap to up to 4 lines (tested: 80-character working title). Long unbroken strings (URLs, file names) break anywhere (`overflow-wrap:anywhere`).
* **Many rows:** the Scene schedule, Talent, Extras and Crew tables flow across pages; the table title + column header repeat on each page; a row is never split. (Stress test: 17 schedule rows, 14 cast, 24 crew over 8 pages.)
* **Conditional columns:** Talent shows Status / Pickup / Hair-MU only when at least one cast row has a value; Crew shows Contact only if any crew member has one.
* **Estimated times:** rows without a `start` use `estimated_starts()`; printed in slate with an `ESTIMATED` label; rows before the first known time print "Not set".
* **Schedule rows** (Meal/Travel/Company Move/Custom) print the kind chip + text, no scene tile/cast/pages, and keep their Est. duration.
* **Images:** 100 : 66 frame, `contain`, never cropped, never stretched; missing → "Image unavailable"; unreadable/too large → "Image can't be printed"; a warning is returned to the UI (not printed). Any number of images, three per row.
* **Final vs Draft vs Replaced:** see D. Draft with missing crew call changes the band's tone and adds the "Not ready to issue" column; both states are in `stress-test/…_minimal_draft`.
* **Department cards:** 2-up grid, cards of unequal height top-aligned; cards never split.
* **Dates:** weekday + long date; ISO strings inside free-text fields (e.g. Advance schedule) are printed as stored.

## H. Pagination and print rules

* Letter portrait; margins 46/46/66/46 pt; footer in the bottom margin zone, header in the top margin — neither can collide with content (content area 520 × 680 pt).
* Flow is continuous within a group. **Group rule ("keep together"):** group 2 ("People on the day") and group 3 ("Production notes") are each laid out as one unit. If the whole group fits in the space left on the current page it stays there; if it does not fit, it starts on a new page; a group taller than a whole page also starts on a fresh page and then splits only between rows/cards (this is what the reference render does — in the long stress test the 14-cast/24-crew group starts on a new page although half of the previous page is free; letting an oversize group flow on instead is an untested alternative). There is no unconditional page break. 20 pt of space separates a group from the content above it (dropped automatically at the top of a page). Page 1 (group 1) is unchanged. A group with no printable content is omitted together with its title block.
* The group title block is kept with the first section of its group (never the last element of a page).
* **Attachments & reference images** is one section kept together (heading, attachment names and images move as a block); an image never sits alone on a page without its heading.
* **Keep-with-next:** a section heading always stays with the first row/card of its content (min. 2 body rows for tables when they have ≥ 2 rows; else the whole table).
* **Table header repeat:** title row + column header repeat on every continuation page.
* **Break-avoid:** table rows, cards, callouts, the masthead, the crew-call band, field rows, safety cards, figures.
* **Do not leave** a section heading as the last element of a page; do not start a page with a single stray row when moving the previous row would avoid it (renderer: if only one row of a table fits on a page, move the table's last two rows together when the table has ≥ 3 rows).
* The HTML demonstrates the behaviour with Chrome print fragmentation (header/footers via `@page` margin boxes; `break-inside`, `break-after`).
* `Page X of Y` is computed from the real total.

## I. HTML/PDF correspondence

* `Call_Sheet.html` is the standalone print reference. It uses the shared stylesheet, shows the real "Low Tide Radio" data of the supplied PDF (the *Extras* block and the *missing-image* card are **fictional illustrative rows** hidden by default).
* `Call_Sheet.pdf` was rendered from it with `tools/render_pdf.py` (headless Chrome 155, `--no-pdf-header-footer`; page size, margins, footer, running header and page counters from CSS `@page`). 4 pages (page 1 schedule, page 2 Cast & Crew, page 3 Production Notes, page 4 attachments + reference image — each group lands on its own page because it does not fit the space left); `tools/verify_pdf.py` passes (page size, footer on all pages with the true `Page n of 4`, logo undistorted, no content in the footer band or outside the margins, running header on every page).
* **Preview toolbar** (screen only, never printed): toggles Weather · Department notes · Advance schedule · Crew · Walkie · Special notes · Attachments & reference images (and each of its parts) · Extras (fictional) · Missing-image placeholder · **Draft state** (swaps Final badge → Draft badge, shows the review callout and "Review before issuing" hint). The PDF captured: Final, all optional sections on except the two illustrative ones.
* **Stress tests** (fictional data, clearly named): `…_long` (title 80 chars, two locations with very long text, 13 schedule rows incl. meal/company move/travel/custom + ESTIMATED starts, 14 cast with every optional field, all optional sections incl. 24 crew and 6 images incl. a missing one); `…_minimal_draft` (nothing entered: every empty state and "Needs input").
* **Known differences from the Rust renderer:** (1) browser typography uses Arial (metric-compatible with Helvetica) — widths match to within rounding; (2) the continuation label ("· continued") cannot be produced by CSS, so the HTML repeats the table title unchanged; (3) Chrome decides break positions; the Rust renderer must implement the rules in H itself; (4) the logo is a PNG here; the PDF writer must embed it (see K).

## J. Acceptance criteria

1. Output is US Letter portrait; every page has the footer: logo image (ratio 4.289 ± 0.02, 17 ± 0.5 pt high), `Created using OpenFrame` directly below it, `Page X of Y` at the lower right with the true total; no other footer text; no rule.
2. Every page carries the running header `<TITLE> · SHOOT DAY NN · REVISION n` top-right; no rule, no logo; no other repeated header.
3. Title/production details/day block appear only in the content area of page 1.
4. Section order exactly as in E (v2): page 1 as before; "Cast & Crew" group = Talent, Extras, Crew, Department notes; "Production Notes" group = Practical/Safety (with the Practical notes card inside it), Weather, Advance schedule, Walkie channels and contacts, Special notes, Attachments & reference images (each group kept together; on a normal sheet each lands on its own page); empty optional sections leave no heading and no gap; there is no separate "Practical notes" heading.
5. No section heading is the last element on a page; each table keeps its title and column header on every page it spans; no row is split across pages.
6. Cast table: *Pickup* and *Hair / MU* never wrap to more than 3 lines for ≤ 40-character values at default widths; no header word breaks mid-word with the supplied sample.
7. Blank required values print `NEEDS INPUT` (text + tint), including crew call, cast call, hospital, emergency contact; draft shows "Draft · Not final" and the review callout; final shows `FINAL · <date>`; superseded shows the red badge.
8. Estimated start times carry the `ESTIMATED` label; schedule rows (meal/travel/move/custom) keep their stripe + kind chip.
9. All text ≥ 6.4 pt; body ≥ 8.4 pt; contrast ≥ 4.5 : 1 for text (orange only as fill/stripe).
10. In a grayscale print every status remains identifiable through text (Needs input, Draft, Final, ESTIMATED, MEAL/TRAVEL, INT/EXT).
11. Images keep aspect ratio and are never cropped; missing image → "Image unavailable".
12. A 17-row schedule, 14 cast rows and 24 crew rows render without clipping, overlap or footer collision (compare with the stress PDF).
13. Output is deterministic: same document + same settings → identical bytes.
14. `includeOptional = false` prints only the six default sections.

## K. Known issues and unresolved questions

**Verified defects in the current output:** B.1 items 1–5.

**Design choices (mine, to be approved):** header on page 1 as well; contact/notes folded into the talent table; locations single-column; table titles inside table headers; footer without project text; ESTIMATED label size.

**Implementation requirements / risks (verified in source)**
* `design::draw_footer` + `lockup()` currently *draw* the mark with vector lines (`draw_mark`) and typeset the wordmark. The brief requires the **official PNG artwork**. The PDF writer already embeds JPEG images (`PdfImage`, `ReportImage`), but `ReportImage::decode` re-encodes at JPEG q85 and blends alpha onto white — adequate for photos, **not ideal for a crisp wordmark**. Recommended: embed the logo PNG losslessly (Flate RGB, 699 × 163, or the 812 × 347 original), embedded once and reused on all pages.
* **Footer geometry changes.** Today content stops at `MARGIN + FOOTER_SPACE` = 46 + 34 = **80 pt** above the page bottom (`report.rs`, `fn` computing available height). The new footer zone is **66 pt**: logo 17 pt + 3 pt gap + 8 pt brand line, bottom edge 22 pt above the page edge (≥ 18 pt printer-safe). Both constants must be re-derived together; every export gains 14 pt of content height per page, so existing pagination (and any golden PDFs) will change.
* The first-page "orphan heading" and wrong "(CONTINUED)" label are caused by `StyledTable.continued` + a separate `Section` block; moving the title into the table header (or emitting heading+table as one keep-together unit) fixes both.
* The logo file was **cropped to its artwork bounds** (69,96–768,259 of 812 × 347) to remove transparent-looking white margins; pixels untouched. The unmodified original is included (`00_SYSTEM/assets/openframe-logo-original.png`). The original has a faint near-white gradient at its right edge (≤ 6/255); on white paper it is invisible, but on a tinted page it would show — footers sit on white.
* Cannot confirm Mac behaviour or non-Latin scripts.

**Unresolved questions for the product owner**
1. Should the running header also appear on page 1? (Default here: yes.)
2. Draft sheets: keep the in-document blue "Before issuing" callout, or move it to a banner under the masthead?
3. Should *Advance schedule → Day* be normalised to the masthead date format (data change)?
4. Should the 2-up Department-notes grid stay (balanced) or become a masonry flow?

**v2 addition to K.** Group page starts follow a *keep-together* rule (H), chosen after the first v2 version forced a page start for groups 2 and 3, which left a nearly empty draft on 3 mostly blank pages. Tested on the three Call Sheet files: primary and long stress unchanged (4 and 9 pages), minimal draft 3 → 1 page. Remaining edge: a group only slightly too large for the space left jumps to the next page and leaves that space empty. Oversize groups (taller than a page) also start on a fresh page, which can leave up to about half a page empty before them (long stress test p3 is 49 % full). The Rust pager needs a "group taller than space left → new page" check (it already measures blocks for keep-with-next).
