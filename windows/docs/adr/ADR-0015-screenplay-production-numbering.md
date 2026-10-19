# ADR-0015: Screenplay production numbering, revisions and locked pages

- **Status:** Accepted 2026-10-07 (owner answer in chat). Amends ADR-0011 (item 3) and answers the first open question in ADR-0012.
- **Date:** 2026-10-05
- **Related:** ADR-0011; ADR-0012 (open product decisions); Screenplay and Production Depth Specification §§10 to 13;
  `migrations/project/0004_screenplay.sql`

## Context

ADR-0011 stores **no display numbers**: scene numbers are derived from position and renumber on reorder. The
revised specification requires: automatic numbering, **import of existing scene numbers**, **alphanumeric
production numbers** (12A), export of numbers, and a production lock after which pagination and numbering form a
baseline for revisions. ADR-0011 and ADR-0012 both flag "frozen numbers after lock" as an open product decision.
Today `screenplay_scene` has no number column, `screenplay_draft` has `locked_at`, `revision_label`,
`revision_color`, `revision_reason`, and `screenplay_element` has `revision_mark`. No changed-pages record exists.

## Decision

1. Scene **identity** remains the UUIDv7 `id` and `lineage_id`. A number is never identity and no foreign key may
   reference it.
2. Add a nullable **`production_number` label** per scene per draft. While a draft is unlocked the label is
   derived and not stored (current behaviour). On **Lock Shooting Draft** the derived numbers are frozen into
   `production_number`. Imported numbers are stored as labels with an `imported` source flag.
3. After lock, new scenes receive inserted labels (12A, 12B) by rule, deleted scenes keep their label with
   `omitted = 1` (existing column) and render as OMITTED. Reordering never changes a frozen label.
4. Add a revision record per `Start Revision` (label, colour, source draft, date, reason) and a derived
   **changed pages and changed scenes** list computed against the locked pagination baseline. Revision colour is a
   production revision marker, never general text colour.
5. Changing a label that has production dependencies shows a dependency warning and stays undoable.
6. The migration is additive (new nullable columns and tables). No existing migration file changes.

## Consequences

- Breakdown, catalog, shots, schedule, call sheets and storyboard keep referencing `scene_id`/`lineage_id`; a
  label change cannot break them.
- Export formats carry labels where supported (FDX scene numbers); Fountain maps by documented rules.
- ADR-0011 item 3 gets an amendment pointer; the "open decision" line in ADR-0012 is closed by this ADR.

## Alternatives rejected

| Alternative | Reason |
|---|---|
| Always derive numbers | Cannot represent 12A or imported production drafts. |
| Store numbers as identity | Breaks every reference on reorder (the reason for ADR-0011). |
