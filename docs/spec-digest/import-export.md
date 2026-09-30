# Digest — OpenFrame Studio Import & Export Specification

Source: `C:\dev\OpenFrame\OpenFrame_Studio_Import_Export_Specification.md` (1467 lines, ~5.1k words, read in full).
Cross-references: `OpenFrame_Studio_Offline_Collaboration_Specification.md` (cited as **OC §n**).
Section refs below are to the Import/Export spec (**§n**) unless marked OC.

**How to read this digest**
- Everything here is sourced from the spec. Text in "quotes" or `> blocks` is copied exactly from the spec.
- **[NOT SPECIFIED]** means the spec says nothing on the point. Do not treat anything under that tag as a requirement. It needs a product/design decision or another source document (FSD / UX-UI / Domain-Data).
- The spec is a *behavioral contract*. It does **not** define byte formats, file layouts, manifest schemas, page geometry, fonts, CSV columns or file-naming patterns. Section 17 of this digest lists these gaps.

---

## 0. Scope and authority (§0.1)

- The document owns the following behavior: screenplay import, screenplay export, Story Board/production document export, Full Project Package export/import, backup-package semantics, review/exchange packages, package metadata, package validation, import preview, identity mapping, stale-source handling, ambiguous mapping, unmapped review material, partial import, round-trip expectations, export snapshots, private-content exclusion, external-file behavior, project collision, error recovery, and AI interaction with import/export.
- Authority order: PRD owns product scope and priority. FSD owns application behavior and acceptance. UX/UI owns presentation. Domain/Data owns logical identities and source-of-truth relationships. AI spec owns AI behavior. This document "joins those specifications; it does not replace them."
- Platform: Windows + macOS desktop. Local-first and offline-capable, with no mandatory OpenFrame cloud. External AI is optional and LAN collaboration is optional.

## 1. Governing principles (§0.2), verbatim list

1. **Export creates a snapshot.**
2. **Import is a deliberate user operation.**
3. **An ordinary document export never mutates the project.**
4. **A review/exchange package is not a live synchronized object.**
5. **A full project package is different from a review package.**
6. **A backup package is different from an ordinary document export.**
7. **Imported content is never silently assumed to be identical to current project content.**
8. **Existing project data must not be blindly overwritten.**
9. **Stable object identities must be preserved where the package represents the same object.**
10. **Copies must receive new identities where the package represents a new copy.**
11. **Historical snapshots must not be silently rewritten.**
12. **Private notes are excluded from normal exports and exchanges unless a future explicitly approved workflow says otherwise.**
13. **Missing external references are reported, not silently replaced or deleted.**
14. **Validation occurs before mutation.**
15. **Any import that can mutate project data must expose the proposed result before application.**
16. **AI may help inspect, explain, search, map, or prepare an import/export action, but may not bypass these rules.**

## 2. Taxonomy: four distinct portability concepts (§1)

"These must never be presented as interchangeable concepts." Implication: separate menu entries, dialog titles and summaries for each.

| Concept | Nature | Purpose | Examples |
|---|---|---|---|
| **A. Document Export** | Human-readable or interchange document snapshot | Distribution and interchange | Screenplay PDF, FDX, Fountain, DOCX, Story Board PDF, Outline PDF, Breakdown PDF/report, Catalog CSV/XLSX-style export, Shot List PDF, Storyboard PDF/image presentation, Schedule PDF, Schedule spreadsheet export "where practical", Call Sheet PDF, Sides PDF, Reports/approved document exports |
| **B. Exchange Package** | Selected project subset | Review or controlled collaboration without OpenFrame cloud | Story Board package, Screenplay review package, Breakdown package, Shot List package, Schedule package, Call Sheet review package |
| **C. Full Project Package** | Portable working project transfer | Move a working project to another installation. "It is not merely a PDF collection and is not a review package." | — |
| **D. Backup Package** | Recovery-oriented copy of supported project state | Backup, recovery, disaster avoidance, migration safety, pre-risk-operation protection | — |

- An Exchange Package "may contain editable project objects, comments, annotations, and selected attachments, but it remains a portable package until deliberately imported."
- A backup "should not be treated as a collaboration response by default."
- Terminology (§7.1): the PRD label **Project Archive** and the FSD/UX term **portable project package / Full Project Package** mean the same thing. Neither refers to a review Exchange Package or an ordinary document export.

## 3. Source-of-truth rules for export (§2)

| Output | Canonical source |
|---|---|
| Screenplay PDF | Selected Screenplay Draft |
| FDX | Selected Screenplay Draft |
| Fountain | Selected Screenplay Draft |
| DOCX screenplay | Selected Screenplay Draft |
| Story Board PDF | Current Story Board state |
| Outline PDF | Current Story Board outline projection |
| Breakdown PDF | Breakdown + selected Production Source context |
| Catalog export | Production Catalog |
| Shot List PDF | Shot List |
| Storyboard PDF/image | Storyboard |
| Schedule PDF | Shooting Schedule |
| Schedule spreadsheet | Shooting Schedule |
| Call Sheet PDF | Call Sheet document |
| Sides PDF | Selected source draft + selected scene set |
| Report export | Report definition + selected data scope |
| Full Project Package | Entire supported project state |
| Review Exchange Package | Explicitly selected workspace/scope |

- The exporter "must not silently choose an unrelated draft merely because it is the most recently edited file."
- "When a choice materially matters, the export UI shows the source draft/version."

## 4. Document export contract (§3)

### 4.1 Lifecycle (§3.1)
`Select source → Select scope → Select format/options → Generate preview where applicable → Export → Create immutable outgoing snapshot → Report output path/result`

- The final step reports the **output path/result** to the user.

**Export must NOT:** edit the source; renumber stored identities; create a new canonical draft; modify the Story Board; update production data; update schedule; update call sheets; delete anything.

### 4.2 Scope selection (§3.2)
"Where practical" the following scopes are supported: current item; selected items; current section; selected scenes; current shooting day; selected shooting days; entire document; entire supported project.
- "The scope must be visible before export."

### 4.3 Private content (§3.3, §27)
- Private notes are excluded by default.
- The export summary should clearly say:
  > Private notes excluded.
- "The system must not accidentally include a user's private notes in a document because they happened to be present in the current workspace."

### 4.4 Comments (§3.4)
- Comments may be included only where the format/workflow supports them **and** the user explicitly chooses inclusion.
- "Normal screenplay PDF export does not silently insert internal review comments into the screenplay document."

### 4.5 Attachments (§3.5)
- Attachments are not embedded automatically in ordinary document exports. They are embedded only when the selected export type explicitly supports attachments.

### 4.6 Snapshot semantics
- Document exports are "snapshots of the source state at export time" (§1.1). They do not live-sync (IEX-008).

---

## 5. Per-format matrix

Legend: **Dir** = direction (I = import, E = export). "Preserve" lists what the spec requires to be kept. "Lost/omitted" lists what the spec permits to be dropped.

### 5.1 Screenplay formats

| Format | Dir | Preserve | Lost / omitted | Parsing / heuristics | Round-trip (§23) |
|---|---|---|---|---|---|
| **Fountain** | I | "Where valid Fountain structure exists, it should become equivalent OpenFrame screenplay elements." (§4.9) | [NOT SPECIFIED] | Fountain "has explicit screenplay-oriented conventions" (§4.9). The spec does not list the rules (for example forced headings or boneyard). | Exported Fountain "should parse back into equivalent screenplay elements" |
| **Fountain** | E | "preserve screenplay semantics through standard Fountain representations" (§5.4) | "Internal application metadata that is not screenplay content should not be silently inserted into screenplay text." | — | same |
| **FDX** (Final Draft `.fdx`) | I | "map supported screenplay structures to OpenFrame screenplay elements as directly as possible" (§4.10) | "Unsupported FDX-specific metadata may be retained as import metadata or ignored according to the documented import result, but it must not silently corrupt screenplay content." | Direct structural mapping | Exported FDX "should be re-importable with screenplay structure substantially intact" |
| **FDX** | E | "preserve screenplay structure that maps to FDX" (§5.3) | "Unsupported OpenFrame metadata remains internal and must not block ordinary FDX export." | — | same (IEX-005) |
| **DOCX** | I | "preserve screenplay structure where recognizable" (§4.7) | — | DOCX "may contain readable screenplay formatting but may also contain ordinary document content". The importer must "expose uncertainty where it cannot" recognize structure. | "Export/re-import should preserve readable screenplay content even if some layout details vary" |
| **DOCX** | E | "an editable ordinary document while preserving readable screenplay structure and layout" (§5.5) | Some layout details may vary on round-trip | — | IEX-007 |
| **TXT** | I | Screenplay-like headings and element patterns "where possible" (§4.8) | — | "TXT may have weak structural signals." The importer should "detect screenplay-like headings and element patterns where possible." It "must not claim guaranteed screenplay fidelity when structure is ambiguous." | [NOT SPECIFIED] (no TXT export) |
| **Pasted screenplay text** | I | Same as TXT | — | "follows the same parsing/preview behavior as TXT." "The user must be able to cancel without changing the current project." (§4.11) | n/a |
| **PDF** | I | Best effort | Semantics may be missing: "PDF is presentation-oriented and may not contain machine-readable screenplay semantics." | Rules (§4.6): parse as best effort; "show parsing confidence/warnings through observable UI states"; "require preview before import"; "preserve the original PDF as a source file if the user chooses to keep it"; "do not represent uncertain parsing as certain". A PDF "that cannot be confidently interpreted as screenplay content must be allowed to remain a normal project file where appropriate" (§4.2). | "Primarily presentation/distribution; editable screenplay round-trip is best-effort with warnings" |
| **PDF** | E | See §6 of this digest (layout) | Internal review comments are not inserted silently. Unsupported metadata is omitted. | — | — |

- Supported screenplay **import** inputs (§4.1): PDF; Final Draft `.fdx`; Fountain; TXT; DOCX; pasted screenplay text.
- Supported screenplay **export** outputs (§5.1): PDF; FDX; Fountain; DOCX. TXT is **not** listed as an export format.
- "Round-trip" does not mean every internal OpenFrame field becomes portable through every ordinary document format (§23).

### 5.2 Other document exports (§6). All are export-only snapshots.

| Output | Represents | Rules | Format(s) named |
|---|---|---|---|
| Story Board PDF | Acts; Sequences; Beats; Scene Cards; "selected notes/references where supported" | "It does not create a new Story Board." | PDF |
| Outline PDF | Current Story Board outline projection | — | PDF |
| Breakdown export | "the selected breakdown/source context" | "Suggested versus confirmed states should remain distinguishable where the format can represent them." | "Breakdown PDF/report" |
| Catalog export | Catalog data in "the selected supported tabular format" | "The export is not a live connection to the Catalog." | "CSV/XLSX-style" (§1.1). Exact column set [NOT SPECIFIED] |
| Shot List export | "the selected shots and their scene relationships" | — | PDF |
| Storyboard export | "panels and supported references" | — | "PDF/image presentation" |
| Schedule export | "shooting days, scene assignments, markers, and supported day information" | "It does not alter the schedule." | PDF; "Schedule spreadsheet export where practical" |
| Call Sheet export | "the exact Call Sheet document state being exported" | "It does not update the Shooting Schedule." | PDF |
| Sides | Selected source draft + selected scene set | — | PDF |
| Reports | Report definition + selected data scope | — | "Reports/approved document exports" |

- CSV/XLSX: the spec says only "Catalog CSV/XLSX-style export" and "Schedule spreadsheet export where practical". It defines no columns, encoding, delimiter, header row or sheet structure. **[NOT SPECIFIED]**

### 5.3 Packages (summary; details in §§8–12 of this digest)

| Package | Dir | Contents | Identity on import | Privacy |
|---|---|---|---|---|
| Full Project Package ("Project Archive") | E + I | Entire supported project state (§7.2 list) | Same identity when opened as the same project. New project identity when imported as a copy. | May contain private project data only when the package is explicitly a private transfer/backup |
| Backup Package | E (create) + restore | Enough supported state to restore | Restore semantics [NOT SPECIFIED beyond purpose] | Same as the Full Project Package class |
| Exchange Packages (6 families) | E + I | Selected scope subset | Same-identity update proposals or new copies, via mapping | Private notes excluded |
| Response Package | E (reviewer) + I (author) | Comments, annotations, permitted edits, new review objects | Mapped back to the author's objects | Only explicitly selected comments/notes |

---

## 6. Screenplay PDF / page-layout requirements

What the spec **does** require (§5.2). PDF "is primarily a presentation/distribution snapshot" and should preserve:
- screenplay formatting;
- scene headings;
- dialogue;
- action;
- character names;
- page breaks;
- scene numbers **"where applicable"**;
- title page;
- revision information **"where applicable"**.

Scene numbers (§5.6, FSD-SCRIPT-050, §14.4):
- "Scene numbers are derived from screenplay order."
- Export "must not write those display numbers back into Scene Card identities or mutate scene identity merely because the screenplay is exported."
- "Display numbers are never identity" (invariant 4).

Comments: normal screenplay PDF export does not insert internal review comments (§3.4).

**[NOT SPECIFIED] in this spec**, so it must come from FSD-SCRIPT-045..050 / UX-UI or from a design decision:
- page size (Letter/A4), margins, and indents per element;
- font family and size (for example Courier 12pt);
- page-numbering format and position, and whether the first page is numbered;
- scene-number placement (left/right/both) and locked or omitted numbering (A-numbers);
- revision-mark style (asterisks, colored pages, revision header), revision colors or labels;
- MORE/CONT'D rules and dialogue splitting across pages;
- title-page fields and layout;
- watermarking;
- Sides layout (crossed-out non-selected content, headers).

---

## 7. Screenplay import workflow (§4)

### 7.1 Intent (§4.2)
- Import turns an existing screenplay into OpenFrame screenplay content. "It is not intended to force every document into screenplay structure."

### 7.2 Lifecycle (§4.3)
`Select source → Read source → Detect format → Parse screenplay structure → Build import preview → Show warnings → Choose New Screenplay / New Draft → Confirm import → Create screenplay content`

UX flow (§28): `Select source → Parse → Preview → Warnings → Choose destination → Import → Complete`

### 7.3 Import preview, required fields "where detectable" (§4.4)
- source filename
- source format
- title
- detected scene count
- detected character count
- detected pages
- detected screenplay elements
- parsing warnings
- uncertain headings
- unsupported constructs

PDF preview must also show parsing confidence/warnings (§4.6, IEX-004).

### 7.4 Destination and existing-project safety (§4.5)
- Destination choices: **"New Screenplay / New Draft"** (§4.3).
- If the project already has screenplay content, imported content "must become a new screenplay or new named draft unless the user explicitly chooses another supported destination."
- "The current draft must never be silently replaced." (IEX-003)
- Cancel is always possible without changing the project (§4.11).

### 7.5 Identity on screenplay re-import
- The spec does not describe re-importing an FDX/Fountain file *as an update* to an existing draft. The default is always **new screenplay / new named draft**, which is effectively a copy.
- Non-OpenFrame sources lack identity. §14.3 applies: "mapping must use supported contextual rules and surface ambiguity."
- Scene numbers are never used as the sole basis for identity (§14.4).

### 7.6 Error message (§25)
> "The PDF could not be interpreted as a screenplay. Your current project was not changed."

---

## 8. Full Project Package (§7, §21, §22, §27.3)

### 8.1 Purpose
Moves the user's working project between supported machines, or preserves it as a complete portable project. OC §7.2 flow: `Machine A Project → Full Project Package → Machine B Open as project/copy`, "without needing OpenFrame cloud."

### 8.2 Expected contents "to the extent supported" (§7.2)
- project identity
- project settings
- Idea Vault content within project scope
- Story Board
- Characters and relationships
- Timeline
- Screenplay drafts
- review data
- comments
- production catalog
- breakdown
- locations
- cast/crew
- moodboards
- storyboards
- shot lists
- shooting schedule
- call sheets
- sides/relevant document state
- notes/tasks/activity as supported
- template references where they are project-owned
- supported project files
- snapshots/recovery information needed by the portable project model
- **required package metadata**
- **external-reference manifest**

### 8.3 Project-owned vs external (§7.3, §22)
- The package must clearly distinguish `Project-owned content` from `External reference`.
- "A missing external file must not make the project falsely appear complete."
- "The import report should show the missing reference." (IEX-026)

### 8.4 Identity (§7.4)
- **Open as same project:** "must preserve the original identity when opening it as the same portable project."
- **Import as copy:** "the copied project receives a new project identity while preserving supported internal content as a copied working set."

### 8.5 Project collision (§21, OC §7.3)
When the package has the same project identity as an existing project, the preferred options are:
```text
Open as Copy
Replace Existing After Backup
Cancel
```
- "The default should favor preservation of the existing project." (IEX-025: defaults toward **Open as Copy**)
- Replacing "is a high-impact action and requires explicit confirmation."
- Result message (§25):
  > "The project package was imported as a copy. The original project was not replaced."

### 8.6 Import UX (§28)
`Open Package → Validate → Project Identity → Collision Choice → File/Reference Report → Import`

### 8.7 Pre-import backup (§8.2)
- For "a substantial full-project or change-heavy import", OpenFrame may offer:
  > Create Backup Before Import
- "This is a safety option and does not alter the source package."

### 8.8 Privacy (§27.3)
- The package can contain private project data "only when the package is explicitly understood as a private project transfer/backup rather than an external review package."
- "The export summary should make the package class unambiguous."

---

## 9. Backup Package (§8; OC §6)

- **Purpose:** "Backup is primarily about safety, not interchange." It restores after accidental deletion, storage failure, migration problems, risky import, corruption of the current working copy, or machine change.
- The user can create a backup **at any time** (OC §6.1). This works offline (OC-006, IEX-029).
- **Destination:** user-selected, which may be another disk (OC §6.2).
- **Naming** (OC §6.4): should include project name; date/time; optional user label. The exact pattern and separator are [NOT SPECIFIED].
- **Integrity** (§8.3): "must not claim that external linked files are backed up when they were not copied."
- **Summary categories.** The two specs word these differently:
  - Import/Export §8.3: **included; external; missing; skipped/unsupported**
  - OC §6.3: **included/copied; external; missing; unsupported**
  - Suggested UI labels: *Included*, *External*, *Missing*, *Skipped/Unsupported*. These reconcile the two lists.
- A backup is not a collaboration response by default (§1.4). Backup and document export are distinct operations (invariant 12).
- Restore flow: recovery UX lives in the OC spec (§5, "We found a recent recovery state for this project."). A backup-restore dialog is [NOT SPECIFIED].

---

## 10. Exchange packages (§9–§11, §18–§20)

### 10.1 Core idea (§9.1)
> "I need another person to review or edit this selected part of the project without giving them the entire project or requiring OpenFrame cloud."

Exchange Packages are "the primary remote collaboration transport" (§30). LAN is not required for exchange interop.

### 10.2 Package families and conceptual extensions (§9.2)
| Family | Conceptual extension |
|---|---|
| Story Board package | `.ofstory` |
| Screenplay review package | `.ofscriptreview` |
| Breakdown package | `.ofbreakdown` |
| Shot List package | `.ofshots` |
| Schedule package | `.ofschedule` |
| Call Sheet review package | `.ofcallreview` |

"These extensions remain conceptual. Exact file extensions may change without changing the package model." Extensions for the Full Project Package, Backup and Response packages are [NOT SPECIFIED].

### 10.3 Common exchange metadata / manifest fields (§9.3)
An exchange package should identify:
- package type
- package format/version
- source project identity
- package export timestamp
- source draft/version where applicable
- included object identities
- selected scope
- comments included or excluded
- attachments included or excluded
- base snapshot/version
- originating user context where appropriate

Also required by the validation pipeline: enough metadata to validate "structure", "supported format version", "required metadata" and "project/source identity" (§13). The container format (zip, sqlite, folder), manifest filename and schema, checksums and signing are [NOT SPECIFIED].

### 10.4 Snapshot semantics (§9.4)
- "Once exported, an exchange package does not live-sync with the source project."
- "Changing the original project does not rewrite the package."
- "Changing the package elsewhere does not rewrite the original project."

### 10.5 Review round-trip workflow (§10)
`Author → Select Draft / Scope → Export Review Package → Send by normal file-transfer method → Reviewer imports → Review / comment / annotate → Export Response Package → Author imports response → Validate + map → Preview changes → Author chooses what to apply`

- **Reviewer isolation (§10.1):** the reviewer receives only the data needed for the review. Examples from the spec: script review need not contain production data; breakdown review need not contain the writer's private notes; Shot List review need not contain the entire Idea Vault.
- **Response package (§10.2)** may contain comments; annotations; "selected object edits where package permissions allow"; "newly created supported review objects". "It does not automatically overwrite the author's project."

### 10.6 Per-type contents and import results (§11)

| Type | Contents | Import behavior |
|---|---|---|
| **Story Board** (`.ofstory`) | Acts; Sequences; Beats; Scene Cards; selected comments; selected attachments | Possible results: new outline copy; selected object changes; comments; review record |
| **Screenplay Review** (`.ofscriptreview`) | source draft identity; screenplay snapshot; selected scene scope where applicable; comments/annotations; optional selected attachments | **Default safe import:** attach comments to the matching draft/context; preserve the current screenplay; "surface text/content changes for explicit review" |
| **Breakdown** (`.ofbreakdown`) | source scene identities; breakdown elements; catalog references; selected notes/comments | "Import may prepare production-data updates after validation and user review." |
| **Shot List** (`.ofshots`) | scene references; shots; shot-planning fields; optional Storyboard links; selected attachments/comments | [import specifics NOT SPECIFIED beyond the general pipeline] |
| **Schedule** (`.ofschedule`) | shooting days; scene assignments; schedule markers; day notes; supported planning metadata | "Import must not silently delete or reorder local schedule data." |
| **Call Sheet review** (`.ofcallreview`) | call-sheet snapshot; source Shooting Day context; comments/review notes | "Import feeds review information into the relevant Call Sheet context and does not silently modify the main schedule." |

---

## 11. Import Session (§12)

- Every exchange or full-project import "that can materially affect the project should conceptually pass through an Import Session." (IEX-015)
- "The Import Session is not itself project content." Domain/Data defines it as a temporary review/import context.
- The session holds: source package; detected package type; compatibility status; source identity; source version; mapping results; conflicts; warnings; excluded content; proposed operations; user selections; final import result.

## 12. Validation pipeline, strict order (§13)

1. Open package
2. Validate package structure
3. Validate supported format version
4. Validate required metadata
5. Validate project/source identity
6. Resolve object mappings
7. Detect stale state
8. Detect conflicts
9. Detect ambiguous mappings
10. Detect unmapped content
11. Build proposed Change Set
12. Show preview
13. User decision
14. Apply safe/selected operations

"There must be no path from 'open package' directly to 'overwrite current project.'"
Zero-mutation guarantee (§25.1): if validation fails before application, the result is "**Zero project mutation.**" "This is a core safety requirement." (IEX-032)

Exchange import UX (§28): `Open Package → Validation → Mapping Preview → Changes by Object → Ambiguities/Unmatched → Select → Apply`

## 13. Identity mapping rules, i.e. copy vs update (§14)

| Case | Rule |
|---|---|
| **Same identity** (§14.1) | If the incoming object "represents an existing project object and the package explicitly carries that identity", import "may propose an update to that existing object after validation and approval." → **update proposal** |
| **New identity** (§14.2) | If the content "is intentionally a copy/new object, import creates a new stable identity." → **copy** |
| **Missing identity** (§14.3) | Old or non-OpenFrame source: "mapping must use supported contextual rules and surface ambiguity." The contextual rules themselves are [NOT SPECIFIED]. |
| **Display numbers** (§14.4) | "Scene numbers must not be used as the sole basis for object identity." "A changed scene number caused by reordering does not mean the scene is a new object." |
| **Deleted host object** (§14.5) | "must not silently recreate or overwrite it." Preview message: `> Incoming package refers to an object that no longer exists in the host project.` Then provide "a safe user decision" (the options are [NOT SPECIFIED]). |

Also:
- Principle 9: stable identities are preserved for the same object. Principle 10: copies get new identities.
- User option **"Copy as New"** in the comparison preview (§18) forces a new identity.

## 14. Stale, ambiguous and unmapped handling

### 14.1 Stale (§15)
- Definition: "A package is stale when the package's relevant source/base state is older than the corresponding host state." Example from the spec: `Package based on Draft 3` / `Host currently has Draft 4`.
- UI message:
  > This package was created from an older project state.
- User options: inspect package; import safe comments only; create a separate review record; compare versions; select safe changes; cancel.
- "Staleness does not automatically mean rejection. It means the package requires review."
- The import must show both source and host versions (IEX-017). The stale state must be visible (IEX-018).

### 14.2 Ambiguous (§16)
- A mapping is ambiguous when multiple host objects plausibly match. "The system must not choose one silently."
- Example from the spec: incoming Character "Ravi" vs host "Ravi" and "Ravi Kumar". Route this to an ambiguity/review state (IEX-019).

### 14.3 Unmapped (§17)
When a comment or annotation cannot be matched to its original context:
- retain it;
- place it in an **"Unmatched Review Notes / Review Queue"** area;
- do not discard it;
- do not invent an attachment point.

"The author can manually attach it later." (IEX-020)

## 15. Comparison preview, Change Set and partial import (§18–§20)

### 15.1 Comparison preview (§18). Example summary shape:
```text
14 comments
3 new Scene Cards
1 changed scene description
2 new Shots
0 deletions

Ambiguous:
2

Unmapped:
1

Stale:
Yes
```
- "The user can drill into each category." Changes are grouped by object/module (IEX-021).
- Action choices "where practical": **Import All Safe**; **Comments Only**; **Selected Items**; **Copy as New**; **Review Ambiguous**; **Cancel**.
- Cancel leaves the project unchanged (IEX-022).

### 15.2 Change Set (§19)
- Mutating imports become a Change Set "or equivalent reviewed action". It must identify: source package; source/base version; affected objects; operations; mapping decisions; exclusions; conflicts; user approval; resulting state.
- "The import itself is not 'the change.' The approved Change Set is the change."
- Accepted mutations become normal project actions with history/undo (IEX-031, invariant 14).

### 15.3 Partial import (§20)
- Example buckets: **Safe** (14 comments), **Review required** (3 scene-card updates), **Conflict** (1 schedule change), **Unmapped** (2 annotations).
- Granular safe application is supported "where the package type permits it" (IEX-023). Conflicting operations are not applied silently (IEX-024).
- The system must never report:
  > "Import complete"

  when only a subset was applied.
- **Result states:** **Applied**; **Partially Applied**; **Pending Review**; **Rejected**; **Failed**.
- Message (§25):
  > "Import finished partially. 14 comments were applied; 3 scene changes remain in Review."

---

## 16. External files (§22)

- **Project-owned file:** may become part of the imported project/package, subject to package rules.
- **External reference:** retain the logical reference; report availability; "do not replace it with a guessed local path."
- **Missing reference:** reported as missing. "The project remains openable where supported." (IEX-027)
- **Relinking:** the user may explicitly relink. "Relinking should preserve the logical object identity where possible."

## 17. Unsupported-feature handling (§24)

When a format cannot represent some OpenFrame metadata:
1. preserve it internally when exporting the project itself;
2. omit it from ordinary document output where appropriate;
3. do not silently translate it into misleading screenplay/content data;
4. expose a warning when omission could matter;
5. never block a supported ordinary export merely because unrelated metadata is not representable.

## 18. Errors and messages (§25)

Every error message must identify: **source**; **operation stage**; **issue**; **whether any project changes occurred**.

Exact example messages:
- "The PDF could not be interpreted as a screenplay. Your current project was not changed."
- "Package validation failed because the package is incomplete. Your project was not changed."
- "Import finished partially. 14 comments were applied; 3 scene changes remain in Review."
- "The project package was imported as a copy. The original project was not replaced."

Other exact UI strings:
- "Private notes excluded." (export summary)
- "Create Backup Before Import" (safety option)
- "Incoming package refers to an object that no longer exists in the host project." (preview)
- "This package was created from an older project state." (stale)
- Do not show "Import complete" for a partial apply.
- Result state labels: Applied / Partially Applied / Pending Review / Rejected / Failed.
- Collision options: Open as Copy / Replace Existing After Backup / Cancel.
- Comparison options: Import All Safe / Comments Only / Selected Items / Copy as New / Review Ambiguous / Cancel.
- Screenplay destination: New Screenplay / New Draft.
- Unmapped area: "Unmatched Review Notes / Review Queue".
- Save-state label from OC §4.3: "Saved with pending package operation".

Error codes and taxonomy are [NOT SPECIFIED].

## 19. AI-assisted import/export (§26)

- AI **may:** explain a package; summarize an incoming package; locate likely matches; explain stale/conflict results; prepare export scope; prepare a safe Change Set; suggest mapping for ambiguous content.
- AI **must not bypass:** package validation; project permissions; private-data restrictions; version checks; user preview; explicit application; normal undo/history.
- "Use AI to explain what changed in this review package." is **read-only**.
- "Use AI to merge these package changes." "must still produce a reviewed Change Set and explicit acceptance."
- Offline: "import/export safety must not depend on AI" (§29). If AI mapping is unavailable, deterministic/manual mapping remains available (OC §28).

## 20. Privacy (§27)

| Package class | Rule |
|---|---|
| Normal export | Private notes excluded by default |
| Review package | "Only explicitly selected comments/notes may be included." Private notes stay excluded from ordinary external review packages (IEX-028). |
| Full project package | May contain supported private project data only when explicitly a private project transfer/backup. The summary must make the package class unambiguous. |

## 21. Offline and collaboration (§29–§30)

- All local imports/exports that do not depend on network services work offline. The spec lists: import FDX; import Fountain; export PDF; export FDX; import exchange package; export exchange package; export full project package; create backup.
- Exchange Packages are the primary remote collaboration transport. A LAN participant can fall back to `Local work → Export package → Send normally → Import/reconcile`. No OpenFrame cloud is required.

## 22. Domain objects used (§32)

Exchange Package; Import Session; Snapshot; Change Set; Project; Project File; Screenplay/Draft/Scene/Element; Story Board objects; Production objects; Call Sheet; Shooting Schedule.
Domain/Data defines Exchange Package "as a snapshot rather than a live shared object" and Import Session "as a temporary safe-review context".

## 23. Safety invariants (§33), verbatim

1. Export never mutates the project.
2. Import does not mutate before validation and user decision.
3. Existing data is not blindly overwritten.
4. Display numbering is never identity.
5. Historical snapshots are immutable.
6. Private notes are not included in normal external review packages.
7. Missing external references are reported.
8. Ambiguous mappings are surfaced.
9. Unmapped review notes are retained.
10. Stale packages are identified.
11. Full project import and review exchange are distinct operations.
12. Backup and document export are distinct operations.
13. AI cannot bypass import safety.
14. Accepted imported changes become normal project changes.
15. Import failure before application leaves the host project unchanged.

## 24. Acceptance criteria (§34)

| ID | Criterion |
|---|---|
| IEX-001 | Export screenplay to PDF, FDX, Fountain and DOCX from a selected draft without changing the project |
| IEX-002 | Import PDF, FDX, Fountain, TXT, DOCX and pasted text through a preview workflow |
| IEX-003 | Existing screenplay content is not overwritten by default during screenplay import |
| IEX-004 | PDF parsing uncertainty is shown before import |
| IEX-005 | FDX round-trip preserves supported screenplay structure substantially |
| IEX-006 | Fountain round-trip preserves supported screenplay semantics |
| IEX-007 | DOCX round-trip preserves readable screenplay content |
| IEX-008 | Exported documents are snapshots and do not live-sync |
| IEX-009 | User can export a Story Board PDF |
| IEX-010 | User can export production documents supported by the current product |
| IEX-011 | User can export a Full Project Package |
| IEX-012 | User can create a Backup Package |
| IEX-013 | Full Project Package and Exchange Package are presented as distinct package intents |
| IEX-014 | Review Exchange Package can contain selected scope and selected comments/attachments |
| IEX-015 | Exchange import creates an Import Session |
| IEX-016 | Exchange import validates before mutation |
| IEX-017 | Exchange import shows source and host versions |
| IEX-018 | Stale package state is visible |
| IEX-019 | Ambiguous mappings are not silently resolved |
| IEX-020 | Unmapped review notes are retained |
| IEX-021 | Import preview shows changes grouped by object/module |
| IEX-022 | User can cancel exchange import without changing the project |
| IEX-023 | User can apply selected safe operations without applying unrelated conflicting operations |
| IEX-024 | Conflicting import does not silently overwrite host data |
| IEX-025 | Full project identity collision defaults toward opening as a copy |
| IEX-026 | Missing external references appear in the import report |
| IEX-027 | Imported project data remains openable when supported external files are missing |
| IEX-028 | Private notes are excluded from normal review exchange packages |
| IEX-029 | Offline import/export of supported local formats works without internet |
| IEX-030 | AI can inspect/prepare import/export operations but cannot bypass validation or user approval |
| IEX-031 | Accepted imported mutations become normal project actions with history/undo |
| IEX-032 | Pre-apply import validation failure leaves project content unchanged |

Traceability (§31): PRD-SCRIPT-005/006/007, PRD-COL-002/003/004, PRD-OFF-001; FSD-SCRIPT-038..044 (import), FSD-SCRIPT-045..050 (export and numbering), FSD-COL-005..016, FSD-COL-022, FSD-OFF-004/007/008. "The specification does not create replacement IDs for those existing requirements."

---

## 25. Gaps: [NOT SPECIFIED] in the source (decide or consult FSD/UX/Domain)

| Area | Missing detail |
|---|---|
| Screenplay PDF layout | Page size, margins, per-element indents, font/size, page-number position/format, scene-number placement, revision-mark style/colors, MORE/CONT'D, title-page layout, watermark |
| Sides PDF | Layout, headers, handling of partial scenes |
| Fountain parser | Concrete rules (forced elements, notes `[[ ]]`, boneyard, sections/synopses, title page keys, dual dialogue, centered text, transitions) and how sections/synopses map to Story Board |
| FDX mapping | Element-type mapping table; handling of revisions, ScriptNotes, tags, dual dialogue; FDX version |
| TXT/PDF/DOCX heuristics | Concrete detection rules (INT./EXT. patterns, uppercase character cues, indentation thresholds), confidence scoring model and thresholds |
| Confidence UI | The spec says only "observable UI states"; labels and levels are undefined |
| File naming | Default filenames for document exports and packages (backup naming elements are specified in OC §6.4, but no pattern is) |
| Package container | Physical format (zip/sqlite/folder), manifest filename/schema, format-version scheme, checksums/integrity hashes, encryption/signing |
| Full/Backup/Response extensions | Only the six exchange extensions are named, and "conceptual" |
| CSV/XLSX | Columns, encoding, delimiter, sheet structure for Catalog, Schedule and Reports |
| Breakdown export | How suggested vs confirmed are represented per format |
| Storyboard image export | Image format/resolution/panels per page |
| Deleted-host-object decision | Specific options after the "no longer exists" message |
| Missing-identity contextual rules | Which fields drive matching (name, heading text, order, etc.) |
| Backup restore UI | Restore dialog flow and restore-as-copy vs replace |
| Import report | Exact layout/format of the "File/Reference Report" |
| Comment inclusion per format | Which formats support comments (for example DOCX comments, FDX ScriptNotes) |
| Export-only role | Which package types an Export-only user can create (OC §8.6 says "permitted snapshots/packages") |

Cross-spec wording differences to reconcile:
- Backup categories: "skipped/unsupported" (IE §8.3) vs "included/copied … unsupported" (OC §6.3).
- The PRD's "Project Archive" and the FSD's "Full Project Package" are the same concept (§7.1). Pick one UI label.
