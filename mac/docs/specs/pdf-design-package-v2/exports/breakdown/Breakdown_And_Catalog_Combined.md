# Breakdown Report — Breakdown by scene and Catalog (combined)

| | |
|---|---|
| **Export** | Breakdown Report |
| **Variant** | Breakdown by scene and Catalog (`content="both"`, UI label “Breakdown by scene and Catalog”, default) |
| **Status** | Design reference — not integrated; not validated against OpenFrame's Rust renderer |
| **Repo commit** | `2f5a045416bd2125801329533fc47f986ccc8174` (main) |
| **Date** | 2026-10-23 |

## A. Purpose and users
The complete breakdown package in one file: **Part 1** the scene-by-scene sheets, **Part 2** the Catalog (every distinct item, with the scenes that use it). It is what a production manager sends when someone needs both views: the 1st AD / department heads for the per-scene work, and the producer, line producer and art/wardrobe/props buyers for the item list ("what do we have to source, and in how many scenes does it appear?"). It is the default of the Export Breakdown dialog.

## B. Verified functionality
| Entry point | File / component | Where it appears | What it sends |
|---|---|---|---|
| **Export Breakdown** dialog | `windows/apps/desktop/src/features/export/buttons.tsx` → `BreakdownExportButton({sceneId, scenes})`, shared `ExportDialog` (`title="Export Breakdown"`, `source="Source: the active Production Source"`, `documentName="Breakdown Report"`, `op="breakdown.export_report"`, `formats={TABLE_FORMATS}` = PDF / CSV / Excel (XLSX)) | Mounted in `workspaces/breakdown/index.tsx` (`<BreakdownExportButton size="xs" sceneId={sceneId} scenes={rows.map(...)}/>`) inside the Production Source info bar at the top of the Breakdown workspace, next to "Change source" | Scope radios: **All scenes of the Production Source** (no `sceneIds`), **Current scene** (`sceneIds:[sceneId]`, disabled when no scene is open), **Selected scenes** (picker → `sceneIds`; empty text "No scenes in the Production Source yet."). Options: choice **Contents** (`content`, PDF/XLSX only; default `both`; values *Breakdown by scene and Catalog* = `both`, *Breakdown by scene* = `scenes`, *Catalog with scene usage* = `catalog`); choice **Table** (CSV only, sends `content`; default `scenes`; *Breakdown by scene* / *Catalog with scene usage*); checkbox **Include suggested elements (marked as suggested)** (`includeSuggested`, default on); checkbox **Include element notes** (`includeNotes`, default off) |
| **Export Catalog** dialog | same file → `CatalogExportButton()` (`title="Export Catalog"`, `sub="The export is a snapshot, not a live connection to the Catalog."`, `source="Source: Catalog and the active Production Source"`, `documentName="Catalog"`, `baseArgs={{content:"catalog"}}`) | Mounted in `workspaces/production/tabs/catalog.tsx` as `<CatalogExportButton />` in the Catalog `PageHeader` actions, left of "Add Item" | Always `content:"catalog"`, whole Production Source (no scope selector, no options). The format list is re-ordered `[XLSX, CSV, PDF]`, so the dialog's initial format is **Excel (XLSX), not PDF** (`ExportFlow` uses `formats[0]`) |

**This variant = `content: "both"`** (Contents choice "Breakdown by scene and Catalog"; default). Available for PDF and XLSX only; CSV refuses it.


**Op:** `breakdown.export_report` — registered in `crates/openframe-application/src/modules/exports/mod.rs` (`r.query("breakdown.export_report", breakdown::export)`), implemented by `breakdown::export` in `exports/breakdown.rs`. Args struct `ExportBreakdownArgs` (`camelCase`, `deny_unknown_fields`):

| Arg key | Type | Meaning (from source) | Default |
|---|---|---|---|
| `format` | `"pdf"` \| `"csv"` \| `"xlsx"` | `Fmt::parse(..., [Pdf, Csv, Xlsx])` | required |
| `path` | string | destination (`destination(core, &a.path, fmt)`) | required |
| `content` | `"both"` \| `"scenes"` \| `"catalog"` \| `"summary"` | what the report holds | `both` for PDF/XLSX, `scenes` for CSV |
| `sceneIds` | string[] | selected scenes of the Production Source; absent = every scene; an empty array is rejected ("Select at least one scene to export.") | absent |
| `includeSuggested` | bool | keep elements whose `confirmation_state = 'Suggested'` | `true` |
| `includeNotes` | bool | print `breakdown_element.notes` under each element | `false` |

CSV with `content=both` is refused ("A CSV file holds one table. Choose Breakdown by scene or Catalog, or export Excel (XLSX) for both."). Any other `content` value is refused ("Choose Breakdown by scene, Catalog, both, or the short summary.").

**`content = "summary"` exists in the Rust code but is not reachable from the user-visible app.** `breakdown.rs` accepts it and routes to `export_summary()` ("Breakdown Summary": one page of scene-status totals and per-category counts; it ignores `sceneIds`, `includeSuggested`, `includeNotes`). I searched `windows/apps/desktop/src` (`features/export/buttons.tsx`, `workspaces/breakdown/index.tsx`, `workspaces/production/tabs/catalog.tsx`) and the whole `windows` tree for callers of `breakdown.export_report`: only the two buttons exist (`BreakdownExportButton`: Contents choice offers exactly *Breakdown by scene and Catalog / Breakdown by scene / Catalog with scene usage*; `CatalogExportButton`: `baseArgs {content:"catalog"}`). No UI option sends `summary`, so **no standalone "Breakdown Summary" export exists for users and none is designed here.** (The older audit pack's `content=summary` mention is accurate about the code but not about the UI.)

**Scope.** As in the scenes-only variant. **With a scene selection the Catalog part is filtered:** only catalog items used in the selected scenes are listed and their "Used in scenes" lists contain only the selected scenes (`wanted.is_some() && nums.is_empty() → None`); the Catalog metric counts those items. Without a selection every non-archived item is listed (including unused ones → "Not used yet").

**Contents.** Masthead (title **Breakdown Report**), metrics Scenes / Confirmed elements / Suggested (if on) / **Catalog items**, then "Breakdown by scene" cards, then — **in the same flow, no page break and no divider** — the "Catalog" section with one table per category. The document title, footer text and running label stay "Breakdown Report" (`document = "Breakdown Report"` unless `content=catalog`).

**Data origin.** `export()` runs one `session.store.read`: `active_source(c)` (Production Source draft; error `validation.no_production_source` "Choose a screenplay draft as the Production Source first. Your screenplay is not changed."), the screenplay title/draft name (`screenplay_draft` ⨝ `screenplay`), `SourceScenes::load` (rows: `id`, `number`, `heading`, `omitted`), `breakdown_element` rows (`scene_id`, `category`, `COALESCE(catalog_item.name, display_name)`, `confirmation_state`, `notes`; filter `deleted_at IS NULL AND archived = 0 AND confirmation_state <> 'Rejected'`, ordered by `created_at, id`, then stable-sorted by category order `BreakdownCategory::ALL`), `catalog_usage(c, &src_scenes)` (distinct `catalog_item_id, scene_id` pairs from **production states only**: `PRODUCTION_STATES = ('Confirmed','Manual')`, so suggested elements never count as catalog usage), and `catalog_item` (`id, category, name, status, description`; `deleted_at IS NULL AND archived = 0`). `project_title(c)` supplies the project name. Catalog rows are sorted by `(cat_order(category), lower(name))`; usage scene numbers come from `sort_refs` (numeric order, then heading).

### B.1 Current-output assessment (`Breakdown — Combined report and Catalog.pdf`, 58 pages)
Section order verified from the PDF: p1 masthead+metrics (4 tiles) → pp1-35 scene cards (identical to the By-Scene PDF) → p36 "CATALOG — Usage is derived from breakdown assignments" → per-category tables pp36-58 (Cast, Extras, Location, Props pp41-50 …, Animals p58).
**Retain** — the order (scenes first, catalog second), per-category catalog tables with a band (`● Category — N items`), the sentence "Usage is derived from breakdown assignments", the source's rule that the catalog follows the scope filter, "Not used yet", status chips.
**Improve visually** — see the scenes-only and catalog-only specs; additionally make the **part boundary explicit**.
**Verified functional/layout problems**
- **By Scene p1:** the intro page is ~two thirds empty (masthead, three metrics, section heading, one sentence, then nothing): the first scene card did not fit under the intro and, because cards move whole, jumped to p2.
- **By Scene p2→p3:** scene 2's card *head* (tile + heading) is stranded alone at the bottom of p2 and its body starts on p3 with **no head repeated**, so p3 shows Cast/Props/... with no scene number (also the same pattern in the combined PDF, pp. 2-3). Cause in source: `split_card()` treats the head as one unit and each `CardPart` as one unit and does not repeat the head.
- **Tag/pill attribution:** the `MANUAL` chip is a separate flowed chip and wraps independently of its pill: p2 "Ship-to-shore radio handset" / `MANUAL` on its own line; p3 Props "Wall chart of the coast with pins" has its `MANUAL` at the start of the *next* line, left of "Grease pencil" `MANUAL`, so it is unclear which pill owns which tag (also p18, p19).
- **Wasted area:** 3-column groups are laid out in rows of equal height with the accent bar stretched to the tallest group (`groups_frag`): p3 has a one-line Cast group beside a Props group about eleven lines tall; p19 leaves ~30 % of the page blank (whole-card moves). 45 scenes take 34 pages.
- **Omitted scene** (scene 22, p18): the heading is replaced by the word `OMITTED`, so the scene's original heading is not printed (source: `if *omitted { "OMITTED" } else { heading.to_uppercase() }`).
- **State legend** exists only as one sentence on p1 and only when suggestions are present; 588 of the 706 elements (83 %) carry `MANUAL` and only 7 carry `SUGGESTED`, so the common label dominates the page visually while `SUGGESTED` (7) is easy to miss.
- **Catalog density:** each row is ~58 pt high for a single-line item (the "USED IN SCENES" caption is repeated in every row, the status chip floats in a wide empty gutter, the count has its own caption). 321 items need 23 pages (≈14 rows per page).
- **"(CONTINUED)" bar printed above a fresh heading:** combined PDF p55 (`VFX (CONTINUED)` directly above the `VFX · 11 items` band) and p58 (`ANIMALS (CONTINUED)` above the `Animals · 5 items` band): the table moved whole to a new page, but the renderer (`StyledTable.continued`) still prints the continuation bar. (Catalog PDF p2-p23 show the intended use: a continuation bar above rows that continue.)
- **No column headers** (`HeaderStyle::None`): the meaning of each column is only given by per-row captions; a reader scanning the Used-in-scenes lists has no header to anchor on.
- **Long scene lists** (ANJALI: 36 scenes) wrap in a narrow column and push the row to 4 lines; there is no visual cue of how heavily an item is used other than the 15 pt count.
- **Status tone:** `Required` and `Confirmed` both use the green `Ok` tone, so 312 of 321 rows carry a green chip (visual noise; no information in the sample).
- **No part boundary:** nothing tells the reader that p36 starts a different kind of table; metrics at the top mix both parts without saying which belongs to which. The Catalog heading happens to land on a fresh page here (p36) only because the first table did not fit under p35.
**Cannot be determined from the evidence** — behaviour for a selection that matches no catalog item (source prints "The catalog is empty." — design keeps it; not seen in the supplied PDFs).

## C. Source mapping
Same as the two single-part variants. Renderer blocks: `Masthead`, `Metrics` (4 tiles), `Section("Breakdown by scene", hint)`, `Text` (state sentence), `Cards`, `Section("Catalog", hint)`, then per category `Block::Styled(StyledTable)` with `header = HeaderStyle::None`, `boxed = true`, `continued = Some(category)`, first row a `Row::spanning(Cell::item(CellItem::Heading{dot, text, right}))` with `fill = pal::PANEL` and `keep_next = true`; data rows with `Cell` items (`Txt`, `CellItem::Chips`). Empty catalog: `Block::Text("The catalog is empty.")`. Footer `footer(project, "Breakdown Report")`.
**Renderer changes this design needs:** everything listed for the scenes-only variant, plus: column header row (`HeaderStyle` other than `None`) repeated with the category band on continuation (and **no "(continued)" bar when a table starts on a fresh page** — the defect above); usage bar cell; part label in `Section`; running header; footer logo PNG.

## D. Field inventory
Masthead, metrics (+ **Catalog items** = `data.catalog.len()`), scene part: exactly the table in the Breakdown-by-scene spec (D). Catalog part:
| Visible element | Source | Kind | Empty / edge treatment |
|---|---|---|---|
| `PART 1` / `PART 2` labels (design addition) | literal | combined only | — |
| Section `Catalog`, hint `Usage is derived from breakdown assignments` | literals | fixed | — |
| Items per category strip (design addition) | count of rows per category | derived | omitted with the empty-state |
| Category band: dot, name, `N item(s)` | `catalog_item.category`, `pal::category`, `plural(n,"item","items")` | repeatable | categories with no item not printed |
| Item name | `catalog_item.name` | required | — |
| Description | `catalog_item.description` (`nb`) | optional | omitted |
| Status chip | `catalog_item.status` ∈ {Required, Searching, Shortlisted, Confirmed, Not Required} | required | neutral tone for unknown values |
| Used in scenes | scene numbers from `catalog_usage` ∩ scope, `join(", ")` | derived | `Not used yet` |
| Uses | `nums.len()` | derived | `0` (and `Not used yet`) |
| Usage bar (design addition) | `uses / scenes in scope` | derived | none when 0 |
| `The catalog is empty.` | literal | conditional (no rows after the scope filter) | shown instead of tables |

## E. Content architecture
Order: **Masthead → Metrics (Scenes, Confirmed, Suggested, Catalog items) → Part 1 "Breakdown by scene" (key + scene cards) → Part 2 "Catalog" (items-per-category strip, one table per category)**.
**Part separation (decision):** *continuous flow with an explicit part label*, **no forced page break**. This follows what the source does (one document, one flow) and avoids a mostly empty page when Part 1 ends near the page bottom; the boundary is made unmistakable by the `PART 1` / `PART 2` label (orange caps, 6.8 pt) in front of each section heading, an 18 pt top margin before Part 2, and by Part 2 changing from cards to tables with navy-ruled headers. The section heading keeps with the strip and the first table's first rows, so Part 2 starts a new page by itself when less than about 150 pt remain (this happens in the sample: Part 2 begins at the top of p6). If a hard split is wanted (e.g. to hand the catalog to purchasing), a "Start Catalog on a new page" option would be a product decision — not in the current UI and not invented here.
**Changes vs. current:** as in the two single-part specs (category rows for scenes; labelled, denser catalog rows with column header, usage bar and category strip); added part labels; the metrics are unchanged (four tiles, same labels).
Catalog sorting unchanged: category canonical order, then name case-insensitive; scene numbers ascending.

## F. Visual specification
### F.1 Page and furniture
- US Letter portrait, 612 x 792 pt; margins L/R 46 pt, top 46 pt, bottom 66 pt (footer zone). Content width **520 pt**.
- Footer (shared system, do not redraw): official logo lockup 17 pt high bottom-left, `Created using OpenFrame` below it, `Page X of Y` bottom-right (real computed values). **Implementation requirement:** the production renderer must embed the official logo PNG; today it redraws a vector mark.
- **Running header (decision: YES).** One compact line, top-right, 6.8 pt slate, tracked 0.04em, no rule, no logo: `LOW TIDE RADIO · BREAKDOWN REPORT` (= `footer(project, document)` text, uppercased). Reason: this is a many-page document (35-58 pages in the sample) that is torn apart on set; every sheet must say which production and document it belongs to. The masthead keeps the document identity on page 1; the header is not repeated as a second masthead.

### F.2 Typography (Helvetica regular/bold only)
| Element | Size / weight / colour |
|---|---|
| Eyebrow `PRODUCTION / BREAKDOWN REPORT` | 7.6 pt bold, +0.14em, `--of-slate` |
| Title `Breakdown Report` | 28 pt bold, navy, lh 1.05 |
| Project/source line | 8.6 pt, slate |
| Export-reference panel | label 6.8 pt bold caps; date 12 pt bold navy; scope 8.4 pt slate |
| Metric value / label | 17 pt bold navy / 6.8 pt bold caps slate |
| Section heading | 10 pt bold caps +0.1em navy over a 1.2 pt navy rule; hint 7.4 pt slate right |
| Part label (combined only) | 6.8 pt bold caps +0.16em `--of-orange-deep` |
| Scene head: tile `SCENE` / number | 6.6 pt bold +0.14em `--of-dark-muted` / 15 pt bold white, on navy |
| Scene head: `BREAKDOWN SCENE` / heading | 6.8 pt bold caps slate / 9.6 pt bold caps navy, lh 1.22 |
| Scene head: counts | `N elements` 9 pt bold navy; `N suggested` chip 6.6 pt |
| Category label | 6.9 pt bold caps +0.07em navy |
| Element name | 8.2 pt bold `--of-ink`, lh 1.22 |
| Element note | 7.6 pt `--of-slate`, lh 1.25 |
| State tag (`MANUAL`/`SUGGESTED`) | 6.6 pt bold caps +0.06em in a 3 pt-radius chip (the chip-size floor; the current renderer uses 5.8 pt, below the package floor) |

### F.3 Components
- **Export-reference panel** (replaces the current "date / Export reference / scope" side panel): 128-170 pt wide, 0.5 pt `--of-line-strong` border, `--of-panel` fill, 6 pt radius, padding 7/12/8, right-aligned. Content order unchanged from `production_masthead`: date (uppercased `today_label`), `Export reference`, scope text.
- **Metrics strip:** equal-width tiles, 6 pt gap, padding 5/9/6, 0.5 pt `--of-line` border, 4 pt radius, `--of-panel` fill. The **Suggested** tile uses the amber fill/line (`--of-amber-*`) and its label is amber-text; it is omitted when `includeSuggested=false`.
- **State key** (`key` box; printed only when `includeSuggested` and at least one suggested element exists — the same condition as the current sentence): 2 pt `--of-orange` left rule, `--of-panel` fill. Line 1 is the source sentence **verbatim**: "Suggested elements are not confirmed yet and carry a SUGGESTED label. Elements added by hand carry a MANUAL label." Line 2 is a visual key (design addition): `Name = confirmed (no label)` · `Name [MANUAL] = added by hand` · `Name [SUGGESTED] = not confirmed yet (tinted row)`.
- **Scene card** = one table per scene: head row (tint fill `--of-tint`, 0.5 pt `--of-line-strong` border, 6 pt top radius, padding 4.5/9/4.5/7, grid `auto | 1fr | auto`, column gap 9 pt) + one row per category (or per 12-element chunk of a long category).
  - Tile: min 34 pt wide, navy, 4 pt radius, padding 3/5/4. **Omitted scene:** tile is `--of-panel` fill, 0.5 pt `--of-line-strong` border, slate text (grayscale-safe: the heading reads `OMITTED` in text).
  - Category row: label cell **92 pt** wide, 3.2 pt left stripe in the category colour, 6 pt dot, label wraps (e.g. `EXTRAS / BACKGROUND` on two lines) with a hanging indent; padding 3.4/9/3.6; 0.5 pt `--of-line` bottom rule; last row has 0.5 pt `--of-line-strong` bottom rule and 6 pt bottom radius.
  - Element area: **2 columns** (14 pt gap) when notes are on; **3 columns** (12 pt gap) when `includeNotes=false` (names only). Elements flow top-to-bottom, balanced across the columns, never split across columns (`break-inside: avoid`).
  - Element: name (bold) left, state tag right-aligned in a fixed gutter at the column's right edge; note (if any) under the name. Confirmed = no tag. Manual = blue-info tag. **Suggested = tag + tinted row** (`--of-amber-fill`, 2 pt `--of-orange` left bar, row extends 6 pt into the cell padding so the bar sits in the gutter).
  - Empty name → `Unnamed element` in `--of-muted`, regular weight (source text).
  - No elements → one full-width cell `No breakdown elements yet.` (source text), 8.4 pt `--of-muted`.
- **Grayscale rule.** Category colour is *decoration only*. Identity is carried by (1) the category **name printed in every row / band** in bold uppercase and (2) position (fixed canonical order). The colour appears as a 3.2 pt left stripe plus a 6 pt dot next to the name. Element state is carried by **text** (`MANUAL`, `SUGGESTED`) and a **tinted row + 2 pt left bar** for suggested; catalog status by the **chip text**. Checked with `verify_pdf.py --gray` (see section I).

### F.4 Catalog section
- **Items-per-category strip** (design addition, derived from the same rows): 4-column grid, 6/9 pt padding, `--of-panel` fill, 0.5 pt `--of-line` border; each cell = 6 pt category dot, category name 8 pt, count right-aligned bold navy; heading `ITEMS PER CATEGORY` 6.8 pt bold caps. Purpose: orientation in a 10-23 page table. Listed in section K as a design choice.
- **One table per category** (as in the source: one `StyledTable` per category). `thead` has two rows that **both repeat on every page the table continues on**: (1) the **category band** — tint fill, 3.2 pt left stripe in the category colour, 6.5 pt dot, category name 9.6 pt bold caps +0.06em navy, `N items` / `1 item` right (7.8 pt slate); (2) the **column header** — `--of-panel` fill, 6.9 pt bold caps slate, 1.2 pt navy bottom rule.
- Columns (520 pt): **Item** flexible (≈198 pt; left; wraps), **Status** 68 pt (chip), **Used in scenes** 206 pt (left; wraps), **Uses** 48 pt (right).
- Row: padding 3.6/7/3.8; 0.5 pt `--of-line` bottom rule; item name 8.8 pt bold navy; description (if any) 7.8 pt slate under the name; scene list 8.4 pt **bold** `--of-ink` comma-separated numbers, full list (never abbreviated); `Uses` = 9.6 pt bold navy count preceded by a 30 x 3 pt usage bar (design addition: fill = uses / scenes in scope; track `--of-line`, fill navy). Zero uses: scene cell `Not used yet` 8.4 pt regular `--of-muted`, count `0` regular muted, no bar.
- Status chip (shared `.chip`, 6.6 pt): `Confirmed`, `Required` → green (`Tone::Ok`); `Searching`, `Shortlisted` → amber (`Tone::Warn`); anything else (`Not Required`) → neutral. Tone mapping is the source's; the chip **text** is the carrier.
- Table bottom rule + 6 pt bottom radius on the last row of each table; 10 pt gap between tables.

## G. Dynamic behaviour
- Everything in the scenes-only spec (G) for Part 1.
- **Catalog:** any number of categories/items; a category table splits between rows (never inside a row); band + column header repeat on every page the table spans. Long item names (even a 92-character token) and 300-character descriptions wrap; rows grow. Scene lists of any length wrap inside the 206 pt column (stress: 96 numbers = 6 lines); they are never abbreviated into ranges (the source prints every number; compressing would be a product decision).
- **Selection filter:** with `sceneIds`, only used items and only selected scene numbers are shown; "Not used yet" cannot occur then.
- **Empty catalog:** Part 2 heading stays, strip and tables are replaced by `The catalog is empty.` (source text).
- **`includeSuggested=false`:** no effect on the Catalog part (usage counts only production states in any case).
- **`includeNotes`:** affects Part 1 only.

## H. Pagination and print rules
- Part 1 rules as in the scenes-only spec. Part 2: section heading `break-after: avoid`; each category table keeps its band + column header with at least **3** rows at the start of a page fragment (`tbody tr:nth-child(-n+2) {break-after:avoid}`) and its **last two rows together**; rows never split; the tables may break at any other row.
- Footer/header as in F.1; running header text `LOW TIDE RADIO · BREAKDOWN REPORT` for the whole file.

## I. HTML/PDF correspondence
- `Breakdown_And_Catalog_Combined.html` → `Breakdown_And_Catalog_Combined.pdf` (Chrome 155, `render_pdf.py`), 10 pages.
- **State captured:** scope **9 selected scenes** (same as the scenes-only sample), notes on, suggested on; Part 2 is therefore the **filtered** catalog (104 real items, usage restricted to those 9 scenes) — exactly what `export()` would produce for that selection. Whole-source catalog (321 items) is shown in `Catalog_With_Scene_Usage.pdf`.
- Toggles: Include element notes, Include suggested elements. The empty-catalog state exists in the stress test HTML (`State: empty catalog`).
- Differences vs. Rust renderer: Chrome repeats the band as identical text (Rust should print "(continued)" on the repeated band, but never when the table begins a page); the usage bar is plain HTML.
- Stress test: `stress-test/Breakdown_And_Catalog_Combined_STRESS_TEST.html/.pdf` (12 pages).

## J. Acceptance criteria
- [ ] All criteria of the scenes-only spec (J) for Part 1.
- [ ] Four metrics in the stated order; Catalog items equals the number of catalog rows printed.
- [ ] `PART 1` / `PART 2` labels present only in this variant; Part 2 heading never last on a page.
- [ ] Catalog rows: Item, Status, Used in scenes, Uses in that order; band + column header repeat on every continued page; no "(continued)" bar directly above a freshly started table.
- [ ] Scene lists are complete and ascending; zero-use rows show `Not used yet` and `0`.
- [ ] With a scene selection no item that is unused in the selection appears.
- [ ] Grayscale: category and status identifiable without colour; footer/page counts correct on every page.

## K. Known issues and unresolved questions
### K.1 Verified defects in the current output
See B.1 (pages cited; the `(CONTINUED)` defects are combined pp55 and 58).
### K.2 Design choices
- Continuous flow with part labels (not a forced page break) — change by setting `break-before: page` on Part 2.
- Usage bar, items-per-category strip, column header — additions derived from existing data; the bar's reference (scenes in scope) is a design choice.
- **`content=summary`:** present in `breakdown.rs` (`export_summary`) but no UI sends it; **not designed**.
### K.3 Items marked unverified
- Whether the Rust `split_card()` can split a single `CardPart::Groups` unit taller than a page (read from source only: each part is one unit; no scene in the sample is that tall). The design splits long categories at element boundaries — **needs a renderer change**.
- Behaviour with a scene that is only used by omitted/removed scenes (UI helper `used_in_label` has a "Removed scenes only" case; the export prints "Not used yet" when no scene number resolves) — not exercised by the sample.
- Whether users can customise category colours in the Windows UI (`modules/production/categories.rs` defines per-category colour tokens and icons, but the export uses the fixed `pal::category` palette). If they can, the PDF will not match the on-screen colour; the design keeps the category *name* as the carrier so this is cosmetic.
- Non-Latin text (fallback fonts) — out of scope for the design; the Helvetica metrics used here are Chrome's, not `pdf-writer`'s.


