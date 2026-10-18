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
   1 Exception approved (R1), 4 Exception waiting for the owner (S7, S11, S12, S13), 20 Open (G2, S1, S8, S9, S14, S15,
   C1 to C11, P1, P2, P4).
5. **Clean-machine proof is blocked** ([vm-blocked.md](../release/evidence/w9/vm-blocked.md)): VirtualBox cannot use the
   processor's virtualisation while Windows Memory Integrity is on, and the guest stalls at first sign-in. The driver
   script `tests/e2e/clean-machine/vm-proof.mjs` is written but has not completed a run.
6. **Other session:** "Rebuild HTML_Mockups" on branch `claude/website-html-mockups-9ecfa7` (worktree
   `.claude/worktrees/website-html-mockups-9ecfa7`). Its status is unknown to this page; check it before doing anything
   there.

## Next three items

1. The owner chooses the VM option (turn Memory Integrity off for a rerun, a second Windows account, or the pilot laptops
   as the clean machines), and decides the four exceptions S7, S11, S12, S13 and confirms the R1 exception.
2. Run the clean-machine proof by the chosen option, then close the Open rows it covers (C1 to C11, S15, P2).
3. Hand the installer to the team with `docs/testing/pilot-kit/` and start W10.

## Open blockers

- Owner choice of the VM option (above). Owner decision on S7, S11, S12, S13.
- Gate rows still Open without a blocker: G2 (installer driven as shipped), S1, S8, S9, S14, P1 and P4 parts not run.

## Do not

- Do not push and do not merge to `main` without the owner's word.
- Do not build a new installer unless app code changes. A new build re-opens G1 to G4.
- Do not ask for the team password (decided; the owner set it on the build laptop).
- Do not re-work W1 to W8 (owner decision 2026-10-18): no AI language tuning, no 679-case rerun, no re-fuzzing.
- Do not raise licensing or ask about the model set again (decided).
- Do not run measurements while the release suite runs; keep the laptop on mains power for both.
