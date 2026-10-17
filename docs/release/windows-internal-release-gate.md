# Windows internal release gate

> Written 2026-10-17 (endgame session, [plan](../implementation/endgame-plan.md)). The owner receives the internal
> installer only when every check below says **Pass** with saved evidence, or **Exception** with the owner's approval and
> date. Nothing else counts: "should work", "passed on debug" and "passed on an earlier build" are not passes. Evidence
> lives in `evidence/w9/`. A code change after the final build re-opens the build checks (G1 to G4).

Status words: Open, Pass, Fail, Exception (owner approved), Exception (waiting for the owner).

## The build

| Id | Check | Pass means | Status | Evidence |
|---|---|---|---|---|
| G1 | One build | The installer was built once from a clean tree at a recorded commit, with the team gate on; its SHA-256, version, commit, build date and size are recorded | Open | |
| G2 | Tested as shipped | The release suite installed and drove exactly the file whose SHA-256 is in G1; the file in `release-out\` has the same hash | Open | |
| G3 | SBOM | CycloneDX for the Rust workspace and `npm sbom` for the web app, saved next to the hash | Open | |
| G4 | Size | Installer size recorded; a jump of more than 20 percent over the estimate is explained | Open | |

## Security

| Id | Check | Pass means | Status | Evidence |
|---|---|---|---|---|
| S1 | Threat model current | Every entry point has attacker input, guard in code and a passing test named | Open | |
| S2 | SQL built safely | Every SQL string built with `format!`, concatenation or `replace` takes user values only as bound parameters; identifiers from a fixed list; grep report saved | Open | |
| S3 | Search input | FTS5 MATCH input cannot use FTS syntax or read other columns; LIKE escapes `%` and `_`; hostile-input tests pass | Open | |
| S4 | Prompt injection | Poisoned-retrieval corpus (40 or more cases, 13 languages): no unrequested tool call, no change without the Proposed Changes card and acceptance, no private note leaked, no URL or path the person did not give | Open | |
| S5 | No live HTML from the model | Model output never renders links, images, frames or scripts that fetch anything; exfiltration tests pass | Open | |
| S6 | AI permissions | Every AI tool keeps its permission, schema and role check; a Viewer cannot change anything through any tool | Open | |
| S7 | Tauri surface | Least-privilege capabilities, no unused commands, no shell open of arbitrary URLs, CSP without `unsafe-eval` and without `unsafe-inline` (or an approved exception), arguments validated, unknown fields refused where input is untrusted | Open | |
| S8 | Files | Traversal, absolute, UNC, links and junctions, reserved names, bombs, huge files, entity expansion, malformed PDF and images refused with bounded memory and time | Open | |
| S9 | Fuzzing | Every importer fuzzed for a fixed time with no crash; corpora and results saved | Open | |
| S10 | AI engine process | Fixed binary and arguments, exits with the app, a crashed engine never hangs the app | Open | |
| S11 | Supply chain | `cargo audit`, `cargo deny`, `npm audit` with zero unexplained findings; lockfiles committed; exceptions dated with a reason | Open | |
| S12 | Binary hardening | BinSkim on the app and engine binaries: ASLR, DEP, CFG, /GS, high entropy VA, no writable and executable section | Open | |
| S13 | Secrets | Installer and repository history scanned (gitleaks): no keys, tokens, passwords or personal paths | Open | |
| S14 | Reviews | Security reviews of the core, importers, AI module and Tauri layer: every finding fixed or recorded here with a reason | Open | |
| S15 | Antivirus | Microsoft Defender scan of the installer on this laptop and in the guest: clean | Open | |

## Clean machine (proof level 4)

| Id | Check | Pass means | Status | Evidence |
|---|---|---|---|---|
| C1 | Setups | VirtualBox Windows 11 guest from a clean snapshot, versions recorded; second Windows account on this laptop | Open | |
| C2 | Install and first start | Per-user install with no administrator prompt; first start; WebView2 present; Visual C++ runtime next to the engine | Open | |
| C3 | Core journey | Create a project, type, close, reopen: text kept | Open | |
| C4 | Team gate | Wrong password refused and delayed, right password opens; the gate cannot be bypassed through settings, files or the registry | Open | |
| C5 | AI download and offline use | Download started by the person, then answers with the network adapter off; processor-only speed recorded separately | Open | |
| C6 | Export, backup, restore | PDF export; backup and restore (also on the second Windows account) | Open | |
| C7 | Upgrade | Install over the previous internal build: data kept | Open | |
| C8 | Uninstall and reinstall | Projects stay unless the person chooses otherwise, and the uninstaller says so; reinstall finds them; start with no network | Open | |
| C9 | Process Monitor | Full session: only expected files and registry keys; network only for the download the person started | Open | |
| C10 | WACK | Windows App Certification Kit run; each failure fixed (core) or recorded for Chapter 3 | Open | |
| C11 | Accessibility | Accessibility Insights FastPass on the main screens: no failure | Open | |
| C12 | SmartScreen note | The steps for an unsigned installer written for the team | Open | |

## Behaviour under stress (release build)

| Id | Check | Pass means | Status | Evidence |
|---|---|---|---|---|
| P1 | Performance | Cold start, typing latency in a 120-page script, memory after 2 hours, AI first answer: within the spec 97 budgets | Open | |
| P2 | Crash and power loss | Kill during save, full disk, network pulled during the download, VM powered off while typing: no lost text, no corrupt project, a clear message | Open | |
| P3 | Release suite | Whole end-to-end suite on the installed release file: 0 failures | Open | |
| P4 | AI harness | All 679 cases on the release build, report saved; English bar (90 and 85 percent) still met | Open | |

## Records

| Id | Check | Pass means | Status | Evidence |
|---|---|---|---|---|
| R1 | Gap list | Every open box in masterplan areas 2 to 8, 16, 20 and every Must-ship row below level 3 is finished, proved, or moved with a reason | Open | |
| R2 | Documents | Feature status, masterplan, manual, licences notice, handoff, progress log current; links, writing count and governance checks green | Open | |
| R3 | Pilot kit | Install note, daily test list, bug template, log sending, triage list and freeze steps ready | Open | |
