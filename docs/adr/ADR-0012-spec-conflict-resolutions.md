# ADR-0012: Resolutions of conflicts between specifications

- **Status:** Accepted. Items 1–4 are implemented in the foundation. Items 5–9 are binding on the modules in
  development.
- **Date:** 2026-09-30
- **Related:** contradiction sections of `docs/spec-digest/fsd-part1.md`, `fsd-part2.md`, `prd.md` (§6.1, §7),
  `ux.md` (§8), `security.md` (§6.1); ADR-0009, ADR-0011

## Context

The authoritative specifications contradict each other in places, or are silent. The Engineering Index requires that
such points are not decided silently in code. Where a higher-precedence spec speaks clearly, it wins
(PRD > FSD > UX > Domain > cross-cutting). This ADR records each resolution and where it lives in code. Items that
need a *product* decision (not an engineering one) are listed at the end and are **not** decided here.

## Decisions

### 1. Project status vs "Archived"

**Conflict:** FSD §4.3 lists "Archived" as a status label. FSD §4.5/§4.6 define archive and restore as a separate
reversible action that hides the project from recent views. FSD Part 2 §72 has another lifecycle
(`Active/Archived/Closed`), the initial status is unspecified, and the PRD has a third vocabulary (C-27).

**Resolution:** *Status* is a manual, user-selected phase label: `Idea` (initial), `Development`, `Writing`,
`Rewrite`, `Shooting Draft`, `Pre-Production`, `Shoot Preparation`, `Shooting` (`ProjectStatus` enum). OpenFrame
never changes it automatically (`status_user_selected`). *Archived* is **not a status**. It is the separate
`project.archived` flag, mirrored in `sys_recent_project.archived` for Home, set by `project.set_archived`. Archiving
keeps the status, so restoring from the archive returns the project exactly as it was (FSD §4.6 "contents
unaltered"). `Active/Closed` from §72 are not modelled: "active" = not archived, and "closed" is not a persistent
state.

### 2. Breakdown vs catalog category names and order

**Conflict:** FSD §26.3 lists "Cast, Extras / Background, Location / Set, Props, Wardrobe, Vehicles, Hair / Makeup,
Special Effects, VFX, Sound, Animals". §28.2 (catalog) uses "Locations", "Makeup/Hair" and "Extras" last.

**Resolution:** one canonical enum, `BreakdownCategory`, with the §26.3 labels **and order** as the stored values.
Catalog views map each value 1:1 to the §28.2 display label and order. The mapping lives in the UI and is never
stored. A catalog item and its breakdown elements always share the same canonical category value.

### 3. "Episodic" vs "Series" project types

**Conflict:** FSD §4.1 offers both, while §25.1 defines only "Series project → Seasons → Episodes".

**Resolution:** both remain selectable project types, since the FSD lists both and removing one would change product
behavior. Both enable the season/episode structure (`ProjectType::is_episodic`). No other behavioral difference
exists in v1. If the product later distinguishes them (e.g. Episodic = episodes without seasons), that requires a
spec change.

### 4. "Contributor" → Editor; professional labels are not roles

**Conflict:** AI spec §19.3 and PRD §4 use "Contributor", "Writer", "Director", "Reviewer" as if they were roles
(PRD C-7).

**Resolution:** the Security spec's Terminology Lock wins. The only roles are Owner, Editor, Commenter, Viewer and
Export-only (`Role`). "Contributor" is implemented as **Editor**. Professional titles are free text in
`project_member.professional_label` and never grant permissions. "Invite reviewer" means *Export Review Package*
(PRD C-8).

### 5. Local AI supersedes passages about external AI

**Conflict:** PRD L296 ("configured AI provider"), L2752 ("External AI providers remain optional"), AC §210.17
("External AI usage is clearly disclosed…") and AC §210.18 ("Local AI **may** operate without internet") assume cloud
providers. The approved build decision is local AI only.

**Resolution:** v1 ships **no external AI path**: no provider setting, no API key field and no code that sends
project content to an AI service. AC §210.17 is verified as "no external transmission path exists". This is a
planned AI-module test: the orchestrator's only outbound HTTP use is the model downloader, and inference traffic goes
to 127.0.0.1 only. AC §210.18 is strengthened to "local AI
**must** work without internet once installed". The general egress-disclosure rule (PRD L298) still applies to every
outbound network use: model download, update check, and opening a web link. The UI labels AI as local/on-device.
See ADR-0006.

### 6. Global search shortcut: Ctrl+K (mock) vs Ctrl/Cmd+F (spec)

**Conflict:** the mock-ups show `Ctrl+K` for global search. The UX spec defines `Ctrl/Cmd+F` as contextual search.

**Resolution:** both exist with distinct meanings. **Ctrl+K** opens the global search overlay (implemented in
`Shell.tsx`, placeholder "Search this project…"). **Ctrl+F** is *contextual find inside the current workspace*
(screenplay find bar with Enter / Shift+Enter / Esc, the Vault search box, and so on), implemented by each workspace.
Ctrl+F never opens global search, and Ctrl+K never searches only the current view.

### 7. No dark theme in the specification

**Gap:** the UX spec and mock-ups define only a light theme and no high-contrast mode (UX digest §8, gaps 2 and 6).

**Resolution:** v1 ships the **light theme only**, exactly as mocked. All colours are CSS tokens
(`styles/openframe.css`), so a theme can be added later without refactoring, but adding one requires a UX spec change
and a new ADR. Windows **High Contrast / forced-colors** mode must remain usable, meaning text, focus rings and
status never rely on colour alone. This is a QA check (22-qa-test-strategy.md), not a separate theme.

### 8. Left navigation: eight top-level items

**Conflict:** PRD §7.2 has 8 top-level items. PRD §222 says users experience only Ideas / Story / Script /
Production, with Breakdown and Call Sheets inside Production (C-9).

**Resolution:** the FSD (behavioral authority for observable UI) §3.1 lists the default left navigation exactly:
Home, Idea Vault, Story, Screenplay, Breakdown, Production, Call Sheets, Files (`app/routes.tsx`). The PRD §222
grouping is treated as a conceptual model, not a navigation layout.

### 9. "Story Board" vs "Storyboard"

**Clarification (PRD N-11):** "Story Board" (two words) is the outline card wall (acts, sequences, beats, scene cards,
tables `story_*`). "Storyboard" (one word) is the visual shot-panel feature (visual module). They are distinct
entities, tables and labels, and code must never alias them.

## Open product decisions (flagged, not decided)

These need the product owner. The code must not assume an answer:

- Post-lock scene numbering (frozen 12A / "OMITTED" vs always-derived; PRD N-1). See ADR-0011.
- Whether Build Screenplay blocks on missing scene headings (PRD C-1).
- Multiple screenplays per project (PRD C-28, FSD Part 1 #9).
- Call sheet status vocabulary and the "needs refresh" trigger (FSD Part 2 #3, Part 1 #16).
- The "Recently Deleted" retention window and any auto-purge (Security §26 item 7). There is **no auto-purge**
  today, and items stay until purged by the Owner.
- Non-Latin script support in PDF export (PRD C-25). See ADR-0008.

## Consequences

- Each resolution is cited in code comments (`enums.rs` references this ADR), so the choice is traceable in review.
- If a spec revision resolves any item differently, the spec wins. Amend this ADR and the code together.
