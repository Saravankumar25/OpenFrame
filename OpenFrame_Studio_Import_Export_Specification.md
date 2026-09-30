# OpenFrame Studio — Import & Export Specification

**Document type:** Cross-cutting import, export, project-package, document-snapshot, exchange-package, mapping, validation, round-trip, safety, and portability specification

**Product:** OpenFrame Studio

**Platform:** Windows + macOS desktop application

**Operating model:** Local-first, offline-capable, user-owned project files; optional external AI; optional local-network collaboration; no mandatory OpenFrame cloud

**Source baseline:**
- `OpenFrame_Studio_Mega_PRD_Aligned_Updated(1).md`
- `OpenFrame_Studio_Functional_Specification_Document_Aligned_Updated(1).md`
- `OpenFrame_Studio_UX_UI_Specification_Updated(1).md`
- `OpenFrame_Studio_Domain_Data_Specification_Updated(1).md`
- `OpenFrame_Studio_AI_Specification_Updated(1).md`

**Purpose:** Define the exact conceptual behavior of all supported import/export workflows in OpenFrame Studio, including ordinary document exports, screenplay interchange, full project portability, backups, review/exchange packages, validation, mapping, stale-version handling, conflict safety, private-data boundaries, AI-assisted import/export operations, and round-trip expectations.

**Status:** Cross-cutting import/export baseline. It specializes the behavior already established by the five core specifications and does not redefine product scope.

---

# 0. Specification Contract

## 0.1 What this document owns

This document owns the detailed behavior of:

- importing supported screenplay sources;
- exporting screenplay documents;
- exporting Story Board and production documents;
- full project package export/import;
- backup-package semantics;
- portable review/exchange packages;
- package metadata;
- package validation;
- import preview;
- identity mapping;
- stale-source handling;
- ambiguous mapping;
- unmapped review material;
- partial import behavior;
- round-trip expectations;
- export snapshots;
- private-content exclusion;
- external-file behavior;
- project collision handling;
- import/export error recovery;
- AI interaction with import/export.

The PRD remains authoritative for product scope and priority. The FSD remains authoritative for application behavior and acceptance requirements. UX/UI remains authoritative for how the workflows are presented. Domain/Data remains authoritative for logical identities and source-of-truth relationships. AI remains the cross-cutting contract for AI-assisted behavior.

This document joins those specifications; it does not replace them.

## 0.2 Governing principles

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

---

# 1. Import/Export Taxonomy

OpenFrame uses four distinct portability concepts.

```text
A. Document Export
   ↓
   Human-readable/interchange document snapshot

B. Exchange Package
   ↓
   Selected project subset for review/collaboration

C. Full Project Package
   ↓
   Portable working project transfer

D. Backup Package
   ↓
   Recovery-oriented copy of supported project state
```

These must never be presented as interchangeable concepts.

## 1.1 Document Export

Examples:

- Screenplay PDF
- FDX
- Fountain
- DOCX
- Story Board PDF
- Outline PDF
- Breakdown PDF/report
- Catalog CSV/XLSX-style export
- Shot List PDF
- Storyboard PDF/image presentation
- Schedule PDF
- Schedule spreadsheet export where practical
- Call Sheet PDF
- Sides PDF
- Reports/approved document exports

Document exports are snapshots of the source state at export time.

## 1.2 Exchange Package

An Exchange Package is a portable project-specific subset intended for review or controlled collaboration without requiring OpenFrame cloud infrastructure.

Examples:

- Story Board package
- Screenplay review package
- Breakdown package
- Shot List package
- Schedule package
- Call Sheet review package

An Exchange Package may contain editable project objects, comments, annotations, and selected attachments, but it remains a portable package until deliberately imported.

## 1.3 Full Project Package

A Full Project Package is intended to move a working OpenFrame project to another installation.

It is not merely a PDF collection and is not a review package.

It should contain the supported internal project state needed to reopen the project as a working copy.

## 1.4 Backup Package

A Backup Package preserves a recoverable project state.

It is intended primarily for:

- backup;
- recovery;
- disaster avoidance;
- migration safety;
- pre-risk-operation protection.

A backup should not be treated as a collaboration response by default.

---

# 2. Source-of-Truth Rules for Export

Exports must begin from the correct authoritative source.

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

The exporter must not silently choose an unrelated draft merely because it is the most recently edited file.

When a choice materially matters, the export UI shows the source draft/version.

---

# 3. Document Export Contract

## 3.1 General export lifecycle

```text
Select source
    ↓
Select scope
    ↓
Select format/options
    ↓
Generate preview where applicable
    ↓
Export
    ↓
Create immutable outgoing snapshot
    ↓
Report output path/result
```

Export must not:

- edit the source;
- renumber stored identities;
- create a new canonical draft;
- modify the Story Board;
- update production data;
- update schedule;
- update call sheets;
- delete anything.

## 3.2 Scope selection

Where practical, export supports:

- current item;
- selected items;
- current section;
- selected scenes;
- current shooting day;
- selected shooting days;
- entire document;
- entire supported project.

The scope must be visible before export.

## 3.3 Private content

Private notes are excluded by default.

The export summary should clearly say:

> Private notes excluded.

The system must not accidentally include a user's private notes in a document because they happened to be present in the current workspace.

## 3.4 Comments

Comments may be included only where the export format/workflow supports them and where the user explicitly chooses inclusion.

Normal screenplay PDF export does not silently insert internal review comments into the screenplay document.

## 3.5 Attachments

Attachments are not automatically embedded into ordinary document exports unless the selected export type explicitly supports attachments.

---

# 4. Screenplay Import

## 4.1 Supported inputs

The current product contract supports:

- PDF;
- Final Draft `.fdx`;
- Fountain;
- TXT;
- DOCX;
- pasted screenplay text.

## 4.2 Import intent

Screenplay import is intended to turn an existing screenplay into OpenFrame screenplay content.

It is not intended to force every document into screenplay structure.

A PDF that cannot be confidently interpreted as screenplay content must be allowed to remain a normal project file where appropriate.

## 4.3 Import lifecycle

```text
Select source
    ↓
Read source
    ↓
Detect format
    ↓
Parse screenplay structure
    ↓
Build import preview
    ↓
Show warnings
    ↓
Choose New Screenplay / New Draft
    ↓
Confirm import
    ↓
Create screenplay content
```

## 4.4 Import preview

The preview must show, where detectable:

- source filename;
- source format;
- title;
- detected scene count;
- detected character count;
- detected pages;
- detected screenplay elements;
- parsing warnings;
- uncertain headings;
- unsupported constructs.

## 4.5 Existing project safety

If the project already contains screenplay content, imported screenplay content must become a new screenplay or new named draft unless the user explicitly chooses another supported destination.

The current draft must never be silently replaced.

## 4.6 PDF import

PDF is presentation-oriented and may not contain machine-readable screenplay semantics.

Therefore:

- parse as best effort;
- show parsing confidence/warnings through observable UI states;
- require preview before import;
- preserve the original PDF as a source file if the user chooses to keep it;
- do not represent uncertain parsing as certain.

## 4.7 DOCX import

DOCX may contain readable screenplay formatting but may also contain ordinary document content.

Import should preserve screenplay structure where recognizable and expose uncertainty where it cannot.

## 4.8 TXT import

TXT may have weak structural signals.

The importer should detect screenplay-like headings and element patterns where possible.

It must not claim guaranteed screenplay fidelity when structure is ambiguous.

## 4.9 Fountain import

Fountain has explicit screenplay-oriented conventions.

Where valid Fountain structure exists, it should become equivalent OpenFrame screenplay elements.

## 4.10 FDX import

FDX should map supported screenplay structures to OpenFrame screenplay elements as directly as possible.

Unsupported FDX-specific metadata may be retained as import metadata or ignored according to the documented import result, but it must not silently corrupt screenplay content.

## 4.11 Pasted screenplay text

Pasted text follows the same parsing/preview behavior as TXT.

The user must be able to cancel without changing the current project.

---

# 5. Screenplay Export

## 5.1 Supported outputs

- PDF;
- FDX;
- Fountain;
- DOCX.

## 5.2 PDF

PDF is primarily a presentation/distribution snapshot.

It should preserve:

- screenplay formatting;
- scene headings;
- dialogue;
- action;
- character names;
- page breaks;
- scene numbers where applicable;
- title page;
- revision information where applicable.

## 5.3 FDX

FDX export should preserve screenplay structure that maps to FDX.

Unsupported OpenFrame metadata remains internal and must not block ordinary FDX export.

## 5.4 Fountain

Fountain export should preserve screenplay semantics through standard Fountain representations.

Internal application metadata that is not screenplay content should not be silently inserted into screenplay text.

## 5.5 DOCX

DOCX export produces an editable ordinary document while preserving readable screenplay structure and layout.

## 5.6 Scene numbers

Scene numbers are derived from screenplay order.

Export must not write those display numbers back into Scene Card identities or mutate scene identity merely because the screenplay is exported.

---

# 6. Other Document Exports

## 6.1 Story Board

Story Board PDF is a snapshot of:

- Acts;
- Sequences;
- Beats;
- Scene Cards;
- selected notes/references where supported.

It does not create a new Story Board.

## 6.2 Breakdown

Breakdown export represents the selected breakdown/source context.

Suggested versus confirmed states should remain distinguishable where the format can represent them.

## 6.3 Catalog export

Catalog export represents catalog data in the selected supported tabular format.

The export is not a live connection to the Catalog.

## 6.4 Shot List

Shot List export represents the selected shots and their scene relationships.

## 6.5 Storyboard

Storyboard export represents panels and supported references.

## 6.6 Schedule

Schedule export represents shooting days, scene assignments, markers, and supported day information.

It does not alter the schedule.

## 6.7 Call Sheet

Call Sheet export represents the exact Call Sheet document state being exported.

It does not update the Shooting Schedule.

---

# 7. Full Project Package

## 7.1 Purpose

The Full Project Package exists so the user's working project can move between supported machines or be preserved as a complete portable project.

**Terminology alignment:** The PRD uses the label **Project Archive**, while the FSD/UX/UI use **portable project package / Full Project Package**. In this specification these terms refer to the same project-level portability concept. They do not refer to a review Exchange Package or ordinary document export.

## 7.2 Expected contents

The package should contain, to the extent supported:

- project identity;
- project settings;
- Idea Vault content within project scope;
- Story Board;
- Characters and relationships;
- Timeline;
- Screenplay drafts;
- review data;
- comments;
- production catalog;
- breakdown;
- locations;
- cast/crew;
- moodboards;
- storyboards;
- shot lists;
- shooting schedule;
- call sheets;
- sides/relevant document state;
- notes/tasks/activity as supported;
- template references where they are project-owned;
- supported project files;
- snapshots/recovery information needed by the portable project model;
- required package metadata;
- external-reference manifest.

## 7.3 Unsupported external references

The package must clearly distinguish:

```text
Project-owned content
```

from:

```text
External reference
```

A missing external file must not make the project falsely appear complete.

The import report should show the missing reference.

## 7.4 Project identity

Importing a Full Project Package must preserve the original identity when opening it as the same portable project.

If importing as a copy, the copied project receives a new project identity while preserving supported internal content as a copied working set.

---

# 8. Backup Package

## 8.1 Purpose

Backup is primarily about safety, not interchange.

A backup should preserve enough supported project state to restore the project after:

- accidental deletion;
- storage failure;
- migration problems;
- risky import;
- corruption of the current working copy;
- machine change.

## 8.2 Backup before risky import

For a substantial full-project or change-heavy import, OpenFrame may offer:

> Create Backup Before Import

This is a safety option and does not alter the source package.

## 8.3 Backup integrity

A successful backup operation must not claim that external linked files are backed up when they were not copied.

The backup summary must distinguish:

- included;
- external;
- missing;
- skipped/unsupported.

---

# 9. Exchange Package Contract

## 9.1 Core idea

Exchange packages solve:

> “I need another person to review or edit this selected part of the project without giving them the entire project or requiring OpenFrame cloud.”

## 9.2 Package types

Conceptual package families defined by the PRD are:

```text
.ofstory
.ofscriptreview
.ofbreakdown
.ofshots
.ofschedule
.ofcallreview
```

These extensions remain conceptual. Exact file extensions may change without changing the package model.

## 9.3 Common exchange metadata

An exchange package should identify:

- package type;
- package format/version;
- source project identity;
- package export timestamp;
- source draft/version where applicable;
- included object identities;
- selected scope;
- comments included or excluded;
- attachments included or excluded;
- base snapshot/version;
- originating user context where appropriate.

## 9.4 Package is a snapshot

Once exported, an exchange package does not live-sync with the source project.

Changing the original project does not rewrite the package.

Changing the package elsewhere does not rewrite the original project.

---

# 10. Review Package Workflow

```text
Author
  ↓
Select Draft / Scope
  ↓
Export Review Package
  ↓
Send by normal file-transfer method
  ↓
Reviewer imports
  ↓
Review / comment / annotate
  ↓
Export Response Package
  ↓
Author imports response
  ↓
Validate + map
  ↓
Preview changes
  ↓
Author chooses what to apply
```

## 10.1 Reviewer isolation

A reviewer should receive only the data needed for the intended review.

Examples:

- Script review need not contain production data.
- Breakdown review need not contain writer private notes.
- Shot List review need not contain the entire Idea Vault.

## 10.2 Response package

A response package may contain:

- comments;
- annotations;
- selected object edits where package permissions allow;
- newly created supported review objects.

It does not automatically overwrite the author's project.

---

# 11. Exchange Package Types

## 11.1 Story Board package

Can contain:

- Acts;
- Sequences;
- Beats;
- Scene Cards;
- selected comments;
- selected attachments.

Possible import results:

- new outline copy;
- selected object changes;
- comments;
- review record.

## 11.2 Screenplay Review package

Contains:

- source draft identity;
- screenplay snapshot;
- selected scene scope where applicable;
- comments/annotations;
- optional selected attachments.

Default safe import behavior:

- attach comments to matching draft/context;
- preserve current screenplay;
- surface text/content changes for explicit review.

## 11.3 Breakdown package

Can contain:

- source scene identities;
- breakdown elements;
- catalog references;
- selected notes/comments.

Import may prepare production-data updates after validation and user review.

## 11.4 Shot List package

Can contain:

- scene references;
- shots;
- shot-planning fields;
- optional Storyboard links;
- selected attachments/comments.

## 11.5 Schedule package

Can contain:

- shooting days;
- scene assignments;
- schedule markers;
- day notes;
- supported planning metadata.

Import must not silently delete or reorder local schedule data.

## 11.6 Call Sheet review package

Contains:

- call-sheet snapshot;
- source Shooting Day context;
- comments/review notes.

Import feeds review information into the relevant Call Sheet context and does not silently modify the main schedule.

---

# 12. Import Session

Every exchange/full-project import that can materially affect the project should conceptually pass through an Import Session.

The Import Session is not itself project content.

It exists to hold:

- source package;
- detected package type;
- compatibility status;
- source identity;
- source version;
- mapping results;
- conflicts;
- warnings;
- excluded content;
- proposed operations;
- user selections;
- final import result.

The Domain/Data specification already defines Import Session as a temporary review/import context.

---

# 13. Validation Pipeline

Import must follow:

```text
Open package
    ↓
Validate package structure
    ↓
Validate supported format version
    ↓
Validate required metadata
    ↓
Validate project/source identity
    ↓
Resolve object mappings
    ↓
Detect stale state
    ↓
Detect conflicts
    ↓
Detect ambiguous mappings
    ↓
Detect unmapped content
    ↓
Build proposed Change Set
    ↓
Show preview
    ↓
User decision
    ↓
Apply safe/selected operations
```

There must be no path from “open package” directly to “overwrite current project.”

---

# 14. Identity Mapping Rules

## 14.1 Same identity

If an incoming object represents an existing project object and the package explicitly carries that identity, the import may propose an update to that existing object after validation and approval.

## 14.2 New identity

If the package content is intentionally a copy/new object, import creates a new stable identity.

## 14.3 Missing identity

If an old/non-OpenFrame source lacks OpenFrame identity, mapping must use supported contextual rules and surface ambiguity.

## 14.4 Display numbers are never identity

Scene numbers must not be used as the sole basis for object identity.

A changed scene number caused by reordering does not mean the scene is a new object.

## 14.5 Deleted host object

If the package refers to an object that the host project deleted since package creation, import must not silently recreate or overwrite it.

The preview should say:

> Incoming package refers to an object that no longer exists in the host project.

Then provide a safe user decision.

---

# 15. Stale Package Rules

A package is stale when the package's relevant source/base state is older than the corresponding host state.

Example:

```text
Package based on Draft 3
Host currently has Draft 4
```

The UI should show:

> This package was created from an older project state.

The user may:

- inspect package;
- import safe comments only;
- create a separate review record;
- compare versions;
- select safe changes;
- cancel.

Staleness does not automatically mean rejection. It means the package requires review.

---

# 16. Ambiguous Mapping

A mapping is ambiguous when multiple host objects plausibly match the incoming object.

The system must not choose one silently.

Example:

```text
Incoming: Character “Ravi”
Host:
- Ravi
- Ravi Kumar
```

The import preview should route this to an ambiguity/review state.

---

# 17. Unmapped Material

When an incoming comment or annotation cannot be matched to the original context:

- retain it;
- place it in an Unmatched Review Notes / Review Queue area;
- do not discard it;
- do not invent an attachment point.

The author can manually attach it later.

---

# 18. Comparison Preview

Before applying an exchange package, show a summary such as:

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

The user can drill into each category.

Where practical, the user can choose:

- Import All Safe;
- Comments Only;
- Selected Items;
- Copy as New;
- Review Ambiguous;
- Cancel.

---

# 19. Change Set Application

If import produces content mutations, the result becomes a Change Set or equivalent reviewed action.

The Change Set must identify:

- source package;
- source/base version;
- affected objects;
- operations;
- mapping decisions;
- exclusions;
- conflicts;
- user approval;
- resulting state.

The import itself is not “the change.”

The approved Change Set is the change.

---

# 20. Partial Import

A package may contain mixed safe and unsafe material.

Example:

```text
Safe:
14 comments

Review required:
3 scene-card updates

Conflict:
1 schedule change

Unmapped:
2 annotations
```

OpenFrame should support granular safe application where the package type permits it.

The system must never report:

> “Import complete”

when only a subset was applied.

Use states such as:

- Applied;
- Partially Applied;
- Pending Review;
- Rejected;
- Failed.

---

# 21. Project Collision Rules

When a Full Project Package has the same project identity as a project already present:

Preferred options:

```text
Open as Copy
Replace Existing After Backup
Cancel
```

The default should favor preservation of the existing project.

Replacing an existing working project is a high-impact action and requires explicit confirmation.

---

# 22. External File Rules

The domain model distinguishes project-owned attachments from external references.

## 22.1 Project-owned file

If an imported package contains a project-owned file, the file can become part of the imported project/package subject to package rules.

## 22.2 External reference

If the package contains an external reference:

- retain the logical reference;
- report availability;
- do not replace it with a guessed local path.

## 22.3 Missing reference

Missing external files are reported as missing.

The project remains openable where supported.

## 22.4 Relinking

A user may explicitly relink a missing external file.

Relinking should preserve the logical object identity where possible.

---

# 23. Round-Trip Contract

Round-trip behavior is format-specific.

| Format | Round-trip expectation |
|---|---|
| FDX | Exported FDX should be re-importable with screenplay structure substantially intact |
| Fountain | Exported Fountain should parse back into equivalent screenplay elements |
| DOCX | Export/re-import should preserve readable screenplay content even if some layout details vary |
| PDF | Primarily presentation/distribution; editable screenplay round-trip is best-effort with warnings |
| Exchange package | Source identity and selected comments/objects remain mappable without cloud access |
| Full Project Package | Reopens as a supported working project or safe copy |

“Round-trip” does not mean every internal OpenFrame field becomes portable through every ordinary document format.

---

# 24. Unsupported Feature Handling

If a format cannot represent some OpenFrame metadata:

1. preserve it internally when exporting the project itself;
2. omit it from ordinary document output where appropriate;
3. do not silently translate it into misleading screenplay/content data;
4. expose a warning when omission could matter;
5. never block a supported ordinary export merely because unrelated metadata is not representable.

---

# 25. Import/Export Errors

Error messages must identify:

- source;
- operation stage;
- issue;
- whether any project changes occurred.

Examples:

> “The PDF could not be interpreted as a screenplay. Your current project was not changed.”

> “Package validation failed because the package is incomplete. Your project was not changed.”

> “Import finished partially. 14 comments were applied; 3 scene changes remain in Review.”

> “The project package was imported as a copy. The original project was not replaced.”

## 25.1 Zero-mutation guarantee on pre-apply failure

If validation fails before application:

> **Zero project mutation.**

This is a core safety requirement.

---

# 26. AI-Assisted Import/Export

AI may help with:

- explaining a package;
- summarizing an incoming package;
- locating likely matches;
- explaining stale/conflict results;
- preparing export scope;
- preparing a safe Change Set;
- suggesting mapping for ambiguous content.

AI must not bypass:

- package validation;
- project permissions;
- private-data restrictions;
- version checks;
- user preview;
- explicit application;
- normal undo/history.

Example:

> “Use AI to explain what changed in this review package.”

is read-only.

Example:

> “Use AI to merge these package changes.”

must still produce a reviewed Change Set and explicit acceptance.

---

# 27. Import/Export and Privacy

## 27.1 Normal export

Private notes excluded by default.

## 27.2 Review package

Only explicitly selected comments/notes may be included.

Private notes remain excluded from ordinary external review packages.

## 27.3 Full project package

A full project package can contain supported private project data only when the package is explicitly understood as a private project transfer/backup rather than an external review package.

The export summary should make the package class unambiguous.

---

# 28. Import/Export UX Contract

The current UX patterns should be preserved.

## Import

```text
Select source
→ Parse
→ Preview
→ Warnings
→ Choose destination
→ Import
→ Complete
```

## Exchange import

```text
Open Package
→ Validation
→ Mapping Preview
→ Changes by Object
→ Ambiguities/Unmatched
→ Select
→ Apply
```

## Full project import

```text
Open Package
→ Validate
→ Project Identity
→ Collision Choice
→ File/Reference Report
→ Import
```

## Export

```text
Source
→ Scope
→ Format
→ Options
→ Preview where relevant
→ Export
```

The UX already requires import failure to leave current project content untouched and exchange imports to present a comparison before apply. This document formalizes those existing requirements.

---

# 29. Import/Export and Offline Operation

All supported local imports/exports that do not depend on external network services should work offline.

Examples:

- import FDX offline;
- import Fountain offline;
- export PDF offline;
- export FDX offline;
- import exchange package offline;
- export exchange package offline;
- export full project package offline;
- create backup offline.

External AI assistance may be unavailable offline, but import/export safety must not depend on AI.

---

# 30. Import/Export and Collaboration

Exchange Packages are the primary remote collaboration transport.

LAN collaboration should not be treated as required for exchange-package interoperability.

A collaboration participant can fall back to:

```text
Local work
→ Export package
→ Send normally
→ Import/reconcile
```

No OpenFrame cloud is required.

---

# 31. Traceability to Current Requirements

## PRD

| PRD ID | Relevant contract |
|---|---|
| PRD-SCRIPT-005 | Screenplay Import |
| PRD-SCRIPT-006 | Screenplay Export and Printing |
| PRD-SCRIPT-007 | Additional File Interchange Formats |
| PRD-COL-002 | Review Packages |
| PRD-COL-003 | Page-Specific Exchange Packages |
| PRD-COL-004 | Better Exchange Package Controls |
| PRD-OFF-001 | Offline Operation, Saving, Portability, External Storage and Backup |

## FSD

| FSD ID | Relevant contract |
|---|---|
| FSD-SCRIPT-038..044 | Screenplay import formats and safety |
| FSD-SCRIPT-045..050 | Screenplay export and numbering |
| FSD-SCRIPT-050 | Scene display numbers derive from screenplay order; export does not create new scene identities |
| FSD-COL-005..016 | Exchange packages, preview, stale/conflict/unmapped behavior |
| FSD-COL-022 | Session end / no cloud publication |
| FSD-OFF-004 | Offline export |
| FSD-OFF-007 | Backup |
| FSD-OFF-008 | Full project portability |

The specification does not create replacement IDs for those existing requirements.

---

# 32. Domain Objects Used

The import/export model relies on:

- Exchange Package;
- Import Session;
- Snapshot;
- Change Set;
- Project;
- Project File;
- Screenplay/Draft/Scene/Element;
- Story Board objects;
- Production objects;
- Call Sheet;
- Shooting Schedule.

The Domain/Data specification already defines Exchange Package as a snapshot rather than a live shared object and Import Session as a temporary safe-review context.

---

# 33. Import/Export Safety Invariants

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

---

# 34. Acceptance Criteria

## IEX-001

User can export a screenplay to PDF, FDX, Fountain and DOCX from a selected draft without changing the project.

## IEX-002

User can import PDF, FDX, Fountain, TXT, DOCX and pasted screenplay text through a preview workflow.

## IEX-003

Existing screenplay content is not overwritten by default during screenplay import.

## IEX-004

PDF parsing uncertainty is shown before import.

## IEX-005

FDX round-trip preserves supported screenplay structure substantially.

## IEX-006

Fountain round-trip preserves supported screenplay semantics.

## IEX-007

DOCX round-trip preserves readable screenplay content.

## IEX-008

Exported documents are snapshots and do not live-sync with the project.

## IEX-009

User can export a Story Board PDF.

## IEX-010

User can export production documents supported by the current product.

## IEX-011

User can export a Full Project Package.

## IEX-012

User can create a Backup Package.

## IEX-013

Full Project Package and Exchange Package are presented as distinct package intents.

## IEX-014

Review Exchange Package can contain selected scope and selected comments/attachments.

## IEX-015

Exchange import creates an Import Session.

## IEX-016

Exchange import validates before mutation.

## IEX-017

Exchange import shows source and host versions.

## IEX-018

Stale package state is visible.

## IEX-019

Ambiguous mappings are not silently resolved.

## IEX-020

Unmapped review notes are retained.

## IEX-021

Import preview shows changes grouped by object/module.

## IEX-022

User can cancel exchange import without changing the project.

## IEX-023

User can apply selected safe import operations without applying unrelated conflicting operations.

## IEX-024

Conflicting import does not silently overwrite host data.

## IEX-025

Full project identity collision defaults toward opening as a copy.

## IEX-026

Missing external references appear in the import report.

## IEX-027

Imported project data remains openable when supported external files are missing.

## IEX-028

Private notes are excluded from normal review exchange packages.

## IEX-029

Offline import/export of supported local formats works without internet.

## IEX-030

AI can inspect or prepare import/export operations but cannot bypass validation or user approval.

## IEX-031

Accepted imported mutations become normal project actions with history/undo according to the underlying operation.

## IEX-032

Pre-apply import validation failure leaves project content unchanged.

---

# 35. Final Import/Export Definition

OpenFrame Studio portability is based on a deliberate distinction between:

```text
document snapshot
review exchange
working project transfer
recovery backup
```

The common rule is:

> **Get the project safely into or out of OpenFrame without making the user's existing work ambiguous, silently overwritten, historically rewritten, or dependent on a cloud service.**

The import/export system is successful when a user can move a screenplay, review a Story Board, exchange a breakdown, transfer an entire project, or recover from a backup while always knowing what is being exported, what is being imported, what will change, and what will remain untouched.
