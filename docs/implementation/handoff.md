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

6. **W5 continued:** Settings AI health card and Export AI report (committed). Gold review page
   `crates/openframe-ai-eval/gold/REVIEW.md`. **Owner 2026-10-16: run a labelled trial of the harness now; the official
   baseline waits for the owner's approval of the gold sets.** Trace outcome `undone` not done yet.
7. **W7 on `windows/w7-gpu`** (stacked on W5): decision function with 34 computers (`openframe-ai/src/gpu.rs`), DXGI
   adapter detection and System32 Vulkan loader check (`openframe-platform/src/graphics.rs`). **Owner approved the Vulkan
   runtime download**: llama.cpp b11259 Vulkan x64 (33,078,808 bytes, SHA-256 afb3ddfc...9610, MIT), extracted to
   `.ai-assets/runtimes/llama-cpp-b11259-win-x64-vulkan` and pinned in the lock (uncommitted at this rewrite); staging
   script and Rust lock check accept one processor build plus at most one Vulkan build. This PC: Intel Arc integrated,
   driver 32.0.101.8132, Vulkan loader present. Package grows by about 89 MB with two complete engine folders (report in
   W9). Spec 96 now retries the first screen-reader query (the tree is built only when a reader asks).
8. **W8 on `windows/w8-download`** (stacked on W7). Done: step 1 signed manifest (`openframe-ai/src/download.rs`,
   tests with a fixed test key, never the production key), step 2 `.partial` download with range resume
   (`openframe-ai/src/fetch.rs`, local test host in its tests), free-space helper (`openframe-platform` `drives::free_bytes`).
   Next: `install.rs` (preflight, size and hash check, atomic activation into `models/<version>`, keep the last good
   version until a health check passes, quarantine), the manager reading models from app data instead of the package,
   the operations and screens, the ADR-0026 decision 10 tests. Release run 2026-10-16 on the open release exe: 29 of 33
   files green, 3 debug-only skipped, spec 95 needed a Day Out of Days recipe (fixed), spec 96 a retry (fixed); rerun of
   the shared group was green for 01, 02, 90 at this rewrite. Performance (release): start 0.72 s, open 0.28 s, 174 MB
   without AI, 524 MB with AI, typing p95 32 ms (budget 16, open item), first AI answer 42.5 s, idle 0.68 percent of a
   core with the engine loaded. Earlier plan: manifest envelope signed with Ed25519 (`ed25519-dalek` already in the tree), `.partial`
   download with HTTP range resume, HTTPS only (loopback HTTP in debug tests), redirects only to the same host, size cap,
   free-space preflight (new platform helper), atomic activation into `%LOCALAPPDATA%\OpenFrame\models\<version>`,
   quarantine, the ADR-0026 decision 10 tests against a local host, then the screens.

9. **Owner rule 2026-10-16: every test from a release-ready point of view** (owner-decisions row). The e2e suite counts
   only on the release exe (open and gated); spec 97 gates P3 (app work per key < 16 ms p95) and P5 (idle without AI
   < 0.5 percent); the AI harness runs with `--release`; the Rust suite once with `--release`. Spec 17 proves the
   internal build on release; specs 15 and 16 stay debug-only (store states need a packaged store build, Chapter 3);
   spec 76 becomes release-capable in W8. Running at this rewrite: `--release=gated` suite (log `e2e-release-gated.log`
   in the session scratchpad), then the English AI trial (`.build-cache/ai-eval/trial-2026-10-16`, log `ai-eval-trial.log`).
10. **W8 next step, the plan:** models leave the package. The runtime stays in `ai/runtimes/` of the install; models come
   from the active set in `<app data>/models/<version>/` with a `set.json` (the signed tier entry) written at activation
   and checked (sizes, hashes) before the first engine start. `prepare-ai-assets.mjs` stages runtimes only; models go to a
   local test host folder. Build-time `MODEL_DISTRIBUTION_BASE_URL` and `OPENFRAME_MANIFEST_PUBLIC_KEY` (release build
   fails without them); test builds get a test key and the local host from `run.mjs`. AI specs pre-activate a models set
   in their data root (as a finished download would) or click Download against the local host (new spec). Then the
   operations (`ai.download.*`), the screens (Download OpenFrame AI, progress, pause, resume, remove), spec 76 rewritten
   for the downloaded set, and the ADR-0026 decision 10 tests. Owner tasks for W8 as a checklist in owner-decisions.

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
