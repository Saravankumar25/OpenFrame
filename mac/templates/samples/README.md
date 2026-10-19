# Sample scripts

OpenFrame ships four fictional sample scripts. OpenFrame Studio wrote them and owns them (owner decision 2026-10-07,
recorded in `docs/implementation/owner-decisions.md`). They serve two purposes: new users open them to learn the app,
and the test suites use them as regression fixtures.

Copyright 2026 OpenFrame Studio. Fictional sample written for OpenFrame; you may use it to learn the app.

Every person, place, organisation and story in these files is invented. No text is taken from an existing script,
book, song or brand. The same copyright note appears inside each file: on the Fountain title page (`Copyright:`
key), and in an XML comment and the title page of the FDX file.

| Sample | File | Format | Scenes | What it covers |
|---|---|---|---|---|
| Example Feature | `Example-Feature.fountain` | Fountain | 36 | "The Tower at Pedda Revu". Three acts marked with sections and synopses, more than 20 locations, day, dusk, night and dawn, ten speaking parts, props (soldering iron, ledger, transformer, marine battery), vehicles (bus, police jeep, fishing boats), a company move from town to coast (marked with a note), an intercut, an insert, and dialogue in Telugu and Hindi with characters who mix English into both. |
| Example Short | `Example-Short.fountain` | Fountain | 10 | "Lost Property". A complete short story with one main location, repeated scene headings, CONTINUOUS and an insert. |
| Example Scene | `Example-Scene.fountain` | Fountain | 1 | "The Spare Key". One scene that uses every standard element: section, synopsis, notes, scene heading with a scene number, action, character, character extension, parenthetical, dialogue, dual dialogue, a shot (an upper-case action line, because Fountain has no shot element), centered text, forced character, forced action, page break, lyrics, emphasis, boneyard, transition and forced transition. |
| FDX Interchange Test | `FDX-Interchange-Test.fdx` | Final Draft XML | 6 | "The Three Desks". Title page fields, scene numbers including 12A and 12B, two revision sets, a custom element type, dual dialogue, an explicit page break, Telugu, Hindi and Tamil text, and unusual but valid constructs. Each construct is listed in `FDX-Interchange-Test.README.md`. |

## Rules for these files

- Keep them fictional and OpenFrame-authored. Do not paste text from any other script.
- Follow the writing standard (`docs/engineering/35-writing-standard.md`): no em dashes or en dashes anywhere, including
  the script prose. Scene headings use a hyphen between location and time of day.
- Scene counts above are counted from the files. If you edit a sample, update the count here and the regression
  expectations that use it.
- The FDX file must stay well-formed XML with no document type declaration.
