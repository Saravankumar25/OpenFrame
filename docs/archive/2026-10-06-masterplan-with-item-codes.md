# OpenFrame Studio: Masterplan to Microsoft Store

> **Status:** Draft v2 (restructured 2026-10-05: waves, security standard, M0.28; 10 boxes added 2026-10-05 from the M0.28 security baseline: M1.21 to M1.28, M11.15, M11.16). 234 boxes. Baseline: repository `master` at `cd60f08`
> (identical tree to GitHub `main` at `d30b588`).
> **Purpose:** one checklist. When every box on the Store path is checked with evidence and every gate in §9 is
> signed, OpenFrame Studio is ready to be submitted to the Microsoft Store and to pass certification.
> Companion files: [01-feature-ledger.md](01-feature-ledger.md) (per-feature state),
> [02-owner-decisions-and-manual-work.md](02-owner-decisions-and-manual-work.md) (D and X items),
> [03-reconciliation-log.md](03-reconciliation-log.md), [04-progress-log.md](04-progress-log.md) (session dashboard and log), [../release/launch-readiness-register.md](../release/launch-readiness-register.md)
> (operations and commercial requirements, IDs such as `STORE-001`).

## 0. Definition of "ready for the Microsoft Store"

OpenFrame Studio is Store-ready only when **all** of this is true:

1. Every feature promised in the Store listing is at state **S5** in the ledger, and nothing S5-incomplete is listed.
2. Every **P0** ledger row is S5. P1 rows are S5 or consciously deferred by the owner in writing (D2) and removed from the listing.
3. Every gate G0 to G13 in §9 is PASS with archived evidence.
4. The exact MSIX that will be uploaded has passed the Windows App Certification Kit and a clean-machine install,
   first-launch, upgrade, uninstall and reinstall run.
5. No open P0 defect, no data-loss, security, migration or recovery defect, no silent import/export loss, no AI
   automatic write path, no unauthorized AI data exposure.
6. Privacy policy, support contact and listing are live, truthful and consistent with the build.
7. The product owner has signed GO in §9.

The software cannot be proven free of every bug. This definition is the standard the plan can honestly meet.

## 1. How to use this file

- **Check a box only with evidence.** Write the evidence after the item: `(ev: path, PR link, test name, or report)`.
  Store large evidence under `docs/release/evidence/<milestone>/`.
- Wave tags **[W1]** to **[W5]**, **[WO]** (owner track): see section 1b. Tags: **[CODE]** engineers or agents; **[OWNER]** only the owner can do it (see doc 02); **[BOTH]**; **[DOC]**
  documentation; **[IF D1=C|D]** only if the business model needs it, otherwise mark N/A with the reason.
- Ledger IDs in brackets (`[SCRIPT-009]`) mean the item also moves that ledger row up one or more states. Update
  the ledger in the same pull request.
- **Every code item has this definition of done**, not repeated below: reachable from the UI; saves; undoes;
  survives restart and crash; exports and re-imports where applicable; permissions enforced in Rust; search
  document registered; AI decision recorded (metadata, tool, retrieval projection, eval case, regenerated
  `docs/engineering/ai-tool-coverage.md`); accessible by keyboard; within performance budget (doc 21); docs and
  ledger updated; `npm run verify` green.
- **Slice small.** One vertical slice per pull request (schema, operation, UI, tests). No unrelated refactors.
  Migrations are linear and numbered on `master` only. No architecture bypass (CONTRIBUTING §2, doc 31).
- **Docs must not drift.** A merged change that alters behaviour updates, in the same PR: the ledger row, this
  file's box, the affected spec or engineering doc, and the user manual source if user-visible (DOC-009).
- A box that becomes irrelevant is marked `N/A: reason`, never deleted.

### 1a. Windows security standard (permanent, applies to every box)

OpenFrame must meet the security bar of well-regarded professional Windows desktop applications, on every change and in every release.

- **Least privilege:** standard user, no elevation; Store capabilities limited to runFullTrust and internetClient, justified in writing.
- **Binary hardening:** verify ASLR, DEP, control-flow guard and safe linker flags in release builds; no secrets or debug data in artifacts; no unsafe Rust without a written reason.
- **WebView and IPC:** strict CSP; no webview filesystem, shell or HTTP; one allow-listed IPC entry point; all paths resolved and confined in Rust; all untrusted input validated in Rust.
- **Process safety:** AI sidecar on 127.0.0.1 with a random port and per-launch key, never run from a project path, killed with the app (Job Object), orphans swept at startup.
- **Data and files:** bounded parsers, no zip-slip or symlink escape, UNC and network paths refused, atomic writes, backup before migration, no project content in logs.
- **Supply chain:** cargo deny, cargo audit, npm audit, licence inventory and SBOM pass without bypass; production keys never in source; the development key is never trusted.
- **Distribution:** Store MSIX that passes WACK; signed manifests and hashes for downloads; approval-based updates only (direct channel); no silent network use; no telemetry.
- **Review:** run security-guidance and ecc:security-review on any change touching parsers, paths, packages, AI, process control, settings or imports.

Evidence for this standard: box M0.28 (baseline), M10.06 (second review), gates G10 and G11.

## 1b. Waves (execution slices)

Every box carries a wave tag. Waves are the order of execution; milestone gates (G0 to G13) are unchanged. A wave may span several milestones.

| Wave | Purpose | Boxes | Share of 234 | Starts when |
|---|---|---|---|---|
| **W1** | Governance controls, safety net and every feature that needs no owner decision and no unfinished engine: M0 controls, all of M1, M2 progression and find scope, M3 writer comfort, M4 Story Board, M8 no-listener test, M9 AI evaluation and hardening, M10 Project Home and localization check | **64** | **27.4%** | now (56 original boxes = 25% of the original 224, plus 8 security boxes added from M0.28) |
| WO | Owner track: M0 documentation review, decisions D1 to D18, manual tasks X1 to X5, Store MinVersion | 18 | 7.7% | now, in parallel; gates W2 to W4 |
| W2 | Screenplay engine and interchange, themes and Appearance, Breakdown and production, packages and backup | 46 | 19.7% | W1 M0 and M1 done; ADR-0014, 0015, 0016 decided |
| W3 | Schedule and DOOD, call sheets, budget, templates, exports, AI hardware and Store runtime, legal and website | 39 | 16.7% | W2 M2 and M5 done; owner items X6 to X10 |
| W4 | Product-wide quality and beta, Store build and WACK, submission gate, certification and launch | 47 | 20.1% | W2 and W3 done |
| W5 | First 90 days, macOS and direct channel | 13 | 5.6% | after the Store listing is live |
| Stop-ship | The seven stop-ship conditions in section 10 | 7 | 3.0% | checked at M13 |

W1 items that wait on an owner input are marked on the box (M9.13 support address, M9.01 real run needs `.dev-models`). ADR-0019 and ADR-0021 were accepted 2026-10-05, which unblocked M1.01, M1.02 and M1.19.

## 2. Map

```text
M0 Decisions, baseline, governance
 │
 ├──► M1 Safety net and verification harnesses ─────────────┐
 │                                                          │
 ├──► M2 Screenplay trust engine ──► M5 Breakdown/production ──► M6 Schedule + DOOD ──► M7 Call sheets,
 │        │                                                                  budget, templates, exports
 │        └──► M8 Packages and backup (after M2 formats; parallel with M5 to M7)
 │
 ├──► M3 Writer comfort + Appearance (parallel; tokenization baseline first)
 ├──► M4 Story Board depth (parallel with M2)
 ├──► M9 AI completion (eval corpus starts at M0; per-feature AI work continues in every milestone)
 ├──► M12 Legal, website, support (long-lead manual work starts at M0)
 │
 └────────────► M10 Product-wide quality and beta ──► M11 Store build and certification pre-flight
                                                            ──► M13 Submission gate (GO/NO-GO)
                                                            ──► M14 Certification, publish, launch
                                                            ──► M15 First 90 days
                                                            (M16 macOS and direct channel: after the Store, optional)
```

Critical path: M0, M1, M2, M5, M6, M7, M10, M11, M13, M14. Everything else runs beside it.
Can start immediately and in parallel: M0 manual work (X1 to X5), M1 harnesses, M3 tokenization baseline, M4,
M9 corpus authoring, M12 domain and policy drafting.

| Milestone | Title | Depends on | Status |
|---|---|---|---|
| M0 | Decisions, baseline, governance | none | ☐ |
| M1 | Safety net and verification harnesses | M0 | ☐ |
| M2 | Screenplay trust engine and interchange | M0, M1 harness items | ☐ |
| M3 | Writer comfort and Appearance | M0, M1 visual baseline | ☐ |
| M4 | Story Board depth | M0 | ☐ |
| M5 | Breakdown, catalog, locations, cast, crew | M2 | ☐ |
| M6 | Schedule and DOOD | M5 | ☐ |
| M7 | Call sheets, budget, templates, samples, exports, visual planning | M6 | ☐ |
| M8 | Packages and backup conformance | M2 | ☐ |
| M9 | AI completion | M0, runs with all | ☐ |
| M10 | Product-wide quality, accessibility, performance, beta, manual | M2 to M9 | ☐ |
| M11 | Windows Store build and certification pre-flight | M10 (RC from M7+) | ☐ |
| M12 | Legal, privacy, website, support | M0, owner items | ☐ |
| M13 | Submission gate | M10, M11, M12 | ☐ |
| M14 | Certification, publish, launch | M13 | ☐ |
| M15 | First 90 days | M14 | ☐ |
| M16 | macOS and direct channel (optional) | M14 | ☐ |

---

## M0. Decisions, baseline and governance

**Goal:** stop drift, decide what only the owner can decide, record the true starting point.

### Documentation (done in this integration, to be reviewed)
- [ ] M0.01 **[WO]** [DOC] Review `docs/README.md` documentation map; supplied documents promoted; stale facts fixed (DOC-001, DOC-002, DOC-004).
- [ ] M0.02 **[WO]** [DOC] Spec banners applied for LAN supersession and the Revision 1 specification (DOC-003).
- [ ] M0.03 **[WO]** [DOC] Screenplay engine contract (doc 32) and production-depth contract (doc 33) reviewed and approved by the owner (DOC-006).

### Owner decisions (doc 02)
- [ ] M0.04 **[WO]** [OWNER] D1 business model and licence decided; ADR-0018 Accepted. **Blocks M11 commerce items and M12 policies.**
- [ ] M0.05 **[WO]** [OWNER] D2 Store v1 scope confirmed (full scope or named exclusions).
- [ ] M0.06 **[WO]** [OWNER] D3 Windows versions and D4 macOS timing decided; ADR-0017 Accepted.
- [ ] M0.07 **[WO]** [OWNER] D5 diagnostics policy decided; ADR-0021 Accepted.
- [ ] M0.08 **[WO]** [OWNER] D6, D7, D8 decided; ADR-0014, ADR-0015, ADR-0016 Accepted, amended or rejected (and ADR-0008, ADR-0011, ADR-0012 pointers added).
- [ ] M0.09 **[WO]** [OWNER] D9 and D10 decided; ADR-0019, ADR-0020 Accepted.
- [ ] M0.10 **[WO]** [OWNER] D11 to D18 answered or explicitly deferred with a date.
- [ ] M0.11 **[WO]** [OWNER] Open product questions from ADR-0012 answered (Build Screenplay blocking, multiple screenplays, call sheet status vocabulary, Recently Deleted retention).
- [ ] M0.12 **[WO]** [OWNER] D16 target launch date and capacity supplied; dates added to §2.

### Manual long-lead work (start now)
- [ ] M0.13 **[WO]** [OWNER] X1 legal entity confirmed.
- [ ] M0.14 **[WO]** [OWNER] X2 Partner Center account verified, second administrator added (`STORE-001`).
- [ ] M0.15 **[WO]** [OWNER] X3 name "OpenFrame Studio" reserved; identity values copied into `packaging/msix/store.config.json` (REL-002).
- [ ] M0.16 **[WO]** [OWNER] X4 licence added; `LICENSE-PENDING.md` deleted; manifests updated (REL-001).
- [ ] M0.17 **[WO]** [OWNER] X5 trademark, domain, handles, logo and font-licence clearance recorded (`LEGAL-005`).

### Baseline and controls
- [x] M0.18 **[W1]** [CODE] Tag `baseline/pre-masterplan-2026-10`. Record in `docs/release/evidence/M0/`: `npm run verify`, `cargo test --workspace`, `npm run perf`, `npm run test:e2e`, `scripts/ai-benchmark.mjs` if models are present, migration versions, `ai-tool-coverage.md`, sample project database. (ev: tag baseline/pre-masterplan-2026-10; docs/release/evidence/M0/INDEX.md#M0-04 to #M0-09, #M0-25 (E2E 17/17), #M0-26 (perf 39/39); AI benchmark N/A without models)
- [x] M0.19 **[W1]** [CODE] Replace every **[R]/[A]/[S]** tag in the ledger with a link to run evidence or a corrected state. (ev: docs/release/evidence/M0/INDEX.md#M0-14; ledger UI route and Tests columns, 65 states corrected)
- [x] M0.20 **[W1]** [CODE] Capture the **visual baseline** of 15 representative screens in the current appearance (before any tokenization) [THEME-002]. (ev: docs/release/evidence/M0/M0.20-visual-baseline/ 15 screens; INDEX.md#M0-27; spec tests/e2e/specs/90-visual-baseline.test.mjs)
- [x] M0.21 **[W1]** [CODE] Fix `cargo deny` and keep `cargo deny`, `npm audit --omit=dev`, licence inventory green and **blocking** [PLAT-003]. (ev: docs/release/evidence/M0/INDEX.md#M0-15, #M0-16; first CI run on the next push)
- [x] M0.22 **[W1]** [CODE] Ledger check script in CI: fails if an in-scope row lacks source, milestone, UI route, tests or state (doc 31 §29). (ev: scripts/check-ledger.mjs; INDEX.md#M0-17, #M0-18; CI job `governance`)
- [x] M0.23 **[W1]** [CODE] Architecture lint in CI: no forbidden React imports (fs, sqlite, shell, HTTP); no SQLite writes outside persistence/Store/approved derived index; no unregistered operation; no binding drift; AI module cannot call `ai.*` apply [PLAT-004]. (ev: scripts/architecture-lint.mjs rules W1-W4, C1-C5, R1-R5; INDEX.md#M0-17, #M0-18; CI job `governance`)
- [x] M0.24 **[W1]** [CODE] Migration policy written in doc 23: files 0001 to 0010 frozen at the first external build; additive changes only afterwards. (ev: doc 23 §4 Enforcement; scripts/check-migrations.mjs with --base; migrations/frozen.json; release gate; INDEX.md#M0-19)
- [x] M0.25 **[W1]** [CODE] Evidence index template and folder structure created under `docs/release/evidence/`. (ev: docs/release/evidence/README.md; INDEX.md#M0-20)
- [x] M0.26 **[W1]** [CODE] Pull request template contains the feature contract, V0 to V8 risk level and the checklist from doc 31 §25 and CONTRIBUTING. (ev: .github/pull_request_template.md; INDEX.md#M0-21)
- [ ] M0.27 **[WO]** [CODE] Store manifest `MinVersion` and listing OS text changed to match D3 [PLAT-001].
- [x] M0.28 **[W1]** [CODE] Windows security baseline audit against the standard in section 1a: compare the repo with docs/engineering/20, SECURITY_REVIEW_2026-09-30 and doc 28; report gaps with file evidence and propose a masterplan item for each (report only; fix only trivial and safe items). (ev: docs/release/evidence/M0/M0.28-security-baseline.md, M0.28-fixes-applied.md; follow-up boxes M1.21 to M1.28, M11.15, M11.16)

**Gate G0 (all of M0 checked):** decisions recorded, baseline evidence committed, supply chain green, controls in CI.

---

## M1. Safety net and verification harnesses

**Goal:** build the tests that make later feature work safe. Do this before the feature rush.

### Data safety
- [x] M1.01 **[W1]** [CODE] Crash-safe Change Set apply: journal migration, recovery on project open, compensation default [AI-005, ADR-0019]. ADR-0019 Accepted 2026-10-05. (ev: migration 0011; change_set.rs journal and recovery; docs/release/evidence/M1/INDEX.md#M1-01, #M1-03)
- [x] M1.02 **[W1]** [CODE] Forced-kill test after **every** apply operation boundary; project reopens consistent. ADR-0019 Accepted 2026-10-05. (ev: tests/ai_apply_recovery.rs::adr_0019_stop_after_every_apply_boundary_reopens_consistent and 7 more; INDEX.md#M1-02)
- [x] M1.03 **[W1]** [CODE] Forced-kill during save, migration, export, package import, backup; recovery offered; no committed edit lost [CORE-003, CORE-004]. (ev: tests/forced_kill.rs, 5 scenarios x 3 kill points; docs/release/evidence/M1/INDEX.md#M1-04)
- [x] M1.04 **[W1]** [CODE] Migration fixture corpus: one fixture per shipped schema version (project 1 to 10, global 1 to 3, app 1); upgrade, backup exists, integrity check, search and AI index rebuild, exportable [CORE-005, PLAT-005]. (ev: tests/migration_fixtures.rs, project 1-11, global 1-3, app 1; INDEX.md#M1-05)
- [ ] M1.05 **[W1]** [CODE] Migration failure fixtures: interrupted, disk full, permission denied, newest-version refusal, corrupt database untouched.
- [ ] M1.06 **[W1]** [CODE] Property tests: ordering after repeated moves; undo and redo identity; delete and restore; import, export, import preserves required semantics.
- [ ] M1.07 **[W1]** [CODE] Stale-proposal tests per domain: retrieve, propose, user edits, apply refuses or rechecks (screenplay, story, breakdown, schedule, call sheet, cast and crew, budget, notes, visual).

### Concurrency and runtime
- [ ] M1.08 **[W1]** [CODE] Soak suite: concurrent IPC calls, rapid autosave, undo during indexing, close and reopen during indexing, repeated AI cancel, sidecar crash and restart loop, hours of edits and thousands of mutations.
- [ ] M1.09 **[W1]** [CODE] Intelligence index shutdown stress: repeated open, close, rebuild while the embedding runtime is stalled.
- [ ] M1.10 **[W1]** [CODE] Sidecar start retries on loopback bind failure (free-port race) with a test.
- [ ] M1.11 **[W1]** [CODE] Failure injection (storage and process): disk full, permission denied, path disappears, external volume removed, read-only destination, sleep and resume during download and indexing, antivirus-locked file, very long paths, non-ASCII project paths.

### Test infrastructure
- [ ] M1.12 **[W1]** [CODE] Desktop E2E runs in CI on a Windows runner and is a **release gate** [PLAT-002].
- [ ] M1.13 **[W1]** [CODE] E2E suite expanded to one spec per workspace and the full lifecycle journey (create project, story, scene, screenplay, breakdown, schedule, call sheet, save, close, relaunch, reopen, edit, undo, redo, export, re-import).
- [ ] M1.14 **[W1]** [CODE] Reachability test generated from the ledger: every workspace, sub-tab and critical command has a UI route.
- [ ] M1.15 **[W1]** [CODE] Hostile-input corpus (normal, malformed, huge, nested, zip-slip, traversal, entity expansion, decompression bomb, symlink, case collision, Unicode path) kept permanently for FDX, Fountain, DOCX, PDF, ZIP, packages; fuzz harness for each parser.
- [ ] M1.16 **[W1]** [CODE] Golden PDF image-compare harness (page count, margins, headings, dialogue, title page, numbers, continuation, revision marks, headers, no clipping) [XFER-010].
- [ ] M1.17 **[W1]** [CODE] UI visual-regression harness using the M0.20 baseline.
- [ ] M1.18 **[W1]** [CODE] Log redaction test (no screenplay text, notes, contacts, prompts, paths, keys) [CORE-014]. Amended by M0.28 (P13): release builds cap `OPENFRAME_LOG` at `info` for non-OpenFrame targets, the project lock path log is removed or redacted, and the test also covers the sidecar log.
- [ ] M1.19 **[W1]** [CODE] **Export diagnostics** bundle: user-initiated, inspectable, redacted [CORE-015, ADR-0021]. ADR-0021 Accepted 2026-10-05.
- [ ] M1.20 **[W1]** [CODE] Performance budgets re-run and extended with budgets for features in M2 to M7 (typing latency, reflow, breakdown filters, schedule ops, call-sheet and PDF export) (doc 21).

### Security hardening from the M0.28 baseline (`docs/release/evidence/M0/M0.28-security-baseline.md`)
- [ ] M1.21 **[W1]** [CODE] Asset-protocol scope: forbid project paths again on project close or switch, and an adapter-level scope test (doc 20 open item 1). The `cache/` exposure itself was removed on 2026-10-05 and is guarded by lint rule C5 (P1).
- [ ] M1.22 **[W1]** [CODE] `confine` and every write destination resolve the nearest existing ancestor through junctions and symlinks and refuse anything outside the canonical root; regression test with a junctioned `assets/` folder (P2).
- [ ] M1.23 **[W1]** [CODE] Network-location policy also refuses mapped network drives (`DRIVE_REMOTE`) and canonical paths that resolve to UNC; dialog-file exception recorded in doc 20 T4a (P3).
- [ ] M1.24 **[W1]** [CODE] One shared `replace_file_atomic` service (doc 28 §14): unique temp name in the target folder, cleanup on failure, no delete before rename; migrate project-format, import-export, AI store, `archive::write_zip`, `backup_to`; covered by M1.03 forced-kill tests (P4).
- [ ] M1.25 **[W1]** [CODE] Integrity check and verified safety backup before migrating `app.sqlite` with the shared runner; fixture in M1.04 (P5).
- [ ] M1.26 **[W1]** [CODE] Release hardening flags: `-C control-flow-guard` for `x86_64-pc-windows-msvc`, `/CETCOMPAT` evaluated, `--remap-path-prefix` for build paths (P6).
- [ ] M1.27 **[W1]** [CODE] Supply-chain pinning: release gates run `cargo audit` and `npm audit --omit=dev` as well as `cargo deny`; the Store job produces the CycloneDX SBOM; every GitHub Action pinned to a commit SHA and every installed tool to an exact version (P9).
- [ ] M1.28 **[W1]** [CODE] Put the four Tauri commands under the Tauri ACL with an explicit command list, and either fold the three helper commands into registry operations or amend §1a and doc 20 T2 to "one data entry point plus three reviewed helpers" (P11).

**Gate G1:** all M1 boxes checked; CI shows E2E, architecture lint, ledger check and supply chain as blocking.

---

## M2. Screenplay trust engine and interchange

**Goal:** a screenplay engine and FDX path that migrating writers can trust. Contract: doc 32. Spec: Revision 1 §§4 to 20, 36 to 42, 45, 50, 51.

### Contract and schema
- [ ] M2.01 **[W2]** [CODE] Additive migration (next free project number; `0011` is the M1.01 apply journal and M4.01 takes the next, doc 23 §4) per doc 32: custom elements, page-break directive, production number labels (ADR-0015), revision records and changed pages, header/footer and page setup settings. No existing migration edited after the freeze.
- [ ] M2.02 **[W2]** [CODE] Element rule profile table shared by editor, layout and FDX mapping [SCRIPT-003].
- [ ] M2.03 **[W2]** [CODE] Operations registered with metadata, permissions, undo, search documents, AI decisions for every new operation.

### Writing
- [ ] M2.04 **[W1]** [CODE] Element progression test matrix and manual override [SCRIPT-001].
- [ ] M2.05 **[W1]** [CODE] Find and replace scope: current scene or whole script, undoable [SCRIPT-004].
- [ ] M2.06 **[W2]** [CODE] Title page fields, preview, export toggle [SCRIPT-013].
- [ ] M2.07 **[W2]** [CODE] Header and footer with placeholders and first-page rule [SCRIPT-014].
- [ ] M2.08 **[W2]** [CODE] Page setup presets (Letter, A4, saved preset, font, size, line spacing), persisted [SCRIPT-015].
- [ ] M2.09 **[W2]** [CODE] Explicit page break directive: insert, remove, undo, export mapping [SCRIPT-012].
- [ ] M2.10 **[W2]** [CODE] Custom elements behind Advanced: name, shortcut, preset, next element, import/export identity [SCRIPT-016].
- [ ] M2.11 **[W2]** [CODE] Production scene numbers: imported, alphanumeric, frozen at lock, dependency warning, undo [SCRIPT-008, ADR-0015].
- [ ] M2.12 **[W2]** [CODE] Lock Shooting Draft, Start Revision, revision colour and reason, changed scenes and pages against the locked baseline [SCRIPT-007].

### Layout and output
- [ ] M2.13 **[W2]** [CODE] Deterministic pagination contract shared by editor preview, print preview and PDF; parity tests [SCRIPT-009, SCRIPT-021].
- [ ] M2.14 **[W2]** [CODE] Keep-together rules and tests [SCRIPT-010].
- [ ] M2.15 **[W2]** [CODE] Dialogue continuation generated from structure; one split block distinct from two blocks [SCRIPT-011].
- [ ] M2.16 **[W2]** [CODE] Embedded subset fonts and shaping for Telugu, Hindi, Tamil, Malayalam, Kannada, English and mixed text; deterministic golden output [SCRIPT-UNI-001, ADR-0016].
- [ ] M2.17 **[W2]** [CODE] Multilingual round trip passes: type, save, reload, PDF, FDX, Fountain, DOCX, search, AI retrieval.
- [ ] M2.18 **[W2]** [CODE] Screenplay outputs carry no OpenFrame branding (test) [XFER-009].

### Interchange
- [ ] M2.19 **[W2]** [CODE] FDX import: preview with counts and warnings; custom element preserve, map or review; scene numbers, title data, revisions; never flatten unknown constructs [XFER-001].
- [ ] M2.20 **[W2]** [CODE] FDX export structurally valid, re-importable; **reopens in Final Draft** (manual check on a licensed copy, evidence screenshots) [XFER-002].
- [ ] M2.21 **[W2]** [CODE] 13 fixture classes complete with source, parsed structure, exported structure, re-imported structure and PDF snapshot [XFER-003].
- [ ] M2.22 **[W2]** [CODE] Fountain fidelity table and round trip [XFER-004]. DOCX compatibility boundary documented and tested [XFER-005]. PDF import best effort with safe failure [XFER-006]. TXT and paste [XFER-007].
- [ ] M2.23 **[W2]** [CODE] Import report exact; export failure leaves the source untouched; partial output cleaned [XFER-008].
- [ ] M2.24 **[W2]** [CODE] Revision 1 §50 migration definition of done passes (11 steps) with fixtures.
- [ ] M2.25 **[W2]** [CODE] Screenplay workspace E2E: write, element flow, title page, revisions, lock, export PDF and FDX, import FDX, continue, restart (SCRIPT-005, 006, 022, 023).

**Gate G2:** Revision 1 §51 acceptance list all green; performance within budget; AI items complete for each new operation; ledger SCRIPT-, XFER- P0 rows at S5.

---

## M3. Writer comfort and Appearance

Spec: Revision 1 §§7, 16, 19, 20, 44, 47A; engineering: doc 30; ADR-0014.

- [ ] M3.01 **[W1]** [CODE] Smart structural suggestions: INT./EXT./I/E, characters (script, Story Board, cast), locations, DAY/NIGHT/LATER/CONTINUOUS/SAME/MOMENTS LATER; filter, Enter accepts, Esc dismisses, arrows, click, Create New; never replaces silently; no model call [SCRIPT-018].
- [ ] M3.02 **[W1]** [CODE] Custom keyboard shortcuts: remap, remove, restore one, restore all, conflict detection, multiple per command; every command keeps an on-screen route [SCRIPT-019].
- [ ] M3.03 **[W1]** [CODE] Scene navigator optional columns [SCRIPT-017].
- [ ] M3.04 **[W1]** [CODE] Proofing: project language, toggle, underline, personal dictionary, available dictionaries, primary and secondary language [SCRIPT-020].
- [ ] M3.05 **[W2]** [CODE] Tokenization pass: every colour literal classified as theme token, intentional fixed, or document output; themeable literals replaced [THEME-003].
- [ ] M3.06 **[W2]** [CODE] **OpenFrame Default renders identically** to the M0.20 baseline (visual regression) [THEME-002].
- [ ] M3.07 **[W2]** [CODE] Appearance engine: Light, Dark, System, live OS follow, no restart [THEME-001].
- [ ] M3.08 **[W2]** [CODE] App migration `0002` stores preferences and My Themes; invalid or deleted theme falls back to Default [THEME-006].
- [ ] M3.09 **[W2]** [CODE] 15 built-in themes from one registry; adding one touches no component [THEME-004].
- [ ] M3.10 **[W2]** [CODE] Theme import: file or paste, 10 KB cap, strict schema, id regex, bounded seeds, whitelisted overrides, preview, contrast check; malicious fixtures change nothing [THEME-005].
- [ ] M3.11 **[W2]** [CODE] My Themes: apply, rename, export, delete [THEME-005].
- [ ] M3.12 **[W2]** [CODE] Contrast tests (AA 4.5:1) for every theme in Light and Dark; High Contrast theme usable [THEME-007].
- [ ] M3.13 **[W2]** [CODE] Test that screenplay PDF and printed call-sheet paper never change with theme [THEME-008].
- [ ] M3.14 **[W2]** [CODE] Visual regression for 15 screens × built-in themes × Light/Dark; restart persistence test.

**Gate G3:** THEME- rows and SCRIPT-017 to 020 at S5.

---

## M4. Story Board depth

- [ ] M4.01 **[W1]** [CODE] Long scene description canonical field, search, AI read and proposal [STORY-CARD-001]. Takes the next free project migration (expected `0012`; `0011` is the ADR-0019 apply journal; doc 23 §4 allocation).
- [ ] M4.02 **[W1]** [CODE] Card Display Settings (12 toggles) stored as view config, not content; changing layout never changes scene or screenplay text (test) [STORY-CARD-002].
- [ ] M4.03 **[W1]** [CODE] Presets Minimal, Story, Production, Custom; expanded card readable [STORY-CARD-003].
- [ ] M4.04 **[W1]** [CODE] Notes and attachments capacity limits and rendering tests [STORY-CARD-004].
- [ ] M4.05 **[W1]** [CODE] Story Board links stay correct when screenplay scene identities change (lineage tests) [STORY-001 to 006].
- [ ] M4.06 **[W3]** [CODE] Build Screenplay behaviour per the owner's answer to PRD C-1 [STORY-004].
- [ ] M4.07 **[W1]** [CODE] Story Board workspace E2E (create, drag, parking lot, convert, build screenplay, episodes, restart, undo) [STORY-001 to 007].
- [ ] M4.08 **[W1]** [CODE] Idea Vault workspace E2E including voice note, drag-in, global vault, send to Story Board [VAULT-001 to 008].

**Gate G4:** STORY- and VAULT- P0 rows at S5.

---

## M5. Breakdown, catalog, locations, cast, crew

- [ ] M5.01 **[W2]** [CODE] Category colour + text + icon + tooltip model; Advanced customisation; accessibility test [BRK-003].
- [ ] M5.02 **[W2]** [CODE] Bulk accept and reject selected, quick reuse, filters (unreviewed, suggested, missing), scene completion state; batch undo [BRK-004].
- [ ] M5.03 **[W2]** [CODE] Source-phrase jump, catalog item to all scenes, concise breakdown report [BRK-005].
- [ ] M5.04 **[W2]** [CODE] Reconciliation after script change never silently deletes confirmed data (test) [BRK-002].
- [ ] M5.05 **[W2]** [CODE] Locations carry address, access, parking, nearest hospital, emergency contact, availability, notes, photos [PROD-002].
- [ ] M5.06 **[W2]** [CODE] Cast and crew carry contact, availability, notes, archive; privacy classification and permissions for contact data [PROD-003].
- [ ] M5.07 **[W2]** [CODE] Identity stable on rename; deletion and reference behaviour explicit; search covers intended fields [PROD-001 to 004].
- [ ] M5.08 **[W2]** [CODE] Large-project performance budgets for breakdown filters and open [BRK-006].
- [ ] M5.09 **[W2]** [CODE] Breakdown, catalog, locations, cast and crew workspace E2E [BRK-001, PROD-001 to 004].

**Gate G5:** BRK- and PROD- P0 rows at S5.

---

## M6. Schedule and DOOD

Contract: doc 33. Extend existing tables; do not replace them (reconciliation C11).

- [ ] M6.01 **[W3]** [CODE] Day timeline fields on `shooting_day`: crew call, shooting start, target wrap [SCHED-003].
- [ ] M6.02 **[W3]** [CODE] Scene start time and computed timeline on strips; markers (meal, travel, company move, custom) feed the timeline and day total [SCHED-004, SCHED-002].
- [ ] M6.03 **[W3]** [CODE] Availability warnings: cast, location, missing location, over target, travel pushes over target; advisory only [SCHED-005].
- [ ] M6.04 **[W3]** [CODE] Filters (location, cast, D/N, length, scheduled, breakdown complete, day, revision); filters never mutate [SCHED-006].
- [ ] M6.05 **[W3]** [CODE] Test: no optimizer, no automatic scene move, ever [SCHED-009].
- [ ] M6.06 **[W3]** [CODE] Stale and review state for source changes; call-sheet stale flag still correct [SCHED-007].
- [ ] M6.07 **[W3]** [CODE] DOOD derived from cast assignments and schedule; cast and day-range selection; manual correction model; PDF and spreadsheet export; read-only to screenplay and breakdown [DOOD-001].
- [ ] M6.08 **[W3]** [CODE] Schedule exports reproducible (PDF and spreadsheet) [SCHED-008].
- [ ] M6.09 **[W3]** [CODE] Schedule workspace E2E (create, drag, markers, off days, conflicts, grouping, stale review, export, restart, undo) [SCHED-001].

**Gate G6:** SCHED- and DOOD- P0 and P1 rows at S5.

---

## M7. Call sheets, budget, templates, samples, exports, visual planning

- [ ] M7.01 **[W3]** [CODE] Call sheet field coverage per Revision 1 §29 and §56: header, day times, weather, sunrise and sunset, locations, scene table with start times and travel rows, talent, extras, department notes, advance schedule, crew by department, communication, practical notes, attachments; unknown values show "Needs input" [CALL-002 to 005].
- [ ] M7.02 **[W3]** [CODE] Six default sections; the rest via + Add Section; empty optional sections hidden.
- [ ] M7.03 **[W3]** [CODE] Update preview lists changed scenes, cast calls and locations; finalized sheets stay immutable snapshots; statuses per owner vocabulary [CALL-001].
- [ ] M7.04 **[W3]** [CODE] Editing a call sheet never changes screenplay, breakdown or schedule (test).
- [ ] M7.05 **[W3]** [CODE] Lightweight budget Planned and Actual, category totals, difference, report; migration keeps existing amounts as Planned [BUDGET-001].
- [ ] M7.06 **[W3]** [CODE] Sides, production reports (6 types), moodboard, storyboard sheet, shot list exports [CALL-007, VIS-001 to 003].
- [ ] M7.07 **[W3]** [CODE] Subtle OpenFrame branding on every non-screenplay export; spreadsheets via metadata or footer; user files never modified (test per export) [BRAND-001].
- [ ] M7.08 **[W3]** [CODE] Call sheet, schedule, DOOD, breakdown, budget, sides PDFs have golden or image regression [CALL-006].
- [ ] M7.09 **[W3]** [OWNER] D12 template inventory decided; D13 and X13 sample scripts written or commissioned.
- [ ] M7.10 **[W3]** [CODE] Built-in templates per D12; Use creates an independent copy; later template edits never change existing documents (test) [TEMP-001, TEMP-002].
- [ ] M7.11 **[W3]** [CODE] Samples (Example Feature, Example Short, Example Scene, FDX Interchange Test) shipped as assets and used as regression fixtures; copyright note per file [TEMP-003].
- [ ] M7.12 **[W3]** [CODE] Call sheets, sides, reports, visual planning, notes and tasks workspace E2E [CALL-001, 007, NOTES-001, VIS-001 to 003].

**Gate G7:** CALL-, BUDGET-, TEMP-, BRAND-, VIS- P0 and P1 rows at S5.

---

## M8. Packages and backup conformance

- [ ] M8.01 **[W2]** [CODE] Exchange, Review and Response packages: export, open, response, version mismatch, unmatched notes, preview before apply, identity collision, role-scoped content [XFER-011].
- [ ] M8.02 **[W2]** [CODE] Backup and whole-project package: create, validate, restore on a new machine, corrupt backup detected, interrupted import [XFER-012].
- [ ] M8.03 **[W2]** [CODE] Packages carry the new data from M2 to M7 (production numbers, custom elements, schedule timeline, budget Planned/Actual) with format version bump and compatibility tests.
- [ ] M8.04 **[W1]** [CODE] No network listener anywhere (test), consistent with ADR-0007; LAN removed from user-facing text.
- [ ] M8.05 **[W2]** [CODE] Package workspace E2E and restore drill.

**Gate G8:** XFER-011 and XFER-012 at S5.

---

## M9. AI completion

- [ ] M9.01 **[W1]** [CODE] Retrieval gold set (`gold_retrieval_cases.jsonl`) across all domains; harness reports Recall@1/5/10, MRR, nDCG, grounding, leakage, stale-result rate for FTS, vector, hybrid, hybrid+graph with real BGE embeddings [AI-003]. Real-embedding runs need the dev models in .dev-models; otherwise build corpus and harness and mark the real run evidence pending.
- [ ] M9.02 **[W3]** [OWNER] Thresholds approved by the owner after the first run; simplify the graph stage if it does not beat hybrid.
- [ ] M9.03 **[W1]** [CODE] Per-feature AI evaluation cases for every milestone feature (intent, expected tool, entities, permission boundary, Change Set behaviour).
- [ ] M9.04 **[W1]** [CODE] `ai-tool-coverage.md` regenerated from the live Registry; every new operation has metadata; no unowned AI-accessible operation [AI-009].
- [ ] M9.05 **[W1]** [CODE] Product knowledge (`knowledge.md`) covers every new feature [AI-014].
- [ ] M9.06 **[W3]** [OWNER] X10 test hardware available. Benchmark and `real_runtime` on minimum (8 GB RAM) and recommended hardware, CPU path, and Vulkan on a discrete GPU; first-token latency, tool-call and JSON reliability, peak memory, disk [AI-004].
- [ ] M9.07 **[W3]** [OWNER] Minimum hardware published (replaces `[N] GB` in the listing).
- [ ] M9.08 **[W3]** [CODE] Real-model E2E: ask, retrieve, propose, apply or reject, stale, restart, index rebuild [AI-006].
- [ ] M9.09 **[W1]** [CODE] Automated no-egress test; Offline claim verified against actual network and process behaviour [AI-007]. Amended by M0.28 (P10): capture traffic for install, first launch, an offline core workflow and AI use including WebView2; disable unneeded WebView2 network features after testing.
- [ ] M9.10 **[W1]** [CODE] Prompt-injection and retrieval-poisoning corpus; privilege-escalation and nested-AI tests [AI-008].
- [ ] M9.11 **[W3]** [CODE] Runtime bundled in the Store package, weights downloaded on click, manifest and hash verification intact, quarantine on mismatch (ADR-0020) [AI-011].
- [ ] M9.12 **[W3]** [OWNER] X7 production manifest key generated and held offline; production host provisioned; `MODEL_DISTRIBUTION_BASE_URL` and `OPENFRAME_MANIFEST_PUBLIC_KEY` set; production manifest signed and published; Gemma terms review recorded (X8) [AI-012].
- [ ] M9.13 **[W1]** [CODE] **Report AI output** action: opens the support channel with app version and optional user-approved context; never sends project content automatically [AI-010]. Needs the support channel address from the owner (X15); use one configurable constant, never an invented address.
- [ ] M9.14 **[W1]** [CODE] Download resilience under packet loss and HTTP 408/429/5xx; disk-space preflight; cancel and resume; corrupt download quarantined.
- [ ] M9.15 **[W1]** [CODE] Model update regression suite wired to the benchmark [AI-013].
- [ ] M9.16 **[W1]** [CODE] Sidecar lifecycle: crash, hang, timeout, cancel, orphan cleanup on start, kill with the app, never from a project path. Amended by M0.28 (P8): create the sidecar suspended (or via `PROC_THREAD_ATTRIBUTE_JOB_LIST`), assign the Job Object, then resume; refuse to run when assignment fails; `JOB_OBJECT_LIMIT_ACTIVE_PROCESS` = 1; sweep orphaned `llama-server` processes under the managed runtime folder at startup; re-hash the executable before launch. Supersedes the AI-05 acceptance.

**Gate G9:** AI- P0 rows at S5; Recall@K and MRR meet approved thresholds; hardware results archived; no automatic AI writes; no cross-user or private-note leakage.

---

## M10. Product-wide quality, accessibility, performance, beta, manual

- [ ] M10.01 **[W4]** [CODE] **Figure verification pass:** every in-scope manual figure (176) mapped to a ledger row and an E2E or manual test result; the five LAN figures (164 to 168) marked removed.
- [ ] M10.02 **[W1]** [CODE] Project Home shows production route and next shooting day / latest call sheet summary [CORE-011].
- [ ] M10.03 **[W4]** [CODE] Accessibility: keyboard-only walkthrough of every workspace, visible focus, labelled icon buttons, no colour-only meaning, Windows High Contrast, screen reader semantics where applicable (Narrator); automated axe checks in component tests; report archived [CORE-016, CORE-017].
- [ ] M10.04 **[W4]** [CODE] High-DPI, multi-monitor, IME and Unicode input checks on Windows.
- [ ] M10.05 **[W4]** [CODE] Performance report on minimum and recommended hardware: launch, project open, save, typing latency, reflow, search on 500, 2k and 10k scene projects, exports, AI retrieval and index rebuild; all within doc 21 budgets.
- [ ] M10.06 **[W4]** [CODE] Second security review: IPC allow-list, filesystem scope, parsers, packages, themes, AI tools, process lifecycle, updater absence in Store build; threat model updated; all High findings fixed with regression tests.
- [ ] M10.07 **[W4]** [CODE] Offline run of the full core workflow with the network disabled (gate G12 in doc 22). Includes the M0.28 P10 traffic capture.
- [ ] M10.08 **[W4]** [CODE] Soak run on a release candidate (hours, repeated sleep and resume, repeated project switching).
- [ ] M10.09 **[W4]** [OWNER] D17 beta program: 8 to 15 real filmmakers use a release candidate for 3 to 4 weeks; feedback collected through the support channel; P0 and P1 findings fixed; summary archived (X12).
- [ ] M10.10 **[W1]** [CODE] Localization-ready architecture confirmed for v1 English-only UI (D18): strings externalised or a recorded exception.
- [ ] M10.11 **[W4]** [DOC] User Manual regenerated against the shipped UI; LAN figures removed; Offline AI and new features described; source-of-truth for the manual decided and stored (DOC-007).
- [ ] M10.12 **[W4]** [DOC] Remaining LAN text removed from the specification bodies (PRD-COL-005/006, FSD §48 to 49, §119 to 120, Collaboration spec, Security spec, UX mock-up references) (DOC-003).
- [ ] M10.13 **[W4]** [DOC] In-app help and public documentation outline complete (DOC-008).
- [ ] M10.14 **[W4]** [CODE] All P0 and P1 defects from beta, E2E and review closed; known-issues list written.

**Gate G10:** ledger P0 rows all S5; accessibility, performance, security, soak and beta reports archived.

---

## M11. Windows Store build and certification pre-flight

### Build and package
- [ ] M11.01 **[W4]** [CODE] Store channel build uses `OPENFRAME_DISTRIBUTION=store`; in-app updater compiled out; `release-validate --store` passes with no bypass (REL-003).
- [ ] M11.02 **[W4]** [CODE] MSIX manifest: Windows version per D3; capabilities only `runFullTrust` and `internetClient` with written justification; 77 visual assets valid; version `X.Y.Z.0`, higher than any prior submission.
- [ ] M11.03 **[W4]** [CODE] Release candidate tag; pipeline produces the MSIX, `msix-build.json`, `THIRD_PARTY_LICENSES.md`, SBOM and `SHA256SUMS.txt` (REL-008). The release candidate is **immutable** once approved.
- [ ] M11.04 **[W4]** [CODE] Licence inventory complete including bundled fonts, runtime, models notices (Gemma notice in the app and notices file).
- [ ] M11.05 **[W4]** [CODE] App data folder behaviour verified for packaged runs (`LocalState`), logs openable, uninstall keeps user projects and removes app data per policy.
- [ ] M11.06 **[W4]** [IF D1=C|D] Store licence/trial check implemented: trial starts, expires deterministically, offline behaviour, reinstall, clock tampering policy, purchase during trial, restore, outage never touches project data.

### Pre-flight tests
- [ ] M11.07 **[W4]** [OWNER] X11 Windows App Certification Kit run against the exact MSIX; all green; report archived (REL-004).
- [ ] M11.08 **[W4]** [BOTH] Clean-machine matrix on Windows 11 (clean VM or Windows Sandbox, no developer tools): install, first launch, create project, write, save, close, reopen, upgrade from the previous build, uninstall, reinstall, projects survive; WebView2 present and missing cases (REL-005).
- [ ] M11.09 **[W4]** [BOTH] First-launch crash check on a bare machine: no dependency on locally installed runtimes.
- [ ] M11.10 **[W4]** [BOTH] Installed-package E2E: full lifecycle journey and Offline AI install (download, verify, start, ask, apply) on the installed MSIX.
- [ ] M11.11 **[W4]** [BOTH] Windows Defender and antivirus interaction checked on the packaged build, including sidecar start.
- [ ] M11.12 **[W4]** [BOTH] Project open and migration from every previously shipped version using the fixture corpus on the packaged build.
- [ ] M11.13 **[W4]** [BOTH] Hardware matrix results (minimum, recommended, discrete GPU) recorded for the listing.
- [ ] M11.14 **[W4]** [CODE] Upgrade test: v0.x package to the next version with settings, projects, AI index invalidation and rebuild preserved.
- [ ] M11.15 **[W4]** [CODE] Binary hardening gate: release pipeline runs BinSkim (or `dumpbin /headers /loadconfig`) on `openframe-desktop.exe` and fails on missing DYNAMICBASE, HIGHENTROPYVA, NXCOMPAT, GUARD_CF, CETCOMPAT, an embedded PDB path, or a manifest that is not `asInvoker`; MSI install-scope decision recorded in doc 24 (M0.28 P7).
- [ ] M11.16 **[W4]** [CODE] Store and release builds fail to compile when `OPENFRAME_MANIFEST_PUBLIC_KEY` is unset (for example `compile_error!` under `cfg(openframe_store)`) (M0.28 P14; the key-sweep half was done 2026-10-05).

**Gate G11:** one immutable MSIX with WACK pass, clean-machine pass, installed-package E2E pass.

---

## M12. Legal, privacy, website, support (Store prerequisites)

- [ ] M12.01 **[W3]** [OWNER] X6 domain, company email, SPF, DKIM, DMARC, HTTPS hosting live (`GTM-001`, `GTM-004`, `GTM-005`).
- [ ] M12.02 **[W3]** [DOC] Privacy policy rewritten for the build and ADR-0021 (what leaves the computer: nothing automatic; model download host and retention; AI disclosure; contact), placeholders removed (REL-006).
- [ ] M12.03 **[W3]** [OWNER] X9 counsel review recorded for privacy policy, terms or EULA, refund text (if any), AI disclosures (`LEGAL-001`, `EXE-12-018`).
- [ ] M12.04 **[W3]** [OWNER] Privacy policy hosted at a stable HTTPS URL; URL set as `STORE_PRIVACY_POLICY_URL` (`EXE-08-008`).
- [ ] M12.05 **[W3]** [OWNER] X15 support channel live: company-owned inbox, public support page, severity levels, response target, second operator able to run it (`SUP-001`, `SUP-002`).
- [ ] M12.06 **[W3]** [DOC] Public knowledge base: install, update, recovery, import and export, AI troubleshooting, Windows troubleshooting (`SUP-004`).
- [ ] M12.07 **[W3]** [DOC] Website per `docs/website/README.md`: product, features, download or Store link, support, privacy, terms, security disclosure; claims match the build (`GTM-002`, `GTM-003`).
- [ ] M12.08 **[W3]** [OWNER] Security disclosure address published and tested (`SEC-009`).
- [ ] M12.09 **[W3]** [DOC] Terms or EULA and third-party notices page published (`LEGAL-002`).
- [ ] M12.10 **[W3]** [DOC] Website copy passes the writing audit: no fake testimonials, no invented numbers, no generic AI marketing phrases, em dashes removed from user-facing text.
- [ ] M12.11 **[W3]** [CODE] Website accessible, responsive at 375 to 1440 px, Lighthouse and broken-link checks recorded.

**Gate G12:** privacy URL, support contact, website and policies live and consistent.

---

## M13. Submission gate

- [ ] M13.01 **[W4]** [OWNER] X14 Store listing written from the shipped build: description, features, keywords, copyright, developed by; no unshipped claim (REL-007).
- [ ] M13.02 **[W4]** [OWNER] Screenshots captured from the exact release candidate with an invented demo project; captions accurate.
- [ ] M13.03 **[W4]** [OWNER] IARC questionnaire answered truthfully (screenplay content may be mature); category, regions, pricing, availability set to match D1.
- [ ] M13.04 **[W4]** [OWNER] Listing discloses live generative AI, the optional on-demand weight download, local operation, and the report path.
- [ ] M13.05 **[W4]** [OWNER] System requirements filled from M9 and M11.13 results (RAM, disk, OS).
- [ ] M13.06 **[W4]** [OWNER] **Notes for Certification** written: no account or login is needed; how to start; Offline AI is optional and how to test it; no network use except user-clicked downloads and opening web links; capabilities justification.
- [ ] M13.07 **[W4]** [DOC] Release notes and known-issues list; rollback artifact (previous MSIX) retained (REL-009).
- [ ] M13.08 **[W4]** [OWNER] X16 continuity: second administrators, recovery codes, key backups, runbooks stored; recovery drill completed.
- [ ] M13.09 **[W4]** [BOTH] Full evidence package assembled under `docs/release/evidence/release/` (see launch register Appendix C).
- [ ] M13.10 **[W4]** [BOTH] Adversarial final audit completed (launch register §26): every question answered with evidence.
- [ ] M13.11 **[W4]** [CODE] Re-check the current Microsoft Store Policies version and Partner Center requirements on the submission date; record any change.
- [ ] M13.12 **[W4]** [OWNER] **X18 GO/NO-GO signed** in §9.

**Gate G13:** every gate G0 to G12 PASS; no P0 stop-ship item open.

---

## M14. Certification, publish, launch

- [ ] M14.01 **[W4]** [OWNER] X19 upload the exact MSIX from M11 to Partner Center and submit.
- [ ] M14.02 **[W4]** [OWNER] Watch certification; respond to any reviewer finding within one business day; fix and resubmit using a higher package version.
- [ ] M14.03 **[W4]** [BOTH] After approval: install from the live Store listing on a clean machine; first launch; create project; trial or purchase flow if any; Offline AI install; update test when the next version exists.
- [ ] M14.04 **[W4]** [BOTH] Smoke-test live listing, privacy URL, support URL and website links.
- [ ] M14.05 **[W4]** [OWNER] Launch war room named: incident commander, engineering lead, product decision maker, support lead, security owner, backups; stop-ship triggers agreed (`OPS-001`).
- [ ] M14.06 **[W4]** [OWNER] X17 launch content ready: trailer, founder story, 30-day content bank, creator and community outreach plan, social accounts with MFA (not a Store gate).
- [ ] M14.07 **[W4]** [OWNER] Status and outage communication channel ready; customer feedback system live.

**Gate G14:** product live in the Store and smoke-tested.

---

## M15. First 90 days

- [ ] M15.01 **[W5]** Daily review for 7 days: Store crashes, ratings and reviews, support tickets, incidents (`OPS-003`).
- [ ] M15.02 **[W5]** Fix only urgent reliability, security and data-safety bugs under the release controls; each fix ships as a new immutable version.
- [ ] M15.03 **[W5]** Day 8 to 30 health review: top bugs, support themes, knowledge-base gaps, security advisories.
- [ ] M15.04 **[W5]** Day 31 to 90: quarterly cadence, backup and restore drill, account-access check, privacy and legal consistency, roadmap reprioritisation (`OPS-004`).
- [ ] M15.05 **[W5]** Maintenance calendar running: dependencies, certificates, domains, policies, vendors (`OPS-004`).

---

## M16. After the Store (optional): macOS and direct channel

- [ ] M16.01 **[W5]** [CODE] Platform service boundary refactor with no Windows behaviour change [PLAT-007, ADR-0017].
- [ ] M16.02 **[W5]** [CODE] macOS CI: format, clippy, tests, web, bindings on arm64 and x86_64.
- [ ] M16.03 **[W5]** [CODE] macOS sidecar lifecycle with the same security properties; WebKit editor tests (clipboard, IME, shortcuts, drag and drop).
- [ ] M16.04 **[W5]** [OWNER] X20 Apple Developer Program, Developer ID, notarization credentials.
- [ ] M16.05 **[W5]** [CODE] Developer ID signing, Hardened Runtime, minimal entitlements, notarization, stapling, Gatekeeper and quarantine tests.
- [ ] M16.06 **[W5]** [CODE] Cross-platform project round trip (Windows to macOS and back).
- [ ] M16.07 **[W5]** [CODE] Lifecycle E2E on macOS; release matrix complete.
- [ ] M16.08 **[W5]** [OWNER] D15, X21 direct channel: Authenticode certificate, updater key, signed NSIS and MSI, approval-based updater with signature verification [REL-010].

---

## 8. Conditional rows by business model (D1)

| Row | A free open source | B free proprietary | C paid with trial | D free + add-ons |
|---|---|---|---|---|
| Licence file and notices | OSI licence | EULA | EULA | EULA |
| M11.06 Store licence/trial code | N/A | N/A | required | required (add-on) |
| `COMM-001..006`, `EXE-09-*` | N/A | N/A | required | required |
| Refund and cancellation text | N/A | N/A | required | required |
| Pricing and trial in listing | Free | Free | required | required |
| Contribution policy (DCO or CLA) | required | N/A | N/A | N/A |

## 9. Store-ready gate sign-off

| Gate | Meaning | Evidence | Owner | Date | PASS |
|---|---|---|---|---|---|
| G0 | Decisions, baseline, controls | `docs/release/evidence/M0/` | | | ☐ |
| G1 | Safety net and harnesses | | | | ☐ |
| G2 | Screenplay engine and interchange | | | | ☐ |
| G3 | Writer comfort and Appearance | | | | ☐ |
| G4 | Story Board and Idea Vault | | | | ☐ |
| G5 | Breakdown and production | | | | ☐ |
| G6 | Schedule and DOOD | | | | ☐ |
| G7 | Call sheets, budget, templates, exports | | | | ☐ |
| G8 | Packages and backup | | | | ☐ |
| G9 | AI completion | | | | ☐ |
| G10 | Product-wide quality and beta | | | | ☐ |
| G11 | Store build and pre-flight | | | | ☐ |
| G12 | Legal, privacy, website, support | | | | ☐ |
| G13 | Submission gate | | | | ☐ |

**Release decision:** version ______ commit ______ MSIX SHA-256 ______ decision owner ______ date ______
**GO / NO-GO / GO WITH FORMAL RISK ACCEPTANCE:** ______

When G0 to G13 are PASS and the decision is GO, submit (M14). G14 closes when the listing is live.

## 10. Stop-ship conditions (any one blocks submission)

- [ ] Data corruption or project loss; migration failure; crash recovery that can lose customer work.
- [ ] Private creative content leaves the computer without a user click; private-note leak; AI privilege bypass or automatic AI write.
- [ ] Unverifiable or placeholder identity, key, URL or policy in the package or listing.
- [ ] Listing claims a feature that is not S5.
- [ ] MSIX not identical to the one tested; WACK or clean-machine failure.
- [ ] Import or export loses meaning against a published promise.
- [ ] Critical unresolved security finding.

## 11. Change log

| Date | Change |
|---|---|
| 2026-10-05 | Draft v1 created from the supplied audits, plans, launch register, specification and repository verification. |
