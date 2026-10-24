# ADR-0036: Production PDF design system v2

- **Status:** Accepted 2026-10-24 (owner: the PDF design package v2 is the design source for production PDFs). The design
  is accepted. The renderer work is a separate slice: when this ADR was written the renderer in
  `crates/openframe-import-export/src/report.rs` still had `MARGIN = 46` and `FOOTER_SPACE = 34` (80 pt of bottom
  clearance) and the vector footer lockup. [feature-status.md](../implementation/feature-status.md) moves only with
  evidence from the renderer and its golden files.
- **Date:** 2026-10-24
- **Amends:** [ADR-0008](ADR-0008-deterministic-pdf.md) for production documents only (Helvetica documents: Beat Board,
  Breakdown, Catalog, Moodboard, Storyboard, Shot List, Shooting Schedule, Day Out of Days, Daily View, Call Sheet,
  Budget, production reports). The screenplay, Sides and the Story page are not covered. Determinism (decision 3 of
  ADR-0008), the base-14 fonts with embedded fallback ([ADR-0016](ADR-0016-pdf-embedded-fonts-and-shaping.md)) and
  paper size as an explicit setting are unchanged.
- **Design source:** [pdf-design-package-v2](../specs/pdf-design-package-v2/README.md), above all
  `00_SYSTEM/PDF_Design_System.md` and one `.md` per export.

## Context

The production PDFs were drawn by a renderer with a hairline rule, a vector lockup and a bold tracked footer text, 80 pt of
bottom clearance, and several tables whose title could be left alone at the foot of a page. A design package (version 1,
then version 2 after the owner's review) specifies every export with reference HTML, rendered PDFs, stress tests and a
specification per export. Version 2 changed the orientation of four exports and the grouping of the call sheet.

## Decision

1. **The package is the design source.** Where an export's `.md` and the renderer disagree, the package wins until the owner
   changes it. A deliberate difference is written in that export's section E with its reason.
2. **Footer on every page.** The official logo as a lossless image (17 pt high, never redrawn, typeset, recoloured or
   cropped) at the bottom left, `Created using OpenFrame` directly below it, `Page X of Y` at the bottom right with the
   real page count. No footer rule and no project text in the footer. The logo is embedded once and reused on every page.
3. **Bottom margin 66 pt** (was 80 pt). Margins left, right and top are 46 pt. Content area 520 by 680 pt portrait and 700 by
   500 pt landscape. `MARGIN` and `FOOTER_SPACE` are re-derived together; every pagination and golden file changes.
4. **Running header.** Mandatory on the Call Sheet only: one line at the top right (`PRODUCTION · SHOOT DAY NN · REVISION n`),
   no rule, no logo. Single-page documents have none. A long multi-page table may carry a small `PROJECT · DOCUMENT` line
   where its export `.md` says so.
5. **Tables.** The title sits inside the repeating header, so a title can never be left alone from its table; the renderer
   adds "· continued" on later pages. Breaks fall only between rows; a group header keeps with the first row of its group;
   a total keeps with the last two rows it sums; a table title is never the last element on a page. Rows grow to fit
   wrapped text, and nothing is cut with an ellipsis.
6. **Keep rules.** Cards, figures and rows move whole to the next page; a card taller than a page splits at field-row
   boundaries only; headings keep with their content; a scene, a day or a category stays whole unless it is taller than a page.
   Images sit in a fixed-ratio frame, contained, never cropped or stretched.
7. **Orientation per export, as the package lists it.** Portrait: Beat Board (Outline and Card Board), Breakdown, Catalog,
   Storyboard (two panels per row, three as a variant), Shot List in every scope, Shooting Schedule, Daily View, Call
   Sheet, Budget and the narrow reports (Location, Cast Scene, Prop, Schedule, Breakdown Completeness). Landscape: Day
   Out of Days, Moodboard and the Scene report. The application keeps switching a report with more than 5 columns to
   landscape. Changing an export's orientation is a builder change, not a style change.
8. **Paper stays US Letter for production documents.** A4 is not added by this ADR.

## Consequences

- The renderer needs the capabilities in Appendix B of the design system: a lossless image path, a per-page running
  header, multi-row repeating headers with keep-first and keep-last rules, keep-whole groups and measured pagination.
  Until each lands, the export stays on the old layout and its feature status says so.
- All golden PDFs of production exports change at once with the footer geometry (14 pt more content per page).
- The Mac tree has its own renderer files and was not compared when the package was made. Porting is in the
  [mac port list](../roadmap/mac-port-list-2026-10-24.md).

## Still open

- A4 for production documents; whether every long export gets the running line (each export `.md` decides for now); a
  Windows-only redesign or a Mac port of the renderer (the port list assumes a port).
