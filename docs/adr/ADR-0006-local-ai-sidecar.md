# ADR-0006: Local-AI-only, managed `llama.cpp` sidecar with signed manifests

- **Status:** Accepted. **Implementation in progress** (AI module). Foundation primitives are implemented:
  `openframe_security::{verify_ed25519, sha256_file}`, `ActorOrigin::Ai`, `Capability::{UseAi, ApplyChangeSet}`,
  `ChangeSetState`/`ChangeSetOrigin` enums, and migration slot `0008_ai.sql`.
- **Date:** 2026-09-30
- **Related:** Engineering Index locked decisions (Offline AI, Local AI only, no Ollama); ESD §3, §10;
  OpenFrame_Local_AI_Runtime_Model_Management_Specification.md; ADR-0012 (resolution of external-AI passages)

## Context

The PRD and AI spec describe an assistant that answers project questions, navigates, computes and proposes changes.
The approved build decision is **local AI only**: no cloud providers, API keys, Ollama or accounts. The user must
never install runtimes or models by hand ("one-click **Download Offline AI**"). AI must never be required for core
work, must never mutate directly, and must treat project text as untrusted data (prompt injection).

## Decision

1. **Runtime:** an OpenFrame-managed `llama.cpp` server sidecar, launched only by OpenFrame. It binds to
   **127.0.0.1 on a random free port**, requires a random per-launch bearer token, and never binds `0.0.0.0`. Its
   generic file and shell agent features are disabled. The sidecar process is placed in a Windows **Job Object** with
   kill-on-close, so it cannot outlive OpenFrame. It is started on demand, never at app startup.
2. **Delivery:** the runtime and the model are delivered by **Download Offline AI** from the OpenFrame-controlled
   distribution base URL (build-time `MODEL_DISTRIBUTION_BASE_URL`), described by a **signed manifest**. The manifest
   signature is Ed25519, verified with a public key compiled into the app. TLS alone is not trusted. Each artifact's
   SHA-256 and size are pinned in the manifest. Downloads go to `*.partial` with HTTP range resume and a disk-space
   preflight. Activation is an atomic rename only after hash verification. The last known-good model is kept until the
   new one passes a health check. Store: `%LOCALAPPDATA%\OpenFrame\{models,runtimes}\`. Projects never contain models.
   Developers may place binaries in the gitignored `apps/desktop/src-tauri/binaries/` or `.dev-models/`.
3. **Model policy:** Qwen-family GGUF profiles (Lightweight / Recommended / High Quality), chosen automatically from
   local hardware (RAM, CPU, GPU/VRAM, free disk) with a user override by simple name. The hardware profile never
   leaves the machine. Model file names live only in the manifest.
4. **Authority boundary:** the model emits structured tool requests validated against a strict schema: tool exists,
   args valid, targets exist, actor permitted, source not stale. Reads run as queries with the requesting user's
   permissions (`Actor { origin: Ai }`). Mutations become a **Change Set** that is previewed and explicitly accepted,
   then applied through the normal command pipeline (undoable, in activity). The model never sees SQL, paths or a
   shell.
5. **Prompt-injection boundary:** retrieved project content is delimited and labelled as data, separate from
   system/tool policy and the user request. Tool execution is always authorized by the application, never by model
   text.
6. **Persistence:** AI requests, resolved scope, profile id, results, provenance references and Change Set links are
   stored in the project (`0008_ai.sql`). Hidden chain-of-thought and unnecessary full-context copies are not stored.
7. **Failure isolation:** the model not being installed, a sidecar crash, malformed output or a disk-full download all
   leave the project unchanged and core editing unaffected (invariant 11).

## Consequences

- The base installer stays small and has no AI dependency. AI needs a one-time download of the order of 1–5 GB,
  depending on profile.
- The signing key for manifests becomes release infrastructure (rotation procedure required before GA). The
  distribution hostname is still an open decision (Engineering Index §7), and `release-validate.mjs` fails closed
  without it once the AI module exists.
- Model and runtime licenses must be verified per artifact and shipped as notices (spec §17).
- Quality on low-memory machines is limited. Every deterministic workflow remains available without AI.

## Alternatives rejected

| Alternative | Reason |
|---|---|
| Cloud AI providers / user API keys | Out of scope for v1 (locked decision); contradicts local-first privacy |
| Ollama or other user-installed runtimes | Requires user setup and a background service outside our control (explicitly excluded) |
| In-process inference (llama.cpp linked into the app) | A model crash or OOM would take down the editor and risk unsaved state. There would be no resource isolation. |
| Bundling the model in the installer | Multi-GB installer for a feature that is optional |
| Unsigned manifest over HTTPS | A CDN or DNS compromise would let an attacker swap binaries. The spec requires application-controlled signing. |
