# Archive index

> Written 2026-10-09. History is kept, not erased. This page lists what is in the archive, what is kept elsewhere but is
> historical, and why. Nothing in the archive is a requirement. Where an archived text and a living document disagree,
> the living document wins ([documentation map](../README.md)). The files in this folder are not edited; mistakes in them
> are recorded in the [reconciliation log](../implementation/reconciliation-log.md).

## 1. In this folder

| File | What it is | Why it matters | Replaced by |
|---|---|---|---|
| `2026-10-03-end-to-end-completion-plan-source.md` | The 4,900-line plan of 2026-10-03: the six completion states, the "hard definition of done", parallel lanes A to G, programs per feature area, the pull request acceptance checklist | The origin of the feature status words and the definition of done that still govern the project | [masterplan.md](../implementation/masterplan.md), [feature-status.md](../implementation/feature-status.md), [CONTRIBUTING.md](../../CONTRIBUTING.md) |
| `2026-10-06-masterplan-with-item-codes.md` | The plan as it stood before the rename: milestones M0 to M16, waves, item codes such as M1.04 | Old evidence folders and old log entries cite these codes | The 26 numbered areas of the masterplan; code translation tables are in the reconciliation log |
| `2026-10-06-feature-ledger-with-item-codes.md` | The feature ledger before the rename (states S0 to S5) | Same | feature-status.md |
| `2026-10-06-owner-decisions-with-item-codes.md` | Owner decisions D1 to D18 with codes | Same | owner-decisions.md |
| `2026-10-06-reconciliation-log-with-item-codes.md` | The reconciliation log before the rename | Same | reconciliation-log.md |
| `2026-10-06-launch-register-before-restructure.md` | The launch readiness register before it was cut down | Keeps rows that were moved | [launch-readiness-register.md](../release/launch-readiness-register.md) |
| `notes/documentation-stack-guidance.md` | Raw intake notes on which documents a project needs | Shaped the documentation map | [../README.md](../README.md) |
| `notes/store-certification-and-launch-gap-notes.md` | Raw intake notes on store certification gaps | Source for the launch register | launch register, [Chapter 3](../roadmap/chapter-3-commercial-distribution.md) |

## 2. Kept in place, but historical

These files could not be moved without changing source code, scripts or test comments that cite their paths, and the
2026-10-09 restructure did not touch source code. Treat them as history or as reference, as noted.

| Path | Status | Why it stays | What to use instead |
|---|---|---|---|
| `docs/spec-digest/` (eight digests and a README, about 7,300 lines) | Historical. Derived summaries written before ADR-0007, ADR-0013 and Revision 1; their LAN passages are superseded | Cited by ADRs, the progress log and `scripts/check-writing-standard.mjs` (exempt list) | The full specifications in [../specs/](../specs) |
| `docs/engineering/AGENT_BRIEF.md` | Partly stale module brief of 2026-09-30 (says specifications are in the repository root, mentions digests) | [DEVELOPMENT.md](../../DEVELOPMENT.md) and ADR-0005 cite it | [CONTRIBUTING.md](../../CONTRIBUTING.md), [architecture/README.md](../architecture/README.md) |
| `docs/engineering/SECURITY_REVIEW_2026-09-30.md` | Evidence snapshot of the first security review | Rust test files cite its path in comments | [20-security-threat-model.md](../engineering/20-security-threat-model.md) |
| `docs/audits/` | Two dated audits (2026-10-02, 2026-10-03). Evidence of what was true on those days | Already a record folder | [feature-status.md](../implementation/feature-status.md) |
| `docs/release/evidence/M0` to `M14`, `release` | Evidence folders named with old item codes | Test results and screenshots; renaming would break citations | The translation tables in the reconciliation log |
| `docs/implementation/progress-log.md`, section "Earlier entries" | Log entries with old item codes | The history of sessions on 2026-10-05 | The newer entries above it |
| `docs/website/prompts/` | Two supplied prompts; tool names in them were never verified | Referenced from the website plan | [Chapter 2 plan](../roadmap/chapter-2-website.md) |
| `docs/engineering/28-...`, `29-...`, `31-...` | Plans promoted from supplied documents on 2026-10-05 (cross-platform plan, target HLD and LLD, AI-assisted development controls) | Their rules are enforced in `CONTRIBUTING.md` and the architecture lint | Current state: [37-platform-boundary.md](../engineering/37-platform-boundary.md), [architecture/README.md](../architecture/README.md) |
| `HTML_Mockups/` | Design reference, 181 screens. Includes removed features (live LAN, mock-ups 164 to 168) and a roles screen (155) that is not in the product | The UX specification and the visual rules cite them | The running application; [Chapter 2](../roadmap/chapter-2-website.md) explains how website screens are made |
| `docs/manual/` | The user manual (.docx, .pdf) still has the removed LAN figures 164 to 168 | The manual source is not in the repository as text | To be rebuilt from the shipped interface (build plan area 17) |

## 3. If a file here is needed again

Copy it to a living location, update it, and add a row to the [decision register](../adr/README.md) or the reconciliation
log saying what changed and why. Do not edit the archived copy.

## 4. A later clean-up that needs code changes

After the next code change window, the files in section 2 can move into this folder: update the exempt list in
`scripts/check-writing-standard.mjs`, the comments in the two Rust test files, `DEVELOPMENT.md` and the ADR citations in
one commit, then run `node scripts/check-links.mjs`.
