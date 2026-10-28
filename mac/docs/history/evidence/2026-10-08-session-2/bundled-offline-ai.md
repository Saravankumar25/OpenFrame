# Bundled Offline AI evidence (ADR-0025)

Recorded 2026-10-08 on branch `area/08-bundled-offline-ai` (worktree `.claude/worktrees/ai-bundled`), started from
`session-2` at `b00fda2` and merged with `session-2` at `c853632` before the gates. Machine: the owner's Windows 11
development machine (Windows 11 26200, x64, processor only). Masterplan section "8. AI assistant quality and hardening";
decision [ADR-0025](../../../adr/ADR-0025-offline-ai-bundled-in-every-package.md). Result labels follow
`docs/release/evidence/README.md` section 2.

The lead ticks the plan boxes and moves the feature rows after running the end-to-end suite. **No release or installer
build was made** (owner rule of 2026-10-08): no `npm run build`, `npm run bundle`, `tauri build` without `--debug` or
`scripts/build-msix.mjs`. Bundling was verified with the debug test build only (section 4).

## 1. What changed

| Part | Change | Files |
|---|---|---|
| Pinned bundle | A committed lock pins every bundled file by path, bytes and SHA-256 with licence ids: the llama.cpp `b11259` processor build for Windows x64 reduced to `llama-server.exe` and the libraries it loads (22 files plus the LLVM OpenMP licence; the other llama.cpp programs and the RPC backend are left out), Gemma 3 1B Instruct Q8_0 and BGE small English v1.5 Q8_0 (tokenizers embedded in the GGUF files, `tokenizer: "embedded"`), with source provenance. Notices for llama.cpp, Gemma and BGE ship next to them | `packaging/ai/ai-assets.lock.json`, `packaging/ai/notices/` |
| Build | `scripts/prepare-ai-assets.mjs` verifies `.ai-assets` (or `OPENFRAME_AI_ASSETS_DIR`, or the main checkout's `.ai-assets` from a worktree) against the lock, fails closed with a plain message, hard-links the files into `.build-cache/ai-bundle` and re-hashes them there. The Tauri overlay `tauri.bundled-ai.conf.json` maps that folder to the `ai/` resource folder | `scripts/prepare-ai-assets.mjs`, `apps/desktop/src-tauri/tauri.bundled-ai.conf.json` |
| Wiring | `npm run build`, `npm run bundle`, `npm run dev` (optional), `npm run ai:assets`, the E2E test build (`tests/e2e/scripts/build-app.mjs`), `scripts/build-msix.mjs` (copies and re-verifies `ai/` in the package layout), `release.yml` (validate, E2E gate, NSIS/MSI build with an output check, Store build), `ci.yml` (E2E gate), `release-validate.mjs` check 6. CI and release jobs fail closed until the runners get `OPENFRAME_AI_ASSETS_DIR` | `package.json`, `.github/workflows/{ci,release}.yml`, `scripts/{build-msix,release-validate}.mjs` |
| Runtime | The AI manager reads the engine and both models from the read-only `<app resources>/ai` folder named by the platform adapter (`openframe_platform::dirs::app_resources_dir`: the executable's folder on Windows including MSIX, `Contents/Resources` in a Mac app bundle). The lock is compiled into `openframe-ai`. At start: sizes and link-free folders. Before the first engine start: every SHA-256, with a check record per installed version in app data (`ai/bundle-check.json`, keyed to the lock fingerprint, the folder, sizes and modification times). Before every launch: the engine files re-hashed (a planted program or redirection file stops it) and the model files compared with their state at the check. The engine starts with an empty environment plus an allow-list. A missing or altered file turns AI off with "Reinstall OpenFrame Studio to repair it"; nothing is ever fetched | `crates/openframe-ai/src/{bundle,manager,engines,supervisor,store,model,hardware,selection}.rs`, `crates/openframe-platform/src/dirs.rs`, `crates/openframe-application/src/{core.rs,modules/ai/*}`, `apps/desktop/src-tauri/src/lib.rs` |
| Writable data | Only in the user's app data folder: the check record, the engine record (`engines.json`), the engine logs. The copies left by the retired download flow (`models/`, `runtimes/`) are removed at start without following links | `crates/openframe-ai/src/store.rs` |
| Removed | Download, resume, partial files, quarantine, disk preflight, the signed manifest, its development key and signing tool, `MODEL_DISTRIBUTION_BASE_URL`, `OPENFRAME_MANIFEST_PUBLIC_KEY`, release check 6b, `ai.setup_info`, `ai.install`, `ai.cancel_install`, `ai.uninstall`, the Download Offline AI screens (`OfflineAiSetup.tsx`), the Remove and Update buttons, the graphics-card runtime choice, the Ed25519 helper only the manifest used, `internetClient` in the MSIX manifest, and the tests that only covered downloading (`install.rs`, `download.rs`, `lifecycle.rs`, `security_ai.rs`, the download part of `soak_sidecar.rs`, `OfflineAiSetup.test.tsx`) | |
| UI | The AI panel is ready on first launch; when the bundle is missing or damaged it says why and how to repair it (reinstall), never offers a download. Settings > Offline AI shows the status, Stop Offline AI, and the technical details (components, licences, SHA-256, installed folder, read-only, this computer) | `apps/desktop/src/ai/{AiPanel,model}.ts(x)`, `apps/desktop/src/workspaces/settings/OfflineAi.tsx` |
| Test-only switches | A debug build on an isolated test data root may read another AI folder (`OPENFRAME_AI_BUNDLE_DIR`) and use deterministic sampling (`OPENFRAME_AI_DETERMINISTIC=1`: temperature 0, seed 42); release builds ignore both (`lib.rs::ai_test_settings_never_reach_a_release_or_a_real_data_root`) | `apps/desktop/src-tauri/src/lib.rs`, `tests/e2e/run.mjs` |

Security review (`ecc:security-reviewer`, read-only, on the branch diff): no Critical or High finding. Fixed in commit
`55ede7f`: the engine environment is now an allow-list (a `GGML_` backend path could otherwise load other code), model
files are compared before every launch, Windows redirection files count as planted programs, the prepare script
re-hashes staged files and refuses a staging folder outside `.build-cache`, the release job checks the bundled AI the
installers were made from. Recorded, not changed: under the per-user NSIS install the folder is user-writable, so the
checks catch corruption and tampering by other means, not malware already running as the same user (threat model T16).

## 2. Sizes

| What | Bytes | Notes |
|---|---|---|
| Pinned files for Windows x64 (lock) | 1,149,732,477 | 25 files: engine, libraries and OpenMP licence 43,740,957; Gemma 1,069,306,368; BGE 36,685,152 |
| `ai/` folder in the debug test build (with 2,339 bytes of notices) | 1,149,734,816 | `C:/OpenFrame/.build-cache/target-ai/debug/ai`, checked with `prepare-ai-assets.mjs --check --source` |
| Debug executable next to it | 66,694,656 | debug profile; the release executable was 18.2 MB on 2026-09-30 (doc 27 §11) |
| Debug output with resources | 1,216,429,472 | executable plus `ai/` |
| Bundled AI compressed (deflate level 6) | about 1,073,660,000 | Gemma 1,021,611,748; BGE 34,411,979; engine folder 17,637,087. The Q8_0 weights barely compress, so each installer grows by about 1.0 to 1.1 GB. Real installer sizes are not measured: no installer was built |

## 3. Gates

Run once after merging `session-2`, on `HEAD` of the branch.

| Gate | Command | Result |
|---|---|---|
| Format | `cargo fmt --all -- --check` (queued) | PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` (queued; `openframe-ai` re-run after the security fixes) | PASS |
| Rust tests | `cargo test --workspace` (queued) | PASS (94 suites: 2,002 passed, 0 failed, 11 ignored) |
| Lint | `npm --workspace apps/desktop run lint` | PASS |
| Types | `npm run typecheck` | PASS |
| Web tests | `npm run test:web` | PASS (53 files, 535 tests) |
| Governance | `npm run check:governance` | PASS (architecture lint, migrations, feature status 171 rows, reachability, writing standard with the baseline lowered, links, plan status, self-tests) |
| Supply chain | `cargo deny check` (queued) | PASS (advisories, bans, licenses, sources ok; duplicate-version warnings only) |
| Real runtime (ignored, bundled assets) | `OPENFRAME_AI_BUNDLE_DIR=.build-cache/ai-bundle cargo test -p openframe-ai --test real_runtime -- --ignored` | PASS: full check 1.5 s, chat 4.0 s, deterministic repeat identical, grammar-constrained JSON, embeddings (near 0.859, far 0.473), long input shortened, check on the next start 0.00 s |
| Real model through the registry (ignored) | `OPENFRAME_AI_BUNDLE_DIR=… cargo test -p openframe-application --test ai_real_model -- --ignored --test-threads=1` | PASS: exact count 20 s, retrieval with sources 24 s, proposal 30 s, apply, one Undo, reject, stale proposal never applied |
| Release gate report | `node scripts/release-validate.mjs --report-only` | "Bundled Offline AI" PASS (25 pinned files, 1096 MB); other failures are the known owner items (licence, signing, updater, migration freeze) |

## 4. Bundling verified with the debug test build

`npm --workspace apps/desktop run build:web`, `node scripts/prepare-ai-assets.mjs`, then the equivalent of
`tauri build --debug --no-bundle --config src-tauri/tauri.bundled-ai.conf.json` through the build queue
(`cargo build -p openframe-desktop --features tauri/custom-protocol` with the overlay passed as `TAURI_CONFIG`, so it
went through `cargo-queued.mjs`). Result: `ai/runtimes/llama-cpp-b11259-win-x64-cpu/` (23 files),
`ai/models/<id>/model.gguf` (2) and `ai/notices/` (3) next to `openframe-desktop.exe`; the folder matches the lock.
The app's resource lookup (`AppConfig::installed_ai_bundle_dir`) is the executable's folder plus `ai`
(`lib.rs::ai_test_settings_never_reach_a_release_or_a_real_data_root`). Not verified: MSIX packaging, the macOS App
Sandbox, the installers.

## 5. E2E specs written (not run here; the lead runs the full suite)

Feature list from `docs/engineering/ai-tool-coverage.md` (operation classes: read, compute, navigate, proposal; the
human-only review operations; the hidden `ai.*` runtime operations) and the AI specification (assistant, retrieval,
scope, Change Sets, crash safety, local-only). Every spec runs the real bundled model with deterministic sampling and
asserts structure and effects, never wording. One answer takes 20 to 40 seconds on this machine; the helpers wait up to
240 seconds.

| Feature | Spec and test |
|---|---|
| AI ready on first launch: no download, no setup, no model names | `72-ai-assistant` "AI is ready on first launch"; `01-journey` AI step |
| Settings > Offline AI: components, licences, SHA-256, read-only folder; no download, update or remove | `72-ai-assistant` "Settings > Offline AI shows the bundled components" |
| Assistant chat grounded in the project: exact answer marked "from project data", with sources | `72-ai-assistant` "an exact answer comes from project data" |
| Retrieval context: meaning question answered from retrieved passages; a source chip opens its scene | `72-ai-assistant` "a meaning question is answered from retrieved project context" |
| Navigation tool: "Open Scene 2." opens the Screenplay | `72-ai-assistant` "a navigation command opens the place it names" |
| No network during AI use: app and engines own only loopback sockets; engines run from the bundle with `--host 127.0.0.1 --offline`, key not on the command line | `72-ai-assistant` "no network" |
| Engine lifecycle: Stop Offline AI frees the engines; the next question starts them | `72-ai-assistant` "Stop Offline AI" |
| Conversation history after relaunch | `72-ai-assistant` "the conversation is still there after a relaunch" |
| Each workspace's AI entry point and inherited scope (Home, Idea Vault, Story, Screenplay, Breakdown, Production, Call Sheets, Notes & Tasks) and the shortcut | `73-ai-workspaces` "the assistant inherits the scope", "the keyboard shortcut" |
| Scoped questions in Story Board, Screenplay (speaking characters) and Production (locations) | `73-ai-workspaces` (three tests) |
| Proposed Change Set shown; nothing changes before Apply | `74-ai-change-sets` "a requested change arrives as a proposal" |
| Apply, one Undo reverses all of it, Redo | `74-ai-change-sets` "Apply makes the reviewed change" |
| Reject leaves the project unchanged | `74-ai-change-sets` "Reject" |
| Stale proposal never applied blindly; Re-check offered | `74-ai-change-sets` "a proposal whose target changed" |
| Crash-safe apply (ADR-0019): killed on Apply, all or nothing after relaunch, the card agrees | `75-ai-crash-safe-apply` "killed while applying" |
| Killed while the engine answers: no stray engine; AI works after relaunch | `75-ai-crash-safe-apply` "killed while the engine answers" |
| Missing bundled file: AI off with a repair message, no download, core work unaffected | `76-ai-bundle-integrity` "a missing model file" |
| Altered model (same size) caught by the hash check before any engine starts | `76-ai-bundle-integrity` "an altered model" |
| Altered engine library caught before launch | `76-ai-bundle-integrity` "an altered engine library" |
| Whole again: AI answers with the real model | `76-ai-bundle-integrity` "with the files whole again" |

Expected to fail with today's model until the AI quality work (found with `ai_real_model.rs` on 2026-10-08, same
deterministic settings): "Open Scene 2." gets a clarification instead of navigation (Navigate test in `72`); "Summarise
this screenplay" is routed to the product guide; the 1B model proposes "Create a task to scout the railway station." twice
in one Change Set (the tests count the changes on the card, so apply and undo still pass); "How many scene cards are on
the Story Board?" is answered from retrieval, not as an exact count. These are AI quality items for area 8, not
bundling defects.

## 6. Open items

| Item | Owner | Notes |
|---|---|---|
| Gemma redistribution check | Owner | ADR-0025 decision 7: confirm the Gemma Terms of Use allow shipping the weights and tokenizer inside every package, and the same for BGE (MIT) and llama.cpp (MIT, with the LLVM OpenMP licence). Check the notice texts in `packaging/ai/notices/` against the upstream licences (the BGE copyright line is not stated there; the Gemma Terms are linked, not copied in full) before any package leaves the team |
| CI and release runners | Owner | Provide the pinned files to runners (`OPENFRAME_AI_ASSETS_DIR`, about 1.1 GB). Until then `ci.yml` E2E, `release.yml` validate, E2E, build and store fail closed at the bundled AI check |
| Installer sizes and limits | Lead / owner | Build the internal `.exe` when the owner allows and record its size; NSIS and MSI cabinets stay under 2 GB with the AI (about 1.1 GB compressed) |
| Visual C++ runtime | Lead | `llama-server.exe` and its libraries import `VCRUNTIME140.dll`, `VCRUNTIME140_1.dll` and `MSVCP140.dll`; the clean-machine validation of the internal `.exe` must confirm they are present on a clean Windows 11, or the installer must carry them |
| Graphics-card build | Owner | Only the processor build is bundled (the only engine in `.ai-assets`). Adding the Vulkan build needs its files pinned and tested as one set (ADR-0025 decision 4) |
| Mac engine | Later (Mac hardware) | No `macos-aarch64` engine is pinned, so a Mac build fails closed at the bundle check by design; `app_resources_dir` for an app bundle is written and unverified |
| Per-format validation | Lead | ADR-0025 validation list (install, offline inference, upgrade, uninstall) for each package format |
| Web view network use | Lead | The no-network check covers the app and the engines, not the WebView2 processes (threat model T33) |
| `graphify update .` | Lead | Not run in the worktree (the graph lives in the main checkout) |
