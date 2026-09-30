# OpenFrame Studio — Domain & Data Specification

**Document type:** Domain model, logical data specification, relationship contract, ownership/source-of-truth specification, and cross-document consistency baseline

**Product:** OpenFrame Studio

**Platform:** Windows + macOS desktop

**Operating model:** Local-first, offline-capable, user-owned project files; optional internet-assisted AI; optional local-network collaboration; no mandatory OpenFrame cloud.

**Source documents:** `OpenFrame_Studio_Mega_PRD_Aligned_Updated.md`, `OpenFrame_Studio_Functional_Specification_Document_Aligned_Updated.md`, `OpenFrame_Studio_UX_UI_Specification_Updated.md`, `OpenFrame_Studio_AI_Specification_Updated.md`

**Source sizes inspected:** PRD 5736 lines; FSD 5809 lines; UX/UI 4549 lines; AI 3919 lines.

**Purpose:** Define the logical domain objects, identities, fields, ownership, lifecycle, cardinality, source-of-truth rules, derived relationships, snapshot/version behavior, and cross-module data behavior required to implement the product consistently with the PRD, FSD, and UX/UI specification.

**Important boundary:** This document is logical, not technical. It does not choose databases, programming languages, frameworks, file-system implementation details, APIs, or deployment architecture.

# 0. Domain/Data Authority Contract

The PRD defines product intent, scope, priorities, and boundaries. The FSD defines observable functional behavior. The UX/UI specification defines presentation and interaction. This document defines the logical business/domain objects required to represent that approved behavior without changing product scope.

The domain model must never introduce a new user-visible capability merely because an additional entity would be convenient. Every new persistent domain concept must be traceable to an approved product/functional requirement or be explicitly classified as an internal supporting construct needed to implement an existing behavior.

## 0.1 Non-negotiable source rules

- **Idea Vault:** Global/Project Idea Vault
- **Story Board:** Acts/Sequences/Beats/Scene Cards
- **Screenplay:** Screenplay Draft/Scene/Element
- **Production Source:** Selected Draft
- **Breakdown:** Breakdown Elements/Catalog references
- **Shot List:** Shots
- **Storyboard:** Storyboard Panels
- **Shooting Schedule:** Shooting Days/Markers
- **Call Sheet:** Call Sheet document
The exact source-of-truth rules above are derived from the aligned product documents; the domain model may represent relationships needed to support them but must not collapse distinct sources into one mutable object.

## 0.2 What this specification does not do

- It does not define database tables, indexes, ORM classes, API schemas, network protocols, or serialization technology.
- It does not redesign screens or alter UX behavior.
- It does not introduce studio-scale accounting, payroll, VFX, post-production, distribution, or cloud management.
- It does not make Idea Vault or Story Board live synchronization layers for screenplay content.
- It does not make AI authoritative over creative or production decisions.
# 1. Domain Model Overview

The model is intentionally centered on a small number of stable concepts. Some concepts are user-visible objects; others are supporting records required to preserve versioning, snapshots, exchange safety, and source-of-truth behavior.

The overall logical structure is:

```text
OpenFrame Studio
│
├── Global Idea Vault
│    └── Global Idea Vault Items
│
└── Project
     ├── Project Settings / Status
     ├── Project Idea Vault / Collections
     ├── Project Files
     ├── Story
     │    ├── Acts
     │    │    ├── Sequences
     │    │    │    ├── Beats
     │    │    │    └── Scene Cards
     │    │    └── Scene Cards
     │    ├── Characters / Relationships
     │    └── Story Timeline
     │
     ├── Screenplay
     │    ├── Drafts
     │    │    └── Scenes → Elements
     │    ├── Review Rounds → Comments
     │    └── Locked Draft / Revisions
     │
     └── Production
          ├── Production Source
          ├── Breakdown → Breakdown Elements → Catalog
          ├── Locations / Cast / Crew
          ├── Moodboards / Storyboards / Shots
          ├── Shooting Schedule → Shooting Days → Markers
          ├── Call Sheets / Sides / Daily View
          ├── Reports / Notes / Tasks / Activity / Budget / Templates
          └── Exchange / Collaboration / AI / Snapshots
```
This structure mirrors the source hierarchy while keeping derived views distinct from authoritative objects.

# 2. Identity Rules

Identity is a foundational domain rule. Users may see names and display numbers, but the model must preserve stable identities behind them.

## 2.1 Stable identity

- Every persistent object has a stable identity that does not change when the object is renamed or moved.
- Moving an Act, Sequence, Beat, Scene Card, Shot or Shooting Day changes parent/order context but does not create a new identity.
- Duplicating an object creates a new identity with copied content according to object-specific rules.
- Importing an exchange package must distinguish a copy/new object from a review/change to an existing identity.
- Deleting an object first transitions it to recoverable deleted state where the FSD permits recovery.
- Exported documents are snapshots; exporting does not create a new canonical project object unless explicitly imported back.
## 2.2 Scene identity and numbering

Scene Cards never store user-maintained display scene numbers. A Scene Card has a stable identity. Screenplay Scenes have ordered positions, and display scene numbers are generated from screenplay order for screenplay/production documents. Moving a Scene Card therefore never creates a numbering-maintenance burden.

Production objects reference screenplay scene identity rather than relying on display number as identity. Display number can change without breaking the logical relationship.

## 2.3 Version identity

A named Screenplay Draft is a persistent version identity. An Automatic History Point is recoverable edit history and is not treated as a named deliverable. A locked draft remains a distinct named draft even when later post-lock revisions are created.

# 2.4 Episodic identity objects

### Season
A lightweight grouping object under an episodic/series project. A Season contains Episodes and exists to provide simple season-level ordering without creating a large season-management subsystem.

Fields:

| Field | Meaning | Rule |
|---|---|---|
| season_id | Stable identity | Required; immutable |
| project_id | Parent series project | Required |
| title | Season title/label | Required |
| order_index | Season order | Required |
| note | Optional season note | Optional |

### Episode
A lightweight episodic unit under a Series/Season. Each Episode can own its Story Board, Screenplay, Breakdown and production context.

Fields:

| Field | Meaning | Rule |
|---|---|---|
| episode_id | Stable identity | Required; immutable |
| season_id | Parent Season | Optional when the series allows an episode outside a season during development |
| episode_number | Display/order number | Derived or user-controlled ordering label; not the stable identity |
| title | Episode title | Required |
| summary | Optional one-line summary | Optional |
| status | Development status | Optional/user-controlled |

**Episodic rule:** Episode order/display number is not the episode identity. Reordering an episode must not create a new episode.

# 3. Object Taxonomy

| Class | Objects | Meaning |
|---|---|---|
| Creative capture | Global Idea Vault Item; Project Idea Vault Item; Vault Collection; Project File | Unstructured or lightly organized creative material. |
| Story | Act; Sequence; Beat; Scene Card; Parking Lot Entry; Character; Character Relationship; Story Timeline Entry | Visual outline/reference structures. |
| Script | Screenplay; Screenplay Draft; Screenplay Scene; Screenplay Element; Automatic History Point; Review Round; Comment; Private Note | Written screenplay and its review/version state. |
| Production planning | Production Source; Breakdown; Breakdown Element; Catalog Item; Location; Cast Member; Crew Member | Production needs derived from and attached to screenplay/source. |
| Visual planning | Moodboard; Storyboard; Storyboard Panel; Shot List; Shot | Visual planning/reference objects. |
| Scheduling/documents | Shooting Schedule; Shooting Day; Schedule Marker; Call Sheet; Side; Daily Production View; Production Report | Practical shoot-planning documents and derived views. |
| Project support | Project Note; Task; Activity Entry; Budget Snapshot; Template | Lightweight support workflows. |
| Portability/collaboration | Exchange Package; Import Session; Collaboration Session | Offline-first exchange and temporary LAN collaboration. |
| AI/change/recovery | AI Request; AI Result; Change Set; AI Tool Invocation; Snapshot; Deleted Item | Supporting constructs that preserve explicit user control, tool-bound execution, reviewability, provenance, and recovery. |

# 4. Canonical Object Definitions

The following definitions are normative for engineering and UX behavior. If another document refers to an object by a different name, the terminology must be normalized to this section during implementation.

## Application User

A person using OpenFrame; identity/role context for local ownership or collaboration sessions; not an HR/talent object.

| Field | Meaning | Rule |
|---|---|---|
| user_id | stable local identity | required; immutable |
| display_name | visible user name | required for collaboration contexts |
| roles | one or more project roles | optional |
| local_profile | user preferences | optional |
| permissions | effective access level | derived from role/context |

## Project

The top-level film/short/series workspace containing all project-scoped content and settings.

| Field | Meaning | Rule |
|---|---|---|
| project_id | stable unique identity | required; immutable |
| title | project title | required |
| type | Feature Film / Short Film / Episodic / Series | required |
| status | Project Status | required; user-controlled |
| created_at | creation timestamp | required |
| modified_at | last meaningful modification timestamp | required |
| owner_user_id | owning user | required |
| settings_id | Project Settings link | required |

## Project Settings

User-configurable project-level settings such as type, language, genre, creator and presentation/export defaults.

| Field | Meaning | Rule |
|---|---|---|
| language | project language | optional |
| genre | project genre | optional |
| creator | creator text | optional |
| export_defaults | presentation/export preferences | optional |
| display_preferences | project-specific display preferences where supported | optional |

## Project Status

A manually selected high-level lifecycle label such as Idea, Writing, Shooting Draft or Pre-Production.

| Field | Meaning | Rule |
|---|---|---|
| status_value | enumerated lifecycle label | required |
| user_selected | whether explicitly selected | required |
| updated_at | last change timestamp | required |

## Global Idea Vault Item

A loose creative item stored outside a project and reusable across projects by explicit copy.

| Field | Meaning | Rule |
|---|---|---|
| item_id | stable identity | required |
| item_type | Note/Image/URL/PDF/Document/Audio/Voice/Video/Collection ref/Sketch/Quote/Screenshot/File | required |
| title | user title | optional |
| body | text/caption/description | optional |
| source_reference | URL or file reference | optional |
| attachments | supporting files | optional |
| collections | Vault Collection links | optional |
| tags | user tags | optional |
| created_at | timestamp | required |
| modified_at | timestamp | required |
| deleted_state | Deleted Item link/state | derived |

**Invariants**

- Metadata is optional unless needed for file persistence or navigation.
- An untitled item remains valid.
- An explicit copy into another context does not create live synchronization.
## Project Idea Vault Item

A project-scoped loose creative item copied or created in the current project.

| Field | Meaning | Rule |
|---|---|---|
| item_id | stable identity | required |
| item_type | same item type choices as Global Vault | required |
| title | user title | optional |
| body | text/caption/description | optional |
| source_reference | URL or file reference | optional |
| attachments | supporting files | optional |
| collections | Vault Collection links | optional |
| tags | user tags | optional |
| created_at | timestamp | required |
| modified_at | timestamp | required |
| global_source_item_id | optional source copy identity | optional |

**Invariants**

- Metadata is optional unless needed for file persistence or navigation.
- An untitled item remains valid.
- An explicit copy into another context does not create live synchronization.
## Vault Collection

A user-defined thematic grouping for Vault items; presentation/organization only.

| Field | Meaning | Rule |
|---|---|---|
| collection_id | stable identity | required |
| scope | Global or Project | required |
| name | collection name | required |
| item_membership | links to Vault items | optional |

## Project File

A miscellaneous project document/attachment kept in the project file cabinet.

| Field | Meaning | Rule |
|---|---|---|
| file_id | stable identity | required |
| name | display filename | required |
| file_type | document/media/file type | required |
| content_reference | local project-owned file reference | required |
| notes | optional description | optional |
| created_at | timestamp | required |
| modified_at | timestamp | required |

## Act

A high-level Story Board container.

| Field | Meaning | Rule |
|---|---|---|
| act_id | stable identity | required |
| project_id | owning project | required |
| title | act title | required |
| note | optional note | optional |
| order_index | sibling order | required |

**Invariants**

- Order is relative to sibling Acts.
- Child objects retain identity when Act moves.
## Sequence

A simple named container used to group related story cards/scenes.

| Field | Meaning | Rule |
|---|---|---|
| sequence_id | stable identity | required |
| act_id | parent Act | required unless temporary/unassigned |
| title | sequence text field | required |
| note | optional note | optional |
| order_index | sibling order | required |

**Invariants**

- Primary user-facing content is the sequence name.
- Sequence is a Story Board grouping aid, not a screenplay sequence object.
## Beat

A small story event or idea that may remain a beat or be converted into a Scene Card.

| Field | Meaning | Rule |
|---|---|---|
| beat_id | stable identity | required |
| parent_type | Act/Sequence/Parking Lot | required |
| parent_id | parent container | required |
| text | beat text | required or initially blank during inline creation |
| note | optional note | optional |
| color | optional user color | optional |
| attachments | optional references | optional |
| converted_state | active/converted/deleted | required |

**Invariants**

- Beat can remain a Beat indefinitely.
- Conversion to Scene Card does not delete the original creative thought.
## Scene Card

A compact outline/reference representation of a scene, independent from screenplay text.

| Field | Meaning | Rule |
|---|---|---|
| scene_card_id | stable identity | required |
| parent_type | Act/Sequence/Parking Lot | required |
| parent_id | parent container | required |
| short_description | scene idea | required or initially blank during inline creation |
| scene_heading | optional screenplay heading | optional until conversion |
| notes | freeform notes | optional |
| attachments | optional | optional |
| created_at | timestamp | required |
| modified_at | timestamp | required |
| screenplay_link | optional link to created screenplay scene | optional |

**Invariants**

- Compact representation; no manual scene number.
- No production metadata is stored as required Story Board fields.
- No live synchronization with Screenplay Scene.
## Parking Lot Entry

A Beat or Scene Card temporarily removed from active Story Board order but retained.

| Field | Meaning | Rule |
|---|---|---|
| entry_id | stable identity | required |
| object_type | Beat or Scene Card | required |
| object_id | parked object | required |
| parked_at | timestamp | required |

## Character

A lightweight story/production directory record.

| Field | Meaning | Rule |
|---|---|---|
| character_id | stable identity | required |
| project_id | owning project | required |
| name | character name | required |
| role_label | optional role | optional |
| description | optional | optional |
| image | optional | optional |
| notes | optional | optional |

## Character Relationship

A lightweight directed or undirected narrative relationship between characters.

| Field | Meaning | Rule |
|---|---|---|
| relationship_id | stable identity | required |
| from_character_id | character | required |
| to_character_id | character | required |
| relationship_type | freeform relation label | required |
| note | optional | optional |

## Story Timeline Entry

Optional chronology assignment for a screenplay scene, primarily Story Day and time-of-day note.

| Field | Meaning | Rule |
|---|---|---|
| entry_id | stable identity | required |
| screenplay_scene_id | target scene | required |
| story_day | chronological day label | required |
| time_note | optional time-of-day text | optional |
| continuity_note | optional warning context | optional |

## Screenplay

The project's authoritative written-script container.

| Field | Meaning | Rule |
|---|---|---|
| screenplay_id | stable identity | required |
| project_id | owning project | required |
| format | Feature/Short/Episodic | required |
| current_draft_id | current named draft | optional |
| locked_draft_id | locked shooting draft | optional |

## Screenplay Draft

A named user-recognized version of a screenplay.

| Field | Meaning | Rule |
|---|---|---|
| draft_id | stable identity | required |
| screenplay_id | parent screenplay | required |
| name | draft name | required |
| status | Draft/Review/Locked/Revision | required |
| created_from_draft_id | source draft | optional |
| created_at | timestamp | required |
| modified_at | timestamp | required |
| lock_timestamp | optional | optional |
| revision_label | optional post-lock label | optional |
| revision_color | optional | optional |

**Invariants**

- Named drafts are user-visible milestones.
- Automatic history points do not appear as named drafts.
- Locked status changes edit workflow and revision behavior.
## Screenplay Scene

A screenplay scene containing structured screenplay elements.

| Field | Meaning | Rule |
|---|---|---|
| screenplay_scene_id | stable identity within screenplay | required |
| draft_id | owning draft | required |
| screenplay_order | ordered position | required; derived from order not manually typed |
| scene_heading | INT/EXT/etc | required for valid screenplay scene |
| scene_elements | ordered screenplay elements | required |
| source_scene_card_id | optional origin card | optional |
| page_range | derived/presentation | derived |
| production_references | links to production planning objects | optional |

**Invariants**

- Scene order determines display numbering.
- Written content is authoritative for the screenplay.
- Production relationships must not depend on display number as identity.
## Screenplay Element

An ordered unit of screenplay content such as heading, action, character, dialogue, parenthetical or transition.

| Field | Meaning | Rule |
|---|---|---|
| element_id | stable identity | required |
| scene_id | parent screenplay scene | required |
| order_index | position in scene | required |
| element_type | Heading/Action/Character/Dialogue/Parenthetical/Transition/Shot/Note | required |
| text | content | required except empty entry state |
| note_state | printed or non-printed note | optional |

## Automatic History Point

Recoverable internal editing state used for restore, not a deliverable draft.

| Field | Meaning | Rule |
|---|---|---|
| history_id | stable recoverable identity | required |
| source_object_scope | screenplay/project/object | required |
| timestamp | creation time | required |
| state_snapshot_ref | recoverable state reference | required |
| restorable | boolean | required |

## Review Round

A review session attached to a selected draft and one or more reviewers.

| Field | Meaning | Rule |
|---|---|---|
| review_round_id | stable identity | required |
| draft_id | reviewed draft | required |
| name | round name | required |
| reviewers | review participants | optional |
| status | Open/Closed | required |
| created_at | timestamp | required |
| closed_at | optional | optional |

## Comment

A discussion item attached to a specific supported object or selected screenplay text.

| Field | Meaning | Rule |
|---|---|---|
| comment_id | stable identity | required |
| target_type | supported object/text target | required |
| target_id | target identity | required |
| text_anchor | optional selected text/position reference | optional |
| body | comment text | required |
| author_user_id | author | required |
| status | Open/Resolved | required |
| created_at | timestamp | required |
| resolved_at | optional | optional |

## Private Note

A note visible only to the permitted user/context and excluded from normal exports.

| Field | Meaning | Rule |
|---|---|---|
| private_note_id | stable identity | required |
| owner_user_id | owner | required |
| target_type | optional target | optional |
| target_id | optional target identity | optional |
| body | note text | required |
| created_at | timestamp | required |

## Production Source

The selected screenplay draft/version used as the baseline for production planning.

| Field | Meaning | Rule |
|---|---|---|
| source_id | stable identity | required |
| project_id | project | required |
| draft_id | selected screenplay draft | required |
| selected_at | timestamp | required |
| selection_reason | optional note | optional |
| active | boolean | required |

## Breakdown

A production-planning record for a screenplay source, organized by scene.

| Field | Meaning | Rule |
|---|---|---|
| breakdown_id | stable identity | required |
| project_id | project | required |
| source_id | Production Source | required |
| created_at | timestamp | required |
| updated_at | timestamp | required |

## Breakdown Element

A confirmed production requirement associated with one scene and one breakdown category.

| Field | Meaning | Rule |
|---|---|---|
| breakdown_element_id | stable identity | required |
| breakdown_id | parent breakdown | required |
| screenplay_scene_id | source scene | required |
| category | one default breakdown category | required |
| catalog_item_id | optional reusable catalog link | optional |
| display_name | entry name | required |
| notes | optional | optional |
| source_evidence | optional source text/reference | optional |
| confirmation_state | Suggested/Confirmed/Rejected/Manual | required |

**Invariants**

- A suggestion is not production truth until confirmed.
- One element belongs to one breakdown category at a time.
## Catalog Item

A reusable project production item referenced by breakdowns and other production workspaces.

| Field | Meaning | Rule |
|---|---|---|
| catalog_item_id | stable identity | required |
| project_id | project | required |
| category | production catalog category | required |
| name | item name | required |
| description | optional | optional |
| image | optional | optional |
| notes | optional | optional |
| status | project item status | optional |
| scene_usage | derived list | derived |

**Invariants**

- Reusable identity is project-scoped.
- Scene usage is derived from references, not manually maintained lists.
## Location

A production location record with practical notes, photos and scene usage.

| Field | Meaning | Rule |
|---|---|---|
| location_id | stable identity | required |
| project_id | project | required |
| name | location name | required |
| address | optional | optional |
| contact | optional | optional |
| photos | optional | optional |
| notes | parking/noise/permission/access/power/etc | optional |
| status | Idea/Shortlisted/Confirmed/Rejected | required |
| scene_usage | derived list | derived |

## Cast Member

A person assigned to a Character for the project.

| Field | Meaning | Rule |
|---|---|---|
| cast_member_id | stable identity | required |
| project_id | project | required |
| person_name | person name | required |
| character_id | assigned character | optional |
| contact | optional | optional |
| photo | optional | optional |
| notes | optional | optional |
| availability_notes | optional | optional |
| scene_usage | derived list | derived |

## Crew Member

A core production collaborator record with role/department/contact.

| Field | Meaning | Rule |
|---|---|---|
| crew_member_id | stable identity | required |
| project_id | project | required |
| person_name | person name | required |
| role | role | required |
| department | department | optional |
| contact | optional | optional |
| notes | optional | optional |

## Moodboard

A visual reference board containing images, notes, links and optional captions.

| Field | Meaning | Rule |
|---|---|---|
| moodboard_id | stable identity | required |
| project_id | project | required |
| name | board name | required |
| items | ordered visual references | optional |
| notes | optional | optional |

## Storyboard

A visual shot-planning board/panel collection associated with scenes.

| Field | Meaning | Rule |
|---|---|---|
| storyboard_id | stable identity | required |
| project_id | project | required |
| scene_id | optional screenplay scene | optional |
| name | board/name | required |
| panel_order | ordered panels | optional |

## Storyboard Panel

A single visual panel/shot reference within a Storyboard.

| Field | Meaning | Rule |
|---|---|---|
| panel_id | stable identity | required |
| storyboard_id | parent board | required |
| shot_id | optional linked shot | optional |
| image_or_sketch | visual content | required |
| description | optional | optional |
| framing | optional | optional |
| movement | optional | optional |
| dialogue | optional | optional |
| notes | optional | optional |

## Shot List

An ordered set of Shot records organized by Scene.

| Field | Meaning | Rule |
|---|---|---|
| shot_list_id | stable identity | required |
| project_id | project | required |
| source_id | optional production source | optional |
| shot_entries | ordered Shot list | optional |

## Shot

A planned camera/coverage unit associated with a Scene.

| Field | Meaning | Rule |
|---|---|---|
| shot_id | stable identity | required |
| shot_list_id | parent list | required |
| screenplay_scene_id | source scene | required |
| shot_label | display identifier | required |
| order_index | coverage order | required |
| description | shot description | required |
| size | optional | optional |
| movement | optional | optional |
| angle | optional | optional |
| lens | optional | optional |
| camera_notes | optional | optional |
| characters | optional references | optional |
| storyboard_panel_id | optional | optional |
| sound_note | optional | optional |

## Shooting Schedule

The project's ordered set of Shooting Days derived from a selected Production Source.

| Field | Meaning | Rule |
|---|---|---|
| schedule_id | stable identity | required |
| project_id | project | required |
| source_id | Production Source | required |
| days | ordered Shooting Days | optional |
| unscheduled_scenes | derived list | derived |
| status | Draft/Active/Finalized | required |

## Shooting Day

A calendar/date container containing scheduled scenes and day markers.

| Field | Meaning | Rule |
|---|---|---|
| shoot_day_id | stable identity | required |
| schedule_id | parent schedule | required |
| date | calendar date | optional until assigned |
| day_number | shooting day sequence | required |
| scene_ids | ordered scheduled scenes | optional |
| location_refs | derived/selected | optional |
| cast_refs | derived/selected | optional |
| estimated_duration | optional | optional |
| notes | optional | optional |
| markers | Schedule Marker list | optional |

## Schedule Marker

A non-scene item such as meal break, travel, company move or custom note.

| Field | Meaning | Rule |
|---|---|---|
| marker_id | stable identity | required |
| shoot_day_id | parent day | required |
| type | Meal/Travel/Company Move/Custom | required |
| label | display label | required |
| position | ordered position | required |
| duration | optional | optional |
| notes | optional | optional |

## Call Sheet

A working document generated from a Shooting Day and then manually editable/finalizable.

| Field | Meaning | Rule |
|---|---|---|
| call_sheet_id | stable identity | required |
| shoot_day_id | source shooting day | required |
| source_snapshot | schedule/source snapshot reference | required |
| status | Draft/Needs Refresh/Ready/Final/Issued | required |
| document_content | editable call-sheet content | required |
| finalized_at | optional | optional |
| issued_at | optional | optional |

**Invariants**

- Call sheet edits do not reverse-sync ordinary changes into the schedule.
- Final/Issued call sheet is a document snapshot.
## Side

A print/export subset of screenplay content for a shooting day or selected scope.

| Field | Meaning | Rule |
|---|---|---|
| side_id | stable identity | required |
| scope | shoot day/selected scene range | required |
| source_draft_id | source draft | required |
| content_snapshot | snapshot of script content | required |

## Production Report

A lightweight read-only/reporting view over existing production data.

| Field | Meaning | Rule |
|---|---|---|
| report_id | stable identity | required |
| project_id | project | required |
| report_type | supported report type | required |
| generated_at | timestamp | required |
| source_scope | data scope/source version | required |
| snapshot | report snapshot | required |

## Daily Production View

A focused production-day workspace combining schedule, scene, cast, location and call-sheet context.

| Field | Meaning | Rule |
|---|---|---|
| view_id | stable identity | required |
| shoot_day_id | target day | required |
| derived_context | schedule/cast/location/call sheet context | derived |

## Project Note

A lightweight project-level note.

| Field | Meaning | Rule |
|---|---|---|
| note_id | stable identity | required |
| project_id | project | required |
| title | optional title | optional |
| body | note text | required |
| created_at | timestamp | required |
| modified_at | timestamp | required |

## Task

A lightweight project task with owner/status/due information where enabled.

| Field | Meaning | Rule |
|---|---|---|
| task_id | stable identity | required |
| project_id | project | required |
| title | task title | required |
| owner_user_id | optional owner | optional |
| status | Open/Done | required |
| due_at | optional due time | optional |
| notes | optional | optional |

## Activity Entry

An activity-history record describing a meaningful project change.

| Field | Meaning | Rule |
|---|---|---|
| activity_id | stable identity | required |
| project_id | project | required |
| actor_user_id | actor | optional for local system activity |
| action_type | activity type | required |
| target_type | target object type | optional |
| target_id | target identity | optional |
| timestamp | timestamp | required |
| summary | human-readable summary | required |

## Budget Snapshot

A lightweight project-level estimate containing high-level budget figures/line items, not full accounting.

| Field | Meaning | Rule |
|---|---|---|
| budget_id | stable identity | required |
| project_id | project | required |
| currency | currency code | required |
| estimated_total | high-level estimate | optional |
| line_items | small list of broad categories | optional |
| notes | optional | optional |
| updated_at | timestamp | required |

## Template

A reusable user/project document/template definition.

| Field | Meaning | Rule |
|---|---|---|
| template_id | stable identity | required |
| scope | global/project | required |
| template_type | document/board/call sheet/etc | required |
| name | template name | required |
| content_defaults | reusable starting content/layout | required |
| created_by | user | optional |

## Exchange Package

A portable snapshot used for review/import/merge outside mandatory cloud synchronization.

| Field | Meaning | Rule |
|---|---|---|
| package_id | stable export identity | required |
| package_type | review/story/breakdown/shots/schedule/call/etc | required |
| source_project_id | origin project identity | required |
| source_object_ids | included objects | required |
| source_versions | relevant source versions | required |
| exported_at | timestamp | required |
| package_format_version | package schema version | required |
| included_attachments | optional | optional |

**Invariants**

- Immutable after export.
- Contains enough source/version context to detect staleness.
## Import Session

A temporary review/import context used to inspect and apply an incoming package safely.

| Field | Meaning | Rule |
|---|---|---|
| import_session_id | stable temporary session identity | required |
| package_id | incoming package | required |
| target_project_id | destination project | required |
| base_snapshot | destination state at import start | required |
| preview_state | Pending/Reviewed/Ready | required |
| conflicts | detected conflicts | optional |
| import_result | Applied/Rejected/Partial | required |

**Invariants**

- Preview before application.
- Must not blindly overwrite project content.
## Collaboration Session

A temporary local-network project session with one host and participants.

| Field | Meaning | Rule |
|---|---|---|
| session_id | stable session identity | required |
| project_id | shared project | required |
| host_user_id | host | required |
| participants | connected users | optional |
| state | Not Running/Host Starting/Running/Participant Joined/Participant Active/Participant Disconnected/Session Ended | required |
| started_at | timestamp | optional |
| ended_at | timestamp | optional |

## AI Request

A user-initiated request to the optional system-wide AI assistant with explicit scope, intent, operation class and authorization context.

| Field | Meaning | Rule |
|---|---|---|
| request_id | stable identity | required |
| user_id | requesting user | required |
| project_id | project scope | optional for global/application queries |
| session_id | conversation/session context | optional |
| scope | resolved context | required |
| request_text | original user request | required |
| intent | normalized requested intent | required after interpretation |
| operation_class | Read/Compute/Navigate/Suggest/Mutate | required |
| target_objects | resolved targets | optional for pure informational requests |
| authorization_state | result of permission evaluation | required before mutation |
| external_processing_state | Local/External/Not Sent | required |
| model_reference | provider/model descriptor | optional |
| status | Created/Resolved/Running/Completed/Failed/Canceled | required |
| created_at | timestamp | required |
| completed_at | completion timestamp | optional |

**Invariants**

- The request is initiated by the user or by an explicit user follow-up in the current session.
- Read/compute/navigation requests do not themselves create a Change Set.
- Mutation requests must pass permission/state checks before a Change Set can become applicable.
- A request cannot grant permissions to itself.

## AI Result

The generated answer, deterministic query result, explanation, suggestion or mutation proposal returned from an AI request.

| Field | Meaning | Rule |
|---|---|---|
| result_id | stable identity | required |
| request_id | source request | required |
| content | user-facing generated/explanatory output | required |
| structured_data | deterministic returned values/results | optional |
| provenance | source scope/version/object references | required when material to interpretation |
| suggested_changes | optional Change Set | optional |
| confidence_state | Exact/Inferred/Unavailable where useful | optional |
| status | Informational/Pending Approval/Accepted/Rejected/Applied/Stale/Conflict/Failed | required |
| error | failure information | optional |
| created_at | timestamp | required |

**Invariants**

- Informational results are not authoritative.
- Exact project facts marked Exact must originate from deterministic project data/calculations.
- Mutating results are not authoritative until explicitly accepted.
- A result cannot directly mutate canonical project objects.

## AI Tool Invocation

An internal supporting construct representing an application-level operation requested by the AI.

| Field | Meaning | Rule |
|---|---|---|
| invocation_id | stable identity | required |
| request_id | parent AI Request | required |
| tool_name | logical application operation | required |
| parameters | structured parameters | required |
| target_objects | resolved targets | optional |
| authorization_state | Allowed/Denied | required |
| execution_state | Pending/Running/Succeeded/Failed/Blocked | required |
| result_reference | returned result | optional |
| created_at | timestamp | required |

**Invariants**

- Tool invocation never grants storage-level authority to the model.
- Application validation occurs before a write-capable operation executes.
- A denied invocation cannot be retried with elevated authority merely by changing model output.

## Change Set

A bounded collection of changes produced by a user edit, review response, package import or approved AI mutation.

| Field | Meaning | Rule |
|---|---|---|
| change_set_id | stable identity | required |
| origin | User/Import/Review/AI | required |
| ai_request_id | originating AI Request when origin=AI | required when applicable |
| ai_result_id | originating AI Result when applicable | optional |
| requesting_user_id | user responsible for request | required |
| approver_user_id | user who explicitly accepted the change | required before application |
| approved_at | approval timestamp | required before application |
| target_objects | affected objects | required |
| affected_modules | affected OpenFrame workspaces | required for material batch changes |
| operations | ordered changes | required |
| base_version | version/state at creation | required |
| review_state | Pending/Accepted/Rejected/Applied/Stale/Conflict | required |
| validation_state | Not Checked/Valid/Invalid/Needs Review | required |

**Invariants**

- Has an explicit base version.
- AI-originated Change Sets require explicit user approval before application.
- Application must revalidate the base version before applying.
- Approval does not override permission restrictions.
- Rejected, invalid, stale or conflicted Change Sets do not mutate the project.
- An accepted Change Set is applied through normal application mutation semantics and becomes undoable where supported.


## Snapshot

A preserved point-in-time representation of project content used for export, review, revision or recovery.

| Field | Meaning | Rule |
|---|---|---|
| snapshot_id | stable identity | required |
| snapshot_type | Draft/Export/Review/Call Sheet/Recovery/etc | required |
| project_id | project | required |
| source_version | source version context | optional |
| created_at | timestamp | required |
| content_reference | preserved state | required |

**Invariants**

- Immutable point-in-time record.
- Used for recovery, comparison, export or document finalization.
## Deleted Item

A recoverable record representing an item moved to deleted state before permanent removal.

| Field | Meaning | Rule |
|---|---|---|
| deleted_id | stable identity | required |
| object_type | deleted object type | required |
| object_id | deleted object identity | required |
| deleted_at | timestamp | required |
| restore_until | optional policy timestamp | optional |
| original_parent | parent context | optional |

# 5. Relationship and Cardinality Rules

| Parent | Child/Related | Cardinality | Functional rule |
|---|---|---|---|
| Project | Project Settings | 1:1 | Every project has one settings record. |
| Project | Project Status | 1:1 logical current value | Status belongs to the project and is user-controlled. |
| Project | Global Idea Vault Item | 0:N reference/copy | Global items are not owned by a project; a project may receive explicit copies. |
| Project | Project Idea Vault Item | 1:N | Project creative dump. |
| Global Idea Vault Item | Project Idea Vault Item | 0:N copy | Copy is independent; edits do not sync back. |
| Project Idea Vault Item | Vault Collection | N:M | A project Vault item may belong to collections; membership is organizational. |
| Project | Project File | 1:N | Miscellaneous project file cabinet. |
| Project | Act | 1:N | Story Board top-level containers. |
| Act | Sequence | 1:N | Sequence is a child of an Act during active outline structure. |
| Sequence | Beat | 1:N optional | Beats may temporarily live inside sequence. |
| Sequence | Scene Card | 1:N optional | Normal scene grouping. |
| Act | Scene Card | 1:N optional | Scene Cards may exist directly under an Act. |
| Beat | Scene Card | 0:1 origin | A Beat can be converted into a Scene Card; original Beat remains. |
| Scene Card | Screenplay Scene | 0:1 origin/reference | A card may create/reference a screenplay scene, but no live sync exists. |
| Project | Character | 1:N | Project story directory. |
| Character | Character Relationship | 1:N | Relationship connects characters. |
| Screenplay | Screenplay Draft | 1:N | Named drafts belong to a screenplay. |
| Screenplay Draft | Screenplay Scene | 1:N | A draft contains ordered scenes. |
| Screenplay Scene | Screenplay Element | 1:N | Elements form the written scene. |
| Screenplay Scene | Story Timeline Entry | 0:1 | Optional story chronology. |
| Screenplay Draft | Automatic History Point | 1:N | Recoverable edits associated with a draft. |
| Screenplay Draft | Review Round | 1:N | Review rounds target a specific draft. |
| Review Round | Comment | 1:N | Comments belong to a review round/context. |
| Project | Comment | 0:N | Comments can attach to multiple object types. |
| Application User | Private Note | 1:N | Private notes belong to their owner. |
| Screenplay Draft | Production Source | 0:N historically; 0:1 active | A project can have historical source selections but one active production source at a time. |
| Production Source | Breakdown | 1:N lifecycle snapshots | Breakdowns derive from a selected production source. |
| Breakdown | Breakdown Element | 1:N | Scene-level production requirements. |
| Breakdown Element | Catalog Item | 0:1 | A confirmed element may point to a reusable catalog item. |
| Project | Catalog Item | 1:N | Project production directory. |
| Catalog Item | Breakdown Element | 1:N | Reusable production object referenced by scenes. |
| Project | Location | 1:N | Production locations. |
| Location | Screenplay Scene | 0:N derived usage | Scene usage is derived from breakdown/source references. |
| Project | Cast Member | 1:N | Production cast directory. |
| Cast Member | Character | 0:1 | Cast may be assigned to a character. |
| Character | Screenplay Scene | 0:N derived usage | Character usage is derived from screenplay parsing/correction. |
| Project | Crew Member | 1:N | Core crew directory. |
| Project | Moodboard | 1:N | Creative visual reference boards. |
| Moodboard | Idea Vault Item | 0:N reference | Moodboard may use copied/reference materials; does not own Vault meaning. |
| Project | Storyboard | 1:N | Visual storyboard boards. |
| Storyboard | Storyboard Panel | 1:N | Ordered panels. |
| Storyboard Panel | Shot | 0:1 | Optional link to a shot. |
| Project | Shot List | 1:N | Shot planning collections. |
| Shot List | Shot | 1:N | Ordered shot coverage. |
| Shot | Screenplay Scene | N:1 | Each shot belongs to one source scene. |
| Project | Shooting Schedule | 1:N historical; usually 1 active | Schedules are source-version-specific. |
| Shooting Schedule | Shooting Day | 1:N | Ordered production days. |
| Shooting Day | Screenplay Scene | N:M scheduled usage | A scene is scheduled to one or more days only when user intentionally duplicates/continues; default use is one planned day. |
| Shooting Day | Schedule Marker | 1:N | Breaks/moves/custom schedule items. |
| Shooting Day | Call Sheet | 1:N historical; 0:1 current | A day may produce multiple call-sheet drafts/exports but one current working document. |
| Shooting Day | Side | 0:N | Sides can be generated from a day or selected scope. |
| Project | Production Report | 1:N | Generated report snapshots. |
| Shooting Day | Daily Production View | 1:1 logical | Daily view is derived from one shooting day. |
| Project | Project Note | 1:N | Lightweight notes. |
| Project | Task | 1:N | Lightweight tasks. |
| Project | Activity Entry | 1:N | Activity history. |
| Project | Budget Snapshot | 0:N historical; 0:1 current logical | Simple budget snapshots over time. |
| Project | Template | 0:N | Project-specific reusable templates. |
| Template | Project | 0:N when global | Global templates can seed multiple projects via copy/use. |
| Exchange Package | Import Session | 1:N | One exported package can be imported into one or more target contexts. |
| Project | Collaboration Session | 1:N historical; 0:1 running | Temporary local-network sessions. |
| Collaboration Session | Application User | 1:N | Host plus participants. |
| Project | AI Request | 0:N | AI is optional and project scoped when asked about project data. |
| AI Request | AI Result | 1:N | A request can yield explanatory, deterministic or actionable results. |
| AI Request | AI Tool Invocation | 0:N | A request may invoke authorized application tools. |
| AI Result | Change Set | 0:1 | A modifying result can produce a pending Change Set. |
| AI Tool Invocation | Change Set | 0:1 | A write-capable invocation may contribute to a proposed Change Set. |
| Change Set | Snapshot | 0:N | Applied changes may be captured in history/snapshots. |
| Project | Snapshot | 1:N | Recovery/export/review snapshots. |
| Deleted Item | any user-owned object | 1:1 at time of deletion | Deletion retains enough parent context to restore where permitted. |

Cardinality uses logical product relationships rather than database implementation. “0:N historical; 0:1 current” means the application may retain prior records while exposing only one current authoritative state.

# 6. Source-of-Truth and Authority Rules

## Idea Vault

Global/Project Idea Vault

## Story Board

Acts/Sequences/Beats/Scene Cards

## Screenplay

Screenplay Draft/Scene/Element

## Production Source

Selected Draft

## Breakdown

Breakdown Elements/Catalog references

## Shot List

Shots

## Storyboard

Storyboard Panels

## Shooting Schedule

Shooting Days/Markers

## Call Sheet

Call Sheet document

## 6.1 No hidden cross-object synchronization

- Idea Vault edits do not rewrite Story Board objects after copying.
- Story Board edits do not continuously rewrite screenplay text.
- Production edits do not rewrite screenplay content.
- Call Sheet edits do not automatically rewrite the schedule.
- Shot List and Storyboard may be linked, but neither is required by the other.
- AI suggestions do not become authoritative until accepted.
# 7. Lifecycle and State Models

| Object | State model | Typical path | Authority rule |
|---|---|---|---|
| Project | Active lifecycle label | Idea → Development → Writing → Rewrite → Shooting Draft → Pre-Production → Shoot Preparation → Shooting → Archived | Manual status is authoritative. |
| Vault Item | Normal → Deleted | Active → Deleted → Permanently Deleted | Recoverable before permanent removal. |
| Beat | Active / Converted / Parked / Deleted | Active → Converted to Scene or Parked; Deleted optional | Original Beat retained after conversion. |
| Scene Card | Active / Parked / Deleted | Active ↔ Parking Lot; Active → Deleted | No screenplay text mutation on board edits. |
| Screenplay Draft | Draft / Review / Locked / Revision | Draft → Review → Draft/Revision; Draft → Locked; Locked → Revision | Exact transitions remain user-controlled. |
| Review Round | Open / Closed | Open → Closed | Closing does not delete comments. |
| Breakdown Element | Suggested / Confirmed / Rejected / Manual | Suggested → Confirmed or Rejected; Manual = confirmed by user | Only Confirmed/Manual counts as production data. |
| Shooting Schedule | Draft / Active / Finalized | Draft → Active → Finalized (or reopened by authorization) | User remains able to modify until finalized. |
| Call Sheet | Draft / Needs Refresh / Ready / Final / Issued | Draft → Needs Refresh/Ready → Final → Issued | Source mismatch creates Needs Refresh rather than silent overwrite. |
| Import Session | Pending / Reviewed / Ready / Applied / Rejected / Partial | Pending → Reviewed → Ready → Applied or Rejected/Partial | Never blind overwrite. |
| Collaboration Session | Not Running / Host Starting / Running / Participant Joined / Participant Active / Participant Disconnected / Session Ended | As defined by FSD | Transient session state; does not redefine project ownership. |
| AI Result | Informational / Pending Approval / Accepted / Rejected | Informational or Pending Approval → Accepted/Rejected | Only Accepted mutation applies. |
| Task | Open / Done | Open ↔ Done | Lightweight only. |
| Location | Idea / Shortlisted / Confirmed / Rejected | User-controlled | Status does not delete rejected location. |

# 8. Derived Data Rules

Derived data should be computed or reconstructed from authoritative objects wherever practical rather than stored as competing editable truth.

| Derived value | Source | Rule |
|---|---|---|
| Display screenplay scene number | Screenplay Scene order | 1..N according to current ordered screenplay scenes. |
| Scene page count/range | Screenplay formatting/content | Derived from current draft/export formatting; not a user identity. |
| Character scene usage | Screenplay Scene/Character recognition and corrections | Derived list; user can correct identity mapping. |
| Location scene usage | Breakdown/Catalog references | Derived list of scenes requiring location. |
| Cast scene usage | Character + screenplay/breakdown | Derived from character use and production assignment. |
| Catalog scene usage | Breakdown Element references | Derived list of scenes using item. |
| Unscheduled scene list | Shooting Schedule + Production Source | All source scenes not currently assigned to a Shooting Day. |
| Shoot Day cast list | Scheduled scenes + cast associations | Derived, with manual corrections only where FSD permits. |
| Shoot Day location list | Scheduled scenes + location associations | Derived, with manual additions only where supported. |
| Call Sheet prefill | Shooting Day + associated project data | Prefill snapshot; call sheet then becomes editable. |
| Breakdown progress | Breakdown Element confirmation state | Count/status derived; no separate manually edited percentage. |
| Activity summary | Activity Entry records | Presentation only; not a second source of truth. |

# 9. Cross-Module Data Flows

## Idea → Story

A Global or Project Vault Item can be explicitly copied to a Beat, Scene Card, Sequence idea, Character note or Story note. Original remains unchanged.

## Story → Screenplay

Build Screenplay creates a new screenplay draft from selected active Scene Cards. Heading must be present for included cards. No live synchronization follows.

## Screenplay → Breakdown

Breakdown is created against a chosen Production Source draft; scene identities and headings are reused, with breakdown elements confirmed by user.

## Breakdown → Catalog

Confirmed production elements may be represented by reusable Catalog Items; later scenes can reference the same catalog item.

## Catalog/Locations/Cast → Schedule

Scheduling groups screenplay scenes into Shooting Days; locations/cast usage is surfaced from breakdown/catalog data where available.

## Schedule → Call Sheet

Call Sheet is generated from a Shooting Day snapshot; the call sheet remains an editable document and does not reverse-sync ordinary document edits into schedule.

## Storyboard ↔ Shot List

Storyboard Panels may optionally link to Shots; either workflow can exist independently.

## Exchange → Import

Exchange Package is inspected in an Import Session before application; stale/base-version issues are surfaced instead of blindly overwriting.

## AI → Project

AI results are informational until accepted; modifications become explicit Change Sets and require approval before application.

# 10. Script → Production Baseline

The key data boundary is the Production Source. A production plan must identify which screenplay draft it is based on. This avoids the false assumption that “latest screenplay” always equals “production screenplay.”

## 10.1 Selecting a Production Source

- User selects a named screenplay draft as production source.
- System records the selection and timestamp.
- Existing production work remains associated with its prior source until the user explicitly reconciles/refreshes it.
- A newer screenplay draft does not silently replace the production source.
## 10.2 Refresh / stale rule

When a production workspace is based on an older source draft than the current screenplay, it can be marked stale. Stale means “built from an older source,” not “invalid.” The user can review changes and intentionally refresh/reconcile.

# 11. Duplication Rules

| Object | Duplication rule |
|---|---|
| Act | Duplicate title/note and child hierarchy only when user explicitly duplicates the Act; children receive new identities. |
| Sequence | Duplicate sequence name/note and optionally children; new sequence identity. |
| Beat | Duplicate beat text/note/color/reference; new Beat identity. |
| Scene Card | Duplicate short description/heading/notes/selected attachments; new identity; no inherited screenplay scene identity. |
| Screenplay Draft | Create a new named draft from current/selected draft; previous draft remains unchanged. |
| Catalog Item | Explicit duplicate creates a distinct production item; normal references should reuse the existing item instead. |
| Shot | Duplicate coverage card to create a new shot; new shot identity. |
| Shooting Day | Duplicate day shell/markers only when explicitly requested; scenes require explicit inclusion to avoid accidental schedule duplication. |
| Call Sheet | New call-sheet revision is a new document snapshot derived from the selected day/source state. |
| Project | Duplicate creates independent project package and project identity. |
| Vault Item | Copy across Global/Project Vault creates independent content. |

# 12. Deletion and Recovery Rules

Deletion must preserve user safety, downstream integrity and explicit user control.

- Creative objects use recoverable deleted state before permanent removal.
- Deleting a Scene Card does not automatically delete its linked Screenplay Scene.
- Deleting a Story Board container offers safe child-handling choices.
- Deleting a Character used in production triggers confirmation and favors archive/remove-from-directory semantics rather than destructive removal.
- Production catalog references must not silently become dangling references; affected breakdown entries must retain visible unavailable/removed status until user resolves them.
- Permanent deletion must require an explicit destructive confirmation for meaningful project data.
- Restore must attempt to return the object to its prior parent/order context using Deleted Item metadata.
# 13. Comments, Notes and Activity Data

Comments, private notes, project notes, tasks and activity entries are intentionally different domain concepts.

| Object | Purpose | Visibility | Lifecycle |
|---|---|---|---|
| Comment | Discussion attached to a supported object/text selection | According to project role/share context | Open/Resolved; history retained |
| Private Note | Personal thinking not intended for collaborators | Owner/authorized private context only | Persists until deleted |
| Project Note | Project-level working note | Project permissions | Editable |
| Task | Small actionable project item | Project permissions | Open/Done |
| Activity Entry | Historical description of meaningful changes | Project activity visibility | Append-oriented; not authoritative content |

# 14. Production Source-Version Integrity

Every downstream production object that materially depends on screenplay content should be traceable to the relevant Production Source or source draft snapshot.

| Production object | Source version reference required? | Reason |
|---|---|---|
| Breakdown | Yes | Breakdown content depends directly on screenplay. |
| Shot List | Yes or scene-level source reference | Shot planning is based on screenplay scene identity/current production source. |
| Storyboard | Recommended/where linked | Visual planning may be created from a scene/source but can continue independently. |
| Shooting Schedule | Yes | Schedule is created from a chosen source scene set. |
| Call Sheet | Yes indirectly via Shooting Day snapshot | Call sheet should be reproducible from its source day. |
| Sides | Yes | Sides are script excerpts from a specific draft/source. |
| Reports | Yes by report scope | Reports should identify their underlying source context where relevant. |

# 15. Episodic Domain Rules

Episodic support must remain lightweight while preserving the product's project/episode organization.

| Object | Rule |
|---|---|
| Series Project | A Project whose type is Episodic/Series. |
| Season | An optional lightweight grouping of Episodes; no giant season-analytics subsystem. |
| Episode | A container under a Series/Season with its own Story Board and Screenplay context. |
| Series Character | A character identity may be shared across episodes through explicit references while episode screenplay usage remains independent. |
| Episode Screenplay | Each episode may have its own named drafts/review/lock lifecycle. |
| Episode Story Board | Acts/Sequences/Beats/Scene Cards belong to the episode story context unless explicitly represented at season level. |

No deeper episodic continuity graph is required by the current P0 scope; deeper continuity remains later/P2 behavior.

# 16. Exchange Package Domain

Portable exchange is a central local-first collaboration mechanism. An Exchange Package is a snapshot, not a live shared object.

## 16.1 Package contents

- Package type and format version
- Source project identity
- Source object identities included
- Relevant source draft/version identities
- Included comments/review data when selected
- Included attachments when selected
- Export timestamp
- Base snapshot/version for safe stale detection
## 16.2 Import behavior

- Import first creates an Import Session.
- The user sees a preview before applying.
- The application checks source/base context for staleness.
- The application never blindly overwrites current project state.
- Incoming comments may be applied without changing screenplay text.
- If content changes are included, they are applied as an explicit Change Set or equivalent reviewed action.
# 17. Local Collaboration Domain

Local-network collaboration is a temporary shared session around a locally owned project.

## 17.1 Session state

- One host owns the active session context.
- Participants join the temporary session.
- Presence is transient and does not replace project identity.
- A disconnected participant may be flagged without deleting their project role.
- Ending a session does not delete or cloud-publish the project.
## 17.2 Collaboration safety

- Project content remains owned by the local project package.
- Conflicts must be visible and reviewable.
- Session changes should be distinguishable from later imported exchange changes.
- Any persistent conflict resolution should produce a clear resulting project state and history entry.
# 18. AI Domain

AI is an optional cross-cutting interaction layer over existing domain data. AI does not become a second authoritative project store.

## 18.1 AI context

| Scope | Allowed context |
|---|---|
| Current selection | Explicitly selected object(s) and their supported linked context. |
| Current scene | Current screenplay scene and explicitly selected supporting objects. |
| Current screenplay | Selected/current draft content and relevant script context. |
| Specific draft | The chosen Screenplay Draft and supported linked context. |
| Story Board | Story Board objects within the current project. |
| Idea Vault selection | Only explicitly selected Vault items when requested. |
| Production | User-authorized production records and their supported relationships. |
| Shooting day | Selected schedule/day and supported related scene/production data. |
| Call Sheet | Selected Call Sheet and its source schedule context. |
| Whole project | User-authorized current-project content within effective permissions. |
| Series/episode | Explicit series, season, episode or selected-episode scope. |
| Global/application | Product knowledge and explicitly requested Global Idea Vault content; project boundaries remain intact. |

## 18.2 AI authority boundary

- AI-generated text is not authoritative project truth.
- Canonical domain objects remain authoritative.
- Deterministic query/calculation results are authoritative for the supported fact being calculated.
- Application tools are authoritative for validation and mutation.
- User approval is authoritative for applying AI-proposed project changes.

## 18.3 AI operation classes

Logical AI operations are:

```text
Read
Compute
Navigate
Suggest/Prepare
Mutate
```

Read, compute and navigation operations do not mutate project state.

Suggest/Prepare operations may produce AI Results and proposed Change Sets.

Mutate operations require an accepted, validated Change Set before application.

## 18.4 AI provenance

For exact or consequential answers, the AI layer should retain enough provenance to identify:

- project;
- draft/version;
- scope;
- source objects;
- calculation basis;
- originating request;
- applicable tool invocation(s).

## 18.5 AI version safety

An AI Change Set is bound to the project/version state against which it was prepared.

If relevant project data changes before application, the Change Set becomes Stale or Conflict and must be revalidated/reviewed.

## 18.6 AI permission boundary

AI inherits the user's effective permissions.

AI cannot:

- grant permissions;
- access another user's private notes;
- bypass locked drafts;
- override approval gates;
- alter archived/deleted objects outside supported lifecycle actions;
- bypass collaboration conflict rules.

## 18.7 Deterministic project facts

Supported exact metrics such as scene count, character count, location count, page count, unscheduled scene count and similar structured statistics are derived from canonical project data rather than stored as AI-generated facts.

## 18.8 Conversation/session context

Session context may resolve references such as “this scene” or “that draft,” but session context must never override current canonical project state.

## 18.9 AI-generated project content

When a user accepts an AI-generated object/content proposal, it becomes a normal canonical OpenFrame object of the appropriate type.

The AI layer does not create a parallel class of “AI scene cards,” “AI characters” or “AI breakdowns.”

# 19. Budget Snapshot Domain

Budget Snapshot is intentionally small and non-accounting.

- It may store a high-level estimated total and a small set of broad line items.
- It does not create payroll, invoices, purchase orders, union accounting, tax records, or full cost reporting.
- Multiple snapshots may be retained for history; one may be treated as current.
- Budget values do not automatically change schedule, script or other production objects.
# 20. Template Domain

Templates provide reusable starting content and layout without introducing a separate document-management suite.

- Templates can be global or project-specific.
- Using a template creates a new object/document from template content; later edits do not modify the template.
- Templates may cover call sheets, reports, moodboards, shot lists or other approved document/board surfaces.
- Template use is optional; no workflow requires a template.
# 21. Document Snapshot and Export Semantics

Exports must be understood as snapshots rather than live synchronized documents.

| Output | Source | Domain behavior |
|---|---|---|
| Screenplay PDF | Selected draft | Snapshot; does not alter draft. |
| FDX/Fountain/DOCX | Selected draft | Interchange snapshot generated from screenplay state. |
| Story Board export | Current board state | Snapshot; cards retain project identity internally only. |
| Breakdown report | Breakdown/source state | Snapshot of current breakdown. |
| Schedule export | Shooting Schedule state | Snapshot; later edits do not retroactively alter exported file. |
| Call Sheet PDF | Call Sheet document | Snapshot of exact call-sheet state at export/issue. |

# 22. Project Files and External References

The domain must distinguish between content that is actually stored in the project and content that is referenced externally.

- Imported/copied files may be project-owned attachments.
- External references may point to files that later become unavailable.
- An unavailable external reference remains visible with an unavailable state rather than being silently replaced.
- Restoring/relinking an external file should preserve the logical project object identity where possible.
# 23. Derived Query/Projection Rules

Many UX screens are projections over authoritative objects. They should not be treated as new authoritative domain entities merely because the UI has a page for them.

| UX surface | Primary domain sources | Nature |
|---|---|---|
| Project Home | Project + recent Activity + current workspace states | Derived overview. |
| Story Board | Acts + Sequences + Beats + Scene Cards + Parking Lot | Authoritative Story structure presentation. |
| Screenplay Navigator | Screenplay Draft + Scenes | Derived navigation view. |
| Production Catalog | Catalog Items | Authoritative catalog directory. |
| Scene Hub | One screenplay scene + its related production references | Derived cross-module view, not a replacement object. |
| Daily Production View | Shooting Day + schedule + related production data | Derived operational view. |
| Reports | Existing domain records | Snapshot/read-only projection. |
| Global Search | Indexed representations of domain objects | Cross-object projection, not new source of truth. |

# 24. Integrity Constraints

- A Scene Card without a screenplay link is valid.
- A Screenplay Scene without a Scene Card origin is valid because users may write/import directly.
- A Breakdown Element must point to a valid source scene from the selected production source.
- A Confirmed breakdown element may reference a Catalog Item; a rejected suggestion must not count as production need.
- A Shot must belong to a Shot List and source a Screenplay Scene.
- A Storyboard Panel may exist without a Shot, and a Shot may exist without a Storyboard Panel.
- A Shooting Day may contain no scenes while being prepared.
- A Call Sheet may exist in Draft state before finalization.
- A Project can contain no Story Board content and still contain an imported screenplay.
- A Short Film project must not require unused Acts/Sequences/Production sections.
- An archived project remains internally intact and reopenable.
- Permanent deletion of a project requires deliberate confirmation and should not be conflated with archive.
# 25. Cross-Document Alignment Matrix

**Reference counts used in the final audit:** 55 distinct PRD requirement IDs; 232 canonical FSD requirement IDs from the complete inventory; 55 distinct PRD IDs referenced by UX/UI; 232 distinct FSD IDs referenced by UX/UI; 11 auxiliary shorthand FSD acceptance IDs outside the canonical inventory.

The Domain/Data model was derived from the supplied PRD, FSD and UX/UI files. The purpose of the matrices below is to make the relationship auditable without making the Domain/Data document a new product specification.

## 25.1 Major-domain alignment

| Domain area | PRD anchor | FSD anchor | UX anchor | Domain objects |
|---|---|---|---|---|
| Project shell/lifecycle | PRD-CORE-001 | Application shell / project lifecycle | Application Home / Project Home / New Project | Application User, Project, Project Settings, Project Status |
| Idea Vault | PRD-IDEA-001..005 | FSD-IDEA-001..020 | Global Idea Vault / Project Idea Vault / Item Detail | Global Idea Vault Item, Project Idea Vault Item, Vault Collection, Project File |
| Story Board | PRD-STORY-001..007 | FSD-STORY-001..030 | Story Workspace / Acts / Sequences / Beats / Scene Cards | Act, Sequence, Beat, Scene Card, Parking Lot Entry |
| Characters/timeline | PRD-STORY-* | FSD-STORY-* | Characters / Story Timeline | Character, Character Relationship, Story Timeline Entry |
| Screenplay | PRD-SCRIPT-001..007 | FSD-SCRIPT-001..050 | Screenplay / Writing Room / Drafts / Review / Lock | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note |
| Episodic | PRD-EP-001..002 | FSD episodic requirements | Episodic/Season/Episode surfaces | Project, Season, Episode, Character |
| Breakdown | PRD-BRK-001 | FSD-BREAKDOWN-001..019 | Breakdown / Source / Suggestions | Production Source, Breakdown, Breakdown Element |
| Production catalog | PRD-PROD-001 | FSD-PROD-* | Catalog / Production | Catalog Item |
| Locations | PRD-LOC-001 | FSD-LOC/PROD requirements | Locations | Location |
| Cast/Crew | PRD-CAST-001 | FSD-CAST/PROD requirements | Cast & Crew | Cast Member, Crew Member, Character |
| Visual planning | PRD-VIS-001, PRD-STB-001..003, PRD-SHOT-001 | FSD-STB/SOT requirements | Moodboards / Storyboards / Shot Lists | Moodboard, Storyboard, Storyboard Panel, Shot List, Shot |
| Scheduling | PRD-SCHED-001..003 | FSD-SCHED-001..016 | Stripboard / Shooting Days / Schedule | Shooting Schedule, Shooting Day, Schedule Marker |
| Call sheets | PRD-CALL-001 | FSD-CALL-001..012 | Call Sheets / Finalize / Refresh | Call Sheet |
| Collaboration | PRD-COL-001..006 | FSD-COL-001..022 | Exchange / LAN Collaboration | Exchange Package, Import Session, Collaboration Session, Application User |
| Offline/recovery | PRD-OFF-001 | FSD-OFF-001..010 | Offline / Save / Recovery | Project, Project File, Snapshot, Deleted Item |
| AI | PRD-AI-001..002 | FSD-AI-001..027 | AI Assistant / Global Command | AI Request, AI Result, AI Tool Invocation, Change Set, Snapshot |
| Project support | PRD-PROD-002..007, PRD-TPL-001, PRD-UX-001 | FSD support requirements | Notes / Tasks / Activity / Reports / Budget / Templates | Project Note, Task, Activity Entry, Production Report, Daily Production View, Budget Snapshot, Template |

## 25.2 PRD requirement → domain concept traceability

| PRD ID | Requirement | Priority | Domain concept(s) |
|---|---|---|---|
| PRD-CORE-001 | Application Shell, Navigation and Project Lifecycle | P0 | Application User, Project, Project Settings, Project Status, Snapshot, Deleted Item |
| PRD-IDEA-001 | Global and Project Idea Vault | P0 | Global Idea Vault Item, Project Idea Vault Item, Vault Collection, Project File |
| PRD-IDEA-002 | Idea Vault Supported Item Types | P0 | Global Idea Vault Item, Project Idea Vault Item, Vault Collection, Project File |
| PRD-IDEA-003 | Idea Vault Views and Organization | P0 | Global Idea Vault Item, Project Idea Vault Item, Vault Collection, Project File |
| PRD-IDEA-004 | Idea Vault Search, Preview and External File Awareness | P0 | Global Idea Vault Item, Project Idea Vault Item, Vault Collection, Project File |
| PRD-IDEA-005 | Move/Copy Idea Vault Material into Story | P0 | Global Idea Vault Item, Project Idea Vault Item, Vault Collection, Project File |
| PRD-STORY-001 | Story Board Views, Acts and Sequences | P0 | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| PRD-STORY-002 | Beat Cards and Beat Conversion | P0 | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| PRD-STORY-003 | Scene Cards and Scene Card Expansion | P0 | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| PRD-STORY-004 | Story Board Drag-and-Drop, Parking Lot, Multi-Select and Experimentation | P0 | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| PRD-STORY-005 | Story Board to Screenplay Build | P0 | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| PRD-STORY-006 | Characters and Character Relationship View | P0 | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| PRD-STORY-007 | Story Timeline | P1 | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| PRD-SCRIPT-001 | Screenplay Workspace and Writing Controls | P0 | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| PRD-SCRIPT-002 | Screenplay Draft History and Comparison | P0 | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| PRD-SCRIPT-003 | Review Rounds, Comments and Private Notes | P0 | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| PRD-SCRIPT-004 | Script Lock and Production Revisions | P1 | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| PRD-SCRIPT-005 | Screenplay Import | P0 | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| PRD-SCRIPT-006 | Screenplay Export and Printing | P0 | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| PRD-SCRIPT-007 | Additional File Interchange Formats | P2 | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| PRD-EP-001 | Episodic and Series Story Organization | P0 | Project, Season, Episode, Character |
| PRD-EP-002 | Deeper Episodic Continuity | P2 | Project, Season, Episode, Character |
| PRD-BRK-001 | Script Breakdown and Breakdown Suggestions | P0 | Production Source, Breakdown, Breakdown Element, Catalog Item |
| PRD-PROD-001 | Production Catalog | P0 | Catalog Item, Project Note, Task, Activity Entry, Production Report, Daily Production View, Budget Snapshot |
| PRD-LOC-001 | Locations and Location Notes | P0 | Location, Catalog Item |
| PRD-CAST-001 | Cast and Crew Directory | P0 | Cast Member, Crew Member, Character |
| PRD-VIS-001 | Moodboards and Visual References | P0 | Moodboard |
| PRD-STB-001 | Basic Storyboards | P0 | Storyboard, Storyboard Panel, Shot |
| PRD-STB-002 | Better Storyboard Tools | P1 | Storyboard, Storyboard Panel, Shot |
| PRD-STB-003 | Advanced Storyboard Editing | P2 | Storyboard, Storyboard Panel, Shot |
| PRD-SHOT-001 | Shot Lists | P0 | Shot List, Shot, Storyboard Panel |
| PRD-SCHED-001 | Stripboard, Shooting Schedule and Basic Schedule Assistance | P0 | Shooting Schedule, Shooting Day, Schedule Marker, Screenplay Scene, Production Source |
| PRD-SCHED-002 | Advanced Schedule Conflict Detection | P1 | Shooting Schedule, Shooting Day, Schedule Marker, Screenplay Scene, Production Source |
| PRD-SCHED-003 | More Sophisticated Schedule Assistance | P2 | Shooting Schedule, Shooting Day, Schedule Marker, Screenplay Scene, Production Source |
| PRD-CALL-001 | Call Sheets | P0 | Call Sheet, Shooting Day, Schedule Marker, Production Source |
| PRD-COL-001 | Permissions | P1 | Exchange Package, Import Session, Collaboration Session, Application User, Snapshot, Change Set |
| PRD-COL-002 | Review Packages | P0 | Exchange Package, Import Session, Collaboration Session, Application User, Snapshot, Change Set |
| PRD-COL-003 | Page-Specific Exchange Packages | P1 | Exchange Package, Import Session, Collaboration Session, Application User, Snapshot, Change Set |
| PRD-COL-004 | Better Exchange Package Controls | P1 | Exchange Package, Import Session, Collaboration Session, Application User, Snapshot, Change Set |
| PRD-COL-005 | Basic Local-Network Collaboration | P0 | Exchange Package, Import Session, Collaboration Session, Application User, Snapshot, Change Set |
| PRD-COL-006 | Advanced Collaboration Conflict Handling | P1 | Exchange Package, Import Session, Collaboration Session, Application User, Snapshot, Change Set |
| PRD-OFF-001 | Offline Operation, Saving, Portability, External Storage and Backup | P0 | Project, Project File, Snapshot, Deleted Item, Exchange Package |
| PRD-AI-001 | AI Assistant | P1 | AI Request, AI Result, AI Tool Invocation, Change Set, Snapshot |
| PRD-AI-002 | More Advanced AI Project Assistance | P2 | AI Request, AI Result, AI Tool Invocation, Change Set, Snapshot |
| PRD-PROD-002 | Daily Production View | P1 | Catalog Item, Project Note, Task, Activity Entry, Production Report, Daily Production View, Budget Snapshot |
| PRD-PROD-003 | Basic Production Reports | P1 | Catalog Item, Project Note, Task, Activity Entry, Production Report, Daily Production View, Budget Snapshot |
| PRD-PROD-004 | Sides | P1 | Catalog Item, Project Note, Task, Activity Entry, Production Report, Daily Production View, Budget Snapshot |
| PRD-PROD-005 | Lightweight Budget Snapshot | P1 | Catalog Item, Project Note, Task, Activity Entry, Production Report, Daily Production View, Budget Snapshot |
| PRD-PROD-006 | Project Notes, Lightweight Tasks and Activity History | P1 | Catalog Item, Project Note, Task, Activity Entry, Production Report, Daily Production View, Budget Snapshot |
| PRD-PROD-007 | More Production Reports | P2 | Catalog Item, Project Note, Task, Activity Entry, Production Report, Daily Production View, Budget Snapshot |
| PRD-TPL-001 | Templates | P1 | Template |
| PRD-UX-001 | Beginner/Professional Experience and Progressive Disclosure | P0 | Project, Application User, Snapshot |
| PRD-CORE-002 | Global Search, Files, Undo/Redo, Delete, Multi-Select, Keyboard and Drag-and-Drop Support | P0 | Application User, Project, Project Settings, Project Status, Snapshot, Deleted Item |
| PRD-CORE-003 | Project Dashboard, Project Status, Empty States, Errors and Document Identity | P0 | Application User, Project, Project Settings, Project Status, Snapshot, Deleted Item |
| PRD-CORE-004 | Richer Project Search | P2 | Application User, Project, Project Settings, Project Status, Snapshot, Deleted Item |

## 25.3 Canonical FSD requirement → domain concept traceability

Every canonical FSD requirement is mapped below. The domain mapping is intentionally concept-level: the FSD remains authoritative for behavior, while this document identifies the logical objects that carry or support that behavior.

| FSD ID | PRD ID | Priority | Functional action | Domain concept(s) |
|---|---|---|---|---|
| FSD-IDEA-001 | PRD-IDEA-001 | P0 | Create item | Global Idea Vault Item, Project Idea Vault Item, Vault Collection, Project File |
| FSD-IDEA-002 | PRD-IDEA-001 | P0 | Global/project separation | Global Idea Vault Item, Project Idea Vault Item, Vault Collection, Project File |
| FSD-IDEA-003 | PRD-IDEA-002 | P0 | Any file | Global Idea Vault Item, Project Idea Vault Item, Vault Collection, Project File |
| FSD-IDEA-004 | PRD-IDEA-002 | P0 | Untitled item | Global Idea Vault Item, Project Idea Vault Item, Vault Collection, Project File |
| FSD-IDEA-005 | PRD-IDEA-003 | P0 | Multiple views | Global Idea Vault Item, Project Idea Vault Item, Vault Collection, Project File |
| FSD-IDEA-006 | PRD-IDEA-003 | P0 | Folder move | Global Idea Vault Item, Project Idea Vault Item, Vault Collection, Project File |
| FSD-IDEA-007 | PRD-IDEA-003 | P0 | Collection | Global Idea Vault Item, Project Idea Vault Item, Vault Collection, Project File |
| FSD-IDEA-008 | PRD-IDEA-003 | P0 | Pin | Global Idea Vault Item, Project Idea Vault Item, Vault Collection, Project File |
| FSD-IDEA-009 | PRD-IDEA-004 | P0 | Search | Global Idea Vault Item, Project Idea Vault Item, Vault Collection, Project File |
| FSD-IDEA-010 | PRD-IDEA-004 | P0 | Preview | Global Idea Vault Item, Project Idea Vault Item, Vault Collection, Project File |
| FSD-IDEA-011 | PRD-IDEA-002 | P0 | Text note autosave | Global Idea Vault Item, Project Idea Vault Item, Vault Collection, Project File |
| FSD-IDEA-012 | PRD-IDEA-002 | P0 | URL fallback | Global Idea Vault Item, Project Idea Vault Item, Vault Collection, Project File |
| FSD-IDEA-013 | PRD-IDEA-002 | P0 | Voice note | Global Idea Vault Item, Project Idea Vault Item, Vault Collection, Project File |
| FSD-IDEA-014 | PRD-IDEA-001 | P0 | Project copy | Global Idea Vault Item, Project Idea Vault Item, Vault Collection, Project File |
| FSD-IDEA-015 | PRD-IDEA-005 | P0 | Move to story | Global Idea Vault Item, Project Idea Vault Item, Vault Collection, Project File |
| FSD-IDEA-016 | PRD-IDEA-005 | P0 | Original retention | Global Idea Vault Item, Project Idea Vault Item, Vault Collection, Project File |
| FSD-IDEA-017 | PRD-IDEA-005 | P0 | No live sync | Global Idea Vault Item, Project Idea Vault Item, Vault Collection, Project File |
| FSD-IDEA-018 | PRD-CORE-002 | P0 | Soft delete | Global Idea Vault Item, Project Idea Vault Item, Vault Collection, Project File |
| FSD-IDEA-019 | PRD-IDEA-003 | P0 | Multi-select | Global Idea Vault Item, Project Idea Vault Item, Vault Collection, Project File |
| FSD-IDEA-020 | PRD-IDEA-004 | P0 | External file awareness | Global Idea Vault Item, Project Idea Vault Item, Vault Collection, Project File |
| FSD-STORY-001 | PRD-STORY-001 | P0 | Create Act | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-STORY-002 | PRD-STORY-001 | P0 | Create Sequence | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-STORY-003 | PRD-STORY-001 | P0 | Sequence contains scenes | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-STORY-004 | PRD-STORY-002 | P0 | Create Beat | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-STORY-005 | PRD-STORY-003 | P0 | Create Scene | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-STORY-006 | PRD-STORY-003 | P0 | Compact card | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-STORY-007 | PRD-STORY-003 | P0 | Expand card | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-STORY-008 | PRD-STORY-004 | P0 | Move scene | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-STORY-009 | PRD-STORY-004 | P0 | Move sequence | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-STORY-010 | PRD-STORY-004 | P0 | Move act | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-STORY-011 | PRD-STORY-004 | P0 | Multi-select move | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-STORY-012 | PRD-STORY-004 | P0 | Parking Lot | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-STORY-013 | PRD-STORY-004 | P0 | Duplicate card | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-STORY-014 | PRD-STORY-004 | P0 | No branch engine | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-STORY-015 | PRD-STORY-004 | P0 | Undo move | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-STORY-016 | PRD-STORY-004 | P0 | Undo delete | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-STORY-017 | PRD-STORY-001 | P0 | Outline view parity | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-STORY-018 | PRD-STORY-001 | P0 | Collapse act | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-STORY-019 | PRD-STORY-001 | P0 | Collapse sequence | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-STORY-020 | PRD-STORY-005 | P0 | Build screenplay selection | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-STORY-021 | PRD-STORY-005 | P0 | Build preview | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-STORY-022 | PRD-STORY-005 | P0 | Missing heading warning | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-STORY-023 | PRD-STORY-005 | P0 | Build safety | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-STORY-024 | PRD-STORY-005 | P0 | Board independent after build | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-STORY-025 | PRD-STORY-005 | P0 | Script order warning | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-STORY-026 | PRD-STORY-003 | P0 | Characters optional | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-STORY-027 | PRD-STORY-003 | P0 | Story day optional | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-STORY-028 | PRD-STORY-001 | P0 | Sequence freeform name | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-STORY-029 | PRD-STORY-002 | P0 | Beat conversion | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-STORY-030 | PRD-STORY-002 | P0 | Scene-to-beat conversion | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-SCRIPT-001 | PRD-SCRIPT-001 | P0 | New screenplay | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-002 | PRD-SCRIPT-001 | P0 | Feature type | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-003 | PRD-SCRIPT-001 | P0 | Short type | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-004 | PRD-SCRIPT-001 | P0 | Episodic type | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-005 | PRD-SCRIPT-001 | P0 | Scene heading | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-006 | PRD-SCRIPT-001 | P0 | Action | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-007 | PRD-SCRIPT-001 | P0 | Character | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-008 | PRD-SCRIPT-001 | P0 | Dialogue | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-009 | PRD-SCRIPT-001 | P0 | Parenthetical | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-010 | PRD-SCRIPT-001 | P0 | Transition | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-011 | PRD-SCRIPT-001 | P0 | Shot direction | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-012 | PRD-SCRIPT-001 | P0 | Element switching | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-013 | PRD-SCRIPT-001 | P0 | Automatic progression | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-014 | PRD-SCRIPT-001 | P0 | Scene navigation | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-015 | PRD-SCRIPT-001 | P0 | Search | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-016 | PRD-SCRIPT-001 | P0 | Replace | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-017 | PRD-SCRIPT-001 | P0 | Focus mode | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-018 | PRD-SCRIPT-001 | P0 | Writing room | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-019 | PRD-SCRIPT-001 | P0 | Idea Vault separation | Global Idea Vault Item, Project Idea Vault Item, Vault Collection, Project File |
| FSD-SCRIPT-020 | PRD-SCRIPT-001 | P0 | Scene notes | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-021 | PRD-SCRIPT-001 | P0 | Autosave | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-022 | PRD-SCRIPT-001 | P0 | Undo/redo | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-023 | PRD-SCRIPT-002 | P0 | Named draft | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-024 | PRD-SCRIPT-002 | P0 | Draft lineage | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-025 | PRD-SCRIPT-002 | P0 | Current draft | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-026 | PRD-SCRIPT-002 | P0 | Automatic history | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-SCRIPT-027 | PRD-SCRIPT-002 | P0 | Draft comparison | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-028 | PRD-SCRIPT-003 | P0 | Review round | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-029 | PRD-SCRIPT-003 | P0 | Comment text | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-030 | PRD-SCRIPT-003 | P0 | Comment scene | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-031 | PRD-SCRIPT-003 | P0 | Reply | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-032 | PRD-SCRIPT-003 | P0 | Resolve | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-033 | PRD-SCRIPT-003 | P0 | Private note | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-034 | PRD-SCRIPT-004 | P1 | Lock draft | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-035 | PRD-SCRIPT-004 | P1 | Locked edit gate | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-036 | PRD-SCRIPT-004 | P1 | Revision label | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-037 | PRD-SCRIPT-004 | P1 | Revision comparison | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-038 | PRD-SCRIPT-005 | P0 | Import FDX | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-039 | PRD-SCRIPT-005 | P0 | Import PDF | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-040 | PRD-SCRIPT-005 | P0 | Import Fountain | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-041 | PRD-SCRIPT-005 | P0 | Import TXT | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-042 | PRD-SCRIPT-005 | P0 | Import DOCX | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-043 | PRD-SCRIPT-005 | P0 | Paste screenplay | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-044 | PRD-SCRIPT-005 | P0 | Import safety | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-045 | PRD-SCRIPT-006 | P0 | Export PDF | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-046 | PRD-SCRIPT-006 | P0 | Export FDX | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-047 | PRD-SCRIPT-006 | P0 | Export Fountain | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-048 | PRD-SCRIPT-006 | P0 | Export DOCX | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-049 | PRD-SCRIPT-006 | P0 | Title page | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-SCRIPT-050 | PRD-SCRIPT-006 | P0 | Scene numbering | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-BREAKDOWN-001 | PRD-BRK-001 | P0 | Select source | Production Source, Breakdown, Breakdown Element, Catalog Item |
| FSD-BREAKDOWN-002 | PRD-BRK-001 | P0 | Scene list | Production Source, Breakdown, Breakdown Element, Catalog Item |
| FSD-BREAKDOWN-003 | PRD-BRK-001 | P0 | Script pane | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-BREAKDOWN-004 | PRD-BRK-001 | P0 | Manual tag | Production Source, Breakdown, Breakdown Element, Catalog Item |
| FSD-BREAKDOWN-005 | PRD-BRK-001 | P0 | Highlight tag | Production Source, Breakdown, Breakdown Element, Catalog Item |
| FSD-BREAKDOWN-006 | PRD-BRK-001 | P0 | Categories | Production Source, Breakdown, Breakdown Element, Catalog Item |
| FSD-BREAKDOWN-007 | PRD-BRK-001 | P0 | Suggest elements | Production Source, Breakdown, Breakdown Element, Catalog Item |
| FSD-BREAKDOWN-008 | PRD-BRK-001 | P0 | Accept suggestion | Production Source, Breakdown, Breakdown Element, Catalog Item |
| FSD-BREAKDOWN-009 | PRD-BRK-001 | P0 | Reject suggestion | Production Source, Breakdown, Breakdown Element, Catalog Item |
| FSD-BREAKDOWN-010 | PRD-BRK-001 | P0 | Edit suggestion | Production Source, Breakdown, Breakdown Element, Catalog Item |
| FSD-BREAKDOWN-011 | PRD-BRK-001 | P0 | Batch suggestion accept | Production Source, Breakdown, Breakdown Element, Catalog Item |
| FSD-BREAKDOWN-012 | PRD-BRK-001 | P0 | Catalog reuse | Production Source, Breakdown, Breakdown Element, Catalog Item |
| FSD-BREAKDOWN-013 | PRD-BRK-001 | P0 | New catalog item | Production Source, Breakdown, Breakdown Element, Catalog Item |
| FSD-BREAKDOWN-014 | PRD-BRK-001 | P0 | Remove scene association | Production Source, Breakdown, Breakdown Element, Catalog Item |
| FSD-BREAKDOWN-015 | PRD-BRK-001 | P0 | Breakdown complete | Production Source, Breakdown, Breakdown Element, Catalog Item |
| FSD-BREAKDOWN-016 | PRD-BRK-001 | P0 | Needs review | Production Source, Breakdown, Breakdown Element, Catalog Item |
| FSD-BREAKDOWN-017 | PRD-BRK-001 | P0 | No silent deletion | Production Source, Breakdown, Breakdown Element, Catalog Item |
| FSD-BREAKDOWN-018 | PRD-BRK-001 | P0 | New scene state | Production Source, Breakdown, Breakdown Element, Catalog Item |
| FSD-BREAKDOWN-019 | PRD-BRK-001 | P0 | Changed heading | Production Source, Breakdown, Breakdown Element, Catalog Item |
| FSD-PROD-001 | PRD-PROD-001 | P0 | Catalog | Catalog Item, Project Note, Task, Activity Entry, Budget Snapshot, Production Report, Daily Production View, Template |
| FSD-PROD-002 | PRD-PROD-001 | P0 | Catalog usage | Catalog Item, Project Note, Task, Activity Entry, Budget Snapshot, Production Report, Daily Production View, Template |
| FSD-PROD-003 | PRD-PROD-001 | P0 | Catalog status | Catalog Item, Project Note, Task, Activity Entry, Budget Snapshot, Production Report, Daily Production View, Template |
| FSD-PROD-004 | PRD-LOC-001 | P0 | Location create | Catalog Item, Project Note, Task, Activity Entry, Budget Snapshot, Production Report, Daily Production View, Template |
| FSD-PROD-005 | PRD-LOC-001 | P0 | Location photos | Catalog Item, Project Note, Task, Activity Entry, Budget Snapshot, Production Report, Daily Production View, Template |
| FSD-PROD-006 | PRD-LOC-001 | P0 | Location notes | Catalog Item, Project Note, Task, Activity Entry, Budget Snapshot, Production Report, Daily Production View, Template |
| FSD-PROD-007 | PRD-LOC-001 | P0 | Location scenes | Catalog Item, Project Note, Task, Activity Entry, Budget Snapshot, Production Report, Daily Production View, Template |
| FSD-PROD-008 | PRD-CAST-001 | P0 | Cast create | Catalog Item, Project Note, Task, Activity Entry, Budget Snapshot, Production Report, Daily Production View, Template |
| FSD-PROD-009 | PRD-CAST-001 | P0 | Crew create | Catalog Item, Project Note, Task, Activity Entry, Budget Snapshot, Production Report, Daily Production View, Template |
| FSD-PROD-010 | PRD-CAST-001 | P0 | Cast availability | Catalog Item, Project Note, Task, Activity Entry, Budget Snapshot, Production Report, Daily Production View, Template |
| FSD-PROD-011 | PRD-VIS-001 | P0 | Moodboard create | Catalog Item, Project Note, Task, Activity Entry, Budget Snapshot, Production Report, Daily Production View, Template |
| FSD-PROD-012 | PRD-VIS-001 | P0 | Moodboard item | Catalog Item, Project Note, Task, Activity Entry, Budget Snapshot, Production Report, Daily Production View, Template |
| FSD-PROD-013 | PRD-VIS-001 | P0 | Moodboard export | Catalog Item, Project Note, Task, Activity Entry, Budget Snapshot, Production Report, Daily Production View, Template |
| FSD-PROD-014 | PRD-STB-001 | P0 | Storyboard create | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-PROD-015 | PRD-STB-001 | P0 | Storyboard panel | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-PROD-016 | PRD-STB-001 | P0 | Storyboard reorder | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-PROD-017 | PRD-STB-001 | P0 | Storyboard shot link | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-PROD-018 | PRD-SHOT-001 | P0 | Shot create | Catalog Item, Project Note, Task, Activity Entry, Budget Snapshot, Production Report, Daily Production View, Template |
| FSD-PROD-019 | PRD-SHOT-001 | P0 | Shot reorder | Catalog Item, Project Note, Task, Activity Entry, Budget Snapshot, Production Report, Daily Production View, Template |
| FSD-PROD-020 | PRD-SHOT-001 | P0 | Shot optional fields | Catalog Item, Project Note, Task, Activity Entry, Budget Snapshot, Production Report, Daily Production View, Template |
| FSD-PROD-021 | PRD-SHOT-001 | P0 | Shot export | Catalog Item, Project Note, Task, Activity Entry, Budget Snapshot, Production Report, Daily Production View, Template |
| FSD-PROD-022 | PRD-PROD-002 | P1 | Daily view | Catalog Item, Project Note, Task, Activity Entry, Budget Snapshot, Production Report, Daily Production View, Template |
| FSD-PROD-023 | PRD-PROD-003 | P1 | Reports | Catalog Item, Project Note, Task, Activity Entry, Budget Snapshot, Production Report, Daily Production View, Template |
| FSD-PROD-024 | PRD-PROD-004 | P1 | Sides | Catalog Item, Project Note, Task, Activity Entry, Budget Snapshot, Production Report, Daily Production View, Template |
| FSD-PROD-025 | PRD-PROD-005 | P1 | Budget snapshot | Catalog Item, Project Note, Task, Activity Entry, Budget Snapshot, Production Report, Daily Production View, Template |
| FSD-PROD-026 | PRD-PROD-005 | P1 | Budget boundary | Catalog Item, Project Note, Task, Activity Entry, Budget Snapshot, Production Report, Daily Production View, Template |
| FSD-SCHED-001 | PRD-SCHED-001 | P0 | Create schedule | Shooting Schedule, Shooting Day, Schedule Marker, Screenplay Scene, Production Source |
| FSD-SCHED-002 | PRD-SCHED-001 | P0 | Unscheduled pool | Shooting Schedule, Shooting Day, Schedule Marker, Screenplay Scene, Production Source |
| FSD-SCHED-003 | PRD-SCHED-001 | P0 | Shooting day | Shooting Schedule, Shooting Day, Schedule Marker, Screenplay Scene, Production Source |
| FSD-SCHED-004 | PRD-SCHED-001 | P0 | Drag scene | Shooting Schedule, Shooting Day, Schedule Marker, Screenplay Scene, Production Source |
| FSD-SCHED-005 | PRD-SCHED-001 | P0 | Reorder day | Shooting Schedule, Shooting Day, Schedule Marker, Screenplay Scene, Production Source |
| FSD-SCHED-006 | PRD-SCHED-001 | P0 | Move day | Shooting Schedule, Shooting Day, Schedule Marker, Screenplay Scene, Production Source |
| FSD-SCHED-007 | PRD-SCHED-001 | P0 | Off day | Shooting Schedule, Shooting Day, Schedule Marker, Screenplay Scene, Production Source |
| FSD-SCHED-008 | PRD-SCHED-001 | P0 | Break marker | Shooting Schedule, Shooting Day, Schedule Marker, Screenplay Scene, Production Source |
| FSD-SCHED-009 | PRD-SCHED-001 | P0 | Duration | Shooting Schedule, Shooting Day, Schedule Marker, Screenplay Scene, Production Source |
| FSD-SCHED-010 | PRD-SCHED-002 | P1 | Conflict actor | Shooting Schedule, Shooting Day, Schedule Marker, Screenplay Scene, Production Source |
| FSD-SCHED-011 | PRD-SCHED-002 | P1 | Conflict location | Location, Catalog Item |
| FSD-SCHED-012 | PRD-SCHED-002 | P1 | Overflow | Shooting Schedule, Shooting Day, Schedule Marker, Screenplay Scene, Production Source |
| FSD-SCHED-013 | PRD-SCHED-002 | P1 | Keep anyway | Shooting Schedule, Shooting Day, Schedule Marker, Screenplay Scene, Production Source |
| FSD-SCHED-014 | PRD-SCHED-001 | P0 | Suggestions | Shooting Schedule, Shooting Day, Schedule Marker, Screenplay Scene, Production Source |
| FSD-SCHED-015 | PRD-SCHED-001 | P0 | Calendar | Shooting Schedule, Shooting Day, Schedule Marker, Screenplay Scene, Production Source |
| FSD-SCHED-016 | PRD-SCHED-001 | P0 | Schedule export | Shooting Schedule, Shooting Day, Schedule Marker, Screenplay Scene, Production Source |
| FSD-CALL-001 | PRD-CALL-001 | P0 | Create from day | Call Sheet, Shooting Day, Schedule Marker, Production Source |
| FSD-CALL-002 | PRD-CALL-001 | P0 | Prefill scenes | Call Sheet, Shooting Day, Schedule Marker, Production Source |
| FSD-CALL-003 | PRD-CALL-001 | P0 | Prefill cast | Cast Member, Crew Member, Character |
| FSD-CALL-004 | PRD-CALL-001 | P0 | Prefill location | Location, Catalog Item |
| FSD-CALL-005 | PRD-CALL-001 | P0 | Call time edit | Call Sheet, Shooting Day, Schedule Marker, Production Source |
| FSD-CALL-006 | PRD-CALL-001 | P0 | Day notes | Call Sheet, Shooting Day, Schedule Marker, Production Source |
| FSD-CALL-007 | PRD-CALL-001 | P0 | Optional sections | Call Sheet, Shooting Day, Schedule Marker, Production Source |
| FSD-CALL-008 | PRD-CALL-001 | P0 | No reverse sync | Call Sheet, Shooting Day, Schedule Marker, Production Source |
| FSD-CALL-009 | PRD-CALL-001 | P0 | Stale state | Call Sheet, Shooting Day, Schedule Marker, Production Source |
| FSD-CALL-010 | PRD-CALL-001 | P0 | Refresh preview | Call Sheet, Shooting Day, Schedule Marker, Production Source |
| FSD-CALL-011 | PRD-CALL-001 | P0 | Issued snapshot | Shot List, Shot, Storyboard Panel |
| FSD-CALL-012 | PRD-CALL-001 | P0 | PDF export | Call Sheet, Shooting Day, Schedule Marker, Production Source |
| FSD-COL-001 | PRD-COL-001 | P1 | Owner | Exchange Package, Import Session, Collaboration Session, Application User, Snapshot, Change Set |
| FSD-COL-002 | PRD-COL-001 | P1 | Editor | Exchange Package, Import Session, Collaboration Session, Application User, Snapshot, Change Set |
| FSD-COL-003 | PRD-COL-001 | P1 | Commenter | Exchange Package, Import Session, Collaboration Session, Application User, Snapshot, Change Set |
| FSD-COL-004 | PRD-COL-001 | P1 | Viewer | Exchange Package, Import Session, Collaboration Session, Application User, Snapshot, Change Set |
| FSD-COL-005 | PRD-COL-001 | P1 | Export-only | Exchange Package, Import Session, Collaboration Session, Application User, Snapshot, Change Set |
| FSD-COL-006 | PRD-COL-003 | P1 | Exchange story | Act, Sequence, Beat, Scene Card, Parking Lot Entry, Character, Character Relationship, Story Timeline Entry |
| FSD-COL-007 | PRD-COL-002 | P0 | Exchange script | Screenplay, Screenplay Draft, Screenplay Scene, Screenplay Element, Automatic History Point, Review Round, Comment, Private Note, Production Source |
| FSD-COL-008 | PRD-COL-003 | P1 | Exchange breakdown | Production Source, Breakdown, Breakdown Element, Catalog Item |
| FSD-COL-009 | PRD-COL-003 | P1 | Exchange shots | Shot List, Shot, Storyboard Panel |
| FSD-COL-010 | PRD-COL-003 | P1 | Exchange schedule | Shooting Schedule, Shooting Day, Schedule Marker, Screenplay Scene, Production Source |
| FSD-COL-011 | PRD-COL-003 | P1 | Exchange call | Call Sheet, Shooting Day, Schedule Marker, Production Source |
| FSD-COL-012 | PRD-COL-004 | P1 | Preview before apply | Exchange Package, Import Session, Collaboration Session, Application User, Snapshot, Change Set |
| FSD-COL-013 | PRD-COL-004 | P1 | Stale review | Exchange Package, Import Session, Collaboration Session, Application User, Snapshot, Change Set |
| FSD-COL-014 | PRD-COL-004 | P1 | No blind overwrite | Exchange Package, Import Session, Collaboration Session, Application User, Snapshot, Change Set |
| FSD-COL-015 | PRD-COL-004 | P1 | Ambiguous mapping | Exchange Package, Import Session, Collaboration Session, Application User, Snapshot, Change Set |
| FSD-COL-016 | PRD-COL-004 | P1 | Unmapped note | Exchange Package, Import Session, Collaboration Session, Application User, Snapshot, Change Set |
| FSD-COL-017 | PRD-COL-005 | P0 | Local host | Location, Catalog Item |
| FSD-COL-018 | PRD-COL-005 | P0 | Join | Exchange Package, Import Session, Collaboration Session, Application User, Snapshot, Change Set |
| FSD-COL-019 | PRD-COL-005 | P0 | Presence | Exchange Package, Import Session, Collaboration Session, Application User, Snapshot, Change Set |
| FSD-COL-020 | PRD-COL-006 | P1 | Same-object conflict | Exchange Package, Import Session, Collaboration Session, Application User, Snapshot, Change Set |
| FSD-COL-021 | PRD-COL-005 | P0 | Disconnect | Exchange Package, Import Session, Collaboration Session, Application User, Snapshot, Change Set |
| FSD-COL-022 | PRD-COL-005 | P0 | End session | Exchange Package, Import Session, Collaboration Session, Application User, Snapshot, Change Set |
| FSD-OFF-001 | PRD-OFF-001 | P0 | Offline open | Project, Snapshot, Deleted Item, Project File, Exchange Package |
| FSD-OFF-002 | PRD-OFF-001 | P0 | Offline edit | Project, Snapshot, Deleted Item, Project File, Exchange Package |
| FSD-OFF-003 | PRD-OFF-001 | P0 | Offline production | Catalog Item, Project Note, Task, Activity Entry, Budget Snapshot, Production Report, Daily Production View, Template |
| FSD-OFF-004 | PRD-OFF-001 | P0 | Offline export | Project, Snapshot, Deleted Item, Project File, Exchange Package |
| FSD-OFF-005 | PRD-OFF-001 | P0 | Save status | Project, Snapshot, Deleted Item, Project File, Exchange Package |
| FSD-OFF-006 | PRD-OFF-001 | P0 | Recovery | Project, Snapshot, Deleted Item, Project File, Exchange Package |
| FSD-OFF-007 | PRD-OFF-001 | P0 | Backup | Project, Snapshot, Deleted Item, Project File, Exchange Package |
| FSD-OFF-008 | PRD-OFF-001 | P0 | Full project portability | Project, Snapshot, Deleted Item, Project File, Exchange Package |
| FSD-OFF-009 | PRD-OFF-001 | P0 | External drive | Project, Snapshot, Deleted Item, Project File, Exchange Package |
| FSD-OFF-010 | PRD-OFF-001 | P0 | Drive loss | Project, Snapshot, Deleted Item, Project File, Exchange Package |
| FSD-AI-001 | PRD-AI-001 | P1 | Optional | AI Request, AI Result, Change Set, Snapshot |
| FSD-AI-002 | PRD-AI-001 | P1 | Context selector | AI Request, AI Result, Change Set, Snapshot |
| FSD-AI-003 | PRD-AI-001 | P1 | Question answering | AI Request, AI Result, Change Set, Snapshot |
| FSD-AI-004 | PRD-AI-001 | P1 | Scene card suggestion | AI Request, AI Result, Change Set, Snapshot |
| FSD-AI-005 | PRD-AI-001 | P1 | Breakdown suggestion | Production Source, Breakdown, Breakdown Element, Catalog Item |
| FSD-AI-006 | PRD-AI-001 | P1 | Synopsis | AI Request, AI Result, Change Set, Snapshot |
| FSD-AI-007 | PRD-AI-001 | P1 | Schedule advice | Shooting Schedule, Shooting Day, Schedule Marker, Screenplay Scene, Production Source |
| FSD-AI-008 | PRD-AI-001 | P1 | Mutation preview | AI Request, AI Result, Change Set, Snapshot |
| FSD-AI-009 | PRD-AI-001 | P1 | Accept/reject | AI Request, AI Result, Change Set, Snapshot |
| FSD-AI-010 | PRD-AI-001 | P1 | No silent rewrite | AI Request, AI Result, Change Set, Snapshot |
| FSD-AI-011 | PRD-AI-001 | P1 | External disclosure | AI Request, AI Result, Change Set, Snapshot |
| FSD-AI-012 | PRD-AI-001 | P1 | AI failure safety | AI Request, AI Result, Change Set, Snapshot |
| FSD-AI-013 | PRD-AI-001 | P1 | System-wide natural-language command interface | AI Request, AI Result, Change Set, Snapshot |
| FSD-AI-014 | PRD-AI-001 | P1 | Deterministic project statistics and exact queries | AI Request, AI Result, Snapshot |
| FSD-AI-015 | PRD-AI-001 | P1 | Cross-module relationship queries | AI Request, AI Result, Snapshot |
| FSD-AI-016 | PRD-AI-001 | P1 | Natural-language navigation and application routing | AI Request, AI Result |
| FSD-AI-017 | PRD-AI-001 | P1 | Structured project-wide rename/replace | AI Request, AI Result, Change Set, Snapshot |
| FSD-AI-018 | PRD-AI-001 | P1 | Batch mutation preparation and review | AI Request, AI Result, Change Set, Snapshot |
| FSD-AI-019 | PRD-AI-001 | P1 | Permission inheritance | AI Request, Change Set |
| FSD-AI-020 | PRD-AI-001 | P1 | Locked/private/approval state enforcement | AI Request, Change Set |
| FSD-AI-021 | PRD-AI-001 | P1 | Change Set base-version validation | AI Request, AI Result, Change Set, Snapshot |
| FSD-AI-022 | PRD-AI-001 | P1 | Application tool boundary and validation | AI Request, Change Set |
| FSD-AI-023 | PRD-AI-001 | P1 | Product knowledge access | AI Request, AI Result |
| FSD-AI-024 | PRD-AI-001 | P1 | Scope/provenance disclosure | AI Request, AI Result |
| FSD-AI-025 | PRD-AI-001 | P1 | AI lifecycle/activity integration | AI Request, AI Result, Change Set, Snapshot |
| FSD-AI-026 | PRD-AI-001 | P1 | Conversation/session context | AI Request, AI Result |
| FSD-AI-027 | PRD-AI-001 | P1 | Cross-project/episodic scope control | AI Request, AI Result, Change Set |

## 25.4 Exact consistency checks

- PRD requirement IDs in PRD registry: **55**.
- Canonical FSD requirement IDs in FSD inventory: **232**.
- Distinct PRD IDs represented in UX/UI: **55**; missing PRD IDs: **0**.
- Distinct canonical FSD IDs represented in UX/UI: **232**; missing canonical FSD IDs: **0**.
- Auxiliary shorthand FSD acceptance IDs outside canonical inventory: **11** (FSD-BRK-001, FSD-BRK-002, FSD-BRK-003, FSD-CAST-001, FSD-CAT-001, FSD-LOC-001, FSD-SCH-001, FSD-SCH-002, FSD-SCH-003, FSD-SHOT-001, FSD-STB-001).
- FSD requirements with an explicit PRD ID in the canonical inventory: **232 / 232**.
- FSD/PRD priority mismatches: **0**.
- Canonical FSD requirements mapped to at least one logical domain concept in this document: **232 / 232**.

The 11 auxiliary shorthand FSD IDs are treated as alias/acceptance labels outside the 232-row canonical inventory. They do not create additional domain requirements or objects.

**Final interpretation:** the source set is internally traceable at requirement level: PRD defines scope and priority, FSD defines function, UX/UI defines presentation, and Domain/Data defines logical representation and relationships. A Domain/Data concept may support multiple requirements, and a requirement may map to multiple domain concepts; this is expected and does not imply duplicated sources of truth.

# 26. Consistency Rules for Future Changes

- Every new PRD requirement must receive a PRD ID before downstream specifications reference it.
- Every new FSD requirement must reference an approved PRD ID and inherit its priority.
- Every new UX surface must map to one or more PRD/FSD requirements and must not silently create scope.
- Every new domain object must be linked to an approved PRD/FSD requirement or explicitly documented as an internal supporting construct.
- Changing object authority or relationship cardinality requires a cross-document consistency review.
- Changing scene numbering, source-version behavior, offline ownership, exchange behavior, episodic identity, or AI mutation policy requires updates across PRD/FSD/UX/Domain together.
- No document should introduce contradictory names for the same concept; use the canonical terminology in this specification.
- The PRD priority remains authoritative; the FSD and Domain/Data documents must not promote/demote a requirement independently.
- UX/UI may choose presentation but must not create domain objects solely because a screen needs another visual grouping unless the object is explicitly an internal projection/support construct.

# 27. Domain-Level Acceptance Checklist

- Project can exist with only a title/type and no setup wizard data.
- Global and Project Idea Vaults support unstructured items without mandatory classification.
- Copying a Vault item produces an independent item.
- Acts, Sequences, Beats and Scene Cards have stable identities and drag-safe ordering.
- Scene Cards never require manual scene numbering.
- Sequences remain simple named containers.
- Board changes do not silently mutate screenplay text.
- Build Screenplay creates screenplay scenes with stable identities and order-derived numbers.
- Named drafts are distinct from automatic history points.
- Locked drafts require an explicit revision action before editing.
- Comments/private notes remain attached to intended targets and respect visibility rules.
- Production Source identifies the screenplay version on which breakdown/schedule work is based.
- Breakdown suggestions remain non-authoritative until confirmed.
- Catalog Items are reusable and scene usage is derived.
- Shot Lists and Storyboards remain optionally linked rather than mutually required.
- Schedules use source scenes and do not duplicate screenplay text as a second script.
- Call Sheets can be generated from Shooting Days and independently edited/finalized.
- Exchange Packages can be previewed/imported without blind overwrite.
- Offline local projects retain ownership and core functionality.
- AI outputs are not authoritative until accepted.
- Exact project facts are calculated from canonical domain data, not model memory.
- AI-originated mutations require a proposed Change Set with explicit user approval.
- Applied AI Change Sets use normal domain mutation semantics and remain subject to undo/recovery/history rules.
- AI permissions never exceed the current user's effective permissions.
- AI Tool Invocations cannot directly bypass the domain/data layer.
- Stale or conflicted AI Change Sets cannot be applied without revalidation.
- Recoverable deletion preserves enough context for restoration.
- Season and Episode exist only for episodic/series projects and do not change feature/short workflows.
- All domain objects named by the UI/FSD have one canonical logical meaning.
- Every canonical FSD requirement maps to at least one domain concept.

# 28. Final Domain/Data Boundary

OpenFrame should be implemented as a connected set of domain objects with clear authority rather than as a collection of duplicated records. The most important logical separation is: Idea Vault is memory, Story Board is outline, Screenplay is written truth, Production Source selects the screenplay baseline for production planning, Breakdown/Catalog represent production requirements, Shot List/Storyboard represent visual planning, Schedule represents shooting order, and Call Sheet represents the communication document for a shooting day.

That separation is what allows the product to remain simple for independent filmmakers while still supporting professional pre-production behavior. The domain model intentionally avoids creating a giant universal object that tries to synchronize every module with every other module.

# Appendix A — Source Consistency Audit Metadata

- **PRD path inspected:** OpenFrame_Studio_Mega_PRD_Aligned_Updated.md
- **FSD path inspected:** OpenFrame_Studio_Functional_Specification_Document_Aligned_Updated.md
- **UX/UI path inspected:** OpenFrame_Studio_UX_UI_Specification_Updated.md
- **PRD line count:** 5736
- **FSD line count:** 5809
- **UX/UI line count:** 4549
- **PRD distinct IDs:** 55
- **Canonical FSD inventory IDs:** 232
- **Auxiliary FSD IDs:** 11
- **UX PRD IDs:** 55
- **UX canonical FSD IDs:** 232
- **UX missing PRD IDs:** 0
- **UX missing canonical FSD IDs:** 0
- **FSD rows with PRD pairing:** 232 / 232
- **FSD/PRD priority mismatches:** 0
- **Domain coverage of canonical FSD requirements:** 232 / 232

**Audit conclusion:** PRD, FSD and UX/UI are requirement-traceable at the source level, and every canonical FSD requirement is represented by at least one logical domain concept. No priority mismatch was detected. The 11 shorthand FSD labels outside the canonical 217-row inventory are treated as aliases and do not expand product scope. The Domain/Data document therefore acts as a logical representation layer rather than a competing product specification.

