# FDX Interchange Test: what each construct tests

Copyright 2026 OpenFrame Studio. Fictional sample written for OpenFrame; you may use it to learn the app.

`FDX-Interchange-Test.fdx` is a short fictional script, "The Three Desks", written to exercise the Final Draft
importer and exporter. It is well-formed XML with no document type declaration and no entities other than the five
that XML predefines, so a safe parser reads it without any special settings. Every construct below is valid Final
Draft or valid XML. A reader that does not support a construct should preserve it, map it on purpose, or report it.
It must never turn it silently into Action.

| Construct | Where in the file | What it tests |
|---|---|---|
| XML declaration with `standalone="no"` and comments before the root | first three lines | The parser skips the prolog and comments and still finds `FinalDraft`. |
| Title page fields | `TitlePage` | Title, credit, author, source, contact (left), draft and date (right) and copyright come through as title data, not as script text. |
| Scene numbers 1, 2, 12, 12A, 12B, 13 | `Number` on each Scene Heading | Numeric and alphanumeric numbers survive the round trip, including a gap (2 to 12) left by cut scenes after locking. |
| `SceneProperties` with `Title`, `Length`, `Page` and `Color` | each Scene Heading | Scene metadata is read or ignored without changing the heading text. |
| Revision marks on two sets | `RevisionID="1"` and `RevisionID="2"` on Text runs, `Revisions` block | Each run keeps its revision set; the set names (Blue, Pink), colours and marks are read; `ActiveSet` is 2. |
| Custom element type "Text Message" | `Paragraph Type="Text Message"` and its `ElementSettings` | A custom element keeps its own type and settings and is not mapped to Action. |
| Dual dialogue with a parenthetical on one side | `DualDialogue` in scene 2 | Both speakers stay side by side; the parenthetical stays attached to the second speaker. |
| Explicit page break | `StartsNewPage="Yes"` on scene 12 | The page break is kept as a page break directive, not as blank paragraphs. |
| Telugu, Hindi and Tamil text | dialogue in scenes 1 and 13 | Non-Latin text survives import, export and PDF rendering with correct shaping (conjuncts, vowel signs, virama). |
| Hindi with a nukta letter | "तूफ़ान" | Characters built from a base letter and the nukta sign round-trip byte for byte. |
| Numeric character references | the Telugu word written as `&#x0C24;...` in scene 13 | References decode to the same text as typed Telugu. |
| Predefined entities | `&amp;`, `&lt;`, `&gt;`, `&apos;` in scene 13 | Entities decode on import and are escaped again on export. |
| CDATA section | Action in scene 13 | Text inside CDATA, including `<` and `&`, is read as plain text. |
| Tab characters | `&#9;` in the whiteboard line | Tabs are kept or turned into spaces on purpose, never dropped. |
| Several Text runs with styles | `Style="Italic"`, `Style="Bold+Underline"`, `Style="AllCaps"` | Inline styles are kept on the right words and runs join without extra spaces. |
| Text run attributes `Font`, `Size`, `Color`, `AdornmentStyle` | scene 13 | Formatting attributes do not break the run or its text. |
| Shot | scene 2 | The Shot element keeps its type. |
| Right-aligned transition | `Alignment="Right"` on CUT TO: | Alignment on a transition is accepted. |
| Character extension | `VIKRAM (V.O.)` | The extension stays with the cue and does not create a new character name. |
| Empty paragraphs | `<Text></Text>` and `<Text/>` after scene 12A | Both empty forms are accepted; they are not counted as script content. |
| Lyrics paragraphs | scene 12B | The Lyrics type is preserved (it maps to the lyric element). |
| General paragraph and a paragraph with no Type | scene 12B | A missing Type is treated as General; neither becomes a scene heading. |
| Single-quoted attributes in a different order | `Type='Action' Alignment='Center'` on THE END | Attribute quoting and order make no difference; centred text is read as centred. |
| Unknown element inside a paragraph | `VendorExtension` in the last transition | An unknown child element is ignored and listed in the import report; the paragraph text is kept. |
| Header and footer, page layout, SmartType lists | blocks after `TitlePage` | These blocks are read where supported and skipped safely where not. |
| Script note with a range | `ScriptNotes` | A note is imported as a note or reported, and never appears as script text. |

What this file does not contain on purpose: a document type declaration, custom entities, external references and
anything larger than a few kilobytes. Hostile inputs of that kind live in
`crates/openframe-import-export/tests/fixtures/hostile/fdx/`.
