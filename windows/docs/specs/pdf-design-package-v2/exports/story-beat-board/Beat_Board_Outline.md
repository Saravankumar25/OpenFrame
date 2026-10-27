# Beat Board Outline: Outline layout (`layout=outline`, portrait)

| | |
|---|---|
| Export family | Beat Board (UI name; the Story workspace's board export). Operation `story.export_board` |
| Variant | Outline layout (`layout=outline`, portrait) |
| Status | **Design reference. Not integrated; not validated against OpenFrame's Rust renderer.** |
| Repo commit | `2f5a045416bd2125801329533fc47f986ccc8174` (`main`, sparse checkout, read-only) |
| Date | 2026-10-23 |
| Deliverables | `Beat_Board_Outline.html` (the design), `Beat_Board_Outline.pdf` (rendered from it), `stress-test/Beat_Board_Outline_STRESS_TEST.html/.pdf` (everything on, project scope, 3 boards), `stress-test/Beat_Board_Outline_STRESS_TEST_B_options_off.html/.pdf` (notes, comments, Parking Lot all off) |
| Out of scope | The Story page (free-writing manuscript, `story_page.rs`, `story.page.export`) is a different export and is not covered. TXT and CSV outputs of this op are not redesigned (PDF only). |

## A. Purpose and users
The outline is the **reading and review copy** of the Beat Board: the story as a vertical, scannable hierarchy (act > sequence > beat / scene card) in board order.
* **Writer / story editor**: reads the structure top to bottom, checks that each sequence earns its place, sees each note next to the card it belongs to.
* **Director**: needs the shape of each act, the act-level beats (midpoint, low point, climax) and the open comments that need a decision.
* **Script supervisor / producer**: needs card numbers (board order), where an act or sequence starts, and a stable printout in which a page that begins mid-sequence still says which act and sequence it is in. Parked ideas must never be mistaken for scenes in the film.

Must find fast: which act/sequence am I in; which card is number N; which items are story beats rather than scenes; what is a note (the item's own text) and what is a comment (a discussion with a status); what is parked.

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

**Layout specifics of this variant.** Portrait US Letter. In the current source the outline is built by `outline_blocks`: act = `Block::Heading{level:1}` (13 pt); sequence = `Block::Heading{level:2}` (11 pt) with the literal prefix "Sequence: "; beat = `Block::Indented` "Beat: <text>"; card = `Block::Indented` "Scene Card N · <heading>: <description>" (or "Scene Card: <description>" when there is no number and no heading); notes = muted `Block::Indented` "Note: ..."; comments = "Comment (<status>) from <author>: <body>" and "Reply from <author>: <body>" (pre-formatted strings from `load_comments`); "Unassigned" and "Parking Lot" = `Block::Heading{level:1}`. In a multi-board export each board gets `Block::Heading{level:1}` = episode title or "Film Beat Board" (same level as acts); outline boards are not separated by a page break.

#### B.1 Current-output assessment (supplied `Beat Board — Outline layout.pdf`, 5 pages, portrait)
**Retain**
* Content, order and grouping: board order, acts > sequences > cards/beats, notes under their item, Parking Lot last, the same numbering ("Scene Card N" in board order, none in the Parking Lot), the same note/comment wording and empty-state strings.
* The real data (Low Tide Radio: 3 acts, 7 sequences, 44 numbered cards, 6 on-board beats, 3 parked cards + 1 parked beat, one comment on card 24) is used verbatim in the redesign.

**Improve visually**
* Act, sequence, beat, card and note are plain text lines that differ only by size or a text prefix; the card number is buried in a run-on line (p1-p4).
* Heading and description are joined into one sentence ("... - NIGHT (1999): Prologue. A storm...") so the slugline does not scan as a title.
* Notes and comments share the same muted 8.5 pt grey; they differ only by the words "Note:" / "Comment (Open) from" (p3, card 24).
* "Current Story Board" is printed twice in the masthead (left line and panel); no counts; the reader cannot tell whether notes, comments or the Parking Lot were included.

**Verified functional/layout problems (supplied PDF)**
* p1 to p2: card 11's description is split across the page break ("... Anjali and Rahul let" / "themselves in ...") and its Note is carried to p2.
* p4 to p5: the Parking Lot item "Scene Card: Alternative opening" is at the bottom of p4 while its Note and the next item "Beat: Alternative opening" are on p5, a page that holds two lines.
* p2 and p3: act-level beats ("Beat: Tone: ...", "Beat: Midpoint: ...", "Beat: Climax: ...") sit at the same indent as the sequence headings, directly after the last card of the previous sequence, so they read as part of it or as a new heading level; the hierarchy is ambiguous. They appear in data order (p3 shows them between two sequences).
* p2, p3, p4: a page that starts mid-sequence does not say which act or sequence it belongs to.
* An empty act or sequence prints only its heading in the outline (the board layout prints an explicit sentence for the same case); source evidence, not in the supplied PDF.
* Source evidence (not observable in the supplied single-board PDF): in a multi-board export the episode title and every act title are both `Heading{level:1}` at 13 pt, so an episode cannot be told from its acts.
* Naming: see the naming section above (stale PDFs).

**Cannot be determined from the evidence**
* Output for scope "Selected items only" or "Entire project", for episodic projects, for comment replies and the "In Discussion" / "Resolved" statuses, and for very long notes (none occurs in the supplied PDF; the behaviour above comes from source only).

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
* Blocks used today by this variant: `Masthead`, `Block::Heading`, `Block::Indented`. **The redesign cannot be built from these**; it needs new blocks (section F, "Renderer requirements").
* Renderer: `windows/crates/openframe-import-export/src/report.rs` (`Block`, `Grid`/`GridCell`, `render_grid`, the `Heading`/`Indented`/`SectionBar` arms) and `report/design.rs`.
* Not verified: the `mac/` tree (not inspected).

### D. Field inventory

| Visible element | Source field / struct | Kind | Empty-value treatment |
|---|---|---|---|
| Document title | `document` ("Beat Board Outline") | constant | n/a |
| Eyebrow | "Story · Outline" | constant | n/a |
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
Order (unchanged from source): masthead > (design addition) count tiles + key/includes box > for each board: [episode bar, only if more than one board] > acts in board order > Unassigned (if any) > Parking Lot (only if `includeParking` and non-empty).

Hierarchy (made explicit):
1. **Act bar**: full-width navy bar; hint at right "N sequences · N scene cards · N beats" (derived, zero parts omitted). Act note and comments directly below, on white.
2. **Sequence band**: tint band with an orange left stripe, label "Sequence" + title, hint at right "N scene cards · N beats"; the sequence note and comments sit inside the band.
3. **Items under a sequence** are indented 10 pt with a 1.2 pt vertical rule at the left. **Items directly under an act** (beats/cards not in a sequence) are flush left and, when the act also has sequences, introduced by the label **"Not in a sequence"**. Data order is preserved: act-level beats stay where `position` puts them (after the last sequence, or between sequences as in Act Two of the sample).
4. **Item row** = [28 pt tile column | text]. Scene Card tile = navy rounded rect with the board-order number. Unnumbered card (Parking Lot) = outlined tile "Card". Beat = outlined orange tile "Beat" and the whole beat row on a soft orange fill. Card text: heading (bold navy), then the description on its own line (the current export joins them with ": "). Beat text in bold.
5. **Note** (the item's own field): label NOTE + text, 1.5 pt grey rule at the left, no box. **Comment** (a discussion): bordered white box with a 2.5 pt teal left rule, label COMMENT, status chip (Open = amber, In Discussion = blue, Resolved = green, always with the text), author in bold, body; replies nested with label REPLY. Notes come first, then comments (as in `extras`).
6. **Unassigned**: outlined area bar (white, navy 1.2 pt border), hint "Not placed in an act". **Parking Lot**: soft-orange area bar with a 5 pt orange stripe, hint "Held outside the board order · not numbered"; always last and visibly unlike an act so a parked idea is never read as a scene.
7. **Episode bar** (multi-board only): eyebrow "Episode", 15 pt navy title, 3 pt orange top rule and navy bottom rule (film board: "Film Beat Board", no eyebrow); one level above act bars.
8. **Continued strip** (new): the first line of every page that starts inside a container: `ACT TWO: THE REEL › SEQUENCE: OXIDE · continued` (ancestors only, episode first when multi-board). Not printed when the page begins with the act bar itself.
9. Empty act: "No sequences, beats or scene cards yet."; empty sequence: "No beats or scene cards yet." (source strings, now also printed in the outline).

Changes versus the current output, and why: tiles, bands and indentation (the hierarchy was ambiguous); heading and description split (the slugline is a title); note vs comment styled differently (a working note must not be confused with an open question); "Not in a sequence" label (act-level beats looked like members of the previous sequence); episode bar (it was the same level as acts); count tiles + key + includes (a printed copy must say what it contains; counts for supervisors); Parking Lot and Unassigned styled as separate areas; running header + continued strips (loose pages); empty-state sentences (parity with the board layout). Not changed: all text strings, order, numbering, what is included.

### F. Visual specification
* **Page**: US Letter portrait 612 x 792 pt; margins L/R/T 46 pt, bottom 66 pt (footer zone); content width 520 pt.
* **Fonts**: Helvetica regular and bold only; no italics (the "· continued" suffix is regular weight).
* **Masthead**: eyebrow 7.6 pt bold tracked .14em slate ("Story · Outline"); title 28 pt bold navy ("Beat Board Outline"); project 13 pt slate; source line 8.4 pt slate. Scope panel at the right: `--of-panel` fill, 0.5 pt `--of-line` border, radius 6, padding 8/12/9, right-aligned, 9.6 pt bold navy line + 8.2 pt slate lines.
* **Count tiles**: four equal tiles (fill `--of-panel`, 0.5 pt line, radius 4, padding 4/8/5); label 6.8 pt bold caps; value 13 pt bold navy; sub line 7 pt ("+ N in Parking Lot").
* **Key / Includes box**: 0.5 pt line border, radius 4, 7.6 pt slate; headings KEY / INCLUDES 6.6 pt bold caps navy; chips = shared `.chip` (6.6 pt floor).
* **Act bar**: navy fill, radius 3, padding 5.5/9/6, 10.5 pt bold white caps tracked .06em; hint 7.4 pt `--of-dark-muted`; margin 14 pt above (first act 8 pt), 4 pt below.
* **Sequence band**: `--of-tint` fill, 3 pt `--of-orange` left stripe, radius 3 on the right, padding 4.6/8/5, margin 9 pt above / 3 pt below; label 6.6 pt bold caps slate, title 9.8 pt bold navy, hint 7.4 pt slate.
* **Item row**: grid 28 pt + 8 pt gap; padding 5 pt top, 5.6 pt bottom; 0.5 pt `--of-line` rule between rows. Card heading 8.8 pt bold navy (lh 1.25); description 8.8 pt ink (lh 1.3); beat text 9 pt bold navy. Card tile: min-width 26 pt, navy fill, 8.4 pt bold white tabular number, radius 3. Beat / unnumbered tiles: white fill, 0.5 pt line (orange-line / navy), 6.6 pt bold caps text. Beat row: `--of-orange-soft` fill, 0.5 pt `--of-orange-line` border, radius 3, padding 5/6, margin 3 pt 0.
* **Note**: 8 pt slate, lh 1.3, label 6.6 pt bold caps (24 pt column + 5 pt gap), 1.5 pt `--of-line-strong` left rule, margin-top 2.6 pt; line breaks in the stored text are kept.
* **Comment**: white fill, 0.5 pt `--of-line-strong` border + 2.5 pt `--of-teal` left rule, radius 3, padding 3.6/6/4.2; author 7.8 pt bold navy; body 8 pt ink; reply indent 7 pt with a 0.5 pt rule. Status chip: Open = `chip warn`, In Discussion = `chip info`, Resolved = `chip ok`.
* **Parking Lot bar**: `--of-orange-soft` fill, 1 pt `--of-orange-line` border, 5 pt `--of-orange` left stripe, `--of-orange-deep` 10.5 pt bold caps text, 22 pt above. **Unassigned bar**: white, 1.2 pt navy border, navy text.
* **Grayscale redundancy**: Beat vs Card is carried by the tile text ("Beat" / "Card" / number) and the filled row, not by colour; comment status by the chip text; the Parking Lot by its label and bar. Checked in grayscale (section I).
* **Footer**: shared system, unchanged (logo lockup 17 pt high bottom-left with "Created using OpenFrame" below, "Page X of Y" bottom-right). **Implementation requirement**: the production renderer must embed the footer logo PNG; today's renderer redraws a vector mark with different text ("LOW TIDE RADIO · STORY BOARD OUTLINE · Page 1 / 5").
* **Running header: yes.** `LOW TIDE RADIO · BEAT BOARD OUTLINE` (project in caps · document name), top-right, 6.8 pt slate, no rule, no logo. Reason: a multi-page outline is often split on a desk and the masthead (page 1 only) is the only other place that names the project. Omit the project part when the project title is blank.
* **Renderer requirements (new)**: the current `Block` set (Heading, Indented, Note, SectionBar, Grid) cannot express an act bar with a hint, a sequence band, tile + text item rows, the note rule, the comment box with a chip, area bars, the continued strip or keep-together leads. They need new blocks or new `design.rs` blocks (composition of the existing `Section` / `Cards` blocks was not evaluated). Comments must be passed as structured data (status, author, body, replies) instead of the pre-formatted strings from `load_comments` (TXT/CSV can keep using the strings).

### G. Dynamic behaviour
* **Notes / comments off**: the NOTE / COMMENT parts disappear with no label or gap; the key hides the matching legend entries; the Includes chips list only what is on ("Board only" when none). Shown by the `Notes` / `Comments` toggles and by `stress-test/Beat_Board_Outline_STRESS_TEST_B_options_off`.
* **Parking Lot off or empty**: the area is omitted entirely (no gap, no placeholder). Included but empty: the chip reads "Parking Lot (none)". The "+ N in Parking Lot" sub line appears only when parked items exist and the option is on.
* **Unassigned** appears whenever it has content, regardless of `includeParking` (source behaviour).
* **Long text**: titles, headings, descriptions, notes and comments wrap and rows grow; nothing is ellipsised; unbroken strings break by character (stress test). The act-bar title wraps in the left cell, the hint stays at the right (max 40 % width). Multi-paragraph notes keep their line breaks.
* **Card with missing parts**: heading only, description only, or "No heading or description" (design addition).
* **Empty act / empty sequence**: the source sentences above.
* **Many cards**: an act or sequence with many cards flows over pages (stress test: 26 cards in one sequence); the continued strip names the container on every page.
* **Multi-board** (scope "Entire project"): an episode bar before each board's first act; empty boards are dropped; numbering restarts at 1 per board.
* **Scope "Selected items only"**: ancestors of the selected items are shown (with their own notes) as context, the descendants of a selected container are all shown, numbers are the original board numbers; the panel says so (`Scope: Selected items only (example)` variant).

### H. Pagination and print rules
* US Letter portrait; margins 46/46/46/66 pt; footer in the bottom margin box on every page; "Page X of Y" uses the true total.
* Item rows and comment boxes never split; a single row taller than a page may split between its parts (note / comment).
* **Keep-with-next ("lead")**: act bar + act notes/comments + first sequence band (+ its notes/comments) + first item row stay together; likewise sequence band + first item, "Not in a sequence" label + first item, area bar + first item, episode bar + the act lead that follows. A lead taller than a page may break after the heading block. A heading, label or band is never the last thing on a page.
* A page that begins inside a container starts with the continued strip (not when it begins with the container's own heading).
* No forced break between acts (continuous flow, as today); the Parking Lot and Unassigned are not forced onto a new page.
* Running header on every page; no content may enter the header or footer band.
* Sample result (Chrome, 6 pages): p4 is 79 % full because the next lead (act bar + note + first band + first row, about 190 pt) does not fit; p6 holds the Parking Lot alone (27 %) because its lead did not fit on p5. Both are the price of never stranding a heading.

### I. HTML/PDF correspondence
* `Beat_Board_Outline.html` is the design; `Beat_Board_Outline.pdf` is its Chrome 155 headless rendering (`tools/render_pdf.py`, margin-box footer and page counters). Both are generated by `tools/build/beat-board/build.py` from one data model (`data.py`: Low Tide Radio transcribed from the supplied PDF; nothing is invented in the primary files).
* **Captured state**: scope "Current page (whole Beat Board)", layout Outline, notes ON, comments ON, Parking Lot ON (the same content as the supplied PDF). 6 pages.
* **Toggles** (preview toolbar, `data-toggle`): `Parking Lot`, `Notes`, `Comments`, `Scope: Selected items only (example)` (cards 24 and 28 and the beat "Low point", acts and sequences as context), `Scope: Entire project, several boards (fictional example)` (film board + a fictional episode, labelled "Illustrative (fictional)"). The two scope variants replace the main body.
* **Pagination pins**: the "continued" strips and the page starts of the captured state are measured and pinned by the generator (probe render with invisible sentinels, then one forced break per page start). When any toggle or variant changes, the preview script hides the strips and disables the pins, because they are valid only for the captured state. In production the renderer computes them.
* **Known differences from the Rust renderer**: Chrome's text metrics, line breaking and fragmentation differ from `pdf-writer` with the Helvetica metrics in `report.rs`, so page breaks will move; chips, rounded rects and left-rule primitives need to be implemented; the left rule of an indented list runs to the bottom of the page when the list continues (a Chrome artifact).
* **Stress tests**: `stress-test/Beat_Board_Outline_STRESS_TEST.pdf` (6 pages; fictional "Harbour Lights"; project scope with 3 boards of which the empty one is dropped; 200-character act title, 130-character sequence title, 190-character slugline, multi-paragraph notes, comment threads with replies and all three statuses, 90-character unbroken strings, a 190-character beat, cards with no heading / no description / neither, an empty sequence, an empty act, a 26-card sequence, Unassigned, Parking Lot, a second board without a Parking Lot) and `..._STRESS_TEST_B_options_off.pdf` (2 pages; notes, comments, Parking Lot off).
* QA performed: `tools/verify_pdf.py` on all four PDFs (letter portrait, header text checked), every page of the primary PDF and the stress PDFs viewed at 90-100 dpi, grayscale thumbnails viewed, both scope variants printed and viewed, the preview script's variant switching tested in headless Chrome.

### J. Acceptance criteria
1. PDF is US Letter portrait; metadata title "Beat Board Outline"; footer on every page with the logo image (aspect 699:163, 17 pt high), "Created using OpenFrame" and "Page X of Y" with the real total (`tools/verify_pdf.py` passes).
2. Running header "<PROJECT> · BEAT BOARD OUTLINE" on every page and nothing else in the top margin; a blank project omits the project part.
3. No text, shape or image enters the footer band or the 46 pt side margins.
4. Order of acts, sequences, beats and cards equals board order (`position`); card numbers equal `Node.number`; Parking Lot items have no number; numbers are unchanged under scope "selected".
5. Act-level beats are never indented or styled as children of a sequence; the "Not in a sequence" label appears only in acts that have at least one sequence.
6. A note and a comment are distinguishable without colour (NOTE label + rule vs COMMENT label + box + status text).
7. Every comment shows its status text (Open / In Discussion / Resolved); replies are nested under their comment.
8. With notes, comments and Parking Lot off: no label, box, area bar or gap remains; the Includes line says "Board only".
9. The Parking Lot, if present, is the last area; Unassigned precedes it.
10. A card row, a comment box, or a heading with its first row never splits across pages; no act bar, sequence band or label is the last element of a page.
11. Every page that starts inside an act or sequence begins with the correct continued strip; no strip on a page that begins with an act bar.
12. A 200-character act title, a 190-character slugline, a 90-character unbroken string and a 3-paragraph note render fully (no truncation, no overflow past the margins).
13. Empty act prints "No sequences, beats or scene cards yet."; empty sequence prints "No beats or scene cards yet."
14. Grayscale rendering stays readable: Beat vs Card by tile text, parking by its label.

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
* Page fill of the sample (p1:87 p2:100 p3:100 p4:79 p5:99 p6:27); the low pages are explained in section H.
* Chrome's pagination of the main file is pinned by the generator; the Rust renderer will paginate differently.
* Whether comments should be shown with their status chip; the status values come from `comments/mod.rs` but only "Open" occurs in the supplied PDFs.
