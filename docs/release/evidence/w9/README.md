# W9 evidence (endgame)

> 2026-10-18. One line per row of the [release gate](../../windows-internal-release-gate.md). A row appears here once
> it has saved evidence. Rows not listed are still open. The `vm/` folder is written by the clean machine runs.

| Gate row | What the evidence says | File |
|---|---|---|
| G1 | Final installer built once from a clean tree: size, SHA-256, commit | [Final installer](#final-installer) |
| G3 | SBOMs next to the installer | [Final installer](#final-installer) |
| G4 | 27 MB against an estimate of 20 to 30 MB | [Final installer](#final-installer) |
| S1 | Threat model refreshed with the proving test per entry point | [20-security-threat-model.md](../../../engineering/20-security-threat-model.md) |
| S2 | No SQL injection, 3 low findings fixed | [sql-audit.md](sql-audit.md) |
| S3 | Search input hostile tests | [sql-audit.md](sql-audit.md) |
| S4 | Prompt injection: 60 of 60 safe | [ai-poisoned/README.md](ai-poisoned/README.md) |
| S5 | Model output never rendered as live HTML, zero raw-HTML sinks | [Other checks](#other-checks) |
| S6 | 186 changing AI tools times 3 read-only roles: 558 refusals | [Other checks](#other-checks) |
| S7 | Tauri commands under the access list, CSP exception | [Other checks](#other-checks) |
| S10 | AI engine process review | [binary-hardening.md](binary-hardening.md) |
| S11 | Supply chain, dev-only exception | [supply-chain.md](supply-chain.md) |
| S12 | BinSkim results and exceptions | [binary-hardening.md](binary-hardening.md), [binskim/](binskim/) |
| S13 | Secrets scan, owner's name in 2 old commits | [supply-chain.md](supply-chain.md) |
| S15 | Defender scan of the rehearsal installer on the laptop (guest scan pending) | [Other checks](#other-checks) |
| P3 | End-to-end runs on the release build (final full run still to come) | [e2e-release-runs.md](e2e-release-runs.md) |
| R1 | Gap list | [gap-list.md](gap-list.md) |

## Final installer

| Item | Value |
|---|---|
| File | `C:\OpenFrame\release-out\OpenFrame-Studio-0.1.0-internal-995bfac.exe` |
| Size | 27,025,974 bytes |
| SHA-256 | `d057d115d1c5f960983be8a16483920795c2f19be64235a6ff479626b9cc2f3f` |
| Commit | 995bfac, built from a clean tree |
| Checks | `scan-installer` 53 files, `check-binary` ok |
| SBOMs | Rust CycloneDX and npm CycloneDX, next to the installer |

## Other checks

| Check | Result |
|---|---|
| Tauri commands | All five app commands sit under the access list; 6 unused permissions removed (commit d271894) |
| IPC arguments | All 377 argument structs refuse unknown fields |
| AI roles | `every_changing_tool_is_refused_for_viewer_commenter_and_export_only` in `crates/openframe-application/tests/ai/ai_toolbox.rs`: 186 tools times 3 roles = 558 refusals with `permission.denied`, no row changes. A Viewer also cannot accept a Change Set an Editor proposed (commit ed081bc) |
| Live HTML | `apps/desktop/src/ai/noLiveHtml.test.tsx`: hostile markdown, raw tags and `javascript:` and `data:` URLs show as literal text; a second test fails if a raw-HTML sink appears in the web source (ed081bc) |
| CSP | `script-src` has no `unsafe-inline` or `unsafe-eval`. `style-src` keeps `'unsafe-inline'` because Radix UI's scroll lock (`react-remove-scroll` through `react-style-singleton`) and the theme engine add style elements at run time. No path for attacker HTML reaches the web view (proved by the no-live-HTML tests). Exception, waiting for the owner |
| Defender | Rehearsal installer d271894: no threats (signatures 1.461.74.0). The final installer scan and the guest scan are pending |

## Added 2026-10-18 by the main session

- Microsoft Defender on this laptop, final installer `OpenFrame-Studio-0.1.0-internal-995bfac.exe`
  (`MpCmdRun -Scan -ScanType 3 -DisableRemediation`, signatures 1.461.74.0): no threats. The guest scan is part of the
  clean-machine run.
- Gate S10 (AI engine process) tests: `supervisor_starts_serves_restarts_after_crash_and_stops` and
  `runtime_that_cannot_start_fails_cleanly_without_restart_loops` (crates/openframe-ai/tests/supervisor.rs),
  `m1_08_sidecar_crash_restart_and_cancel_loop` (tests/soak_sidecar.rs),
  `the_startup_sweep_stops_a_stray_engine_from_the_record` and
  `a_changed_missing_or_planted_program_stops_the_launch` (src/engines.rs). The engine runs in a Job Object with
  kill-on-close and refuses to start when that guard is unavailable, so it exits with the app.
- The poisoned-retrieval set covers the 12 languages the product supports (ADR-0028). The session brief said 13; the
  product language list has 12.
