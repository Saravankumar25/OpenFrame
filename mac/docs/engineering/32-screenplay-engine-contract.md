# 32: Screenplay engine contract (proposed)

> Status: **proposed**. Approval comes first (Project controls and baseline), then the build (Screenplay engine and file formats). Product behaviour: Revision 1 §§4 to 20, 36 to 42, 45, 50, 51.
> Decisions: ADR-0015 (numbering), ADR-0016 (fonts). Features in the feature status file: SCRIPT-*, XFER-*.
> Principle: **one reconciled contract, built once.** Do not implement the old manual version and rebuild it.

## 1. Verified current state

| Area | Current | Evidence |
|---|---|---|
| Document model | `screenplay` → `screenplay_draft` → `screenplay_scene` (id, lineage_id, position, heading, omitted) → `screenplay_element` (id, type, text, dual, revision_mark) | `migrations/project/0004_screenplay.sql` |
| Element types | Fixed `CHECK`: scene_heading, action, character, dialogue, parenthetical, transition, shot, note | same |
| Title page | `screenplay.title_page_json` | same |
| Draft lock and revision | `status` Draft, Review, Locked, Revision; `locked_at`, `revision_label`, `revision_color`, `revision_reason`, `locked_by` | same |
| Scene numbers | Not stored, derived from position | ADR-0011 |
| Editor | ProseMirror-style editor with `editor/rules.ts` (next-element suggestion), `schema.ts`; browser spellcheck on | `apps/desktop/src/workspaces/screenplay/editor/` |
| PDF layout | `layout.rs` (868 lines): pagination, MORE and CONT'D, scene numbers, headers | `crates/openframe-import-export/src/layout.rs` |
| Formats | `fdx.rs`, `fountain.rs`, `docx.rs`, `pdf_import.rs`, `text.rs`, `heuristics.rs`, `report.rs` | same crate |
| Character coverage | PDF fails outside WinAnsi | ADR-0008 |

Unknown until verified in the first build step ("Verify editor pagination", section 3): how the editor paginates on screen and whether it shares `layout.rs` rules.

### 1.1 Built 2026-10-07 (engine, writing features, layout)

| Part | Where | Notes |
|---|---|---|
| Migration | `migrations/project/0013_screenplay_engine.sql` | `screenplay_custom_element`; element rebuild with `page_break` and `custom` types, `custom_element_id`, `import_source`; `production_number(_source)` on scenes; `screenplay.page_setup_json`; `screenplay_page_preset`; `screenplay_revision`; `screenplay_lock_baseline`. Ids, revs and timestamps kept. |
| Element rule table | `openframe-import-export/src/rules.rs` | One table for layout positions, case, spacing, keep-together and split flags, Enter progression, FDX type and Fountain form. The editor reads the generated `apps/desktop/src/workspaces/screenplay/editor/elementRules.json` (test `element_rules.rs`). |
| Page geometry | `layout.rs` `PageSetup`, `Geometry` | US Letter and A4 (same 1.5" to 7.5" columns; A4 gains 4 lines), Courier 10 to 14 pt, single, 1.5 or double spacing, first page number, header (left) and footer (centred) with `{title} {draft} {revision} {date} {page}`, first-page rule. Defaults reproduce the earlier layout. |
| Keep-together | `layout.rs` | Heading kept with the next block's first lines (speech: cue and two lines; non-splittable blocks whole); speech split only after a dialogue line, cue and two lines before, two lines after; action split leaves two lines on each page. |
| Continuation | `layout.rs` | `(MORE)` and `CUE (CONT'D)` only for one speech split by a page boundary; the cue keeps its extension (`MEERA (V.O.) (CONT'D)`). |
| One layout | `interchange/export.rs::print_preview`, `screenplay/pages.rs` | Print preview, the editor's page view (`screenplay.page_view`), page facts (`screenplay.pagination`) and the PDF all call `layout::layout` with the same options. Agreement tests compare preview lines with the PDF's text positions. |
| Numbers, lock, revisions | `screenplay/numbering.rs`, `pages.rs`, `drafts.rs` | Labels derived until frozen; lock freezes them and records the page baseline; inserted scenes get 12A, 12B; deleting a numbered scene of a revision keeps it as OMITTED; revisions carry a dated record and list changed pages and scenes. |
| No branding | `pdfgen.rs::finish_unbranded`, `docx.rs` | Screenplay PDF and DOCX metadata name only title and author. |

### 1.2 Several screenplays per project (built 2026-10-08)

Owner decision of 2026-10-08 (ADR-0012 amendment): a project, or an episode in a series, may hold several screenplays.
No migration was needed: nothing in the schema limited a scope to one `screenplay` row.

| Part | Where | Notes |
|---|---|---|
| Scope list | `screenplay/mod.rs::screenplays_in_scope` | Every live screenplay of the project or episode, oldest first. Replaces the old `screenplay_for_scope` (`LIMIT 1`). |
| Overview | `screenplay.overview` (`screenplayId` argument, `screenplays` list) | Shows the screenplay asked for; a stale id falls back to the oldest. The picker always names the open one. |
| Create and rename | `screenplay.create`, `screenplay.rename` (`drafts.rs`) | No guard on a second screenplay. A typed title already used in the scope is refused (`validation.duplicate`); a default title gets a number such as "Black Rain (2)". Rename also sets the title page title; search follows through the screenplay row's search dependents. Both are one undo step and need Edit. |
| Delete and restore | `screenplay.delete`, trash handler `screenplay` (`delete.rs`) | Owner decision of 2026-10-08. Moves a whole screenplay to Recently Deleted for 30 days (SoftDelete: Owner and Editor; one transaction, one undo step; the store lock refuses it). Only the `screenplay` row is marked deleted; drafts, locked drafts, scenes, production numbers, page setup, custom elements, character links and Story Board links stay as they are and come back on restore. While deleted: `load_draft` and `drafts_of` require a live screenplay, so its drafts can neither be read nor edited; pickers, import targets, package sources, the AI draft list, character counts and Story Board links skip it; search drops its scenes. Refused while one of its drafts is the active Production Source ("Choose a draft of another screenplay as the Production Source first"). The last screenplay may be deleted; the project then has none, like a new project. A restore whose title was taken meanwhile gets a numbered title and a `restoredNote`; a screenplay of a deleted episode waits for the episode, and a deleted draft of a deleted screenplay waits for the screenplay. Permanent removal (by hand or by the 30-day purge) removes every draft, scene, comment, history point, custom element and character link, and is refused while production planning refers to the screenplay (an earlier Production Source, breakdown rows, schedule strips or sides); it then stays in Recently Deleted. Hidden from the assistant (`hidden::WHOLE_SCREENPLAY`); the assistant may propose restoring it. |
| Search dependents | `Registry::search_dependents`, `store.rs::reindex_rows` | A change of a `screenplay` row re-indexes the scenes of its current draft, in a mutation, an undo, a redo or a package apply, so search never shows a deleted screenplay and names a renamed one. |
| Per screenplay | drafts, locks, revisions, production numbers, page setup, custom elements | All already belonged to a draft or a screenplay row; tests prove a lock, a scene number or A4 in one screenplay leaves the other unchanged. |
| Links | `scene_nav`, `draft_nav`, `screenplay.locate` | Carry `screenplayId`, so search, comments and other workspaces open the right screenplay. Search context reads "Screenplay · title · draft". |
| Story Board | `story.build_screenplay`, `story.order_preview` | New Screenplay is allowed beside existing ones; the order preview lists every screenplay and the dialog lets the writer choose which draft to reorder. |
| Import | `interchange/import.rs` | New Screenplay is always offered (in a series: any episode); New Draft needs `screenplayId` when there are several. |
| Packages | `packages/exchange.rs::current_draft(c, screenplay_id)`, `only_current_draft` | A package without a draft uses the only screenplay; with several it asks for a draft. A review package comes back only to its own screenplay. The Production Source label names the screenplay. |
| AI | `ai/queries.rs`, `ai/catalog/screenplay.rs`, `ai/catalog/story.rs`, `ai/scope.rs` | Draft labels name the screenplay when there are several ("Draft 2 of “The Short”"), references may name it, "the current draft" without an open screenplay is a question, `propose_rename_screenplay` is new, and Build Screenplay takes a `screenplay` argument. |
| Frontend | `workspaces/screenplay/picker.tsx`, `index.tsx`, `workspaces/trash/index.tsx` | Picker (switch, New Screenplay, Rename Screenplay, Delete Screenplay) at the left of the toolbar; the open screenplay is kept in the saved view state and the route (`screenplayId`). Delete asks first, names the screenplay, says it goes to Recently Deleted for 30 days, shows the core's reason when it is refused, then opens the next screenplay. Recently Deleted lists screenplays and restores them; a draft whose screenplay is also deleted says to restore the screenplay first. |

## 2. Target contract

### 2.1 Canonical data (one additive project migration, next free number)

> Numbering updated 2026-10-05: project `0011` is taken by the ADR-0019 apply journal (crash-safe AI changes) and the Story Board long scene description takes the
> next one. This contract takes the next free number when the screenplay schema migration starts (doc 23 §4, "Allocation").

- **Element kind**: replace the closed list with a reference to `element_kind` (built-ins plus custom elements).
  Table rebuild preserving `id`, `rev`, `created_at`, `updated_at` (doc 23 §4 rule 3). New columns on elements:
  `kind_id`, `import_source` (opaque, for round trip), `custom_name`.
- **`custom_element`**: id, name (unique), shortcut (nullable), display preset, spacing preset, next element, import/export identity.
  At most **64 custom elements per screenplay** (`MAX_CUSTOM_ELEMENTS` in `modules/screenplay/setup.rs`). An import
  with more kinds keeps the first 64 as custom elements and imports the rest as action with an attention warning
  (`custom_elements_over_limit`) so they are reviewed. The FDX parser reads at most 256 `ElementSettings`
  (the first settings of a type win), keeps at most 256 distinct custom paragraph types per file (paragraphs of
  further types are imported as action with the `fdx_custom_types_over_limit` warning), and cleans every type
  name once (control characters removed, trimmed, at most 60 characters), so types that only differ past the
  limit are one identity. Importing a new draft into a screenplay with too little room left is never
  refused: the kinds that fit become custom elements and the paragraphs of the others are imported as
  action, with their text, and the `custom_elements_no_room` warning names how many paragraphs and which
  kinds (first 20). (Security review 2026-10-07: bounded time and memory for hostile files; tests
  `security_parsers.rs::fdx_custom_style_flood_is_bounded_deduplicated_and_keeps_every_line`,
  `screenplay_engine.rs::hostile_fdx_with_thousands_of_look_alike_custom_styles_is_bounded`,
  `::a_file_with_more_custom_styles_than_fit_imports_with_every_line_and_a_warning`.)
- **Page break directive**: an element kind `page_break` with no text; never blank paragraphs.
- **Production number** (ADR-0015): `screenplay_scene.production_number` nullable, `production_number_source`
  (`derived|imported|inserted`), frozen at lock.
- **Revision record**: `screenplay_revision` (id, draft_id, label, colour, source_draft_id, date, reason) and a
  derived `changed_scenes`, `changed_pages` list computed from the locked baseline pagination snapshot
  (`screenplay_pagination_baseline`: draft_id, page → element ids hash).
- **Page setup and header/footer**: per screenplay: paper size (Letter, A4), font, size, line spacing,
  page-number start, title-page behaviour, header text, footer text, first-page rule, scene-number behaviour,
  revision display. Defaults reproduce today's output exactly.
- **Title page fields**: title, author, contact name, email, phone, address, draft name, revision label, date, optional representation block.
- No flattening: every import construct is stored, mapped or flagged; `import_source` keeps the original token.

### 2.2 Element rule profiles

One table drives editor display, layout and FDX/Fountain mapping. Each profile: display style, indent and margin
preset, alignment, spacing before and after, wrap rule, next-element rule, page-break rule, continuation rule,
import and export mapping, production semantics. Built-ins: Scene Heading, Action, Character, Parenthetical,
Dialogue, Transition, Shot, General Note (outside clean output by default).

### 2.3 Layout service

`ScreenplayDocument → normalize → page layout model → page breaks → continuation → scene and page numbers →
revision overlay → renderer (screen preview, print preview, PDF, DOCX)`.
Deterministic: same document state and settings produce the same pages. Keep-together: scene heading with the
following block, character cue with its dialogue, parenthetical attached, orphan lines handled consistently.
Continuation (MORE and CONT'D) is generated from "one dialogue block split by a boundary", never typed text.
Text metrics come from the same shaped fonts as the renderer (ADR-0016).

### 2.4 Interchange mapping

For each format, a written fidelity table: preserved, mapped, warned, dropped-with-report. FDX preserves
elements, custom elements, scene numbers, title data, revisions where supported, and page breaks where supported.
Unknown constructs: preserve as custom, map explicitly, or review; never default to Action. DOCX and PDF are
compatibility paths and never claim exact structure.

Built 2026-10-07 (tests: `fdx_suite.rs` thirteen classes, `multilingual.rs`, `fountain.rs`, `docx.rs`, `pdf.rs`,
application `screenplay_formats.rs`):

| Construct | FDX import | FDX export | Fountain | DOCX | PDF import | Plain text |
|---|---|---|---|---|---|---|
| Built-in elements | preserved | preserved | preserved (forced markers where needed) | mapped from styles, else indentation | best effort from indentation, confidence shown | heuristics, warnings |
| Text of headings, cues, transitions | preserved as written | written as stored (Final Draft shows capitals) | preserved | preserved | as printed (capitals) | as typed |
| Custom elements | preserved with identity and look (`ElementSettings`) | preserved with `ElementSettings` | written as action, warned (`custom_as_action`) | written with the look's style | not detected | not detected |
| Scene numbers | kept as production labels | stored labels written | `#12A#` both ways | printed, stripped back on import | margin numbers stripped | not kept |
| Title page | preserved (title, credit, authors, source, draft, contact) | preserved | preserved | preserved | best effort | not kept |
| Revisions | marks kept with their set names | sets and marks written | dropped, warned | asterisk only | asterisks stripped | not kept |
| Explicit page breaks | `StartsNewPage` | `StartsNewPage` | `===` | page break | not detected | not kept |
| Dual dialogue | preserved (inside a paragraph or straight under Content) | `DualDialogue` | `^` | table, read back | not detected | not kept |
| Notes | attached script notes kept as notes, reported; ScriptNotes section reported | left out unless notes are included (then visible `[Note: …]`) | `[[…]]` | left out | not detected | not kept |
| Indian scripts, joiners | every character | every character | every character | every character, script font and language hints | every character (ActualText) | every character |
| Unknown blocks and settings | unknown blocks named in a warning; layout settings reported as not used | none written | boneyard, sections and synopses mapped or reported | Word formatting is never structure | page furniture stripped | none |

Embedded fonts (ADR-0016): Courier Prime and Noto Sans Devanagari, Telugu, Tamil, Malayalam and Kannada ship in
`crates/openframe-import-export/fonts/` with `fonts.json` (version, size, SHA-256, licence; checked by
`fonts_manifest.rs`). A script written only in Windows-1252 characters prints with base-14 Courier, byte for byte
as before; anything else switches the screenplay PDF to embedded subsets shaped with rustybuzz. Report and
production document PDFs keep Helvetica for Windows-1252 runs and draw other runs with the bundled fonts. Noto
faces print at 0.9 of the page size so stacked marks keep clear of the neighbouring lines on the fixed grid; layout
and drawing use the same shaped widths. Characters no bundled font covers (emoji, Chinese) are a clear export error
and nothing is written. The bundled fonts add about 0.8 MB to the installer.

### 2.5 Operations and AI

Built: the read tool `screenplay_pages` covers `screenplay.pagination`, `screenplay.page_setup`,
`screenplay.custom_elements` and `screenplay.revision_pages`. Page setup, presets, custom element definitions, scene
number overrides, renumbering and restoring omitted scenes are hidden from the assistant (`PAGE_LAYOUT`, `NUMBERING`).
Edits in a Revision draft mark the changed elements with the revision label automatically.

Every new operation is registered with `OperationMetadata`. AI read tools expose page and scene facts; proposal
tools may edit text, headings and notes through Change Sets; layout settings, locking, numbering overrides and
revisions are human-only unless the owner decides otherwise. Retrieval projections cover scenes and elements.
Search documents cover headings, characters and notes with private-note filtering.

## 3. Order of build inside the Screenplay engine and file formats area

1. Verify editor pagination and `layout.rs` sharing; write the gap list.
2. The additive migration (next free number, doc 23 §4) and operations (no UI).
3. Rule profiles and layout parity (preview equals PDF).
4. Numbering, lock and revision records.
5. Title page, header and footer, page setup, page break, custom elements UI.
6. Fonts and shaping (ADR-0016) with the multilingual golden corpus.
7. FDX, Fountain, DOCX, PDF conformance and the 13-class fixture suite.
8. E2E and the Revision 1 §50 and §51 gates.

## 4. Acceptance

Revision 1 §51 (21 criteria) and §50 (11-step migration definition of done), plus: preview page count equals PDF
page count on every fixture; performance within doc 21 budgets for a feature-length script; export failure leaves the
project and the source file untouched.
