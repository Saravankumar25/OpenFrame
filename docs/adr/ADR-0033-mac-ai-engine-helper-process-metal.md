# ADR-0033: On the Mac the offline AI engine is a helper process that uses Metal

- **Status:** Accepted 2026-10-18 (owner delegated the decision on 2026-10-18; the lead agent of Mac phase M8 made it).
- **Date:** 2026-10-18
- **Answers:** [ADR-0027](ADR-0027-rust-core-two-ui-shells.md) open sub-decision 2 (helper process or linked library).
- **Builds on:** [ADR-0026](ADR-0026-offline-ai-download-after-install.md) (the runtime ships in the package, the models
  download after install), [ADR-0028](ADR-0028-ai-quality-program-and-harness.md) (measure first),
  [ADR-0029](ADR-0029-gpu-acceleration.md) (the Windows graphics decision; its closing line promised a Mac decision),
  [ADR-0032](ADR-0032-models-from-hugging-face.md) (models from pinned Hugging Face files).

## Context

ADR-0027 left one question open for the Mac: run the `llama.cpp` engine as a separate helper program, as Windows does, or
link it into the app as a library. The App Sandbox, which the store build needs in Chapter 3, allows a helper executable
that is embedded in the app bundle and signed with the `com.apple.security.inherit` entitlement; it then runs in the same
sandbox as the app. The shared core already starts, watches, restarts and stops the engine (`crates/openframe-ai`,
`crates/openframe-platform/src/process.rs`, which has macOS code), and the Windows evidence for that code (supervisor
tests, soak, crash and restart, scrubbed environment, per-launch hash check) was gathered with a helper process.

## Decision

1. **A helper process.** On the Mac the engine is the official `llama-server` program, started by the same supervisor as on
   Windows. `llama.cpp` is not linked into the app or into the Rust core.
2. **Why.** The code path and its evidence are the same as Windows. An engine crash, a bad model file or an out-of-memory
   stop ends the helper and never the editor, and the supervisor restarts it. Under the App Sandbox a helper embedded in
   the bundle inherits the sandbox, so nothing here blocks Chapter 3. Linking would also bring every `llama.cpp` update
   into our build and our crash reports.
3. **One build, Metal inside.** The runtime lock gains one entry for Apple silicon: backend `metal`, the official
   `ggml-org/llama.cpp` release asset `llama-b11259-bin-macos-arm64.tar.gz`, the same release tag `b11259` as the Windows
   entries. That build holds the Metal code and the processor code, so the Mac has one main build per platform where
   Windows has the processor build plus an optional Vulkan build. Only `llama-server` and the libraries it loads are
   pinned, each by size and SHA-256, with the `LICENSE` file.
4. **Where the engine runs.** Every Apple silicon Mac has one Metal device that shares the processor's memory. The chat
   engine runs on it (`-ngl 999 --device MTL0`) unless the setting Graphics card is Off, the memory is smaller than the
   model plus a margin and a reserve, or an earlier session recorded a failure on this device and engine. No timing check
   runs first: Windows needs one because a graphics card can be slower than the processor, but on Apple silicon the Metal
   build was measured many times faster (evidence in `docs/release/evidence/mac/m8-ai-evaluation.md`). If the engine stops
   while in use the supervisor falls back to the processor, as ADR-0029 decision 5 says, and the failure is remembered. The
   embedding engine stays on the processor (ADR-0029 decision 7).
5. **Layout and checks are unchanged.** The package carries `Contents/Resources/ai/runtimes/<runtime-id>/<files>` and
   `notices/`. The core re-hashes every engine file before every launch and refuses links and added program files. The
   archive keeps `libggml.0.dylib` as a link to `libggml.0.25.3.dylib`, so the install copies each file under the name the
   engine loads, as a regular file. Every library finds its neighbours through `@loader_path`, so no `install_name_tool`
   change is needed. `packaging/ai/fetch-macos-runtime.sh` downloads, verifies and lays this out.
6. **Signing later (Chapter 3).** The engine and its libraries are signed with the app's identity and hardened runtime,
   and `llama-server` gets `com.apple.security.inherit`. Until then internal builds are unsigned (ADR-0030). The libraries
   in the release archive carry only an ad-hoc signature from the upstream build.
7. **Apple's on-device model** stays a separate, opt-in provider behind the `InferenceProvider` interface (phase M8). It
   needs no helper process and is not the default.

## Consequences

- The lock validation accepts a `metal` build as the one main build of a platform (instead of a `cpu` build). Windows
  entries are unchanged, so the Windows evidence stays valid.
- `scripts/prepare-ai-assets.mjs` and the lock check in the Rust crate know the `metal` backend.
- The Intel Mac has no pinned engine yet. Its entry (the `macos-x64` asset, processor only) is added when phase M8 measures
  the Intel Mac; until then the Offline AI says it is not available there.
- A Metal run uses unified memory: the 1B chat model plus its cache is about 1.5 GB. The hardware check in
  `selection.rs` is unchanged.
