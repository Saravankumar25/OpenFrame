# OpenFrame Studio — PDF Design System (v1.0 system; package v2)

Status: **design reference for the coding assistant — not integrated into OpenFrame, not validated against the Rust renderer.**
Evidence base: repository `Saravankumar25/OpenFrame` `main` @ `2f5a045416bd2125801329533fc47f986ccc8174` (Windows tree), the 19 supplied PDF exports, and the earlier audit pack (older commit `324cd96…`, used only as a lead and re-verified).
Visual companion: `Brand_and_Footer_Reference.html / .pdf`. Machine-readable tokens and components: `assets/openframe-print.css`.

---

## 1. What this system is — and is not

* It extends the **existing** production-document design language that already ships in `windows/crates/openframe-import-export/src/report/design.rs` (navy ink on white, orange accent, restrained status colours, Helvetica, flat fills and hairlines). It sharpens it (type floor, contrast, grayscale safety, footer, pagination rules) rather than replacing it.
* It covers **production PDFs** produced by the `modules/exports/*` builders. It excludes Screenplay, Sides, the Story page and any Breakdown Summary (not a user-reachable export — see `Source_Map.md`).
* Every design stays inside what the Rust renderer can draw (§3). The HTML references are a *visual and layout specification*, not an implementation.

## 2. Logo and footer (non-negotiable)

| Rule | Value |
|---|---|
| Artwork | `assets/openframe-logo.png` — the user-supplied official logo, **cropped only to its artwork bounds** (699 × 163 px). Unmodified original: `assets/openframe-logo-original.png` (812 × 347). No vector/SVG version of the lockup exists in the repository (the repo's `BrandLogo.tsx` is a UI re-creation of the mark + typed wordmark and the website favicon is the mark only), so the supplied raster is the only authoritative asset. |
| Use | Footer, bottom-left, every page of every production PDF in scope. Never in a header. Never redrawn, typeset, recoloured, stretched, cropped into the artwork. |
| Size | 17 pt high × 72.9 pt wide (ratio 4.289 : 1). Minimum 14 pt high. |
| Brand line | `Created using OpenFrame` (the constant `BRAND_LINE` in `report.rs`) 6.8 pt regular, slate `#566277`, directly under the logo, left edges aligned. |
| Page number | `Page X of Y`, 7.4 pt regular ink, lower-right margin, same baseline as the brand line. X, Y = real page number / real page count. |
| Position | Logo bottom edge 33 pt above the page bottom; brand-line baseline ≈ 22 pt above the bottom edge. No footer rule. No project/document string. |
| Embedding | Embed once as a lossless image and reuse on every page. (`ReportImage::decode` converts to JPEG q85 — fine for photos, not for the wordmark; add a lossless path.) |
| Running header | **Call Sheet only (mandated):** one compact line, top-right, `PRODUCTION · SHOOT DAY NN · REVISION n`, no rule, no logo. Other exports: a single small top-right line only where it genuinely identifies pages of a long printed document; decided and specified per export. |

Footer geometry replaces the current one (`design::draw_footer`: hairline rule + vector `lockup()` + bold tracked footer text). Today the content area ends at `MARGIN + FOOTER_SPACE` = 80 pt above the page bottom (`report.rs`); this system uses a **66 pt** bottom margin (14 pt more content per page). Re-derive both constants together; existing pagination/golden PDFs will change.

## 3. Renderer constraints that shape every design (verified in source)

* Writer: `pdf-writer` via `openframe-import-export/src/pdfgen.rs`; documents are `report::Document` = ordered `Block`s (`Masthead`, `Metrics`, `Section`, `Banner`, `Callout`, `Legend`, `Cards`, `StyledTable`, `Grid`, `Text`, …). Text is laid out with the same Helvetica metrics that paint it (wrapping is exact; nothing is ellipsised).
* Fonts: PDF **base-14 Helvetica** (regular + bold); text the base font cannot print triggers embedded fallback fonts, or the `export.pdf_unsupported_characters` error (`HINT_*` strings in `exports/mod.rs`). No web fonts, no italics system.
* Drawing vocabulary: text (size, bold, colour, tracking), rects/rounded rects (fill + stroke), lines, circles, JPEG images, the vector mark. **No** shadows, gradients, opacity, rotated text, blend modes, SVG, hyperlinks styling.
* Existing invariants to preserve: measure = paint; rows grow, text wraps; table header repeats; styled rows move whole to the next page; cards move whole (tall cards have a split strategy); deterministic bytes (no clock/random in layout); footer identical on every page.
* Paper: US Letter, portrait 612 × 792 pt, landscape 792 × 612 pt (all supplied PDFs). Whether A4 is ever wanted for production documents is an **open question** (screenplay export supports A4; production documents do not appear to).
* Units in the HTML are points so values map 1:1.

## 4. Page geometry

| | Portrait | Landscape |
|---|---|---|
| Page | 612 × 792 | 792 × 612 |
| Margins L/R/T | 46 / 46 / 46 | 46 / 46 / 46 |
| Bottom margin (footer zone) | 66 | 66 |
| Content area | 520 × 680 | 700 × 500 |
| Print-safe | nothing within 18 pt of an edge (footer text bottom edge is 22 pt) | same |

### Orientation per export (package v2)

Portrait: Beat Board (Outline and Card Board), Breakdown, Catalog, Storyboard, Shot List (all scopes), Shooting Schedule, Daily View, Call Sheet, Budget, and the five narrow reports (Location, Cast Scene, Prop, Schedule, Breakdown Completeness). Landscape: Day Out of Days, Moodboard, and the Scene report (7 columns; the application switches to landscape when a report has more than 5 columns). Changing a document's orientation is a builder change (`Document::landscape(...)` in the export module) as well as a layout change.

## 5. Colour

Tokens are the Windows `pal::*` palette; additions are limited to a *muted* text colour and explicit contrast rules.

| Token | Hex | Use | Contrast on white |
|---|---|---|---|
| `--of-navy` | `#1A2640` | structure: headers, rules, titles | 15.1 : 1 (white on navy) |
| `--of-ink` | `#1A2438` | body text | — |
| `--of-slate` | `#566277` | secondary text, labels | 6.2 : 1 |
| `--of-muted` *(new)* | `#66728A` | smallest readable metadata | 4.8 : 1 |
| `--of-faint` | `#8792A5` | **decoration only** (stripes, dots) | 3.1 : 1 ✕ text |
| `--of-line` / `--of-line-strong` | `#D8DEE8` / `#B9C2D1` | hairlines / card borders | — |
| `--of-panel`, `--of-tint` | `#F5F7FA`, `#E9EFF5` | zebra, card heads | — |
| `--of-teal` | `#2A6A8C` | secondary accent (ids, location tiles) | 5.9 : 1 |
| `--of-orange` | `#E07B1F` | accent fills, rules, bullets, stripes | **3.0 : 1 ✕ text** |
| `--of-orange-deep` | `#8A4F00` | orange text | 5.9 : 1 on `#FFF1E0` |
| Status triplets | amber `#FDF0D9/#EFC57E/#8A4F00`; red `#FDE9E4/#F0B3A4/#9A3822`; green `#E2F4E9/#A6D9B8/#1C6A3B`; blue `#E5F0F8/#B3CFE2/#245A7D` | fill / line / text | all text ≥ 5.7 : 1 on its fill (computed) |

Logo colours (gradient mark, orange "STUDIO") live inside the artwork and are not document palette.
**Breakdown category accents** (`pal::category`): Cast `#7A4CC2`, Extras `#4F9A3D`, Location/Set `#1B84BE`, Props `#C0623A`, Wardrobe `#1F9A8A`, Vehicles `#6B7A8F`, Hair/Makeup `#D6527A`, SFX `#D99A1A`, VFX `#4B63D1`, Sound `#C23A96`, Animals `#8A6A3B`. They are pure accents: always printed beside the category **name**.
**Grayscale rule:** no state, status, category, or code may depend on colour alone. Every status is a text label (NEEDS INPUT, DRAFT, FINAL, ESTIMATED, MEAL, REMOVED, SUGGESTED …) plus a tint/stripe; verify in a grayscale render.

## 6. Typography (Helvetica / Helvetica-Bold)

| Role | pt | Weight | Notes |
|---|---|---|---|
| Document title (portrait masthead) | 28 | bold, navy, leading 1.05 | up to 4 lines; shrink to 24 for long titles, never clip |
| Compact title (landscape / report shell) | 20 | bold, navy | |
| Section heading | 10 | bold caps, +0.1 em | 1.2 pt navy rule below; hint 7.4 pt slate right |
| Lead line (card) | 9.6 | bold | |
| Body | 8.8 | regular | leading 1.3 |
| Table cell | 8.4 | regular | leading 1.28 |
| Secondary | 8 | regular | |
| Label (caps) | 6.8 | bold, +0.1 em, slate | |
| Chip / status (floor) | 6.6 | bold caps, +0.06 em | nothing smaller than 6.4 pt anywhere |
| Footer | 6.8 / 7.4 | regular | |
| Metric value | 15 | bold | |
| Hero time / numeral | 24–30 | bold | call sheet only |

Numerals are tabular wherever columns align (Helvetica digits are tabular by design). Minimum practical size for printed text: 6.4 pt (chips/labels) and 8.4 pt (running text and table cells).

## 7. Spacing and shape

Scale (pt): **2 · 4 · 6 · 8 · 12 · 16 · 24**. Section gap 12; card padding 6/9; table cell padding 4.2/6 (compact 2.6/4); grid gap 8. Radii: 3 (chips/tiles), 4 (metrics), 6 (cards, callouts, day block). Rules: 1.2 pt navy section rule; 0.5 pt hairlines.

## 8. Components (all in `assets/openframe-print.css`)

| Component | Class | Key behaviour |
|---|---|---|
| Masthead | `.masthead`, `.eyebrow`, `h1`, `.badges`, `.daybox` | eyebrow + title + badges + meta line; optional dark side block; content area only |
| Section heading | `.sec` (`h2`, `.hint`) | `break-after: avoid` |
| Callout / banner | `.callout.{accent,warn,info,danger,ok}`, `.banner.*` | banner = stripe + bold text label (grayscale-safe) |
| Metrics | `.metrics > .metric` | equal-width tiles, label / value / sub |
| Cards | `.cards.c1…c4`, `.card`, `.head(.dark)`, `.body`, `.fields .row(.long|.missing)`, `.bullets` | `break-inside: avoid`; long values stack label-over-value |
| Table | `.tbl(.zebra,.boxed,.compact)`, `tr.title/.gap` in `thead`, `tr.group/.subtotal/.total/.tint-*`, `.stripe-*` | `thead` repeats; rows `break-inside: avoid`; title row travels with the header |
| Chips | `.chip(.info,.warn,.danger,.ok,.accent,.navy,.pill)`, `.tile`, `.needs` | text label always present |
| Images | `.frame.r169/.r43/.r11/.r66`, `.missing`, `.caption` | `object-fit: contain`, aspect preserved, flat panel placeholder with dashed hairline |
| Legend / note | `.legend`, `.note`, `.note-box` | |
| Preview shell | `.preview-bar`, `.no-print`, `data-toggle` | screen only |

Document-specific styling is added in a per-document inline `<style>`; promote to the shared file only when two or more documents need it.

## 9. Tables: columns, wrapping, continuation

* Column widths: fixed `pt` for narrow data (ids, times, pages, counts, thumbnails) and flexible `fr`/weights for prose; the prose column absorbs width. Header words must not break mid-word at the specified widths with the supplied sample data (verified by the stress tests).
* Numbers right-aligned; text left; status/centred codes centred; money right-aligned with a fixed decimal convention (`mod.rs::amount_text`).
* Rows grow with wrapped content (`overflow-wrap:anywhere`); no ellipsis; no max-height.
* Header repeats on every page; each table carries its **title inside the repeating header** so it can never be orphaned from its name. The renderer prints "· continued" after the title on continuation pages (CSS cannot; the HTML repeats the plain title).
* Break only between rows. Do not leave one row alone at the top of a page when two can travel together; do not leave a table title as the last element on a page.
* Group header rows (`tr.group`) keep with the first row of their group.
* Subtotal/total rows keep with the last two rows they summarise.

## 10. Cards, callouts, metrics

Cards move whole to the next page; a card taller than a page splits at field-row boundaries only. Card grids use `align-items: start` (cards keep their own height). Callouts/banners/metric rows never split. Missing values use the `NEEDS INPUT` marker where the application already prints "Needs input", otherwise an en dash "–"; optional absent values are omitted.

## 11. Images

Placed inside a fixed-ratio frame (16 : 9 default for storyboard/moodboard/shot thumbnails; 100 : 66 for call-sheet references) with `contain` fit on `#F5F7FA` — **never cropped, never stretched**. Captions below, 8 pt slate. Missing → flat panel frame (dashed hairline) with the application's own wording ("Image unavailable", "Image can't be printed"); a warning is returned to the UI, not printed. Embed JPEG, max edge as `ReportImage::MAX_EDGE`.

## 12. Pagination and dynamic content (summary; each export spec has the specifics)

Flow by measured height; no absolute positioning for flowing content; optional/empty sections vanish without a gap (except sentences the application already prints as empty states); sections expand; headings keep with content; rows/cards/figures never split; avoid single-row pages, orphan headings and nearly empty final pages; page numbers/total are computed from the real rendered page count; footer/header are page furniture in the margins and cannot collide with content.

## 13. Running metadata line policy

Mandated only for the Call Sheet. For other documents the per-export spec states whether a small top-right line (project · document · scope) helps. The default for **single-page documents is none**; for long multi-page tables (schedule, DOOD, breakdown, shot list, reports) a one-line `PROJECT · DOCUMENT` identifier is recommended so a loose page is attributable.

## 14. Accessibility and print

Contrast ≥ 4.5 : 1 for text; status redundantly encoded; grayscale review required; no text on images; ≥ 18 pt print-safe edge. PDFs should carry the document title (the Rust writer already sets `doc.title`; `b.finish(&doc.title, author)`).

## 15. How the HTML references are rendered

`tools/render_pdf.py` runs headless Chrome (v155 used here): `--headless=new --no-pdf-header-footer --print-to-pdf`. Page size, margins, the footer logo (background image of the `@page` bottom-left margin box), brand line and `Page X of Y` (`counter(page)` / `counter(pages)`) come from CSS `@page` rules in `openframe-print.css`; running headers are set per document in `@page { @top-right }`. `tools/verify_pdf.py` checks page size, footer on every page with the real total, logo aspect ratio, content never in the footer band / outside margins, header presence/absence, blank/near-empty pages, and can emit grayscale thumbnails. Mechanical checks complement, and do not replace, visual inspection.

## 16. Open questions for the product owner

1. Footer wording/position approved as specified? (Logo above brand line, left; page counter right.)
2. A4 support for production documents?
3. Lossless logo embedding in the Rust writer — confirm acceptable.
4. Should every long multi-page export get the small running metadata line (recommended) or only the Call Sheet?
5. Mac build parity (`mac/` has its own renderer files) — redesign Windows only, or port?

---

## Appendix A — Components defined locally in individual exports (candidates to promote into the shared system when implementing)

The shared stylesheet was frozen while the exports were designed in parallel. These patterns were defined inside individual HTML files and recur or are likely to recur; the implementer should build them once in the shared renderer primitives (`report/design/*`) rather than per export.

| Pattern | Where it appears | Purpose |
|---|---|---|
| Repeating *band-in-header* table (title / group band + column header both in `thead`) | Call Sheet, Breakdown, Catalog, Budget categories, Shot List scene groups, Shooting Schedule days | A group keeps its name and column labels on continuation pages; replaces the stand-alone "(CONTINUED)" bar |
| *Continued strip* (names the act / sequence / scene a page starts in) | Beat Board, Storyboard | Context for containers that span pages |
| Status chip with text + tone + shape mark (disc / ring / square) | Production Reports, Breakdown, Shot List | Grayscale-safe status |
| Compact masthead + "generated / scope" side panel; fit-content metrics strip | Production Reports, Shot List, Storyboard, Budget | Saves ≈ 60 pt on page 1 |
| Tinted facts band (day banner) | Shooting Schedule, Daily View | Key times at a glance, repeats on split days |
| Frozen-column matrix with month band and count rows; code swatch with edge caps | DOOD | Matrix legibility, grayscale-safe codes |
| Number tile / shot tile / beat tile | Beat Board, Shot List, Call Sheet | Stable identifiers |
| Variance tag + minus-sign convention + decimal-alignment spacer | Budget | Finance readability |
| Notes vs comments components (plain labelled note; boxed thread with status chip) | Beat Board | Distinguishes the two |
| Illustrative-sample banner | Daily View | Marks fictional sample data |
| Sentence-case card head (user text is never upper-cased) | Moodboard, Storyboard | Case of names/titles is preserved |
| Flat missing-image frame (no hatching) | all image exports | Renderer cannot draw gradients (shared CSS already flat) |

## Appendix B — Renderer capabilities the designs assume but the current Rust renderer does not (verified or flagged by the per-export specs)

* Embed the official logo PNG losslessly in the footer; footer geometry per §2 (66 pt bottom margin).
* A page-count-aware footer (`Page X of Y` already exists) and a **per-page running header** line (also per scene / storyboard where specified).
* Table fragments with **multi-row repeating headers**, header rows that carry the table/group title, and "keep first N / last M rows together" rules.
* Keep-whole-unless-taller-than-a-page groups (shot-list scenes, schedule days, budget categories, storyboard scenes).
* Frozen-column groups for wide matrices (DOOD), partial-colspan month band, cell edge caps/outlines.
* Shrink-to-fit single-line values (metric tiles with large amounts), decimal alignment.
* Whole-text wrapping of long card captions on the Beat Board board layout (the current `render_grid` truncates an over-long caption to one page — source-verified).
* Pagination measured from real heights (Chrome decides break points in the HTML references; the Rust pager must implement the documented rules itself).
