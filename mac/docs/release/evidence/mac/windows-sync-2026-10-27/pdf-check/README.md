# Fresh PDFs on the Mac after the Windows sync (2026-10-27)

Made by `cargo run -p openframe-gold-project -- --no-ai` on commit `68ce45c6` (Apple M1, macOS 26.5): the gold film
"Low Tide Radio" built and exported through the same core operations the app calls. Result: "done in 236s: 0
unexpected failures, verification passed". The PDFs are in `tests/gold-project/outputs/`. The pages here were drawn
with PDFKit at 1.2x and looked at one by one.

| Document | Pages looked at | Checked against | Result |
|---|---|---|---|
| Screenplay, First Draft | title page, pages 1 and 2 | Formatting standard v5 | Title page blocks apart with a gutter: the known copyright and representation collision is gone. First body page numbered `1.` (number_first_page on). 24 pt after a scene heading, speech blocks with cue, parenthetical and dialogue columns, `(CONT'D)` on a continued speech. No brand line |
| Sides, day 1 | cover, page 1 | v5 section 8 | Running header with production, day and draft; scene numbers both sides; no OpenFrame logo and no "Created using OpenFrame" line |
| Call sheet, day 01 | page 1 | PDF design package v2 | Masthead, shoot day block, crew call box, Production, Locations in that order; footer logo as ADR-0036 asks for production reports |
| Day Out of Days | page 1 | package v2 | One matrix, cast frozen, legend in the header, totals inside, off days shaded |
| Budget | page 1 | package v2 | Summary tiles, one table per category, Indian digit grouping, decimal alignment |

Not done here: every page of every PDF, and a side by side with the package's own sample PDFs.
