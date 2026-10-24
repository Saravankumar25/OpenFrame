# Beat Board Card Board: Card Board layout (`layout=board`, portrait; v2, user decision)

| | |
|---|---|
| Export family | Beat Board (UI name; the Story workspace's board export). Operation `story.export_board` |
| Variant | Card Board layout (`layout=board`, portrait; v2, user decision) |
| Status | **Design reference. Not integrated; not validated against OpenFrame's Rust renderer.** |
| Repo commit | `2f5a045416bd2125801329533fc47f986ccc8174` (`main`, sparse checkout, read-only) |
| Date | 2026-10-23 |
| Deliverables | `Beat_Board_Card_Board.html` (the design), `Beat_Board_Card_Board.pdf` (rendered from it), `stress-test/Beat_Board_Card_Board_STRESS_TEST.html/.pdf` (everything on, project scope, 3 boards), `stress-test/Beat_Board_Card_Board_STRESS_TEST_B_options_off.html/.pdf` (notes, comments, Parking Lot all off) |
| Out of scope | The Story page (free-writing manuscript, `story_page.rs`, `story.page.export`) is a different export and is not covered. TXT and CSV outputs of this op are not redesigned (PDF only). |

### Changes from v1 (user decision: portrait)
v1 of this document was landscape (792 x 612 pt, 4 cards per row, 7 pages). **User decision: the Card Board is portrait US Letter.** What changed and what did not:
* **Changed**: `@page` size 8.5in x 11in (no landscape class); content width 700 pt to 520 pt; **cards per row 4 to 3** (card width 169 pt to 168 pt, gap 8 pt, same as v1); the continued strip, running header and all other components are unchanged in style. Pagination re-tuned: 7 pages to 6 (sample), 9 to 8 (stress). The renderer must also stop building this layout landscape (`story.rs`, `.landscape(board_layout)` and the two `columns: 4`), see B.
* **Not changed**: content, wording, order, numbering, included data, hierarchy (act bar, sequence band, boxed scene cards with number tile, beat cards, "Not in a sequence", Unassigned and Parking Lot areas, notes vs comments, count tiles, key/includes box, running header text, continued strips, episode bars).
* The Outline layout is untouched (already portrait).

## A. Purpose and users
The card board is the **pin-up / binder copy** of the Beat Board: each sequence is a set of cards in rows and each act a section, like the on-screen board. It is printed portrait (user decision, v2) so that it prints, punches, binds and files like every other OpenFrame document, and is read at arm's length or passed round a room.
* **Writer / story editor**: sees the shape of an act at a glance, which sequence has too many or too few cards, where the beats sit.
* **Director**: scans cards (slugline + one-line description), the act-level beats, and the notes and comments that need a decision, without reading a list.
* **Script supervisor / producer**: card numbers (board order), which cards belong to which sequence and act, parked ideas clearly set apart; a sequence that runs over a page must still be labelled on the next page.

Must find fast: act and sequence boundaries; card N; beat vs scene card; note vs comment; the Parking Lot.

### B. Verified functionality

**Entry point.** `StoryExportButton` (`windows/apps/desktop/src/features/export/buttons.tsx`) is mounted in the page-header actions of the Beat Board workspace (`windows/apps/desktop/src/workspaces/story/index.tsx`: `<StoryExportButton episodeId={episodeId} selectedIds={ui.selected.map(...)} />`, next to the chip "No scene numbers; they come from the screenplay"). It opens the shared `ExportDialog` (`features/export/ExportDialog.tsx`): title **Export Beat Board**, "Source: current Beat Board", then scope, format, options, native Save dialog. Dialog note: "Private notes excluded. Export creates a snapshot and does not change your project."

| UI control | Exact label | Arg key / value |
|---|---|---|
| Scope | "Current page (whole Beat Board)" | `scope:"board"`, `episodeId` (default) |
| Scope | "Selected items only" / "Selected items only (N)"; disabled when nothing is selected | `scope:"selected"`, `episodeId`, `itemIds` (selected acts, sequences, beats, cards; children of a selected container are included) |
| Scope | "Entire project" | `scope:"project"` (film board + one board per episode) |
| Format | "PDF", "Plain text", "CSV" | `format:"pdf"`, `"txt"`, `"csv"` (this design = PDF only) |
| Layout (PDF only) | "Outline PDF" (default), "Board PDF" | `layout:"outline"`, `"board"` |
| Checkbox | "Include Parking Lot" (default off) | `includeParking` |
| Checkbox | "Include notes" (default off) | `includeNotes` (act/sequence/beat/card notes; never private notes) |
| Checkbox | "Include comments" (default off) | `includeComments` |

Other facts (`windows/crates/openframe-application/src/modules/exports/story.rs`): args struct `ExportStoryOutlineArgs`; `layout:"board"` with a non-PDF format is rejected ("The board layout is a PDF. Choose Outline for plain text or CSV."); unknown layout: "Choose Board PDF or Outline PDF."; `scope:"selected"` with no ids: validation error on `itemIds` ("Select at least one act, sequence, beat or scene card to export."). Empty result: error `nothing_to_export` ("The Beat Board is empty, so there is nothing to export yet." / "None of the selected items are on the Beat Board any more. Select items and try again."). A board with no acts, no Unassigned items and no (included) parking items is dropped, so an empty episode prints nothing. Only active beats are exported (`state = 'active'`). Scene Card numbers are **board order** (acts, then Unassigned; the Parking Lot is never numbered) and are computed before scope filtering, so a "selected" export keeps the original numbers; every board (film, each episode) numbers from 1.

**Data dependencies.** Tables read: `story_act` (title, note, position), `story_sequence` (act_id, title, note, position), `story_beat` (text, note, position), `story_scene_card` (scene_heading, short_description, notes, position; parent_type act / sequence / unassigned / parking), `comment` (status, author_name, body, parent_id; targets story_act / story_sequence / story_beat / story_scene_card), project title, episode title. A sequence whose act no longer exists is exported under "Unassigned". Comment statuses (`comments/mod.rs`): "Open", "In Discussion", "Resolved".

**Layout specifics of this variant.** The current source builds this layout **landscape**: `story.rs` file header comment (""Board" is a landscape card grid per act"), the `layout` arg doc (""board" (landscape card grid; PDF only)") and `Document::new(document).footer(footer(&project, document)).landscape(board_layout)` (the builder passes `board_layout` to `.landscape(...)`, so `layout=board` is landscape and `layout=outline` portrait). **Required change for this design: `.landscape(false)` for both layouts, i.e. portrait for the board** (and the comments above updated). In the current source (`board_blocks`, `cell`): act = `Block::SectionBar` (uppercase, navy bar) followed by `Block::Note` lines for the act's notes/comments; sequence = `Block::Heading{level:2}` "Sequence: <title>" + `Block::Note`s; children go into `Block::Grid{columns:4, frame_ratio:0.5, cells}` (the literal `4` appears twice in `board_blocks`; **it must become 3**) where each `GridCell` has `title` ("Scene Card N" / "Beat"), `lines` (heading, description) and `notes` (note + comments); a run of beats/cards that are direct children of the act is collected into a grid of its own; an empty sequence prints "No beats or scene cards yet."; an empty act prints "No sequences, beats or scene cards yet."; Unassigned and Parking Lot are `SectionBar`s followed by a grid; in a multi-board export a `PageBreak` precedes every board after the first and each board starts with `Heading{level:1}`. Grid constants in `report.rs`: `GRID_GAP = 12`, `CAPTION_SIZE = 8.5`; no frame is drawn (the cells have no image or placeholder). `render_grid` computes `cell_w = (width - GRID_GAP*(cols-1))/cols`, so portrait with 3 columns at the current 12 pt gap would give 165.3 pt cards (this design: 168 pt with an 8 pt gap).

#### B.1 Current-output assessment (supplied `Beat Board — Card Board layout.pdf`, 5 pages, landscape; the v1 design was also landscape, 7 pages)
**Retain**
* Board order; card caption = "Scene Card N" / "Beat" title, heading, description, then notes; acts as bars; the same empty-state strings; rows of cards move whole to the next page; the real Low Tide Radio data.

**Improve visually**
* Cards have no boundary: four columns of loose text, so one card's note runs into the next card's heading (p1-p5). Notes are 8.5 pt mid-grey with a "Note:" prefix, identical in style to comments (p3, card 24).
* A sequence is a plain bold line, visually weaker than a card title; the acts are the only strong element; beats are text-only cells titled "Beat".
* No counts and no indication of the included options; the footer is the old vector-mark footer with different wording.

**Verified functional/layout problems (supplied PDF)**
* p2 bottom: the heading "Sequence: Storm Warning" and its note are alone at the bottom while its cards start on p3. p4 bottom: "Sequence: Low Tide" and its note are alone, the cards are on p5. (`Block::Heading` keeps a heading with a fixed 40 pt, `ensure(size*1.3 + 40)`, not with the grid it introduces.)
* p2, p3 and p5: act-level beats ("Beat" cells) are printed in a grid immediately after the last sequence's cards with no heading, so they look like members of the previous sequence ("Static", "Storm Warning", "Low Tide") although they belong to the act. In data order they can also sit between sequences (p3).
* p5: the Parking Lot shares a page with the end of Act Three and its section bar has the same style as an act bar, so parked cards look like another act.
* Source evidence, not visible in the supplied PDF: `render_grid` cuts an over-long caption to one page ("An over-long caption is cut to one page rather than overflowing it", `report.rs`; `keep = floor((limit - frame_space)/CAPTION_LINE)`), i.e. a very long note on a card is silently truncated on the board layout while the outline layout paginates the same text. **Requirement for the portrait implementation: card captions (heading, description, notes, comments, replies) must wrap whole; the portrait card is narrower than a page and its text longer in lines, so truncation must be removed, never relied on.** Text that does not fit the rest of a page continues on the next page (section H).
* A sequence that continues on the next page has no label (p2, p4, p5).
* Naming: see the naming section above (stale PDFs).

**Cannot be determined from the evidence**
* Output for "Selected items only", "Entire project", episodic projects, comment replies / other statuses, and an over-long card (none is in the supplied PDF). In a multi-board export the episode title is a 13 pt `Heading{level:1}` above `SectionBar` acts: weak separation, from source only.

### Naming: "Story Board" in the supplied PDFs vs "Beat Board" in the UI (verified)
* The metadata title of the supplied PDFs is `Story Board` (card board) and `Story Board Outline` (outline); the masthead, the panel (`Current Story Board`) and the footer text also say Story Board.
* At commit `2f5a045` **source and UI say "Beat Board" everywhere**: `story.rs` sets `document` = `"Beat Board"` / `"Beat Board Outline"`, `source_label` = `"Current Beat Board"` / `"Beat Board: <episode>"`, eyebrow `"Story · Beat Board"` / `"Story · Outline"`; `buttons.tsx` has title `Export Beat Board` and `documentName="Beat Board"`. A grep of `windows/crates` and `windows/apps/desktop/src` (the AI module folder excluded, not inspected) finds no remaining "Story Board" string.
* Git history confirms a deliberate rename: commit `349a3b7` "Rename the Story Board to Beat Board in everything a person sees" (2026-10-20); `story.rs` at `324cd96` (the audit-pack snapshot) still said "Story Board".
* Conclusion: this is **not** an inconsistency in current source. The supplied PDFs were produced by a build that pre-dates the rename (they are stale), although their date stamp says 22 Oct 2026. The designs use the current source names. Which build produced the PDFs cannot be determined from the evidence.

### C. Source mapping
* Commit `2f5a045416bd2125801329533fc47f986ccc8174`.
* UI: `StoryExportButton` (`features/export/buttons.tsx`), mounted in `workspaces/story/index.tsx` (page header). Dialog: `features/export/ExportDialog.tsx`.
* Op `story.export_board`; builder `pub(super) fn export` in `modules/exports/story.rs`; helpers `load_board`, `load_comments`, `number_cards`, `filter_selected`, `count`, `card_title`, `extras`, `outline_blocks`, `board_blocks`, `cell`.
* Structs: `ExportStoryOutlineArgs`, `Node { kind: Act|Sequence|Beat|Card, title, text, note, comments, children, number }`, `Board { episode, acts, unassigned, parking }`.
* Document: `Document::new(document).footer(footer(&project, document)).landscape(board_layout)`; masthead `Masthead::new(document).eyebrow(..., pal::ORANGE)` with `subtitle` = project, `lines` = source label, `side = Side::Panel { lines: [(scope_label, true), ("Exported <date>", false)] }`, `rule = true`.
* Blocks used today by this variant: `Masthead`, `Block::SectionBar`, `Block::Note`, `Block::Heading`, `Block::Grid` (`GridCell`), `Block::PageBreak`. **The redesign cannot be built from these**; it needs new blocks (section F, "Renderer requirements").
* Renderer: `windows/crates/openframe-import-export/src/report.rs` (`Block`, `Grid`/`GridCell`, `render_grid`, the `Heading`/`Indented`/`SectionBar` arms) and `report/design.rs`.
* Not verified: the `mac/` tree (not inspected).

### D. Field inventory

| Visible element | Source field / struct | Kind | Empty-value treatment |
|---|---|---|---|
| Document title | `document` ("Beat Board") | constant | n/a |
| Eyebrow | "Story · Beat Board" | constant | n/a |
| Project name (masthead subtitle) | `project_title(c)` via `nb(&project)` | optional | omitted when blank |
| Source line | `source_label`: "Current Beat Board" or "Beat Board: <episode title>" | derived | n/a |
| Scope panel line 1 | `scope_label`: "Current Beat Board" / "Selected items only" / "Entire project" | derived | n/a |
| Scope panel line 2 | "Exported <d Mon yyyy>" (`today_label()`) | derived (clock) | n/a |
| Scope panel line 3 (design addition) | "Acts and sequences shown for context", only when scope = selected | derived | omitted otherwise |
| Count tiles (design addition) | counts of acts, sequences, scene cards, beats (as in `Counts`, split on-board / Parking Lot) | derived | a tile whose total and parked counts are both 0 is omitted |
| Key / Includes box (design addition) | the three option flags | derived | chips only for enabled options; "Parking Lot (none)" when included but empty; "Board only" when all off |
| Episode bar | `Board.episode` title, or "Film Beat Board" | conditional (only when more than one board is exported) | n/a |
| Act bar | `story_act.title` | repeatable | n/a |
| Act / sequence / beat / card note | `story_act.note`, `story_sequence.note`, `story_beat.note`, `story_scene_card.notes` | optional; conditional on `includeNotes` | omitted (no label, no gap) |
| Comment thread | `comment`: `status`, `author_name`, `body`, replies (`parent_id`) | optional, repeatable; conditional on `includeComments` | omitted |
| Sequence band | `story_sequence.title` | repeatable | no children: "No beats or scene cards yet." (source string, board layout) |
| Empty act | no children | conditional | "No sequences, beats or scene cards yet." (source string, board layout) |
| Card number | `Node.number` (board order) | derived; absent in the Parking Lot | no number: tile text "Card" (outline) / header without tile (board) |
| Card heading | `story_scene_card.scene_heading` | optional | omitted |
| Card description | `story_scene_card.short_description` | optional | omitted |
| Card with neither | | conditional | "No heading or description" (design addition; the current export prints a bare "Scene Card") |
| Beat text | `story_beat.text` | required | n/a |
| Unassigned area | `Board.unassigned` (items and sequences with no act) | conditional | whole area omitted when empty |
| Parking Lot area | `Board.parking` | conditional: `includeParking` and non-empty | whole area omitted |
| Page furniture | footer (shared system), running header | constant | n/a |

Colour labels on beats/cards (`clean_color`) and attachments are **not** exported today and are not designed.

### E. Content architecture
Order (unchanged): masthead > (addition) count tiles + key/includes box > per board: [episode bar if more than one board; every board after the first starts on a new page, as `PageBreak` does today] > acts in board order > Unassigned > Parking Lot (only if included and non-empty).

Per act: **act bar** (navy; title in caps; hint "N sequences · N scene cards · N beats"), act notes/comments below the bar, then its children in **data order**: sequences and runs of loose beats/cards.
* **Sequence lane** = tint band with an orange left stripe (label "Sequence", title, hint "N scene cards · N beats"; sequence notes/comments inside the band) followed by rows of cards.
* **Loose beats/cards directly under the act**: grouped into their own rows, introduced by the label **"Not in a sequence"** when the act also has sequences (changed from the current export, where they float after the previous sequence's grid).
* **Scene card** = rounded box: header strip with the number tile + "SCENE CARD" label, then heading (bold), description, note (label NOTE + text), comment boxes. Unnumbered (Parking Lot) cards: header "SCENE CARD" without a tile. **Beat card** = the same box in soft orange, header "BEAT", beat text in bold. Cards in a row stretch to the tallest card.
* **Unassigned**: outlined area bar "Unassigned" ("Not placed in an act"). **Parking Lot**: soft-orange area bar ("Held outside the board order · not numbered"), cards with orange-line borders and header tint. Always last.
* **Continued strip** at the top of a page that starts inside a container: `ACT TWO: THE REEL › SEQUENCE: STORM WARNING · continued`.
* Empty act / empty sequence: the source sentences. Card with no heading and no description: "No heading or description" (addition).

**Column rule (portrait): 3 cards per row.** Every row (sequence rows, "Not in a sequence" rows, Unassigned rows, Parking Lot rows) is chunked three at a time in data order; card width = (520 - 2 x 8) / 3 = **168 pt**, column gap 8 pt, row gap 8 pt, text width inside a card 155 pt (padding 6.4 pt each side). A last row with fewer than three cards stays left-aligned with empty columns (no placeholders, no stretching). Beats and scene cards share the same grid and are chunked together as one run.

**Why 3 and not 4 (evidence).** The 4 x 125.5 pt alternative (gap 6 pt) was built and viewed on the same data (`BB_COLS=4` in the generator):
* It does stay legible on the real Low Tide Radio data and needs 5 pages instead of 6.
* But a 125 pt card has 112 pt of text width: sluglines wrap to 2-3 lines ("INT. THOMAS HOUSE - APPACHAN'S ROOM - NIGHT"), every "Story day: ... Characters: ..." note takes 3 lines; the comment box (2.5 pt rule + padding) leaves about 100 pt, about 20 characters a line; unbreakable strings break every ~18 characters.
* Stress case: the tall card (3-paragraph note, 2 comments, 2 replies) is about 620 pt high in 4 columns (of 680 pt usable) against about 450 pt in 3 columns, so with 4 columns a realistic worst case already approaches the point where a row no longer fits on a page.
* Six-card sequences are common in the sample (Static, Seven Days, What Joe Did, Listening Night): with 3 columns they are two full rows; with 4 columns a row of 4 plus an orphan row of 2.
* A 168 pt card is practically the v1 landscape card width (169 pt), so the heading/note wrapping tuned in v1 carries over unchanged.
A reader who wants 4 across can still use the landscape v1, which is kept in the original package.

Changes versus the current output, and why: boxed cards (card boundaries); stacked heading/description; number tile; beats visually different from cards; lanes (a sequence must be stronger than a card); "Not in a sequence" (hierarchy); heading + first row kept together (the orphaned headings on p2/p4); the Parking Lot as a separate-looking area; notes vs comments; counts/key/includes; continued strips + running header; **portrait, 3 columns** (user decision, v2). Not changed: order, numbering, strings, scope behaviour, a new page per extra board.

### F. Visual specification
* **Page**: US Letter **portrait** 612 x 792 pt (`@page { size: 8.5in 11in }`, no landscape class); margins L/R/T 46 pt, bottom 66 pt; content width 520 pt, usable height 680 pt.
* **Fonts**: Helvetica regular and bold only.
* **Shared with the outline variant** (pt values in `Beat_Board_Outline.md`, section F): masthead (title "Beat Board" 28 pt, eyebrow "Story · Beat Board"), scope panel, four count tiles, key / includes box, act bar (10.5 pt bold white caps on navy), sequence band (3 pt orange stripe, tint fill), note (8 pt, label 6.6 pt), comment box (status chip, author, replies), Parking Lot bar, Unassigned bar, episode bar, continued strip. Inside a card the note is 7.6 pt with the label inline and the comment body is 7.6 pt.
* **Card row**: grid of **3** equal columns (`repeat(3, minmax(0, 1fr))`), 8 pt gap, 8 pt below each row, 6 pt above the first row after a band; cards stretch to the row height.
* **Scene card box** (168 pt wide): white fill, 0.5 pt `--of-line-strong` border, radius 6, content clipped to the rounded box. Header strip: `--of-tint` fill, 0.5 pt bottom rule, padding 3.2/6, label 6.8 pt bold caps tracked .1em navy; number tile min-width 20 pt, 7.6 pt bold white on navy, radius 3. Body padding 4.6/6.4/5.4. Heading 8.4 pt bold navy (lh 1.25); description 8.4 pt ink (lh 1.3). **All caption text wraps whole inside the card; nothing is cut or ellipsised (this replaces the truncation in `render_grid`).**
* **Beat card box**: `--of-orange-soft` fill, 0.5 pt `--of-orange-line` border, header strip `#ffe3c2` with `--of-orange-deep` text "BEAT", beat text 8.6 pt bold navy.
* **Parking Lot cards**: scene cards get an `--of-orange-line` border and an `--of-orange-soft` header; beat cards unchanged.
* **Grayscale redundancy**: beat = header text "BEAT" + fill; scene card = number tile; comment status = chip text; Parking Lot = label + bar.
* **Footer**: shared system (logo lockup bottom-left, brand line, "Page X of Y"). **Implementation requirement**: embed the footer logo PNG (the current renderer draws a vector mark and prints "LOW TIDE RADIO · STORY BOARD · Page 1 / 5").
* **Running header: yes.** `LOW TIDE RADIO · BEAT BOARD`, top-right, 6.8 pt slate, no rule. Reason: loose printed pages need the project on each page; the masthead names it on page 1 only.
* **Renderer requirements (new)**: boxed card with a header strip and clipped rounded corners, tiles, chips, the note rule, the comment box, the lane band, area bars, the continued strip, row stretch, keep-together leads; structured comment data (status, author, body, replies) instead of the pre-formatted strings; **portrait board document with 3 columns and a whole-caption wrap** (see B). The existing `Grid` supports captions only and truncates a caption at one page, so it is not usable as is.

### G. Dynamic behaviour
* **Notes / comments off**: NOTE and COMMENT parts are removed with no gap (cards shrink); the key legend entries and the Includes chips follow; "Board only" when none (`stress-test/Beat_Board_Card_Board_STRESS_TEST_B_options_off`).
* **Parking Lot off or empty**: omitted entirely; included but empty: the "Parking Lot (none)" chip. Unassigned is shown whenever non-empty.
* **Long text**: heading, description, notes and comments wrap in the 155 pt text width; unbroken strings break by character; a long title makes the row taller and the other cards in the row stretch. Stress PDF p4: a 190-character slugline in a card; p5: the tall card (3-paragraph note, 2 comments, 2 replies; about 450 pt high). Nothing is truncated.
* **A row taller than a page**: by rule the row splits between its parts (note / comment boundary) and the continuation page gets the continued strip; the current renderer would cut the caption instead. Not reached by the stress data (the tallest row is about 450 pt of 680 pt); the rule is untested.
* **Many cards**: a sequence with 26 cards flows over 3 pages in rows of 3 (stress PDF p1-p3); each continuation page has the strip with act and sequence.
* **Last row**: fewer than 3 cards leave empty columns (no placeholders).
* **Empty act / sequence**: the source sentences. **Card with nothing**: "No heading or description".
* **Multi-board**: each board after the first starts a new page with an episode bar; empty boards are dropped; numbers restart at 1 per board.
* **Selected items only**: ancestors shown as context (third panel line), original numbers kept.

### H. Pagination and print rules
* US Letter portrait; margins 46/46/46/66 pt; footer in the bottom margin box; true "Page X of Y".
* **Rows move whole** (as today). A card never splits unless its row is taller than a page.
* **Keep-with-next ("lead")**: act bar + act notes + first sequence band + first row are one unit; sequence band + first row; "Not in a sequence" label + first row; area bar + first row; episode bar + the act lead that follows. A unit taller than a page may break after its heading block. Minimum before a break: a band is never the last thing on a page; at least one full card row follows it.
* A continued strip is the first line of any page that starts inside a container (act, sequence or episode), not of a page that starts with the container's own heading.
* Each additional board starts on a new page (current behaviour); acts do not start new pages.
* Running header on all pages; content never enters the header or footer band.
* **Sample result (Chrome, 6 pages vs 7 in v1), rows per page** (scene-card rows + beat rows): p1 2 + 0 (act one, band "Static", cards 1-6); p2 3 + 1 (cards 7-12, beat row, act two, band "Oxide", cards 13-15); p3 4 + 0 (16-26, strip "Act Two › Sequence: Oxide"); p4 3 + 1 (27-28, beat row, band "What Joe Did", 29-34); p5 4 + 1 (act three, 35-44, beat row); p6 1 + 1 (Parking Lot). Fill: p1 78 %, p2 85 %, p3 84 %, p4 75 %, p5 97 %, p6 27 %. p1 is short because the next lead (band "Seven Days" + a 150 pt row) did not fit; p6 holds the Parking Lot alone because p5 is full (it stays whole and visibly separate).
* Stress PDF (8 pages vs 9 in v1): fill p1 80 %, p2 98 %, p3 25 %, p4 81 %, p5 74 %, p6 92 %, p7 62 %, p8 34 %. p3 is short because the lead of the long-title act (act bar + note + comment + band + comments + first row) does not fit after the end of the 26-card sequence.

### I. HTML/PDF correspondence
* `Beat_Board_Card_Board.html` is the design; the PDF is its Chrome 155 headless rendering (`tools/render_pdf.py`), generated by `tools/build/beat-board/build.py board` (v2 copy) from the same data model as the outline (`data.py`: Low Tide Radio transcribed from the supplied PDF). `BB_COLS=4` rebuilds the rejected 4-column variant for comparison (not shipped).
* **Captured state**: scope "Current page (whole Beat Board)", layout Board, notes ON, comments ON, Parking Lot ON (the same content as the supplied PDF). 6 pages, portrait.
* **Toggles**: `Parking Lot`, `Notes`, `Comments`, `Scope: Selected items only (example)`, `Scope: Entire project, several boards (fictional example)` (these two replace the main body; the fictional episode is labelled "Illustrative (fictional)").
* **Pagination pins**: the same mechanism as the outline (probe render, forced breaks and continued strips for the captured state; disabled automatically when a toggle changes).
* **Differences from the Rust renderer**: text metrics, line breaking and fragmentation (Chrome vs `report.rs`); row stretch and clipped rounded corners need new drawing code; page breaks will move.
* **Stress tests**: `stress-test/Beat_Board_Card_Board_STRESS_TEST.pdf` (8 pages, fictional "Harbour Lights", portrait, project scope, 3 boards of which the empty one is dropped; the second board starts on a new page; 190-character slugline, 200-character act title, 130-character sequence title, 90-character unbroken strings, 3-paragraph note, comment threads with 3 statuses and replies, cards with missing parts, empty sequence, empty act, a 26-card sequence over 3 pages, Unassigned, Parking Lot) and `..._STRESS_TEST_B_options_off.pdf` (2 pages, notes/comments/Parking Lot off).
* QA performed: `tools/verify_pdf.py --size letter` with the running-header text on all primary and stress PDFs (passes); the primary PDF (all 6 pages, contact sheet, p2 and p3 at 100 dpi) and the stress PDF (all 8 pages, contact sheet) viewed; the 4-column experiment viewed (p2, p3 and a stress page); grayscale thumbnails generated and p3 viewed; the "Entire project" variant printed and viewed (3 pages); the "Selected items only" variant was printed but not viewed in portrait. Outline files compared byte-for-byte with v1: identical.

### J. Acceptance criteria
1. PDF is US Letter **portrait** (612 x 792 pt), metadata title "Beat Board"; footer on every page (logo 17 pt high, brand line, "Page X of Y" with the true total); `tools/verify_pdf.py --size letter` passes.
2. Running header "<PROJECT> · BEAT BOARD" on every page and nothing else in the top margin.
3. Exactly 3 cards per row (the last row of a run may have fewer, left-aligned), each 168 pt wide with an 8 pt gap; cards in a row have equal height; no card text outside its box; **no truncation of any text (a note longer than the page continues on the next page)**.
4. Cards appear in board order; numbers equal `Node.number`; Parking Lot cards are unnumbered; numbers are unchanged under scope "selected".
5. Act-level beats/cards are never inside a sequence lane; the "Not in a sequence" label precedes them when the act also has sequences.
6. A sequence band, act bar, area bar or label is never the last element of a page; a row is never split unless taller than a page.
7. Beat cards differ from scene cards by header text and fill; notes (NOTE label + rule) differ from comments (box + status chip + author) without colour.
8. The Parking Lot, if present, is the last area, labelled and styled as a separate area; Unassigned precedes it.
9. Pages that start inside an act or sequence begin with the right continued strip; none when the page begins with the container heading.
10. With notes, comments and Parking Lot off: nothing printed for them, no gap; the Includes line says "Board only".
11. Empty act and empty sequence print the source sentences; a card with no heading and no description prints "No heading or description".
12. Every extra board starts a new page with an episode bar; empty boards are not printed.
13. Grayscale rendering keeps beat / scene / status distinguishable by text and shape.
14. `story.rs` builds the board document portrait (no `.landscape(true)` for `layout=board`), and `Grid.columns` is 3.

### K. Known issues and unresolved questions
**Verified defects in the current output**: see B.1 (page citations).

**Design choices (deliberate, they change the current output)**
* Added: count tiles, Key/Includes box, continued strips, running header, "Not in a sequence" label, area-bar subtitles ("Not placed in an act", "Held outside the board order · not numbered"), "No heading or description" for an empty card, the third scope-panel line for "selected". All are derived from existing data or args; none adds a field or an option, and each can be dropped without affecting the rest.
* Card heading and description are stacked instead of joined by ": ".
* Page count grows (sample: 5 pages to 6) because of the added furniture and more generous spacing; the last page (p6) holds the Parking Lot.
* Beats are always drawn on a soft orange fill. The UI draws beats with dashed outlines and the Parking Lot hatched (`workspaces/story/BoardView.tsx`, file header comment); this design uses solid strokes only (dashed strokes and hatching were not verified as available primitives).

**Unverified assumptions**
* That the Rust renderer can implement every primitive used (rounded rects with a separate left rule, chips, a per-page repeated strip, row stretch). Not tested.
* That "Scene Card N" (board order) is the number users expect. The UI says scene numbers come from the screenplay, so the key states that card numbers are board order, not screenplay scene numbers.
* Page fill of the sample (p1:78 p2:85 p3:84 p4:75 p5:97 p6:27); the low pages are explained in section H.
* Chrome's pagination of the main file is pinned by the generator; the Rust renderer will paginate differently.
* Whether comments should be shown with their status chip; the status values come from `comments/mod.rs` but only "Open" occurs in the supplied PDFs.
