# FSD Engineering Digest — Part 2 (§52 → end, Appendices A–G, §86–§177)

Source: `OpenFrame_Studio_Functional_Specification_Document_Aligned_Updated.md`, lines 2100–5809. `Lnnnn` = source line.

**Legend**
- `[V1]` = spec says mandatory / must / P0 / release-blocker. `P1` = spec marks P1 (release membership not defined by the spec).
- `[LATER]` = spec defers, marks optional at feature level, or hedges ("may", "if implemented", "where supported", "if present").
- "opt" on a field = optional input, not a deferred feature.
- Quoted strings are exact default UI copy from the spec.
- Requirement IDs are verbatim. **Warning:** §68–70 and §171 reuse the same IDs for *different* requirements (see Contradictions).

---

## §52 Delete, Archive, Restore, Destructive Safety (L2100)
- Soft delete: important creative/production objects go to a recoverable deleted state. [V1]
- Permanent delete needs a second deliberate action and a clear warning.
- No automatic cascade to downstream objects unless the user explicitly confirms the cascade. For example, deleting a Story Board Scene Card never deletes the screenplay scene built from it.
- Use Archive (preferred) for reusable catalog items, projects and people. Use Delete for accidental or unwanted records.
- Restore puts the object back in its previous container and order where possible. If the container is gone, the object goes to an **Unassigned** area and the user is notified.

## §53 Script-to-Production Source Rules (L2122)
- Production breakdown is created from a chosen screenplay draft/revision. The source version is shown at the top of Breakdown.
- The user can mark a source as the **Production Script Version** (production snapshot).
- When a newer revision is approved, the user can choose **Update Production Source**.
- Update compares old and new sources and reports: added scenes, removed scenes, changed scenes, moved scenes, changed headings, changed page/length info.
- Reconciliation: production data stays attached to stable scene identity when the match is unambiguous. New or substantially changed scenes require production review.
- The app never silently removes props, cast, shots, storyboard panels or location info because the script changed. [V1]
- Changed scenes can appear in a simple **"Needs Review"** list.

## §54 Breakdown Reconciliation After Script Changes (L2153)
- **Scene deleted:** its breakdown stays in historical state, marked **"Scene removed from current source."** The user may archive it.
- **Scene added:** appears as **"Needs Breakdown."**
- **Heading changed:** warn that the location/time description changed. Existing production associations stay until reviewed.
- **Text changed, identity same:** breakdown stays. Auto-suggest may find new likely elements. Existing elements are never silently removed.
- Impact summary on a changed scene (example strings):
  - "New elements suggested: 3"
  - "Existing elements requiring review: 2"
  - "Shot list exists: Yes — review recommended"
  - "Storyboard exists: Yes — review recommended"
- Must not become an analytics dashboard.

## §55 Shot List and Storyboard Reconciliation (L2178)
- Scene unchanged: visual planning unchanged.
- Scene text changed: visual planning stays, with indicator **"Scene changed since planning"**.
- Scene removed: shot/storyboard records are kept historically and hidden from active production views unless the user chooses to inspect removed content.
- Scene duplicated: may be treated as a new scene identity. Visual planning is not duplicated unless the user chooses **Copy Planning**.
- Scene reordered: shots/storyboards follow scene identity, not the physical scene number. Display numbering updates automatically.

## §56 Schedule Reconciliation After Script Changes (L2196)
- Scene still exists: schedule assignment stays unless the source mapping becomes ambiguous.
- New scene enters **Unscheduled**.
- Removed scene is marked in schedule history. It leaves the active future schedule only after user confirmation.
- Changed page/time estimate: the schedule shows the updated estimate and flags a day if its total is affected.
- A call sheet prepared from a changed day is marked **"Needs Refresh."** The existing exported PDF is not overwritten.

## §57 Daily Production View (L2214) — FSD-PROD-022, P1
- Contents: date, day number, scenes, locations, cast, estimated duration, key notes, call sheet status, important breakdown items.
- Navigation: scene → Scene/Breakdown; cast → Cast; location → Location; call sheet → Call Sheet editor.
- Lightweight day-specific edits are allowed. They must not bypass the source rules of the underlying modules.

## §58 Basic Production Reports and Sides (L2240)
- Reports ("may provide"): scenes by location, cast scene list, props by scene, shooting schedule, breakdown summary, shot list. (FSD-PROD-023, P1)
- Sides (P1, FSD-PROD-024): user selects a shooting day → **Generate Sides**. Output contains only that day's screenplay scenes, with optional call sheet cover info.
- Sides are snapshots from the selected screenplay source. Editing them outside the app does not change the screenplay.
- Export: PDF required.

## §59 Templates (L2266)
- Template types: Project starter, Moodboard starter, Story Board starter, Call sheet template, Shot list template, Breakdown document template, Simple budget template.
- Users can save a configured document or workspace as a template.
- Isolation: changing a template never changes projects already created from it.
- Template marketplace is out of core scope. [LATER / out of scope]

## §60 Export and Printing (L2290)
- Every major workspace needs an obvious **Export** action. [V1]

| Workspace | Formats |
|---|---|
| Screenplay | PDF / FDX / Fountain / DOCX |
| Outline/Story Board | PDF |
| Moodboard | PDF / image presentation |
| Breakdown | PDF / report |
| Shot List | PDF |
| Storyboard | PDF / image sheet |
| Schedule/Stripboard | PDF |
| Call Sheet | PDF |
| Sides | PDF |
| Project | portable project package |
| Review | exchange package |

- Scope options where practical: current item, selected items, current section, entire project document.
- Print uses the same document-generation logic as PDF.

## §61 Empty States (L2330) — each needs a purpose message plus 1–2 actions
| Workspace | Message (exact) | Primary action |
|---|---|---|
| Idea Vault | "This is where you throw anything about the film. Add a note, image, link, file, or voice note." | Add |
| Story Board | "Start shaping the story. Add an Act, Sequence, Beat, or Scene Card." | Add Scene |
| Screenplay | "Write your screenplay or import one you already have." | New Screenplay / Import |
| Breakdown | "Choose a screenplay draft to begin breaking down scenes." | Select Source |
| Schedule | "Create a shooting schedule from your screenplay." | Create Schedule |
| Call Sheets | "Call sheets will appear here after you create shooting days." | Open Schedule |

## §62 Error and Recovery (L2365)
- Every error must explain in plain language, preserve work, give a next action, and avoid internal jargon.
- **Save failure:** "OpenFrame could not save the latest changes to the project." Actions: **Retry Save**, **Save Backup Copy**, **Keep Editing**.
- **Import failure:** show the source file, the failure stage, whether a partial preview is available, and whether the source file is untouched.
- **Export failure:** the project is unchanged. The user can retry to another destination.
- **Missing linked file:** show a relink action and project path info. Never delete the reference.
- **Collaboration disconnect:** show that the live session ended, then return to local project state.

## §63 Source-of-Truth and Connection Matrix (L2404) — mandatory [V1]
| From | To | Relationship | Automatic? | User control |
|---|---|---|---|---|
| Global Idea Vault | Project Idea Vault | Copy | No | User initiates |
| Project Idea Vault | Story | Copy/convert | No | User initiates |
| Story Board | Screenplay | Build/copy structure | No | User initiates |
| Screenplay | Breakdown | Source snapshot/referenced source | User initiates | Yes |
| Screenplay | Shot List | Scene-based creation | User initiates | Yes |
| Screenplay | Storyboard | Scene-based creation | User initiates | Yes |
| Breakdown | Catalog | Reuse/create production identity | Suggest/confirm | Yes |
| Catalog | Schedule | Scene/day requirements | Derived | Yes |
| Schedule | Call Sheet | Document prefill | Generated by user | Yes |
| Storyboard | Shot List | Optional reference reuse | User initiates | Yes |
| Shot List | Call Sheet | Optional reference/notes | User initiates | Yes |

**§63.1 Connections that must NOT exist:**
- Idea Vault ↔ screenplay live sync.
- Story Board live sync with every screenplay edit.
- Call Sheet edits rewriting the schedule.
- Moodboard edits modifying scene production data.
- Shot changes rewriting the screenplay.
- Catalog edits rewriting script prose.
- AI suggestions becoming data automatically.

## §64 Walkthrough — First-Time Filmmaker (L2434) — canonical QA flow
1. Create `BLACK RAIN` as Feature Film → Project Home opens.
2. Add text note, image, URL, PDF and voice note → all appear in Project Idea Vault with no mandatory classification.
3. Create Act 1, Sequence `Hero Introduction` and Scene Cards → cards are small rectangles with no manual numbers.
4. Drag Card 5 before Card 3 → order changes; undoable.
5. Give cards valid headings → **Build Screenplay** → screenplay in card order, scene numbers from screenplay order.
6. Create Draft 2 → Draft 1 unchanged.
7. Reviewer comments on selected text → comments retained and resolvable.
8. Lock Draft 6 → editing requires **Start Revision**.
9. Choose the Locked Draft as production source → scene list populated; manual tagging and confirming suggestions work.
10. Confirmed props, locations and cast become reusable catalog items.
11. Create a shot list and storyboard for selected scenes.
12. Create shooting days and drag scene strips onto them.
13. Schedule the same actor in incompatible locations → advisory conflict warning.
14. Generate a call sheet from Shoot Day 3 → scenes, location and cast prefilled.
15. Export screenplay, shot list, schedule and call sheet PDFs → snapshots created; project unchanged.

## §65 Walkthrough — Existing Screenplay (L2508)
- Must not force the user through Idea Vault or Story Board.
- Flow: create/open → import PDF/FDX/Fountain/TXT/DOCX/paste → review import preview → import as new screenplay/draft → Breakdown → confirm/suggest elements → build Catalog → add Locations/Cast/Crew → Shot List/Storyboard → Schedule → Call Sheets.
- The Story Board stays optional for reference.

## §66 Walkthrough — Short Film (L2527)
- A short film may use only: Idea Vault → Story Board → Screenplay → Breakdown → Shot List → Schedule → Call Sheet.
- Never force: budget, character profiles, moodboards, storyboard, complex schedule metadata, episodic structures.
- Unused modules stay accessible but visually quiet.

## §67 Walkthrough — Episodic (L2545)
- Series → Season → Episodes. Each episode has its own Story Board and screenplay.
- Series-level recurring character/location references.
- Break down selected episode scripts and schedule as needed.
- Season navigation is simple and episode-centric. No giant season-room database.

## §68 Acceptance Matrix — Core Creative (L2559) — all P0
- FSD-IDEA-001 — Create text Vault item with no metadata. Verify: create note, save.
- FSD-IDEA-002 — Add image, URL, PDF, document, audio, video, screenshot, sketch and ordinary file items. Verify: one test per type.
- FSD-IDEA-003 — Global and project Vaults are independent. Verify: create global item, copy to project, edit copy.
- FSD-IDEA-004 — Move to Story creates a copy; original stays. Verify: convert, then edit destination.
- FSD-STORY-001 — Acts contain sequences and/or scenes. Verify: create and move.
- FSD-STORY-002 — Sequence is a simple named container. Verify: one name field.
- FSD-STORY-003 — Scene cards are compact, show short description, no manual numbering. Verify: create/reorder 20 cards.
- FSD-STORY-004 — Dragging changes order and is undoable. Verify: drag, then undo.
- FSD-STORY-005 — Parking Lot preserves cards. Verify: move, close/reopen, restore.
- FSD-SCRIPT-001 — Required screenplay element types supported. Verify: write sample.
- FSD-SCRIPT-002 — Build Screenplay creates scenes from selected cards. Verify: order and headings.
- FSD-SCRIPT-003 — No live sync from Idea Vault. Verify: edit note, script unchanged.
- FSD-SCRIPT-004 — Named drafts preserve prior versions. Verify: multiple drafts, compare.
- FSD-SCRIPT-005 — Comments can be created and resolved. Verify: add, reply, resolve.
- FSD-SCRIPT-006 — Locked script cannot be edited without starting a revision. Verify: lock, attempt edit.

## §69 Acceptance Matrix — Production (L2580)
- FSD-BRK-001 P0 — Breakdown from a selected screenplay version.
- FSD-BRK-002 P0 — Manual tagging creates scene-specific associations (prop/location/cast).
- FSD-BRK-003 P0 — Auto suggestions require confirmation (accept/reject/edit).
- FSD-CAT-001 P0 — Confirmed elements are reusable as catalog items (same prop in 2 scenes).
- FSD-LOC-001 P1 — Locations display scene usage.
- FSD-CAST-001 P1 — Cast records connect actors → characters → scenes.
- FSD-SHOT-001 P0 — Shots are ordered by drag/drop and tied to scenes.
- FSD-STB-001 P1 — Storyboard panels can be associated with shots/scenes.
- FSD-SCH-001 P0 — Scenes enter Unscheduled when a schedule is created.
- FSD-SCH-002 P0 — Scenes can be dragged into shooting days.
- FSD-SCH-003 P1 — Basic conflicts are displayed without forced auto-correction.
- FSD-CALL-001 P0 — Call sheet generated from shooting day data.
- FSD-CALL-002 P0 — Call sheet changes never silently rewrite the schedule.

## §70 Acceptance Matrix — Offline and Collaboration (L2599)
- FSD-OFF-001 P0 — Core workflows available with the network disconnected.
- FSD-OFF-002 P0 — Local save is the source of truth (edit/save/reopen offline).
- FSD-COL-001 P0 — Export a portable exchange package; inspect it in another installation.
- FSD-COL-002 P0 — Review package import does not blindly overwrite the host (import stale response).
- FSD-COL-003 P1 — Local-network session works without OpenFrame cloud (host/join on LAN).
- FSD-COL-004 P1 — Same-object conflicts are surfaced, not silently lost.
- FSD-AI-001 P1 — AI optional; core works with AI disabled.
- FSD-AI-002 P1 — AI modifications require user confirmation (ask AI to create a card).

## §71 Data Mutation Rules (L2613) — complete
| Action | Type | Default safety |
|---|---|---|
| Add Idea | Additive | Immediate save |
| Move Idea to Story | Copy | Original retained |
| Move Scene Card | Mutation | Undo available |
| Duplicate Scene Card | Create new object | Original unchanged |
| Build Screenplay | Create draft | Existing drafts untouched |
| Edit Draft | Mutation to current draft | Auto history retained |
| Create Named Draft | Copy/version | Parent retained |
| Lock Draft | State change | Explicit confirmation |
| Start Revision | Create new revision | Locked source retained |
| Add Breakdown Element | Additive/link | Undo available |
| Remove Breakdown Element | Association removal | Catalog item retained |
| Delete Catalog Item | State change/archive preferred | Scene associations protected |
| Schedule Scene | Assignment | Undo available |
| Edit Call Sheet | Document mutation | Schedule unchanged |
| Export | Snapshot | Project unchanged |
| Import Exchange | Add/update after preview | No blind overwrite |

## §72 State and Status Rules (L2637) — complete, verbatim
- **Project:** `Active / Archived / Closed`
- **Idea Vault Item:** `Active / Pinned / Deleted`
- **Story Card:** `Active / Parking Lot / Deleted`
- **Draft:** `Current / Named / Locked / Superseded / Deleted`
- **Review Comment:** `Open / In Discussion / Resolved`
- **Breakdown Element:** `Suggested / Confirmed / Rejected / Removed`
- **Catalog Item:** `Required / Searching / Shortlisted / Confirmed / Not Required / Archived`
- **Location:** `Idea / Shortlisted / Confirmed / Rejected`
- **Shooting Day:** `Planned / Ready / Completed / Archived`
- **Call Sheet:** `Draft / Final / Superseded / Archived`
- Statuses must be human-readable and must not expose internal implementation states.
- The spec lists no transition graph. Transitions implied elsewhere:
  - Draft: Current → Named via Create Named Draft; → Locked via Lock (explicit confirm); Locked stays unchanged; Start Revision creates a new writable doc.
  - Comment: Open → Resolved via Resolve (never deleted); Reply does not resolve automatically.
  - Breakdown Element: Suggested → Confirmed (Accept) or Rejected (Reject); Confirmed → Removed (remove association; catalog kept).
  - Story Card: Active ↔ Parking Lot (drag); → Deleted (soft) → restore.
  - Location: starts Idea or Shortlisted → Confirmed after real-world confirmation.
  - Call Sheet: Draft → Final ("Final/Issued") → Superseded by a new revision on refresh/reissue.
- Other status sets defined elsewhere:
  - Task: `Open / Done` (§106/§158).
  - Research verification: `Unchecked / Checked / Needs Expert Check` (§107).
  - Collab session: see §119.
  - Screen-level visible states: see §136.

## §73 Cross-Module Traceability (L2672)
- Idea → Story: a copied Vault item becomes a Story object; the original stays intact.
- Story → Screenplay: selected Scene Cards create screenplay scenes. Card order drives creation order; scene numbers are derived later.
- Screenplay → Breakdown: the chosen draft/revision is the production source. Scene identities carry production data across revisions where mapping is clear.
- Breakdown → Catalog: confirmed items are reusable across scenes.
- Screenplay → Shot/Storyboard: visual planning starts from scenes without writing camera directions into the script.
- Catalog + Screenplay → Schedule: shows scenes plus required locations, cast and practical data.
- Schedule → Call Sheet: prefilled, but remains a snapshot document.
- Every exported document or package is a snapshot. Re-import behavior is defined per package/document type.

## §74 Feature Access by Project Type (L2701) — "Optional" = available but never forced
| Feature | Feature film | Short | Episodic |
|---|---|---|---|
| Idea Vault, Story Board, Screenplay, Drafts/Versions, Shot List | Yes | Yes | Yes |
| Acts, Sequences, Beats | Yes | Optional | Yes |
| Breakdown, Schedule, Call Sheets | Yes | Optional | Yes |
| Storyboard, Budget Snapshot | Optional | Optional | Optional |
| Season/Episode | No | No | Yes |

## §75 UX-Level Simplicity Requirements (L2723)
- One obvious primary action per workspace:
  - Idea Vault → **Add**
  - Story Board → **Add Scene / Add Sequence**
  - Screenplay → **Write / Import**
  - Breakdown → **Tag / Suggest**
  - Production → **Choose workspace**
  - Schedule → **Create Shooting Day**
  - Call Sheet → **Create / Update**
- Progressive disclosure: advanced fields sit behind Details/Edit/More. Default forms stay small.
- No terminology gate. No forced workflow: import bypasses Vault/Board, and a user can develop for months with no schedule.
- Story Board and Stripboard may be dense, but each card/strip stays scannable. No spreadsheet walls.

## §76 QA Test Layers (L2755)
- Four layers: (1) feature behavior, (2) state transitions, (3) cross-module identity survival, (4) recovery/safety (undo, delete/restore, import/export, backup, conflict).
- **Every P0 requirement needs ≥1 positive test and ≥1 destructive/error-path test.** [V1]
- Minimum test sets:
  - Scene Card: create, reorder, duplicate, delete, restore, build.
  - Draft: create, edit, compare, export, lock, revision.
  - Breakdown: suggest, accept, reject, remove, script-change reconciliation.
  - Schedule: drag, move, conflict, date change, call-sheet refresh.
  - Exchange: export, import matching, import stale, conflict, cancel.

## §77 Regression-Critical Workflows (L2786)
1. Scene Card → Build Screenplay → Breakdown.
2. Import FDX → Breakdown → Schedule → Call Sheet.
3. Draft 1 → Draft 2 → Compare → Lock → Revision A.
4. Breakdown Element → Catalog → Schedule.
5. Storyboard → Shot List → Export.
6. Schedule change → Call Sheet marked stale → Refresh.
7. Offline edit → Save → Close → Reopen.
8. Exchange export → 2nd machine import → response package → host import.
9. LAN collaboration → disconnect → project intact.
10. Delete/restore a scene with linked downstream data.

## §78 Edge Cases (L2802)
- Duplicate scene card with no heading: allowed. Build Screenplay requires a valid heading later.
- Empty Sequence and empty Act: allowed.
- Very long card description: store in full; truncate only in the card display.
- Same scene description twice: allowed. Identity is independent of text.
- Same character name twice: show the likely match first, but allow creating a second character.
- Same prop name in multiple scenes: default to reusing the existing catalog item when the user confirms.
- Deleting a sequence with cards: offer to move the children. Never lose them automatically.
- Screenplay scenes reordered independently of the board: supported. The board is a reference layer unless the user deliberately applies an order change.
- Script imported into a project with an existing board: creates a screenplay/draft and leaves the board untouched.
- Schedule from a screenplay with no breakdown: allowed.
- Call sheet with incomplete cast call times: allowed, with a visible missing-value indication.
- AI unavailable: hide/disable AI; the core app continues.

## §79 Must-Not-Build Boundaries (L2844)
- No enterprise accounting, payroll engine, union rules engine, VFX shot tracking, post-production asset review, distribution CRM, talent marketplace, or studio resource planner.
- No mandatory cloud sync.
- No required classification of Vault items.
- No story branching engine, mandatory character-arc scoring, or screenplay quality-scoring dashboard.
- No automatic schedule optimizer that silently moves scenes.
- No automatic AI rewriting of screenplay text.
- No giant metadata form on Scene Cards; no manual scene numbers on the Story Board.

## §80 Implementation Handoff Checklist (L2867)
Answer all before a module is implementation-ready: what opens it; default screen state; minimum input; optional fields; what Create, Edit, Duplicate and Delete do; what Undo restores; what Save persists; what Cancel does; missing required object; object already used elsewhere; offline; export; another user imports/edits; downstream affected vs deliberately not affected; what is authoritative after the action; what is visible in history; how QA verifies it. Any unanswered item is logged as a spec question and never invented in code.

## §81 ID Conventions (L2896)
`PRD-[MODULE]-NNN`, `FSD-[MODULE]-NNN`, `TEST-[MODULE]-NNN` (QA), `UI-[MODULE]-NNN` (UX screens).

## §82 PRD Traceability (L2928)
- Core chain: IDEA VAULT → VISUAL STORY BOARD → ACTS/SEQUENCES/BEATS/SCENE CARDS → SCREENPLAY → REVIEW/REWRITE/VERSION → LOCKED SHOOTING SCRIPT → SCRIPT BREAKDOWN → CAST/LOCATIONS/PROPS/WARDROBE/OTHER → SHOT LIST/STORYBOARD/MOODBOARD → STRIPBOARD/SHOOTING SCHEDULE → CALL SHEETS.
- The spec is incomplete while any PRD requirement has no FSD mapping. An FSD feature without a PRD ID is unapproved scope.
- Contradiction test per FSD requirement: has a PRD ID → same priority → no scope violation → has behavior → has acceptance test.
- PRD = intent, scope, priority. FSD = observable behavior; it may clarify but not change scope. Conflicts must be resolved explicitly; engineering must not pick one.

## §83 Final Functional Definition (L2970) — the 20 capabilities
1. Create or import a project.
2. Global/Project Vault with no categorizing.
3. Drag/drop Act/Sequence/Beat/Scene Card board.
4. Board stays a reference after writing starts.
5. Build a screenplay from cards, or write/import one.
6. Named drafts plus automatic history.
7. Compare drafts and run lightweight review rounds.
8. Lock a shooting draft plus controlled revisions.
9. Manual breakdown plus confirmed auto suggestions.
10. Simple catalogs: locations, cast, props, others.
11. Moodboards, storyboards and shot lists, none required.
12. Stripboard by dragging scenes onto days.
13. Conflict warnings and advisory grouping.
14. Call sheets from days.
15. Offline local files.
16. Portable project packages.
17. Review/exchange packages instead of cloud.
18. Private LAN live collaboration.
19. Optional AI that never takes control.
20. Professional exports while keeping ownership.

No mandatory wizard; minimal duplicate entry.

## §84 Non-Functional Boundary (L2998)
- Out of FSD scope (defined in the ESD): language, framework, storage engine, file internals, concurrency algorithm, network transport, encryption, AI provider, parsers, PDF/FDX/DOCX libraries, updater, telemetry, logging.
- Implementation choices must not violate FSD behavior.

## §85 Engineering Sign-Off Gate (L3019)
- A functional area is ready when it has: UX spec coverage; FSD coverage; domain/data spec coverage; collaboration spec coverage (where relevant); import/export coverage; acceptance IDs; zero unresolved questions (or explicitly recorded future-scope ones).
- "Done" means behavior, cross-module relationships, error handling, persistence, undo/recovery, export and acceptance tests all pass. UI existing is not enough.

## Appendix A — PRD → FSD Ledger (L3034)
"no ID" = "Covered by functional sections and cross-cutting constraints; no standalone inventory ID."

**Idea**
- PRD-IDEA-001 → FSD-IDEA-001..002, 014
- PRD-IDEA-002 → FSD-IDEA-003..004, 011..013
- PRD-IDEA-003 → FSD-IDEA-005..008, 019
- PRD-IDEA-004 → FSD-IDEA-009..010, 020
- PRD-IDEA-005 → FSD-IDEA-015..017

**Story**
- PRD-STORY-001 → FSD-STORY-001..003, 017..019, 028
- PRD-STORY-002 → FSD-STORY-004, 029..030
- PRD-STORY-003 → FSD-STORY-005..007, 026..027
- PRD-STORY-004 → FSD-STORY-008..016
- PRD-STORY-005 → FSD-STORY-020..025
- PRD-STORY-006, -007 → no ID

**Script**
- PRD-SCRIPT-001 → FSD-SCRIPT-001..022
- PRD-SCRIPT-002 → FSD-SCRIPT-023..027
- PRD-SCRIPT-003 → FSD-SCRIPT-028..033
- PRD-SCRIPT-004 → FSD-SCRIPT-034..037
- PRD-SCRIPT-005 → FSD-SCRIPT-038..044
- PRD-SCRIPT-006 → FSD-SCRIPT-045..050
- PRD-SCRIPT-007 → no ID

**Episodic:** PRD-EP-001, -002 → no ID

**Breakdown and production**
- PRD-BRK-001 → FSD-BREAKDOWN-001..019
- PRD-PROD-001 → FSD-PROD-001..003
- PRD-LOC-001 → FSD-PROD-004..007
- PRD-CAST-001 → FSD-PROD-008..010
- PRD-VIS-001 → FSD-PROD-011..013
- PRD-STB-001 → FSD-PROD-014..017
- PRD-STB-002, -003 → no ID
- PRD-SHOT-001 → FSD-PROD-018..021
- PRD-PROD-002 → FSD-PROD-022
- PRD-PROD-003 → FSD-PROD-023
- PRD-PROD-004 → FSD-PROD-024
- PRD-PROD-005 → FSD-PROD-025..026
- PRD-PROD-006, -007 → no ID

**Schedule and call sheet**
- PRD-SCHED-001 → FSD-SCHED-001..009, 014..016
- PRD-SCHED-002 → FSD-SCHED-010..013
- PRD-SCHED-003 → no ID
- PRD-CALL-001 → FSD-CALL-001..012

**Collaboration, offline, AI**
- PRD-COL-001 → FSD-COL-001..005
- PRD-COL-002 → FSD-COL-007
- PRD-COL-003 → FSD-COL-006, 008..011
- PRD-COL-004 → FSD-COL-012..016
- PRD-COL-005 → FSD-COL-017..019, 021..022
- PRD-COL-006 → FSD-COL-020
- PRD-OFF-001 → FSD-OFF-001..010
- PRD-AI-001 → FSD-AI-001..027
- PRD-AI-002 → no ID

**Core, templates, UX**
- PRD-CORE-002 → FSD-IDEA-018
- PRD-CORE-001, -003, -004 → no ID
- PRD-TPL-001, PRD-UX-001 → no ID

## Appendix B — Core Object Behavior (L3096)
| Object | Create | Edit | Duplicate | Move | Delete | Export | Downstream |
|---|---|---|---|---|---|---|---|
| Idea Vault Item | Y | Y | Yes/copy | Folder/collection | Soft delete | Package/file | Optional Story copy |
| Act | Y | Y | Optional | Y | Safe delete | Outline | Story structure |
| Sequence | Y | Y | Optional | Y | Safe delete | Outline | Story structure |
| Beat | Y | Y | Y | Y | Soft delete | Outline | Optional Scene creation |
| Scene Card | Y | Y | Y | Y | Soft delete | Outline | Optional Screenplay creation |
| Character | Y | Y | Optional | N/A | Archive | Report | Screenplay/Production |
| Draft | Y | Y | Yes/version | N/A | Soft delete | Script formats | Breakdown/review |
| Breakdown Element | Y | Y | Catalog reuse | Scene association | Remove association | Report | Catalog/Schedule |
| Catalog Item | Y | Y | Optional | N/A | Archive | Report | Schedule/Call Sheet |
| Shot | Y | Y | Y | Y | Soft delete | Shot list | Storyboard/Call Sheet |
| Storyboard Panel | Y | Y | Y | Y | Soft delete | Board export | Shot List |
| Shooting Day | Y | Y | Duplicate | Calendar/schedule | Archive | Schedule PDF | Call Sheet |
| Call Sheet | Y | Y | Duplicate | N/A | Archive | PDF | External distribution |

## Appendix C — Default UI Labels (L3115) — full list, use verbatim unless the UX spec justifies otherwise
New Project · Open Project · Idea Vault · Add Note · Add File · Add Image · Add URL · Add Voice Note · Story Board · Add Act · Add Sequence · Add Beat · Add Scene · Parking Lot · Build Screenplay · Screenplay · New Draft · Compare Drafts · Start Review · Comment · Resolve · Lock as Shooting Draft · Start Revision · Breakdown · Suggest Elements · Confirm · Reject · Edit Suggestion · Production · Locations · Cast & Crew · Catalog · Moodboards · Storyboards · Shot List · Schedule · Unscheduled · Create Shooting Day · Call Sheets · Create Call Sheet · Export · Import · Share / Exchange · Start Collaboration Session · Search · Undo · Redo

## Appendix D — Definition of Done, Functional Layer (L3168)
- Every P0 has explicit behavior. P1 has explicit behavior or is described as a lightweight extension. P2 is bounded if retained.
- Each object needs lifecycle rules plus explicit source-of-truth, offline, collaboration and import/export behavior.
- Destructive actions must be safe. Undo/redo expectations and acceptance criteria must exist for core workflows.
- No unstated cloud backend, no enterprise assumptions, no re-entry of data just because another module needs it, no automatic silent overwrite of creative/production decisions.
- After engineering starts, changes are versioned like product changes (especially source-of-truth, identity, import/export and collaboration).

---

## §86 Screen-by-Screen Functional Contract (L3194) — regions mandatory
- **Home:**
  - Regions: Recent Projects list, New Project button, Open Project button, Global Idea Vault shortcut, Archived Projects access, Search/command access.
  - Selecting a row opens the project. Row context actions: Rename, Duplicate, Archive, Reveal in File Manager.
  - No-data state: short explanation plus New Project / Open Project.
- **Project Home:**
  - Regions: project identity, **Continue**, Quick Access, Recent, Project Files, lightweight status.
  - Continue opens the most recent meaningful workspace; if gone, falls back to Project Home.
- **Idea Vault:** add, search, view selector, folder/collection nav, item canvas/list, item preview/editor. Add is never hidden behind settings.
- **Story Board:**
  - Regions: act/sequence hierarchy, cards, Parking Lot, view selector, undo/redo, build-screenplay action.
  - Only objects in the current hierarchy show on the canvas. Metadata appears only in the expanded card.
- **Screenplay:**
  - Regions: scene navigator, page, element selector, comments/notes toggle, draft selector, export, search.
  - **Focus Mode** removes nonessential chrome without removing function.
- **Breakdown:** scene list, script text, breakdown panel, category groups, suggested-elements area, catalog selector.
- **Production:**
  - Grouped workspace with tabs: Catalog, Locations, Cast & Crew, Moodboards, Storyboards, Shot Lists, Schedule, lightweight Budget.
  - The landing view is a gateway, not a detail dump.
- **Call Sheets:** list, Create Call Sheet, status filter, document editor, export, refresh-from-schedule.

## §87 Form and Field Rules (L3254)
- Required fields: only those needed for a valid object. Visibly marked; inline message on empty commit.
- Optional fields are collapsed or omitted until requested.
- Save model: short card/form edits autosave. Longer dialogs may have Save/Cancel.
- **Cancel** discards only edits made in that dialog since it opened.
- **Escape** closes a transient panel/dialog when there are no unsaved edits. With edits, it gives the same discard confirmation as Cancel.
- Invalid input stays visible in context; never navigate away to report an error.
- Duplicate names are allowed unless functionally required unique; parent context disambiguates.
- Long text is never silently truncated (visual scroll/collapse only).
- Paste keeps readable content. Screenplay paste goes through the import/parser flow, not arbitrary rich text.

## §88 Idea Vault Granular (L3283)
- `Add` opens a compact menu: **Note, Image, URL, File, Audio/Voice Note, Video**. Recent choices may appear first [LATER]; the full set is always available.
- Dragging files onto the canvas adds them. A multi-file drop imports all valid items in one operation, with progress for large files.
- Single click selects, double click opens. Multi-select batch actions: Move to Folder, Tag, Delete, Export.
- Preview: images and PDFs inline where practical; video/audio playable. Unsupported types stay valid file cards with filename and file actions.
- Text notes use a simple editor with autosave. Close/reopen needs no Save step.
- URL item: the URL is authoritative and preview metadata is secondary. If metadata fetch fails, the URL stays usable.
- Collections are user visual groupings that can reference items across folders. Removing from a collection never deletes the item.
- Pin changes ordering/display only and never moves files.
- Global → project copy creates a separate item. Label must be **"Copy to Project"**, never "Sync to Project".
- No automatic cleanup by age or "unused" status.

## §89 Story Board Granular (L3316)
- Drop zones for Act/Sequence/Beat/Scene are visually obvious and predictable before release.
- Double click opens the expanded card. Clicking outside saves and returns to the board unless the user explicitly cancels.
- While dragging: a vertical insertion marker for sibling insert; a container highlight for re-parent.
- Moving a Sequence carries its Scene Cards and Beats (identities kept). Moving an Act carries all descendants.
- Multi-select drag moves the group and keeps internal order.
- Zoom is presentation only; nothing persists unless the user saves view settings.
- Filters [LATER/optional] hide cards by type/status but never remove them.
- Parking Lot is reachable from every board view. Parking removes a card from the active flow but keeps its content and parent history.

## §90 Scene Card Lifecycle (L3346)
| Stage | Action | Result |
|---|---|---|
| New | Add Scene | Card inserted and focused |
| Edit | Type description | Updates and autosaves |
| Detail | Double click | Expanded editor |
| Move | Drag | Parent/order changes |
| Duplicate | Duplicate | New independent card |
| Park | Drag to Parking Lot | Removed from active flow |
| Restore | Drag back | Returns to selected position |
| Build | Include in Build Screenplay | Creates screenplay scene |
| Delete | Delete | Soft-deleted, recoverable |
| Restore delete | Restore | Former or unassigned location |

- Deleting a card never deletes a screenplay scene created from it; after conversion they are separate objects.
- Description is plain text. Line breaks are allowed in expanded view; collapsed cards truncate visually.
- Heading is optional. It is stored as a *proposed* heading, not a production heading, until the screenplay scene exists.
- Attachments on the expanded card are reference-only and never become production assets automatically.

## §91 Build Screenplay Flow (L3373)
1. Open Story Board.
2. **Build Screenplay**.
3. The app collects active Scene Cards in board order.
4. A build preview appears.
5. Cards without a heading are marked **"Heading needed."**
6. The user supplies headings inline, excludes cards, or cancels.
7. Destination: **New Screenplay** or **New Draft** in an existing screenplay.
8. Scenes are created in order.
9. Scene numbers come from screenplay order.
10. The user lands on the first created scene.

- **Rebuild warning:** if a screenplay exists, explain that a new draft/document will be created and nothing is overwritten.
- **Description treatment:** "Use description as scene planning note" (default) or "Insert as temporary action text". Temporary text is clearly marked as editable draft text.
- **Order:** exactly the board order at build time. Parking Lot is excluded by default.

## §92 Screenplay Editor Contract (L3401)
- Standard cursor, selection, copy/cut/paste and undo.
- Each element is an editable unit; the element selector converts its type.
- New scene: at the cursor or via the navigator. The user types the heading; the number is generated.
- The navigator updates on add/remove/reorder. Clicking moves focus only.
- Act/sequence context from the Story Board may be displayed [LATER], read-only.
- Draft selector: switching saves first or asks for a decision when unsaved changes exist.
- The notes/comments toggle is visibility only and never inserts into content.
- Focus Mode hides panels and toolbars but keeps typing, search, undo/redo, save and essential navigation.

## §93 Script Review Package Flow (L3428)
- **Create:** Draft History or Review → **Share/Exchange → Review Package**.
- **Scope:** whole draft, selected scenes, or selected comments. The preview lists what is included.
- **Export:** to a user-chosen path. Draft status does not change.
- **Import:** **Import Exchange**. Identify package type and source draft/version.
- **Mapping order:** same scene identity → same textual anchor → same heading plus local position.
- Ambiguous mappings go to the **Review Queue** for manual targeting.
- Cancel leaves the project unchanged and stores no partial review.

## §94 Comment Anchoring (L3456)
- Text comment stores selected text context, owning scene, and enough surrounding context to relocate after edits.
- A scene comment is attached to scene identity.
- A card comment stays attached through reorders and container moves.
- If the target is deleted, the comment stays in history pointing at the deleted object until permanent purge.
- Resolve never deletes; the comment stays in review history.
- Replies append to the same thread and are not separate review items.

## §95 Script Lock (L3477)
- Precondition: resolving all comments is not required. The lock dialog shows unresolved comments.
- Result: the draft becomes the **Shooting Draft** reference. Record lock timestamp and user identity.
- Typing in locked content triggers **Start Revision**. The locked source never changes.
- A revision is a new writable document derived from the locked source. The user can name it and optionally choose a revision color.
- Breakdown can reference the locked draft or a later approved revision. The chosen source is always visible.

## §96 Breakdown Screen (L3497)
- Scene list shows generated numbers, headings and concise status indicators.
- A script reading pane is available while tagging.
- Selecting text and choosing a category creates a suggested or manual element, depending on entry mode.
- **Add Element**: type an item name without selecting text.
- While typing, existing catalog matches appear immediately; the user picks one or creates a new item.
- Categories with confirmed content come first; empty categories may collapse.
- **Breakdown Complete** is a manual per-scene planning indicator only.

## §97 Suggestion Decision Rules (L3521)
- Pipeline: analyze scene → candidates → show category plus matched context → user **Accept / Edit / Reject** → confirmed production data.
- If a suggestion resembles a catalog item, show the match as the default choice but never attach it silently.
- Reject affects only the current suggestion set. It deletes no catalog item and doesn't block the same word elsewhere.
- The category can be changed before acceptance.
- Batch accept "can be supported" and must show the exact count and categories before commit.
- AI suggestions are labelled as suggestions. Once confirmed they are ordinary data with no AI branding.

## §98 Catalog Cross-Scene Behavior (L3553)
- One identity per item (e.g. `Red Motorcycle` used by 10 scenes). Each scene records that it requires the item.
- Editing description, image or status updates every display and never touches historical script text.
- The catalog association can be replaced for selected scenes without deleting the old item.
- Archive removes the item from active selection; historical associations stay readable.

## §99 Location Workflow (L3571)
1. Create a location, standalone or during breakdown.
2. Status starts at Idea or Shortlisted.
3. Add address, photos, contact, notes.
4. See all scenes needing it.
5. Set Confirmed after real-world confirmation.
6. The schedule uses it.
7. The call sheet pulls name, address and selected info.

- A status change never alters the screenplay.

## §100 Cast/Crew (L3584)
- Cast: create an actor/person, link to a character, add contact and availability notes.
- The Character is the story object; the Actor is the real person attached to it.
- Availability is freeform and referenced during scheduling.
- Crew are grouped by department and appear on call sheets depending on the day.
- Removing someone from the directory doesn't remove references from exported documents; historical snapshots are preserved.

## §101 Moodboard (L3602)
- Create a board with a name only; no category required.
- Add items: drop image/file, paste image, text note, URL.
- Drag to move; multi-select moves groups. Resize is visual only.
- Caption is optional and attached to the item.
- **Private notes are excluded from standard exports.**
- Export: choose a layout. The output is a snapshot; editing it does not change the board.

## §102 Storyboard (L3626)
- Entry: select a screenplay scene → **Create Storyboard**, or Production → Storyboards.
- A new panel has an empty visual area and a short description.
- Visual source: import an image, use a drawing tool "if present" [LATER], or leave a blank placeholder.
- A panel may attach to a Shot. A second panel on a shot is an *additional option* and never replaces the first.
- Panel order drives displayed shot/panel numbering.
- Export: one scene, selected scenes, or the full board "where available".

## §103 Shot List (L3647)
- Opens to a selected scene or the full project list.
- **Add Shot** appends at the end of the selected scene's coverage.
- Shot number is generated from scene identity plus order; no manual renumbering after drag.
- A shot is valid with only a description; camera fields are optional.
- Group by scene. "master"/"coverage"/"insert" are freeform notes, not a taxonomy.
- Export for camera/lighting/AD: readable number, description, selected technical fields.

## §104 Stripboard (L3668)
- A schedule is tied to a selected screenplay source. All relevant scenes start Unscheduled.
- **Create shooting day:** date plus optional note → day strip/column.
- Drag a strip into a day → the day auto-updates scenes, locations, cast and estimates.
- Reordering within a day changes shooting order only.
- Moving to another day: leaves the original, enters the destination, and marks call sheets for **both** days potentially stale.
- Remove: drag back to Unscheduled, or **Remove from Day**.
- Off day: no scenes, notes allowed.
- Day duration: optional simple target; warn when estimates exceed it.

## §105 Call Sheet (L3698)
- **Create Call Sheet** uses one shooting day as source.
- Prefill: title, date, day, scenes, locations, cast, selected notes.
- Missing call times or contacts become blank placeholders with a **"needs input"** marker; creation is never blocked.
- Edits stay local to the call sheet unless the user explicitly updates the source record.
- On source change, show **"Source changed."** → **Refresh** → preview differences → apply.
- **Finalize** makes a named snapshot. Later changes create a new revision/snapshot and never overwrite the issued PDF.

## §106 Project Notes, Tasks, Activity (L3719)
- **Notes:** title/body, optional tag, optional attachment. Not tied to Vault/Story/Production.
- **Tasks:**
  - Fields: title; opt due date; opt owner; status Open/Done; opt related object.
  - Relatable to Scene, Location, Character, Breakdown Item, Shooting Day, Call Sheet.
  - Done never changes the related object. Not a PM system.
- **Activity history** records: Draft created, Scene moved, Script locked, Breakdown source changed, Schedule day created, Call sheet issued, Exchange imported. Never per-keystroke.

## §107 Research / Reference Boards (L3759)
- Name required, description optional.
- Sources: note, URL, PDF, image, document, quote. Research note explains "why it matters".
- Optional link to Scene Card, screenplay scene, Character, Location or production item.
- Optional verification flag: `Unchecked / Checked / Needs Expert Check`. Nothing is marked "true" automatically.

## §108 Production Dashboard (L3782)
- May show: current script source, breakdown completion count, schedule completion count, upcoming day, unresolved location/cast needs, latest call sheet state.
- Every item is clickable to its source workspace. Metrics are never prerequisites.

## §109 Lightweight Budget (L3800) — optional, ignorable (FSD-PROD-025/026 P1)
- Line item: description and amount, opt category. Category totals are computed.
- Contingency: a percent or amount, user-controlled.
- Values never auto-update from production changes. The dashboard may show a manual **"Review budget"** reminder. No silent cost mutation.
- PDF/CSV-style export "if implemented in the release" [LATER].

## §110 Production Reports (L3821)
- Scene report: heading, page count, indicators.
- Location report: locations → scenes plus status.
- Cast scene report: actor → character → scenes.
- Prop report: catalog prop → scenes.
- Schedule report: day → date → scenes → locations.
- Breakdown completeness: confirmed/suggested/unreviewed by scene.
- Reports are generated on demand as snapshots; nothing persists unless saved/exported.

## §111 Sides (L3846)
- Select a shooting day. Contains that day's scheduled scenes from the chosen production source.
- Optional call-sheet cover page.
- Must display the screenplay source/draft name. Output is a PDF snapshot.

## §112 File and External Storage (L3864)
- A project on a removable drive works normally while mounted.
- If the drive becomes unavailable while open: warn immediately, preserve unsaved state as safely as possible, offer reconnect/retry.
- External reference: store the location and show a reference indicator.
- Collect referenced files into a portable copy "where supported" [LATER].
- Resolve references by project context, not filename alone.

## §113 Template Application (L3882)
- Choose a template at New Project or from Templates.
- May include: sample Story Board structure, call sheet layout, moodboard layout, basic budget categories, project notes.
- The project is independent of later template edits. A template reset affects future uses only.

## §114 Beginner / Professional Paths (L3903)
- Beginner progression: Idea Vault → Story Board → Screenplay → Breakdown → Production → Schedule → Call Sheet.
- Experienced: Import Screenplay bypasses the board; Breakdown opens immediately; Schedule works without Moodboard/Storyboard.
- No mode lock; "Beginner" is not a permission.
- First-open helper text is dismissable and never blocks.

## §115 Project Status / Phase (L3920)
- Phase is a user-controlled label, changed from Project Home.
- Changing it creates no tasks and disables nothing.
- Shown in the project header and the recent-project row.
- The app may suggest a transition [LATER]; the user must accept.

### Contextual Attention Indicators (L3937, unnumbered — §116 missing)
- Examples: "Call Sheet needs refresh." · "3 scenes need breakdown review." · "Production source changed." · "2 exchange comments waiting." · "Schedule conflict detected."
- Attached to the relevant workspace. Disappears when resolved or dismissed. No notification spam.

## §117 Round-Trip Expectations (L3951)
- **FDX** export re-imports with structure substantially intact.
- **Fountain** parses back to equivalent elements.
- **DOCX** keeps readable content; layout may vary.
- **PDF** round-trip is best-effort and must show parse warnings.
- **Exchange** packages keep source identity and comments/objects with no cloud.

## §118 Integrity and Recovery (L3971)
- After abnormal close, reopening detects recoverable edits.
- Recovery dialog: **Recover latest state / use last confirmed save / dismiss recovery copy**.
- Offer a backup before risky operations (full exchange/project package import into an existing project).
- If content can't be fully read, name the affected area; never present partial data as complete.

## §119 Local Collaboration Session State Model (L3990)
- States: `Not Running → Host Starting → Running → Participant Joined → Participant Active → Participant Disconnected → Session Ended`.
- Host picks project plus collaboration scope. Participants see project/session name and their granted role.
- Shared-scope changes are visible to connected participants.
- Private notes stay private unless explicitly converted.
- On end: the session stops and each machine keeps local state per the agreed final state.

## §120 Locking and Presence (L4012)
- Presence indicator on an actively edited object.
- Soft lock [LATER/"may"]: **"X is editing this."** for screenplay text or complex schedule moves. Others can request access or continue if concurrent editing is allowed.
- No permanent lockout. Locks expire when the participant is gone, and an authorized host can release them.
- Lock state lives only in the session and is never persisted to the project.

## §121 Exchange Comparison Before Apply (L4027)
- A summary is shown before applying (e.g. "14 comments / 3 new scene cards / 1 changed scene description / 2 new shots / 0 deletions"). Categories expand.
- Apply options: **import all / comments only / selected items / cancel**. Prefer granular over all-or-nothing.

## §122 Export Selection (L4053)
- Scope must be explicit: Current scene / Selected scenes / Current sequence / Entire screenplay / Current shooting day / Entire schedule.
- The dialog shows source version and destination format before confirm.
- If private content is excluded, the summary states **"Private notes excluded."**

## §123 Printing Layout (L4071)
- Print uses the same content model as PDF.
- Headers/footers: project title plus page info where appropriate.
- Avoid separating a heading/label from its content.
- Printer-safe margins; print preview equals the generated PDF.

## §124 Change Impact Rules (L4088) — derived display vs source mutation
| Change | Allowed automatic effect | Forbidden automatic effect |
|---|---|---|
| Story Board card reorder | Update board order | Rewrite prose silently |
| Story Board card edit | Update card | Rewrite screenplay |
| Scene heading edit | Update screenplay | Rename location catalog silently |
| Screenplay source update | Flag production review | Delete breakdown/shot/storyboard data |
| Catalog status edit | Update catalog displays | Rewrite script |
| Schedule move | Update day totals | Rewrite screenplay |
| Schedule date edit | Update day | Overwrite issued call-sheet snapshot |
| Call-sheet edit | Update call-sheet document | Rewrite master schedule |
| AI suggestion | Show preview | Mutate project silently |

## §125 Stale Objects (L4105) — complete
- **Stale** means the source changed since the object was generated/prepared.
- Examples:
  - Call Sheet whose source schedule changed.
  - Shot List whose scene changed.
  - Breakdown whose source script changed.
  - Review package based on an older draft.
- The indicator must state (1) what changed, (2) when, (3) the available action. Example: **"Scene 24 changed in Shooting Draft B. Review Breakdown."**
- Stale is not invalid; the object stays usable until the user chooses to refresh.

## §126 Historical Snapshots (L4129) — complete
- Exported and issued documents are historical snapshots.
- Examples:
  - Issued Call Sheet 2027-06-14 v1 stays unchanged after schedule changes.
  - An exported Shooting Draft PDF stays exact after Revision A.
  - An exchange package snapshots its source state.
- Newer related versions may be displayed; the historical artifact is never rewritten.

## §127 Cross-Cutting Constraints (L4142)
- **Privacy:**
  - A local project is readable only via the user's machine/file access unless shared/exported.
  - External AI use must clearly state that selected/project info leaves OpenFrame.
  - **Private notes never appear in standard exports, exchange packages or call sheets.** [V1]
  - Collaborators can see their own role and access scope.
- **Accessibility:**
  - Keyboard focus moves between major controls; shortcuts for important commands where practical.
  - Non-drag alternative for drag/drop where needed.
  - Labels exposed to assistive tech; color is never the only state indicator; visible selection/focus.
- **Performance** (user-perceived, numbered 129.x in source):
  - Typing stays responsive.
  - Hundreds of scene cards stay navigable.
  - A long feature script stays searchable and editable as one document.
  - A large Vault is browsable via list/grid/folder plus search.
  - Many shooting days stay manageable in board/list views.

## §130 Module Dependency Matrix (L4199)
| Module | Requires | Produces/Feeds | Must not mutate |
|---|---|---|---|
| Global Idea Vault | Local project/user | Project Vault copies | Project/story/script |
| Project Idea Vault | Project | Story copies | Screenplay automatically |
| Story Board | Project | Optional screenplay build | Existing screenplay silently |
| Characters | Project | Screenplay/Production refs | Screenplay prose |
| Screenplay | Project | Breakdown, shots, storyboard, schedule source | Idea Vault automatically |
| Breakdown | Screenplay source | Catalog | Script text |
| Catalog | Project/breakdown | Schedule/Call Sheet data | Screenplay |
| Locations | Project/catalog | Schedule/Call Sheet | Screenplay |
| Cast & Crew | Project/characters | Schedule/Call Sheet | Screenplay |
| Moodboards | Project | Exports/refs | Production data |
| Storyboard | Scene/optional shot | Shot List/ref | Screenplay prose |
| Shot List | Scene | Call Sheet/ref | Screenplay prose |
| Schedule | Screenplay/catalog | Call Sheets | Issued call sheets |
| Call Sheet | Shooting Day | PDF/export | Master schedule |
| AI | Selected context | Suggestions/answers | Anything automatically |
| Exchange | Selected workspace | Portable review/update package | Host project automatically |

## §131 Full Acceptance Scenario Set (L4221) — minimum E2E suite
**A. Idea/development**
1. Global idea → close → reopen → persists.
2. Image + note → switch views → same items.
3. Copy global → project, edit copy → original unchanged.
4. Project idea → Story → original stays.
5. Act → Sequence → Beat → Scene.
6. Move Scene between sequences.
7. Move Sequence between acts.
8. Park Scene and restore.
9. Duplicate Scene and edit the duplicate.

**B. Screenplay**
10. Build from board.
11. Build with missing headings, then resolve.
12. Import FDX.
13. Import PDF with warning.
14. Import DOCX.
15. Paste screenplay.
16. Export PDF/FDX/Fountain/DOCX.
17. Create Draft 2.
18. Compare D1/D2.
19. Comment on text.
20. Resolve comment.
21. Lock shooting draft.
22. Attempt edit → start revision.
23. Create revision color.

**C. Production**
24. Create Breakdown.
25. Manually tag a prop.
26. Suggest elements.
27. Reject a suggestion.
28. Accept using an existing catalog item.
29. New location.
30. Assign cast.
31. Moodboard.
32. Storyboard.
33. Shots.
34. Link panel → shot.
35. Build schedule.
36. Drag scenes to days.
37. Cast/location conflict warning.
38. Create call sheet.
39. Change schedule → call sheet stale.
40. Refresh call sheet.
41. Export sides.
42. Export schedule.

**D. Offline/ownership**
43. Edit with network off.
44. Save/reopen offline.
45. Export project package.
46. Restore backup.

**E. Collaboration**
47. Export review package.
48. Import response.
49. Import stale response.
50. Resolve ambiguous mapping.
51. Start LAN session.
52. Join a second desktop.
53. Concurrent edits to different scenes.
54. Same-text conflict.
55. Participant disconnect.
56. End session.

**F. Safety**
57. Delete a scene card with a linked screenplay scene.
58. Delete a catalog item referenced by scenes.
59. Cancel a destructive operation.
60. Crash/reopen, recover autosave.

## §132 Change Management (L4298)
- Product review is required before changing: source-of-truth; object identity; screenplay versioning; script→breakdown mapping; schedule→call-sheet behavior; offline ownership; collaboration packages; delete/restore; import/export formats; AI permission boundaries.
- UI wording/layout may change freely. Changes to what is authoritative or to cross-module effects must be versioned.

## §133 Module Completeness Checklist (L4316)
All 21 must be specified and tested: entry point, empty state, creation, editing, selection, navigation, reorder/move, duplication, delete/archive, restore, undo/redo, save/autosave, offline, permissions, export, import, cross-module links, source-of-truth rule, stale/conflict, error behavior, acceptance criteria. Happy-path screens alone ≠ done.

## Appendix E — PRD Section Ledger (L4345)
- Rows 0–231 map every PRD section to boilerplate: "Expanded into one or more functional contracts…". QA expectation: ≥1 scenario/acceptance test per applicable area.
- PRD areas listed here that get **no concrete behavior in Part 2** (look for them in Part 1 or log spec questions): Character Relationship View (31), Story Timeline (32), Intelligent Element Navigation (35), Screenplay Layout (36), Writing Room Side Panels (40), Series-Level Story Board (56), Breakdown Categories (60), Stripboard Views (75), Schedule Calendar (81), Global Project Search (98), Keyboard Support (103), Project Search vs Script Search (169), Contextual Navigation (170), Scene Hub (171), Sequence Hub (172), Project-level Quick Actions (173), Context Menus (174), Quick Capture (175), Idea Vault Favorites (176), Media Handling Scope (182), Desktop Windowing (184), External Monitor Use (185), Fullscreen Writing Mode (186), What Happens After Shooting? (157), Primary Journey — Writer Only (165), Primary Journey — Producer/AD (166), Feature Priority P0/P1/P2 (195–197), Explicitly Deferred (198).

## Appendix F — Suggested Build Order (L4583) — ordering only, not architecture
- **W1:** shell; project create/open/close; Global/Project Vault; Story Board; Acts/Seq/Beats/Cards; undo/redo; autosave/recovery.
- **W2:** screenplay data/editing; build from board; drafts/history; comparison; comments/reviews; lock/revisions; import/export.
- **W3:** breakdown; catalog; locations; cast/crew; moodboards; storyboard; shot list.
- **W4:** stripboard; conflict warnings; Daily Production View; call sheets; sides/reports; budget.
- **W5:** exchange packages; LAN session; conflict/reconciliation; AI assistant; import/export hardening.

## Appendix G — PO Release Checklist (L4637)
- **Creative:** dump an idea with no setup; quick act/sequence/scene creation; rearrange with no numbering work; write/import; preserve/review drafts.
- **Production:** locked/current script → breakdown; confirmed → catalog; plan visuals/shots; stripboard; call sheet without retyping the day.
- **Ownership:** works with no internet; backup/move; remote review via files; LAN collaboration without cloud.
- **Simplicity:** no purposeless inputs; no new permanent sidebar item; cards small; short film stays comfortable.
- **Safety:** undo experiments; recover deletes; recognize stale docs; no silent loss of collaborator work.

---

## §134 Field-Level Contract (L4676)
**Project**
| Field | Req | Editable | Used by |
|---|---|---|---|
| Title | Y | Y | All views |
| Type | Y | Y | Workflow/context |
| Language | N | Y | Documents/metadata |
| Genre | N | Y | Reference |
| Creator | N | Y | Documents |
| Status | N | Y | Home/header |
| Project notes | N | Y | Home/Files |

**Idea Vault Item**
| Field | Req | Editable | Notes |
|---|---|---|---|
| Item type | Y | Limited | Drives presentation |
| Title | N | Y | May stay blank |
| Body/caption | N | Y | Type-dependent |
| File/URL | Type-dependent | Limited | Primary source |
| Folder | N | Y | Organization |
| Collection(s) | N | Y | Organization |
| Tags | N | Y | Search |
| Pin | N | Y | Display |

**Other objects** (required → optional):
- **Act:** title → note.
- **Sequence:** name (main field) → note.
- **Beat:** beat text → note, color, attachment.
- **Scene Card:** short description → heading, notes, attachments. **No manual scene number, no production fields.**
- **Character:** name → role, description, image, notes, relationship notes.
- **Screenplay Scene:** valid heading for normal output. Action/dialogue are content, not metadata.
- **Draft:** name, parent/source → note. State is generated by workflow.
- **Review Round:** source draft, name → reviewers, deadline. Status generated/controlled.
- **Breakdown Element:** category, name/reference → matched text, notes.
- **Catalog Item:** name, category, status → image, description, notes, scene usage.
- **Location:** name → address, contact, status, photos, practical notes.
- **Person:** name → role, department, contact, image, availability, notes.
- **Shot:** description (plus generated order/shot id) → framing, movement, angle, lens, notes, storyboard ref.
- **Storyboard Panel:** panel identity/order plus visual area → description ("recommended"), shot link, technical notes.
- **Shooting Day:** date, day number → notes. Scene membership is derived from the schedule.
- **Call Sheet:** source shooting day (plus generated core data) → editable call times, notes, meeting point, attachments. Revision state is generated by workflow.

## §135 Page Action Matrix (L4752)
| Page | Primary | Secondary | Forbidden/hidden by default |
|---|---|---|---|
| Home | Open/Create Project | Archive, duplicate, rename, search | Production metadata |
| Idea Vault | Add | Search, filter, collection, copy to project, export | Required classification |
| Story Board | Add Scene/Add Sequence | Add Act/Beat, move, duplicate, park, build screenplay | Production fields |
| Characters | Add Character | Edit, archive, open scenes | Payroll/HR |
| Screenplay | Write/Edit | Drafts, compare, comment, export, lock | Idea Vault live sync |
| Breakdown | Tag/Suggest | Catalog, filter scenes, reports | Silent auto-confirm |
| Production | Open workspace | Catalog, locations, cast, visuals, schedule, budget | Enterprise modules |
| Shot List | Add Shot | Reorder, storyboard, export | Required camera metadata |
| Storyboard | Add Panel | Import/draw, reorder, export | Full illustration suite |
| Schedule | Create Shooting Day | Drag scenes, date, warning review, export | Forced auto-optimization |
| Call Sheets | Create Call Sheet | Edit, refresh, issue, export | Silent schedule mutation |
| Files | Add File | Move, rename, open, export, delete | Duplicate file bureaucracy |
| AI | Ask | Select context, accept/reject actions | Silent project changes |

## §136 Screen-Level Visible States (L4771)
- **Story Board:** Empty, Populated, Filtering, Card Expanded, Dragging, Building Screenplay, Saving, Save Error.
  - Drop on a valid target commits immediately.
  - Drop outside valid targets returns the card to its original position with **no mutation**.
- **Screenplay:** Loading Draft, Editing, Searching, Comparing, Review Overlay, Locked, Revision Mode, Saving, Save Error.
  - Locked stays readable; editing moves toward creating a revision.
- **Breakdown:** No Source, Loading Source, Reviewing Scene, Suggesting, Suggestions Ready, Scene Breakdown Complete, Needs Review.
- **Schedule:** No Schedule, Unscheduled Pool, Planning, Conflict Warning, Day Selected, Export Preview.
- **Call Sheet:** None, Draft, Source Changed, Ready, Finalized, Superseded.

## §137 Screenplay Import Mapping (L4793)
- Recognized headings → Scene heading. Ordinary text → Action. Character labels → Character.
- Dialogue stays tied to the preceding character. Parentheticals stay distinct where recognizable. Transitions → Transition.
- Unknown structures stay readable; **never discard text** because its formatting is unknown.
- The summary reports the detected scene count; the preview can be inspected before commit.
- Import into an existing project **always creates a new screenplay/draft**. Replacement happens only through an explicitly destructive workflow the user selects.

## §138 Import Validation Checklist (L4825)
- Display when applicable: file opened; text extracted; headings detected; characters detected; page count estimated; unsupported/ambiguous formatting; potentially empty scenes; original file untouched.
- The user may continue with warnings.

## §139 Export Configuration (L4841)
- Every dialog must answer, before confirm: **What? Which version/source? What format? Where saved?** Optional settings come after.
- Defaults must produce professional, usable output.
- Private notes, comments and internal IDs are excluded from normal deliverables. They are included only in review packages or internal reports.

## §140 Revision Colors (L4855)
- Used only after a production-locked baseline. **Start Revision** is launched from the locked source.
- The system identifies changed content. Color is applied to supported markers/pages per export mode.
- Ordinary Draft 1/2 writing has no revision color.
- Each revision records source, label, date, and reason (opt).

## §141 Review Round Matrix (L4875)
| Action | Actor | Effect | Does not |
|---|---|---|---|
| Start review | Owner/Editor | Review linked to selected draft | Lock script |
| Add comment | Reviewer/Commenter/Editor | Note on target | Mutate script |
| Reply | Any permitted reviewer | Appends reply | Auto-resolve |
| Resolve | Permitted reviewer/editor | Thread resolved | Delete |
| Complete review | Owner/Editor | Closes round | Approve script unless explicit |
| Export review | Authorized user | Review package | Change source draft |
| Import review | Owner/Editor | Adds responses after preview | Overwrite current draft |

## §142 Script Lock Matrix (L4888)
| Action | Result | Safety |
|---|---|---|
| Lock Draft | Draft → Shooting Draft | Explicit confirmation |
| Edit locked draft | Prompt Start Revision | Locked source intact |
| Start Revision | New writable revision | Source retained |
| Export locked draft | Snapshot exported | Export can't mutate source |
| Delete locked draft | **Blocked** until another valid source exists or it is downgraded by an authorized workflow | Protects the sole production baseline |

## §143 Breakdown → Catalog Matching (L4899)
- Search existing items by visible identity fields.
- Exact/obvious match → shown first. Multiple matches → shown with distinguishing context; choose or create. No match → **Create New Catalog Item**.
- Renaming before confirmation **re-runs matching**.
- The user can always choose **Create New**, even when a similar item exists.

## §144 Catalog Relationships (L4919)
- One item connects to many scenes. Removing one association never deletes the item.
- Renaming updates the directory and its displays; historical exports are never rewritten.
- Archiving keeps historical references.

## §145 Scheduling Field Rules (L4932)
| Value | Source | Editable in Schedule | Behavior |
|---|---|---|---|
| Scene number | Screenplay order | No | Generated display |
| Scene heading | Screenplay source | No in strip default; edit via source | Shows latest selected source |
| Page count | Screenplay source | Optional manual override | Override visibly marked |
| Location | Breakdown/catalog or heading | Yes (production assignment) | Never rewrites script |
| Cast requirement | Breakdown/character refs | Derived | Feeds conflict warnings |
| Estimated duration | User | Yes | Affects day total |
| Shoot date | Shooting Day | Yes | Used by call sheet |
| Day order | Schedule | Yes, by drag | Production order only |

## §146 Schedule Conflicts (L4946)
- **Actor conflict:** show both scenes/days and the actor. **Location conflict:** same location in incompatible simultaneous placements, only where timing data exists.
- **Duration overflow:** estimates exceed the target. **Missing info:** show **Missing**; never invent a duration or call time.
- Every warning offers: **Open affected item / Keep schedule anyway / Move scene (user performs) / Dismiss warning** (where appropriate). The system never makes the final choice.

## §147 Call Sheet Refresh Matrix (L4971)
| Source change | Status | Default action |
|---|---|---|
| Scene moved to another day | Needs Refresh | Preview changes |
| Scene removed | Needs Refresh | Review removal |
| New scene added to day | Needs Refresh | Preview addition |
| Cast assignment changed | Needs Refresh | Preview cast changes |
| Location changed | Needs Refresh | Preview location changes |
| Call-sheet-only note changed | No schedule effect | Continue |
| Schedule date changed | Needs Refresh | Preview new date |

## §148 Call Sheet Finalization (L4984)
1. Open the shooting day.
2. **Create Call Sheet**.
3. Verify prefilled scenes, cast and locations.
4. Add call times and notes.
5. Save.
6. Optionally mark **Final/Issued**.
7. Export PDF.

- Later schedule changes leave the issued document historical. Refresh plus reissue creates a new revision.

## §149 Package Taxonomy (L4997)
| Package | Use | Contains | Import result |
|---|---|---|---|
| Full Project Package | Move/backup | Workspaces + data + selected files | New local project or copy |
| Story Exchange | Story review | Acts/seq/beats/scenes/selected notes | Review/copy/selected apply |
| Script Review | Script feedback | Draft snapshot + comments/annotations | Review record/comments |
| Breakdown Exchange | Breakdown review | Selected scene breakdown/catalog refs | Proposed production updates |
| Shot Exchange | Shot planning | Shots + optional storyboard refs | Shot updates/copies |
| Schedule Exchange | Scheduling | Days + scene assignments | Schedule review/selected apply |
| Call Sheet Review | Daily doc feedback | Call sheet snapshot + comments | Call sheet review/comments |

## §150 Exchange Validation (L5010)
- Validate: package type; source project ref; source draft/version; completeness; compatibility; conflicts/staleness.
- Then preview. **Validation failure ⇒ zero project mutation.**

## §151 Comment Mapping Decision Tree (L5026)
1. Source object identity exists → map to it.
2. Exact text anchor exists → map to text.
3. Heading/context gives one clear match → *propose* it.
4. Multiple matches → **Review Queue**.
5. No safe match → keep as **Unmapped Review Note**.

- Never silently discard a comment.

## §152 LAN Session Flow (L5039)
- **Host:** select project → **Start Session** → shared scope → assign participant roles → show join info.
- **Participant:** join → select identity/name → receive role → enter workspace.
- **During:** edit allowed objects; presence shown; normal object rules apply.
- **Disconnect:** local changes stay on each machine; "participant disconnected" reported; host continues.
- **Rejoin:** receives current shared state; unresolved conflicts are presented, never silently discarded.
- **End:** host ends; each project remains usable offline.

## §153 Collaboration Scope (L5060)
- Scopes: **Entire project / Story only / Screenplay only / Production only**.
- A Story-only user never gains screenplay/production access through hosting.
- Export-only participants cannot join live editing.

## §154 Offline/Online UI States (L5074) — never conflate
- **Offline** = all core local functions. **Online** = optional external services. **Local Session** = connected to another OpenFrame on the private network. **Cloud AI** = provider connected.
- Status copy: `Offline · Local Project Saved` · `Local Session · 2 collaborators connected` · `AI Provider Connected`.
- **No "Syncing to OpenFrame Cloud" state exists.**

## §155 External AI Flow (L5103)
1. Determine selected context.
2. Show provider/context disclosure when required.
3. User submits.
4. Answer/suggestion returned.
5. User may accept a supported mutation.
6. The mutation becomes a normal, undoable app action.

- On network failure: report that the AI action could not be completed; local content unchanged.

## §156 AI Mutation Preview Contract (L5117)
| Request | Preview | Apply |
|---|---|---|
| Create Scene Card | Proposed text + parent sequence | Add to Story Board |
| Create Breakdown Elements | List grouped by category | Accept selected/all |
| Generate Synopsis | Proposed text | Copy into selected field |
| Suggest Schedule Grouping | Grouping explanation | User manually applies/confirms |
| Draft Summary | Summary | Save as note/copy only |
| Rename/modify object | Exact proposed change | Confirm before mutation |

## §157 Activity History (L5129)
- Records: Project created; Draft created; Draft locked; Revision started; Scene card moved between sequences; Breakdown source changed; Catalog item created; Shooting day created; Call sheet finalized; Exchange imported/exported; Collaboration session started/ended.
- No typing entries. Fields: actor, action, date/time, affected object. **Read-only in UI.**

## §158 Tasks (L5152)
- Create: title required; opt due date, owner, related object.
- Complete → Done, with no related-object mutation. Reopen → Open. Delete = soft/recoverable.
- Clicking the relation opens the object. The list filters by owner/status.
- No dependencies, Gantt, time tracking or sprints.

## §159 Project Notes (L5177)
- Title, body, attachment, optional related object.
- Exportable/printable, but excluded from formal documents by default.

## §160 Source Version Banner (L5191)
- Every screenplay-derived production workspace shows a compact banner: `Production Source: Shooting Draft 6`.
- When newer approved material exists: `Newer revision available — Review Production Update`. Informational until the user acts.

## §161 Production Update Review Flow (L5206)
1. Open Production.
2. A newer revision is detected.
3. **Review Update**.
4. Compare sources.
5. Impact list by scene.
6. **Review each / Accept all safe mappings / Dismiss**.
7. The new source becomes the production source only after confirmation.
8. Affected data is flagged for review.

- "Accept all safe mappings" applies only unambiguous scene identities and never removes production data.

## §162 Relationship Preservation (L5220)
- **Moving a Scene Card between Sequences:** keeps identity, comments and attachments; creates no screenplay scene.
- **Duplicating a card:** new identity, content copied, source screenplay relationship **not** copied.
- **Reordering screenplay scenes:** renumbers; keeps breakdown/shot/storyboard associations by identity.
- **Moving a catalog item between UI groups:** no screenplay effect.

## §163 Delete Cascade Rules (L5242)
| Deleted | Default effect | Warning |
|---|---|---|
| Act | Children stay available for move/restore; no silent cascade | Yes if non-empty |
| Sequence | Scene Cards available for reassignment | Yes if non-empty |
| Scene Card | Screenplay scene remains | Yes if linked |
| Character | Screenplay text remains | Yes if linked |
| Catalog Item | Scene associations removed/archived only with confirmation | Yes |
| Location | Schedule history readable | Yes |
| Shooting Day | Call Sheet snapshots remain | Yes |
| Call Sheet | Schedule remains | Yes |
| Draft | Other drafts remain | Yes if current/locked |
| Project | May be recoverable; backup first | Strong confirmation |

## §164 Copy vs Reference vs Snapshot (L5258)
- **Copy** = independent; edits don't propagate. **Reference/association** = distinct objects, one points to the other. **Snapshot** = fixed exported/issued state.
- Global Idea → Project Idea = Copy.
- Breakdown scene → Catalog Prop = Reference.
- Shooting Day → Call Sheet = source relationship + snapshot on export.
- Screenplay → Production Source = reference to a selected version/snapshot.
- Board Scene → Screenplay Scene = creation relationship, **not** live sync.

## §165 Cross-Module Regression Matrix (L5277)
| Change | Verify now | Verify downstream | Must NOT change |
|---|---|---|---|
| Move Scene Card | Board order | Next build preview | Existing screenplay text |
| Edit Scene Card | Card content | Build preview if rebuilt | Screenplay automatically |
| Build Screenplay | New draft | Draft history | Idea Vault |
| Edit Screenplay | Current draft | Production stale flag | Board text automatically |
| Lock Draft | Locked state | Production can select source | Older drafts |
| Script revision | New revision | Production update review | Locked source |
| Add breakdown prop | Scene breakdown | Catalog usage | Screenplay prose |
| Rename catalog prop | Catalog displays | Schedule displays | Historical PDF |
| Move schedule scene | Day assignment | Call sheet stale flag | Screenplay |
| Edit call sheet | Call sheet snapshot | None | Schedule |
| Export document | External file | None | Project content |
| Import review package | Review queue | Comments | Current draft text automatically |

## §166 Release-Blocking Defects (L5295) [V1]
- Screenplay text lost after save/reopen.
- Draft history silently overwritten.
- Scene identity changes on ordinary reorder.
- Breakdown lost after a revision.
- A schedule move changes the screenplay.
- A call sheet silently overwrites an issued version.
- Exchange import overwrites the host without confirmation.
- An offline project can't reopen after network loss.
- Collaborator changes silently lost in a conflict.
- AI silently mutates content.
- Private notes appear in external exports without explicit permission.

## §167 QA Data Sets (L5312)
- Short film: 5–15 scenes, 3–8 characters, 2–5 locations.
- Indie feature: 60–120 scenes, 10–30 characters, 15–40 locations.
- Large project: 150–250 scenes.
- Episodic: 1 season, 8 episodes, recurring characters/locations.
- Messy Vault: hundreds to thousands of mixed items with incomplete metadata.
- Import stress: FDX, PDF, Fountain, TXT, DOCX and paste, including malformed files.

## §168 Manual Walkthrough Checklist (L5337)
Without a manual, a reviewer must be able to:
- Create a project; add 5 Vault item types; switch Vault views.
- Create Act/Seq/Beat/Scene; drag reorder; park cards.
- Build screenplay; write scene/dialogue; create a named draft; compare; add/resolve comments; lock and revise; import screenplay.
- Build breakdown; accept/reject suggestions; create locations/cast/props; create moodboard/storyboard/shot list.
- Schedule scenes; trigger/read a conflict; generate a call sheet; export docs.
- Work offline; create/import an exchange package; run a LAN session; use AI with confirmation.

Failing this walkthrough = not functionally complete.

## §169 Vocabulary Guardrails (L5370)
- Use "Scene Cards", "Story Board", "Parking Lot", "Shooting Schedule", "Suggest Elements", "Review Package".
- Never "Narrative Units", "Narrative Architecture Graph", "Inactive Branch Repository", "Production Optimization Engine", "Automated Semantic Breakdown Pipeline", "Distributed Collaboration Artifact".
- Industry terms (INT/EXT, scene heading, stripboard, call sheet) are fine in documents.

## §170 Coverage Statement (L5384)
The PRD is the authority for purpose, users, boundaries and priorities. The FSD is the authority for detailed behavior. The UX spec owns visual composition; the ESD owns implementation.

---

## §171 Complete Functional Requirement Inventory (L5393)
Format: `ID Pri — requirement: contract (verify)`. PRD parent in group header.

### Idea Vault (PRD-IDEA-001..005, PRD-CORE-002)
- FSD-IDEA-001 P0 — Create item: Add creates the chosen type; focus in the least-structured useful input (save without metadata). [IDEA-001]
- FSD-IDEA-002 P0 — Global/project separation: independently editable; copying explicit (edit project copy → global unchanged). [IDEA-001]
- FSD-IDEA-003 P0 — Any file: ordinary attachments accepted without classification (drop mixed set). [IDEA-002]
- FSD-IDEA-004 P0 — Untitled item: store with no title (untitled image/audio/file, reopen). [IDEA-002]
- FSD-IDEA-005 P0 — Multiple views: Grid/Card/List/Folder show the same items (count/content unchanged). [IDEA-003]
- FSD-IDEA-006 P0 — Folder move changes organization only (move then global search). [IDEA-003]
- FSD-IDEA-007 P0 — Collection add/remove never duplicates or deletes. [IDEA-003]
- FSD-IDEA-008 P0 — Pin affects display priority only (file path/content unchanged). [IDEA-003]
- FSD-IDEA-009 P0 — Search: title/body/caption/tag/filename (exact phrase). [IDEA-004]
- FSD-IDEA-010 P0 — Preview: media previews in project context (image/audio/video/PDF). [IDEA-004]
- FSD-IDEA-011 P0 — Note autosave (type, close, reopen). [IDEA-002]
- FSD-IDEA-012 P0 — URL fallback: usable without metadata (network off). [IDEA-002]
- FSD-IDEA-013 P0 — Voice note: record, play later (record/play/reopen offline). [IDEA-002]
- FSD-IDEA-014 P0 — Project copy creates independent content. [IDEA-001]
- FSD-IDEA-015 P0 — Move to story → Beat / Scene / Sequence idea / Character note / Story note (convert each). [IDEA-005]
- FSD-IDEA-016 P0 — Original retention: Vault original never deleted. [IDEA-005]
- FSD-IDEA-017 P0 — No live sync: Story edits never rewrite the Vault. [IDEA-005]
- FSD-IDEA-018 P0 — Soft delete: recoverable (delete/restore). [CORE-002]
- FSD-IDEA-019 P0 — Multi-select for batch organize/delete/export (5 items → collection). [IDEA-003]
- FSD-IDEA-020 P0 — External file awareness: external refs distinguishable (indicator). [IDEA-004]

### Story Board (PRD-STORY-001..005)
- FSD-STORY-001 P0 — Act title creates a top-level container.
- FSD-STORY-002 P0 — Sequence needs only a name ("Hero Introduction").
- FSD-STORY-003 P0 — Sequence contains scenes in order (add 3, reorder).
- FSD-STORY-004 P0 — Beat: small text card, can exist before becoming a scene (create in Parking Lot).
- FSD-STORY-005 P0 — Scene Card needs only a short description; heading optional.
- FSD-STORY-006 P0 — Compact card: short description plus minimal indicators; truncation is visual only.
- FSD-STORY-007 P0 — Double click opens details drawer/editor.
- FSD-STORY-008 P0 — Drag scene within/between sequences.
- FSD-STORY-009 P0 — Drag sequence between acts; children stay.
- FSD-STORY-010 P0 — Drag act with descendants (Act 3 before Act 2).
- FSD-STORY-011 P0 — Multi-select move keeps relative order.
- FSD-STORY-012 P0 — Parking Lot: leave active story without deletion.
- FSD-STORY-013 P0 — Duplicate = new independent identity.
- FSD-STORY-014 P0 — No branch engine; no branch UI appears.
- FSD-STORY-015 P0 — Undo move. FSD-STORY-016 P0 — Undo delete.
- FSD-STORY-017 P0 — Board/Outline parity: same hierarchy and order.
- FSD-STORY-018 P0 — Collapse act (data unchanged). FSD-STORY-019 P0 — Collapse sequence.
- FSD-STORY-020 P0 — Build: choose which active cards are included (subset).
- FSD-STORY-021 P0 — Build preview in current order.
- FSD-STORY-022 P0 — Missing heading flagged before conversion.
- FSD-STORY-023 P0 — Existing screenplay never silently overwritten.
- FSD-STORY-024 P0 — Board independent after build.
- FSD-STORY-025 P0 — Applying a board reorder to a populated screenplay requires confirmation.
- FSD-STORY-026 P0 — No character fields required. FSD-STORY-027 P0 — No story day required.
- FSD-STORY-028 P0 — Sequence has no mandatory purpose/goal field.
- FSD-STORY-029 P0 — Beat → new Scene Card using its text.
- FSD-STORY-030 P0 — Scene → beat convert/duplicate "if UI supports"; source retained. [LATER-ish hedge]

### Screenplay (PRD-SCRIPT-001..006)
- FSD-SCRIPT-001 P0 — Empty screenplay without a board.
- FSD-SCRIPT-002/003/004 P0 — Feature / Short (no extra structure) / Episodic (script inside episode container).
- FSD-SCRIPT-005..011 P0 — Elements: Scene heading (INT/EXT), Action, Character, Dialogue, Parenthetical, Transition (CUT TO), optional Shot direction.
- FSD-SCRIPT-012 P0 — Manual element switching (action→dialogue).
- FSD-SCRIPT-013 P0 — Auto next-element suggestion on Enter; overridable.
- FSD-SCRIPT-014 P0 — Navigator lists scenes; click jumps.
- FSD-SCRIPT-015 P0 — Find with next/previous. FSD-SCRIPT-016 P0 — Replace with options.
- FSD-SCRIPT-017 P0 — Focus mode. FSD-SCRIPT-018 P0 — Writing-room optional panels (comments/scene notes).
- FSD-SCRIPT-019 P0 — Vault never appears or syncs in the writing room.
- FSD-SCRIPT-020 P0 — Scene notes excluded from standard output (PDF).
- FSD-SCRIPT-021 P0 — Autosave. FSD-SCRIPT-022 P0 — Undo/redo.
- FSD-SCRIPT-023 P0 — Named draft from current/selected. FSD-SCRIPT-024 P0 — Lineage shown (3 drafts).
- FSD-SCRIPT-025 P0 — Exactly one named draft is current.
- FSD-SCRIPT-026 P0 — Automatic background recovery history, separate from named drafts.
- FSD-SCRIPT-027 P0 — Compare at scene and text level.
- FSD-SCRIPT-028 P0 — Review linked to a draft. FSD-SCRIPT-029 P0 — Text-anchored comment. FSD-SCRIPT-030 P0 — Scene comment. FSD-SCRIPT-031 P0 — Reply in thread. FSD-SCRIPT-032 P0 — Resolve keeps history.
- FSD-SCRIPT-033 P0 — Private note invisible to unauthorized users and excluded from export.
- FSD-SCRIPT-034 **P1** — Lock → Shooting Draft. FSD-SCRIPT-035 **P1** — Edit gate starts revision. FSD-SCRIPT-036 **P1** — Revision label/color ("Revision A/Blue"). FSD-SCRIPT-037 **P1** — Revision shows changed scenes/content.
- FSD-SCRIPT-038..043 P0 — Import FDX; PDF (preview/warnings); Fountain; TXT; DOCX; paste.
- FSD-SCRIPT-044 P0 — Import never overwrites the current draft by default.
- FSD-SCRIPT-045..048 P0 — Export PDF (professional); FDX (reimport); Fountain (reimport); DOCX (editable).
- FSD-SCRIPT-049 P0 — Title page data (title/author). FSD-SCRIPT-050 P0 — Display numbers derive from order.

### Breakdown (PRD-BRK-001) — all P0
- FSD-BREAKDOWN-001 — Requires a selected source.
- FSD-BREAKDOWN-002 — All source scenes listed (count/headings).
- FSD-BREAKDOWN-003 — Script pane visible while tagging.
- FSD-BREAKDOWN-004 — Manual tag (e.g. pistol prop).
- FSD-BREAKDOWN-005 — Highlight text then tag.
- FSD-BREAKDOWN-006 — Categories limited to the defined default list (list not in Part 2; see Part 1).
- FSD-BREAKDOWN-007 — Suggest elements.
- FSD-BREAKDOWN-008 — Accept → production data.
- FSD-BREAKDOWN-009 — Reject creates no data.
- FSD-BREAKDOWN-010 — Edit category/name before accept.
- FSD-BREAKDOWN-011 — Batch accept after review.
- FSD-BREAKDOWN-012 — Attach an existing catalog item.
- FSD-BREAKDOWN-013 — Create a catalog item from breakdown.
- FSD-BREAKDOWN-014 — Removing an element keeps the catalog item.
- FSD-BREAKDOWN-015 — Manual complete, filterable.
- FSD-BREAKDOWN-016 — Source change flags affected scenes (Needs Review).
- FSD-BREAKDOWN-017 — No silent deletion on a script change.
- FSD-BREAKDOWN-018 — New scene → Needs Breakdown.
- FSD-BREAKDOWN-019 — Heading change flags review.

### Production (PRD-PROD/LOC/CAST/VIS/STB/SHOT)
- FSD-PROD-001 P0 — Catalog exists (create/search). FSD-PROD-002 P0 — Usage lists scenes. FSD-PROD-003 P0 — Simple status.
- FSD-PROD-004 P0 — Location needs only a name. FSD-PROD-005 P0 — Photos. FSD-PROD-006 P0 — Practical notes (parking/noise). FSD-PROD-007 P0 — Linked scenes.
- FSD-PROD-008 P0 — Actor with character association. FSD-PROD-009 P0 — Crew with role/department. FSD-PROD-010 P0 — Availability as notes.
- FSD-PROD-011 P0 — Moodboard create/rename. FSD-PROD-012 P0 — Add/move image/note/link. FSD-PROD-013 P0 — Clean export (PDF).
- FSD-PROD-014 P0 — Storyboard for a scene. FSD-PROD-015 P0 — Panel with visual area plus short description. FSD-PROD-016 P0 — Drag reorder. FSD-PROD-017 P0 — Panel → shot link.
- FSD-PROD-018 P0 — Shot with description only. FSD-PROD-019 P0 — Drag reorder. FSD-PROD-020 P0 — Technical fields optional (no lens). FSD-PROD-021 P0 — Shot list PDF.
- FSD-PROD-022 P1 — Daily view. FSD-PROD-023 P1 — Reports (scene/location/prop). FSD-PROD-024 P1 — Sides PDF.
- FSD-PROD-025 P1 — Budget categories/lines with total. FSD-PROD-026 P1 — No payroll/accounting.

### Schedule (PRD-SCHED-001..002)
- FSD-SCHED-001 P0 — Created from a selected source. FSD-SCHED-002 P0 — All scenes start unscheduled. FSD-SCHED-003 P0 — Create day/date.
- FSD-SCHED-004 P0 — Drag to day. FSD-SCHED-005 P0 — Reorder within day. FSD-SCHED-006 P0 — Move between days. FSD-SCHED-007 P0 — Off day.
- FSD-SCHED-008 P0 — **Break marker: Meal / Travel / Company move / Custom** (only defined here).
- FSD-SCHED-009 P0 — Day estimated total.
- FSD-SCHED-010 P1 — Actor conflict. FSD-SCHED-011 P1 — Location conflict. FSD-SCHED-012 P1 — Overflow. FSD-SCHED-013 P1 — Keep anyway.
- FSD-SCHED-014 P0 — Grouping suggestions, never silent moves. FSD-SCHED-015 P0 — Calendar: dates map to days. FSD-SCHED-016 P0 — Export snapshot PDF.

### Call Sheet (PRD-CALL-001) — all P0
- FSD-CALL-001 — Create from day. FSD-CALL-002 — Prefill scenes. FSD-CALL-003 — Prefill cast where available. FSD-CALL-004 — Prefill location/address.
- FSD-CALL-005 — Edit call times. FSD-CALL-006 — Day notes. FSD-CALL-007 — Optional sections: **weather / attachments / special notes**.
- FSD-CALL-008 — No reverse sync. FSD-CALL-009 — Stale on schedule change. FSD-CALL-010 — Refresh preview. FSD-CALL-011 — Issued snapshot is historical. FSD-CALL-012 — Professional PDF.

### Collaboration (PRD-COL-001..006)
- Roles (all P1): FSD-COL-001 **Owner** (full control) · FSD-COL-002 **Editor** (authorized content) · FSD-COL-003 **Commenter** (review/comment, no core edits) · FSD-COL-004 **Viewer** (read-only) · FSD-COL-005 **Export-only** (permitted snapshot/package only).
- Exchange packages: FSD-COL-006 Story P1 · **FSD-COL-007 Script Review P0** · FSD-COL-008 Breakdown P1 · FSD-COL-009 Shots P1 · FSD-COL-010 Schedule P1 · FSD-COL-011 Call-sheet review P1.
- FSD-COL-012 P1 — Preview before apply (import, cancel). FSD-COL-013 P1 — Stale review recognized. FSD-COL-014 P1 — No blind overwrite. FSD-COL-015 P1 — Ambiguous → review queue. FSD-COL-016 P1 — Unmapped comment kept.
- FSD-COL-017 **P0** — LAN host. FSD-COL-018 **P0** — Join with role. FSD-COL-019 **P0** — Presence.
- FSD-COL-020 P1 — Same-object conflict surfaced.
- FSD-COL-021 **P0** — Usable after disconnect. FSD-COL-022 **P0** — End session never publishes to cloud.

### Offline (PRD-OFF-001) — all P0
- FSD-OFF-001 — Open offline. FSD-OFF-002 — Edit Vault/story/script offline. FSD-OFF-003 — Breakdown/schedule/call sheet offline. FSD-OFF-004 — Export offline.
- FSD-OFF-005 — Save status reflects local persistence. FSD-OFF-006 — Crash recovery to latest safe state. FSD-OFF-007 — Backup package. FSD-OFF-008 — Move to another machine.
- FSD-OFF-009 — External drive works while mounted. FSD-OFF-010 — Drive loss never silently creates a divergent project.

### AI (PRD-AI-001) — all P1
- FSD-AI-001 — Optional. FSD-AI-002 — Context selector: current scene / screenplay / story / project / production.
- FSD-AI-003 — Q&A with no data change. FSD-AI-004 — Propose Scene Card. FSD-AI-005 — Suggest breakdown. FSD-AI-006 — Synopsis. FSD-AI-007 — Schedule grouping advice.
- FSD-AI-008 — Preview for every mutation. FSD-AI-009 — Accept/reject. FSD-AI-010 — No silent screenplay rewrite. FSD-AI-011 — External disclosure. FSD-AI-012 — Failure leaves data unchanged.
- FSD-AI-013 — Natural-language command interface for supported navigation/app requests.
- FSD-AI-014 — Exact counts/metrics come from canonical project queries (deterministic).
- FSD-AI-015 — Cross-module relationship queries. FSD-AI-016 — NL navigation routes to existing app actions.
- FSD-AI-017 — Structured rename separates structured references from arbitrary script text.
- FSD-AI-018 — Batch proposals show affected counts, exclusions and conflicts before apply.
- FSD-AI-019 — Never exceeds the user's permissions (Viewer/Commenter test). FSD-AI-020 — Respects locked content, private notes and approval gates.
- FSD-AI-021 — **Change Set base-version validation**: stale proposals are revalidated before apply.
- FSD-AI-022 — AI cannot write project storage directly; tool calls are validated.
- FSD-AI-023 — Product-knowledge Q&A. FSD-AI-024 — Scoped answers disclose scope/source.
- FSD-AI-025 — Accepted AI mutations appear in normal history/undo. FSD-AI-026 — Follow-ups reuse session context without mutating.
- FSD-AI-027 — Keeps project boundaries and explicit episode/series scope.

## §172 Field Validation Matrix (L5630)
| Object.Field | Req | Validation | Used in |
|---|---|---|---|
| Project.Title | Req | 1+ readable chars | Create/open/home |
| Project.Type | Req | `Feature / Short / Episodic` | Workflow |
| Project.Language | Opt | Free text | Metadata |
| Project.Genre | Opt | Free text / selected tags | Metadata |
| Project.Status | Opt | Defined lifecycle labels | Home |
| Idea.Type | Req | Supported type | Rendering |
| Idea.Title | Opt | May be blank | Vault |
| Idea.Body/Caption | Opt | Plain text | Preview/search |
| Act.Title | Req | Non-empty | Board |
| Sequence.Name | Req | Non-empty | Board |
| Beat.Text | Req for useful beat | Non-empty after save unless intentionally parked | Board |
| SceneCard.Description | Req for active scene | Text | Board/build |
| SceneCard.Heading | Opt | Valid heading when used | Build |
| Character.Name | Req | Non-empty | Directory |
| Draft.Name | Req | Non-empty | History |
| Review.SourceDraft | Req | Existing draft | Review |
| BreakdownElement.Category | Req | Default category | Breakdown |
| BreakdownElement.Name | Req | Non-empty | Catalog |
| CatalogItem.Name | Req | Non-empty | Production |
| CatalogItem.Status | Req | Allowed status | Production |
| Location.Name | Req | Non-empty | Production |
| Person.Name | Req | Non-empty | Cast/Crew |
| Person.Role/Department | Req for crew | Non-empty | Crew |
| Shot.Description | Req | Text | Shot list |
| StoryboardPanel.Description | Opt | Text | Storyboard |
| ShootingDay.Date | Req | Valid date | Schedule |
| ShootingDay.DayNumber | Req | Positive integer/display order | Schedule |
| CallSheet.SourceDay | Req | Existing day | Call sheet |
| Task.Title | Req | Non-empty | Task |
| BudgetLine.Description | Req | Text | Budget |
| BudgetLine.Amount | Req | Non-negative monetary | Budget total |

## §173 Cross-Module Event Contract (L5667)
- **EVT-001 Scene Card moved:** order/parent change; undo entry; screenplay untouched.
- **EVT-002 Duplicated:** new identity; content copied; no screenplay relationship.
- **EVT-003 Parked:** excluded from active outline; kept in Parking Lot.
- **EVT-004 Restored:** placed at a user-selected insertion point; identity kept.
- **EVT-005 Build confirmed:** new screenplay/draft from selected cards; existing drafts untouched.
- **EVT-006 Existing-script reorder request:** warning before applying board order to a written script.
- **EVT-007 Draft created:** parent unchanged; current/non-current applied per selection.
- **EVT-008 Locked:** shooting baseline; editing prompts revision.
- **EVT-009 Production source updated:** scene diff shown; affected objects flagged, not deleted.
- **EVT-010 Scene removed from source:** production records keep historical mapping, flagged (Breakdown/Schedule).
- **EVT-011 Scene added:** enters production review / unscheduled.
- **EVT-012 Catalog attached:** scene association; identity reusable.
- **EVT-013 Catalog renamed:** active refs show the new name; historical exports unchanged.
- **EVT-014 Schedule scene moved:** day totals update; the generated call sheet goes stale.
- **EVT-015 Schedule date changed:** call sheet from the affected day goes stale.
- **EVT-016 Call sheet edited:** call sheet only; schedule unchanged.
- **EVT-017 Call sheet refreshed:** preview, then the new snapshot state is saved.
- **EVT-018 Export complete:** snapshot; no project mutation.
- **EVT-019 Exchange imported:** preview, then selected changes/comments added; no blind overwrite.
- **EVT-020 AI mutation accepted:** normal undoable action.

## §174 Canonical Scenario Catalog (L5693) — Given → Action → Expected
- **SC-001:** no project → create with title/type → opens at Project Home with empty workspaces and **three clear next actions**.
- **SC-002:** global item → copy to Project A → independent copy in Project Vault.
- **SC-003:** mixed files → add 10 items untagged/untitled → all stored and searchable.
- **SC-004:** empty board → Act + Sequence + 5 cards → compact, ordered, drag-editable.
- **SC-005:** short project → Act + cards directly, no sequence → build still possible.
- **SC-006:** park 2, restore 1 → parked excluded; restored returns.
- **SC-007:** valid headings → build → scenes in card order, generated numbers.
- **SC-008:** one card missing a heading → build → prompt to supply or exclude.
- **SC-009:** new project → screenplay directly → normal writing; board optional.
- **SC-010:** no script → import FDX → draft with parsed scenes plus import summary.
- **SC-011:** Draft 1 → create D2, edit, compare → D1 unchanged; differences shown.
- **SC-012:** Draft 3 → review plus comments → attached and resolvable.
- **SC-013:** Draft 5 → lock, edit, revision → locked source intact; revision separate.
- **SC-014:** Shooting Draft → open Breakdown → all scenes with source label.
- **SC-015:** obvious prop/location/cast → suggest, accept/reject → only accepted become data.
- **SC-016:** prop in catalog → tag in a 2nd scene → same catalog item referenced.
- **SC-017:** location → photos, notes, confirm → shows scenes and notes.
- **SC-018:** scene → storyboard + shot list → both reference the scene; neither requires the other.
- **SC-019:** script + data → days + drag → assignment and totals update.
- **SC-020:** actor in 2 locations, overlap → advisory warning; can keep.
- **SC-021:** Shoot Day 4 → call sheet, call times, export → PDF snapshot with day data.
- **SC-022:** issued call sheet → move scene → issued file stays; stale/refresh state.
- **SC-023:** network off → open/edit/export → works.
- **SC-024:** export package → import on 2nd machine → opens as a working copy.
- **SC-025:** remote reviewer → export, comment, import response → mapped or queued; no blind overwrite.
- **SC-026:** 2 machines on a LAN → host/join/edit → changes visible; ends without cloud.
- **SC-027:** AI configured → suggest, accept 2 → only those 2 enter breakdown.
- **SC-028:** no network/AI → ask AI → clear failure; project unchanged.
- **SC-029:** card that created a scene → delete card → screenplay scene remains and is findable.
- **SC-030:** deleted location → restore → returns, or goes to unassigned if the container is gone.

## §175 Canonical Journey Day 1 → Shoot (L5729) — expected states
- **Day 1:** create project → immediately enters Project Idea Vault. Add text, screenshots, video ref, PDF, voice note and handwritten sketch with no title or classification. On reopen: all present, **opens at last meaningful location**, no setup wizard.
- **Story:** Act 1, Sequence `Hero Introduction`, 6 description-only cards, drag, park 2. Result: no manual numbering and no character/production metadata.
- **Build:** add headings → Build Screenplay → preview order → confirm new draft. Result: generated numbers; board stays an independent reference.
- **Writing:** named drafts; review round; text/scene comments; resolved comments stay in history.
- **Lock:** later edits start a revision; label/color assignable.
- **Production:** locked/revised script as Production Source; accept useful suggestions → Catalog; add locations/cast.
- **Visual:** moodboards, storyboard for the key scene, shot list; same scene identity throughout.
- **Schedule:** days, drag scenes; actor/location/duration warnings; never forced.
- **Call sheet:** day → generate → call times + parking instructions → PDF → user sends it through their own channel (no in-app send).

## §176 Test Oracle (L5768)
- **Correctness:** the action occurs. **Persistence:** it survives reopen. **Identity:** a move never creates a new identity. **Safety:** no silent destruction.
- **Separation:** no cross-module mutation unless documented. **Traceability:** production data knows its screenplay source.
- **Export fidelity:** selected content only; private/internal excluded. **Offline ownership:** core works offline. **Human control:** suggestions are not decisions.

## §177 Allowed Open Questions for UX/ESD (L5791) — the only permissible gaps
1. Panel dimensions/responsive layout.
2. Shortcut assignments.
3. Revision-color/badge visuals.
4. Project package extension/naming.
5. Exchange package extension/naming.
6. Offline storage implementation.
7. LAN transport/hosting.
8. AI provider integration.
9. FDX/PDF/DOCX parsers.
10. PDF/document rendering.

Anything else outside PRD+FSD+UX is logged as a spec question, never invented.

---

## Contradictions and Ambiguities (with source lines)
1. **Requirement IDs collide.** §68–70 and §171 give the same IDs different meanings:
   - FSD-IDEA-002/003/004 (L2564–2566 vs L5398–5400).
   - FSD-STORY-003/004/005 (L2569–2571 vs L5419–5421).
   - FSD-SCRIPT-002..006 (L2573–2577 vs L5448–5452).
   - FSD-CALL-002 (L2596 "no schedule rewrite" vs L5559 "prefill scenes").
   - FSD-COL-001..004 (L2605–2608 vs L5570–5573).
   - FSD-AI-002 (L2610 vs L5603).
   - §69 uses a separate prefix family (FSD-BRK/CAT/LOC/CAST/SHOT/STB/SCH) from §171 (FSD-BREAKDOWN/PROD/SCHED).
   - TEST-ID mapping needs a canonical choice.
2. **Priority conflicts:**
   - Script lock: FSD-SCRIPT-006 P0 (L2577) vs FSD-SCRIPT-034/035 P1 (L5480–5481).
   - LAN session: FSD-COL-003 P1 (L2607) vs FSD-COL-017..019/021/022 P0 (L5586–5591).
   - No-blind-overwrite: P0 at L2606 and release-blocker at L5305, vs FSD-COL-014 P1 (L5583).
   - Locations/Cast: FSD-LOC-001/FSD-CAST-001 P1 (L2588–2589) vs FSD-PROD-004..010 P0 (L5519–5525).
   - Storyboard: FSD-STB-001 P1 (L2591) and "Optional" in every project type (L2714), vs FSD-PROD-014..017 P0 (L5529–5532).
   - Batch accept: "can be supported" (L3547) vs FSD-BREAKDOWN-011 P0 (L5507).
3. **Call Sheet status vocabulary is inconsistent.**
   - §72 `Draft/Final/Superseded/Archived` (L2667).
   - §136.5 `None, Draft, Source Changed, Ready, Finalized, Superseded` (L4790).
   - "Needs Refresh" (L2211, L4975–4981); "Source changed." (L3713); "potentially stale" (L3686); "Final/Issued" (L4991).
   - Stale/Needs Refresh is not in §72; "Ready" has no defined meaning.
4. **Deleting a locked draft:** §142 blocks it (L4896); §163 only warns "Yes if current/locked" (L5254).
5. **Shooting Day deletion:** Appendix B says Delete = Archive (L3111), but §163 describes deleting a Shooting Day with a warning (L5252).
6. **Catalog Item delete:** §71 "archive preferred; scene associations protected" (L2630) vs §163 "scene associations removed/archived only with confirmation" (L5250). Whether associations can be removed at all is unclear.
7. **Project status is ambiguous.** §72 lifecycle `Active/Archived/Closed` (L2640) vs §115 user-controlled "phase" label with suggested transitions (L3922–3934). §134.1/§172 Status is optional with "Defined lifecycle labels" (L4688, L5638). The phase label set is never defined.
8. **Draft statuses are not mutually exclusive.** `Current / Named` (L2649) vs "One named draft is current" (L5471). There is no "Revision" status, though revisions are separate documents (L3491).
9. **Breakdown status sets differ.** §110.6 "unreviewed" (L3841) and §136.3 "Needs Review"/"Needs Breakdown" (L4784, L2159) are not in the §72 Breakdown Element set (L2655). Scene-level vs element-level status is never separated.
10. **Location status:** §72 Location statuses (L2661) are separate from Catalog Item statuses (L2658). It is unclear whether a Location is a Catalog Item. Location status is optional in §134.13 (L4734), while Catalog status is required (L5653).
11. **Comment states:** "Unmapped Review Note" (L5034) and "Review Queue" are not in §72 comment states (L2652). Roles: "Reviewer" (§141, L4880–4882) is not among the defined roles Owner/Editor/Commenter/Viewer/Export-only (L5570–5574).
12. **Beat text rule:** "Required" (L4710) vs "non-empty after save unless intentionally parked" (L5644).
13. **Storyboard panel description:** optional (L4743, L5658) vs "Panel has visual area and short description" (L5530) and §102.2 (L3632).
14. **Primary action labels differ.**
    - Screenplay: "Write / Import" (L2732) vs "New Screenplay / Import" (L2347) vs "Write/Edit" (L4760).
    - Call Sheet: "Create / Update" (L2740) vs "Create Call Sheet" (L4766).
    - Vault Add menu includes Video (L3286), but Appendix C has no "Add Video" (L3119–3126).
15. **Restore target:** EVT-004 "user-selected insertion point" (L5674) vs §52.5 "previous container/order" (L2119).
16. **Undefined items:**
    - Breakdown default category list (FSD-BREAKDOWN-006, L5502) is not defined in Part 2.
    - "Approved" revision (L2131, L5199) has no approval action or state defined.
    - Review round status is "generated/controlled" (L4725) with no values.
17. **Numbering defects:** §116 missing (unnumbered block at L3937); §127 subsections reuse 127.1–127.4 (L4146–4159); performance items numbered 129.x under 127.3 (L4183–4195); no §128/§129 headings. The §171 table header has 6 columns but its separator has 5 (L5395–5396). The embedded word/line counts disagree (L3192, L4674, L5391, L5809). A stray citation artifact appears at L2930/L2934.
