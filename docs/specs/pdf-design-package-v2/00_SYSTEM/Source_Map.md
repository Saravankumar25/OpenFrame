# Source Map — what was inspected and what it says

All identifiers below were read from the repository at the commit named here. Anything not listed was not verified.

## 1. Repository state and reconciliation

| Item | Value |
|---|---|
| Repository | `https://github.com/Saravankumar25/OpenFrame` |
| Branch inspected | `main` |
| **Commit used as source of truth** | `2f5a045416bd2125801329533fc47f986ccc8174` — "Add website/: OpenFrame Studio public website (live at openframe-studio.pages.dev)" (committed 2026-10-21; still the tip of `main` when checked with `git ls-remote` on 2026-10-23) |
| Working checkout | **None supplied**: the working directory (`export changes/`) is not a git repository and contains no application source. The repository was therefore cloned read-only (blobless, sparse: `windows/crates/**`, `windows/apps/desktop/src/**`, `mac/crates/…/exports`, selected docs) into a scratch folder outside the package. No application file was modified, committed or pushed. |
| Earlier audit pack (supplied) | pinned to `324cd963240b6c563c93ad065b239b1e6d5e31c2` (2026-10-20) |
| Commits between audit and `main` | 43 (`git rev-list --count 324cd96..2f5a045`), mostly WindowsHTML/MacHTML captures, Mac work and the website. |

### Meaningful differences between the audit pack and current `main` (export-relevant, verified by `git diff --stat 324cd96 2f5a045`)

| Area | Change | Effect on this package |
|---|---|---|
| `windows/crates/openframe-application/src/modules/exports/daily_view.rs` | **New file, 333 lines**; op `schedule.export_daily_view` registered in `exports/mod.rs` ("Export one shooting day's Daily View"), formats PDF · TXT | The audit pack (and its CSV) describes the Daily View as `schedule.export_schedule` opened with `dayFirst`; that is **outdated**. Daily View now has its own builder → its own design (no baseline PDF was supplied). |
| `exports/story_page.rs` + op `story.page.export` | **New**: Story page (free writing) export TXT/PDF/DOCX | Excluded from scope by the brief. |
| `exports/story.rs`, `buttons.tsx`, `ExportDialog.tsx`, `model.ts`, `export.test.tsx` | "Story Board" → **"Beat Board"** rename in user-visible strings; `StoryExportButton` title "Export Beat Board"; layout choice labels "Outline PDF" / "Board PDF" | The supplied PDFs' metadata titles still read "Story Board" / "Story Board Outline" (verified with PyMuPDF) — those PDFs were produced before the rename or the PDF title string was not renamed (see Beat Board spec). |
| `modules/exports/mod.rs` (+60) | module list/doc table updated for daily view + story page | Op table in the header comment is the authoritative export list (§2 below). |
| `windows/crates/openframe-import-export/src/report.rs`, `report/design*.rs` | **No change** since the audit (not in the diff stat) | The renderer description in the audit pack remains valid. |
| `breakdown.rs`, `budget.rs`, `callsheet.rs`, `dood.rs`, `reports.rs`, `schedule.rs`, `sides.rs`, `style.rs`, `visual.rs` | unchanged | — |

Not compared: the `mac/` tree (separate Rust crates exist: `mac/crates/openframe-application/src/modules/exports/{daily.rs,…}` and `mac/crates/openframe-import-export/src/report*`). The supplied PDFs match the **Windows** renderer (Helvetica base-14, navy/orange design blocks, `Created using OpenFrame` footer), so Windows is the authority; any Mac port is an open question.

## 2. The export inventory (from `exports/mod.rs` header table + UI)

| Op | Document | Formats | In scope |
|---|---|---|---|
| `story.export_board` | Beat Board outline / board | PDF · TXT · CSV | **yes** — 2 PDF layouts |
| `story.page.export` | Story page | TXT · PDF · DOCX | no (excluded) |
| `breakdown.export_report` | Breakdown by scene + Catalog with usage | PDF · CSV · XLSX | **yes** — 3 PDF contents |
| `moodboard.export_pdf` | Moodboard | PDF | **yes** |
| `storyboard.export_sheet` | Storyboard sheet | PDF | **yes** |
| `shot.export_list` | Shot List (scene / scenes / project) | PDF · CSV · XLSX | **yes** — 3 scopes |
| `schedule.export_schedule` | Shooting Schedule / stripboard | PDF · CSV · XLSX | **yes** |
| `schedule.export_dood` | Day Out of Days | PDF · CSV · XLSX | **yes** |
| `schedule.export_daily_view` | Daily View of one shooting day | PDF · TXT | **yes** (no baseline PDF) |
| `callsheets.export_pdf` | Call Sheet | PDF | **yes** |
| `sides.export_pdf` | Sides | PDF | no (excluded) |
| `reports.export_report` | Production reports (live or saved) | PDF · CSV · XLSX | **yes** — 6 report types |
| `budget.export_summary` | Budget summary / snapshot | PDF · CSV · XLSX | **yes** |
| `screenplay.export` (interchange module) | Screenplay | various | no (excluded) |

**Breakdown Summary** is *not* a user-reachable export: `BreakdownExportButton` offers only the `content` choices `both` ("Breakdown by scene and Catalog"), `scenes` ("Breakdown by scene"), `catalog` ("Catalog with scene usage"); `CatalogExportButton` fixes `content: "catalog"`. No design was produced for it (see the Breakdown specs for what the builder source says about any other mode).

## 3. UI entry points (verified)

All buttons are in `windows/apps/desktop/src/features/export/buttons.tsx` and open the shared `ExportDialog.tsx`; scopes/options are declared per button.

| Export | Button component | Mounted in | Scope / option arguments |
|---|---|---|---|
| Beat Board | `StoryExportButton` | `workspaces/story/index.tsx:298` | scope `board` / `selected` / `project`; `layout` (`outline`\|`board`), `includeParking`, `includeNotes`, `includeComments` |
| Breakdown | `BreakdownExportButton` | `workspaces/breakdown/index.tsx:186` | scope all / current / selected scenes (`sceneIds`); `content`, `includeSuggested` (default on), `includeNotes` |
| Catalog | `CatalogExportButton` | `workspaces/production/tabs/catalog.tsx:122` | `baseArgs {content:"catalog"}` |
| Moodboard | `MoodboardExportButton` | `workspaces/production/tabs/moodboards.tsx:117` | `moodboardId`; `includeTitle`, `includeImages`, `includeCaptions`, `includeInternalNotes` (default off) |
| Storyboard | `StoryboardExportButton` | `workspaces/production/tabs/storyboards.tsx:226` | scope current / all / selected (`storyboardIds`); `includeNotes` |
| Shot List | `ShotListExportButton` | `workspaces/production/tabs/shots.tsx:180` | scope current / selected / project (`sceneIds`); `includeRemoved` |
| Shooting Schedule | `ScheduleExportButton` | `workspaces/production/tabs/schedule.tsx:220` and `workspaces/call-sheets/shared/ScheduleViews.tsx:273` | scope all / day / days (`dayIds`); `includeUnscheduled` (default on), `includeNotes` |
| Daily View | `DailyViewExportButton` | `workspaces/production/tabs/daily.tsx:59` | `dayId`; formats PDF, TXT |
| Day Out of Days | `DoodExportButton` | `workspaces/production/tabs/dood.tsx:169` | `args` of the view (cast and days shown) |
| Call Sheet | `CallSheetExportButton` | `workspaces/call-sheets/CallSheetList.tsx:204`, `CallSheetEditor.tsx:952` | `id`; `includeOptional` (default on) |
| Reports | `ReportExportButton` | `workspaces/production/tabs/reports.tsx:92` | `reportType` ∈ {scene, location, cast_scene, prop, schedule, breakdown_completeness} + `filter`, or saved `reportId` |
| Budget | `BudgetExportButton` | `workspaces/production/tabs/budget.tsx:258` (with `budgetId`), `:306` | `includeNotes`; `budgetId` selects a saved snapshot |

## 4. Builders and renderer

| Layer | Path (under `windows/`) |
|---|---|
| Document builders | `crates/openframe-application/src/modules/exports/{story,breakdown,visual,schedule,dood,daily_view,callsheet,reports,budget}.rs` (+ `style.rs`, `mod.rs`) |
| Source data models | `crates/openframe-application/src/modules/schedule/{callsheet,board,dood,budget,…}.rs`, `crates/openframe-domain/src` |
| Renderer | `crates/openframe-import-export/src/report.rs` (`Document`, `Block`, `to_pdf_strict`, `MARGIN=46`, `FOOTER_SPACE=34`, `BRAND_LINE`) and `report/design.rs`, `design/{blocks,cards,table}.rs` (`pal`, `Tone`, `Txt`, `Chip`, `Masthead`, `Metrics`, `Section`, `Banner`, `Callout`, `Legend`, `Cards`, `StyledTable`) |
| Low-level PDF | `crates/openframe-import-export/src/pdfgen.rs` (`pdf-writer`), `pdf_fonts.rs`, `winansi.rs` |
| Branding | `exports/mod.rs::deliver` → `out.doc.brand = Some(BRAND_LINE)`; `design::draw_footer` + `lockup()` + `draw_mark` |
| Result object | `ExportResult` (path, pages, scope/contents labels, warnings, `private_notes_excluded`) |

Per-export field-level mappings are in each export's `.md` (section C/D).

## 5. Supplied PDFs → variants

| Supplied PDF | Pages | Page | Variant designed |
|---|---|---|---|
| Beat Board — Outline layout | 5 | Letter portrait | Beat Board Outline |
| Beat Board — Card Board layout | 5 | Letter landscape | Beat Board Card Board |
| Breakdown — By Scene only | 35 | portrait | Breakdown by scene |
| Breakdown — Combined report and Catalog | 58 | portrait | Breakdown + Catalog |
| Catalog — Items with scene usage | 23 | portrait | Catalog |
| Moodboard — image-rich presentation | 2 | landscape | Moodboard |
| Storyboard — Panel Grid Sheet | 6 | landscape | Storyboard sheet |
| Shot List — Single Scene / Multiple Selected Scenes / Entire Project | 1 / 2 / 8 | landscape | three scopes |
| Shooting Schedule — full schedule | 15 | landscape | Shooting Schedule |
| Day Out of Days (DOOD) | 8 | landscape | DOOD |
| Call Sheet — One Shooting Day | 5 | portrait | Call Sheet |
| Production Report — Scene / Location / Cast & Scene / Prop / Schedule / Breakdown Completeness | 6 / 2 / 4 / 7 / 2 / 3 | Scene landscape, others portrait | six report designs |
| Budget — Summary | 4 | portrait | Budget Summary |
| *(none supplied)* | — | — | **Daily View — no baseline** |

The supplied *design-reference folder* (`OpenFrame_PDF_Export_Audit_and_Redesign_Pack/`) contains audit/redesign **documents only** — no earlier HTML or visual designs. The reconciliation rule "structure from one reference, polish from another" therefore reduced to: structure and content = current source + current PDFs; polish = the Windows `report/design.rs` design language, sharpened in `PDF_Design_System.md`.
