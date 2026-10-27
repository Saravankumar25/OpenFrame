# OpenFrame Studio — Vendor-Neutral Screenplay Formatting Standard

## Purpose and scope

This document defines a **vendor-neutral default profile for a US Letter feature-film screenplay**. It is intended to help OpenFrame produce clean, readable, conventional screenplay PDFs. It is not a request to copy Final Draft, StudioBinder, or any other application's quirks, and it does not claim that every measurement below is legally or universally mandated by the film industry.

Screenplays vary by country, production, genre, broadcaster, and use. The defaults below represent common US feature-screenplay practice. If a production explicitly requires a house style, that profile can override optional choices while retaining the engine's correctness guarantees.


## Mandatory agent guardrails: preserve content and fix only real deviations

These requirements govern implementation and take precedence over any interpretation that would unnecessarily rewrite the screenplay or redesign working features.

1. **Preserve screenplay content.** Do not rewrite, delete, reorder, paraphrase, or silently reclassify screenplay text. Preserve dialogue, action, character names, scene headings, scene order, metadata, and existing semantic element types. Do not modify the source screenplay just to achieve a preferred page count or visual match.
2. **Preserve correct behavior and settings.** Change only demonstrated deviations from the chosen vendor-neutral screenplay profile or verified defects. Keep existing features, user-configured settings, and formatting that already work unless a change is strictly necessary to fix a demonstrated issue. Document any necessary exception.
3. **Allow pagination to change without changing the story.** Fixing element spacing and pagination can move content between pages. That is acceptable as long as all source text remains present and in order. Output-generated markers such as page numbers and dialogue continuation markers (`(MORE)` / `(CONT'D)`) may change when required by correct pagination; do not write generated markers back into the source screenplay or create duplicates.
4. **Apply the necessary changes to Sides as well.** The Sides PDF shares core screenplay layout/rendering behavior, so applicable fixes to element spacing, alignment, wrapping, typography, and pagination must be verified in Sides too. Check Sides-specific behavior separately, including selected-scene order, its running production/day/draft header, page numbering, branding, and the optional Sides cover page. A screenplay title-page fix does not automatically fix the separate Sides cover.
5. **Check every supported platform implementation.** If the repository maintains separate Mac and Windows source trees, identify and keep corresponding implementations, generated rule definitions, and tests aligned as appropriate. Do not assume a fix in one tree updates the other.
6. **Verify before declaring completion.** Add regression tests, generate fresh PDFs, and inspect rendered pages. Report the root cause, changed files, tests run, before/after measurements, verification of source-text preservation, results for screenplay and Sides exports, and any remaining limitations. Do not claim success based only on compilation or unit tests.

## 1. Rule categories

Every rule should be classified as one of the following:

- **Core convention** — widely accepted screenplay practice and the default for a conventional US feature screenplay.
- **Layout invariant** — must never break, regardless of template: no overlaps, clipping, accidental content loss, detached cues, or broken pagination.
- **Profile-dependent** — varies between a clean submission draft, a shooting/production draft, and sides.
- **House-style option** — more than one approach is accepted; be consistent and document the choice.

Do not treat a specific application's default as an industry mandate. Do not change working settings solely to match one reference PDF. Correct demonstrated defects and implement consistent screenplay conventions.

## 2. Page setup and typography

| Property | Default for this profile | Classification |
|---|---|---|
| Page size | US Letter, 8.5 × 11 in (612 × 792 pt) | Core convention for US scripts; support other sizes as explicit profiles |
| Body typeface | 12 pt Courier or a metrically compatible Courier Prime | Core convention |
| Line leading | Single-spaced; approximately 12 pt baseline-to-baseline at 12 pt type | Core convention |
| Left page margin | About 1.5 in to allow binding | Common US convention |
| Right page margin | About 1 in | Common default; modest variations exist |
| Top and bottom text margins | About 1 in | Common default; keep all glyphs and page furniture inside safe areas |
| Typical page density | Roughly 55 typed lines per page, depending on template and what is counted as a line | Guideline, not a guaranteed page-count formula |
| Horizontal character advance | Standard Courier metrics, approximately 10 characters per inch at 12 pt | Convention; rendering must use the actual selected font metrics |

Screenplay page count is often used as a rough runtime estimate, which is one reason traditional 12-point Courier formatting remains common. Do not shrink type, compress leading, or squeeze margins merely to force a script to a desired page count.

### Geometry rules

1. Use one consistent coordinate system and page origin for all layout calculations.
2. The line layout and PDF drawing stages must use the same font metrics and line-leading model.
3. Page size, body margins, header/footer safe areas, and usable body rows must be derived together. Do not change a margin constant without rechecking pagination and page furniture.
4. Support page size as an explicit setting/profile; never silently switch between US Letter and A4.
5. Keep screenplay columns consistent from page to page. Text must not drift because of different font substitution or inaccurate width estimates.

## 3. Horizontal element layout

The following are **typical target regions** for 12-point Courier on US Letter, not coordinates that must match any particular application to the fraction of a point.

| Element | Typical position | Text treatment |
|---|---|---|
| Scene heading / slugline | Starts around 1.5 in from left; uses the main action column | Uppercase; left-aligned |
| Action / description | Starts around 1.5 in; ends around 7.5 in | Normally sentence case; left-aligned |
| Character cue | Positioned above and visually centered over the dialogue column | Uppercase; consistent position relative to dialogue |
| Dialogue | Starts around 2.5 in; narrower than action, often about 3.5 in wide | Left-aligned; single-spaced |
| Parenthetical | Narrower than dialogue, slightly indented from dialogue's left edge | In parentheses; concise; attached to relevant speech |
| Transition | Usually right-aligned near the right edge of the main text area | Uppercase, e.g. `CUT TO:`; use deliberately |
| Shot heading | Usually shares the scene-heading/action column | Often uppercase; keep with the description that follows |
| Page number | Upper-right header area | Consistent on all pages where numbering is enabled |

The exact character-cue and parenthetical positions vary slightly among accepted templates. The requirement is that character cues align consistently with their dialogue, dialogue remains visibly narrower than action, and elements never collide or drift.

Do not use leading spaces in screenplay text to simulate columns. Store semantic element types and render them through the element-layout rules. An action paragraph must not be rendered as dialogue because of its textual position in an imported file.

## 4. Vertical spacing and gaps

### Baseline convention

For 12 pt Courier-style single spacing, treat one line unit as approximately 12 pt (1/6 in). A gap of 24 pt between text baselines corresponds to one intervening blank line; a gap of 36 pt corresponds to two intervening blank lines. The visible white-space height is not itself the baseline-to-baseline measurement.

The following grid is a **recommended default for a conventional US feature screenplay**. It is a consistent house profile based on common formatting guidance; spacing details can differ among legitimate templates. The engine should represent these as element/paragraph spacing rules, not literal empty paragraphs in the screenplay content.

| Relationship | Recommended baseline gap | Intended appearance |
|---|---:|---|
| Lines inside one action or dialogue paragraph | 12 pt | Single-spaced lines |
| One action paragraph to the next | 24 pt | One blank-line separation |
| Action to character cue | Usually 24 pt | Clear separation between action and speech |
| Character cue to dialogue | 12 pt | No blank line between cue and its speech |
| Character cue to a parenthetical, then parenthetical to dialogue | Usually 12 pt per line step | The speech group reads as one block |
| One speech block to the next character cue | Usually 24 pt | One blank-line separation between speakers |
| Action/dialogue to transition | Usually 24 pt | Transition has breathing room before it |
| Transition to the next scene heading | Recommended 36 pt | Two blank-line separation before the new scene |
| Previous scene content to a new scene heading when no transition is present | Recommended 36 pt | Same clear scene separation |
| Scene heading to the first action line | Recommended 24 pt | One blank-line separation after the heading |
| Shot heading to its description | Normally 24 pt or the profile's matching heading rule | Heading stays visually attached to its description |

**Important:** a scene-heading gap of 36 pt is a conventional target, not a universal law. The StudioBinder sample exhibited 36 pt in the transitions inspected, while OpenFrame showed 24 pt. That is a useful reference comparison, but the decision should be encoded as a single consistent profile rule, not as a special-case patch for those three pages.

### Spacing-engine requirements

1. Choose one owner for each gap: either the preceding element's space-after or the next element's space-before. Never count both accidentally.
2. Keep spacing inside paragraphs separate from spacing between elements.
3. Use a relationship matrix like the table above as the source of truth. Do not add arbitrary y-coordinate offsets to selected scenes or pages.
4. Keep the scene-heading-to-first-action rule independent from the gap before a scene heading.
5. Transitions are optional. Apply the same clear scene-separation intent whether a transition precedes the scene or not.
6. Keep spacing values in line units or points derived from the same layout geometry; avoid pixel-based spacing.
7. Allow explicitly configured styles when a production needs a different house style, but make the default internally consistent.
8. Do not put literal blank paragraphs into screenplay content merely to push elements down the page. Such paragraphs can create unstable pagination after edits.

## 5. Screenplay element semantics

- **Scene heading:** begins a scene and normally identifies INT./EXT. (or an accepted equivalent), location, and time of day. Use uppercase consistently. Scene numbers are usually a production-draft feature, not required in a clean submission draft.
- **Action:** describes what can be seen or heard. Use the main action column and normal paragraph spacing. Avoid treating prose as dialogue or mixing stage directions into character speech blocks.
- **Character cue:** identifies who speaks, in uppercase, immediately before the speech block.
- **Dialogue:** sits in the narrower dialogue column under its cue.
- **Parenthetical:** a short delivery or action note associated with a line of dialogue. Keep it attached to the speech it qualifies; do not use it as a substitute for action paragraphs.
- **Transition:** a distinct, normally uppercase element aligned near the right edge. Use transitions where useful, not as mandatory decoration between every scene.
- **Shot/camera heading:** optional; when used, render consistently and keep with its following description.
- **Character extensions:** forms such as `(V.O.)` and `(O.S.)` should be attached to the appropriate character cue according to the selected profile.
- **Centered text, lyrics, montages, inserts and custom elements:** have explicit semantic styles and pagination rules. Do not infer their layout from manually typed spaces.
- **Opening/closing markers:** `FADE IN:`, `FADE OUT.`, and `THE END` are accepted conventions but not all are mandatory in every screenplay. The opening `FADE IN:` may have a distinct placement from mid-script transitions depending on the chosen style; keep that treatment deliberate and consistent.

## 6. Pagination and page breaks

These are layout invariants and should be treated as higher priority than matching a reference application's whitespace.

### Keep-with-next and keep-together

1. **Scene heading:** never leave a scene heading alone as the last meaningful item on a page. Keep it with the start of the scene's action.
2. **Shot heading:** keep it with the description that follows.
3. **Character cue:** never leave a cue at the bottom while its parenthetical/dialogue starts on the next page. Keep the cue, its parenthetical when present, and a useful opening portion of the dialogue together.
4. **Parenthetical:** keep it with the dialogue it modifies.
5. **Short dialogue:** if a speech fits on the next page without an unreasonable blank area, prefer moving the block together rather than splitting after the cue.
6. **Long action paragraphs:** allow controlled splits. Where practical, avoid a single-line fragment at the top or bottom of a page.
7. **Transition:** do not strand a transition at the bottom of one page while the following scene heading begins on the next. Keep the transition and next heading together when feasible, while avoiding excessive blank space or pagination loops.
8. **Intentional page break:** honor explicit page breaks. Do not start every feature-film scene on a new page unless a chosen format calls for it.
9. **Safe regions:** body content, page numbers, headers, footers, branding and revision marks must not collide or print outside the page.
10. **No accidental loss:** all screenplay text must remain present and in order after export; no line should disappear, duplicate, or change element type silently.

### Dialogue continuation across pages

When a speech genuinely continues across a page break, use the chosen screenplay profile's continuation convention, commonly `(MORE)` at the end of the first page and the character cue with `(CONT'D)` at the start of the next page. Generate these from layout state, not by inserting them permanently into the screenplay text.

Do not add `(CONT'D)` simply because the same character speaks in a later scene. Distinguish a single speech split by a page break from separate speeches, and distinguish both from a scene continuing across pages in a production draft.

If automatic character-continuation labels are supported, document the behavior and test the cases where action interrupts a speech, a new scene begins, or a page break occurs. Preserve imported explicit labels without creating duplicates.

### Page-boundary validation

After pagination, validate at minimum:

- no orphaned scene/shot headings;
- no orphaned character cues;
- no detached parentheticals;
- no stranded transitions;
- no empty pages or accidental repeated page breaks;
- no overlap with headers, footers or page numbers;
- no missing or duplicated text at split boundaries;
- no pagination loop or non-deterministic result.

## 7. Page numbers, headers, scene numbers and title page

These contain legitimate profile-specific choices, so OpenFrame should not impose one behavior on every kind of screenplay.

### Clean submission/spec screenplay

- Use a clean title page with title and writer credit. Contact details, draft date and other details depend on submission requirements; blind/anonymous submissions may require removing identifying information.
- Keep the title page unnumbered.
- A common practice is to omit a visible number on the first screenplay page and begin with `2.` on the next page. Some valid workflows number the first body page `1`. Make the chosen numbering convention an explicit profile setting, not an accidental side effect.
- Page numbers normally appear in the upper-right header area, consistently aligned. A period after the number is common in US screenplay format.
- Running production headers, confidentiality footers and scene numbers are usually omitted unless requested by the submission or house style.

### Production/shooting draft

- Scene numbers, draft/revision identifier, revision marks, production label and additional headers can be included when required by the production.
- Keep scene-number placement aligned to its scene heading and consistent across pages.
- If a production uses locked pages or revision-color conventions, handle those as explicit production features rather than silently applying them to all scripts.
- A scene-continuation marker is a production-specific mechanism and is not the same as dialogue `(MORE)` / `(CONT'D)`.

### Title-page layout invariants

- No visible script page number on the title page.
- Keep title, writer credit, “based on” credit, contact information, representation, copyright, draft/date and production metadata in deliberate blocks.
- If two blocks sit side-by-side, give each a constrained width and independent positioning; wrap long text inside its own block.
- No collision, clipping, overlap or text outside the safe area. Never use long runs of spaces as a substitute for columns.
- Title-page typography and alignment can be a house-style choice, but legibility and correct content flow are invariants.

### Known title-page defect in the current OpenFrame export — explicitly investigate and fix

The reviewed `LOW TIDE RADIO - Second Draft` PDF has a visible collision in the lower title-page information area: the copyright text (`Copyright 2026 Kayal Pictures. All rights reserved.`) runs into the adjacent representation text (`Representation: Lighthouse Talent, Mumbai`). In the extracted/rendered output, the collision is visible as `AllRepresentation: Lighthouse`. Treat this as a confirmed defect to reproduce and fix, not merely a general checklist item.

Required handling:
1. Trace the title-page layout path and identify why the two blocks overlap (for example, unrestricted line width, incorrect column origin/width, or text-flow logic). Confirm the cause in code rather than guessing.
2. Keep copyright/legal text and representation details as independently positioned, width-constrained blocks or columns. Allow each block to wrap within its own width; neither may paint into the other block's area.
3. Preserve all title-page content and metadata. Do not delete words or abbreviate text merely to hide the overlap.
4. Add a regression test using sufficiently long copyright and representation strings, and inspect a newly generated PDF at normal zoom to verify the text remains separated.
5. Check whether the separate Sides cover-page renderer has analogous overlap risks and fix them only where found; do not assume the screenplay title-page fix automatically covers Sides.

## 8. Screenplay PDF versus shooting draft versus sides

Share the core layout engine where possible, but do not assume the document types have identical page furniture or content-selection behavior.

| Profile | Core requirements | Profile-specific behavior |
|---|---|---|
| Feature screenplay / spec draft | Conventional screenplay typography, margins, element columns, whitespace, page numbers and robust pagination | Generally no scene numbers, production header, revision marks or confidentiality footer unless requested |
| Production / shooting draft | Same core layout correctness and element spacing | Scene numbers, draft labels, production headers, revision marks, and optional scene-continuation labels |
| Sides | Same core element rules, dialogue layout, spacing and keep-together behavior for selected scenes | Selected-scene ordering, day/draft header, optional sides cover, page numbering and production branding |

The Sides cover is a distinct layout from the screenplay title page and must be tested separately. Shared engine fixes should benefit both output types; cover-page fixes only benefit the relevant cover component unless intentionally shared.

## 9. Minimum regression-test matrix

Implement automated tests for the layout model and visual inspection fixtures for generated PDFs. Cover all of these cases:

### Element spacing
- action → action;
- action → character cue → dialogue;
- cue → parenthetical → dialogue;
- dialogue → next cue;
- action/dialogue → transition → scene heading;
- direct scene change without a transition;
- scene heading → action;
- shot heading → description.

### Pagination
- scene heading close to bottom margin;
- character cue close to bottom margin;
- parenthetical close to bottom margin;
- dialogue fitting on one page versus genuinely requiring a split;
- long action paragraph split across a page;
- transition near bottom margin;
- explicit page break;
- several consecutive short scenes;
- footer/header enabled and disabled;
- title page included and omitted.

### Script content
- repeated speakers in separate speeches;
- dialogue interrupted by action;
- a character speaking again in a later scene;
- existing `(CONT'D)` labels on imported text;
- dual dialogue;
- multiline character names and long parentheticals;
- long unbroken words, punctuation and accented characters;
- non-Latin/Indic dialogue with shaping and correct line wrapping.

### Output profiles
- clean screenplay;
- shooting draft with scene numbers and headers;
- Sides with several selected scenes;
- Sides with and without a cover page;
- US Letter and A4.

For each test, check semantic text order, bounding-box collisions, page dimensions, expected element coordinates/gaps, and rendered page images. A green build alone is not evidence of correct PDF layout.

## 10. Implementation and acceptance criteria

1. Inspect the existing engine and identify which current settings already satisfy this spec. Do not replace working code merely to mimic another application.
2. Implement the selected default profile through one source of truth for element rules and spacing. Avoid page-specific hardcoding.
3. Preserve source screenplay content and semantic element types. Do not rewrite, delete, reorder, paraphrase, or silently reclassify text. Pagination changes are acceptable if source text remains present and in order; generated `(MORE)` / `(CONT'D)` markers must not be written back into screenplay source data.
4. Change only demonstrated deviations from the selected profile; record legitimate alternatives as explicit profile settings.
5. Verify title page, representative body pages, transitions, page breaks, dialogue continuation, and last page by rendering fresh PDFs. Also apply the necessary changes to Sides and verify a fresh Sides PDF, including selected-scene order, running header, page numbers, branding/footer, and the optional Sides cover page.
6. Compare measured positions as well as the actual visual result. Keep fonts, line leading, wrapping and pagination in sync.
7. Report the deviations found, changes made, tests performed, before/after measurements, visual checks, and remaining limitations.

## Sources and interpretation

This specification synthesizes common US screenplay conventions described in screenwriting-format references, including Final Draft's general screenplay-format guidance and element/pagination guidance, Screenplay.com's basic formatting reference, and the Writers Guild Foundation's screenplay primers. These references support broad conventions such as 12-point Courier, conventional margins and element indentation, uppercase scene headings, distinct character/dialogue elements, and keeping headings with following material. Exact paragraph spacing, first-page numbering, headers, scene numbers, revisions and cover-page details can vary by workflow and house style.

- Final Draft — How to Format a Screenplay: https://www.finaldraft.com/learn/how-to-format-a-screenplay/
- Final Draft — Script elements and pagination: https://kb.finaldraft.com/hc/en-us/articles/27646947570196-What-are-script-elements
- Final Draft — Scene-heading spacing: https://kb.finaldraft.com/hc/en-us/articles/15575250899732-Should-there-be-two-blank-lines-between-transitions-and-scenes
- Screenplay.com — Basic Screenplay Format: https://screenplay.com/pages/basic-screenplay-format
- Writers Guild Foundation — Screenplay Primers: https://www.wgfoundation.org/screenplay-primers
