# OpenFrame Studio — PDF Export Design Package (v2)

Implementation-ready **design and specification** package for every user-accessible PDF export of OpenFrame Studio in scope (Screenplay, Sides, the Story page and any standalone Breakdown Summary are excluded). It is meant to be handed to the coding assistant that will integrate the approved designs into OpenFrame's Rust PDF renderer.

> **Status:** design references only. Nothing here changes OpenFrame; nothing has been rendered by OpenFrame's own renderer. The HTML files are visual/layout references, rendered to PDF with headless Chrome. Source of truth: `Saravankumar25/OpenFrame` `main` @ `2f5a045416bd2125801329533fc47f986ccc8174`.

> **This is version 2.** The original package (`OpenFrame_PDF_Design_Package/`, v1) is kept unchanged next to it. v2 = v1 + the changes below, requested after reviewing v1. Everything not listed is byte-for-byte the same design as v1.
>
> | Export | v2 change |
> |---|---|
> | Storyboard | **Portrait**; compact header; "visual lift" (film-frame border, number tile, shot chips, caption band, accent bar); panels per row selectable **2 (recommended, `Storyboard_Sheet.pdf`) or 3 (`Storyboard_Sheet_3col.pdf`)** — 4 panels on page 1 (2-up) / 6 (3-up) instead of 3 |
> | Shot List (single / multiple / entire) | **Portrait**; Notes now stacked under Description |
> | Shooting Schedule | **Portrait**, at real size (13 pp, 6.4 pt type floor) |
> | Beat Board Card Board | **Portrait**, 3 cards per row (Outline unchanged) |
> | Call Sheet | Page 1 unchanged; regrouped as **Cast & Crew** (Talent, Extras, Crew, Department notes) and **Production Notes** (Practical/Safety + Practical notes, Weather, Advance schedule, Walkie, Special notes, Attachments & reference images); groups are kept together (a group moves to a new page only if it does not fit the space left) with compact title blocks |
> | Cast Scene and Location reports | Shooting Days now in **ascending shoot-day order** |
> | DOOD, Budget, Moodboard, Daily View, Breakdown/Catalog, other reports | unchanged |
>
> Each changed export's `.md` has a "Changes from v1" subsection. `00_SYSTEM/QA_and_Validation_Report.md` §9 records the v2 checks.

## Start here

0. **Handing this to a coding agent?** Give it `00_SYSTEM/AGENT_HANDOFF.md` with this package — task, reading order, rules and checks in one page.
1. `00_SYSTEM/PDF_Design_System.md` — shared system: logo/footer rules, renderer constraints, colour, type, spacing, components, tables, pagination, open questions, promotion candidates, renderer gaps.
2. `00_SYSTEM/Brand_and_Footer_Reference.pdf` (+ `.html`) — the system on paper (logo use, footer geometry, palette, components).
3. `00_SYSTEM/Export_Inventory.csv` — every export variant → UI name, scope, HTML/PDF/MD paths, stress tests, op/builder/UI entry point, page size, verification status, limitations.
4. `00_SYSTEM/Source_Map.md` — what was inspected, commit reconciliation (audit pack `324cd96` → `main` `2f5a045`), UI entry points, builders, renderer.
5. `00_SYSTEM/QA_and_Validation_Report.md` — what was verified, what was not.
6. The export you are implementing: `exports/<family>/<Name>.md` (spec) + `.html` (layout reference) + `.pdf` (rendered reference).

## Contents

| Family | Variants (each = `.html` + `.pdf` + `.md`) |
|---|---|
| `exports/story-beat-board/` | Beat_Board_Outline · Beat_Board_Card_Board |
| `exports/breakdown/` | Breakdown_By_Scene · Breakdown_And_Catalog_Combined · Catalog_With_Scene_Usage |
| `exports/moodboard/` | Moodboard |
| `exports/storyboard/` | Storyboard_Sheet (2 per row) · Storyboard_Sheet_3col.pdf (3 per row, same HTML) |
| `exports/shot-list/` | Shot_List_Single_Scene · Shot_List_Multiple_Scenes · Shot_List_Entire_Project |
| `exports/shooting-schedule/` | Shooting_Schedule |
| `exports/dood/` | Day_Out_of_Days |
| `exports/daily-view/` | Daily_View (**no baseline PDF existed — not validated against a real export**) |
| `exports/call-sheet/` | Call_Sheet |
| `exports/production-reports/` | Report_Scene · Report_Location · Report_Cast_Scene · Report_Prop · Report_Schedule · Report_Breakdown_Completeness |
| `exports/budget/` | Budget_Summary (current budget and saved-snapshot identity) |

Each family folder also has `stress-test/` with clearly named `*_STRESS_TEST*.html/.pdf` edge-case renders (fictional data: long text, many rows, missing images, empty states, wide tables). They are QA evidence, not the primary reference.

Other folders: `00_SYSTEM/assets/` (shared stylesheet, preview script, **official logo** cropped to artwork bounds + the unmodified original), `00_SYSTEM/sample-assets/` (real images extracted from the supplied PDFs), `tools/` (render + verify scripts; `tools/build/` holds the generators that produced the references — provided for transparency, some contain machine-specific paths to the supplied PDFs and are not a portable build).

## Key design decisions (all explained in the specs)

* **One system, document-specific identities.** Navy/orange Helvetica design language already used by OpenFrame, sharpened (6.6 pt type floor, contrast ≥ 4.5 : 1 for text, status never by colour alone).
* **Footer on every page:** official logo (17 pt high) bottom-left, `Created using OpenFrame` directly below, `Page X of Y` bottom-right (real values). No footer rule, no project text, no repeated logo header. Content bottom margin 66 pt.
* **Running header:** mandatory only on the Call Sheet — `LOW TIDE RADIO · SHOOT DAY 01 · REVISION 2` (top-right, no rule). Other exports use a small identifying line only where it helps a long printed document; each spec states its decision.
* **Dynamic content:** content flows by measured height; optional sections disappear without gaps; rows grow; table titles + headers repeat; headings stay with content; rows/cards never split; images are never cropped.
* **Structure preserved** from source and the supplied PDFs; every deliberate change in grouping/order is listed in the export's MD section E with its reason.

## Things the implementer must know first

* The current renderer draws a vector mark and typed wordmark in the footer; the official PNG must be embedded (ideally lossless) — `PDF_Design_System.md` §2.
* New footer geometry changes every export's pagination (66 pt vs today's 80 pt bottom clearance).
* Several designs need renderer features that do not exist yet — `PDF_Design_System.md` Appendix B.
* Chrome decided page breaks in the references; the Rust pager must implement the documented rules (section H of each spec).
* The Windows tree was used; the `mac/` tree has its own renderer files and was not compared.

## Re-rendering

```
python tools/render_pdf.py exports/call-sheet/Call_Sheet.html exports/call-sheet/Call_Sheet.pdf
python tools/verify_pdf.py exports/call-sheet/Call_Sheet.pdf --header "SHOOT DAY 01"
```
Needs Python 3 + PyMuPDF and Chrome/Chromium/Edge (≥ 131; built with 155). Open any `.html` in a browser: a screen-only toolbar toggles optional sections/variants and offers *Print / Save as PDF*; it never prints.
