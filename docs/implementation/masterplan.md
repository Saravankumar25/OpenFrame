# OpenFrame Studio build plan

> **Current decisions (2026-10-15).** This plan is the checklist and evidence ledger. The phases W1 to W10 and M1 to M10 in [roadmap/](../roadmap/README.md) say what to do and in what order, and its table maps each area to its phase. Statements in area 8 about the processor build only, one model and bundled models are replaced by [ADR-0026](../adr/ADR-0026-offline-ai-download-after-install.md), [ADR-0028](../adr/ADR-0028-ai-quality-program-and-harness.md) and [ADR-0029](../adr/ADR-0029-gpu-acceleration.md). Mac statements in areas 7 and 20 follow [ADR-0027](../adr/ADR-0027-rust-core-two-ui-shells.md). "Verified" follows the five proof levels of [ADR-0030](../adr/ADR-0030-verification-ladder-and-release-flow.md); a box ticked before 2026-10-15 is at proof level 2 unless its evidence says more.

> Rewritten 2026-10-06, chapter map added 2026-10-09. This is the single checklist for finishing OpenFrame Studio and shipping it in three files built
> from one codebase. It replaces the earlier coded checklist. Every item is a plain sentence under a descriptive heading.
> Companion files: [feature-status.md](feature-status.md) (state of each feature), [owner-decisions.md](owner-decisions.md)
> (what only you can decide or do), [progress-log.md](progress-log.md) (what each session did, with live counts),
> [reconciliation-log.md](reconciliation-log.md) (conflicts found, old-to-new name tables),
> [../release/launch-readiness-register.md](../release/launch-readiness-register.md) (operations, legal and support requirements).

## What we are building and shipping

One application, one codebase, two platform adapters (Windows and macOS). Four release files (ADR-0024; the table had
three before 2026-10-07, when the installer became an internal build):

| Release file | For | Built from |
|---|---|---|
| Microsoft Store package (`.msix`) | Windows 11, sold through the Microsoft Store | Windows adapter, Store channel |
| Mac App Store package (`.app` in a `.pkg`) | Macs with Apple silicon, sold through the Mac App Store | macOS adapter, App Store channel |
| Internal Windows installer (`.exe`) | The owner's team only, never sold (ADR-0023) | Windows adapter, internal channel |
| Internal Mac disk image (`.dmg`) | The owner's team only, never sold (ADR-0024) | macOS adapter, internal channel |

Every one of them contains the complete offline AI (runtime, Gemma 3 1B weights, embedding weights, tokenizer), so AI
the AI runtime ships in every package and the AI models download after install on the user's click ([ADR-0026](../adr/ADR-0026-offline-ai-download-after-install.md), reinstating the download that ADR-0025 had replaced).

**Current goal (owner decision 2026-10-08).** Finish building, testing, verification, end-to-end tests and packaging of
the two internal builds: the Windows `.exe` now, and the Mac `.dmg`, which is built and tested on a real Apple silicon
Mac later. Both stay unsigned for now and keep the team password gate on (ADR-0024 amendment); the owner sets
`OPENFRAME_TEAM_PASSWORD` on the build PC. **Later step:** publishing to the Microsoft Store (`.msix`) and the Mac App
Store (`.app` in a `.pkg`). Boxes that only matter for that step (store packages, Partner Center, App Store Connect,
StoreKit and Microsoft Store purchase calls against live stores, listings, certification, submission, publication,
launch, the first 90 days, signing) carry the tag **(Later: store publishing)**. Boxes that cannot be done without an
Apple silicon Mac carry **(Needs Mac hardware)** and stay unverified until a Mac is available. A box can carry both.
`npm run plan:status` prints a "Current goal" line that counts only the boxes without the store publishing tag.

Business model: paid, with a 7-day free trial and no credit card needed to start the trial. Each channel handles payment and
trial in its own way, behind one shared entitlement interface in the code (see the area "Payment and free trial").

## Chapters (owner decision 2026-10-09)

The work is organised in three chapters; the plan for each is in [../roadmap/](../roadmap/README.md). This build plan
stays the detailed checklist. The chapter pages say what to do next and how each item is proved.

| Chapter | Areas of this plan |
|---|---|
| 1. Complete, test and validate (internal `.exe` and `.dmg`, then 2 to 3 days with writers and production teams) | 1 to 17, 20 (the internal installers); the store-only boxes inside them keep their "Later" tag |
| 2. Product website | 22 (website, support pages); the website plan in [../roadmap/chapter-2-website.md](../roadmap/chapter-2-website.md) |
| 3. Commercial distribution (Microsoft Store and Mac App Store) | 18, 19, 21, 22 (licence agreement, counsel), 23 to 26 |

The area-by-area table with notes is in [../roadmap/README.md](../roadmap/README.md). New boxes added on 2026-10-09 come from
the full audit: typed text on window close, cloud-synced project folders, project delete and locate tests, a release-build
smoke test, a measurement of non-English AI requests and the internal tester pilot.

## What "done" means

OpenFrame is ready to ship only when all of this is true:

1. Every feature named in a store listing or on the website is **Release-ready** in [feature-status.md](feature-status.md).
   Every feature marked "Must ship" is Release-ready. "Should ship" features are Release-ready or removed from all listings.
2. Every area below has its "Done when" condition met, with evidence.
3. The exact release file that will be uploaded or published has passed its own install, first launch, upgrade, uninstall
   and reinstall checks on a clean machine, and the store's own validation tool where one exists.
4. No open defect that loses data, breaks security, fails a migration or recovery, silently drops import or export content,
   lets the AI write on its own, or exposes private data to the AI.
5. Privacy policy, support contact, licence terms and store listings are live, true and match the build.
6. The owner has signed the final go decision.

No software is free of every bug. This is the standard the plan can honestly meet.

## How to use this plan

- **Tick a box only with evidence.** Write the evidence after the item as `(ev: file, test name or pull request)`. Store larger
  evidence under `docs/release/evidence/`. The folder names there still use the old item codes; the translation table in
  [reconciliation-log.md](reconciliation-log.md) maps each old code to its item below.
- **Definition of done for every build task** (not repeated below): the feature can be found in the UI, saves, undoes,
  survives a restart and a crash, exports and re-imports where that applies, enforces permissions in Rust, is searchable,
  has an AI decision recorded (tools, retrieval, tests, regenerated `docs/engineering/ai-tool-coverage.md`), works with the
  keyboard, stays inside the performance budget, and has its documentation and feature status updated.
- **Small slices.** One vertical slice per pull request (data, operation, screen, tests). No unrelated refactors. No bypass
  of the architecture rules in CONTRIBUTING.
- **Git flow.** Work on a feature branch. When a slice is finished and verified, commit locally with a plain message and
  merge it locally once its tests and end-to-end specs pass (owner decision 2026-10-08). Push, pull requests and
  repository changes happen only when the owner asks; then the pull request merges only on green checks. Never merge red.
- **Clean as you go.** When a feature is finished, remove the code and documents it replaced. Mark an older decision
  superseded in place, add the new decision to [../adr/README.md](../adr/README.md), and delete dead code in the same pull request.
- **Keep documents true.** A change that alters behaviour updates the feature status, the affected specification or
  engineering document, the user manual source if users see it, and this plan.
- **Writing standard.** No em dashes and no AI-sounding patterns in the app, exported files, installers, store listings,
  website, manual or documents. The rules are in [../engineering/35-writing-standard.md](../engineering/35-writing-standard.md).
- A box that no longer applies is marked `N/A: reason`. It is never deleted.

### Windows and Mac security standard (permanent)

OpenFrame must meet the security bar of well-regarded professional desktop applications on both platforms, on every change.

- **Least privilege:** runs as a standard user, no elevation. Windows store capabilities limited to `runFullTrust` and `internetClient`. Mac entitlements limited to the sandbox, user-selected files and outgoing network.
- **Binary hardening:** address-space randomisation, data execution prevention, control-flow guard on Windows, hardened runtime on Mac, no secrets or debug data in release files, no unsafe Rust without a written reason.
- **Web view and IPC:** strict content security policy, no web view access to files, shell or network, one allow-listed entry point to the Rust core, every path resolved and confined in Rust, every untrusted input validated in Rust.
- **Process safety:** the AI engine listens only on 127.0.0.1 with a random port and a per-launch key, never runs from a project folder, stops with the app, and stray copies are cleaned up at startup.
- **Data and files:** bounded parsers, no archive path escapes, no symlink escapes, network folders refused, atomic writes, backup before migration, no project content in logs.
- **Supply chain:** `cargo deny`, `cargo audit`, `npm audit`, the licence inventory and the software bill of materials pass without bypass. Production keys never live in source. The development key is never trusted.
- **Distribution:** signed and verified release files, pinned sizes and SHA-256 hashes for the AI files downloaded from Hugging Face (ADR-0026, ADR-0032), updates only with user approval, no silent network use, no telemetry. The internal `.exe` and `.dmg` stay unsigned until the store and public step (owner decision 2026-10-08, ADR-0024 amendment).
- **Review:** run the `security-guidance` plugin and `ecc:security-review` on any change that touches parsers, paths, packages, AI, process control, settings or imports.

## Build order

Work down this table. An area starts when the areas it needs are far enough along. Areas with no dependency can run in parallel
(use separate branches and worktrees). Owner decisions that block an area are named so they are asked early.

| Order | Area | Needs first | Blocked by owner input |
|---|---|---|---|
| 1 | Decisions and owner tasks | nothing | you, any time |
| 2 | Project controls and baseline | nothing | none |
| 3 | Data safety and crash recovery | Project controls | none |
| 4 | Runtime resilience and failure testing | Data safety | none |
| 5 | Test infrastructure and quality gates | Project controls | none |
| 6 | Security hardening from the baseline audit | Project controls | none |
| 7 | Platform adapter layer for Windows and Mac | Test infrastructure | none (Mac hardware needed for tests) |
| 8 | AI assistant quality and hardening | Test infrastructure | support address, AI test hardware |
| 9 | Beat Board and Idea Vault | Test infrastructure | one product question |
| 10 | Writing comfort | Test infrastructure | none |
| 11 | Screenplay engine and file formats | Data safety, Test infrastructure | production numbers, PDF fonts |
| 12 | Appearance and themes | Test infrastructure | themes decision |
| 13 | Breakdown, catalog, locations, cast and crew | Screenplay engine | none |
| 14 | Shooting schedule and Day Out of Days | Breakdown | none |
| 15 | Call sheets, budget, templates, samples and exports | Shooting schedule | template list, sample scripts |
| 16 | Packages and backup | Screenplay engine | none |
| 17 | Product-wide quality, accessibility and beta | areas 8 to 16 | beta group |
| 18 | Payment and free trial in all three channels | Platform adapter layer | price, Partner Center product identity |
| 19 | Microsoft Store package | areas 17, 18 | Partner Center account |
| 20 | Windows installer | areas 17, 18 | none for the internal build (signing certificate only at the store and public step) |
| 21 | Mac App Store package | areas 7, 17, 18 | Apple Developer account, Mac hardware |
| 22 | Legal, privacy, website and support | area 1 | counsel, domain, support inbox |
| 23 | Final review and submission | areas 19 to 22 | your go decision |
| 24 | Certification, publication and launch | area 23 | none |
| 25 | First 90 days after launch | area 24 | none |
| 26 | Conditions that stop a release | checked in area 23 | none |

---

## 1. Decisions and owner tasks

Only you can decide or do these. Details and recommendations are in [owner-decisions.md](owner-decisions.md).

**Decided**
- [x] **Business model.** Paid with a 7-day free trial, no credit card to start (ev: owner answer in chat, 2026-10-06; ADR-0017)
- [x] **Windows versions.** Windows 11 only (ev: owner answer in chat, 2026-10-06; ADR-0017)
- [x] **Mac hardware.** Apple silicon only, no Intel (ev: owner instruction in chat, 2026-10-06; ADR-0017)
- [x] **Release files.** Three files from one codebase with two platform adapters: Microsoft Store package, Windows installer, Mac App Store package (ev: owner instruction in chat, 2026-10-06; ADR-0017)
- [x] **Diagnostics.** The app sends nothing automatically; users can export an inspectable, redacted diagnostics file themselves (ev: ADR-0021 Accepted 2026-10-05)
- [x] **Crash-safe AI changes.** A change that was interrupted by a crash is undone on the next open (ev: ADR-0019 Accepted 2026-10-05)
- [x] **GitHub flow.** Feature branch, pull request, merge when checks pass (ev: owner answer in chat, 2026-10-06; written into CLAUDE.md). Amended 2026-10-08: work reaches `main` by a local merge after tests and end-to-end specs pass; no push, pull request or repository change unless the owner asks (ev: owner answer in chat 2026-10-08; CLAUDE.md git flow; owner-decisions.md)

**Still open (ask early because they block later areas)**
- [ ] **Price, currency and regions** for the paid app, and what the trial includes. (2026-10-08: the owner will decide later; nothing in the app, listings or refund text changes until then) (Later: store publishing)
- [x] **How the Windows installer is sold and licensed.** Payment provider, licence key method, offline checking, refund rules. (ev: owner answer in chat 2026-10-07, ADR-0023: not sold; internal team build with no licence)
- [x] **How the Mac App Store trial works.** Free download that unlocks with an in-app purchase after 7 days, or a different method Apple allows. (ev: owner answer in chat 2026-10-08: free download, buy in the app after 7 days, full block at expiry; ADR-0022 confirmed; ADR-0018 decision 6 for the method) (Later: store publishing)
- [ ] **Refund and cancellation rules** for each channel. (2026-10-08: decided later by the owner) (Later: store publishing)
- [x] **Themes and Dark mode** (ADR-0014). (ev: owner answer in chat 2026-10-07; ADR-0014 Accepted)
- [x] **Production scene numbers** that stay fixed after a script is locked (ADR-0015). (ev: owner answer in chat 2026-10-07; ADR-0015 Accepted)
- [x] **PDF fonts for Indian scripts** (ADR-0016). (ev: owner answer in chat 2026-10-07; ADR-0016 Accepted)
- [x] **AI engine packaged inside each store build** instead of downloaded later (ADR-0020). Both stores restrict downloaded programs, so this is recommended. (ev: owner answer in chat 2026-10-08, ADR-0020 Accepted: the engine ships inside every package, only model weights download)
- [x] **Built-in template list** (seven exist, eight proposed) and **who writes the sample scripts**. (ev: owner answer in chat 2026-10-07: eight templates; four OpenFrame-owned fictional samples; owner-decisions.md)
- [ ] **Support model and address**, **target launch date and team capacity**, **beta group**, **English-only first release**.
- [x] **Product questions left open earlier:** whether Build Screenplay stops on missing scene headings, whether a project can hold several screenplays, the call sheet status words, how long Recently Deleted keeps items. (ev: three answered in chat 2026-10-07 and the fourth on 2026-10-08, a project may hold several screenplays; ADR-0012 amendments of both dates)
- [x] **Keep the GitHub repository private** (the app is paid and closed source). (ev: owner answer in chat 2026-10-06; `gh repo view --json visibility` returns PRIVATE; progress-log entry 2026-10-06)

**Owner tasks that code cannot do**
- [ ] Confirm the legal entity that publishes the app. (Later: store publishing)
- [ ] Create the Microsoft Partner Center account, verify it, add a second administrator, reserve the name "OpenFrame Studio", and put the identity values into `packaging/msix/store.config.json`. (Later: store publishing)
- [ ] Create the Apple Developer Program account and App Store Connect record, add a second administrator. (Later: store publishing)
- [ ] Buy a Windows code-signing certificate for the installer. (Later: store publishing)
- [ ] Write or approve the end-user licence agreement and replace `LICENSE-PENDING.md` with it. Update the package manifests. (Later: store publishing)
- [ ] Trademark, domain, social handles, logo ownership and font licence checks.
- [ ] Domain, company email and website hosting. (Later: store publishing)
- [ ] N/A: superseded 2026-10-16 by ADR-0032 (no model host, no signing key). **Production model host and the offline signing key for the AI download list.** Reopened 2026-10-15 by ADR-0026 (owner task in `owner-decisions.md`). Needed before any release build that downloads the models.
- [ ] Counsel review of the privacy policy, licence terms, refund text and AI disclosures. (Later: store publishing)
- [ ] Test hardware: a minimum-spec Windows 11 PC, a PC with a discrete graphics card, clean Windows 11 virtual machines, an Apple silicon Mac.
- [ ] Turn on branch protection for `main` so it needs green checks. Owner approved on 2026-10-06; GitHub refused the setting on a private repository without GitHub Pro (HTTP 403). Either upgrade the account or use a free ruleset: Settings, Rules, Rulesets, New branch ruleset for `main`, require a pull request and the five status checks.
- [ ] Recruit and run the beta group. (Later: store publishing)
- [ ] Marketing assets: trailer, founder story, content plan, outreach plan. (Later: store publishing)

**Done when:** every open decision is answered or deferred with a date, and every owner task is done or scheduled.

## 2. Project controls and baseline

Goal: stop drift and record the true starting point.

- [x] **Baseline tag and evidence run.** Tag `baseline/pre-masterplan-2026-10`; record verify, Rust tests, performance, desktop end-to-end tests, migration versions, the AI tool coverage file and a sample project. (ev: tag baseline/pre-masterplan-2026-10; docs/release/evidence/M0/INDEX.md#M0-04 to #M0-09, #M0-25 (end-to-end 17/17), #M0-26 (performance 39/39); AI benchmark not applicable without models)
- [x] **Feature status checked against evidence.** Every unverified tag in the status file replaced by a link to evidence or a corrected state. (ev: docs/release/evidence/M0/INDEX.md#M0-14; 65 states corrected)
- [x] **Visual baseline.** Screenshots of 15 key screens in the current appearance, taken before any colour clean-up. (ev: docs/release/evidence/M0/M0.20-visual-baseline/, 15 screens; INDEX.md#M0-27; tests/e2e/specs/90-visual-baseline.test.mjs)
- [x] **Supply-chain checks green and blocking.** `cargo deny`, `npm audit --omit=dev` and the licence inventory pass and block releases. (ev: docs/release/evidence/M0/INDEX.md#M0-15, #M0-16)
- [x] **Feature status check in CI.** A script fails when a feature row lacks a source, area, screen, tests or state. (ev: scripts/check-feature-status.mjs, renamed from check-ledger.mjs on 2026-10-06; INDEX.md#M0-17, #M0-18; CI job `governance`)
- [x] **Architecture lint in CI.** No forbidden web imports, no database writes outside the allowed layers, no unregistered operations, no binding drift, no AI call to the apply step. (ev: scripts/architecture-lint.mjs; INDEX.md#M0-17, #M0-18; CI job `governance`)
- [x] **Migration policy.** Migrations are frozen at the first build that leaves the team; later changes only add. (ev: `docs/engineering/23-release-migration.md` section 4; scripts/check-migrations.mjs; migrations/frozen.json; INDEX.md#M0-19)
- [x] **Evidence folder and index template.** (ev: docs/release/evidence/README.md; INDEX.md#M0-20)
- [x] **Pull request template.** Contains the feature contract, the risk level and the review checklist. (ev: .github/pull_request_template.md; INDEX.md#M0-21)
- [x] **Windows security baseline audit.** Repository compared with the security standard; gaps reported with file evidence. (ev: docs/release/evidence/M0/M0.28-security-baseline.md, M0.28-fixes-applied.md; follow-up items are in the area "Security hardening from the baseline audit")
- [x] **Documentation structure.** Specifications in `docs/specs/`, manual in `docs/manual/`, decision register in `docs/adr/README.md`, older LAN text marked superseded in place. (ev: docs/README.md; docs/adr/README.md)
- [x] **Plain names in the plan documents.** All plan, status, decision and log documents use descriptive headings instead of item codes. (ev: this file; reconciliation-log.md translation tables)
- [x] **Progress counter and link checker.** `npm run plan:status` counts boxes per area and fails when a checked box has no evidence; `npm run check:links` fails on a broken link in a living document; both run in the CI job `governance`. (ev: scripts/plan-status.mjs, scripts/check-links.mjs, scripts/tests/governance.test.mjs 47 of 47 passing 2026-10-06)
- [x] **Git flow and working rules written down.** Feature branch, pull request, merge only on green checks; writing standard; clean as you go; three-file scope. (ev: CLAUDE.md, CONTRIBUTING.md section 6, .github/pull_request_template.md)
- [ ] **Align local history with GitHub.** The local `master` and GitHub `main` have different histories with identical files. Create a working branch from `main`, move the work onto it, and use branches from `main` from now on.
- [ ] **Dead code and old documents audit.** List unused Rust code, unused TypeScript exports, unused dependencies and documents that nothing links to; remove what is safe; record the rest.
- [ ] **Owner review of the documentation set.** Read the documentation map, the plan, the engineering contracts for the screenplay engine and production depth, and approve or correct them.

**Done when:** controls run in CI, the baseline evidence is committed, the supply chain is green, and `main` carries the working history.

## 3. Data safety and crash recovery

Goal: the user's project survives crashes, bad files and partial operations.

- [x] **Crash-safe AI changes.** Interrupted multi-step AI changes are detected on open and undone. (ev: migration `0011_ai_apply_journal.sql`; `change_set.rs` journal and recovery; INDEX.md#M1-01, #M1-03; ADR-0019)
- [x] **Forced stop after every apply step.** The project reopens consistent after a stop at any step. (ev: `tests/ai_apply_recovery.rs`, 8 tests; INDEX.md#M1-02)
- [x] **Forced stop during save, migration, export, package import and backup.** Recovery is offered and no committed edit is lost. (ev: `tests/forced_kill.rs`, 5 scenarios at 3 kill points; INDEX.md#M1-04)
- [x] **Migration test files for every schema version.** Project versions 1 to 11, global 1 to 3, app 1: upgrade, backup exists, integrity check, search and AI index rebuild, still exportable. (ev: `tests/migration_fixtures.rs`; INDEX.md#M1-05)
- [x] **Migration failure cases.** Interrupted, disk full, read-only disk, newer-than-supported file, corrupt database left untouched. (ev: docs/release/evidence/2026-10-06-foundation/INDEX.md#F-03; `tests/migration_failures.rs` 5 of 5)
- [x] **Property tests.** Order after repeated moves, undo and redo identity, delete and restore, import then export then import keeps the required meaning. (ev: docs/release/evidence/2026-10-06-foundation/INDEX.md#F-04; `tests/property_invariants.rs` 6 of 6)
- [x] **Stale AI proposal tests for every domain.** Retrieve, propose, user edits, apply refuses or rechecks: screenplay, story, breakdown, schedule, call sheet, cast and crew, budget, notes, visual planning. (ev: docs/release/evidence/2026-10-06-foundation/INDEX.md#F-05; `tests/ai_stale_proposals.rs` 10 of 10)
- [ ] **Typed text survives closing the window.** The window waits until every editor (screenplay, Idea Vault notes, call sheet fields) has sent its pending text to Rust before it closes; an end-to-end step types and closes within 100 ms, on Windows and on the Mac. Added 2026-10-09 after the audit found no close handler (`sync.ts` waits 400 ms; `lib.rs` shuts down on the destroyed event). Windows done 2026-10-16 (proof level 2: `close_gate.rs`, `app/pendingSaves.ts`, spec 04 passed 20 rounds of each way of closing, docs/release/evidence/w2/close-handshake-and-data-safety-2026-10-16.md); the Mac half waits for the Mac phases.
- [ ] **Cloud-synced project folders.** OpenFrame detects a projects folder inside OneDrive, Dropbox, Google Drive or iCloud Drive, says so in plain words and offers a local folder; a 10-minute typing test runs on a synced folder. Added 2026-10-09 (ADR-0003 allows synced folders, nothing warns). Detection, warning and tests done 2026-10-16 (docs/release/evidence/w2/close-handshake-and-data-safety-2026-10-16.md); the manual test (`docs/testing/onedrive-typing-test.md`) waits for the owner.
- [x] **Project delete and locate are tested.** `project.delete` (to the Recycle Bin or Trash) and `project.locate` have integration tests and one end-to-end step on Home. Added 2026-10-09: no test calls either operation. (ev: docs/release/evidence/w2/close-handshake-and-data-safety-2026-10-16.md, `platform/project_lifecycle.rs`, `10-home.test.mjs` delete step)
- [x] **Migration backup restores.** The migration fixture test opens each `pre-migration` backup and compares rows with the old fixture, not only that the file exists. Added 2026-10-09. (ev: docs/release/evidence/w2/close-handshake-and-data-safety-2026-10-16.md, `migration_fixtures.rs` with a deliberate break shown to fail)

**Done when:** every crash and failure case above passes in CI and the project reopens consistent each time.

## 4. Runtime resilience and failure testing

- [ ] **Long-running soak tests.** Many parallel calls, rapid autosave, undo during indexing, close and reopen during indexing, repeated AI cancel, AI engine crash and restart loop, hours of edits and thousands of changes. (short form passes in CI, docs/release/evidence/2026-10-06-foundation/INDEX.md#F-07; the 30-minute form `m1_08_soak_long` has not been run)
- [x] **Index shutdown stress.** Repeated open, close and rebuild while the embedding engine is stalled. (ev: docs/release/evidence/2026-10-06-foundation/INDEX.md#F-06; `tests/ai_index_stress.rs` no longer ignored, close under 2 s while the embedder is stalled)
- [x] **AI engine start retries when its port is taken.** With a test. (ev: docs/release/evidence/2026-10-06-foundation/INDEX.md#F-09; `openframe-ai/tests/supervisor.rs` 8 of 8)
- [x] **Failure injection.** Disk full, permission denied, folder disappears, external drive removed, read-only destination, sleep and resume during download and indexing, antivirus locks a file, very long paths, non-ASCII project paths. (ev: docs/release/evidence/2026-10-06-foundation/INDEX.md#F-08; `tests/failure_injection.rs` 16 of 16, in CI through `cargo test --workspace`)

**Done when:** the soak and failure suites run in CI and leave no corrupt project, hung process or lost edit.

## 5. Test infrastructure and quality gates

- [ ] **Desktop end-to-end tests in CI as a release gate.** Runs on a Windows runner and blocks release. (job `Desktop E2E (release gate)` exists in `ci.yml` and `release.yml`; the suite passes locally, docs/release/evidence/2026-10-06-foundation/INDEX.md#F-24; a green run on the GitHub runner is still to be observed)
- [ ] **One end-to-end spec per workspace and one full journey.** Create project, story, scene, screenplay, breakdown, schedule, call sheet; save, close, relaunch, reopen, edit, undo, redo, export, re-import. (journey, relaunch, home, idea vault, story, screenplay, breakdown, notes and tasks and project pages pass, docs/release/evidence/2026-10-06-foundation/INDEX.md#F-24; schedule, catalog and call sheets pass since 2026-10-09 in 61, 51 and 62; still missing: a Files workspace spec and export then re-import inside the journey)
- [x] **Reachability test.** Generated from the feature status file: every workspace, tab and key command has a screen route. (ev: docs/release/evidence/2026-10-06-foundation/INDEX.md#F-24; `scripts/check-reachability.mjs` in the governance job and `tests/e2e/specs/95-reachability.test.mjs` clicking all 30 canonical routes in the real app)
- [ ] **Hostile file collection.** Normal, malformed, huge, nested, archive escape, path traversal, entity expansion, decompression bomb, symlink, case collision and Unicode path files kept for FDX, Fountain, DOCX, PDF, ZIP and packages, plus a fuzz run for each parser. (parser corpus passes 13 of 13 and nine parser defects were fixed, docs/release/evidence/2026-10-06-foundation/INDEX.md#F-10; the packages corpus test and the fuzz runs are still to write)
- [ ] **PDF image comparison tests.** Page count, margins, headings, dialogue, title page, numbers, continuation, revision marks, headers, no clipping. (work started)
- [x] **Screen visual comparison tests** using the saved baseline. (ev: docs/release/evidence/2026-10-06-foundation/INDEX.md#F-24; `tests/e2e/specs/90-visual-baseline.test.mjs`, 15 screens at a 0.1 % threshold with deterministic capture; baseline rows M0-29 to M0-31)
- [ ] **Log redaction test.** No screenplay text, notes, contacts, prompts, paths or keys in any log, including the AI engine log. Release builds cap log detail for non-OpenFrame code and remove or redact the project lock path. (work started)
- [x] **Export diagnostics.** A user-started, inspectable, redacted diagnostics file. (ADR-0021) (ev: docs/release/evidence/2026-10-06-foundation/INDEX.md#F-11; `tests/diagnostics.rs` 9 of 9, `SupportDiagnostics.test.tsx` 7 of 7; Settings > Support)
- [ ] **Performance budgets extended** for typing, page reflow, breakdown filters, schedule operations, call sheet and PDF export. (work started)
- [x] **Writing standard check with a ratchet.** A script fails when the count of em dashes and banned AI-sounding phrases goes up in any group of folders. (ev: scripts/check-writing-standard.mjs, scripts/writing-standard-baseline.json, docs/engineering/35-writing-standard.md, CI job `governance`; baseline 2973 hits on 2026-10-06, lowered to 2909 the same day)
- [ ] **Clean the writing standard hits to zero.** Start with text the user can see: app screens and errors, exports and templates, installer text, listings, then living documents, then code comments. Lower the baseline with each pull request and switch the check to strict mode at zero. The generated `docs/engineering/ai-tool-coverage.md` has 439 hits, so fix its generator first. The mock-ups folder has about 1300.
- [ ] **Dead code and unused dependency check** in CI as a warning that becomes a failure once the first cleanup is done.

- [ ] **One test binary per crate.** `openframe-application` builds 40 separate integration test binaries (65 across the workspace), and each one compiles and links the whole crate graph. Join each crate's `tests/*.rs` into one binary (`tests/main.rs` with modules) after the session 2 branches merge, with the same test count before and after, to cut test build and link time.

**Done when:** CI shows end-to-end tests, architecture lint, feature status check, supply chain, writing standard and the test collections as blocking.

## 6. Security hardening from the baseline audit

Source: `docs/release/evidence/M0/M0.28-security-baseline.md`.

- [ ] **Asset protocol scope.** Remove project paths from the allowed list when a project closes or switches, with a test at the adapter. (The cache folder exposure was already removed on 2026-10-05.) (work started, then reverted)
- [x] **Link and junction safety.** Path confinement and every write destination resolve the nearest existing folder through junctions and symlinks and refuse anything outside the root; test with a junctioned assets folder. (ev: docs/release/evidence/2026-10-06-foundation/INDEX.md#F-12; `openframe-security/tests/confinement_and_network.rs` 9 of 9, `security_paths.rs` 10 of 10)
- [x] **Network drive refusal.** Mapped network drives and paths that resolve to network locations are refused; the dialog-file exception is recorded. (ev: docs/release/evidence/2026-10-06-foundation/INDEX.md#F-13; same suites; dialog-file exception recorded in `docs/engineering/20-security-threat-model.md`)
- [x] **One shared atomic file replace.** Unique temp name in the target folder, cleanup on failure, no delete before rename; used by project format, import-export, AI store, zip writing and backup; covered by the forced-stop tests. (ev: docs/release/evidence/2026-10-06-foundation/INDEX.md#F-14; `openframe_platform::atomic`, 12 call sites listed in `docs/engineering/37-platform-boundary.md` section 4; `forced_kill` 5 of 5)
- [ ] **App database migration safety.** Integrity check and verified backup before migrating `app.sqlite`.
- [ ] **Release hardening flags.** Control-flow guard for Windows builds, evaluate CET compatibility, remove build paths from the binary. Note 2026-10-08: dev and test builds link with rust-lld (`.cargo/config.toml`); every release build keeps the Microsoft linker through `scripts/with-msvc-linker.mjs` (`npm run build`, `bundle`, `perf`) and `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=link.exe` in `release.yml`, so these flags must be set and checked on Microsoft-linked binaries. Note 2026-10-18: `with-msvc-linker.mjs` now also passes `-C link-arg=/CETCOMPAT` (BinSkim BA2025) and adds `/ZH:SHA_256` to `CFLAGS`/`CXXFLAGS_x86_64_pc_windows_msvc` for `cc`-built C code such as bundled SQLite (BA2004); `scripts/check-binary.mjs` now fails without CETCOMPAT. The final installer build proves it.
- [ ] **Supply-chain pinning.** Release gates also run `cargo audit` and `npm audit`; the store job produces the CycloneDX software bill of materials; every GitHub Action pinned to a commit and every installed tool to an exact version.
- [ ] **Tauri command permissions.** Put the four Tauri commands under the Tauri access list and fold the three helper commands into registry operations, or document "one data entry point plus three reviewed helpers". (ev: `build.rs` `APP_COMMANDS` + `allow-of-*` in `capabilities/default.json`; Tauri's build fails on an unknown permission id and `cargo check`, `test`, `clippy -D warnings` and `check:governance` pass; unused fullscreen, app-version and message/ask/confirm permissions removed. One data entry point plus four reviewed helpers, documented in threat model T1/T2.) (2026-10-17: done in code, build-time permission check, desktop tests 42 pass and governance 89 of 89; tick after the release end-to-end run)

- [ ] **Maintained font shaping crates.** `rustybuzz` (RUSTSEC-2026-0206) and `ttf-parser` (RUSTSEC-2026-0192) are unmaintained and sit in `deny.toml` as a dated exception until 2027-01-06 (owner decision 2026-10-08). Replace them with maintained crates, or renew the exception with a reason, with PDF golden tests unchanged.

**Done when:** each gap is fixed with a regression test, or recorded as an accepted limit with the owner's approval.

## 7. Platform adapter layer for Windows and Mac

Goal: one product, two thin adapters. Feature code never learns which operating system it runs on.

- [x] **Platform boundary with no change to Windows behaviour.** One interface for app folders, open and reveal in file manager, start and stop managed processes, atomic file replace, power events and window integration. (ev: docs/release/evidence/2026-10-06-foundation/INDEX.md#F-15; crate `openframe-platform`, lint R6, `docs/engineering/37-platform-boundary.md`; open and reveal stay in the desktop shell through tauri_plugin_opener, power events and window integration are listed as still to move)
- [ ] **Move every operating-system call behind the boundary,** and add a lint rule that forbids operating-system specific code in feature modules. (lint rule R6 is in place with 10 self-tests; seven allow-listed files remain, listed in `docs/engineering/37-platform-boundary.md` section 6)
- [ ] **Windows adapter** finished and covered by tests.
- [ ] **macOS adapter:** Application Support and Documents locations, open and reveal, process groups, atomic replace, menu bar, Command key mapping. (Needs Mac hardware)
- [ ] **Mac continuous integration** on Apple silicon: format, clippy, Rust tests, web tests, bindings. (Needs Mac hardware)
- [ ] **AI engine on Mac.** Build the engine for Apple silicon with Metal, supervise it with the same security properties as on Windows, sign it inside the app bundle. (Needs Mac hardware)
- [ ] **Mac App Store sandbox compatibility.** App Sandbox entitlements, security-scoped access to project folders the user picks, outgoing network only for the model download (ADR-0026), downloaded model files written to the app container, no downloaded programs, helper process allowed to inherit the sandbox. Test all of it. (Later: store publishing) (Needs Mac hardware)
- [ ] **Web view behaviour on Mac.** Clipboard, input methods, shortcuts, drag and drop, fonts and the screenplay editor tested in WebKit. (Needs Mac hardware) **Superseded (ADR-0027, 2026-10-16 sweep):** the Mac shell is SwiftUI with no web view; Mac input, clipboard and editor behaviour are tested in the SwiftUI shell in the Mac phases.
- [ ] **Project round trip between Windows and Mac.** A project made on one opens, edits and exports identically on the other. (Needs Mac hardware)
- [ ] **Full-journey end-to-end test on Mac.** The same journey as on Windows. (Needs Mac hardware)

**Done when:** the same test suites pass on Windows 11 and on Apple silicon macOS, and the lint rule proves no feature code is operating-system specific.

## 8. AI assistant quality and hardening

- [ ] **Retrieval test set.** A gold set of questions across all domains (`gold_retrieval_cases.jsonl`); a harness that reports Recall at 1, 5 and 10, mean reciprocal rank, nDCG, grounding, leakage and stale results for plain text, vector, hybrid and hybrid plus graph retrieval, using the real embedding model. Without the dev models, build the set and harness and mark the real run as pending. (work started)
- [ ] **Owner approves the quality thresholds** after the first run. Drop the graph stage if it does not beat hybrid.
- [ ] **AI test cases for every feature** (intent, expected tool, entities, permission limit, change behaviour).
- [x] **AI tool list regenerated from the live registry;** every operation has metadata; nothing AI-accessible without an owner. (ev: ai_tool_coverage.rs::every_registered_operation_has_explicit_metadata (every operation has a module, class and a reasoned AI exposure), ::tool_coverage_matches_operation_metadata, ::coverage_matrix_document_is_generated_from_the_registry; `docs/engineering/ai-tool-coverage.md` regenerated from the live registry at `c853632` and after each later merge; docs/release/evidence/2026-10-08-session-2/bundled-offline-ai.md section 3 gates)
- [ ] **Product knowledge file** (`knowledge.md`) covers every new feature.
- [ ] **No-network test.** The offline claim is checked against real network and process behaviour: install, first launch, an offline core workflow, AI use, including the web view. Disable unneeded web view network features after testing. (work started)
- [ ] **Prompt injection and poisoned retrieval collection;** privilege escalation and nested AI tests.
- [ ] **AI runtime inside each package, AI models downloaded after install** (`.exe` and `.dmg` for the current goal, `.msix` and `.app` later). The package holds the `llama.cpp` runtime, OpenMP and Visual C++ runtime DLLs (ADR-0020 decision 1). Gemma 3 1B weights, embedding weights and tokenizer data download on the user's click (ADR-0026). Writable AI data and the downloaded models sit in the user's app data folder, laid out per platform; the app loads under MSIX packaging and the macOS App Sandbox. Reopened 2026-10-15: the 2026-10-08 bundling of the models was reverted by ADR-0026, so no code is done for the download yet.
- [x] **Rebuild the Download OpenFrame AI flow (ADR-0026, ADR-0032).** Settings and first run offer **Download OpenFrame AI** with size and free space shown; the user clicks; the app fetches each file straight from Hugging Face at the commit the compiled-in lock pins (ADR-0032; redirects only to `huggingface.co` or `*.hf.co`), downloads to `*.partial` with HTTP range resume, runs a disk-space preflight, checks each file's size and SHA-256, activates by atomic rename, keeps the last known-good model until the new one passes a health check, and quarantines a bad file. Nothing downloads at install, silently or as a repair. A release build that names the local test host fails without `OPENFRAME_TEST_BUILD=1`. A person who has the files imports them (copied, used only when they match a pin). Pinned hashes and sizes are compiled into the app from the lock (ADR-0032 replaced the signed manifest). Tests cover hash mismatch, interrupted and resumed download, disk full, offline start with the not-installed state, a file gone from the host with manual import, a failed health check rolled back, and the host seeing only the pinned file requests. (ev: openframe-ai bundled.rs 27 flow tests including the_download_installs_checks_and_starts_the_ai, an_interrupted_download_resumes_where_it_stopped, a_damaged_file_is_refused_set_aside_and_never_used, a_file_gone_from_the_host_stops_plainly_and_manual_import_still_works, a_new_set_that_fails_its_health_check_falls_back_to_the_last_good_one, a_download_checked_while_the_app_warms_up_keeps_the_new_set; E2E 76-ai-files-integrity and 78-ai-download green on the gated release executable, docs/release/evidence/w4/README.md, proof level 3)
- [ ] N/A: superseded by the rebuilt download flow in the box above (ADR-0026, ADR-0032). **Remove the download flow and model host code** (2026-10-08, ADR-0025). Reopened 2026-10-15 by ADR-0026: the code was removed, so the download flow is rebuilt in the box above. The 2026-10-08 removal evidence below stays for history. (ev: docs/release/evidence/2026-10-08-session-2/bundled-offline-ai.md section 1 ("Removed"); ai.rs::bundled_ai_needs_no_setup_and_the_download_operations_are_gone; documents changed in reconciliation-log.md entry "the download flow is removed"; E2E 72-ai-assistant.test.mjs "Settings > Offline AI ... offers no download or removal" and 76-ai-bundle-integrity.test.mjs "nothing offers a download", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [ ] **Download OpenFrame AI validation for the internal `.exe`.** On a clean Windows 11 machine: install with no network; download with a network; an interrupted download resumed; a corrupt file refused; offline inference after the download; start with the network off showing the not-installed state; upgrade keeps matching models; uninstall; record package size and hash checks (ADR-0026). (To be done after the download flow is rebuilt)
- [ ] **Download OpenFrame AI validation for the internal `.dmg`.** The same checks on a clean Apple silicon Mac (ADR-0026). (Needs Mac hardware)
- [ ] **Download OpenFrame AI validation for the Microsoft Store `.msix`.** The same checks, with the download and models working under MSIX packaging (ADR-0026). (Later: store publishing)
- [ ] **Download OpenFrame AI validation for the Mac App Store `.app`.** The same checks under the App Sandbox (ADR-0026). (Later: store publishing) (Needs Mac hardware)
- [ ] **Download resilience.** Packet loss, retry codes, disk space check, cancel and resume, corrupt download quarantined. Reopened 2026-10-15 by ADR-0026: part of the download flow to be rebuilt.
- [ ] **Model update regression suite** wired to the benchmark.
- [ ] **AI engine lifecycle.** Crash, hang, timeout, cancel, cleanup of stray engines at start, stop with the app, never run from a project folder. Create the engine suspended, put it in the job object, then resume; refuse to run when that fails; allow one process; re-check the engine file hash before launch. (work started) (2026-10-09: suspended start in the job object, startup sweep and the hash check before launch are built and tested; hang, timeout, refusal when the job object fails and the one-process rule have no named test)
- [ ] **Report AI output.** A button that opens the support channel with the app version and optional user-approved context, and never sends project content on its own. Needs the support address.
- [ ] **Hardware benchmarks.** Minimum (8 GB), recommended, and discrete graphics card; processor-only path; first answer time, tool and JSON reliability, memory, disk. Publish the minimum hardware.
- [ ] **Real-model end-to-end test.** Ask, retrieve, propose, apply or reject, stale, restart, index rebuild. (2026-10-09: specs 72 to 76 pass with the real bundled model; index rebuild is not covered yet)
- [ ] **Gemma redistribution check recorded.** The owner confirms the Gemma terms allow the app to fetch the pinned file from Hugging Face; the same check for the embedding model and runtime; notices in the app and the notices file (ADR-0026 decision 9). (The model host and signing key are no longer needed, ADR-0032.)

- [ ] **Visual C++ runtime app-local.** The Microsoft Visual C++ runtime DLLs the AI engine needs ship next to it in every Windows package (owner decision 2026-10-08), checked on a clean Windows 11 machine with no runtime installed. CPU engine only for the first release.
- [x] **Non-English requests measured.** The gold set includes Telugu and Hindi requests; the report shows how often the right tool and the right scene are found, because tool choice scores English words (`toolbox.rs`, `tools_for_request`) and the embedding model is English (`bge-small-en`). The owner decides what to do after reading the numbers; the model is not changed without approval. Added 2026-10-09. (ev: docs/release/evidence/w5/baseline/report.md "By language" and docs/release/evidence/w6/report.md; the owner chose multilingual E5 small on 2026-10-16, which took retrieval from 202 to 232 of 252 cases, docs/release/evidence/w5/README.md)
- [ ] **macOS engine pinned.** `ai-assets.lock.json` gains an Apple silicon llama.cpp build with sizes and hashes, loaded from `Contents/Resources/ai`; the choice between processor and Metal follows a benchmark and the owner's approval. (Needs Mac hardware) Added 2026-10-09; today only a Windows x64 engine is pinned.

**Done when:** retrieval quality meets the approved thresholds, hardware results are archived, the AI never writes on its own, and no private note or other user's data reaches the AI.

## 9. Beat Board and Idea Vault

- [x] **Long scene description** as a real field, searchable and readable by the AI. Uses the next free migration. (ev: story_cards.rs::long_scene_description_is_a_real_field_saved_undone_and_kept_after_restart, ::long_scene_description_is_searchable_and_leaves_search_when_cleared_or_deleted, ::long_scene_description_is_readable_by_the_assistant_and_editable_only_by_proposal; migration_fixtures.rs::v11_to_v12_adds_long_description_and_research_flag_without_touching_content (project migration 0012); E2E 31-story-board-flow.test.mjs "long scene description and Card Display presets", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Card display settings** (12 toggles) stored as view settings, never as content; changing the layout never changes scene or screenplay text. (ev: story_cards.rs::changing_card_display_never_changes_scene_or_screenplay_data, ::card_display_is_per_user_and_viewers_may_choose_their_own, ::board_cards_carry_what_the_display_toggles_need; BoardView.test.tsx; E2E 31-story-board-flow.test.mjs, green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Presets** Minimal, Story, Production and Custom; expanded card is easy to read. (ev: story_cards.rs::card_display_defaults_to_minimal_and_offers_three_presets_plus_custom; display.rs (unit) presets_differ_and_minimal_is_the_compact_card; BoardView.test.tsx; E2E 31-story-board-flow.test.mjs, green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Notes and attachment limits** defined and tested. (ev: limits in docs/engineering/33-production-depth-contract.md section 1; story_cards.rs::card_capacity_limits_are_enforced_with_clear_messages; story.rs::attachments_on_cards_are_copied_by_duplicate_and_survive_undo)
- [x] **Links stay correct** when screenplay scene identities change. (ev: story_cards.rs::card_links_follow_the_scene_into_a_new_current_draft_and_drop_when_removed; story.rs::deleting_linked_card_keeps_the_screenplay_scene; E2E 31-story-board-flow.test.mjs Build Screenplay step, green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Build Screenplay behaviour** follows your answer on missing scene headings. (ev: story_cards.rs::build_screenplay_asks_for_a_heading_or_leaving_the_card_out_and_never_invents_one; story.rs::fsd_story_020_021_022_build_preview_order_selection_and_missing_headings; BuildDialog.test.tsx; E2E 31-story-board-flow.test.mjs "Build Screenplay: a card without a heading is left out, never invented", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Story Board end-to-end test.** Create, drag, parking lot, convert, build screenplay, episodes, restart, undo. (ev: E2E 31-story-board-flow.test.mjs, 9 tests: create, mouse drag with undo and redo, Parking Lot, convert a beat, Build Screenplay, delete and restore, relaunch, episodes; green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [ ] **Idea Vault end-to-end test.** Voice note, drag-in, global vault, send to Story Board. (2026-10-09: 21-idea-vault-flow.test.mjs passes voice note, Global Idea Vault and Send to Story Board; drag-in from Explorer cannot be driven by WebDriver and is covered only by IdeaVault.test.tsx and idea_vault.rs, so it needs a manual check)
- [x] **Recently Deleted 30-day purge.** Items in project Recently Deleted and Idea Vault Recently Deleted are removed 30 days after deletion (owner decision 2026-10-07, ADR-0012 amendment); the screens say how many days are left; the owner can still purge earlier; the purge is crash-safe, never touches live items, and needs no network. Tests for the boundary day, clock changes and an interrupted purge. (ev: docs/release/evidence/2026-10-08-session-2/session-1-handoffs.md section 5; recently_deleted_purge.rs (7: boundary millisecond, days left, clock back, clock forward, interrupted purge, record pointing at a live object, purge on open and at app start); retention.rs (unit, 2); RecentlyDeleted.test.tsx; DeletedView.test.tsx; E2E 25-recently-deleted-purge.test.mjs, green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)

**Done when:** both workspaces are Release-ready in the feature status file.

## 10. Writing comfort

- [x] **Smart suggestions while typing.** INT., EXT., I/E, character names from the script, Story Board and cast, locations, DAY, NIGHT, LATER, CONTINUOUS, SAME, MOMENTS LATER; type to filter, Enter accepts, Esc dismisses, arrows move, click accepts, Create New; never replaces text silently; no AI model involved. (ev: suggest.test.ts; flow.test.ts; writing_comfort.rs::fsd_script_018_suggestion_sources_come_from_story_cast_and_production; suggestions.rs (unit); E2E 42-writing-comfort.test.mjs "smart suggestions build a scene without retyping and never change text silently" and the two one-Enter tests, green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Custom keyboard shortcuts.** Remap, remove, restore one, restore all, conflict detection, several per command; every command keeps an on-screen route. (ev: shortcuts.test.ts; Shortcuts.test.tsx; writing_comfort.rs::fsd_script_019_shortcuts_remap_remove_restore_and_persist, ::fsd_script_019_damaged_stored_shortcuts_are_ignored_not_applied; E2E 42-writing-comfort.test.mjs "a custom shortcut works in the editor and survives a relaunch; restore all", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Scene navigator columns** the user can show or hide. (ev: writing_comfort.rs::fsd_script_017_navigator_columns_pages_notes_and_absent_production_numbers, ::fsd_script_017_navigator_revision_column_and_production_numbers_when_present; navigatorColumns.test.tsx; E2E 42-writing-comfort.test.mjs "navigator columns can be shown and are remembered", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Proofing.** Project language, on and off switch, underline, personal dictionary, available dictionaries, main and second language. (ev: writing_comfort.rs::fsd_script_020_proofing_settings_are_undoable_validated_and_survive_restart, ::fsd_script_020_check_spelling_with_personal_dictionary_and_indian_scripts, ::fsd_script_020_personal_dictionary_is_personal_bounded_and_persistent; spelling.test.ts; Writing.test.tsx; E2E 42-writing-comfort.test.mjs spelling test, green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8; Windows dictionaries only, the Mac spelling adapter waits for Mac hardware (area 7))
- [x] **Element flow test matrix** with manual override. (ev: flow.test.ts element flow matrix with the manual override; E2E 42-writing-comfort.test.mjs "element flow: Tab moves through the dialogue block and a manual override sticks", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Find and replace scope.** This scene or the whole script, undoable. (ev: writing_comfort.rs::fsd_script_004_replace_all_in_this_scene_only_and_one_undo, ::fsd_script_004_scene_scope_refuses_a_scene_of_another_draft_and_respects_roles; findScope.test.tsx; E2E 42-writing-comfort.test.mjs "find and replace in this scene only, undone in one step", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)

**Done when:** all six are Release-ready and covered by tests.

## 11. Screenplay engine and file formats

Contract: `docs/engineering/32-screenplay-engine-contract.md`. Specification: Screenplay and Production Depth Specification.

**Data and structure**
- [x] **One new migration** (next free number) for custom elements, page-break marks, production number labels, revision records and changed pages, header, footer and page setup. No frozen migration edited. (ev: migrations/project/0013_screenplay_engine.sql; migration_fixtures.rs::screenplay_engine_migration_keeps_elements_and_defaults; `node scripts/check-migrations.mjs` in the governance gate (no frozen migration changed))
- [x] **Element rule table** shared by the editor, page layout and FDX mapping. (ev: openframe-import-export/src/rules.rs; element_rules.rs; editor.test.ts "shared element rule table"; import-export screenplay_engine.rs::defaults_reproduce_the_standard_layout)
- [x] **Operations registered** with metadata, permissions, undo, search and an AI decision for every new operation. (ev: docs/engineering/32-screenplay-engine-contract.md section 2.5; ai_tool_coverage.rs::every_registered_operation_has_explicit_metadata; screenplay_engine.rs::page_setup_is_saved_with_the_screenplay_undone_and_survives_restart (undo, Viewer refused), ::changing_a_number_used_in_production_needs_acknowledgement, ::ai_reads_page_facts_but_never_changes_layout_or_numbers)
- [x] **Several screenplays per project** (ADR-0012 amendment 2026-10-08). Data check done 2026-10-08: the schema allows several screenplays, so no migration is needed; the limit sits in four Rust guards, three `LIMIT 1` lookups and the frontend (list in `reconciliation-log.md`). Remove the guards, require a screenplay id in those lookups, add a screenplay picker wherever Story Board, Breakdown, Schedule, Call Sheets and exports read a screenplay, and an upgrade test that an older project opens with its one screenplay and loses nothing. (ev: docs/release/evidence/2026-10-08-session-2/several-screenplays.md; screenplay.rs::several_screenplays_keep_their_own_drafts_locks_numbers_and_page_setup, ::screenplays_are_renamed_with_unique_titles_and_undo; migration_fixtures.rs::a_project_from_before_several_screenplays_opens_with_its_one_screenplay_intact; ai_toolbox.rs::several_screenplays_are_named_and_never_picked_silently; screenplay_delete.rs (7); picker.test.tsx; E2E 43-several-screenplays.test.mjs, green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)

**Writing features**
- [x] **Title page** fields, preview and export switch. (ev: screenplay_engine.rs::title_page_fields_preview_and_export_switch; import-export screenplay_engine.rs::long_title_page_lines_are_wrapped_inside_the_page; E2E 41-screenplay-engine.test.mjs "title page fields with a live preview", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Header and footer** with placeholders and a first-page rule. (ev: import-export screenplay_engine.rs::header_footer_placeholders_and_first_page_rule, ::a_long_header_never_runs_into_the_page_number; pagesetup.test.tsx; E2E 41-screenplay-engine.test.mjs "page setup: A4 with a header", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Page setup presets** (Letter, A4, saved preset, font, size, line spacing), saved with the project. (ev: screenplay_engine.rs::page_setup_is_saved_with_the_screenplay_undone_and_survives_restart; import-export screenplay_engine.rs::a4_keeps_columns_and_gains_lines_per_page, ::font_size_and_spacing_change_the_page_grid; pagesetup.test.tsx; E2E 41-screenplay-engine.test.mjs, green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Explicit page break** that can be inserted, removed, undone and exported. (ev: screenplay_engine.rs::explicit_page_break_insert_remove_undo_and_export; import-export screenplay_engine.rs::page_breaks_round_trip_through_fdx_and_fountain; commands.test.ts; E2E 41-screenplay-engine.test.mjs, green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Custom elements** under Advanced: name, shortcut, look, next element, import and export identity. (ev: screenplay_engine.rs::custom_elements_define_use_export_reimport_and_protect, ::hostile_fdx_with_thousands_of_look_alike_custom_styles_is_bounded; commands.test.ts; E2E 41-screenplay-engine.test.mjs "a custom element under Advanced, used in the script", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [ ] **Production scene numbers.** Imported, alphanumeric, frozen when the script is locked, with a dependency warning and undo. (2026-10-09: built with Rust tests in screenplay_engine.rs; no end-to-end step uses the Scene number dialog yet)
- [x] **Lock shooting draft and revisions.** Revision colour and reason; changed scenes and pages compared with the locked pages. (ev: screenplay_engine.rs::revisions_mark_changes_and_list_changed_pages_and_scenes; screenplay.rs::fsd_script_034_035_locked_draft_rejects_edits, ::fsd_script_036_037_revision_leaves_locked_draft_unchanged; lock.security.test.ts; E2E 41-screenplay-engine.test.mjs "lock, start a dated revision, edit it and see the changed pages", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)

**Layout and output**
- [x] **One page layout** shared by the editor preview, print preview and PDF, with tests that they agree. (ev: screenplay_engine.rs::page_view_print_preview_pagination_and_pdf_agree; import-export screenplay_engine.rs::layout_is_deterministic, ::print_preview_and_pdf_agree_on_letter, ::print_preview_and_pdf_agree_on_a4_with_header_and_footer)
- [x] **Keep-together rules** with tests. (ev: import-export screenplay_engine.rs::heading_stays_with_a_speech_cue_and_two_lines, ::character_cue_is_never_left_alone, ::parenthetical_stays_with_the_dialogue_it_introduces, ::split_paragraphs_leave_at_least_two_lines_on_each_page; layout.rs (unit) heading_is_never_orphaned)
- [x] **Dialogue continuation** generated from structure; one split block is different from two blocks. (ev: import-export screenplay_engine.rs::one_split_speech_differs_from_two_speeches; layout.rs (unit) long_dialogue_splits_with_more_and_contd; pdf.rs::split_dialogue_is_rejoined_on_import)
- [x] **Embedded fonts and shaping** for Telugu, Hindi, Tamil, Malayalam, Kannada, English and mixed text, with byte-identical golden output (ADR-0016). (ev: ADR-0016; pdf_golden.rs::golden_multilingual_scripts_shaped_with_embedded_fonts; fonts_manifest.rs; exports.rs::telugu_and_hindi_print_in_every_pdf_export; E2E 41-screenplay-engine.test.mjs "Telugu and Hindi print in the PDF", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Multilingual round trip.** Type, save, reload, PDF, FDX, Fountain, DOCX, search and AI retrieval keep every character. (ev: screenplay_formats.rs::multilingual_text_survives_save_restart_search_retrieval_and_every_format; multilingual.rs::every_format_keeps_every_character_of_a_mixed_indian_script; E2E 41-screenplay-engine.test.mjs, green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Screenplay files carry no OpenFrame branding** (test). (ev: import-export screenplay_engine.rs::screenplay_files_carry_no_openframe_branding; exports.rs::every_production_export_is_branded_and_user_files_are_never_modified (unbranded screenplay PDF case); E2E 62-call-sheets-exports.test.mjs unbranded screenplay PDF step, green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)

**File formats**
- [x] **FDX import.** Preview with counts and warnings; custom elements preserved, mapped or sent to review; scene numbers, title data and revisions kept; unknown parts never flattened. (ev: fdx_suite.rs::thirteen_fdx_classes_parse_export_reimport_and_print_as_expected; interchange.rs::fsd_script_038_fdx_import_creates_new_screenplay_with_new_identities; security_parsers.rs::fdx_custom_style_flood_is_bounded_deduplicated_and_keeps_every_line; E2E 41-screenplay-engine.test.mjs "export PDF and FDX, import the FDX back as a draft", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [ ] **FDX export** is valid, re-importable and opens in Final Draft (manual check on a licensed copy with screenshots).
- [x] **Thirteen test script classes** each with the source, parsed result, exported result, re-imported result and PDF snapshot. (ev: tests/fixtures/fdx-suite/; fdx_suite.rs::thirteen_fdx_classes_parse_export_reimport_and_print_as_expected, ::each_class_exercises_its_construct)
- [x] **Fountain, DOCX, PDF import and text paste.** Fountain fidelity table and round trip; DOCX limits documented and tested; PDF import is best effort and fails safely; plain text paste works. (ev: fountain.rs::fountain_round_trip_is_stable; docx.rs::docx_round_trip_keeps_readable_screenplay; pdf.rs::scanned_or_broken_pdfs_are_rejected_with_the_standard_message; txt.rs::pasted_text_is_recognised_with_honest_warnings; fidelity table and DOCX limits in docs/engineering/32-screenplay-engine-contract.md section 2.4; E2E 01-journey.test.mjs "import pasted Fountain", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Exact import report;** a failed export leaves the source and project untouched and removes partial output. (ev: interchange.rs::failed_imports_change_nothing, ::export_destination_is_validated_and_failures_leave_no_file; exports.rs::a_failed_write_leaves_the_previous_file_and_no_partial_file; screenplay_engine.rs::a_file_with_more_custom_styles_than_fit_imports_with_every_line_and_a_warning)
- [ ] **Migration test for Final Draft users** passes all eleven steps in the specification. (2026-10-09: screenplay_formats.rs::final_draft_migration_definition_of_done covers ten steps; step 8, reopening in Final Draft, is a manual owner check)
- [x] **Screenplay end-to-end test.** Write, element flow, title page, revisions, lock, export PDF and FDX, import FDX, continue, restart. (ev: E2E 41-screenplay-engine.test.mjs, 8 tests: element flow, page break, title page, page setup, custom element, lock and revision, PDF and FDX export, FDX import, relaunch and continue; green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)

**Done when:** the specification's acceptance list is green, performance is inside the budget, and every new operation has its AI work finished.

## 12. Appearance and themes

Specification: section on appearance themes in the Screenplay and Production Depth Specification. Engineering: `docs/engineering/30-appearance-themes.md`. Needs the themes decision (ADR-0014).

- [x] **Colour clean-up.** Every colour in the UI classed as theme colour, deliberately fixed colour or document colour; theme colours replaced by named tokens. (ev: docs/engineering/30-appearance-themes.md section 7; architecture lint rule T1 and its governance.test.mjs cases; engine.test.ts "defines every token and role the screens use"; E2E 91-appearance-themes.test.mjs, green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Default look unchanged.** The default theme matches the saved baseline screenshots, except the six Light-mode pairs raised to 4.5 to 1 (owner decision 2026-10-07, ADR-0014 amendment), whose baselines are regenerated with that reason. (ev: theme.rs (unit) contrast_every_builtin_theme_passes_in_light_and_dark (the six Light-mode pairs raised, ADR-0014 amendment); baselines regenerated with a reason per screen in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 3; E2E 90-visual-baseline.test.mjs 15 of 15 screens, 0 pixels differ, green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Light, Dark and System modes** that follow the system live, with no restart. (ev: engine.test.ts "System follows the operating system live, without a restart"; appearance.rs::mode_and_theme_survive_a_restart; AppearancePanel.test.tsx; E2E 60-appearance.test.mjs, 91-appearance-themes.test.mjs relaunch test, green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **App database migration** for preferences and My Themes; a broken or deleted theme falls back to the default. (ev: app migration 0002_appearance.sql; appearance.rs::a_damaged_or_missing_saved_theme_falls_back_to_default; migration_fixtures.rs::every_app_database_version_upgrades_and_keeps_recents_and_settings; E2E 60-appearance.test.mjs, green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Fifteen built-in themes** from one list; adding one touches no screen code. (ev: themes.json; theme.rs::the_fifteen_theme_ids_are_the_reference_list; appearance.rs::every_builtin_theme_can_be_applied; engine.test.ts "adding a theme is one list entry"; E2E 91-appearance-themes.test.mjs, green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Theme import.** File or paste, 10 KB limit, strict format, id rule, bounded values, safe overrides only, preview, contrast check; hostile files change nothing. (ev: appearance.rs::security_hostile_theme_files_are_refused_and_change_nothing (57 inputs), ::the_ten_kilobyte_limit_is_exact, ::a_corrected_import_always_survives_being_read_back; AppearancePanel.test.tsx; E2E 60-appearance.test.mjs, green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **My Themes.** Apply, rename, export, delete. (ev: appearance.rs::rename_export_reimport_and_delete; AppearancePanel.test.tsx; E2E 60-appearance.test.mjs "mode, theme, import, rename, restart and delete", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Contrast tests** (4.5 to 1) for every theme in Light and Dark; High Contrast is usable. (ev: theme.rs (unit) contrast_every_builtin_theme_passes_in_light_and_dark, contrast_high_contrast_keeps_seven_to_one_for_main_and_muted_text; E2E 91-appearance-themes.test.mjs rendered text in 16 themes x 2 modes x 15 screens, green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Screenplay PDF and printed call sheet paper never change** with the theme (test). (ev: exports.rs::screenplay_pdf_and_call_sheet_paper_never_change_with_the_theme; engine.test.ts "keeps the screenplay page and call-sheet paper on fixed document colours"; E2E 91-appearance-themes.test.mjs "paper stays white", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Visual tests** for 15 screens across built-in themes and both modes, plus a restart test. (ev: E2E 91-appearance-themes.test.mjs "16 themes x 2 modes x 15 screens are applied and readable; paper stays white" and "the theme and mode come back after a relaunch"; 90-visual-baseline.test.mjs for the default theme; green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)

**Done when:** the appearance features are Release-ready and the default look is pixel-compatible with the baseline.

## 13. Breakdown, catalog, locations, cast and crew

- [x] **Category colour, text, icon and tooltip** for the eleven categories, with Advanced customisation and an accessibility test. (ev: production_depth.rs::eleven_categories_have_colour_text_icon_and_tooltip_with_advanced_customisation; categories.rs (unit); breakdown.test.tsx (colour together with icon, text and tooltip); BreakdownCategories.test.tsx; E2E 51-breakdown-depth.test.mjs, green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Bulk actions.** Accept and reject selected, quick reuse, filters (unreviewed, suggested, missing), scene completion state, one undo step per batch. (ev: production_depth.rs::accept_and_reject_selected_are_one_undo_step_per_batch, ::quick_reuse_offers_neighbouring_items_and_reuse_is_one_step_for_many_scenes, ::filters_unreviewed_suggested_missing_completion_category_and_catalog_item; breakdown.test.tsx; E2E 51-breakdown-depth.test.mjs, green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [ ] **Jump to source phrase,** catalog item to all its scenes, short breakdown report. (2026-10-09: Show in script and catalog item to all scenes pass in 51-breakdown-depth.test.mjs; the short breakdown report has Rust tests only, no end-to-end step)
- [x] **Script changes never silently delete confirmed data** (test). (ev: production_depth.rs::script_changes_never_silently_delete_confirmed_breakdown_data; production.rs::fsd_brk_016_017_018_production_source_update_never_deletes_breakdown_data; e2e_flows.rs::e2e_script_revision_after_production_source_flags_everything_and_deletes_nothing)
- [x] **Locations** hold address, access, parking, nearest hospital, emergency contact, availability, notes and photos. (ev: production_depth.rs::locations_hold_call_sheet_fields_availability_notes_and_photos; production.rs::locations_fields_notes_photos_and_trash; depth.test.tsx; E2E 51-breakdown-depth.test.mjs "location call-sheet fields, availability dates, and crew availability", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Cast and crew** hold contact, availability, notes and archive, with privacy rules for contact data. (ev: production_depth.rs::cast_and_crew_hold_contact_availability_notes_and_archive, ::contact_data_is_hidden_by_role_from_the_assistant_and_from_exports; diagnostics.rs::core_015_bundle_contains_no_content_paths_contacts_prompts_or_keys; depth.test.tsx; E2E 51-breakdown-depth.test.mjs, green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Stable identity.** Rename keeps links; delete and references are explicit; search covers the intended fields. (ev: production_depth.rs::renames_keep_links_and_deletes_name_their_references; production.rs::catalog_crud_archive_delete_and_search)
- [ ] **Large-project performance budgets** for breakdown filters and opening. (2026-10-09: filters measured with `npm run perf` on the area 13 branch on 2026-10-07; opening is not measured and `npm run perf` was not run on the joined code)
- [x] **End-to-end test** for breakdown, catalog, locations, cast and crew. (ev: E2E 50-breakdown.test.mjs (6 tests) and 51-breakdown-depth.test.mjs (6 tests), both with a relaunch, green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)

**Done when:** all are Release-ready.

## 14. Shooting schedule and Day Out of Days

Contract: `docs/engineering/33-production-depth-contract.md`. Extend the existing tables; do not replace them.

- [x] **Day timeline.** Crew call, shooting start and target wrap on each shooting day. (ev: docs/release/evidence/2026-10-08-session-2/area-14.md section 1; schedule_depth.rs::sched_003_day_times_are_stored_undoable_and_validated, ::night_shoot_wraps_after_midnight; migration_fixtures.rs::v14_to_v15_keeps_an_existing_schedule_and_adds_empty_day_times; E2E 61-schedule.test.mjs "a meal break, day times and the computed timeline", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Scene start time and computed timeline.** Meal, travel, company move and custom markers feed the timeline and day total. (ev: docs/release/evidence/2026-10-08-session-2/area-14.md section 1; schedule_depth.rs::sched_004_scene_starts_and_breaks_feed_the_timeline_and_day_total; Strip.test.tsx; E2E 61-schedule.test.mjs, green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Availability warnings** for cast, locations, missing location, over target, and travel pushing the day over; advice only. (ev: docs/release/evidence/2026-10-08-session-2/area-14.md section 1; schedule_depth.rs::sched_005_over_target_and_travel_warnings_are_advice_only, ::sched_005_cast_and_location_availability_warn_on_dated_days_only, ::missing_location_still_warns; E2E 61-schedule.test.mjs "conflicts are advice: Keep anyway", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Filters** by location, cast, day or night, length, scheduled, breakdown complete, day and revision; filters never change data. (ev: scheduleFilter.test.ts "match on every dimension of spec section 27", "never changes the schedule it filters"; E2E 61-schedule.test.mjs "filters only change what is shown", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **No automatic planner.** A test proves nothing ever moves a scene without a user action. (ev: schedule_depth.rs::sched_009_nothing_moves_a_scene_without_a_user_move; schedule.rs::fsd_sched_014_grouping_suggestions_need_explicit_confirmation; E2E 61-schedule.test.mjs "grouping suggestions move nothing", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Stale and review state** for source changes; call sheet stale flag still correct. (ev: schedule_depth.rs::sched_007_call_sheet_stale_flag_follows_day_times_and_starts, ::sched_007_script_change_review_keeps_new_fields, ::migration_0015_upgrades_an_existing_schedule_without_marking_call_sheets_stale; E2E 61-schedule.test.mjs "call sheet stale review follows the day times", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Day Out of Days.** Built from cast assignments and the schedule; cast and day range selection; manual correction; PDF and spreadsheet export; read-only to screenplay and breakdown. (ev: docs/release/evidence/2026-10-08-session-2/area-14.md section 1; schedule_depth.rs::dood_001_derived_from_cast_and_schedule_with_ranges_and_corrections, ::dood_corrections_and_day_times_survive_restart, ::dood_correction_notes_are_searchable_on_their_day; dood.test.tsx; E2E 61-schedule.test.mjs "Day Out of Days: derived, corrected by hand, undo", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Schedule exports** (PDF and spreadsheet) are reproducible. (ev: schedule_depth.rs::sched_008_schedule_and_dood_exports_are_byte_reproducible; E2E 61-schedule.test.mjs "exports: dialogs open, files are byte-reproducible", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Schedule end-to-end test.** Create, drag, markers, off days, conflicts, grouping, stale review, export, restart, undo. (ev: E2E 61-schedule.test.mjs, 9 tests: create, drag with undo and redo, off day, markers and day times, conflicts, grouping, filters, stale review, Day Out of Days correction, exports, relaunch; green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)

**Done when:** all are Release-ready.

## 15. Call sheets, budget, templates, samples and exports

- [x] **Call sheet content.** Header, day times, weather, sunrise and sunset, locations, scene table with start times and travel rows, talent, extras, department notes, advance schedule, crew by department, communication, practical notes, attachments; unknown values show "Needs input". (ev: docs/release/evidence/2026-10-08-session-2/area-15.md section 1; schedule.rs::fsd_call_001_call_sheet_generated_from_shooting_day; pdf_regression.rs::call_sheet_pdf_matches_its_golden; CallSheetEditor.test.tsx; E2E 62-call-sheets-exports.test.mjs "create a call sheet: six default sections, Needs input, Add Section", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Six default sections;** the rest through "Add Section"; empty optional sections stay hidden. (ev: schedule.rs::add_section_is_searchable_undoable_and_needs_edit_rights; CallSheetEditor.test.tsx "shows the six default sections and hides optional ones that were not added"; E2E 62-call-sheets-exports.test.mjs, green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Update preview** lists changed scenes, cast calls and locations; finalized sheets stay unchanged snapshots; status words follow your answer. (ev: schedule.rs::fsd_call_009_schedule_change_marks_call_sheet_needs_refresh, ::fsd_call_011_finalized_call_sheet_is_immutable_and_revised; migration_fixtures.rs::v15_to_v16_keeps_amounts_as_planned_and_maps_call_sheet_status_words; E2E 62-call-sheets-exports.test.mjs "a schedule change marks the sheet Needs refresh" and "finalize freezes a snapshot", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Editing a call sheet never changes** the screenplay, breakdown or schedule (test). (ev: schedule.rs::call_sheet_operations_never_change_screenplay_breakdown_or_schedule, ::fsd_call_002_call_sheet_edits_never_change_the_schedule)
- [x] **Lightweight budget** with Planned and Actual, category totals, difference and report; existing amounts become Planned. (ev: exports.rs::budget_report_exports_planned_actual_and_difference; pdf_regression.rs::budget_pdf_matches_its_golden; budget.test.tsx; migration_fixtures.rs::v15_to_v16_keeps_amounts_as_planned_and_maps_call_sheet_status_words; E2E 62-call-sheets-exports.test.mjs "budget keeps Planned and Actual with the difference", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Sides, six production reports, moodboard, storyboard sheet and shot list exports.** (ev: exports.rs::sides_pdf_contains_only_the_days_scenes_in_screenplay_format, ::reports_export_live_and_saved_in_every_format, ::moodboard_pdf_embeds_images_and_excludes_internal_notes_unless_selected, ::storyboard_sheet_has_panels_shot_numbers_framing_and_optional_notes, ::shot_list_exports_by_scene_selection_or_whole_project; E2E 62-call-sheets-exports.test.mjs "sides, a report and the visual planning exports carry the brand line" (renamed 2026-10-27: sides no longer carry it, owner decision), green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Subtle OpenFrame branding** on every non-screenplay export; spreadsheets use metadata or a footer; user files are never modified (test per export). (ev: exports.rs::every_production_export_is_branded_and_user_files_are_never_modified (18 export and format cases); report.rs::branded_pdfs_and_workbooks_name_openframe_and_plain_ones_do_not; E2E 62-call-sheets-exports.test.mjs, green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **PDF regression tests** for call sheet, schedule, Day Out of Days, breakdown, budget and sides. (ev: pdf_regression.rs (7 tests, goldens in tests/fixtures/pdf-regression/ for call sheet, schedule, Day Out of Days, breakdown, budget and sides; compares page sizes and every text run with its position, not pixels))
- [x] **Built-in templates** per your answer; using a template makes an independent copy and later template edits never change existing documents (test). (ev: notes.rs::the_eight_built_in_templates_follow_the_owner_decision, ::using_a_template_makes_independent_documents_in_one_undo_step, ::a_template_use_that_fails_leaves_nothing_behind; E2E 62-call-sheets-exports.test.mjs "a call sheet from the Indie Call Sheet template is an independent copy", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Sample scripts** (Example Feature, Example Short, Example Scene, FDX Interchange Test) shipped as assets and used as regression fixtures; copyright note per file. (ev: openframe-import-export/tests/samples.rs (6: embedded bytes, copyright note per file, scene counts, round trips, unbranded PDF); E2E 62-call-sheets-exports.test.mjs "open a sample script that ships with the app", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **End-to-end test** for call sheets, sides, reports, visual planning, notes and tasks. (ev: E2E 62-call-sheets-exports.test.mjs, 10 tests including notes and tasks and a relaunch, green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)

**Done when:** all are Release-ready.

## 16. Packages and backup

- [x] **Exchange, Review and Response packages.** Export, open, response, version mismatch, unmatched notes, preview before apply, identity collision, role-scoped content. (ev: docs/release/evidence/2026-10-08-session-2/area-16.md section 1; packages.rs::fsd_col_001_exchange_package_can_be_inspected_in_another_installation, ::fsd_col_002_stale_response_is_imported_as_comments_only_without_changing_text, ::exchange_identity_collision_never_overwrites_another_project, ::package_roles_scope_what_each_member_can_send_and_apply; ReviewViewer.test.tsx; E2E 70-packages.test.mjs "review round trip", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [x] **Backup and whole-project package.** Create, validate, restore on a new machine, corrupt backup detected, interrupted import. (ev: docs/release/evidence/2026-10-08-session-2/area-16.md sections 1 and 6; packages.rs::backup_restores_on_a_new_computer_with_a_fresh_data_root, ::corrupt_backups_are_detected_and_leave_nothing_behind, ::interrupted_restore_leaves_nothing_behind, ::restored_private_notes_belong_to_the_restorer_and_stay_private; E2E 70-packages.test.mjs backup, restore as a copy, damaged backup refused, relaunch, green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [ ] **Packages carry the new data** (production numbers, custom elements, schedule timeline, budget Planned and Actual) with a format version bump and compatibility tests. (2026-10-09: production numbers and custom elements travel with format 2 and compatibility fixtures; the schedule timeline and budget Planned and Actual are listed in package coverage but no test proves they travel)
- [x] **No network listener anywhere** (test), consistent with ADR-0007; LAN removed from user-facing text. (ev: packages.rs::the_core_workflow_opens_no_network_listener, ::the_desktop_app_depends_on_no_network_server_crate, ::no_lan_collaboration_wording_in_user_facing_text, ::the_lan_wording_check_catches_offers)
- [ ] **Packages end-to-end test and restore drill.** (2026-10-09: 70-packages.test.mjs passes; the manual restore drill on a second Windows account is not done)

**Done when:** both workflows are Release-ready.

## 17. Product-wide quality, accessibility and beta

- [ ] **Manual figure check.** Each of the 176 manual figures mapped to a feature and a test or manual result; the five LAN figures marked removed.
- [ ] **Project Home** shows the production route and the next shooting day or latest call sheet.
- [ ] **Accessibility.** Keyboard-only walkthrough of every workspace, visible focus, labelled icon buttons, no meaning by colour alone, Windows High Contrast, Narrator, automated checks in component tests, report archived.
- [ ] **High resolution, several monitors, input methods and Unicode input** checked on Windows and on Mac. (Needs Mac hardware)
- [ ] **Performance report** on minimum and recommended hardware for launch, open, save, typing, reflow, search on 500, 2,000 and 10,000 scene projects, exports, AI retrieval and index rebuild, all inside the budgets.
- [ ] **Second security review** of IPC, file scope, parsers, packages, themes, AI tools, process lifecycle, and the absence of an updater in store builds; threat model updated; all high findings fixed with tests.
- [ ] **Offline run** of the full core workflow with the network disabled, with a traffic capture.
- [ ] **Soak run on a release candidate.** Hours, repeated sleep and resume, repeated project switching.
- [ ] **Beta.** 8 to 15 real writers, screenwriters and production team members use a release candidate for 3 to 4 weeks; feedback comes through the support channel; serious findings fixed; summary archived. (Later: store publishing)
- [ ] **English-only check.** User text is kept in one place so translation can be added later, or the exception is recorded.
- [ ] **User manual rebuilt** from the shipped UI; LAN figures removed; Offline AI and new features described; the manual source and how to rebuild it are stored.
- [ ] **Remaining LAN text removed** from the specification bodies, the security specification and the UX mock-up references.
- [ ] **In-app help and public documentation outline** complete.
- [ ] **Beta and review defects closed;** a known-issues list written.
- [ ] **Internal tester pilot.** Writers and production teams use the internal `.exe` and `.dmg` as their main tool for 2 to 3 days, following [the internal test plan](../testing/internal-test-plan.md); every report has a severity, every Blocker is fixed and re-checked, and the summary is archived. This is the Chapter 1 test; the long beta above stays a Chapter 3 step. Added 2026-10-09.

**Done when:** accessibility, performance, security, soak and beta reports are archived and every Must ship feature is Release-ready.

## 18. Payment and free trial in all three channels

Business model: paid, with a 7-day free trial and no credit card to start. One shared entitlement interface in the Rust core; each channel has its own adapter.

- [x] **Shared entitlement interface.** States: trial active with days left, trial ended, purchased, refunded or revoked, unknown because offline. The app never blocks opening or exporting existing projects when the state is unknown. (ev: docs/release/evidence/2026-10-06-foundation/INDEX.md#F-16; crate `openframe-entitlement`, `entitlement.status` and `entitlement.restore_purchase`, Licence card in Settings; `docs/engineering/36-entitlement-and-trial.md`)
- [ ] **Trial rules** written down: exactly 7 days, no credit card, behaviour offline, after reinstall, after changing the clock, after a refund. (written in `docs/engineering/36-entitlement-and-trial.md` section 3; the rule after a reinstall waits on the owner) (Later: store publishing)
- [ ] **Microsoft Store adapter.** The Store runs the 7-day trial (Partner Center); the app reads `StoreContext.GetAppLicenseAsync` (`IsActive`, `IsTrial`, `TrialTimeRemaining`) at launch and on licence change; trial over and not bought gives the full block with only Buy and Restore (ADR-0023); purchase through the Store purchase window; restore purchase; behaviour during a Store outage. (Later: store publishing)
- [ ] **Mac App Store adapter.** StoreKit purchase and trial method you choose; restore purchases; confirm against Apple's current rules when building. (Later: store publishing) (Needs Mac hardware)
- [x] **Internal installer build.** The `.exe` has no trial, licence, payment, Buy button, reminders or lock, and says "Internal build, not for sale" in About and Settings > Licence (ADR-0023); the old installer licence and local trial code is removed. (ev: docs/release/evidence/2026-10-08-session-2/session-1-handoffs.md sections 3 and 4; entitlement.rs::plat_009_the_internal_installer_build_is_never_sold_and_never_locks; About.test.tsx; Licence.test.tsx; E2E 15-trial-first-run.test.mjs "the internal build shows no chip, no Buy and no popup" and 16-about.test.mjs, green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8; proven on the internal channel of the debug build, the installed `.exe` is checked under area 20)
- [x] **No price in the app.** No screen, reminder, error, tutorial or in-app text mentions a price, currency or pricing model; Buy opens the store's own purchase window; a test fails on price wording (ADR-0023). (ev: licensing.test.tsx price wording scan; E2E 15-trial-first-run.test.mjs "Mac first launch cannot be closed and shows no price", green in docs/release/evidence/2026-10-08-session-2/e2e-session-2.md section 8)
- [ ] **No data loss on expiry.** Mac App Store and Microsoft Store: the app is fully blocked after the trial ends or a refund, with only Buy and Restore (ADR-0022, ADR-0023), and the user's files are never changed or deleted. (earlier ev: docs/release/evidence/2026-10-06-foundation/INDEX.md#F-17 covered the old rule only; unticked 2026-10-07 until the Mac block and its Rust refusal tests are merged) (Later: store publishing) (Needs Mac hardware)
- [ ] **Tests for every state change** and for tampering: clock changes, copied licence files, offline use, outage. (clock changes, edited record, locked and unwritable files, forged licence and store outage are covered by 29 crate tests and 12 application tests; a licence copied to another computer still counts as a purchase until the owner decides on machine binding) (Later: store publishing)
- [ ] **Refund and cancellation text** for each channel; privacy policy states exactly what payment data each channel handles. (Later: store publishing)
- [ ] **Prices, regions and trial details** entered in both stores (Partner Center: 7-day trial) and on the website, and checked to agree; the app itself never shows a price. (Later: store publishing)

**Done when:** purchase, trial, restore and expiry work in each channel and tests cover every state.

## 19. Microsoft Store package

- [ ] **Store build.** `OPENFRAME_DISTRIBUTION=store`; in-app updater compiled out; `release-validate --store` passes with no bypass. (Later: store publishing)
- [ ] **Package manifest.** Minimum version Windows 11 (`10.0.22000.0`); only `runFullTrust` and `internetClient` with written justification; 77 visual assets valid; version `X.Y.Z.0`, higher than any earlier submission. Listing text says Windows 11. (Later: store publishing)
- [ ] **Release candidate** built by the real pipeline: package, build record, third-party notices, software bill of materials, checksums. The approved candidate never changes. (Later: store publishing)
- [ ] **Licence inventory complete,** including bundled fonts, the AI engine and model notices (Gemma notice in the app and the notices file).
- [ ] **Packaged app data folder** works (`LocalState`); logs can be opened; uninstall keeps user projects and removes app data as the policy says. (Later: store publishing)
- [ ] **Windows App Certification Kit** run against the exact package; all green; report archived. (Later: store publishing)
- [ ] **Clean-machine matrix on Windows 11** (clean virtual machine or Windows Sandbox, no developer tools): install, first launch, create project, write, save, close, reopen, upgrade, uninstall, reinstall, projects survive; web view runtime present and missing. (Later: store publishing)
- [ ] **First launch on a bare machine** needs nothing installed locally. (Later: store publishing)
- [ ] **Installed-package end-to-end test.** Full journey and Offline AI (download, signature and hash verified, start, ask, apply, offline use after download; ADR-0026) on the installed package. (Later: store publishing)
- [ ] **Windows Defender and antivirus** checked on the packaged build, including AI engine start. (Later: store publishing)
- [ ] **Open and migrate every earlier project version** with the test collection, on the packaged build. (Later: store publishing)
- [ ] **Hardware results** recorded for the listing. (Later: store publishing)
- [ ] **Upgrade test** from one package version to the next keeps settings, projects and rebuilds the AI index. (Later: store publishing)
- [ ] **Binary hardening gate.** The release pipeline checks the executable (BinSkim or `dumpbin`) and fails on missing randomisation, data execution prevention, control-flow guard or CET, an embedded debug path, or a manifest that is not `asInvoker`; record the MSI install scope decision. (Later: store publishing)
- [ ] N/A: superseded 2026-10-16 by ADR-0032 (no key, no host setting; release check 6a and `build.rs` refuse the test host instead). **Release build fails to compile** when the AI list public key or the model host setting is not set (ADR-0026). Reopened 2026-10-15: to be done with the download flow. (Later: store publishing)

**Done when:** one unchanged package has passed the certification kit, the clean-machine runs and the installed-package test.

## 20. Windows installer

- [ ] **Installer build.** NSIS `.exe` (and MSI if kept), per-user install, no administrator needed, EULA page shows the real licence.
- [ ] **Signing.** Code-signing certificate, timestamp, signature verified independently on a clean machine. (Later: store publishing)
- [ ] **Updater with approval.** Check, tell the user, user approves, download, verify the signature, close the project safely, update, restart; failed updates leave the old version working.
- [ ] **Uninstall keeps projects;** reinstall finds them.
- [ ] **Clean-machine tests** for install, first launch, upgrade, uninstall, reinstall, with and without the web view runtime; SmartScreen behaviour recorded.
- [ ] **Team password gate.** Internal `.exe` and `.dmg` ask the team password on every launch before anything opens; 5 wrong tries wait 1 minute, doubling to 30 minutes, count kept across restarts; Argon2id hash with a random salt from a build-machine environment variable, constant-time check in Rust; the build fails without the variable; store packages carry no check and no hash; tests for right, wrong, lockout, restart and a missing variable (ADR-0024). (2026-10-09: built and tested in team_gate/tests.rs (9) and team_gate.rs (4); no internal release build was run with a test password, and the build failure without the variable is proven only by a unit test)
- [ ] **Internal Mac disk image.** Unsigned `.dmg` for now (owner decision 2026-10-08), hardened runtime, team password gate on, AI runtime in the package and the model download after install (ADR-0026); installs and runs on a clean Apple silicon Mac after the documented Gatekeeper override; no trial, payment or store lock (ADR-0024). Reworded 2026-10-08: signing moved to the next box. (Needs Mac hardware)
- [ ] **Signing of the internal Mac disk image.** `.dmg` signed with Developer ID, notarized and stapled; Gatekeeper accepts it on a clean Apple silicon Mac (moved from the box above on 2026-10-08; ADR-0024 amendment). (Later: store publishing) (Needs Mac hardware)
- [ ] **Team distribution.** The internal file (unsigned for now) reaches the owner's team with a published checksum; it is not offered on the public website (ADR-0023).
- [ ] **Binary hardening gate** applies to this file too.
- [ ] **Internal `.exe` build with the team gate on, unsigned.** Built on the owner's build PC with `OPENFRAME_TEAM_PASSWORD` set by the owner (nobody else sees or stores it), unsigned (owner decision 2026-10-08), team password gate on, AI runtime in the package and the model download after install (ADR-0026), no trial, payment or store lock; installs, launches, asks the password and runs the full journey on a clean Windows 11 machine.
- [x] **Release-build smoke test.** One end-to-end spec drives the release executable (not the debug test build): start, create, type, close, reopen, an AI answer, and only loopback sockets. With and without the team password gate. Added 2026-10-09: the existing suite drives debug builds only, so the gate, the release log filter and the bundled resource path are covered by unit tests alone. (ev: 00-release-smoke.test.mjs on the gated release executable within the full suite, docs/release/evidence/w4/release-run-2026-10-17-gated.txt, and on the open release executable, docs/release/evidence/w4/release-run-2026-10-17-open.txt; proof level 3)
- [ ] **Disk image target.** `tauri.conf.json` gains the `dmg` bundle target for the Mac build; the unsigned image and the Gatekeeper steps are documented as an installation restriction. (Needs Mac hardware) Added 2026-10-09: today the configuration lists only `nsis` and `msi`, and the Mac workflow only runs `cargo check`. **Superseded (ADR-0027, 2026-10-16 sweep):** the `.dmg` comes from the SwiftUI shell build in Mac phase M9, not from `tauri.conf.json`.

**Done when:** the internal unsigned installer passes the clean-machine tests with the team gate on and Offline AI working
offline, and the updater works. (Changed 2026-10-08 from "a signed installer"; signing is checked at the store and
public step.)

## 21. Mac App Store package

Confirm Apple's current rules at build time. Known constraints: App Sandbox is required, downloaded programs are not allowed, there is no self-updater.

- [ ] **Apple accounts.** Developer Program membership, App Store Connect record, bundle identifier, second administrator. (Later: store publishing)
- [ ] **Signing.** Apple Distribution certificate, provisioning profile, hardened runtime, minimal entitlements, helper process signed inside the bundle. (Later: store publishing) (Needs Mac hardware)
- [ ] **Package build.** Apple silicon `.app` inside a signed `.pkg`; verification commands pass. (Later: store publishing) (Needs Mac hardware)
- [ ] **Sandbox behaviour.** Projects open and save in folders the user picks; downloaded model files and writable AI data and logs live in the app container (ADR-0026); AI engine runs as a sandboxed helper. (Later: store publishing) (Needs Mac hardware)
- [ ] **No updater** in this build; updates come only from the App Store. (Later: store publishing) (Needs Mac hardware)
- [ ] **TestFlight for Mac** run on a clean Apple silicon Mac: install, first launch, create project, write, save, close, reopen, update, delete, reinstall. (Later: store publishing) (Needs Mac hardware)
- [ ] **App Store Connect details.** Privacy answers, export compliance (encryption), age rating, category, pricing, screenshots at the required sizes, review notes explaining the bundled offline AI and the trial. (Later: store publishing) (Needs Mac hardware) **Superseded in part (ADR-0026, 2026-10-16 sweep):** the models download after install, so the review notes explain the download, not a bundled model.
- [ ] **App Review readiness.** Review notes, demo steps and no unfinished features listed. (Later: store publishing)

**Done when:** the package passes Apple's validation and a clean-machine TestFlight run.

## 22. Legal, privacy, website and support

- [ ] **Domain, company email and hosting** live with SPF, DKIM, DMARC and HTTPS. (Later: store publishing)
- [ ] **Privacy policy rewritten** for the three channels and ADR-0021: what leaves the computer (nothing automatic), the model download host named and what it receives (ADR-0026), payment data per channel, AI disclosure, contact. Placeholders removed. (Later: store publishing)
- [ ] **Counsel review** of the privacy policy, licence terms, refund text and AI disclosures. (Later: store publishing)
- [ ] **Privacy policy hosted** at a stable HTTPS address and set in the store configuration. (Later: store publishing)
- [ ] **End-user licence agreement** and third-party notices page published. (Later: store publishing)
- [ ] **Support live.** Company inbox, public support page, severity levels, response target, second person able to run it. (Later: store publishing)
- [ ] **Public knowledge base.** Install, update, recovery, import and export, AI troubleshooting, Windows and Mac troubleshooting. (Later: store publishing)
- [ ] **Website** per `docs/website/README.md`: product, features, store links, installer download, pricing and trial, support, privacy, terms, security contact. Claims match the build. (Later: store publishing)
- [ ] **Security contact published and tested.** (Later: store publishing)
- [ ] **Website copy checked** against the writing standard: no fake testimonials, no invented numbers, no AI-sounding phrases, no em dashes. (Later: store publishing)
- [ ] **Website accessible and responsive** from 375 to 1440 pixels; Lighthouse and broken link checks recorded. (Later: store publishing)

**Done when:** privacy address, support contact, website and legal pages are live and consistent.

## 23. Final review and submission

- [ ] **Store listings written** from the shipped build for the Microsoft Store and the Mac App Store; no claim for an unfinished feature; prices and trial match everywhere. (Later: store publishing)
- [ ] **Screenshots** from the exact release candidates with an invented demo project; captions accurate. (Later: store publishing)
- [ ] **Age ratings** answered truthfully for both stores (screenplay content may be mature). (Later: store publishing)
- [ ] **Listings disclose** live generative AI, the optional one-time AI model download, offline use after it, local operation and the report path (ADR-0026). (Later: store publishing)
- [ ] **System requirements** filled from the hardware results. (Later: store publishing)
- [ ] **Certification notes and review notes** written: how to start, the trial, optional AI and how to test it, network use limited to user-clicked links and the user-clicked AI model download (ADR-0026), capability justification. (Later: store publishing)
- [ ] **Release notes and known issues,** and the previous release files kept for rollback. (Later: store publishing)
- [ ] **Continuity.** Second administrators, recovery codes, key backups and runbooks stored; recovery drill done. (Later: store publishing)
- [ ] **Evidence package** assembled under `docs/release/evidence/release/`. (Later: store publishing)
- [ ] **Final adversarial audit** done: each question in the launch register answered with evidence. (Later: store publishing)
- [ ] **Store rules re-checked** on the submission date and any change recorded. (Later: store publishing)
- [ ] **Your signed go decision** (see sign-off below). (Later: store publishing)

**Done when:** every area above has met its condition and no stop condition is open.

## 24. Certification, publication and launch

- [ ] **Submit** the unchanged Microsoft Store package and Mac App Store package; publish the installer on the website. (Later: store publishing)
- [ ] **Answer reviewer findings** within one business day; fix and resubmit with a higher version. (Later: store publishing)
- [ ] **After approval, install from the live listings** on clean machines: first launch, create project, trial and purchase, Offline AI model downloaded and working offline afterwards (ADR-0026). (Later: store publishing)
- [ ] **Smoke-test** live listings, privacy address, support address and website links. (Later: store publishing)
- [ ] **Launch team named:** incident lead, engineering lead, product decision maker, support lead, security owner and backups; stop conditions agreed. (Later: store publishing)
- [ ] **Launch content ready:** trailer, founder story, 30-day content plan, creator and community outreach, social accounts with two-step sign-in. (Later: store publishing)
- [ ] **Status and outage notices channel** and a customer feedback channel live. (Later: store publishing)

**Done when:** all three release files are live and smoke-tested.

## 25. First 90 days after launch

- [ ] **Daily review for 7 days:** store crashes, ratings and reviews, support tickets, incidents. (Later: store publishing)
- [ ] **Urgent fixes only** (reliability, security, data safety); each ships as a new unchanged version. (Later: store publishing)
- [ ] **Days 8 to 30 health review:** top bugs, support themes, knowledge base gaps, security advisories. (Later: store publishing)
- [ ] **Days 31 to 90:** quarterly cadence, backup and restore drill, account access check, privacy and legal check, plan review. (Later: store publishing)
- [ ] **Maintenance calendar running:** dependencies, certificates, domains, policies, vendors. (Later: store publishing)

## 26. Conditions that stop a release

Any one of these blocks submission.

- [ ] Data corruption or project loss; failed migration; crash recovery that can lose customer work.
- [ ] Private creative content leaves the computer without a user click; private note leak; AI bypasses a permission or writes on its own.
- [ ] A placeholder or unverified identity, key, address or policy in a release file or listing.
- [ ] A listing or the website claims a feature that is not Release-ready. (Later: store publishing)
- [ ] A release file differs from the one that was tested, or its store validation or clean-machine test failed.
- [ ] Import or export loses meaning against a published promise.
- [ ] A critical security finding is open.

## Release sign-off

| Area | Evidence location | Owner | Date | Passed |
|---|---|---|---|---|
| Decisions and owner tasks | `docs/release/evidence/M0/` | | | no |
| Project controls and baseline | | | | no |
| Data safety and crash recovery | | | | no |
| Runtime resilience and failure testing | | | | no |
| Test infrastructure and quality gates | | | | no |
| Security hardening from the baseline audit | | | | no |
| Platform adapter layer for Windows and Mac | | | | no |
| AI assistant quality and hardening | | | | no |
| Beat Board and Idea Vault | | | | no |
| Writing comfort | | | | no |
| Screenplay engine and file formats | | | | no |
| Appearance and themes | | | | no |
| Breakdown, catalog, locations, cast and crew | | | | no |
| Shooting schedule and Day Out of Days | | | | no |
| Call sheets, budget, templates, samples and exports | | | | no |
| Packages and backup | | | | no |
| Product-wide quality, accessibility and beta | | | | no |
| Payment and free trial in all three channels | | | | no |
| Microsoft Store package | | | | no |
| Windows installer | | | | no |
| Mac App Store package | | | | no |
| Legal, privacy, website and support | | | | no |
| Final review and submission | | | | no |

**Release decision:** version ______ commit ______ checksums of the three files ______ decision owner ______ date ______
**Go, no go, or go with formal risk acceptance:** ______

## Change log

| Date | Change |
|---|---|
| 2026-10-05 | First version, written from the supplied audits, plans, launch register and repository checks. |
| 2026-10-06 (foundation session) | Thirteen boxes ticked with evidence in `docs/release/evidence/2026-10-06-foundation/`: data safety suites, runtime resilience, diagnostics, security hardening (atomic replace, links, network drives), the platform boundary crate and lint rule R6, the entitlement interface. New owner decisions recorded. |
| 2026-10-07 | ADR-0024: four release files; internal `.exe` and `.dmg` behind a team password asked on every launch. Area 20 gains the boxes "Team password gate" and "Internal Mac disk image". |
| 2026-10-07 | ADR-0023: the Microsoft Store runs the trial with the same full block as the Mac; the `.exe` is an internal team build; no price in the app. Area 1 box on installer sales ticked; area 18 boxes reworded, two added (internal installer build, no price in the app); area 20 website download becomes team distribution. |
| 2026-10-07 | Owner decisions: Mac App Store trial end blocks the app with only Buy and Restore (ADR-0022; area 18 box "No data loss on expiry" reworded and unticked until the new tests merge); OpenFrame Default raises six Light-mode pairs to 4.5 to 1 (ADR-0014 amendment; area 12 box reworded); new area 9 box "Recently Deleted 30-day purge". |
| 2026-10-08 | Owner decisions: local merge to `main` only after tests and end-to-end specs pass; ADR-0020 Accepted (AI engine inside every package); a project may hold several screenplays (ADR-0012 amendment, new area 11 box); Mac trial confirmed (ADR-0022); price and refunds decided later. Area 1: three boxes ticked. |
| 2026-10-08 | `cargo deny`: dated exception for `rustybuzz` and `ttf-parser` (owner choice), new area 6 box "Maintained font shaping crates". |
| 2026-10-08 | Build tools: sccache and rust-lld for dev and test builds in `.cargo/config.toml`; release builds keep the Microsoft linker (area 6 note on "Release hardening flags"). |
| 2026-10-08 | Owner decisions: current goal is the internal `.exe` and `.dmg`; store publishing later; Mac work waits for a Mac; internal builds unsigned for now; team password gate stays on; Offline AI bundled in every package (ADR-0025). Every box tagged "(Later: store publishing)" or "(Needs Mac hardware)" where it applies; intro lists four release files and the current goal; area 8 AI packaging reworded with six new boxes (bundle with pinned hashes, remove the download code, validation per format) and the download and model host items marked N/A; area 20 gains "Internal `.exe` build with the team gate on, unsigned" and splits Mac signing into its own box; `plan:status` prints a "Current goal" line. Nothing ticked or unticked. |
| 2026-10-09 | Session 2 verification: 79 boxes ticked with evidence after the joined `session-2` passed every gate and the full end-to-end run (30 spec files, 158 tests, 0 failed; `docs/release/evidence/2026-10-08-session-2/e2e-session-2.md`): area 8 (2), 9 (8), 10 (6), 11 (21), 12 (10), 13 (7), 14 (9), 15 (11), 16 (3), 18 (2). Boxes left open carry a dated note with the reason. `npm run perf` was not run on the joined code. |
| 2026-10-09 | Full retrospection and audit ([report](../reports/2026-10-09-project-retrospection-and-audit.md)). The work is organised in three chapters ([roadmap](../roadmap/README.md)); a "Chapters" section maps the 26 areas. Nine boxes added from the audit: area 3 (typed text on window close, cloud-synced project folders, project delete and locate tests, migration backup restores), area 8 (non-English requests measured, macOS engine pinned), area 17 (internal tester pilot), area 20 (release-build smoke test, disk image target). Area 3 is no longer complete. |
| 2026-10-15 | Owner decision in chat: AI models download after install (ADR-0026, supersedes ADR-0025; reinstates ADR-0006 decision 2, ADR-0013 decision 2 and ADR-0020 decisions 1 (weights part), 3 and 5). Area 8 bundle and download-removal boxes reopened as "to be done"; owner tasks (model host, manifest key, Gemma redistribution check) reopened; store, privacy, QA, CI and specification wording updated. Nothing ticked except the boxes already evidenced. |
| 2026-10-06 | Rewritten with descriptive headings and no item codes. Added three release files, two platform adapters, paid model with free trial, Windows 11 only, Apple silicon only, GitHub flow, cleanup rule and writing standard. Earlier progress kept: 14 items done. |
