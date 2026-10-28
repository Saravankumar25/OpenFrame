# QA and Validation Report (v2)

Date of work: 2026-10-23. Source of truth: `Saravankumar25/OpenFrame` `main` @ `2f5a045416bd2125801329533fc47f986ccc8174`.
This report states what was actually run and looked at, and what was not. Nothing here claims validation against OpenFrame's Rust renderer — none of these designs has been rendered by it.

## 1. Scope check

| Requirement | Result |
|---|---|
| Every user-accessible PDF export in scope inventoried from source + UI | Yes — `Source_Map.md` §2–3, `Export_Inventory.csv` (21 export variants + 1 system sheet) |
| Screenplay, Sides, Story page, standalone Breakdown Summary excluded | Yes. `content=summary` exists in `breakdown.rs` but no UI sends it; not designed |
| Trio (HTML + PDF + MD) for every variant | Yes — 21/21 (`exports/*`), verified by directory listing and `check_pkg` script |
| Daily View included, no baseline claimed | Yes — marked unvalidated in MD, HTML and a banner printed in the PDF |
| Application source untouched | Yes. The repository was cloned read-only outside the package; `git status` in the clone is clean; no project files edited, nothing committed or pushed |
| Supplied PDFs / logo untouched | Yes (modified times predate the session; nothing was written in `exports/` of the input folder) |

## 2. Mechanical verification (`tools/verify_pdf.py`, run over all 61 PDFs)

For every PDF: page size is US Letter (portrait or landscape as designed); every page has the brand line, the logo image in the footer band with undistorted aspect ratio (4.289 : 1), and `Page X of Y` with the **true** page number and total; no text, shape or image intrudes into the 66 pt footer band or the side margins; running header text present where the HTML sets one (absent/none where it does not); no blank page.

* Result: **0 failures** on the final set, with one intentional exception explained below.
* `Moodboard_STRESS_TEST.pdf` p10: the header intentionally prints `MOODBOARD` instead of `LOW TIDE RADIO · MOODBOARD` (this is the "project title off" case — the project name must not leak when the option is unticked). The automated header check flags this; it is correct behaviour.
* Near-empty-page warnings occur only on **stress-test** PDFs (`Storyboard_Sheet_STRESS_TEST*.pdf`) where a short final scene / board starts on its own page by design (scene kept whole); none on any primary PDF.
* All 60 export HTML files were re-rendered after the final shared-CSS change; page count and per-page text matched the delivered PDFs 60/60 before they were replaced by the re-renders, so each PDF corresponds to its HTML.
* All HTML files: every `src`/`href`/`url()` resolves to a file inside the package; no network resource is loaded (a few sample URLs appear only as printed text).
* Every MD contains headings A–K (script check).

## 3. Visual inspection performed

* **Coordinator (me):** every page of the Call Sheet (primary + both stress tests) and the Brand reference; page 1 of all 21 primary PDFs; page pairs of Beat Board Card Board, Shot List Multiple Scenes; a sample of middle pages (Breakdown Combined p6/p10, Catalog p7, Budget p3, DOOD p2, Shooting Schedule p8, Shot List Entire p4, Storyboard p4).
* **Per-family agents (self-reported, spot-checked above):** every page of each primary PDF and most stress PDFs, at 60–110 dpi or as contact sheets; grayscale renders for at least one page per document.
* **Not viewed page-by-page at full size by anyone:** the full-length Breakdown Combined and Catalog primaries (judged from contact sheets + fill metrics), Beat Board "Selected items" variant on the board layout, and some pages of the long stress PDFs. These are not defects found — they are coverage gaps.
* **Grayscale:** done on sample pages for every family (status chips, category names, MANUAL/SUGGESTED, NEEDS INPUT, OVER PLAN, DOOD codes remain distinguishable through text/shape). A full grayscale pass over every page was not performed.

## 4. Dynamic-content behaviour — what was tested (HTML/PDF level)

| Behaviour | Evidence |
|---|---|
| Long titles / labels | Call Sheet (80-char title), Breakdown (230-char heading), Budget (long line names), Reports, Shot List, Beat Board stress tests |
| Long notes / descriptions | Call Sheet, Breakdown (400-char notes), Beat Board, Storyboard, Shot List |
| Many rows / multi-page tables with repeating headers | Call Sheet (17 schedule rows, 24 crew), Catalog (321 items), Shot List (40+), Reports, Shooting Schedule (14 days), Budget (56 lines), DOOD (42 cast × 36 days) |
| Optional sections present / absent | Call Sheet (all on / all off / minimal draft), Beat Board (options off), Moodboard (images/captions/title/notes toggles), Breakdown (notes / suggested toggles) |
| Missing or odd images | Call Sheet reference images, Moodboard, Storyboard (tall, square, missing, unreadable, blank panel), Shot List (no image, odd aspect) |
| Empty datasets | Reports (`_EMPTY`, `_NO_SOURCE`), Catalog empty, Call Sheet minimal draft, empty storyboard |
| Wide / unexpected column counts | Reports `_WIDE` (orientation rule), DOOD column groups, Call Sheet talent table with every optional column |
| First / middle / last page, footer + page numbers | Verified mechanically on every page of every PDF |

## 5. Defects found in the CURRENT application output (verified in the supplied PDFs and/or source)

Cross-cutting
* The footer redraws the logo mark as vector lines with typed "OpenFrame STUDIO" text (`design::draw_footer` / `lockup`) instead of the official artwork; footer text carries project/document and `Page n / N`.
* `ReportImage::decode` converts every image to JPEG q85 (lossy for logos).
* Table titles are separate `Section` blocks followed by a "(CONTINUED)" band: headings are orphaned at page foot (Call Sheet p1, Beat Board board p2/p4, Breakdown scene 2 head) and "(CONTINUED)" is printed on a table's first page (Call Sheet p2, Breakdown p55/p58).
* Tiny text: chips/labels at 5.8 pt in several exports; `pal::FAINT` (#8792A5, 3.1 : 1) used for readable text (Moodboard notes, Storyboard notes).

Per export (full lists with page citations are in each MD, section B.1)
Beat Board: card split across pages, stranded sequence headings, act-level beats indistinct, notes vs comments only differ by prefix · Breakdown: scene head stranded, tag chips detached from their element, 2/3 of a page empty on p1, "OMITTED" replaces heading, Catalog rows ≈ 58 pt with no column header · Budget: no category band on continuation, planned vs actual sums not comparable without a note, over-plan marked by red text only, clock-dependent masthead date · Call Sheet: orphaned heading, wrong "(CONTINUED)", cramped cast columns, lone reference-image page · DOOD: 8 pages for 14 days × 17 cast, legend missing on continuation pages, totals sentence wrong for filtered ranges (source) · Moodboard: image count ignores option, 3 + 1 image split · Reports: 2-row pages, false red MISSING chip on Off Day rows (source), filter dropped on snapshot export (source) · Shooting Schedule: near-empty last page, wrapped cast names, orphaned legend · Shot List: "Overhea/d" mid-word break, lens text overflow, duplicated scene heading · Storyboard: lone-panel pages, no scene identity on continuation pages.

## 6. Unverified items (must be checked by the implementer)

1. **Nothing was rendered by the Rust renderer.** Wrap points, row heights, stripe/chip metrics and every pagination rule must be re-validated; Chrome (Arial standing in for Helvetica) decided breaks in the references.
2. New renderer capabilities required (Appendix B of `PDF_Design_System.md`) — feasibility not assessed.
3. Mac tree (`mac/`): not compared; Windows is authority because the supplied PDFs match the Windows renderer.
4. Daily View: no baseline PDF; layout/visuals unvalidated; sample times/status invented and labelled.
5. Non-Latin scripts / embedded-font behaviour: out of scope, not tested.
6. Rust handling of a single row/card taller than a page.
7. Selected-scene ordering (Shot List), Beat Board scope `selected`/`project` output, comment replies: from source only.
8. Whether the product owner accepts the structural changes listed in each MD, section E (e.g. Call Sheet talent contact/notes folding, Cast Scene column order swap, Budget category subtotals, Storyboard 2/4-column option, Moodboard 2-column rule).
9. Contrast figures are computed from hex values (WCAG formula), not measured from printed output; no physical print test was done.
10. The logo asset: only a raster (812 × 347) was supplied/found; the package crops it to artwork bounds. If a vector master exists it should replace it.

## 7. Blockers

None. Open product questions are collected in `PDF_Design_System.md` §16 and in section K of each export MD.

## 8. Reproducing the checks

```
python tools/render_pdf.py exports/call-sheet/Call_Sheet.html exports/call-sheet/Call_Sheet.pdf
python tools/verify_pdf.py exports/call-sheet/Call_Sheet.pdf --header "SHOOT DAY 01" --thumbs out --gray
```
Requires Python 3 with PyMuPDF, and Chrome/Chromium/Edge (v131+ for CSS page margin boxes; built with 155).

## 9. Version 2 changes — what was checked

v2 was made by duplicating the v1 folder and changing only the files below; the v1 folder was hashed before and after the work: **0 files changed in v1** (223/223 identical to the pre-v2 snapshot).

| Export | Change | Pages v1 → v2 | Checked |
|---|---|---|---|
| Storyboard | portrait; 2 / 3 per row; compact header; visual lift | 5 → 5 (2-up), 4 (3-up) | verify OK on all 4 PDFs; every page of both primaries and stress PDFs viewed (contact-sheet scale, several pages at full scale); grayscale samples |
| Shot List ×3 | portrait; Notes under Description | 1→1, 2→1, 7→5 | verify OK; all primary pages viewed; word-split scan for "Overhea/d"-type breaks: none |
| Shooting Schedule | portrait at real size | 14 → 13 | verify OK; 5 pages + all stress pages viewed; smallest text measured 6.4 pt; wraps measured (see its MD §K) |
| Beat Board Card Board | portrait, 3 per row | 7 → 6 | verify OK; primary and stress viewed; 4-column alternative built and rejected with evidence (MD) |
| Call Sheet | regrouped (E in its MD); page 1 unchanged; groups kept together (no forced page start) | 4 → 4 (stress 8→9, minimal draft 1→1) | verify OK; every page of primary and both stress tests viewed |
| Cast Scene / Location reports | days ascending | unchanged | verify OK; parsed day lines of 8 PDFs: 0 descending sequences |

* All 60 v2 HTML files were re-rendered and compared with their delivered PDFs (page count and per-page text): 0 mismatches. `Storyboard_Sheet_3col.pdf` and `..._STRESS_TEST_3col.pdf` are produced from the same HTML with `#cols=3` (`tools/build/visual/render_storyboard.py`).
* `verify_pdf.py` over all v2 PDFs: 0 failures other than the intentional Moodboard "title off" header. Near-empty-page warnings only on Storyboard stress tests (empty board). The Call Sheet minimal draft no longer warns: a first v2 build forced page starts for the two groups and produced 3 mostly empty pages; replacing that with a keep-together rule (tested on all three Call Sheet files) fixed it without changing the normal sheet.
* Unchanged documents (Breakdown ×3, Budget, DOOD, Moodboard, Daily View, Beat Board Outline, four other reports) compared with v1 by per-page text: identical.
* **Not verified in v2:** everything in §6 still applies. Additionally: the real Rust renderer cannot yet draw several v2 elements (storyboard number tile / chips / caption band / per-storyboard running header; shot-list mixed bold/regular note runs); portrait page counts are Chrome's and fragile for the Shot List Entire sample (scene 42 sits at 99 % of page 5) and the Shooting Schedule (a different cast-column width changes 13 to 12 pages).
* Evidence-based caveats found while building portrait: Shooting Schedule heading wraps to two lines in 14 of 44 strips (v1: 2) and 6 of 145 cast entries wrap; Shot List description column is 160–180 pt so long descriptions plus notes make tall rows (one stress row ≈ 440 pt).
