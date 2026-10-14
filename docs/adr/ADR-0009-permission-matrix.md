# ADR-0009: Permission matrix: decisions for cells the Security spec leaves open

- **Status:** Accepted (implemented in `crates/openframe-domain/src/auth.rs`, `Role::allows`)
- **Date:** 2026-09-30
- **Related:** Security spec §5–§8, §16.4, SEC-004..009 (digest `docs/spec-digest/security.md` §6.3, §26 item 9);
  FSD §46; ADR-0007 (LAN removed)

## Context

The Security spec fixes **five roles** (Owner, Editor, Commenter, Viewer, Export-only) as the entire security model.
Professional labels such as Writer, Director or Reviewer are not roles. The spec defines role *boundaries*, but many
cells of the per-action matrix are "not stated" (digest §6.3 "—"). The spec asks engineering to decide them
explicitly rather than invent silently.

## Decision

### Mechanism

- Every operation declares exactly one **`Capability`**. Queries call `actor.require(Capability::View, …)`, and
  commands pass the capability in `MutationMeta`, where `Store::mutate` checks it **before** opening the transaction.
  A denial never mutates (`permission.denied`, "You don't have permission to … in this project.").
- Object-state rules are checked by the module *in addition to* the capability: locked drafts
  (`permission.locked_draft`), private content (`permission.private`), finalized documents. A capability never lets
  anyone bypass a locked or approved state (Security §6.3–6.4).
- **Local use:** whoever opens a project folder on their own computer is its **Owner** (`project.open` adds the local
  profile as an Owner member if absent). A local folder is a user-owned working asset (FSD §45, Security §2).
  Non-Owner roles apply to **packages and exports**. They define what a collaborator receiving a review/exchange
  package may do with it, and what a response package may propose. A response is always applied by the receiving
  project's Owner/Editor through validate → preview → Change Set → apply. With LAN collaboration removed (ADR-0007),
  no one acts inside another user's project database.

### The matrix (✔ = allowed)

| Capability | Covers | Owner | Editor | Commenter | Viewer | Export-only |
|---|---|:-:|:-:|:-:|:-:|:-:|
| `View` | Read permitted content, search, activity, Recently Deleted list | ✔ | ✔ | ✔ | ✔ | ✔ ¹ |
| `Edit` | Create/edit ordinary content (vault, story, screenplay, production, files); explicit Save | ✔ | ✔ | | | |
| `SoftDelete` | Move to Recently Deleted; restore | ✔ | ✔ ² | | | |
| `PermanentDelete` | Purge from Recently Deleted; delete a project (to the Recycle Bin) | ✔ | | | | |
| `Comment` | Add comments and replies; own Private Notes (create/edit) ³ | ✔ | ✔ ² | ✔ | | |
| `ResolveComments` | Resolve/reopen comments, complete review rounds | ✔ | ✔ ² | ✔ | | |
| `LockOrFinalize` | Lock/unlock a shooting draft; finalize/supersede production documents | ✔ | ✔ ² | | | |
| `ManageProject` | Settings, status, archive, duplicate, production-source selection | ✔ | | | | |
| `ManagePermissions` | Change collaborator roles | ✔ | | | | |
| `Export` | Document exports (PDF/FDX/…) | ✔ | ✔ ² | | | ✔ |
| `CreatePackage` | Exchange/review/full-project/backup packages | ✔ | ✔ ² | | | ✔ |
| `Import` | Import that mutates the project (always via preview/Change Set) | ✔ | ✔ | | | |
| `ApplyChangeSet` | Accept/apply AI, import or review Change Sets | ✔ | ✔ | | | |
| ~~`HostSession`~~ | *Obsolete: LAN removed (ADR-0007). Granted by nothing that is used; remove from `Capability`.* | — | — | — | — | — |
| ~~`JoinSession`~~ | *Obsolete: LAN removed (ADR-0007). Remove from `Capability`.* | — | — | — | — | — |
| `UseAi` | Ask read/compute/navigate questions (reads use the asker's permissions) | ✔ | ✔ | ✔ | ✔ | ² |

¹ Implied by the spec. Export-only must see content to produce permitted snapshots.
² **Decided here** (the spec cell was "—"). Rationale below.
³ Private Notes are per user (`owner_user_id = actor.user_id` in every query, including search). Nobody, including
the Owner, can read another user's Private Notes (Security §8). Reading your own existing notes needs only `View`.
Creating or editing them needs `Comment`, so a Viewer stays strictly read-only (Security §6.6).

### Rationale for the decided cells

| Cell | Decision | Why |
|---|---|---|
| Editor: delete/restore | Allowed (recoverable only) | Deletion is recoverable and undoable, so it is part of editing. Permanent deletion stays Owner-only ("unrestricted destructive control" is not granted, §6.4). |
| Owner/Editor: comment, resolve | Allowed | Anyone who can edit can annotate. Blocking it would be absurd. |
| Editor: lock/unlock, finalize | Allowed | PRD: lock is undoable "by authorized users" (N-2). Locking or unlocking is an explicit, recorded action and not a bypass. Editors still cannot *edit* locked content without the revision flow. |
| Editor: export, packages | Allowed | Editors already hold the content, and export is how they hand work to others. |
| Commenter/Viewer: export, packages | **Denied** | Export is data egress. Viewer "cannot turn a read into … export workflow" (§6.6), and the same caution is applied to Commenter. Export-only exists for this purpose. |
| Live sessions (host/join) | Not applicable | LAN collaboration removed by product-owner decision (ADR-0007). `Role::allows` still answers for `HostSession`/`JoinSession`, but no operation uses them. |
| Export-only: AI | Denied | Not needed for the export purpose. Least privilege. |
| Export-only: import, change sets | Denied | "without editing project data" (§6.7) |
| Duplicate project | `ManageProject` (Owner) | Creates a new project identity from this one, which is a project-level act |

## Consequences

- The matrix is small, total and unit-tested (`auth.rs` tests: viewer cannot mutate, Export-only cannot join,
  only Owner manages permissions, hosts or permanently deletes, Commenter cannot edit).
- Every module test suite must include Viewer and Commenter denial tests (CONTRIBUTING §4).
- **Hardening note:** `history.undo` / `history.redo` perform no capability check. They only replay the actor's own
  earlier steps, which were authorized when made. The local actor is always Owner and LAN is removed, so there is no
  exposure today. For defence in depth, undo/redo should still require `Edit` (a small foundation change in
  `modules/history.rs`).
- The Export-only "cannot join" unit test and the `HostSession`/`JoinSession` variants become dead code once the
  capabilities are removed (ADR-0007).
- Changing a cell later is a product decision: amend this ADR and the tests together.

## Alternatives rejected

| Alternative | Reason |
|---|---|
| Per-object ACLs | No spec basis. Contradicts "these roles are the security model" and "no separate parallel ownership database". |
| Professional labels as roles (Writer, Director, …) | Explicitly rejected by the Terminology Lock (Security App. B) |
| Checking permissions in React | The UI is untrusted. Checks must run in Rust before any transaction. The UI only mirrors them to hide unavailable actions. |
