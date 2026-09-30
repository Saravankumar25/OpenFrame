# ADR-0007: LAN collaboration removed; collaboration is file-based through packages

- **Status:** Accepted: **product-owner decision, 2026-09-30.** Supersedes the earlier engineering plan
  (host-authoritative LAN sessions with soft locks and optimistic revisions).
- **Date:** 2026-09-30
- **Related:** Engineering Index locked decisions (LAN collaboration, concurrent collaboration); PRD-COL-005/006;
  FSD §48–49, §119–120; Offline/Collaboration spec (LAN sections); UX mock-ups 164–168; ADR-0003, ADR-0009

## Context

The PRD tiered basic LAN collaboration as P0 (PRD-COL-005) and advanced conflict handling as P1 (PRD-COL-006). The
Engineering Index had already deferred it ("architecture-ready now; ship after stable solo core") with
"soft locks + optimistic version/conflict handling; no CRDT". The feature would have required a network listener,
discovery, pairing and authentication, transport encryption, presence, soft-lock leases and a conflict-review UI.
The specifications left most of these undefined (Collaboration digest §30), and each one adds attack surface to an
otherwise offline, listener-free application.

## Decision

**Real-time local-network collaboration is removed from OpenFrame Studio entirely**, by product-owner decision
(rationale: scope reduction). OpenFrame v1 has **no** host or join sessions, network discovery, presence, soft locks,
WebSocket or other network server, or live co-editing.

**Remote collaboration is file-based only**, through **Exchange, Review and Response packages** (export package →
send by any channel the users choose → recipient imports via validate → preview → Change Set → apply). The five
security roles (Owner, Editor, Commenter, Viewer, Export-only) remain, and govern what package, review and export
workflows permit (ADR-0009).

**Superseded specification content** (no longer implemented, tested or shown):

- FSD §48–49 (LAN collaboration and its conflict rules) and §119–120;
- the LAN sections of the Offline/Collaboration specification (sessions, discovery, participants, soft locks,
  conflict state machine for live editing);
- PRD-COL-005 and PRD-COL-006;
- UX mock-ups 164–168 (LAN session screens);
- Engineering Index locked decisions "LAN collaboration" and "Concurrent collaboration: soft locks + optimistic
  version/conflict handling", and document 18 (LAN Networking & Conflict Protocol), which will not be written.

**Engineering consequences:**

- There will be no `openframe-collaboration` crate. Package exchange lives in the application's package/import-export
  modules.
- The application never opens a listening socket. Outbound network use is limited to explicit, user-approved
  downloads (local AI model, updates) and to handing web links to the browser.
- Invariant 9 ("LAN participants never open the host SQLite file over a network share") has no LAN participants left
  to govern. Its storage half still holds: projects on network shares (UNC paths) are refused (ADR-0003). The only
  supported ways to share are copying the project or exchanging packages.

## Consequences

- Foundation remnants that now have no use: `Capability::{HostSession, JoinSession}`, `ActorOrigin::Lan`, the `lan`
  value of `sys_activity.origin`, and the collaboration migration slot's "LAN sessions" wording. They are harmless
  (nothing grants or uses them) but should be removed by a foundation change. **User-facing copy must also change:**
  the `project_format.network_location` message in `modules/project.rs` still suggests "use a local-network
  collaboration session". It should instead point to copying the project locally or using an exchange package.
- Optimistic `rev` checks (`rows::update_fields(…, expected_rev)` → `conflict.stale`) and the undo stale check
  (ADR-0005) remain useful. They protect against stale UI buffers, stale AI Change Sets and stale package responses.
- Teams working at the same time exchange packages and resolve differences in the import preview. There is no live
  co-editing. This is the intended trade-off.
- Security scope shrinks: no network-listener, pairing or LAN-authorization threats (20-security-threat-model.md).

## Alternatives rejected

| Alternative | Reason |
|---|---|
| Keep LAN as "architecture-ready", ship later | Product owner removed the scope. Keeping dormant hooks invites partial implementations. |
| Shared project folder over SMB | Unsafe for SQLite WAL and has no per-user authorization. Remains refused. |
| Cloud relay / sync service | Violates the no-cloud and no-account decisions |
| CRDT document model | Out of scope, and it would introduce a second source of truth |
