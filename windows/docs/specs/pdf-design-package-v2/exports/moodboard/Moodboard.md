# Moodboard — image-rich presentation (`moodboard.export_pdf`)

| | |
|---|---|
| Export | Moodboard (PDF, landscape US Letter) |
| Variant | `Moodboard` (primary) + `stress-test/Moodboard_STRESS_TEST` |
| Status | Design reference — not integrated; not validated against OpenFrame's Rust renderer |
| Repo commit | `2f5a045416bd2125801329533fc47f986ccc8174` (`main`, Windows tree) |
| Date | 2026-10-23 |
| Files | `Moodboard.html` / `.pdf` / `.md`; `stress-test/Moodboard_STRESS_TEST.html` / `.pdf`; generator `tools/build/visual/build_moodboard.py` |

---

## A. Purpose and users

A moodboard PDF is the reference deck a director, DOP, production designer or costume/art lead sends to someone who will never open OpenFrame: "this is what the film should feel like". Roles and what they must find fast:

- **Director / DOP / production designer:** the pictures, large and uncropped, with the one-line caption saying why each image is there (palette, texture, light).
- **Producer / department heads receiving it:** which board this is (name, project), which scene it supports (when linked), the notes and links that explain the pictures.
- **Anyone forwarding it:** certainty that private material has not leaked. Internal items and the internal board notes are printed only when "Internal notes" is ticked, and are then unmistakably labelled.

Images are the hero of this document. Everything else (title block, captions, notes) must stay quiet and never crop, shrink-to-nothing or push a picture off the page.

## B. Verified functionality

**Scope.** One moodboard (the board currently open in Production → Moodboards). No multi-board scope.

**Dialog** (`ExportDialog`, `MoodboardExportButton`): title `Export moodboard: {board name}`; default document name `Moodboard - {board name}`; format `PDF` only (`PDF_ONLY`, hint "printable snapshot"); no scope list. Footer note in the dialog: "Private / internal notes excluded unless you tick Internal notes. Export creates a snapshot and does not change your project."

**Options (exact UI label → arg key → default → effect, from `export_moodboard`):**

| UI label | Arg key | Default | Effect |
|---|---|---|---|
| Project title and board name | `includeTitle` | on | on: masthead with eyebrow "Visual development · Moodboard", board name, project name, optional "Reference for Scene…" line, side panel (image count, export date); document title "Moodboard: {name}", footer "{project} · Moodboard: {name}". off: masthead is only the word "Moodboard"; document title and footer are just "Moodboard" (no project, no board name anywhere). |
| Images | `includeImages` | on | off: image cards lose their picture frame; a card keeps only its caption (and "Internal" label); a card with neither is dropped. |
| Captions | `includeCaptions` | on | off: image captions and note/link titles (captions) are not printed (notes/links are then titled "Note" / "Link"). |
| Internal notes | `includeInternalNotes` | **off** | on: items with `is_private` are included (marked "Internal") and the board's own notes print under "Internal notes — Not for distribution". off: both are omitted. |

Other args: `moodboardId`, `format` ("pdf"), `path`.

**Data dependencies.** `moodboard::get` → `MoodboardDto { board: MoodboardSummary{ name, scene_number, scene_heading, … }, notes: Option<String>, items: Vec<MoodboardItemDto> }`. Item fields used: `kind` ("image" / "note" / "link"), `x`,`y`,`z` (reading order only), `caption`, `body`, `url`, `link_title`, `asset: Option<AssetInfo>`, `is_private`. Project name from `schedule::board::project_title`. Date = today (`today_label`, e.g. "22 Oct 2026").

**Ordering.** Items are sorted by `(y / 40, x, z)` (top to bottom in 40-unit bands, then left to right): the canvas reading order. Images are then split from notes/links; all images print first (grid), then "Notes and links".

**Empty / error behaviour.**
- No printable content at all (no image card, no note/link, and no internal notes) → export fails with `nothing_to_export`: "This moodboard has nothing to export with the selected options."
- Image file missing → picture frame holds the placeholder text `Image unavailable: {original file name}`; image has no asset record → `Image unavailable`; file unreadable or too large → `Image can't be printed`. The project reference is never changed.
- Export result warnings (shown in the export result, not printed in the PDF), from `mod.rs::image_warnings(missing, unreadable, "image")`: "`{n} image(s)` could not be found and is/are shown as a placeholder. The references in your project were not changed." and "`{n} image(s)` could not be read as a picture and is/are shown as a placeholder."

### B.1 Current-output assessment (baseline `Moodboard — image-rich presentation.pdf`, 2 pages, 4 images)

**Retain**
- Title block content and order: eyebrow, board name, project, side panel "N images / Exported {date}", rule.
- Images shown whole inside a uniform frame (`CardPart::Image`, ratio 0.72, picture fitted inside = contain; p.1 shows letterbox bars, so nothing is cropped).
- Optional centred caption under the picture; "Internal" marker on private items.
- Second section "Notes and links" (two-up cards, caption-or-kind as title, kind eyebrow when captioned, "Internal" chip) and "Internal notes / Not for distribution" callout.
- Wording: `Image unavailable…`, `Image can't be printed`, "Internal", "Not for distribution".

**Improve visually**
- p.1: the masthead title uses the 28 pt display size, so a normal board name wraps to two lines and eats ~35 pt of the page; use the compact (20 pt) title for landscape.
- p.1: 4 images fall 3 + 1; the lone fourth card sits under empty space. Row-height equalisation (retained) means a caption-less card has a blank strip where the neighbours have text.
- Orange eyebrow text (3.0:1 on white) → slate eyebrow, orange only as brand fill (shared system rule).
- Footer carries project + board name in a long tracked line; the shared footer (logo, "Created using OpenFrame", "Page X of Y") replaces it and a one-line running header carries the document identity.
- Frame placeholder is a plain grey rect with faint text (`pal::FAINT`, 3.1:1) → muted text (4.8:1).
- Internal label is 6.4 pt coloured text only → chip with the word "Internal" (6.6 pt floor, amber fill, text label survives grayscale).

**Verified functional / layout problems**
1. p.1: masthead says "4 images" regardless of options. Source: `image_count` is incremented for every image item before `with_images` is checked and before the "card is empty" drop, so with Images off the header can announce images that are not printed (source `visual.rs`, `"image" =>` arm). Not visible in the baseline (Images was on).
2. p.2: the baseline was exported with Internal notes ticked (Internal chip and "Internal notes" section are printed); that is consistent with the option, not a defect, but the PDF alone cannot tell a recipient that it was meant to stay internal beyond the chip/section label.
3. The page-2 section ("Notes and links") begins a new page only because the image rows filled page 1; nothing keeps the section heading with its first row except the generic "heading + next block" rule (`Section` is followed by `Cards`, rows are placed whole).

**Cannot be determined from the evidence**
- How `x/y/w/h` canvas size of an item should influence print size (the builder ignores `w`/`h`; a large canvas image prints the same size as a small one). Whether that is intended is a product question.
- Behaviour of the Mac app (`mac/` not examined).
- Behaviour with a scene-linked board: the supplied baseline board has no scene link, so the "Reference for Scene {n}: {heading}" line is verified from source only; the design shows it as an illustrative toggle.

## C. Source mapping

| | |
|---|---|
| Commit | `2f5a045416bd2125801329533fc47f986ccc8174` |
| UI entry | `windows/apps/desktop/src/features/export/buttons.tsx` → `MoodboardExportButton({ board })`; rendered in `windows/apps/desktop/src/workspaces/production/tabs/moodboards.tsx`, `Header` → `PageHeader actions` (`{current && <MoodboardExportButton board={current} />}`), next to "New Moodboard". |
| Op | `moodboard.export_pdf` |
| Builder | `windows/crates/openframe-application/src/modules/exports/visual.rs::export_moodboard` (args `ExportMoodboardArgs`) |
| Helpers | `mod.rs`: `footer`, `nonblank`, `plural`, `load_image`, `image_warnings`, `today_label`; `visual.rs::Pics::load` (placeholder wording) |
| Data | `modules/visual/moodboard.rs`: `MoodboardDto`, `MoodboardSummary`, `MoodboardItemDto`; `util::AssetInfo` |
| Renderer blocks | `Masthead` (eyebrow, subtitle, lines, `Side::Panel`, rule), `Cards::new(3, …)` of `Card` with `CardPart::Image{ img, placeholder, ratio: 0.72 }` and `CardPart::Centered([caption, "Internal" label])`; `Section("Notes and links")`; `Cards::new(2, …)` of `Card{ head: CardHead{ eyebrow, title, right: Chip "Internal" }, parts: [CardPart::Text] }`; `Section("Internal notes", "Not for distribution")` + `Callout(Tone::Warn)` |
| Renderer files | `openframe-import-export/src/report/design/cards.rs` (card geometry: `PAD` 10, head height = text + 18, image part `fw = w − 20`, `fh = fw × ratio`, frame fill `#eef1f5` + 0.6 pt line, picture scaled `min(fw/iw, fh/ih)` and centred), `report.rs` |

Card geometry facts used by the design (verified in `cards.rs::cards`): column width `(content_w − gap × (cols−1)) / cols` with `gap = 10`; a row of cards is placed whole (`p.ensure(row_h + 2)`), every card in the row is stretched to the row's tallest; a row taller than a page splits card-by-card between parts.

## D. Field inventory

| Visible element | Source field / struct | Kind | Empty-value treatment |
|---|---|---|---|
| Eyebrow "Visual development · Moodboard" | literal | fixed (only with title) | — |
| Title (board name) | `board.board.name` | required | with title off: literal "Moodboard" |
| Project line | `project_title` via `nb()` | optional | omitted |
| "Reference for Scene {n}: {heading}" / "Reference for {heading}" | `scene_number`, `scene_heading` | conditional (linked scene) | omitted |
| Side panel line 1 | count of image items (`plural(n, "image", "images")`) | derived | "0 images" for a text-only board |
| Side panel line 2 | `Exported {today_label}` | derived | — |
| Image | `item.asset` via `load_image` | repeatable | placeholder (see §B) |
| Image caption | `item.caption` (trimmed) | optional | omitted; card keeps the picture only |
| "Internal" label under image | `item.is_private` | conditional (only when Internal notes on) | — |
| Note / link card title | `item.caption`, else the kind word "Note" / "Link" | derived | — |
| Card eyebrow ("Note"/"Link") | kind, only when a caption is the title | conditional | — |
| Card body (note) | `item.body` | optional | card with head only |
| Card body (link) | `link_title (url)`, or `url`, or `link_title` | optional | card with head only |
| "Internal" chip on card head | `item.is_private` | conditional | — |
| Internal notes callout | `board.notes` | conditional (Internal notes on and non-blank) | section omitted |

## E. Content architecture

Order (unchanged from the current PDF): masthead → image grid → "Notes and links" → "Internal notes". Sections that are empty leave no heading and no gap (the source has no empty-state sentence for any of them; the only message is the `nothing_to_export` error).

Table of changes versus the current output:

| # | Change | Why |
|---|---|---|
| E.1 | Compact 20 pt title in the masthead | a 28 pt landscape title wraps and costs a row of images |
| E.2 | Slate eyebrow, light "stat box" for count + date | shared-system contrast rule; same content |
| E.3 | **Columns: 3 for 3+ images, 2 for 1–2 images** (source: always 3). Renderer change is a one-liner: `Cards::new(if n <= 2 {2} else {3}, cards)` | one or two pictures as 3-up thumbnails waste the page and are the opposite of "images are the hero"; frames stay in the 0.72 ratio so no image is ever cropped |
| E.4 | Running header (one line) "LOW TIDE RADIO · MOODBOARD" top-right | the shared footer no longer carries project/board text; the board name stays in the masthead (page 1). With **Project title and board name** off the header is just "MOODBOARD" (no project anywhere) |
| E.5 | Internal label becomes a chip with text | grayscale-safe, 6.6 pt floor |

## F. Visual specification

**Page.** US Letter landscape 792 × 612 pt; margins 46 / 46 / 46 top-left-right, 66 bottom (footer zone); content box 700 × 500 pt. Footer: shared system (logo lockup 17 pt high, "Created using OpenFrame", "Page X of Y"). **Running header:** yes, compact single line top-right, 6.8 pt, slate, tracked: `{PROJECT} · MOODBOARD`; with "Project title and board name" off: `MOODBOARD`. It does not carry the board name (a long name would not fit one line; it is in the masthead).

**Masthead.** Eyebrow 7.6 pt bold tracked slate; board name 20 pt bold navy; project 9.6 pt slate; optional scene line 8.6 pt slate; right: stat box (fill `--of-panel`, 0.5 pt `--of-line`, radius 6, padding 7 / 14 / 8, min-width 118): bold 9.6 pt navy "4 images" + 8.2 pt slate "Exported 22 Oct 2026". Rule 1.2 pt navy under the block, 8 pt padding above it, 10 pt below. Title off: plain 20 pt "Moodboard" + the same rule.

**Image card.** 3-up: column width (700 − 2×10)/3 = **226.7 pt**; card padding 8; border 0.5 pt `--of-line-strong`, radius 6, white fill. Picture frame inside: 210.7 × **151.7 pt** (ratio 0.72), fill `#eef1f5` (flat), 0.5 pt hairline, radius 3. 2-up: column 345, frame 329 × 236.9.
- **Fit policy: contain, never crop.** The picture is scaled by `min(frame_w / img_w, frame_h / img_h)` and centred; the remaining bars are the flat frame colour. Portrait, square and panoramic images therefore appear whole with side bars or top/bottom bars. (This is exactly what `CardPart::Image` already does; the design keeps it and forbids a cover mode.)
- **Why 0.72 and not 9/16.** A landscape-biased uniform frame that still gives portrait/square pictures usable height. 16:9 pictures get thin bars (≈ 22% of height). Kept from source; the frame ratio is a constant, not per image.
- Caption: below the frame, gap 6, centred, 9 pt ink, line-height 1.3, full card inner width (210.7 pt at 3-up ≈ 38 characters per line); wraps to as many lines as needed.
- "Internal" chip: centred under the caption, `.chip.warn`, 6.6 pt bold uppercase.
- Gutters: 10 pt between columns and between rows.

**Placeholder (frame, no picture).** Flat `#eef1f5` frame, same size as a picture frame, text centred 8.5 pt `--of-muted`, padding 8 (wraps; a long file name wraps, never truncated). Strings exactly: `Image unavailable: {original file name}`, `Image unavailable`, `Image can't be printed`.

**Notes and links.** Section heading (`.sec`: 10 pt bold tracked caps navy, 1.2 pt navy rule). Two-up cards, 10 pt gap. Card head: fill `--of-panel`, 0.5 pt divider, padding 6 / 10; optional eyebrow 6.6 pt bold tracked slate; title 10 pt bold navy, **sentence case** (user text is never upper-cased); Internal chip right-aligned. Body: 8.8 pt ink, leading 1.35, padding 6 / 10 / 8; URLs wrap anywhere.

**Internal notes.** Section heading with right hint "Not for distribution" (7.4 pt slate, bold); callout `warn` (amber fill, 9 pt amber-deep text, leading 1.4).

**Status / grayscale.** Internal is always the word "Internal" (chip) plus the section title "Internal notes — Not for distribution"; nothing depends on colour. Checked in grayscale (`verify_pdf.py --gray`): chips and callouts stay readable.

## G. Dynamic behaviour

- **Zero images:** the grid is omitted entirely, no gap; side panel says "0 images"; "Notes and links" starts directly under the masthead rule (stress case 3).
- **One or two images:** 2-column grid, left-aligned, remaining column empty (stress case 2). Notes cards below keep their own 2-up grid.
- **Twelve images:** 3-up, 4 rows; two rows fit on the first page (≈ 185–200 pt per row); continuation pages carry rows only, no repeated masthead (stress case 1: 4 pages incl. the notes section).
- **Very long caption:** wraps; the row grows to the tallest card, every card in that row stretches to it (stress case 4: ≈ 280 pt row). A row that is taller than the page body is the one case where the renderer's `split_card` applies; the design does not try to shrink text.
- **Missing / unreadable / no-asset image:** same frame size with placeholder text; caption still prints; the export result lists the warning.
- **Images off:** image cards print only caption + Internal label; caption-less cards disappear (stress case 5). Masthead count still counts image items (source behaviour, see §B.1 problem 1).
- **Captions off:** captions omitted; note/link titles fall back to "Note"/"Link".
- **Title off:** plain masthead and plain header (stress case 6).
- **Optional lines:** project, scene reference, internal notes, notes/links are removed without leaving a gap.
- **Long URLs / words:** `overflow-wrap:anywhere`; never ellipsised.
- **Order:** canvas reading order (§B). Private items appear in that same position when Internal notes is on.

## H. Pagination and print rules

- US Letter landscape, margins 46 / 46 / 46 / 66; footer in the bottom 66 pt only.
- Rows of cards are never split (a row moves whole to the next page); a card inside a row is never split unless the row is taller than the page.
- The masthead keeps with the first image row. A section heading ("Notes and links", "Internal notes") keeps with its first row/callout (`break-after: avoid`); an orphan heading at a page foot is a failure.
- Note/link rows move whole; cards stretch to the row height.
- No repeated table headers (no tables). No repeated masthead. No extra blank pages.
- Page numbering "Page X of Y" from the real total.
- Widow rule for image rows: not applied (a last row with one or two cards is normal for a moodboard); a final page holding only the notes section is acceptable.

## I. HTML / PDF correspondence

- `Moodboard.html` is the primary; **the PDF captured the default option state** (title on, images on, captions on, Internal notes **off**, no linked scene): 2 pages, 4 images (3 + 1), then "Notes and links" with the Link and Note cards of the baseline. The baseline's private note ("Budget note for Vivek: balloon lights only on the beach night.") and the board's internal note ("Shared with Vivek Anand (DOP) and Lakshmi Iyer (design). Grade reference for the DI.") exist in the HTML behind the **Internal notes** toggle.
- Toggles (preview toolbar): `Project title and board name`, `Linked scene line (illustrative)` (the "Reference for Scene 37: EXT. KADALKKARA BEACH - DUSK" text is **invented for illustration**; the real board has no scene link), `Images`, `Captions`, `Internal notes`. A small inline script handles the "title off" plain masthead (shown while the title toggle is unchecked).
- `stress-test/Moodboard_STRESS_TEST.html/.pdf`, 10 pages, six boards back to back (each on a fresh page; the board name states the case): (1) twelve images: 16:9, **portrait 2:3, square 1:1, panorama 4:1** (derived crops of the real sample images, kept in `stress-test/assets/`), **missing file**, **unreadable file**, **no asset**, an image marked Internal, a very long caption, a note with a very long caption-as-title, a long URL, an Internal note and the internal-notes callout; (2) one image; (3) no images; (4) one very long caption among short ones with a portrait neighbour; (5) Images option off; (6) title option off (plain masthead, header "MOODBOARD").
- Rendered with headless Chrome 155 via `tools/render_pdf.py`; reproducible through `python tools/build/visual/build_moodboard.py`.
- **Known differences vs the Rust renderer:** CSS grid rows stand in for `Cards` rows (same equal-height / whole-row behaviour); Chrome text metrics differ slightly from `text_width`; the header/footer are CSS margin boxes (production draws them as page furniture); the shared logo must be embedded as a PNG in production; the `stress-test` placeholder frames use a flat fill (the shared `.frame.missing` hatch gradient is overridden because the production renderer has no gradients).

## J. Acceptance criteria

1. Landscape Letter; every page has footer logo (17 pt high), "Created using OpenFrame", "Page X of Y" with the true total; nothing but the footer inside the bottom 66 pt.
2. Running header is exactly `{project} · MOODBOARD` (or `MOODBOARD` when the title option is off) on every page; the board name never appears in it.
3. With default options no text of any item with `is_private`, and not `board.notes`, appears anywhere in the PDF (search the PDF text for the private strings).
4. With Internal notes ticked each private item shows "Internal" and the board notes appear under "Internal notes" with "Not for distribution".
5. No image is cropped: for every image the drawn bitmap's aspect ratio equals the source's within 1%, and lies inside its frame.
6. Frame ratio 0.72 for every image; placeholders have the same frame size; strings match §F exactly.
7. 0 images → no empty frame, no gap, "0 images". 1–2 images → 2 columns. 3+ → 3 columns.
8. A caption of 400 characters wraps inside its card, the row grows, cards in the row share one height, nothing overlaps the next row or the footer.
9. A "Notes and links" / "Internal notes" heading never sits alone at the bottom of a page.
10. Title option off → masthead is only "Moodboard"; neither project nor board name occurs in the text layer (document title, header, footer).
11. Images off → only captioned/Internal cards remain; no picture frame is drawn.
12. `nothing_to_export` error text is exactly "This moodboard has nothing to export with the selected options."
13. Grayscale print: Internal chips and placeholders remain distinguishable by text.

## K. Known issues and unresolved questions

**Verified defects in the current output / source**
- Masthead image count ignores the Images option and the empty-card drop (§B.1 #1).
- Baseline masthead wraps to two lines (28 pt) and image rows are 3 + 1 (p.1).
- Frame placeholder text and note text use `pal::FAINT` (3.1:1), below the readable minimum.

**Design choices (mine, flagged)**
- 2 columns for 1–2 images (E.3); compact title; running header; chips for Internal; slate eyebrow.
- Frame ratio kept at 0.72 rather than introducing per-image ratios or a cover mode.

**Unverified assumptions / open questions**
- Whether the Rust renderer can add a running header per document (the design assumes a one-line, top-right text). If not, drop it and keep the footer text it already has.
- Whether canvas size (`w`,`h`) should scale print size (currently ignored by the builder).
- The baseline's date "22 Oct 2026" is the day the user exported; the design prints that value, the renderer prints today's date.
- The effect of the Mac moodboard export was not examined.
