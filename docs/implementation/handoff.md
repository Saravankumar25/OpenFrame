# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. It exists so the next session starts from the facts
> instead of from a long history, which is how context rot is avoided. If this page is older than the newest progress log
> entry, trust the log and fix this page first. Last rewritten: 2026-10-18, after the W9 endgame session.

## State

1. **`main`** holds W1 to W8. Nothing pushed. **Branch `windows/w9-endgame`** (not merged) holds the W9 work, newest first:
   `8f92cbb` security evidence and gate update; `3b19453` only `file:/` URIs count as links in the AI safety check;
   `1fbc0db` harness back on webdriverio 9; `8faeea7` W10 pilot kit and freeze steps; `14ecbaa` poisoned-retrieval set;
   `995bfac` CET compatible binary and SHA-256 hashing of C sources; `ed081bc` AI role and no-HTML tests;
   `c2d72f9` installer pipeline; `d271894` Tauri access list; `224bc59` and `1a7bc06` SQL (LIKE escaping, stored table
   names); `423789e` W1 to W8 gap list; `c71577b` GSD cleanup and supply-chain evidence; `2b7cffc` dependency upgrades;
   `746be09` endgame plan and release gate. The documentation pass of this session is committed on top.
2. **Internal installer:** `release-out\OpenFrame-Studio-0.1.0-internal-995bfac.exe`, 27,025,974 bytes, SHA-256
   `d057d115d1c5f960983be8a16483920795c2f19be64235a6ff479626b9cc2f3f`. Built from a clean tree at 995bfac with the team
   gate on. Scans OK, Defender on this laptop found no threats (signatures 1.461.74.0), SBOMs next to it. Later commits
   changed only tests, the test harness, the AI eval harness and documents, not the app.
3. **Final release end-to-end run (2026-10-18):** 39 spec files passed, 188 tests, 0 failures
   ([e2e-release-runs.md](../release/evidence/w9/e2e-release-runs.md), Run 4).
4. **Release gate** ([windows-internal-release-gate.md](../release/windows-internal-release-gate.md)), 38 rows: 13 Pass,
   13 Exception approved (R1, C1 to C11, S15), 4 Exception waiting for the owner (S7, S11, S12, S13), 8 Open (G2, S1, S8,
   S9, S14, P1, P2, P4).
5. **Clean-machine proof** is carried by the W10 pilot on the team's laptops (owner decision 2026-10-18,
   [vm-blocked.md](../release/evidence/w9/vm-blocked.md)). The VM and Windows image are deleted. The script
   `tests/e2e/clean-machine/vm-proof.mjs` stays for a machine where VT-x is free.
6. **HTML mockups session:** 462 snapshots replace 183 old files, committed on branch `claude/website-html-mockups-9ecfa7`,
   not merged.

## Next three items

1. The owner decides the four exceptions S7, S11, S12, S13.
2. Hand the installer and `docs/testing/pilot-kit/` to the team and start W10.
3. Next session: triage the pilot reports with `docs/testing/pilot-kit/triage.md`.

## Open blockers

- Owner decision on S7, S11, S12, S13. The VM is no longer a blocker.
- Gate rows still Open without a blocker: G2 (installer driven as shipped), S1, S8, S9, S14, P1, P2 and P4 parts not run.

## Do not

- Do not push and do not merge to `main` without the owner's word.
- Do not build a new installer unless app code changes. A new build re-opens G1 to G4.
- Do not ask for the team password (decided; the owner set it on the build laptop).
- Do not re-work W1 to W8 (owner decision 2026-10-18): no AI language tuning, no 679-case rerun, no re-fuzzing.
- Do not raise licensing or ask about the model set again (decided).
- Do not run measurements while the release suite runs; keep the laptop on mains power for both.
