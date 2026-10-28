# Breakdown Report — Breakdown by scene

| | |
|---|---|
| **Export** | Breakdown Report |
| **Variant** | Breakdown by scene (`content="scenes"`, UI label “Breakdown by scene”) |
| **Status** | Design reference — not integrated; not validated against OpenFrame's Rust renderer |
| **Repo commit** | `2f5a045416bd2125801329533fc47f986ccc8174` (main) |
| **Date** | 2026-10-23 |

## A. Purpose and users
A scene-by-scene breakdown sheet for the people who have to **prepare** what a scene needs: the 1st AD and production manager (is the scene fully broken down? how many suggestions are still unconfirmed?), art department (props, set dressing, vehicles, animals), wardrobe and hair/makeup, SFX/VFX supervisors, sound, casting/extras coordinators. Each wants to open the page for scene N, find **their** department's rows immediately, and see what is confirmed, what was added by hand and what is only suggested. It is printed and annotated on paper (hence legible notes and stable category order) and also sent as PDF.

## B. Verified functionality
| Entry point | File / component | Where it appears | What it sends |
|---|---|---|---|
| **Export Breakdown** dialog | `windows/apps/desktop/src/features/export/buttons.tsx` → `BreakdownExportButton({sceneId, scenes})`, shared `ExportDialog` (`title="Export Breakdown"`, `source="Source: the active Production Source"`, `documentName="Breakdown Report"`, `op="breakdown.export_report"`, `formats={TABLE_FORMATS}` = PDF / CSV / Excel (XLSX)) | Mounted in `workspaces/breakdown/index.tsx` (`<BreakdownExportButton size="xs" sceneId={sceneId} scenes={rows.map(...)}/>`) inside the Production Source info bar at the top of the Breakdown workspace, next to "Change source" | Scope radios: **All scenes of the Production Source** (no `sceneIds`), **Current scene** (`sceneIds:[sceneId]`, disabled when no scene is open), **Selected scenes** (picker → `sceneIds`; empty text "No scenes in the Production Source yet."). Options: choice **Contents** (`content`, PDF/XLSX only; default `both`; values *Breakdown by scene and Catalog* = `both`, *Breakdown by scene* = `scenes`, *Catalog with scene usage* = `catalog`); choice **Table** (CSV only, sends `content`; default `scenes`; *Breakdown by scene* / *Catalog with scene usage*); checkbox **Include suggested elements (marked as suggested)** (`includeSuggested`, default on); checkbox **Include element notes** (`includeNotes`, default off) |
| **Export Catalog** dialog | same file → `CatalogExportButton()` (`title="Export Catalog"`, `sub="The export is a snapshot, not a live connection to the Catalog."`, `source="Source: Catalog and the active Production Source"`, `documentName="Catalog"`, `baseArgs={{content:"catalog"}}`) | Mounted in `workspaces/production/tabs/catalog.tsx` as `<CatalogExportButton />` in the Catalog `PageHeader` actions, left of "Add Item" | Always `content:"catalog"`, whole Production Source (no scope selector, no options). The format list is re-ordered `[XLSX, CSV, PDF]`, so the dialog's initial format is **Excel (XLSX), not PDF** (`ExportFlow` uses `formats[0]`) |

**This variant = `content: "scenes"`** (Contents choice "Breakdown by scene"; for CSV the "Table" choice with the same label).


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

**Scope.** All scenes / current scene / selected scenes of the *active Production Source* (a screenplay draft). Scenes keep Production-Source order (`SourceScenes::rows`). `sceneIds` not in the source → `not_found("scene")`.

**Contents in this variant.** Masthead, 2-3 metrics, section "Breakdown by scene", optional state sentence, one card per scene. Elements: every non-archived, non-deleted, non-`Rejected` `breakdown_element` of the scene; `Suggested` only if `includeSuggested`; notes only if `includeNotes`. Categories with no elements are **not printed** (no empty category headers). A scene with no elements prints "No breakdown elements yet." (an omitted scene prints the heading `OMITTED`).

**Formats / other outputs.** PDF (this design); CSV (`Breakdown by Scene` table: Scene, Heading, Category, Element, State, Catalog item, [Notes]); XLSX (same table; with `both` also the Catalog table). State labels in CSV/XLSX: `Suggested`, `Added manually`, `Confirmed` (`state_label`). Unsupported characters in the PDF → error with `HINT_TABLES` ("Export it as CSV or Excel (XLSX) ...").

**Errors.** No Production Source (`validation.no_production_source`); `sceneIds: []` ("Select at least one scene to export."); no scenes (`nothing_to_export`: "The Production Source has no scenes to export.").

**Data origin.** `export()` runs one `session.store.read`: `active_source(c)` (Production Source draft; error `validation.no_production_source` "Choose a screenplay draft as the Production Source first. Your screenplay is not changed."), the screenplay title/draft name (`screenplay_draft` ⨝ `screenplay`), `SourceScenes::load` (rows: `id`, `number`, `heading`, `omitted`), `breakdown_element` rows (`scene_id`, `category`, `COALESCE(catalog_item.name, display_name)`, `confirmation_state`, `notes`; filter `deleted_at IS NULL AND archived = 0 AND confirmation_state <> 'Rejected'`, ordered by `created_at, id`, then stable-sorted by category order `BreakdownCategory::ALL`), `catalog_usage(c, &src_scenes)` (distinct `catalog_item_id, scene_id` pairs from **production states only**: `PRODUCTION_STATES = ('Confirmed','Manual')`, so suggested elements never count as catalog usage), and `catalog_item` (`id, category, name, status, description`; `deleted_at IS NULL AND archived = 0`). `project_title(c)` supplies the project name. Catalog rows are sorted by `(cat_order(category), lower(name))`; usage scene numbers come from `sort_refs` (numeric order, then heading).

### B.1 Current-output assessment (`Breakdown — By Scene only.pdf`, 35 pages, portrait; sample = Low Tide Radio, 45 scenes, 699 confirmed + 7 suggested elements, notes included)
**Retain**
- Content order: masthead -> metrics (Scenes / Confirmed elements / Suggested) -> "Breakdown by scene" + hint "Ordered by the Production Source" -> one card per scene, elements grouped by category in canonical category order.
- Scene identity: orange-family tile with `SCENE` + number, `BREAKDOWN SCENE` eyebrow, uppercased heading.
- Suggested/manual labelling and the exact state sentence; category colour accents; notes under elements; the exact empty-state text; "OMITTED" treatment.
**Improve visually**
- Replace the 3-column equal-height grid by labelled category rows with a 2-column element flow (measured on the same nine scenes: they occupy 7 pages in the current PDF, pp. 2-5 and 18-20, and 5 pages here including the masthead; scene 1 is ~518 pt high in the current PDF and ~462 pt here; extrapolation to all 45 scenes, unverified: ~25 pages instead of 35).
- Tag in a fixed right gutter (no wrapping away from its element); suggested elements tinted; manual tags quieter.
- Repeat the scene head when a card continues; never leave a head alone.
- Light key explaining the three element states.
**Verified functional/layout problems (supplied PDF)**
- **By Scene p1:** the intro page is ~two thirds empty (masthead, three metrics, section heading, one sentence, then nothing): the first scene card did not fit under the intro and, because cards move whole, jumped to p2.
- **By Scene p2→p3:** scene 2's card *head* (tile + heading) is stranded alone at the bottom of p2 and its body starts on p3 with **no head repeated**, so p3 shows Cast/Props/... with no scene number (also the same pattern in the combined PDF, pp. 2-3). Cause in source: `split_card()` treats the head as one unit and each `CardPart` as one unit and does not repeat the head.
- **Tag/pill attribution:** the `MANUAL` chip is a separate flowed chip and wraps independently of its pill: p2 "Ship-to-shore radio handset" / `MANUAL` on its own line; p3 Props "Wall chart of the coast with pins" has its `MANUAL` at the start of the *next* line, left of "Grease pencil" `MANUAL`, so it is unclear which pill owns which tag (also p18, p19).
- **Wasted area:** 3-column groups are laid out in rows of equal height with the accent bar stretched to the tallest group (`groups_frag`): p3 has a one-line Cast group beside a Props group about eleven lines tall; p19 leaves ~30 % of the page blank (whole-card moves). 45 scenes take 34 pages.
- **Omitted scene** (scene 22, p18): the heading is replaced by the word `OMITTED`, so the scene's original heading is not printed (source: `if *omitted { "OMITTED" } else { heading.to_uppercase() }`).
- **State legend** exists only as one sentence on p1 and only when suggestions are present; 588 of the 706 elements (83 %) carry `MANUAL` and only 7 carry `SUGGESTED`, so the common label dominates the page visually while `SUGGESTED` (7) is easy to miss.
**Cannot be determined from the evidence**
- How the renderer behaves for a single category with more elements than fit on a page (see K.3).
- Whether category colours are user-customisable in the app (see K.3).

## C. Source mapping
- Commit `2f5a045416bd2125801329533fc47f986ccc8174`.
- UI: `features/export/buttons.tsx::BreakdownExportButton` (mounted in `workspaces/breakdown/index.tsx`); op `breakdown.export_report`; builder `breakdown::export` (`content = "scenes"`).
- Domain: `SourceScenes`, `breakdown_element`, `catalog_item` (name resolution `COALESCE(ci.name, e.display_name)`), `BreakdownCategory::ALL`, `confirmation_state` ∈ {Confirmed, Manual, Suggested} (`Rejected` excluded), `modules::schedule::board::project_title`.
- Renderer blocks used today: `Block::Masthead(production_masthead(...))` (`style.rs`), `Block::Metrics`, `Block::Section`, `Block::Text` (state sentence), `Block::Cards(Cards::new(1, cards))` with `Card{head: CardHead{eyebrow, tile: Tile}, parts: [CardPart::Groups{groups, cols: 3} | CardPart::Empty]}`; `Group{label, accent: pal::category(..), items: GroupItem{text, tag: Chip, note, tone}}`; footer `footer(project, "Breakdown Report")`.
- **Renderer changes this design needs** (not designed in code here): a two-column "category row" card part replacing `CardPart::Groups` 3-column grid; head repeat on split + split at category/element boundaries (`split_card`); tag gutter; 6.6 pt tag size; footer logo image; running header.

## D. Field inventory
| Visible element | Source | Kind | Empty / edge treatment |
|---|---|---|---|
| Eyebrow `PRODUCTION / BREAKDOWN REPORT` | literal in `export()` (top bar `"PRODUCTION / BREAKDOWN REPORT"`; eyebrow text `"Production breakdown"` is dropped, see E) | fixed | — |
| Title `Breakdown Report` | `document` | fixed | — |
| `Project: … · Source: …` | `project_title(c)`; `source_label` = `"{screenplay title}: {draft name}"` (draft name only if title empty) | derived; each part omitted when blank (`nb`) | whole line omitted if both blank |
| Date | `today_label()` (e.g. `22 OCT 2026`), uppercased | derived | — |
| Scope | `"Entire production source"` or `plural(n, "selected scene", "selected scenes")` | derived | — |
| Metric Scenes | `data.scenes.len()` | derived | `0` impossible (error first) |
| Metric Confirmed elements | elements with state ≠ Suggested (Confirmed + Manual) | derived | `0` |
| Metric Suggested | elements with state Suggested | conditional (`includeSuggested`) | tile omitted when off |
| Section `Breakdown by scene`, hint `Ordered by the Production Source` | literals | fixed | — |
| State sentence | literal | conditional (`includeSuggested && sugg > 0`) | omitted |
| Scene tile number | `SourceScenes` row `number` | required | — |
| Scene heading | row `heading` uppercased; `OMITTED` if `omitted` | required | — |
| Category label / accent | `breakdown_element.category` / `pal::category` | repeatable | categories with no element not printed |
| Element name | `COALESCE(catalog_item.name, display_name)` | repeatable | blank → `Unnamed element` |
| Tag `SUGGESTED` / `MANUAL` | `confirmation_state` (`Suggested` / `Manual`; `Confirmed` has none) | conditional | none |
| Element note | `breakdown_element.notes` trimmed (`nonblank`) | conditional (`includeNotes`) | omitted |
| Empty scene text | `No breakdown elements yet.` | conditional | — |
| Scene element count / suggested count (design addition) | derived from the scene's elements | derived | omitted when 0 elements |

## E. Content architecture
Order (unchanged from the current PDF): **Masthead → Metrics → Section "Breakdown by scene" → [state key] → Scene 1 … Scene n**. Within a scene: head, then categories in canonical order, then elements in creation order (`created_at, id`).
**Changes versus the current PDF, and why**
1. Eyebrow merges the two current labels ("PRODUCTION / BREAKDOWN REPORT" top-bar text and "PRODUCTION BREAKDOWN" orange eyebrow) into one eyebrow — the top bar belonged to the logo header, which the package replaces with the footer lockup.
2. Category grid (3 columns, equal-height rows) → **category rows** (label column + 2-column element list). Reason: scanning for one department, density, no stretched empty bars.
3. State tag moves from a flowing chip to a **right gutter**; suggested rows are tinted. Reason: attribution and findability of the 7 suggestions among 588 manual tags.
4. Added: scene element/suggested counts in the head; a visual state key (the verbatim sentence is kept). Both derived from existing data.
5. Added: running header (top-right) and head repeat on continuation.
6. Elements are **not** re-sorted within a category (creation order, as source).

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


## G. Dynamic behaviour
- **Scenes:** any number; each scene is a table that may split between category rows (see H). A scene with 100+ elements spans pages with the head repeated (stress test p2-p5).
- **Long categories:** more than 12 elements → emitted as 12-element chunks, each a row with the label (`PROPS`, then `PROPS cont.`) so a page break can fall between chunks and every page shows which category it is. (Rust: split at element boundaries and re-print the label with `cont.`.)
- **Long names/notes:** wrap inside their column; nothing is ellipsised; an unbroken token (e.g. a 92-character identifier) wraps at any character (`overflow-wrap:anywhere`).
- **Empty categories:** not printed. **No elements:** `No breakdown elements yet.` **Omitted scene:** `OMITTED` head, neutral tile.
- **Options:** `includeNotes=false` → notes disappear and the element area becomes 3 columns; `includeSuggested=false` → suggested rows, the Suggested metric tile, the state key and the head's suggested chip disappear and counts shrink. No placeholder remains for any removed part.
- **Scope text** changes with `sceneIds` (`n selected scenes`; a single scene = `1 selected scene`).
- Metric numbers are plain integers (no thousands separator, as source).

## H. Pagination and print rules
- Letter portrait, margins 46/46/46/66 pt. Scene head + at least **one** category row stay together (the head is never stranded); the **last two rows** of a scene stay together (no single trailing row on a new page); a category row/chunk and every element are never split. A scene may break between rows; the **head repeats** on each continuation page.
- Section heading keeps with the state key and the first scene (`break-after: avoid`). The masthead + metrics are never split.
- Footer and running header are page furniture (shared CSS `@page`); content never enters the 66 pt footer band; verified by `verify_pdf.py`.
- Deterministic: no clock in layout (the date is data).

## I. HTML/PDF correspondence
- `Breakdown_By_Scene.html` is the source of `Breakdown_By_Scene.pdf` (Chrome 155 headless, `tools/render_pdf.py`; `@page size 8.5in 11in` + running header).
- **State captured in the PDF:** scope **9 selected scenes** (1, 2, 3, 4, 21, 22 [omitted], 23, 24, 25 of the real 45-scene Low Tide Radio source), `includeNotes` **on**, `includeSuggested` **on** (137 confirmed + 5 suggested elements). Data is extracted from the supplied PDF (`tools/build/breakdown/extract_data.py`), not invented. The extract shows: an omitted scene, suggested elements (scenes 23, 24), a card that splits (scenes 3, 4, 21, 23) with a repeated head, notes of 1-3 lines.
- Preview toolbar toggles: **Include element notes**, **Include suggested elements** (recounts the figures). Scope variants (all 45 scenes) are not toggled in HTML (35+ pages); the scope text and the metrics change as described in G.
- Differences vs. the Rust renderer: Chrome balances the two element columns and chunks long categories statically (12 per row); the Rust version measures and splits exactly. Head repeat shows the same head (the Rust version should print the heading with a `(continued)` suffix — not expressible in the HTML). Chrome's Helvetica substitute (Arial/Liberation) differs slightly from the PDF base-14 metrics.
- Stress test: `stress-test/Breakdown_By_Scene_STRESS_TEST.html/.pdf` (5 pages, fictional data).

## J. Acceptance criteria
- [ ] Section order and labels as in E; eyebrow/title/meta/panel strings exactly as D.
- [ ] Metrics: Scenes always; Confirmed = Confirmed+Manual; Suggested tile only when `includeSuggested`.
- [ ] State sentence is verbatim and appears only when suggested elements are included and exist.
- [ ] Categories appear in canonical order; empty categories absent; the category name is printed in every row (grayscale test: remove colour — every row still identified).
- [ ] Suggested element = tint + `SUGGESTED` tag; Manual = `MANUAL` tag; Confirmed = none; the tag never wraps to a different line than its element's name row's top edge.
- [ ] Notes printed only with `includeNotes`; element area is 3 columns without notes, 2 with.
- [ ] A scene head is never the last thing on a page; a continued scene repeats its head; a continued category repeats its label with `cont.`.
- [ ] No text clipped or ellipsised; unbroken 92-char tokens wrap; very long scene headings wrap to several lines.
- [ ] Omitted scene prints `OMITTED`; scene with no elements prints `No breakdown elements yet.`; blank element name prints `Unnamed element`.
- [ ] Footer logo (PNG), `Created using OpenFrame`, `Page X of Y` correct on every page; running header on every page; nothing in the footer band.
- [ ] Output identical on two runs.

## K. Known issues and unresolved questions
### K.1 Verified defects in the current output
See B.1 (all cited to pages of `Breakdown — By Scene only.pdf`). Also: the footer redraws a vector logo + text on every page (the package replaces it with the official PNG lockup).
### K.2 Design choices (changed or added; reversible)
- Category rows instead of the 3-column grid; right tag gutter; tinted suggested rows; head repeat; counts in the head; visual key; running header. Splitting scenes across pages is **allowed** (the current renderer moves whole cards); rationale: with whole-scene moves the sample pages were only 47-63 % full in a first trial (pages 3, 4 and 7 of an earlier 7-page render). If crews prefer one scene per page, set scenes to `break-inside: avoid` (tall scenes still split).
- **`content=summary`:** present in `breakdown.rs` (`export_summary`) but no UI option sends it — **not designed**; no user-visible entry point.
### K.3 Items marked unverified
- Whether the Rust `split_card()` can split a single `CardPart::Groups` unit taller than a page (read from source only: each part is one unit; no scene in the sample is that tall). The design splits long categories at element boundaries — **needs a renderer change**.
- Behaviour with a scene that is only used by omitted/removed scenes (UI helper `used_in_label` has a "Removed scenes only" case; the export prints "Not used yet" when no scene number resolves) — not exercised by the sample.
- Whether users can customise category colours in the Windows UI (`modules/production/categories.rs` defines per-category colour tokens and icons, but the export uses the fixed `pal::category` palette). If they can, the PDF will not match the on-screen colour; the design keeps the category *name* as the carrier so this is cosmetic.
- Non-Latin text (fallback fonts) — out of scope for the design; the Helvetica metrics used here are Chrome's, not `pdf-writer`'s.


