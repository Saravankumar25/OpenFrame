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
| `2026-10-15-chapter-1-plan-before-restructure.md` | The seven-stream Chapter 1 plan of 2026-10-09 | Source of the work items now in the Windows and Mac phase plans | [chapter-1a-windows-phases.md](../roadmap/chapter-1a-windows-phases.md), [chapter-1b-mac-phases.md](../roadmap/chapter-1b-mac-phases.md) |
| `2026-10-15-chapter-2-plan-before-restructure.md` | The Chapter 2 website plan of 2026-10-09 | Page list and the faithful-screens method | [chapter-2-website.md](../roadmap/chapter-2-website.md) |
| `2026-10-15-chapter-3-plan-before-restructure.md` | The Chapter 3 plan of 2026-10-09 | Store research table and requirements | [chapter-3-commercial-distribution.md](../roadmap/chapter-3-commercial-distribution.md) |
| `2026-10-15-roadmap-map-before-restructure.md` | The roadmap map of 2026-10-09 | Area-to-chapter table | [roadmap/README.md](../roadmap/README.md) |
| `2026-10-15-owner-decisions-before-restructure.md` | The owner decisions table including replaced rows | Records what was answered when | [owner-decisions.md](../implementation/owner-decisions.md) |
| `2026-10-15-vision-and-scope-before-restructure.md` | The vision page of 2026-10-09 | Earlier wording of scope | [vision-and-scope.md](../product/vision-and-scope.md) |
| `2026-10-15-documentation-map-before-restructure.md` | The documentation map of 2026-10-09 | Earlier layout and precedence | [../README.md](../README.md) |
| `2026-09-spec-digest/` | Eight digests of the product specifications and their README (about 7,300 lines), written before ADR-0007, ADR-0013 and Revision 1. Moved here in Windows phase W1 | ADR-0009 and ADR-0012 cite them for the contradictions they listed | The full specifications in [../specs/](../specs) |
| `2026-09-30-agent-brief.md` | The module brief of 2026-09-30 (was `docs/engineering/AGENT_BRIEF.md`). Moved in W1 | ADR-0005 cites its section 4 | [CONTRIBUTING.md](../../CONTRIBUTING.md), [architecture/README.md](../architecture/README.md) |
| `2026-09-30-security-review.md` | The first security review (was `docs/engineering/SECURITY_REVIEW_2026-09-30.md`). Moved in W1 | `security_review.rs` and `security_paths.rs` name its finding IDs | [20-security-threat-model.md](../engineering/20-security-threat-model.md) |
| `2026-10-05-cross-platform-compatibility-security-plan.md` | Engineering document 28, the Tauri-on-both cross-platform plan. Moved in W1 | ADR-0017 cites it | [37-platform-boundary.md](../engineering/37-platform-boundary.md), [ADR-0027](../adr/ADR-0027-rust-core-two-ui-shells.md) |
| `2026-10-05-desktop-hld-lld-target.md` | Engineering document 29, the target high- and low-level design. Moved in W1 | The ESD specification points to it | [architecture/README.md](../architecture/README.md) |
| `2026-10-05-ai-assisted-development-controls.md` | Engineering document 31, AI-assisted development controls. Moved in W1 | Its risk levels and feature contract were copied into CONTRIBUTING.md section 8 | [CONTRIBUTING.md](../../CONTRIBUTING.md) section 8 |
| `notes/documentation-stack-guidance.md` | Raw intake notes on which documents a project needs | Shaped the documentation map | [../README.md](../README.md) |
| `notes/store-certification-and-launch-gap-notes.md` | Raw intake notes on store certification gaps | Source for the launch register | launch register, [Chapter 3](../roadmap/chapter-3-commercial-distribution.md) |

## 2. Kept in place, but historical

These files could not be moved without changing source code, scripts or test comments that cite their paths, and the
2026-10-09 restructure did not touch source code. Treat them as history or as reference, as noted.

| Path | Status | Why it stays | What to use instead |
|---|---|---|---|
| `docs/audits/` | Two dated audits (2026-10-02, 2026-10-03). Evidence of what was true on those days | Already a record folder | [feature-status.md](../implementation/feature-status.md) |
| `docs/release/evidence/M0` to `M14`, `release` | Evidence folders named with old item codes | Test results and screenshots; renaming would break citations | The translation tables in the reconciliation log |
| `docs/implementation/progress-log.md`, section "Earlier entries" | Log entries with old item codes | The history of sessions on 2026-10-05 | The newer entries above it |
| `docs/website/prompts/` | Two supplied prompts; tool names in them were never verified | Referenced from the website plan | [Chapter 2 plan](../roadmap/chapter-2-website.md) |
| `HTML_Mockups/` | Design reference, 181 screens. Includes removed features (live LAN, mock-ups 164 to 168) and a roles screen (155) that is not in the product | The UX specification and the visual rules cite them | The running application; [Chapter 2](../roadmap/chapter-2-website.md) explains how website screens are made |
| `docs/manual/` | The user manual (.docx, .pdf) still has the removed LAN figures 164 to 168 | The manual source is not in the repository as text | To be rebuilt from the shipped interface (build plan area 17) |

## 3. If a file here is needed again

Copy it to a living location, update it, and add a row to the [decision register](../adr/README.md) or the reconciliation
log saying what changed and why. Do not edit the archived copy.

## 4. The clean-up that needed code changes

Done in Windows phase W1 (2026-10-16): the digests, the agent brief, the first security review and engineering documents
28, 29 and 31 moved into this folder in one commit, with the writing-check exempt list, the two Rust test comments,
`DEVELOPMENT.md`, CONTRIBUTING.md and the ADR citations updated together.
