# OpenFrame Studio — Local AI Runtime & Model Management Specification

    **Product:** OpenFrame Studio  
    **Document class:** Implementation / engineering specification  
    **Baseline:** Windows 11 production architecture, v1  
    **Updated:** 30 September 2026 (single Offline AI profile: Gemma 3 1B Instruct + BGE small embeddings; supersedes the three-tier Qwen design)  
    **Purpose:** Define a zero-technical-setup, local-only AI subsystem where filmmakers click **Download Offline AI** and OpenFrame manages everything else.


## Authority and non-conflict rule

This engineering document implements the approved OpenFrame specifications and the agentic AI + hybrid RAG spec (`openframe_ai_agentic_rag_spec_prompt.md`, §1, §12, §23–§25, §36–§37). It does **not** redefine product behavior.

Implementation precedence is:

1. Product PRD — scope, intent, priority and product boundaries.
2. Functional Specification — observable behavior and acceptance contract.
3. UX/UI Specification — presentation and interaction.
4. Domain & Data Specification — canonical identities, relationships, lifecycle and source-of-truth.
5. AI / Import-Export / Offline-Collaboration / Security specifications — cross-cutting specialized contracts.
6. This engineering document — concrete implementation choices.

If an implementation detail conflicts with an authoritative product rule, the implementation detail must change.


## Locked implementation decisions

| Decision | Locked choice |
|---|---|
| Initial OS | Windows 11 first; preserve macOS-ready boundaries |
| Desktop shell | Tauri v2 |
| Core/business logic | Rust owns domain rules; React is presentation/input |
| Offline AI | One-click **Download Offline AI**; OpenFrame installs runtime + models |
| AI profile | Exactly **one** production profile, internal id `openframe-local-ai-v1`; no model picker |
| Language model | Google **Gemma 3 1B Instruct**, GGUF **Q8_0** (chosen by benchmark, §4.2) |
| Embedding model | **BGE small English v1.5**, GGUF Q8_0, 384 dimensions, CLS pooling (MIT) |
| Runtime | Pinned llama.cpp `llama-server` build (CPU and Vulkan builds), two managed sidecars |
| Model distribution | Signed manifest (Ed25519); OpenFrame-controlled endpoint in production |
| AI providers | **Local AI only**; no cloud AI, no API keys, no Ollama |
| Accounts / telemetry | None / none |
| Collaboration | File packages only (LAN collaboration removed) |
| Store build | Microsoft Store builds compile the app updater out; Offline AI downloads are user-initiated |


## 1. Product rule

The user must never be required to install Ollama, Python, CUDA tooling, model managers, command-line utilities or a separate server. OpenFrame is fully usable if AI is never installed.

```text
AI feature clicked
      ↓
"Offline AI is not installed."   + exact download size + free-space check
      ↓
[Download Offline AI]            (one button, no choices)
      ↓
Checking device → Downloading → Verifying → Installing → Starting → Ready
      ↓
AI Ready                         (usable immediately, no restart)
```

User-facing wording (spec §1, §24, §36):

| Situation | Wording |
|---|---|
| Offer | **Enable Offline AI** — "Download the AI components required for OpenFrame's AI features. AI runs on this computer and project data is not sent to OpenFrame or an AI cloud service." |
| Not installed | "Offline AI is not installed." + **[Download Offline AI]** |
| Installed | **AI Ready** (header chip) |
| Steps | Checking device · Downloading · Verifying · Installing · Starting · Ready |
| Paused / failed | "Paused. You can resume the download at any time." / plain reason + **[Retry]** |

The words Gemma, GGUF, Q8_0, embedding model, llama.cpp, sqlite-vec, RRF or context graph never appear in ordinary workflow UI. They appear only in **Settings → Offline AI → Technical details** (component names, versions, licences, sizes, SHA-256, backend, hardware, manifest channel, store folder, log file), in third-party notices, the privacy policy, the user manual and technical documentation. Tests enforce this (`crates/openframe-application/tests/ai.rs`, `apps/desktop/src/ai/OfflineAiSetup.test.tsx`, E2E journey).

After a successful download, AI use requires no internet.

## 2. Components

```text
React: OfflineAiSetup (panel + Settings)       Settings → Offline AI → Technical details
   ↓ ai.status / ai.setup_info / ai.install / ai.cancel_install / ai.uninstall / ai.stop_runtime / ai.diagnostics
openframe-application  modules/ai/offline.rs (+ service.rs: install task, progress events)
   ↓
openframe-ai  AiManager
   ├─ manifest    signed manifest v2: profile = runtime + chat model + embedding model
   ├─ hardware    RAM / CPU / GPU (DXGI) / Vulkan loader / free disk — never transmitted
   ├─ selection   device check: graphics card or processor, layers, disk needed, "may be slow"
   ├─ download    resumable, retried, SHA-256 verified, quarantining downloads
   ├─ store       %LOCALAPPDATA%/OpenFrame/{models,runtimes} + atomic active.json
   ├─ supervisor  chat sidecar  (llama-server, loopback, -c 8192)
   │              embedding sidecar (llama-server --embedding --pooling cls, CPU only)
   └─ model       ChatModel adapter; AiManager::embed / embedding_info (C1 API)
```

Neither model has any project-database or filesystem authority. The chat model produces text or grammar-constrained JSON; the embedding model produces vectors. The application validates both.

## 3. Runtime

OpenFrame downloads a pinned llama.cpp release (`b11259`, MIT; Windows x64 CPU build and Vulkan build) listed in the manifest with exact bytes and SHA-256.

Production rules (unchanged, enforced in `supervisor.rs`):
- launched only by OpenFrame, with a fixed argument list; `LLAMA_*` environment overrides are stripped;
- bound to `127.0.0.1` on a random free port — never `0.0.0.0`;
- a random 256-bit per-launch API key passed through the environment (`LLAMA_API_KEY`), never the command line;
- `--no-webui --no-slots --offline --parallel 1`, no console window;
- a kill-on-close Job Object: the sidecars can never outlive OpenFrame;
- stdout/stderr go to sanitized, redacted, size-rotated logs (`logs/ai-runtime.log`, `logs/ai-embedding.log`);
- lazy start, idle unload after 15 minutes, bounded automatic restart after a crash (2 within 10 minutes), explicit retry after a start failure.

Chat sidecar: `-c 8192 -ngl <layers> -t <physical cores ≤16> --reasoning off --alias openframe-local`.  
Embedding sidecar: `--embedding --pooling cls -c 512 -b 512 -ub 512 -ngl 0 -t <≤4> --alias openframe-embedding` (one input always fits one batch; it stays on the processor so graphics memory is left to the chat model).

## 4. Model policy

One profile, `openframe-local-ai-v1`. The manifest may move it to a new version (new model files, new runtime); the id never changes and users never choose a model.

### 4.1 Benchmark method (`scripts/ai-benchmark.mjs`)

Candidates are downloaded into the gitignored `.dev-models/` and SHA-256 checked. Each is run through the **real runtime path** — the pinned `llama-server` with the same flags the supervisor uses — and measured in the order of the product priorities:

1. **Tool calling** — the real OpenFrame planner request captured from the application (`scripts/ai-benchmark/planner-request.json`: policy text, 23 tools + 7 proposal tools, grammar-constrained `oneOf` schema), 18 requests from the §35 evaluation categories (exact counts, navigation, search, unscheduled scenes, character co-occurrence, task/scene-card/rename/status proposals, product help, private notes, workspace navigation, summaries, breakdown suggestions, draft comparison, an ambiguous "Change it."), scored for schema-valid JSON, correct tool and correct key arguments; the same requests without a grammar (raw JSON discipline); one prompt-injection case (hostile text in the project-data block).
2. **Summarization / creative** — scene summary (grounding check: names + setting present) and three alternative endings (saved for review).
3. **Latency** — load, planner p50/p95, prompt and generation tokens/s.
4. **Download size.** 5. **Memory** — peak working set of the server process.

### 4.2 Results and decision (30 Sept 2026)

Machine: Intel Core Ultra 5 125H (14 cores / 18 threads), 16 GB RAM, Intel Arc integrated graphics, Windows 11, CPU build, 14 threads. **Measured while four other build agents were compiling on the same machine**, so absolute latencies are pessimistic and noisy; relative ranking is what the decision uses.

| Gemma 3 1B Instruct file | Download | Valid JSON (grammar) | Tool + args correct | JSON without grammar | Injection resisted | Planner p50 / p95 | Prompt / gen tok/s | Load | Peak RAM | Summary grounded |
|---|---:|---:|---:|---:|---|---:|---:|---:|---:|---|
| Q4_K_M (ggml-org) | 769 MB | 100% | 22.2% | 88.9% | yes | 7.9 s / 54.8 s | 16 / 5 | 2.3 s | 1.18 GB | yes |
| QAT Q4_0 (ggml-org) | 687 MB | 100% | 33.3% | 88.9% | yes | 10.7 s / 26.1 s | 13 / 3 | 5.4 s | 1.38 GB | yes |
| **Q8_0 (ggml-org)** | **1020 MB** | **100%** | **38.9%** | **94.4%** | **yes** | **4.4 s / 13.9 s** | **28 / 8** | **2.7 s** | **1.35 GB** | **yes** |

Embedding model (BGE small en v1.5 Q8_0, 35 MB): 384 dimensions, returned vectors L2-normalised, cosine(similar scene pair) 0.859 vs cosine(unrelated) 0.473, 32 chunks of ~600 characters in 9.9 s under the same load, 67 MB peak memory. An input over the 512-token window is refused by the server (HTTP 500), so OpenFrame bounds inputs to 1 500 characters and retries a refused input shorter (§9.1).

**Decision: Q8_0.** It is best on priority 1 (tool + arguments 38.9% vs 33.3% / 22.2%, raw JSON 94.4%) and on latency (Q8_0 dequantisation is cheap on CPU), with memory equal to the QAT file; the cost is priority 4 (≈330 MB larger download). Full Offline AI download: **1 125 156 323 bytes** (runtime 19.2 MB + Gemma 1 069.3 MB + BGE 36.7 MB) on a processor-only computer.

Findings for the agent layer (owned by the orchestrator, recorded here because they came out of the benchmark), and how the merged code applies them:
- **Key order matters.** llama.cpp emits properties in schema order; `serde_json` sorts keys, so the captured planner schema listed `"arguments"` before `"tool"` and the model filled in arguments before choosing the tool. The benchmark puts `"tool"` first. *Applied:* the step schema names the tool with the key `"action"`, which sorts before `"arguments"`, so the model names the tool first (`orchestrator::step_schema`; `"tool"` is still accepted when parsing older transcripts).
- A 1B model choosing among 30 tools in one shot is weak (≈39%). *Applied:* each request is offered the core tools plus the request-relevant domain tools within a 7,000-character catalogue (`toolbox::tools_for_request`; proposal tools only when the request asks for a change), exact facts come from deterministic tools, and grammar-constrained output keeps every reply schema-valid (100% in all runs). See the AI specification §8.

### 4.3 Backend (CPU / Vulkan)

The Vulkan build is chosen only when a real (non-software) adapter has **dedicated** memory ≥ the chat model's `gpuVramBytes` + 512 MB and a Vulkan loader is installed. Integrated graphics report little dedicated memory (they share system RAM) and use the processor build. If the Vulkan runtime fails its health check at install time, OpenFrame downloads the processor build (added to the same progress figure), health-checks it and activates it instead; the unused runtime is then removed.

## 5. Device check (no picker)

Collected locally, never transmitted: OS/arch, total/available RAM, logical/physical CPUs, GPU adapters and dedicated memory (DXGI), Vulkan loader presence, free disk space.

`ai.setup_info` returns, before anything is downloaded: exact bytes still to download (partial downloads excluded), full size, bytes already downloaded, free space needed (remaining download + 3 × runtime archive + 512 MB margin), free space on the drive, "Graphics card" / "Processor", and plain warnings ("may take a while" below 8 GB RAM or 4 threads; "less memory than Offline AI normally needs" below 4 GB). Low-memory computers are warned, not refused. Nothing is downloaded when space is short.

## 6. Manifest (format 2)

```json
{
  "manifestVersion": 2, "sequence": 2, "channel": "dev", "issuedAt": "2026-09-30",
  "profile": { "profileId": "openframe-local-ai-v1", "version": "1.0.0",
               "chatModelId": "gemma-3-1b-it-q8-0-v1", "embeddingModelId": "bge-small-en-v1.5-q8-0-v1" },
  "runtimes": [ { "runtimeId": "llama-cpp-b11259-win-x64-cpu", "engine": "llama.cpp", "version": "b11259",
                  "backend": "cpu", "os": "windows", "arch": "x86_64", "url": "https://…", "bytes": 19164803,
                  "sha256": "…", "archive": "zip", "executable": "llama-server.exe", "licenseId": "MIT" }, … ],
  "models": [
    { "modelId": "gemma-3-1b-it-q8-0-v1", "role": "chat", "displayName": "Gemma 3 1B Instruct", "family": "gemma3",
      "quantization": "Q8_0", "version": "f9c28bc", "url": "https://…", "bytes": 1069306368, "sha256": "…",
      "licenseId": "Gemma-Terms-of-Use", "licenseUrl": "https://ai.google.dev/gemma/terms", "contextTokens": 8192,
      "minRamBytes": 4294967296, "recommendedRamBytes": 8589934592, "gpuVramBytes": 1879048192, "layers": 26 },
    { "modelId": "bge-small-en-v1.5-q8-0-v1", "role": "embedding", "displayName": "BGE small English v1.5", "family": "bert",
      "quantization": "Q8_0", "version": "f2068ed", "url": "https://…", "bytes": 36685152, "sha256": "…",
      "licenseId": "MIT", "licenseUrl": "https://huggingface.co/BAAI/bge-small-en-v1.5", "contextTokens": 512,
      "minRamBytes": 268435456, "recommendedRamBytes": 536870912, "gpuVramBytes": 134217728, "layers": 12,
      "embeddingDim": 384, "pooling": "cls" } ]
}
```

Validation (`manifest::validate`): format 2 only (format 1 — the retired tiers — is refused); profile id must be `openframe-local-ai-v1`; every id a single safe path component; HTTPS URLs (plain HTTP only for loopback mirrors; userinfo refused; every redirect hop re-checked); licence URL HTTPS; non-zero exact bytes; lowercase SHA-256; zip runtimes with a safe relative `.exe`; unique ids; the profile's chat model has role `chat` and ≥ 2048 context; its embedding model has role `embedding`, a dimension 8–4096 and a pooling mode.

Trust: Ed25519 signature over the exact bytes, public key compiled in. Development builds trust `crates/openframe-ai/keys/manifest-dev.pub` (rotated 30 Sept 2026; the private key is kept outside the repository by the owner). Public builds set `OPENFRAME_MANIFEST_PUBLIC_KEY`; `scripts/release-validate.mjs` fails a release that still trusts the development key, and an embedded manifest not signed by the trusted key is never used. A cached/remote manifest with a lower `sequence` never replaces a newer one; a cache that fails verification is deleted. Signing: `node crates/openframe-ai/tools/sign-manifest.mjs <manifest.json> <private-key.pem>`.

## 7. Download manager

- one combined progress for the whole package: `done`/`total` are bytes across runtime + chat model + embedding model, monotonic, total fixed (grows only when the processor runtime is added after a graphics-card failure);
- resume with HTTP range requests (clean restart when unsupported or the server resumes at another offset); a partial for another URL/hash is discarded, never spliced;
- **Pause** keeps partial files; **Cancel** discards them; a paused download shows what is already on disk and resumes from there;
- retries with exponential backoff; stall timeout; more bytes than promised → quarantine;
- size + SHA-256 verified before a file is moved (atomic rename) into place; a mismatch is quarantined (last 3 kept) and never used;
- runtime zips extracted with archive limits (entries, total size, ratio, path safety) into a temp folder, then renamed.

## 8. Storage

```text
%LOCALAPPDATA%/OpenFrame/            (Microsoft Store edition: %LOCALAPPDATA%\Packages\<PackageFamilyName>\LocalState\OpenFrame, 27-microsoft-store.md §5.2)
├─ models/
│  ├─ manifest-cache.json (+ .sig)
│  ├─ active.json                     { schema: 2, profileId, profileVersion, runtimeId, backend,
│  │                                    chatModelId, embeddingModelId, activatedAt }
│  ├─ .downloads/  .quarantine/
│  ├─ gemma-3-1b-it-q8-0-v1/{model.gguf, metadata.json, integrity.json}
│  └─ bge-small-en-v1.5-q8-0-v1/{model.gguf, metadata.json, integrity.json}
├─ runtimes/llama/llama-cpp-b11259-win-x64-cpu/
└─ logs/ai-runtime.log, logs/ai-embedding.log
```

Components are shared by all projects; projects contain no model copies (the per-project `cache/intelligence.sqlite` semantic index is derived data owned by the retrieval layer). An `active.json` in the retired format reads as "not installed"; its leftovers are removed by the next successful install or by uninstall.

## 9. Sidecar lifecycle

States: NotInstalled → Installed → Starting → LoadingModel → Ready ⇄ Busy → Stopping; Failed. Both sidecars share the supervisor implementation; `ai.status` reports the chat sidecar, diagnostics report both. Model loading never blocks the UI (the install and every request run on background threads).

### 9.1 Embeddings (C1 contract)

```rust
pub struct EmbeddingInfo { pub model_id: String, pub version: String, pub sha256: String, pub dim: u32 }
impl AiManager {
    pub fn embedding_info(&self) -> Option<EmbeddingInfo>;                     // None when not installed
    pub fn embed(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, AiError>; // AiError = AppError
}
```

`embed` is blocking (call from a background worker), lazily starts the embedding sidecar, sends batches of 32 to `/v1/embeddings`, and returns one L2-normalised 384-value vector per input in input order (count, index, dimension and finiteness are checked). Inputs are trimmed, empty inputs become a space, and each input is cut to 1 500 characters; if the runtime still refuses an input as too long, that batch is retried one input at a time and only the refused input is shortened (halving, down to ~100 characters). Errors: `ai.not_installed`, `ai.unavailable` (sidecar down), `ai.malformed`, `ai.embedding_rejected`. `model_id` + `version` + `sha256` belong in the semantic index metadata: a different embedding model means the index is rebuilt.

## 10–13. Context, tools, prompt-injection boundary, persistence

Owned by the orchestrator / retrieval / tool layers and specified in `OpenFrame_Studio_AI_Specification_Updated.md` §5–§13 (agent loop, toolbox, Change Sets, prompt-injection boundary, persistence) and `docs/engineering/18-retrieval-intelligence-index.md` (derived index, hybrid retrieval, context graph). The runtime contributes: grammar-constrained JSON output, no hidden reasoning (`--reasoning off`, `<think>` blocks stripped), and a model reference (`openframe-local-ai-v1:<chat model id>`) recorded with each AI request. No chain-of-thought is persisted.

## 14. Failure behavior

| Failure | Required behavior |
|---|---|
| Not installed | "Offline AI is not installed." + Download Offline AI; core app unaffected |
| Manifest unverifiable | Nothing downloaded or run; clear message |
| Offline during download | Retries, then fails safely; partial file kept and resumable |
| Hash mismatch | Quarantined; previous install untouched and still active |
| Insufficient disk | Download does not start; needed / free / missing space explained |
| Graphics runtime can't start | Processor runtime downloaded, checked and used |
| New version can't start | Previous install stays active (rollback); verified files kept for retry |
| Sidecar crash | Project unchanged; automatic bounded restart; explicit retry |
| Embedding runtime unavailable | Retrieval falls back to FTS + SQL (agentic spec §33) |
| Malformed model output | Rejected; never applied |

## 15. Update and removal

A newer signed manifest whose profile version or model ids differ shows **Update Offline AI** (panel menu and Settings). Only changed components are downloaded (exact size shown first). The current install keeps working until the new one passes its health check; only then does `active.json` switch, and only then are superseded model/runtime folders (and retired-profile leftovers) deleted. Nothing is updated silently.

**Remove Offline AI** (Settings / panel menu, with confirmation) stops both sidecars and deletes every model, runtime and partial download; links/junctions inside the AI folders are removed without following them; nothing outside `models/` and `runtimes/llama/` is touched. Projects and assistant history are unaffected; Offline AI can be downloaded again.

## 16. Verification

Automated (no real model): manifest trust and format validation; tampered/foreign-key/retired-format manifests; cache tampering and rollback protection; exact plan without network; hash mismatch keeps the previous install; verified-but-unstartable components are not activated; `tests/lifecycle.rs` runs the whole install through the real code with the test executable acting as `llama-server` — combined monotonic progress and step order, both sidecars health-checked, chat and embeddings usable without restart and after restart, update with clean-up, Vulkan → CPU fallback, rollback of a failing new version, pause/resume with range requests, cancel, uninstall (including a junction that must not be followed), embedding batching/normalisation/too-long retry.

Manual with the real components: `OPENFRAME_REAL_AI_DIR=.dev-models cargo test -p openframe-ai --test real_runtime -- --ignored --nocapture` (plan → install → chat → grammar-constrained JSON → embeddings → uninstall), and `node scripts/ai-benchmark.mjs` for model/quantization changes. A profile version cannot ship until both pass on minimum and recommended hardware.

Real-component validation, 30 Sept 2026 (same machine, under build load, files served from a loopback mirror of `.dev-models`): plan 1 125 156 323 bytes on the processor build (1.70 GB free space required); install incl. SHA-256 verification, extraction and both health checks 10.5 s; first chat answer 3.2 s; grammar-constrained tool JSON 4.2 s; 32 embeddings 0.62 s; cosine 0.859 (similar) vs 0.473 (unrelated); a 3 000-word input was shortened and embedded; uninstall freed 1 155 182 990 bytes. Not yet measured: the Vulkan build on a discrete graphics card, and minimum-spec (8 GB) hardware — release prerequisites.

## 17. Licensing

- llama.cpp — MIT (Windows build also contains the LLVM OpenMP runtime, Apache-2.0 WITH LLVM-exception).
- Gemma 3 1B Instruct — Gemma Terms of Use (https://ai.google.dev/gemma/terms) and the Gemma Prohibited Use Policy. OpenFrame must show the notice "Gemma is provided under and subject to the Gemma Terms of Use found at ai.google.dev/gemma/terms." (Settings → Offline AI → Technical details and THIRD_PARTY_LICENSES.md) and pass the terms on when redistributing through its own endpoint.
- BGE small English v1.5 — MIT.

`npm run licenses` lists these components (from the manifest) in `THIRD_PARTY_LICENSES.md`.

## 18. Privacy

Normal inference: React → Rust → local context → local llama.cpp → local Gemma → result. Project data is never sent to Google, OpenFrame or any AI service because the weights came from Google. The AI subsystem uses the network only for user-initiated component downloads and the signed manifest. No telemetry of prompts, screenplay text, retrieved chunks, private notes or AI responses exists. See `docs/store/privacy-policy.md`.

## 19. Explicit exclusions

Cloud AI providers; user API keys; Ollama; model picker or user-loaded models; silent downloads or updates; autonomous project edits; shell/browser control by the model; mandatory AI for app operation.
