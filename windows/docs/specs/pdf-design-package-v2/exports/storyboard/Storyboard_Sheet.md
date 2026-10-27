# Storyboard — panel grid sheet, portrait v2 (`storyboard.export_sheet`)

| | |
|---|---|
| Export | Storyboard sheet (PDF, **portrait** US Letter) |
| Variant | `Storyboard_Sheet` v2 (primary; 2 or 3 panels per row) + `stress-test/Storyboard_Sheet_STRESS_TEST` |
| Status | Design reference — not integrated; not validated against OpenFrame's Rust renderer |
| Repo commit | `2f5a045416bd2125801329533fc47f986ccc8174` (`main`, Windows tree) |
| Date | 2026-10-23 |
| Files | `Storyboard_Sheet.html`; `Storyboard_Sheet.pdf` (2 per row, recommended); `Storyboard_Sheet_3col.pdf` (3 per row); `stress-test/Storyboard_Sheet_STRESS_TEST.html` + `.pdf` (2 per row) + `_3col.pdf`; generator `tools/build/visual/build_storyboard.py`; PDF render step `tools/build/visual/render_storyboard.py` |

### Changes from v1 (landscape) — what changed and why
| # | Change | Why |
|---|---|---|
| 1 | **Portrait** US Letter (612 × 792). The current builder calls `Document::new(..).landscape(true)`; it must become portrait for this export. | User feedback: landscape sheet looked like a plain A4 page and page 1 held only 3 panels. |
| 2 | **Panels per row: 2 or 3, selectable.** 2 is the recommended default; 3 is what the app does today (`columns` defaults to 3). The dialog needs a new option (§B). 4 columns dropped from the design. | User asked for both, chosen at export time. |
| 3 | **Compact header**: one block (eyebrow + title + project left, "N panels · Exported date" right) instead of a title block + stat box. | Page 1 gains ≈ 25 pt. |
| 4 | **Scene banner is one navy band** with an orange accent bar: name, scene line merged under it, panel count and status pill on the right. Same words, one block instead of band + tinted sub-line. | Saves ≈ 15 pt per scene start and reads as one unit. |
| 5 | **Visual lift of the panel card**: navy film-frame border (1.4 pt) around each picture on a neutral tint (picture still *contain*, never cropped); navy panel-number tile + "Panel · Shot 25A"; framing / movement / angle as chips (same words); tinted caption band with bold labels `Sound:` `Duration:` `Note:` (the only labels that exist in the source); action/description text 8.6 pt (2-up: 9 pt) instead of 8.8 pt centred. | Less "plain sheet", faster scanning; only flat fills, hairlines, rounded rects, text and images (all drawable by the renderer). |
| 6 | Caption text is left-aligned inside the band (was centred). | Labelled lines read better flush-left. |
| 7 | Keep rules recomputed for portrait (body 680 pt): same rules, new measurements (§H). Page counts and panels per page in §H.4. | — |
| 8 | Note line colour `--of-neutral-text` (was faint grey 3.1:1). | Contrast. |

No content, field, order, option label or wording was changed. The word "Panel" and the shot label are kept as text next to the number tile (`1  Panel · Shot 1A`), so the numbering stays in the text layer.

---

## A. Purpose and users
The sheet is the printed storyboard read in a prep meeting, handed to a DOP or 1st AD, or pinned on a wall.
- **Director / storyboard artist:** the picture sequence in order, big enough to judge composition; panel numbers that match the app.
- **DOP / 1st AD:** shot label ("Shot 25A"), framing · movement · angle, sound, duration, notes, in the same place under every picture.
- **Production / script supervisor:** which scene a board belongs to; whether the scene changed or was removed since planning.
- **Everyone:** a scene never starts without its banner, and a continuation page still names its scene (running header).

## B. Verified functionality
**Scopes** (`StoryboardExportButton({ current, boards })`, `windows/apps/desktop/src/features/export/buttons.tsx`): "Current storyboard ({name})" (only when a board is open; `storyboardIds: [id]`), "All storyboards" (no ids → every storyboard), "Selected storyboards" (picker, `storyboardIds`). Dialog title "Export Storyboard"; sub-line "A clean storyboard sheet: panels with shot numbers, descriptions and framing."; default document name `Storyboard - {name}` or `Storyboards`; format PDF only.

**Options in the UI today:** one checkbox **Include shot notes** → `includeNotes`, default off.

**Panels per row.** `ExportStoryboardSheetArgs.columns: Option<u8>`, doc comment "Panels per row (2 to 4, default 3)", applied as `a.columns.unwrap_or(3).clamp(2, 4)`. **The UI never sends it** (`StoryboardExportButton` passes only `storyboardIds` and `includeNotes`; nothing under `features/export/` mentions `columns`), so the app always exports **3** per row, for one board or all. The supplied baseline confirms 3. The single-board path (`boards.len() == 1`) only changes masthead/banner, not the column count — the "2 columns for a single storyboard" claim in earlier notes is **not supported by source**.
**New design requirement:** add a dialog option **"Panels per row"** with choices **2 (recommended)** and **3**, sent as `columns: 2 | 3`, default **2** for this design (changing the default from today's 3 is a product decision; the builder's `unwrap_or(3)` would then need to become 2 or the UI must always send it). The builder already accepts 2–4, so no argument change is needed; 4 is no longer designed.

**Orientation.** `export_storyboards` currently calls `Document::new(&document).landscape(true)`. This design is **portrait**: `.landscape(false)` (shot-list exports are separate and unaffected).

**Data:** `storyboard::get` → `StoryboardDto { board: StoryboardSummary{ name, scene_number, scene_heading, scene_removed, needs_review, … }, panels: Vec<PanelDto> }`; `PanelDto { number (derived), visual_kind ("placeholder" | "image" | "sketch"), asset, description, framing, movement, angle, sound_note, duration_ms, note, shot_label }`. Project title via `project_title`.

**Behaviour (unchanged)**
- Each storyboard starts on a new page (`Block::PageBreak` between boards).
- Multi-board: per board a banner (eyebrow "Storyboard", board name, "N PANELS", scene line, status). `banner.keep_next = 150.0`. Single board: no banner; the status (if any) is one line under the header; header title = board name, scene line under it.
- Scene line: `Scene {n}: {heading}` / `{heading}` / `Standalone storyboard`. Status: `scene_removed` → "Scene removed from the script (kept for reference)"; else `needs_review` → "Scene changed since planning".
- Panel: title `Panel {n}` + ` · Shot {label}` when linked; picture ratio 9/16; lines: `framing · movement · angle`, description, `Sound: …`, `Duration: {s}` (`15s`, `2.5s`), and with notes on `Note: …`. Blank panel (`visual_kind == "placeholder"`) → `Blank panel`.
- Empty board: "No panels yet."
- Errors: empty `storyboardIds` → "Select at least one storyboard to export."; no storyboards → `nothing_to_export` "There are no storyboards to export yet.".
- Image problems (`Pics::load`): `Image unavailable: {original file name}`, `Image unavailable` (no asset), `Image can't be printed`. Result warnings (`image_warnings(.., "panel image")`): "{n} panel image(s) could not be found and is/are shown as a placeholder. The references in your project were not changed." / "… could not be read as a picture and …".
- `Grid`/`GridCell` (report.rs) are not used here (Beat Board uses them); this builder uses `Cards` + `CardPart::Image` + `CardPart::Centered`.

### B.1 Current-output assessment (baseline `Storyboard — Panel Grid Sheet.pdf`, 6 pages, 12 panels, 11 images; all boards, notes on, landscape, 3 columns)
**Retain:** content and order; banner per board with scene line and status; "Panel n · Shot X"; caption stack order; `Blank panel`; contain-fit pictures; one board per page start.
**Improve visually:** the sheet reads as a plain table of grey-bordered boxes (user feedback); notes in faint grey; centred captions with no hierarchy; chips/frames/number tiles added in v2.
**Verified functional / layout problems:** (1) p.2 holds only "Panel 4" and p.4 only panels 4–5: lone rows on near-empty pages; (2) continuation pages (p.2, p.4) carry no scene name; (3) page 1 holds one row of 3 because masthead (≈ 100 pt) + banner (≈ 60 pt + 150 pt keep) leave room for one 220 pt row; (4) low-contrast notes (`pal::FAINT` 3.1:1); (5) long tracked footer.
**Cannot be determined from the evidence:** whether `columns` was meant to be exposed; Mac behaviour (not examined); the intended meaning of board "Scene 33: …" linked to scene 34 (board name is user text, printed verbatim).

## C. Source mapping
| | |
|---|---|
| Commit | `2f5a045416bd2125801329533fc47f986ccc8174` |
| UI entry | `buttons.tsx` → `StoryboardExportButton({ current, boards })`, in `windows/apps/desktop/src/workspaces/production/tabs/storyboards.tsx` (`PageHeader actions`, before "New Storyboard" / "Add Panel"; `disabled={boards.length === 0}`). New option "Panels per row" goes in its `options` array (`{ kind: "choice", key: "columns", label: "Panels per row", default: "2", options: [{value:"2",label:"2 (recommended)"},{value:"3",label:"3"}] }` — shape modelled on the `choice` options already used by `StoryExportButton`; the value must reach the op as a number). |
| Op | `storyboard.export_sheet` |
| Builder | `modules/exports/visual.rs::export_storyboards`, `ExportStoryboardSheetArgs { storyboard_ids, format, path, include_notes, columns }` |
| Data | `modules/visual/storyboard.rs`: `StoryboardDto`, `StoryboardSummary`, `PanelDto`; `util::AssetInfo` |
| Renderer blocks | `Masthead` (to become a compact header), `Banner` (to gain the merged scene line + accent bar), `Text`, `Block::PageBreak`, `Cards` with `Card{ head: CardHead, parts: [CardPart::Image{ratio 9/16}, …] }` — v2 needs: a tile in `CardHead` (`Tile` exists in `cards.rs`: number tile with fill/colour), chips row, a tinted caption part (new `CardPart` or `Text` on a filled rect), image frame stroke 1.4 pt navy (today 0.6 pt `pal::LINE`, fill `#eef1f5`) |
| Geometry facts (verified) | `cards.rs`: column width `(content_w − gap × (cols−1)) / cols`, `Cards.gap` default 10; `PAD` 10; head height = text + 18; image part `fw = w − 2·PAD`, `fh = fw × ratio`, contain fit; rows placed whole; all cards in a row stretched; row taller than a page splits by part. `blocks.rs::banner`: `ensure(total + keep_next + 4)` |

## D. Field inventory
| Visible element | Source | Kind | Empty-value treatment |
|---|---|---|---|
| Eyebrow "Pre-production · Storyboard" | literal | fixed | — |
| Title | "Storyboards" (multi) / `board.name` (single) | required | — |
| Project (next to title) | `project_title` | optional | omitted |
| Scene line under title (single board) | `scene_line` | derived | "Standalone storyboard" |
| "N panels · Exported {date}" | sum of panels; `today_label` | derived | "0 panels" |
| Banner name / eyebrow "Storyboard" / "N PANELS" | `board.name`, literal, count | multi-board only | — |
| Banner scene line | `scene_line` | derived | "Standalone storyboard" |
| Status pill / status line | `scene_removed`, `needs_review` | conditional | omitted |
| Number tile | `PanelDto.number` | derived | — |
| "Panel" + " · Shot {label}" | literal; `shot_label` | conditional | shot omitted |
| Picture | `asset` (`visual_kind` image/sketch) | conditional | placeholders (§B) |
| Blank panel | `visual_kind == "placeholder"` | conditional | "Blank panel" |
| Chips | `framing`, `movement`, `angle` (each its own chip) | optional | chip omitted; row omitted if none |
| Action line | `description` | optional | omitted |
| `Sound:` | `sound_note` | optional | omitted |
| `Duration:` | `duration_ms > 0` | optional | omitted |
| `Note:` | `note`, only with Include shot notes | optional | omitted |
| "No panels yet." | no panels | conditional | — |

Source detail: the single combined line `framing · movement · angle` becomes up to three chips; the words are identical, the " · " separators are dropped.

## E. Content architecture
Order is the source order: header → per board [new page] → banner (multi) → panel rows in panel order (row-major, left to right). No grouping change. The only structural changes are the merged banner (scene line moved into the band) and the chip split (see "Changes from v1").

## F. Visual specification
**Page.** US Letter portrait 612 × 792; margins 46 / 46 / 46 top-left-right, 66 bottom; content box **520 × 680 pt**. Footer: shared system. **Running header** (kept): one line top-right, 6.8 pt slate tracked: `LOW TIDE RADIO · STORYBOARDS · {BOARD NAME}` (single board: `STORYBOARD`); set per storyboard; a name over 44 characters is shortened with "…" (page furniture only).

**Header (document start only), ≈ 38 pt + 8 pt gap.** Left: eyebrow 7 pt bold tracked slate; title 17 pt bold navy ("Storyboards" / board name) with the project name inline, 9 pt slate; single board: scene line 8.4 pt bold slate. Right: "**12 panels**" (9.6 pt bold navy) · "Exported 22 Oct 2026" (8.2 pt slate), bottom-aligned. Rule 1.2 pt navy under the block, 6 pt padding.

**Scene banner, ≈ 49 pt (one-line title) + 8 pt gap.** Navy rounded rect (radius 6), 5 pt **orange** accent bar on the left, padding 6 / 12 / 7 / 10. Left: eyebrow "STORYBOARD" 6.8 pt bold tracked `#f5c27a`; board name 12.5 pt bold white (wraps to more lines); scene line 8.2 pt bold `#dbe3f0`. Right: "4 PANELS" 8.2 pt tracked white; below it the status pill (7.2 pt bold; amber tokens = "Scene changed since planning", red tokens = "Scene removed from the script (kept for reference)"); pill and count are right-aligned, max 42% of the width.

**Panel card.** 2 per row: gap 10 → card width **255 pt**; 3 per row: gap 8 → **168 pt**. Padding 6; border 0.5 pt `--of-line-strong`, radius 6, white; rows 10 pt (2-up) / 8 pt (3-up) apart. Vertical stack, gap 4:
1. **Head** (15 pt): navy tile (radius 3.5, min 15 pt wide, number 9 pt bold white) · "Panel" 7.6 pt slate · "· Shot 25A" 8.6 pt bold teal (2-up 9 pt). Shot part omitted when not linked.
2. **Picture frame** 16:9: **2-up 243 × 136.7 pt**, **3-up 156 × 87.8 pt** (card width − 12). Border **1.4 pt navy**, radius 2.5, fill neutral tint `#e6ebf2`; picture scaled `min(frame_w/img_w, frame_h/img_h)`, centred (**contain, never cropped**: 16:9 fills the frame; tall/square pictures are pillarboxed, ultrawide letterboxed on the tint).
3. **Chips** (framing, movement, angle): 7.2 pt (2-up 7.6) bold navy on `--of-tint`, 0.5 pt `--of-line-strong`, radius 3, padding 2.6 / 4.6, gap 2.5; wrap to a second line when the card is narrow ("Handheld follow" at 3-up). User text stays sentence case (no uppercase transform).
4. **Caption band** (fills the rest of the card so bands align along a row): fill `--of-panel`, 0.5 pt `--of-line`, radius 4, padding 4.5 / 6 / 5. Action line = description, **8.6 pt ink (2-up 9 pt)**, leading 1.28; `Sound:` / `Duration:` 7.8 pt (2-up 8.2) slate with the label bold navy; `Note:` 7.8 pt neutral text with a hairline above it and bold navy label. Left-aligned. Lines absent in the data are omitted.
- Placeholder (no picture): same frame, text 8.2 pt `--of-neutral-text`, centred, wraps: `Blank panel`, `Image unavailable: {file}`, `Image unavailable`, `Image can't be printed`.
- Typical row height (measured): 2-up ≈ 215–240 pt, 3-up ≈ 187–210 pt.

**Grayscale / status.** Number tile, banner band and frame border are dark on any print; status is words in a pill; chips and bands are labelled by text, not colour. Checked with `verify_pdf.py --gray`.

## G. Dynamic behaviour
- **Panels per row 2 / 3:** same content; caption sizes per §F; the row grid is `cols` equal columns with the last row left-aligned.
- **Long description / sound / note:** wrap inside the band; the band and card grow; the row stretches every card to the tallest (stress A panel 1: ≈ 330 pt row at 2-up).
- **Many chips / long chip text:** chips wrap onto a second line; a chip is never truncated.
- **Tall / square pictures:** pillarboxed in the 16:9 frame (stress A panels 2–3). **Missing / unreadable / no asset:** placeholder with exact wording (panels 4, 6, 7). **Blank panel** with long shot label "Shot 118AAB-LONG" (head wraps, tile stays).
- **No framing / description / sound / duration:** the chip row and/or band are absent (blank panel in stress D shows only head + frame).
- **Shot notes off:** `Note:` lines not printed. **No panels:** banner "0 PANELS" + "No panels yet.". **Standalone:** banner scene line "Standalone storyboard". **Single-storyboard scope:** header carries the board name and scene line; no banner; status as one line.
- **Long board name / scene line:** banner grows (stress A: name on 2 lines, scene line on 2 lines, status pill beside the count).
- Optional parts leave no gap.

## H. Pagination and print rules
Body height 680 pt. Rules (unchanged in kind, re-measured for portrait):
1. A storyboard always starts on a new page; the header appears on page 1 only.
2. Rows are whole; cards in a row have equal height; a row is split only if it is taller than a page.
3. **Banner (and header) stay with the first row**: inseparable.
4. **Whole scene together when it fits** (header on page 1 + banner + all rows ≤ 680 pt).
5. Otherwise rows are paginated by measured height with **widow repair** (no page other than the first carries a single row when the previous page can spare one; cascading).
6. Running header names the storyboard on every page. Footer from the shared system, "Page X of Y" true total.
7. Implementation in the HTML: a small measuring script marks `.k` (keep with next) and `.pb` (break before) from rendered row heights, as the Rust renderer would do from its measurements.

### H.4 Panels per page (measured in the delivered PDFs)
| Mode | Page 1 (header + banner) | Later pages (no header/banner on continuation; banner on a new scene) |
|---|---|---|
| **2 per row** | 2 rows = **4 panels** (content area free for rows ≈ 577 pt; a row is 215–240 pt; a 3rd row needs ≈ 700 pt) | 2 rows = **4 panels** (a 3rd row would need ≈ 700 pt of 680) |
| **3 per row** | 2 rows = **6 panels** guaranteed; a 3rd row (9 panels) fits only with short captions (rows ≈ 187 pt: 3 × 187 + 16 = 577 ≈ the free height) | 3 rows = **8–9 panels** (stress 14-panel scene: page with rows 3–5 carries 8 panels) |

Examples (baseline data): 2-up PDF 5 pages: p1 scene 1 (4 panels, whole scene), p2–3 scene 25 (5 panels: widow repair gives row 1 | rows 2–3), p4 scene 34, p5 scene 40. 3-up PDF 4 pages: p1 scene 1 (3 + 1), p2 scene 25 (3 + 2, whole scene on one page), p3 scene 34, p4 scene 40. v1 landscape needed 5 pages for the same data at 3 columns; the baseline needed 6.
Unavoidable lone rows: a scene whose last row is a single panel and that cannot be rebalanced (e.g. 5 panels at 2-up: panel 5 sits in a 2-row page), and one-row scenes. These are scene-end rows, not widowed pages.

## I. HTML / PDF correspondence
- `Storyboard_Sheet.html` contains four views (preview toolbar, radio-like): **All storyboards · 2 panels per row (recommended, default)**, **All · 3 panels per row**, **Current storyboard only · 2 per row** and **· 3 per row** (Harbour breakwater board). Toggle **Include shot notes**.
- **How each PDF is produced** (state selected through the URL hash, read by the page script; no edit of `tools/render_pdf.py`): `python tools/build/visual/render_storyboard.py` runs headless Chrome 155 with the same flags as `tools/render_pdf.py` on `Storyboard_Sheet.html#cols=2` → `Storyboard_Sheet.pdf`, `#cols=3` → `Storyboard_Sheet_3col.pdf`; likewise the stress HTML → `stress-test/Storyboard_Sheet_STRESS_TEST.pdf` (`#cols=2`) and `…_3col.pdf`. Hash keys: `cols=2|3`, optional `scope=one`. Opening the HTML with no hash shows the 2-up view; the toolbar switches views; **Print / Save as PDF** prints the visible view. Both primary PDFs: all storyboards, Include shot notes **on** (as in the baseline).
- Data: the baseline's real texts (4 boards, 12 panels, 11 images, 1 blank). Date "22 Oct 2026" is the baseline export date.
- Stress (`stress-test/`, 9 pages at 2-up, 6 at 3-up): A) very long board name, scene line and status; description of 300 characters + long sound + long note; tall 9:16 and square 1:1 images (derived crops of real sample frames, in `stress-test/assets/`); missing file; blank panel with long shot label; unreadable file; no-asset; B) 14-panel scene over several pages (widow repair); C) board with no panels, scene removed; D) standalone storyboard with a blank panel. The 4-column renders of v1 were removed.
- Known differences vs the Rust renderer: CSS grid rows ≈ `Cards` rows; the measuring script replaces `ensure()`; chips/tile/band are not existing `CardPart`s; running header/footer need renderer support; the shared logo must be embedded as PNG; text metrics differ slightly.

## J. Acceptance criteria
1. Portrait Letter; every page has the shared footer (logo 17 pt high, brand line, "Page X of Y" true total); nothing but the footer in the bottom 66 pt.
2. Each storyboard starts on a new page; header only on page 1; running header names the storyboard on every page.
3. `columns = 2` and `= 3` both produce a grid of exactly that many equal columns; default 2 when the option is untouched in this design.
4. Page 1 holds at least 2 rows at 2 and 3 per row (given typical captions); the banner is never separated from its first row.
5. A scene that fits in one page's body is on one page; no page other than the first carries a single row when avoidable.
6. Every picture sits in a 16:9 frame with a 1.4 pt navy border; bitmap aspect = source aspect (±1%); never cropped; portrait/square pillarboxed.
7. Panel number tile shows `PanelDto.number`; "Panel" and "· Shot X" printed; shot label only when linked.
8. Chips contain exactly the framing, movement and angle words; no chip for missing values.
9. Caption labels are exactly `Sound:`, `Duration:`, `Note:`; description unlabelled; `Note:` only with Include shot notes. Action text ≥ 8.4 pt.
10. Placeholder strings exact; `Blank panel` only for `visual_kind == "placeholder"`; status wording exact; "No panels yet."; "Standalone storyboard".
11. No overlap, no clipping beyond margins at any caption length; grayscale print readable.
12. Orange appears only as the banner accent bar and brand footer elements (never as text).

## K. Known issues and unresolved questions
**Verified defects in the current output / source:** lone panel rows and near-empty pages; continuation pages without scene identity; low-contrast notes; long tracked footer; page 1 holds one row; landscape orientation (user feedback).
**Source vs other evidence:** earlier claim "2 columns for a single storyboard" is not in source (default 3 for all scopes, no UI). The brief's `Grid/GridCell` are not used by this builder.
**Design choices (mine):** portrait; 2 as recommended default; merged banner; number tile / chips / caption band / navy frame; left-aligned captions; per-storyboard running header; widow repair.
**Unverified assumptions:**
- The Rust renderer can print a per-page header text, draw a tile in a card head, a chips row and a tinted caption rect (not in the current `CardPart` list), and merge the banner sub-line into the band. If merging is not possible, keep the current banner (band + tinted sub-line, ≈ 15 pt more) — page 1 still holds 2 rows at 3 per row.
- Real Rust row heights (the numbers above are Chrome measurements).
- The dialog's `choice` option can send a numeric `columns`.
- Mac behaviour not examined.
- 3 rows on page 1 at 3 per row are borderline (§H.4); do not promise them.
