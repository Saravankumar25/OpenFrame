# Product and engineering specifications

> **Current decisions (2026-10-15).** The AI, runtime, ESD, Engineering Index and Security specifications carry a "Current decisions" note under their title that lists the ADRs overriding them ([ADR-0026](../adr/ADR-0026-offline-ai-download-after-install.md) to [ADR-0030](../adr/ADR-0030-verification-ladder-and-release-flow.md)). The [decision register](../adr/README.md) is the authority.

> Moved here from the repository root on 2026-10-05; file names are unchanged and nothing was edited except status
> banners. Precedence is in [docs/README.md](../README.md) §1. Decisions that replaced older text are in the
> [decision register](../adr/README.md).

| File | Role | Status |
|---|---|---|
| `OpenFrame_Studio_Mega_PRD_Aligned_Updated.md` | Product requirements (what and why) | Active. LAN requirements superseded (2 sections marked); amended by Revision 1 |
| `OpenFrame_Studio_Functional_Specification_Document_Aligned_Updated.md` | Feature behaviour and acceptance criteria | Active. LAN sections §48 to 49, 62.6, 119 to 120, 152 superseded (13 marked); amended by Revision 1 |
| `OpenFrame_Studio_UX_UI_Specification_Updated.md` | Screens and interaction | Active. Screens 66 to 68 (LAN) superseded; light-only rule under amendment (ADR-0014) |
| `OpenFrame_Studio_Domain_Data_Specification_Updated.md` | Objects, identity, lifecycle | Active. "Collaboration Session" superseded; amended by Revision 1 |
| `OpenFrame_Studio_Screenplay_and_Production_Depth_Specification.md` | **Revision 1**: screenplay depth, indie production depth, themes (§47A) | Active, newest. Wins over the four above for the areas it covers. Three points await ADR-0014, 0015, 0016 |
| `OpenFrame_Studio_Offline_Collaboration_Specification.md` | Solo, offline and package workflows | **Superseded in part**: live LAN parts (16 sections marked). Packages remain normative |
| `OpenFrame_Studio_Import_Export_Specification.md` | Formats and import and export rules | Active; amended by Revision 1 §§36 to 43; PDF scripts await ADR-0016 |
| `OpenFrame_Studio_Security_Privacy_Data_Ownership_Specification.md` | Permissions, privacy, data ownership | Active. LAN sections superseded (22 marked); telemetry wording awaits ADR-0021 |
| `OpenFrame_Studio_AI_Specification_Updated.md` | AI product, safety, Change Sets | Active; implemented by ADR-0013 |
| `OpenFrame_Local_AI_Runtime_Model_Management_Specification.md` | Offline AI runtime, one profile, manifest, download | Active (single Gemma profile) |
| `openframe_ai_agentic_rag_spec_prompt.md` | Agentic AI and hybrid RAG behaviour (originally written as an implementation prompt) | Active, implemented; code and docs cite its section numbers |
| `OpenFrame_ESD_Technical_Architecture.md` | Engineering system design | Active, with a status note; target additions in `docs/engineering/29` |
| `OpenFrame_Studio_Engineering_Package_Index.md` | Locked engineering decisions, phases, invariants | Active; some locked decisions marked under amendment (see the register) |

The user manual is in [../manual/](../manual) (`.docx` and `.pdf`); it still contains the removed LAN figures 164 to 168
(regeneration is feature DOC-007; build plan item: User Manual regenerated against the shipped UI). Historical derived digests are archived in ../archive/2026-09-spec-digest/.

**Reading rule:** a section carrying "SUPERSEDED by ADR-0007" is history. Do not implement or test it. Where the same
topic appears elsewhere, the register in [../adr/README.md](../adr/README.md) names the current rule.
