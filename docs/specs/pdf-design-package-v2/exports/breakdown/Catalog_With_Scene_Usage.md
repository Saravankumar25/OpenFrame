# Catalog — Items with scene usage

| | |
|---|---|
| **Export** | Catalog |
| **Variant** | Catalog with scene usage (`content="catalog"`; from the Catalog tab Export button and from Export Breakdown → Contents) |
| **Status** | Design reference — not integrated; not validated against OpenFrame's Rust renderer |
| **Repo commit** | `2f5a045416bd2125801329533fc47f986ccc8174` (main) |
| **Date** | 2026-10-23 |

## A. Purpose and users
The production's **item inventory with scene usage**: for every Catalog item (character, prop, wardrobe piece, vehicle, effect, sound, animal, location …) its category, status, and the scenes that use it. Users: line producer and production manager (what must be sourced / budgeted), art director, props master, wardrobe and costume supervisors, vehicle and animal coordinators, the 1st AD (item used in 36 scenes = continuity risk). Typical question: "Where is the Nagra, how many scenes need it, and is it confirmed?" A snapshot, "not a live connection to the Catalog" (dialog text).

## B. Verified functionality
| Entry point | File / component | Where it appears | What it sends |
|---|---|---|---|
| **Export Breakdown** dialog | `windows/apps/desktop/src/features/export/buttons.tsx` → `BreakdownExportButton({sceneId, scenes})`, shared `ExportDialog` (`title="Export Breakdown"`, `source="Source: the active Production Source"`, `documentName="Breakdown Report"`, `op="breakdown.export_report"`, `formats={TABLE_FORMATS}` = PDF / CSV / Excel (XLSX)) | Mounted in `workspaces/breakdown/index.tsx` (`<BreakdownExportButton size="xs" sceneId={sceneId} scenes={rows.map(...)}/>`) inside the Production Source info bar at the top of the Breakdown workspace, next to "Change source" | Scope radios: **All scenes of the Production Source** (no `sceneIds`), **Current scene** (`sceneIds:[sceneId]`, disabled when no scene is open), **Selected scenes** (picker → `sceneIds`; empty text "No scenes in the Production Source yet."). Options: choice **Contents** (`content`, PDF/XLSX only; default `both`; values *Breakdown by scene and Catalog* = `both`, *Breakdown by scene* = `scenes`, *Catalog with scene usage* = `catalog`); choice **Table** (CSV only, sends `content`; default `scenes`; *Breakdown by scene* / *Catalog with scene usage*); checkbox **Include suggested elements (marked as suggested)** (`includeSuggested`, default on); checkbox **Include element notes** (`includeNotes`, default off) |
| **Export Catalog** dialog | same file → `CatalogExportButton()` (`title="Export Catalog"`, `sub="The export is a snapshot, not a live connection to the Catalog."`, `source="Source: Catalog and the active Production Source"`, `documentName="Catalog"`, `baseArgs={{content:"catalog"}}`) | Mounted in `workspaces/production/tabs/catalog.tsx` as `<CatalogExportButton />` in the Catalog `PageHeader` actions, left of "Add Item" | Always `content:"catalog"`, whole Production Source (no scope selector, no options). The format list is re-ordered `[XLSX, CSV, PDF]`, so the dialog's initial format is **Excel (XLSX), not PDF** (`ExportFlow` uses `formats[0]`) |

**This variant = `content: "catalog"`**, reachable two ways with the same output family:
1. **Catalog tab → Export** (`CatalogExportButton`): always the whole Production Source; no scope, no options; formats listed Excel (initial), CSV, PDF.
2. **Breakdown workspace → Export Breakdown** with Contents = "Catalog with scene usage" (PDF/XLSX) or Table = "Catalog with scene usage" (CSV): additionally allows scope (all / current / selected scenes — which then filters the catalog to items used in those scenes) and the two checkboxes (which have **no effect** on a catalog-only report because no scene cards are produced).


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

**Differences of the catalog-only document** (`content == "catalog"` in `export()`): document name **`Catalog`** (footer text `LOW TIDE RADIO · CATALOG`), top-bar/eyebrow **`PRODUCTION / CATALOG`** and **`Production inventory`**, title **`Catalog`**, metrics only **Scenes** and **Catalog items** (no element metrics), the same "Catalog" section and tables. `data.scenes` is still loaded for the Scenes metric; the "no scenes" error is only raised when scene cards are requested.

**Data origin.** `export()` runs one `session.store.read`: `active_source(c)` (Production Source draft; error `validation.no_production_source` "Choose a screenplay draft as the Production Source first. Your screenplay is not changed."), the screenplay title/draft name (`screenplay_draft` ⨝ `screenplay`), `SourceScenes::load` (rows: `id`, `number`, `heading`, `omitted`), `breakdown_element` rows (`scene_id`, `category`, `COALESCE(catalog_item.name, display_name)`, `confirmation_state`, `notes`; filter `deleted_at IS NULL AND archived = 0 AND confirmation_state <> 'Rejected'`, ordered by `created_at, id`, then stable-sorted by category order `BreakdownCategory::ALL`), `catalog_usage(c, &src_scenes)` (distinct `catalog_item_id, scene_id` pairs from **production states only**: `PRODUCTION_STATES = ('Confirmed','Manual')`, so suggested elements never count as catalog usage), and `catalog_item` (`id, category, name, status, description`; `deleted_at IS NULL AND archived = 0`). `project_title(c)` supplies the project name. Catalog rows are sorted by `(cat_order(category), lower(name))`; usage scene numbers come from `sort_refs` (numeric order, then heading).

### B.1 Current-output assessment (`Catalog — Items with scene usage.pdf`, 23 pages; whole source: 45 scenes, 321 items in 11 categories)
**Retain** — one table per category in canonical order with a band `● Category … N items`; per row item name (bold), optional description under it, status chip, "Used in scenes" list, count of uses ("USE"/"USES" caption); "Not used yet"; sentence "Usage is derived from breakdown assignments"; the empty state "The catalog is empty."
**Improve visually** — compact rows (one line for a plain item) instead of ~58 pt rows (est. 13 pages instead of 23), real column header that repeats, the status chip next to the name, scene lists in a wider column, usage bar, category strip.
**Verified functional/layout problems**
- **Catalog density:** each row is ~58 pt high for a single-line item (the "USED IN SCENES" caption is repeated in every row, the status chip floats in a wide empty gutter, the count has its own caption). 321 items need 23 pages (≈14 rows per page).
- **"(CONTINUED)" bar printed above a fresh heading:** combined PDF p55 (`VFX (CONTINUED)` directly above the `VFX · 11 items` band) and p58 (`ANIMALS (CONTINUED)` above the `Animals · 5 items` band): the table moved whole to a new page, but the renderer (`StyledTable.continued`) still prints the continuation bar. (Catalog PDF p2-p23 show the intended use: a continuation bar above rows that continue.)
- **No column headers** (`HeaderStyle::None`): the meaning of each column is only given by per-row captions; a reader scanning the Used-in-scenes lists has no header to anchor on.
- **Long scene lists** (ANJALI: 36 scenes) wrap in a narrow column and push the row to 4 lines; there is no visual cue of how heavily an item is used other than the 15 pt count.
- **Status tone:** `Required` and `Confirmed` both use the green `Ok` tone, so 312 of 321 rows carry a green chip (visual noise; no information in the sample).
- **Masthead mismatch:** the orange eyebrow says `PRODUCTION INVENTORY` while the top bar says `PRODUCTION / CATALOG` (p1) — two labels for one identity; merged here.
- **Dialog default:** the Catalog tab's dialog starts on **Excel (XLSX)** (`[TABLE_FORMATS[2], [1], [0]]`), so a user who wants this PDF must change the format; not a PDF defect but a discoverability one.
**Cannot be determined from the evidence** — how very long item names and 300+ character descriptions render today (none in the sample, see K.3).

## C. Source mapping
- UI: `features/export/buttons.tsx::CatalogExportButton` (mounted at `workspaces/production/tabs/catalog.tsx` `PageHeader actions`), and `BreakdownExportButton` (Contents = `catalog`).
- Op/builder: `breakdown.export_report` → `breakdown::export`, `content == "catalog"` (`with_scenes = false`, `with_catalog = true`).
- Data: `catalog_item` (+ `catalog_usage`, `SourceScenes`); `CatalogStatus` ∈ Required / Searching / Shortlisted / Confirmed / Not Required (`modules/production/catalog.rs`).
- Renderer blocks: `Masthead`, `Metrics(2)`, `Section("Catalog", "Usage is derived from breakdown assignments")`, per category `Block::Styled(StyledTable)` (`HeaderStyle::None`, `boxed`, `continued`), band row = `Row::spanning(CellItem::Heading{dot, text, right})` with `keep_next`; cells `Txt` (name 9.4 bold navy, description 8.2 slate, scene list 9.0 bold, count 15 bold) and `CellItem::Chips`; `Block::Text` for the empty state.
- **Renderer changes this design needs:** column header row repeated with the band; no continuation bar when the table starts a page; usage bar; status chip beside the name (column re-order widths); part of the footer/header changes listed for the other variants.

## D. Field inventory
| Visible element | Source | Kind | Empty / edge treatment |
|---|---|---|---|
| Eyebrow `PRODUCTION / CATALOG` | literal (top-bar text; the orange `Production inventory` eyebrow is merged into it) | fixed | — |
| Title `Catalog` | `document` | fixed | — |
| Project/source line, date, scope | as in the scenes spec (scope is `Entire production source` from the Catalog tab) | derived | — |
| Metric Scenes | `data.scenes.len()` (scenes in scope) | derived | — |
| Metric Catalog items | `data.catalog.len()` | derived | `0` → empty state |
| Section `Catalog`, hint `Usage is derived from breakdown assignments` | literals | fixed | — |
| Items per category strip (design addition) | rows per category | derived | omitted when empty |
| Category band: dot, name, `N item(s)` | `catalog_item.category`, `pal::category` | repeatable | empty categories not printed |
| Item name | `catalog_item.name` | required | — |
| Description | `catalog_item.description` | optional | omitted |
| Status chip | `catalog_item.status` | required | neutral tone if unknown |
| Used in scenes | scene numbers (usage ∩ scope), ascending | derived | `Not used yet` |
| Uses + bar | `nums.len()`; bar = uses / scenes in scope (design addition) | derived | `0`, no bar |
| `The catalog is empty.` | literal | conditional | — |

## E. Content architecture
Order (unchanged): **Masthead → Metrics (Scenes, Catalog items) → Section "Catalog" → category tables (canonical order, items A-Z case-insensitive)**.
**Changes vs. current and why:** (1) columns re-ordered/sized to **Item | Status | Used in scenes | Uses** with a real column header — the current table has no header and repeats a caption in every row; (2) status chip moves next to the item (68 pt column) instead of the middle of the row; (3) the count loses its own caption and gains a usage bar; (4) added "Items per category" strip for navigation; (5) running header `LOW TIDE RADIO · CATALOG`; (6) one eyebrow instead of two labels. Grouping by category, sort order and content of each cell are unchanged.

## F. Visual specification
Page, furniture and running header as in the scenes spec F.1 (header text `LOW TIDE RADIO · CATALOG`). Typography/metrics/section heading/panel as F.2-F.3 (the metrics strip has two tiles).
**Grayscale rule.** Category colour is *decoration only*. Identity is carried by (1) the category **name printed in every row / band** in bold uppercase and (2) position (fixed canonical order). The colour appears as a 3.2 pt left stripe plus a 6 pt dot next to the name. catalog status by the **chip text**. Checked with `verify_pdf.py --gray` (see section I).

| Category (`BreakdownCategory`, canonical order) | `pal::category` colour | Row label in the design |
|---|---|---|
| Cast | `#7a4cc2` | CAST |
| Extras / Background | `#4f9a3d` | EXTRAS / BACKGROUND |
| Location / Set | `#1b84be` | LOCATION / SET |
| Props | `#c0623a` | PROPS |
| Wardrobe | `#1f9a8a` | WARDROBE |
| Vehicles | `#6b7a8f` | VEHICLES |
| Hair / Makeup | `#d6527a` | HAIR / MAKEUP |
| Special Effects | `#d99a1a` | SPECIAL EFFECTS |
| VFX | `#4b63d1` | VFX |
| Sound | `#c23a96` | SOUND |
| Animals | `#8a6a3b` | ANIMALS |

(unknown category → `#8792a5`.) The order is verified from the supplied PDFs (Cast, Extras, Location, Props, Wardrobe, Vehicles, Hair/Makeup, Special Effects, VFX, Sound, Animals) and matches `BreakdownCategory::ALL` usage in `cat_order`.

### F.4 Catalog section
- **Items-per-category strip** (design addition, derived from the same rows): 4-column grid, 6/9 pt padding, `--of-panel` fill, 0.5 pt `--of-line` border; each cell = 6 pt category dot, category name 8 pt, count right-aligned bold navy; heading `ITEMS PER CATEGORY` 6.8 pt bold caps. Purpose: orientation in a 10-23 page table. Listed in section K as a design choice.
- **One table per category** (as in the source: one `StyledTable` per category). `thead` has two rows that **both repeat on every page the table continues on**: (1) the **category band** — tint fill, 3.2 pt left stripe in the category colour, 6.5 pt dot, category name 9.6 pt bold caps +0.06em navy, `N items` / `1 item` right (7.8 pt slate); (2) the **column header** — `--of-panel` fill, 6.9 pt bold caps slate, 1.2 pt navy bottom rule.
- Columns (520 pt): **Item** flexible (≈198 pt; left; wraps), **Status** 68 pt (chip), **Used in scenes** 206 pt (left; wraps), **Uses** 48 pt (right).
- Row: padding 3.6/7/3.8; 0.5 pt `--of-line` bottom rule; item name 8.8 pt bold navy; description (if any) 7.8 pt slate under the name; scene list 8.4 pt **bold** `--of-ink` comma-separated numbers, full list (never abbreviated); `Uses` = 9.6 pt bold navy count preceded by a 30 x 3 pt usage bar (design addition: fill = uses / scenes in scope; track `--of-line`, fill navy). Zero uses: scene cell `Not used yet` 8.4 pt regular `--of-muted`, count `0` regular muted, no bar.
- Status chip (shared `.chip`, 6.6 pt): `Confirmed`, `Required` → green (`Tone::Ok`); `Searching`, `Shortlisted` → amber (`Tone::Warn`); anything else (`Not Required`) → neutral. Tone mapping is the source's; the chip **text** is the carrier.
- Table bottom rule + 6 pt bottom radius on the last row of each table; 10 pt gap between tables.

## G. Dynamic behaviour
- Any number of categories/items; tables split between rows; band + column header repeat; no row splits.
- Long names (92-character unbroken token), long descriptions (300+ characters), 96-number scene lists wrap and the row grows (stress test).
- **Zero uses:** `Not used yet` + `0`, no bar. **All five statuses** supported (neutral for `Not Required`).
- **Empty catalog:** `The catalog is empty.` replaces the strip and tables; the section heading stays.
- **From the Breakdown dialog with a scene selection:** only used items, only those scene numbers; masthead scope reads `n selected scenes`; Scenes metric = selected scene count.
- No notes/suggested handling in this variant.

## H. Pagination and print rules
Section heading keeps with the strip and the first table. Each table: band + column header repeat on every page; at least 3 rows after the header at a page start; last 2 rows together; rows never split; 10 pt between tables; ~38 single-line rows per full page. Footer/header per F.1.

## I. HTML/PDF correspondence
- `Catalog_With_Scene_Usage.html` → `.pdf` (Chrome 155; `render_pdf.py`), 13 pages.
- **State captured:** the **whole Production Source** as produced by the Catalog tab's Export (45 scenes, **all 321 real catalog items**, extracted from the supplied PDF by `extract_catalog.py`). It is not truncated: the full table is the honest "multi-page table with header repetition" (the current PDF needs 23 pages).
- Real data contain only two statuses (Required 312, Confirmed 9); other statuses are exercised in the stress test (fictional rows, labelled).
- Differences vs. Rust: repeated band shows identical text (Rust: add `(continued)` only when rows continue); Chrome Helvetica substitute metrics; usage bar is HTML.
- Stress test: `stress-test/Catalog_With_Scene_Usage_STRESS_TEST.html/.pdf` (7 pages): item used in 96 scenes, a ~150-character name, a 92-character unbroken token as a name, a ~300-character description, four statuses besides Required, a zero-use row, a one-item category, the real 152-row Props table across 6 pages, hidden `State: empty catalog` toggle.

## J. Acceptance criteria
- [ ] Eyebrow `PRODUCTION / CATALOG`, title `Catalog`, two metric tiles, footer/header text per F.1.
- [ ] Tables per category in canonical order; items sorted case-insensitively; empty categories absent.
- [ ] Column order Item | Status | Used in scenes | Uses; header row present and repeated on every continued page together with the category band.
- [ ] A table never starts a page with a "(continued)" marker above a fresh band; no row splits; ≥ 3 rows after a header, ≥ 2 rows at the end.
- [ ] Scene numbers complete, ascending, comma separated; `Not used yet` + `0` for unused items; uses = number of listed scenes.
- [ ] Status chips carry their text; tones: Required/Confirmed green, Searching/Shortlisted amber, others neutral.
- [ ] Description appears only when present; very long names/descriptions wrap without clipping.
- [ ] `The catalog is empty.` when no rows.
- [ ] Grayscale: categories identified by band name; statuses by chip text.
- [ ] Footer logo/brand/page numbers on every page; nothing in the footer band; deterministic output.

## K. Known issues and unresolved questions
### K.1 Verified defects in the current output
See B.1 (catalog density, "(CONTINUED)" misuse in the combined PDF pp55/58, no column header, status tone noise, eyebrow mismatch, Excel default in the Catalog dialog).
### K.2 Design choices
- Column header + usage bar + category strip are additions from existing data; the usage bar's reference is "scenes in scope".
- Full scene lists are kept (no range compression).
- Item status tone mapping kept from source although `Required` as green is questionable: a product decision.
- **`content=summary`:** exists in code (`export_summary`) but is not exposed anywhere in the UI; **not designed**.
### K.3 Items marked unverified
- Whether the Rust `split_card()` can split a single `CardPart::Groups` unit taller than a page (read from source only: each part is one unit; no scene in the sample is that tall). The design splits long categories at element boundaries — **needs a renderer change**.
- Behaviour with a scene that is only used by omitted/removed scenes (UI helper `used_in_label` has a "Removed scenes only" case; the export prints "Not used yet" when no scene number resolves) — not exercised by the sample.
- Whether users can customise category colours in the Windows UI (`modules/production/categories.rs` defines per-category colour tokens and icons, but the export uses the fixed `pal::category` palette). If they can, the PDF will not match the on-screen colour; the design keeps the category *name* as the carrier so this is cosmetic.
- Non-Latin text (fallback fonts) — out of scope for the design; the Helvetica metrics used here are Chrome's, not `pdf-writer`'s.


