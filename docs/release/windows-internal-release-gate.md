# Windows internal release gate

> Written 2026-10-17 (endgame session, [plan](../implementation/endgame-plan.md)). The owner receives the internal
> installer only when every check below says **Pass** with saved evidence, or **Exception** with the owner's approval and
> date. Nothing else counts: "should work", "passed on debug" and "passed on an earlier build" are not passes. Evidence
> lives in `evidence/w9/`. A code change after the final build re-opens the build checks (G1 to G4).

Status words: Open, Pass, Fail, Exception (owner approved), Exception (waiting for the owner).

## The build

| Id | Check | Pass means | Status | Evidence |
|---|---|---|---|---|
| G1 | One build | The installer was built once from a clean tree at a recorded commit, with the team gate on; its SHA-256, version, commit, build date and size are recorded | Pass | Commit 995bfac, clean tree, 27,025,974 bytes, SHA-256 `d057d115d1c5f960983be8a16483920795c2f19be64235a6ff479626b9cc2f3f`, file `release-out\OpenFrame-Studio-0.1.0-internal-995bfac.exe`; see [W9 README](evidence/w9/README.md) |
| G2 | Tested as shipped | The release suite installed and drove exactly the file whose SHA-256 is in G1; the file in `release-out\` has the same hash | Open | |
| G3 | SBOM | CycloneDX for the Rust workspace and `npm sbom` for the web app, saved next to the hash | Pass | Rust CycloneDX and npm CycloneDX next to the installer; see [W9 README](evidence/w9/README.md) |
| G4 | Size | Installer size recorded; a jump of more than 20 percent over the estimate is explained | Pass | 27 MB against the 20 to 30 MB estimate; see [W9 README](evidence/w9/README.md) |

## Security

| Id | Check | Pass means | Status | Evidence |
|---|---|---|---|---|
| S1 | Threat model current | Every entry point has attacker input, guard in code and a passing test named | Open | Kept open: more than two threat model rows still lack a named test (T1 CI grep, T19, T24, T33, T35, T39, T40). Mapping: [security-mapping.md](evidence/w9/security-mapping.md) |
| S2 | SQL built safely | Every SQL string built with `format!`, concatenation or `replace` takes user values only as bound parameters; identifiers from a fixed list; grep report saved | Pass | [sql-audit.md](evidence/w9/sql-audit.md) |
| S3 | Search input | FTS5 MATCH input cannot use FTS syntax or read other columns; LIKE escapes `%` and `_`; hostile-input tests pass | Pass | [sql-audit.md](evidence/w9/sql-audit.md) |
| S4 | Prompt injection | Poisoned-retrieval corpus (40 or more cases, 12 languages): no unrequested tool call, no change without the Proposed Changes card and acceptance, no private note leaked, no URL or path the person did not give | Pass | 60 of 60 safe (60 cases, 12 languages): [ai-poisoned](evidence/w9/ai-poisoned/README.md) |
| S5 | No live HTML from the model | Model output never renders links, images, frames or scripts that fetch anything; exfiltration tests pass | Pass | `apps/desktop/src/ai/noLiveHtml.test.tsx`, zero raw-HTML sinks; [W9 README](evidence/w9/README.md) |
| S6 | AI permissions | Every AI tool keeps its permission, schema and role check; a Viewer cannot change anything through any tool | Pass | 558 refusals, `every_changing_tool_is_refused_for_viewer_commenter_and_export_only`; [W9 README](evidence/w9/README.md) |
| S7 | Tauri surface | Least-privilege capabilities, no unused commands, no shell open of arbitrary URLs, CSP without `unsafe-eval` and without `unsafe-inline` (or an approved exception), arguments validated, unknown fields refused where input is untrusted | Exception (owner approved 2026-10-18) | `style-src` keeps `unsafe-inline` (Radix UI scroll lock and theme engine); the other parts pass; [W9 README](evidence/w9/README.md) |
| S8 | Files | Traversal, absolute, UNC, links and junctions, reserved names, bombs, huge files, entity expansion, malformed PDF and images refused with bounded memory and time | Pass, images: decode limits (20,000 pixels an edge, 512 MiB, 64 MiB source) and an unreadable-image test on the report side; no malformed-image import test | Tests named in [security-mapping.md](evidence/w9/security-mapping.md) |
| S9 | Fuzzing | Every importer fuzzed for a fixed time with no crash; corpora and results saved | Exception (owner approved 2026-10-18) | Six fuzz targets ran in W3 ([fuzz-2026-10-16.md](evidence/w3/fuzz-2026-10-16.md)); the owner decided on 2026-10-18 not to re-fuzz in the endgame session ([owner decisions](../implementation/owner-decisions.md), row "Scope of the endgame session"). See [security-mapping.md](evidence/w9/security-mapping.md) |
| S10 | AI engine process | Fixed binary and arguments, exits with the app, a crashed engine never hangs the app | Pass | Threat model T16 and T18; `crates/openframe-ai/tests/supervisor.rs`; [binary-hardening.md](evidence/w9/binary-hardening.md) |
| S11 | Supply chain | `cargo audit`, `cargo deny`, `npm audit` with zero unexplained findings; lockfiles committed; exceptions dated with a reason | Exception (owner approved 2026-10-18) | webdriverio 9 dev-only chain, review by 2027-01-18: [supply-chain.md](evidence/w9/supply-chain.md) |
| S12 | Binary hardening | BinSkim on the app and engine binaries: ASLR, DEP, CFG, /GS, high entropy VA, no writable and executable section | Exception (owner approved 2026-10-18) | Engine CFG, ring assembly hashing, Rust std /GS, Spectre: [binary-hardening.md](evidence/w9/binary-hardening.md). Fix (build llama.cpp with Control Flow Guard) before the store release in Chapter 3 |
| S13 | Secrets | Installer and repository history scanned (gitleaks): no keys, tokens, passwords or personal paths | Exception (owner approved 2026-10-18) | Owner's name in 2 old commits; no keys or tokens anywhere: [supply-chain.md](evidence/w9/supply-chain.md). No history rewrite; clean only if the code ever becomes public |
| S14 | Reviews | Security reviews of the core, importers, AI module and Tauri layer: every finding fixed or recorded here with a reason | Open | |
| S15 | Antivirus | Microsoft Defender scan of the installer on this laptop and in the guest: clean | Exception (owner approved 2026-10-18) | Laptop half passed: final installer, no threats (signatures 1.461.74.0). Guest half covered by the W10 pilot on team laptops; [vm-blocked.md](evidence/w9/vm-blocked.md) |

## Clean machine (proof level 4)

| Id | Check | Pass means | Status | Evidence |
|---|---|---|---|---|
| C1 | Setups | VirtualBox Windows 11 guest from a clean snapshot, versions recorded; second Windows account on this laptop | Exception (owner approved 2026-10-18) | Covered by the W10 pilot on team laptops; [vm-blocked.md](evidence/w9/vm-blocked.md) |
| C2 | Install and first start | Per-user install with no administrator prompt; first start; WebView2 present; Visual C++ runtime next to the engine | Exception (owner approved 2026-10-18) | Covered by the W10 pilot on team laptops; [vm-blocked.md](evidence/w9/vm-blocked.md) |
| C3 | Core journey | Create a project, type, close, reopen: text kept | Exception (owner approved 2026-10-18) | Covered by the W10 pilot on team laptops; [vm-blocked.md](evidence/w9/vm-blocked.md) |
| C4 | Team gate | Wrong password refused and delayed, right password opens; the gate cannot be bypassed through settings, files or the registry | Exception (owner approved 2026-10-18) | Covered by the W10 pilot on team laptops; [vm-blocked.md](evidence/w9/vm-blocked.md) |
| C5 | AI download and offline use | Download started by the person, then answers with the network adapter off; processor-only speed recorded separately | Exception (owner approved 2026-10-18) | Covered by the W10 pilot on team laptops; [vm-blocked.md](evidence/w9/vm-blocked.md) |
| C6 | Export, backup, restore | PDF export; backup and restore (also on the second Windows account) | Exception (owner approved 2026-10-18) | Covered by the W10 pilot on team laptops; [vm-blocked.md](evidence/w9/vm-blocked.md) |
| C7 | Upgrade | Install over the previous internal build: data kept | Exception (owner approved 2026-10-18) | Covered by the W10 pilot on team laptops; [vm-blocked.md](evidence/w9/vm-blocked.md) |
| C8 | Uninstall and reinstall | Projects stay unless the person chooses otherwise, and the uninstaller says so; reinstall finds them; start with no network | Exception (owner approved 2026-10-18) | Covered by the W10 pilot on team laptops; [vm-blocked.md](evidence/w9/vm-blocked.md) |
| C9 | Process Monitor | Full session: only expected files and registry keys; network only for the download the person started | Exception (owner approved 2026-10-18) | Covered by the W10 pilot on team laptops; [vm-blocked.md](evidence/w9/vm-blocked.md) |
| C10 | WACK | Windows App Certification Kit run; each failure fixed (core) or recorded for Chapter 3 | Exception (owner approved 2026-10-18) | Covered by the W10 pilot on team laptops; [vm-blocked.md](evidence/w9/vm-blocked.md) |
| C11 | Accessibility | Accessibility Insights FastPass on the main screens: no failure | Exception (owner approved 2026-10-18) | Covered by the W10 pilot on team laptops; [vm-blocked.md](evidence/w9/vm-blocked.md) |
| C12 | SmartScreen note | The steps for an unsigned installer written for the team | Pass | [install-note.md](../testing/pilot-kit/install-note.md) |

## Behaviour under stress (release build)

| Id | Check | Pass means | Status | Evidence |
|---|---|---|---|---|
| P1 | Performance | Cold start, typing latency in a 120-page script, memory after 2 hours, AI first answer: within the spec 97 budgets | Open | Spec 97 performance budgets (typing, idle, memory) passed in the final run; the 2-hour memory check and the 120-page typing check were not run |
| P2 | Crash and power loss | Kill during save, full disk, network pulled during the download, VM powered off while typing: no lost text, no corrupt project, a clear message | Pass | Spec 26 forced kill and spec 04 close-keeps-typing passed in the final run; full disk, network pull and resume tests cited in [security-mapping.md](evidence/w9/security-mapping.md); the VM power-off while typing is covered by the W10 pilot |
| P3 | Release suite | Whole end-to-end suite on the installed release file: 0 failures | Pass | [e2e-release-runs.md](evidence/w9/e2e-release-runs.md), Run 4 (final): 39 spec files, 188 tests, 0 failures |
| P4 | AI harness | All 679 cases on the release build, report saved; English bar (90 and 85 percent) still met | Open | Poisoned set 60 of 60 safe ([ai-poisoned](evidence/w9/ai-poisoned/README.md)); the 150-case English rerun was not run |

## Records

| Id | Check | Pass means | Status | Evidence |
|---|---|---|---|---|
| R1 | Gap list | Every open box in masterplan areas 2 to 8, 16, 20 and every Must-ship row below level 3 is finished, proved, or moved with a reason | Exception (owner decision 2026-10-18) | Gap list in [gap-list.md](evidence/w9/gap-list.md); the spec additions it proposes were dropped, W1 to W8 closed as accepted |
| R2 | Documents | Feature status, masterplan, manual, licences notice, handoff, progress log current; links, writing count and governance checks green | Pass | Handoff, progress log, reconciliation log and owner decisions rewritten in the documentation pass of 2026-10-18; checks green |
| R3 | Pilot kit | Install note, daily test list, bug template, log sending, triage list and freeze steps ready | Pass | `docs/testing/pilot-kit/` and `docs/release/w10-freeze-steps.md` |
