> Superseded by [the Windows internal release gate](../release/windows-internal-release-gate.md) and the progress log on 2026-10-24: W9 closed on 2026-10-18 and this session plan is finished. Moved to history unchanged.

# Endgame plan: finish the Windows track (W9 and W10 preparation)

> Written 2026-10-17 at the start of the endgame session, on branch `windows/w9-endgame` from `main` at 53b504d. One
> page. Work items run one at a time in this order. Each ends with a local commit after its check passes. Proof levels
> follow [ADR-0030](../adr/ADR-0030-verification-ladder-and-release-flow.md): 2 is the debug build, 3 the release
> executable, 4 a clean computer driven by a script, 5 people. The release gate that every item feeds is
> [windows-internal-release-gate.md](../release/windows-internal-release-gate.md). Evidence goes to
> `docs/release/evidence/w9/`.

Scope rule: no new features. Prove what exists, find what is weak, fix it at the root with a failing test first.

| # | Work item | Components | Verified by | Proof level |
|---|---|---|---|---|
| E1 | This plan and the release gate | docs | Review against the session brief | n/a |
| E2 | Threat model refresh: every entry point with attacker input, guard, proving test | `docs/engineering/20-security-threat-model.md` | Each row names a test that exists and passes | 2 |
| E3 | SQL: every built SQL string audited; FTS5 MATCH and LIKE escaping; hostile-string tests | storage and application search and list operations | Grep report saved; new tests fail before the fix, pass after | 2, then 3 with the suite |
| E4 | AI safety: poisoned-retrieval corpus (40 or more cases, 13 languages); no live HTML from model output; Viewer cannot change anything through any AI tool | AI module, `openframe-ai-eval`, `apps/desktop/src/ai` | Corpus run report; one Rust test per tool for the Viewer role; vitest for rendering | 2 and 3 |
| E5 | Tauri surface: capabilities, commands, CSP, IPC argument validation, `deny_unknown_fields` | `apps/desktop/src-tauri` | Config review report; tests for rejected arguments | 2 and 3 |
| E6 | Files: traversal, reserved names, links, bombs, entity expansion, malformed PDF and images; fuzz every importer for a fixed time | importers, `fuzz/` | Hostile file tests; fuzz results and corpora saved | 2 |
| E7 | Process control of the AI engine: fixed binary and arguments, exits with the app, crash never hangs | AI engine supervisor | Rust tests; release suite specs 72 to 76 | 2 and 3 |
| E8 | Supply chain and SBOM: cargo audit, cargo deny, npm audit; CycloneDX for Rust and npm | lockfiles, `deny.toml` | Reports saved, zero unexplained findings | n/a |
| E9 | Gap list for masterplan areas 2 to 8, 16, 20 and Must-ship rows below level 3: finish, prove or move to Chapter 2 or 3 with a reason | as each item needs | `gap-list.md` in the evidence folder | per row |
| E10 | AI numbers: the nine English end-state misses, answer language, retrieval for French, Japanese, Korean, Chinese; measure first, keep only a change that improves its number | AI module, prompts, retrieval | `ai-eval` before and after reports | 2, final run at 3 |
| E11 | The one build: gated installer built once, release suite on that exact file, SHA-256, SBOM, BinSkim, secrets scan, Defender | build scripts, installer | Gate checks with saved evidence | 3 |
| E12 | Clean machine: VirtualBox Windows 11 guest from a "clean" snapshot; install to reinstall steps, AI on the processor, WACK, Process Monitor, Accessibility Insights | VM, installed app | Screenshots, logs and captures per step | 4 |
| E13 | Performance and crash behaviour on the release build: cold start, typing in a 120-page script, 2-hour memory, AI first answer; kill during save, full disk, network pulled, VM powered off | whole app | Numbers against spec 97 budgets; crash test logs | 3 and 4 |
| E14 | Mac and store readiness of the core: Windows-only assumptions fixed with portable tests; App Sandbox, privacy manifest and MSIX inputs recorded | Rust core, `chapter-1b-mac-phases.md` | Tests that run on any platform; recorded findings | 2 |
| E15 | W10 preparation: pilot kit, triage list, freeze steps | `docs/testing/` | Review | n/a |
| E16 | Clean as you go (documentation and clean-up agents only, file-ownership map first): dead code, links, writing counts, licences, status rows | docs, unused code | `check:governance` green | n/a |
| E17 | Final: whole `ai-eval` (679 cases) on the release build, handoff, progress log, dashboard, graph update | docs | Saved report; `plan:status` | 3 |

Owner inputs this plan waits on (everything else continues meanwhile): the `OPENFRAME_TEAM_PASSWORD` variable set on this
computer (E11), a click on the Windows administrator prompt when VirtualBox installs (E12), and a second Windows user
account on this laptop, which only the owner may create (E12, backup and restore on a second account).

New tools, each announced with name, source and size before install: VirtualBox and the Windows 11 Enterprise evaluation
image (approved), Microsoft BinSkim, gitleaks, cargo-cyclonedx, cargo-machete, knip, Sysinternals Process Monitor,
Accessibility Insights for Windows. The Windows App Certification Kit is already installed with the Windows SDK.
