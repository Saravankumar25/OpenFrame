# ADR-0032: The models come straight from Hugging Face, pinned in the app

- **Status:** Accepted 2026-10-16 (owner, in chat: "Download model files directly from public Hugging Face repositories
  using pinned revisions and SHA-256 verification. No self-hosted server, S3, R2, Ollama installation, or cloud backend.
  Support resumable downloads, local caching, manual import, and fully offline inference after download. Verify model
  licensing before release.")
- **Date:** 2026-10-16
- **Amends:** [ADR-0026](ADR-0026-offline-ai-download-after-install.md). Supersedes its decision 4 (signed manifest) and
  decision 8 (owner's model host and keys, `MODEL_DISTRIBUTION_BASE_URL`, `OPENFRAME_MANIFEST_PUBLIC_KEY`). Every other
  decision of ADR-0026 stands: the runtime in the package, download only on the person's click, `.partial` with resume,
  preflight, atomic activation, quarantine, last good set kept until the health check passes, the models in app data, the
  validation per format.

## Context

ADR-0026 asked the owner to run an HTTPS model host and to keep an offline Ed25519 key that signs the download list. The
pinned model files already live on public Hugging Face repositories (`ggml-org/gemma-3-1b-it-GGUF`,
`ggml-org/bge-small-en-v1.5-Q8_0-GGUF`), and the committed lock `packaging/ai/ai-assets.lock.json` pins each one by commit
revision, size and SHA-256. Checked 2026-10-16 with header requests only: both answer without a login, report the pinned
commit and size, and redirect to Hugging Face's own download servers on `*.hf.co`.

## Decision

1. **Source.** The app downloads each model from its Hugging Face `resolve` address at the commit revision pinned in the
   lock. No server of OpenFrame's own, no storage bucket, no cloud backend, no Ollama.
2. **Trust.** The sizes and SHA-256 hashes compiled into the app (the lock) are the trust root. A file is used only when it
   matches them exactly. There is no signed list and no signing key: the app itself is the signed, shipped artifact (signing
   at Chapter 3).
3. **Network rules.** HTTPS only. The first address must be `huggingface.co`; redirects are followed only to
   `huggingface.co` or a host under `hf.co`, at most five. No cookie, no token, no account. A test build may name a loopback
   test host instead (build value `OPENFRAME_TEST_MODEL_SOURCE`); a release build accepts it only with `OPENFRAME_TEST_BUILD=1`.
4. **Resume and cache.** Unchanged from ADR-0026: `.partial` files with range resume, kept across sessions, checked before
   use, activated by one rename into `<app data>/models/<version>/`.
5. **Manual import.** Settings, Offline AI offers "Import a model file…": the person picks a file (for example one copied
   from another computer); it is used only when its size and SHA-256 match a pinned model, and it is copied, never moved.
6. **Offline.** After the download (or import) the AI runs without any network.
7. **Licences.** The Gemma Terms of Use and the MIT licences stay in the notices file and Settings. The owner handles
   licence compliance (owner decision 2026-10-16).
8. **Model set (owner decision 2026-10-16, amended the same day).** Three pinned files: Gemma 3 1B Instruct Q8_0
   (`ggml-org/gemma-3-1b-it-GGUF`, chat), BGE small English v1.5 Q8_0 (`ggml-org/bge-small-en-v1.5-Q8_0-GGUF`, English
   search) and multilingual E5 small Q8_0 (`TwinSunsLLC/multilingual-e5-small-gguf` at `b6cac96`, search in every other
   language). The official E5 repository has no GGUF; of the public conversions, `cstr/multilingual-e5-small-GGUF` does not
   load in llama.cpp b11259 ("bert model needs to define token type count") and the TwinSuns file does. It placed all 110
   test sentences in eleven languages next to their English originals, where BGE managed 1 of 10 in each Indic language
   (`engine-bench --exp embed`, evidence in `docs/release/evidence/w5/`). E5 needs the prefixes `query: ` and `passage: `,
   carried by the lock. A project index uses one search model; changing it re-embeds the project.

## Consequences

- No owner host or key to run. The owner tasks "choose the model host" and "generate the production manifest key" close.
- A new model reaches people with a new app version (a new lock), not as a separate download list. Replay of an older list
  no longer applies.
- If Hugging Face removes a file or a repository, the download stops with a plain message; manual import still works.
- The privacy policy names Hugging Face as the place the files come from and what it receives (the request and the IP
  address, like any website; no project data, no account).

## Alternatives rejected

| Alternative | Reason |
|---|---|
| Keep ADR-0026's signed list on an owner host | Owner decision: no self-hosted server or cloud backend |
| Ollama or another model manager | Owner decision; a second program to install and trust |
| Follow any redirect | A redirect must stay on Hugging Face's own hosts |
