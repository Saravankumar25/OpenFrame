# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. It exists so the next session starts from the facts
> instead of from a long history, which is how context rot is avoided. If this page is older than the newest progress log
> entry, trust the log and fix this page first. Last rewritten: 2026-10-16, mid-session (W4 release run in progress, W5 started).

## State

1. **W1 to W3 are on `main`** (local fast-forward, owner's word 2026-10-16; not pushed).
2. **W4 on `windows/w4-release-proof`** (3 commits): WebView2 arguments in `tauri.conf.json` (`additionalBrowserArgs`, read
   by the e2e launcher too), release build honours `OPENFRAME_DATA_ROOT` for file locations only (ADR-0031), web view
   profile of a test run inside the test folder, runner `--release=gated|open`, `@e2e-debug-only` (specs 15, 16, 76),
   `@e2e-net-log`, `scripts/check-binary.mjs`, new specs 00, 26, 44, 63, 86, 96, 97, all 191 scenarios in
   `tests/e2e/scenarios/`, screenplay editor tells keyboard and screen reader users that Escape leaves it.
3. **Release build made** (`tauri build --no-bundle`, open variant, 47.5 MB exe): `check-binary` ok (CFG, ASLR, DEP, no
   symbols), `scan-installer` on exe + `ai/runtimes` + `ai/notices` clean. Full suite on it was running at this rewrite
   (`--release=open`, log in the session scratchpad); smoke, journey and relaunch were green.
4. **Owner decision 2026-10-16:** WebView2's own Microsoft diagnostic-data upload is allowed and disclosed (threat model
   section 4 item 8, privacy policy); spec 05 tolerates only TCP 443 from the browser process.
5. **W5 on `windows/w5-measure-ai`** (branched from the W4 tip): `InferenceProvider` with capabilities, token usage, trace
   table `sys_ai_trace` (app migration 3), `ai.health`, `ai.export_report`, crates `openframe-ai-checks` and
   `openframe-ai-eval` (`ai-eval` binary), 679 gold cases in `crates/openframe-ai-eval/gold/`.

## Next items

1. Read the release run result; fix failures; run `--release=gated` (scan the gated exe too); write
   `docs/release/evidence/w4/` (release run, binary checks, performance numbers, keyboard pass); move proven rows to
   proof level 3; finish the W4 roadmap boxes; then the debug suite once.
2. W5: Settings AI health card and Export AI report button (`ai.health`, `ai.export_report`; run the application tests so
   ts-rs writes the new TypeScript types); undo of an applied proposal should set the trace outcome `undone`.
3. W5: owner approves the gold sets, then the baseline run with `ai-eval` on the release profile; the 8 GB laptop run
   needs the owner.

## Blocked on the owner

- Approving the 679 gold cases (W5). The 8 GB laptop performance and baseline runs. Any model download (W5 bake-off,
  W7 Vulkan runtime): name, source, size and licence first. The OneDrive manual test.

## Do not

- Do not push, merge to `main`, or build an installer, MSI, MSIX or bundle. A release executable for checks is allowed.
- Do not change prompts, models or AI behaviour before the W5 baseline report exists (ADR-0028).
- Do not write the team password anywhere; a gated test build uses a random password made by `run.mjs` for that run.
- Do not edit `docs/archive/`, `docs/audits/` or existing files in `docs/release/evidence/`.

## Facts a new session cannot see from the code

- The owner is a beginner in Rust, Tauri and Claude Code: plain words, short replies, smallest working change.
- Hardware: 15.4 GB memory, 18 logical cores, Intel Core Ultra 5; a release app build takes about 20 minutes from cold.
- `npm run test:e2e -- <flags>` now passes flags through (it silently dropped them before).
- A stray folder `.claude/worktrees/agent-a056d60c8c58fe10a` (not a registered worktree) holds an old copy; ask the
  owner before removing it.
