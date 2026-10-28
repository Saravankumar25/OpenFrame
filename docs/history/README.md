# History

Created 2026-10-27, when the owner asked for old documentation to be kept as history instead of sitting next to the
current documents. Everything here is a record of the past. Nothing here is a current instruction: never implement from
it, and never edit a file here except to fix this index. A current document that disagrees with a file here wins.

| Folder | What it holds | Moved from |
|---|---|---|
| [evidence/](evidence) | Evidence of the first plan: milestone folders `M0` to `M14` (old item codes) and the sessions of 2026-10-06 and 2026-10-08 | `docs/release/evidence/` |
| [audits/](audits) | The audits of 2026-10-02 and 2026-10-03 | `docs/audits/` |
| [reports/](reports) | The project retrospection and audit of 2026-10-09 | `docs/reports/` |
| [adr/](adr) | ADR-0025 (offline AI bundled in every package), superseded on 2026-10-15 by ADR-0026; still listed in the [decision register](../adr/README.md) | `docs/adr/` |
| [logs/](logs) | Progress log entries and reconciliation records up to 2026-10-09, including the "Old code to new name" tables | `docs/implementation/` |

Single files, moved on 2026-10-27 in the documentation sync with Windows `114dff50` (each starts with one added
"Superseded by" line, copied from the Windows tree; nothing else changed):

| File | What it is | Moved from | Replaced by |
|---|---|---|---|
| [endgame-plan.md](endgame-plan.md) | The Windows W9 endgame session plan of 2026-10-17 | `docs/implementation/` | [windows-internal-release-gate.md](../release/windows-internal-release-gate.md) |
| [final-polish-plan.md](final-polish-plan.md) | The Windows final polish plan of 2026-10-19 | `docs/implementation/` | [ux/change-log.md](../ux/change-log.md) |
| [2026-10-26-ai-answer-paths-five-lanes.md](2026-10-26-ai-answer-paths-five-lanes.md) | How a question ran through the five AI lanes, 2026-10-20 to 2026-10-26 | `docs/engineering/ai-answer-paths.md` | [ai-answer-paths.md](../engineering/ai-answer-paths.md), [ADR-0037](../adr/ADR-0037-conversational-assistant-focus-and-dossiers.md) |
| [2026-10-24-ai-harness-rebuild-plan.md](2026-10-24-ai-harness-rebuild-plan.md) | The AI harness rebuild plan of 2026-10-24 (five lanes, briefings, code verifiers); ADR-0035 cites it | Windows `docs/specs/ai-harness-rebuild.md` (never in the Mac tree) | [ai-conversational-harness.md](../specs/ai-conversational-harness.md), ADR-0037 |
| [specs/screenplay-formatting-standard-v4.md](specs/screenplay-formatting-standard-v4.md) | The screenplay formatting standard, version 4; the register and the specifications index cite it | Windows `docs/history/specs/` (never in the Mac tree) | [screenplay-formatting-standard-v5.md](../specs/screenplay-formatting-standard-v5.md) |

Moved on 2026-10-28 (owner order: older decisions and finished documents go to history; the current folders hold only
documents about the current app). Each Markdown file starts with one added "Moved to history" line; the evidence folder
was moved unchanged:

| File or folder | What it is | Moved from | Why |
|---|---|---|---|
| [next-session-prompt-machtml.md](next-session-prompt-machtml.md) | The session prompt for the MacHTML recapture | `docs/implementation/` | Finished: MacHTML was recaptured on 2026-10-28 |
| [mac-port-list-2026-10-24.md](mac-port-list-2026-10-24.md) | The list of Windows changes of 2026-10-24 to port to the Mac | `docs/roadmap/` | Finished: ported in the Mac sync of 2026-10-27 |
| [mac-ai-harness-port-orders.md](mac-ai-harness-port-orders.md) | The session orders to port the conversational AI to the Mac | `docs/roadmap/` | Finished: ported in the Mac sync of 2026-10-27 |
| [evidence/ai-harness-2026-10-24/](evidence/ai-harness-2026-10-24) | Baseline and final runs of the five-lane AI harness (ADR-0035), measured on Windows | `docs/release/evidence/` | Superseded: the five lanes were retired for the conversational path of ADR-0037, measured in `release/evidence/ai-conversational-after-s8-2026-10-26/` |

The Windows tree also keeps the old `docs/archive/` files in its `docs/history/`. The Mac removed that folder on
2026-10-19 (git history keeps it), so those files are not copied here.

Evidence of the current Windows and Mac tracks (`w2` to `w9`, `mac`, `release`) stays in
[release/evidence](../release/evidence): the feature status check requires proof files there and each phase adds new
ones (the one exception is the five-lane harness folder above). Files were moved, not edited, so links inside them may point at the old places; the link check skips this folder.

What goes here from now on: a document or ADR that is superseded as a whole. A document that is superseded only in part
keeps its place, with the old part marked SUPERSEDED in place.
