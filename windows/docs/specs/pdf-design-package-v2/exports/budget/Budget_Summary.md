# Budget Summary — PDF design handoff

| | |
|---|---|
| Export | Budget summary — op `budget.export_summary` (PDF · XLSX · CSV; this spec covers the PDF) |
| Variant | Portrait. One trio: `Budget_Summary.html/.pdf/.md` + `stress-test/Budget_Summary_STRESS_TEST.html/.pdf`. **Current budget** and **saved snapshot** identities are `data-toggle` variants in the same HTML (mutually exclusive toggles "Current budget identity" / "Saved snapshot identity"; "Line notes" toggles the notes under line items) |
| Status | Design reference — not integrated; not validated against OpenFrame's Rust renderer |
| Repo commit | `2f5a045416bd2125801329533fc47f986ccc8174` (`main`, Windows tree) |
| Date | 2026-10-23 |
| Generator | `tools/build/matrix-finance/build_budget.py` + `budget_lib.py` (ports `amount_text`/`money`/`group`, `sum`, contingency and category roll-up from the Rust source; asserts the Rust unit-test values and checks the sample's totals against the supplied PDF) |

---

## A. Purpose and users

- **Producer / line producer**: "What did we plan, what have we spent, are we over, how much contingency is left to allocate?" — first page answers it.
- **Production accountant**: every line with planned, actual and difference to the paisa, category subtotals that foot, a total that matches the screen.
- **Financier / completion guarantor / co-producer** (shared copy): a clean, honest estimate with the disclaimer, clear current-vs-snapshot identity and date.
- **Director / HODs**: find their department's lines quickly.

Find fast: planned total, contingency, actual so far, under/over plan; category rows; the line (department-prefixed description) with its numbers aligned in columns; which lines are over; which budget this is (current or which snapshot, saved when).

## B. Verified functionality

**Scope.** The *current* budget (`budget_snapshot.is_current = 1`) or, when `budgetId` is passed, one *frozen snapshot*. Amounts are exported exactly as entered; nothing is recalculated beyond the same totals the Budget view shows. A line without an Actual leaves Actual and Difference empty and never counts as "under plan" (builder doc comment).

**Inputs / options** (`ExportBudgetArgs`, `deny_unknown_fields`):

| UI | arg key | Default |
|---|---|---|
| Format: PDF / CSV / Excel (XLSX) (`TABLE_FORMATS`) | `format`, `path` | — |
| (snapshot dialog button only) | `budgetId` | omitted = current budget; dialog `documentName` becomes "Budget Snapshot" |
| checkbox "Include line notes" | `includeNotes` | `false` |

Dialog title "Export Budget"; sub "A simple budget summary: planned total, contingency, category totals and line items." Dialog `documentName` is "Budget Summary" (current) or "Budget Snapshot" (`budgetId`).

**Formats / data.** PDF: masthead, 4 metrics, reconciliation sentence, "By category", "Line items", "Notes". XLSX: 3 sheets in this order — "Line items" (`Category, Line item, Planned (CUR), Actual (CUR), Difference (CUR)[, Notes]`), "Categories" (`Category, Lines, Planned, Actual, Difference`), "Summary" (`Item, Amount (CUR)`: Planned total, Contingency, Planned in lines, Actual so far, Difference (lines with an actual)). CSV = first table = line items. Spreadsheets carry **plain** numbers (`amount_text(…, grouped=false)`), the PDF grouped ones.

**Currency formatting — verified against `exports/mod.rs`** (unit test `money_groups_like_the_budget_view` re-asserted in the generator):
- Stored as integers in minor units. Digits: `JPY`, `KRW` = 0, everything else (including unknown codes, `PCT`) = 2.
- INR groups the integer part as 3 then 2-digit groups: 120 000 000 paise → `INR 12,00,000`; other currencies group by 3: 150 050 → `USD 1,500.50`; JPY 1 500 → `JPY 1,500`.
- Decimals are printed only when the fractional part is non-zero (PDF/grouped), so `USD 1,500` and `USD 1,500.50` can sit in one column. Negative: leading `-` (ASCII hyphen-minus) before the grouped integer: `-0.05`.
- `Difference` = `planned − actual` (positive = under plan, negative = over); contingency percent = `(base × bp + 5000) / 10000` with `base = planned_total` or, when not set, planned-in-lines; "percent" values are basis points (1000 = 10 %, printed `10%`; 1250 → `12.50%`).

**Empty / error behaviour.** No budget → `nothing_to_export` "There is no budget to export yet."; unknown/deleted snapshot id → not found; no lines → the "By category" section prints the sentence "No lines yet." and "Line items" is omitted; `Notes` section only when the budget notes are non-blank; `Planned total` unset → metric "Not set"; lines in a category the app no longer lists are exported under "Other". Capability `Export` required.

### B.1 Current-output assessment (supplied `Budget — Summary.pdf`, 4 pages)

**Retain**: eyebrow "Production · Budget"; title; project line; the disclaimer "A simple estimate for planning. Formal accounting is outside OpenFrame."; right-hand identity panel (Currency, Current budget / Saved snapshot, Saved/Exported date); the four metrics and their labels; the reconciliation sentences; By category (dot, count, share bar, Planned/Actual/Difference, navy "Total in lines"); Line items grouped by category with the note under the description; Notes callout; right-aligned numerals; `-1,40,000` style negatives.

**Improve visually**: numeral alignment across both tables (same three numeric column widths), clearer category/line/total hierarchy, variance treatment that works in grayscale, decimal alignment, category band that repeats on continuation pages, restrained colour (one tinted cell + text for "over").

**Verified functional / layout problems**
1. **Category lost across the page break**: Line items for Cast start at the bottom of p. 1 (3 lines) and continue on p. 2 with *no* category band — the first rows on p. 2 ("Child performers…", "Extras…", "Animals…") appear under only "LINE ITEMS (CONTINUED)". Same risk for any split category.
2. **Not comparable columns**: in By category, Planned sums all lines but Actual/Difference sum only lines that have an Actual (e.g. Crew: Planned 70,80,000, Actual 3,50,000, Difference 10,50,000 ≠ 70,80,000 − 3,50,000). Nothing on the page says so (p. 1). The semantic is documented only in the Rust doc comment.
3. **Blank vs zero**: lines without an Actual print empty Actual/Difference cells; a real zero difference prints `0` (e.g. Post/Other, p. 3). Blank cells in a wide table are ambiguous.
4. **Over-plan is colour-led**: negative difference = bold red text only (`-1,40,000`, `-15,000`); in grayscale it is bold and signed but not labelled.
5. **No category subtotal inside Line items**; the only totals are in the By-category table on p. 1, so a reader of pp. 2–3 cannot check a category without flipping back. The grand total appears only on p. 1.
6. **Last page nearly empty**: p. 4 holds only the Notes callout.
7. **Raggedness risk**: because `amount_text` drops `.00` for whole amounts, a column mixing `1,500` and `1,500.50` is not digit-aligned when right-aligned (not visible in the INR sample, which has no paise; shown in the stress test).
8. **Name inconsistency**: PDF title/footer "Budget Report" (`render()`), dialog `documentName` "Budget Summary", supplied file name "Budget — Summary". For snapshots the title is "Budget Snapshot: {label | saved date | "saved"}".
9. **Clock in the document**: the masthead prints `Exported {today}` (`today_label()`), so the file is not reproducible byte-for-byte (the DOOD export, by contrast, uses the data's last-change date).
10. **Metric width**: "10% · INR 46,00,000" and "INR 4,60,00,000" fit a quarter-width card only at current values; crore-scale amounts would overflow the 15 pt value.
11. **Two meanings of "Contingency"**: one of the nine fixed line categories is "Contingency" (`BUDGET_CATEGORIES`), and the Contingency metric is a separate setting (percent/amount). Nothing in the PDF relates them.
12. "Contingency" and "Other" categories get the neutral grey `0x8792a5` in `budget_color()` (no distinct colour).

**Cannot be determined from the evidence**: whether any real budget has a category named outside the nine (printed as "Other"); whether a negative `planned` is possible (validation not read) — the share bar is clamped to 0–100 % in this design; Mac behaviour (not compared).

## C. Source mapping

| Item | Value |
|---|---|
| UI entry points | `features/export/buttons.tsx` → `BudgetExportButton({ budgetId })`; mounted in `workspaces/production/tabs/budget.tsx`: (1) `PageHeader` actions, no `budgetId` (current budget, before "Save snapshot"/"Add Line"); (2) `SnapshotDialog` footer with `budgetId={id}` (saved snapshot, read-only dialog) |
| Op / builder | `budget.export_summary` → `modules/exports/budget.rs::export` → `load()` + `render(project, b, notes)`; totals `sum()`; categories `modules/schedule/budget.rs::BUDGET_CATEGORIES` (Cast, Crew, Locations, Equipment, Art/Props, Travel/Transport, Food, Post/Other, Contingency) |
| Data | `budget_snapshot` (label, currency, planned_total, contingency_mode/value, notes, is_current, frozen_at), `budget_line` (category, description, amount=planned, actual, notes) ordered by `position, created_at, id` then stable-sorted by category order |
| Renderer blocks used today | `Masthead` (+`Side::Panel`, `rule`), `Metrics` (+`Tone::Ok/Danger`), `Block::Text`, `Section`, `StyledTable` (`boxed`, `HeaderStyle::Tint`, `continued`), `Cell` / `CellItem::{Heading, Bar, Text}`, `Row::spanning` (category band, `keep_next`), `Callout` (`Tone::Info`); money via `mod.rs::{money, amount_text}` |

## D. Field inventory

| Visible element | Source | Kind | Empty treatment |
|---|---|---|---|
| Eyebrow "Production · Budget" | literal | fixed | — |
| Title | current: "Budget Report"; snapshot: "Budget Snapshot: {label}" (label, else saved date, else "saved") | derived | — |
| Project line | `project_title` | conditional | omitted |
| Disclaimer | literal | fixed | — |
| Identity panel | "Currency {CUR}"; "Current budget" / "Saved snapshot"; "Saved {date}" (snapshot only, `frozen_at`); "Exported {today}" | derived | Saved line omitted if no `frozen_at` |
| Metric "Planned total" | `planned_total` → `money()` | conditional | "Not set" |
| Metric "Contingency" | percent: `{pct}% · {money}`; amount: `{money}` | derived | — |
| Metric "Actual so far, N lines of M" | `sum(lines).actual`, `actual_count`, `lines.len()` | derived | `money(0)` |
| Metric "Under plan / Over plan, lines with an actual" | `abs(difference)`; tone Ok when ≥ 0 else Danger | derived | — |
| Reconciliation | "Planned in lines: {money}." + ("Planned total not yet in lines: …" if planned_total ≥ lines, else "Lines over the planned total: …") | derived | second sentence only if `planned_total` set |
| By category row | category colour dot, name; line count; share bar = planned / max(lines planned, 1); Planned (bold); Actual and Difference only if the category has an Actual | derived | empty → "—" (design) |
| `n actual` under the count | `Sum.actual_count` | derived (design addition) | omitted at 0 |
| "Total in lines" | `sum(all lines)`, count | derived | Actual/Difference "—" if none |
| Line item | `description` (bold), note (only with `includeNotes` and non-blank) | repeatable | — |
| Line Planned / Actual / Difference | `amount`, `actual`, `planned − actual` | required / optional / derived | Actual and Difference "—" (source: empty) |
| Category subtotal row | `sum(category lines)` | derived (design addition) | Actual/Difference "—" |
| "Total in lines" at end of Line items | same grand total | derived (design addition) | — |
| Notes callout | `budget_snapshot.notes` | conditional | section omitted |
| Empty state | "No lines yet." | conditional | — |

## E. Content architecture

Order (unchanged from source): Masthead → Metrics → reconciliation sentence → **By category** → **Line items** → **Notes**.

Changes versus the current PDF, with reasons:
1. **By category columns re-ordered**: Category · Lines · Share · Planned · Actual · Difference (the source puts the share bar last). Reason: the three money columns now sit at exactly the same x-positions as in Line items (84 / 84 / 92 pt), so a reader can run a finger from category total to line.
2. **Line items = one small table per category**, each with its category band *inside the repeating header* (name + line count + the three money column labels). Reason: the category (and the column labels) reappear automatically when a category crosses a page; fixes defect 1. The columns are the same four as the source (Line item, Planned, Actual, Difference).
3. **Category subtotal row** (derived with the same `sum`) at the end of each category, and a **grand "Total in lines"** at the end of Line items. Reason: defect 5; the numbers equal the By-category row and total.
4. **"—" for no Actual** (source: empty cell) in Actual/Difference. Reason: blank vs zero (defect 3). A legend sentence under By category explains it.
5. **"How to read the amounts" note** under By category: Planned adds every line; Actual and Difference add only the lines with an Actual; Difference = Planned − Actual; positive = under plan; minus sign = over plan. All of it is source semantics (builder comment, `BudgetLineDto` comment).
6. **"n actual"** under the line count (from `Sum.actual_count`).
7. **Contingency metric**: the percent moves into a small "10%" chip beside the label; the value is the amount `INR 46,00,000`. The same two facts, re-laid so the value stays on one line at crore scale.
8. **Currency code** printed as a small prefix before the metric value (`INR 4,60,00,000`) so large figures shrink-to-fit.
9. Running header added (see F). Notes stay last.

Table columns (portrait content width 520 pt):

| Table | Columns (pt) |
|---|---|
| By category | Category flex (≈ 130) · Lines 46 right · Share of planned 84 · Planned 84 right · Actual 84 right · Difference 92 right |
| Line items (per category) | Line item flex (≈ 260, wraps, bold navy; note 7.8 pt slate below) · Planned 84 · Actual 84 · Difference 92 (all right) |
| Total in lines | label flex · 84 · 84 · 92 |

## F. Visual specification

- **Page**: US Letter portrait 612×792, margins 46/46/46/66 pt. Footer from the shared system (logo PNG, "Created using OpenFrame", "Page X of Y").
- **Running header**: yes, `@top-right "LOW TIDE RADIO · BUDGET · INR"` (project · "BUDGET" · currency). Reason: financial pages travel loose and must identify project/currency on every page. The text is deliberately identical for current and snapshot (CSS `@page` cannot switch by state); snapshot identity is in the masthead, and the renderer could append the snapshot label.
- **Type**: title 26 pt bold navy; project 11 pt slate; disclaimer 8.4 pt; metric value 14 pt bold (currency prefix 7.8 pt bold), label 6.8 pt tracked caps; table body 8.4 pt, line description 8.8 pt bold navy, note 7.8 pt slate; category band name 10 pt bold, count 7.4 pt; column labels 6.9 pt tracked caps; subtotal label 7.4 pt tracked caps / numbers 8.4 pt bold; section headings = shared `.sec` (10 pt tracked caps, navy rule).
- **Numerals**: Helvetica has no tabular figures flag in the renderer, but digits are equal-width in Helvetica/Arial; right-aligned, no wrap.
- **Colour**: navy ink and slate; category dots from `budget_color()` (decorative; the name is always printed); Under-plan metric `--of-green-*`, Over-plan metric `--of-red-*`; over-plan *line* cells: `--of-red-fill` background, `--of-red-text` bold, plus the text "OVER PLAN" (6.6 pt bold tracked caps) under the signed number. **Grayscale redundancy for variance**: minus sign (source), bold weight, shaded cell, the words OVER PLAN. Under-plan values are plain (no "+" — the source prints none; adding one would change the exported text).
- **Hierarchy**: line rows regular weight → subtotal rows tinted `--of-panel` with 0.75 pt rules and bold figures → "Total in lines" navy bar with white bold figures.
- **Share bar**: 5 pt rounded rect, track `--of-tint`, fill `--of-slate` (source uses the category colour; either is acceptable — the bar is proportional only). Fill clamped 0–100 %.
- **Decimal alignment** (design rule): when any amount in a table prints decimals, amounts that print none are right-shifted by the width of ".00" (Helvetica digits are all the same width, so `.50` and `.00` match). The printed text is unchanged. Demonstrated in the stress test.
- **Metrics**: 4 equal cards, `--of-panel` fill, 0.5 pt border, 4 pt radius; value font steps down (14 → 12 → 11 pt) until it fits on one line; if it still does not fit, the currency prefix moves above the number (renderer rule — Chrome in this reference does not shrink).

## G. Dynamic behaviour

- **Many categories / lines**: each category is its own table; band + column labels repeat on continuation; subtotal and last line stay together; the first two lines stay with the band; categories with ≤ 3 lines never split.
- **Long names / notes**: wrap, rows grow; nothing is ellipsised; unbroken tokens wrap (`overflow-wrap:anywhere`); notes print only when "Include line notes" is on **and** the note is non-blank.
- **Large values** (crore, hundred-crore, 10⁹ minor units in the sample): numerals never wrap or clip; the stress test prints `12,45,67,891.50`, `22,92,65,513.83`.
- **Negative variance / zero rows**: signed, shaded, labelled; a zero difference prints `0` (never blank); a row with planned 0 prints `0`.
- **No actuals anywhere**: Actual/Difference columns show "—"; metrics: Actual `INR 0` ("0 lines of N"), Under plan `INR 0`.
- **Planned total unset**: metric "Not set"; reconciliation shows only "Planned in lines: …". **Planned total below lines**: "Lines over the planned total: …" and the Over-plan metric is red when `difference < 0`.
- **Unknown category**: lines roll up under "Other" (stress test). **Contingency category** prints like any other category (neutral grey dot).
- **No lines**: "By category" + "No lines yet."; no Line items; Notes if present.
- **Snapshot**: title "Budget Snapshot: {label}" (long labels wrap), panel with a "SNAPSHOT" tag, "Saved snapshot", "Saved {date}", "Exported {date}". The sample label "Pre-shoot lock" is **fictional**; figures in the snapshot variant are the same sample numbers (identity-only demo).

## H. Pagination and print rules

- Portrait Letter; header repeat via `<thead>` per category table (band + labels); rows `break-inside: avoid`; `tbody.k` groups (first two rows; last row + subtotal) `break-inside: avoid`; By-category table, metrics, reconciliation, Notes callout and the grand total are not split; section headings `break-after: avoid`.
- Min 2 lines under a category band before a break; subtotal never alone.
- Primary sample: 3 pages (current output: 4); page 3 ends with the grand total and the Notes.
- Footer on every page; real `Page X of Y`.

## I. HTML / PDF correspondence

- `Budget_Summary.html` = the PDF content. Data: the supplied PDF's 36 lines (INR; re-expressed in paise), all 8 category totals and the grand totals re-computed by the generator and asserted equal to the printed values (planned in lines INR 3,78,75,000; actual INR 60,15,000 over 10 lines; difference INR 36,85,000; contingency 10 % = INR 46,00,000; planned total INR 4,60,00,000).
- Toggles: *Current budget identity* ↔ *Saved snapshot identity* (mutually exclusive, via a small preview-only script), *Line notes*. PDF captured: **current budget, line notes on**.
- Stress test (fictional): 56 lines in 10 categories (incl. Contingency and a legacy "Marketing (older file)" → Other), crore-scale and 0.02-scale amounts with paise (decimal alignment), zero rows, negative variances, 180-character names, a ~140-character unbroken token, ~400-character notes, planned total below lines (red Over-plan metric + "Lines over the planned total"), a 12.50 % contingency, a snapshot title with a very long label, a currency-format reference table (INR/USD/JPY/KRW) and the "No lines yet." fragment → 7 pages.
- Render: Chrome 155 headless via `tools/render_pdf.py`; `tools/verify_pdf.py --header "BUDGET"` passes (3 and 7 pages); grayscale thumbnails inspected.
- Known differences vs the Rust renderer: Chrome substitutes Arial for Helvetica; the renderer will measure and wrap with its own metrics; per-category repeating header rows, keep-together row groups and the shrink-to-fit metric value are new behaviours (K).

## J. Acceptance criteria

1. For any input, `Planned (lines)`, `Actual`, `Difference`, counts in By category and the grand total equal `sum()` of the lines; each subtotal equals the By-category row of the same name; `Total in lines` appears in By category and at the end of Line items with identical numbers.
2. Amount text equals `amount_text(v, currency, grouped=true)`: `INR 12,00,000`, `USD 1,500.50`, `JPY 1,500`, `-0.05`; no rounding; contingency = `(base·bp+5000)/10000`.
3. All numerals right-aligned; Planned/Actual/Difference columns in By category and Line items share x-positions; a table that mixes whole and decimal amounts is digit-aligned.
4. A line without Actual shows "—" in Actual and Difference and is excluded from Actual/Difference sums and from "lines with an actual"; a real zero difference prints `0`.
5. Negative differences: `-` sign, bold, shaded cell and the words OVER PLAN; verified legible in a grayscale render.
6. A category that crosses a page repeats its band (name + count + column labels) at the top of the next page; the subtotal row is never separated from the last line.
7. Title/identity: current → "Budget Report" + "Current budget"; snapshot → "Budget Snapshot: {label}" + "Saved snapshot" + "Saved {date}"; running header and footer on every page.
8. Disclaimer sentence present verbatim; Notes only when notes exist; "No lines yet." when empty; "Not set" when no planned total.
9. Line notes appear only with `includeNotes = true`.
10. Long names/notes wrap and rows grow; no clipping at 520 pt width; metrics fit on one line or reflow per F.
11. No page has content in the footer band; no orphan category band; last page not a lone heading.

## K. Known issues and unresolved questions

**Verified defects in the current output**: items 1–12 in B.1.

**Design choices**: columns re-ordered in By category; per-category tables and subtotal rows; "—" for no Actual; "OVER PLAN" label; "How to read" note; contingency chip; running header; no "+" on positive differences (to keep the exported text).

**Unverified assumptions / open questions**
- Whether the "Contingency" *category* and the Contingency *setting* are meant to be combined anywhere (the source does not combine them; this design keeps them separate and labelled as in the source).
- Whether the exported date should stay in the masthead (clock-dependent) — kept because it is in the current output and the Budget view; reproducibility concern flagged in B.1 item 9.
- Whether "Budget Report" or "Budget Summary" is the intended document name (source builder vs dialog vs file name); this reference keeps the builder strings.
- Shrink-to-fit of metric values and per-category repeating headers need renderer support; Chrome only approximates them.
- The share bar uses planned / max(lines planned, 1); negative planned amounts were not investigated.
- Mac app not compared.

**Promote to shared CSS (suggestions)**: `.dp` decimal-alignment spacer; `.ovr` variance tag + `td.over`; `table.li` category-band-in-thead table pattern with `tbody.k` keep groups; `.bar` share bar; `.idpanel` identity panel with a tag; `.fn` footnote style.
