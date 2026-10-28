# Agent handoff — implementing the OpenFrame PDF designs

Give this file to the coding assistant together with the package (this folder, or `OpenFrame_PDF_Design_Package_v2.zip`) and access to the OpenFrame repository.

## Task

Implement the PDF designs in this package in OpenFrame's Rust PDF renderer (the `windows/` tree of `Saravankumar25/OpenFrame`, current `main`; the package was prepared against commit `2f5a045416bd2125801329533fc47f986ccc8174`). Work one export at a time.

## Read first

1. `README.md` — what the package contains and what changed in v2.
2. `00_SYSTEM/PDF_Design_System.md` — shared rules: logo and footer, renderer constraints, colour, type, spacing, tables, pagination. **Appendix B lists renderer features the designs need that do not exist yet.**
3. `00_SYSTEM/Export_Inventory.csv` — every export: operation name, builder, UI entry point, page size, limitations.
4. `00_SYSTEM/Source_Map.md` — where each export lives in the repository.
5. `00_SYSTEM/QA_and_Validation_Report.md` — what was verified and what was not.

## For each export

| File | Use |
|---|---|
| `exports/<family>/<Name>.md` | The specification. §E content and order, §F visual spec, §G dynamic behaviour, §H pagination, §J acceptance criteria, §K open questions and known defects |
| `exports/<family>/<Name>.html` | Layout reference. Open in a browser; the toolbar toggles optional sections and variants |
| `exports/<family>/<Name>.pdf` | Visual target |
| `exports/<family>/stress-test/*` | Edge cases the implementation must also survive (long text, many rows, missing images, empty states) |

## Rules

- The HTML and PDF are references rendered by Chrome, **not** an implementation. Match the spec's rules (pagination, keep-together, repeated table headers, type sizes, spacing), not Chrome's exact break positions or the browser's Arial metrics. Renderer text is PDF base-14 Helvetica.
- Verify every source identifier named in a spec against the repository before relying on it. If something differs, say so and follow the repository for behaviour, the spec for appearance.
- Do the shared work first, then exports one by one:
  1. Footer on every page: embed the official logo `00_SYSTEM/assets/openframe-logo.png` losslessly (once, reused), `Created using OpenFrame` directly below it, real `Page X of Y` at the lower right; bottom content margin 66 pt. Do not redraw or retype the logo. No footer rule, no project text in the footer.
  2. Running-header support (a single top-right line). Mandatory only for the Call Sheet (`PRODUCTION · SHOOT DAY NN · REVISION n`); other exports as each spec states.
  3. Shared table and card behaviour: title row inside the repeating header, keep-with-next, no split rows or cards, keep-together groups.
- Several designs need new renderer features (Appendix B of the design system). Flag anything you cannot build instead of approximating it silently.
- Orientation changes are builder changes too (`Document::landscape(...)`): v2 makes Storyboard, Shot List, Shooting Schedule and Beat Board Card Board portrait; Day Out of Days, Moodboard and the Scene report stay landscape.
- Some specs ask for small behaviour changes in the application, not just layout: for example sorting Shooting Days in the Cast Scene and Location reports (§E and §J of those specs), a "Panels per row" option (2 or 3) for the Storyboard export dialog, and the Call Sheet section order. They are listed in each spec's "Changes from v1" and §K.
- Do not change the Screenplay, Sides or Story page exports. Do not add exports or UI options that a spec does not describe.
- Keep output deterministic (same data and settings give the same PDF bytes), and keep the existing rules: text wraps and rows grow, nothing is ellipsised or clipped; unsupported characters remain a clear error.

## Check your work

- Compare each export against its acceptance criteria (§J) and against the stress-test PDFs.
- Page counts in the reference PDFs are Chrome's. Some are fragile: the Shot List Entire Project sample sits at 99 % of its last page, and the Shooting Schedule page count depends on the cast column width. Judge pagination by the rules in §H, not by page count.
- The Daily View design has **no baseline PDF** and is unvalidated against a real export (its spec says so). Treat it as a proposal and confirm it against the real Daily View screen.
- `tools/render_pdf.py` and `tools/verify_pdf.py` re-render and check the reference PDFs (they need Python with PyMuPDF and Chrome/Chromium/Edge). They do not test your Rust output; for that, render your PDFs and compare them to the references by eye and against the acceptance criteria.

## If something in the package conflicts

Order of authority: the current source code and real UI, then the supplied example PDFs, then this package's specs, then the HTML. When you find a conflict, record it and ask rather than guessing.
