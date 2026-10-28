# ADR-0026: Offline AI downloaded after install, on the user's click

- **Status:** Accepted 2026-10-15 (owner, in chat: "revert back to download after install"). Permanent and cross-platform.
- **Date:** 2026-10-15
- **Supersedes:** [ADR-0025](../history/adr/ADR-0025-offline-ai-bundled-in-every-package.md) in full.
- **Reinstates** the download clauses that ADR-0025 superseded: [ADR-0006](ADR-0006-local-ai-sidecar.md) decision 2
  (delivery by **Download Offline AI** with a signed manifest), [ADR-0013](ADR-0013-single-local-model-hybrid-rag-agent.md)
  decision 2 ("One download"), and [ADR-0020](ADR-0020-ai-runtime-packaging-per-channel.md) decisions 1 (weights download
  part), 3 and 5. Their text stays in place with a pointer; read the rules below, which are the current wording.
- **Keeps:** ADR-0020 decision 1 for the runtime (the `llama.cpp` runtime and OpenMP runtime ship inside every release
  file), ADR-0020 decision 4 (Report AI output), ADR-0006 decisions 1 and 4 to 7, ADR-0013 except decision 2.
- **Related:** [ADR-0017](ADR-0017-cross-platform-windows-and-macos.md), [ADR-0021](ADR-0021-diagnostics-telemetry-analytics-policy.md)
  (nothing sent automatically), [ADR-0024](ADR-0024-internal-team-builds.md) (release files);
  `crates/openframe-ai/`; `docs/specs/OpenFrame_Local_AI_Runtime_Model_Management_Specification.md`;
  `docs/implementation/masterplan.md` area 8; `docs/implementation/owner-decisions.md`.

## Context

ADR-0025 (2026-10-08) bundled the whole offline AI, about 1.1 GB, into every package so that AI worked on the first
launch without a network connection. The owner now wants the installer to stay small. The application file should be
about 20 to 30 MB, and users who want AI download it once, after installing, from inside the app.

The owner accepts the trade-offs this brings: the first AI setup needs internet, a model host must be run and paid for,
and the store listings and privacy policy must describe the one network use.

## Decision

1. **The package contains the app and the AI runtime.** The `llama.cpp` runtime, the OpenMP runtime and the Microsoft
   Visual C++ runtime DLLs the engine needs ship app-local inside every release file (ADR-0020 decision 1). The runtime
   is about 43 MB and is not downloaded.
2. **The AI models are downloaded after install.** The Gemma 3 1B Instruct chat model (Q8_0, about 1,069 MB), the BGE
   small English embedding model (about 37 MB) and their tokenizer data are not in the package. They download as one
   combined, resumable set.
3. **Download happens only after an explicit user click.** The first run and the AI settings show **Download OpenFrame
   AI** with the size, the free space needed and a plain statement that the download uses the internet. The app never
   starts a download on its own, never downloads as a repair, and never downloads at install time.
4. **Superseded 2026-10-16 by [ADR-0032](ADR-0032-models-from-hugging-face.md) (the sizes and SHA-256 in the app are the
   trust root; no signed list).** **Signed manifest.** The download is described by a manifest signed with Ed25519. The app verifies the signature with
   a public key compiled into the app. TLS alone is not trusted. The manifest gives each file's name, size and SHA-256
   hash, and the profile version (ADR-0013 format 2).
5. **Verify, then activate.** Files download to `*.partial` with HTTP range resume. A disk-space preflight runs before the
   download starts. A file becomes usable only after its size and SHA-256 match, by an atomic rename. The last
   known-good model is kept until the new one passes a health check. A bad file is quarantined and the user is told in
   plain words.
6. **Storage.** Models live in `%LOCALAPPDATA%\OpenFrame\models\` (Windows) and the platform app-data folder on the Mac,
   once per user, never per project. Projects never contain models. The Mac App Store build keeps the models in its
   container, inside App Sandbox.
7. **Without the AI.** Before the download, and if it is removed, OpenFrame works fully. AI features show "not installed"
   with the download button. Nothing else changes (ADR-0006 decision 7).
8. **Superseded 2026-10-16 by [ADR-0032](ADR-0032-models-from-hugging-face.md) (the files come from Hugging Face; no owner
   host, no key).** **Model host and keys.** The owner chooses and runs the HTTPS model host. The owner generates the production Ed25519
   manifest key and keeps the private key offline. The build sets `MODEL_DISTRIBUTION_BASE_URL` and
   `OPENFRAME_MANIFEST_PUBLIC_KEY`, and the release build fails if either is missing. The host serves the file list and
   the model files only; it receives no project data.
9. **Licensing.** The Gemma terms still apply. Hosting the weights for download is distribution. Before the host goes
   live, the owner confirms the redistribution terms, the Gemma notice ships in the app and in the third-party notices
   file, and the derivative terms are checked (ADR-0025 decision 7 stays in force as a check).
10. **Validation per format.** Each package is tested on a clean machine for: install with no network; first download
    with a network; an interrupted download resumed; a corrupted file refused; offline inference after the download;
    starting with the network off and no download, which shows the not-installed state; upgrade keeps the downloaded
    models when their versions match; uninstall removes the app and leaves projects in place.

## Consequences

- **Installer size.** The package drops from about 1.1 GB to about 20 to 30 MB. The 20 to 30 MB is an estimate. The last
  measured release executable was 18.2 MB (2026-09-30); the real package size is measured and recorded when first built.
- **First AI use needs internet once.** Offline AI is available after that download. The offline claim in the store
  listings and manual becomes "AI works offline after its one-time download".
- **Network use is disclosed.** The download is a user-clicked network use. The `internetClient` capability on the
  Microsoft Store package is justified by this download and by links and update checks. The privacy policy names the
  model host and what it receives.
- **Owner tasks are back.** "Choose the model host", "generate the production manifest key" and the Gemma redistribution
  check are open owner tasks again (`owner-decisions.md`).
- **Code must be rebuilt.** ADR-0025 removed the download code. Re-adding it is a new implementation task in
  masterplan area 8 with its own tests. Nothing is restored from history without review.
- **Model updates ship as downloads.** A new model version is a new manifest, so the app can update the AI without a
  new installer. The runtime still ships in the app.
- **Documents.** Store, privacy, QA, release, CI and specification documents describe the download as the target design,
  and say which parts the current code does not yet do. Where a document disagrees with this decision, this decision
  wins.

## Implementation notes (2026-10-16, Windows phase W8)

> Notes 1, 4, 5 and the signed-list parts of 6 describe the first build of W8 and are superseded by ADR-0032 the same day.

How the code meets the decisions above. None of this changes a decision; each point is a choice the decisions left open.

1. **What is trusted.** The manifest envelope is `{ format, keyId, payload, signature }`, Ed25519 over the exact payload
   bytes. The envelope is kept with the activated set as `set.json`, and on every start the app checks its signature
   again before it reads the sizes and hashes it checks the model files against. Editing `set.json` or a model file in
   the writable app data folder therefore never makes the app trust another file. The compiled lock still pins the
   engine and gives the model profile (context window, vector size, pooling, memory); the manifest's
   `profileVersion` must match it (2), and its tier must name the engine version this app carries.
2. **Tiers.** This build downloads only the `small` tier (the models the lock describes). The larger tier waits for the
   W6 model choice.
3. **Layout.** `<app data>/models/staging/<version>/<name>.partial` while downloading, `<app data>/models/<version>/`
   after activation, `active.json` naming the set in use and the previous one until the health check passes,
   `quarantine/` for a file or set that failed. The health check is the full hash check plus the engine starting on the
   new set; a failure rolls back to the previous set, or to none.
4. **The local test host.** Every test build (debug and release, run by `tests/e2e/run.mjs`) is compiled with
   `MODEL_DISTRIBUTION_BASE_URL=http://127.0.0.1:47811/` and the public half of a throwaway test key made in the
   git-ignored build cache (`scripts/model-manifest.mjs testHost()`). Plain HTTP is accepted only when the compiled-in
   address is that loopback address; the address cannot change at run time, and `build.rs` refuses any other
   non-https address. Such a build prints that it is a test build.
5. **The owner's tool.** `node scripts/model-manifest.mjs new-key` (the owner, once, offline) and `publish` (signs the
   list and lays out the host folder) are the only ways a key or a list is made. The private key never enters the
   repository or a build computer.
6. **Security review 2026-10-16 (no critical or high finding), fixed in the same phase.** The signed list carries a
   `sequence`; the app refuses a list older than the highest one it has activated, so an old list replayed by a host or
   network cannot take it back to older models (rolling back after a failed health check keeps the higher number).
   Model files are hashed again in every session (the shortcut record is used for the engine only) and are held open
   for reading only, from just before that check until the engines stop, so no other program can change, rename or
   delete them meanwhile. Only the newest quarantined item is kept, and Remove deletes the quarantine too. Names the
   models folder uses itself (`staging`, `quarantine`, `active.json`, `set.json`, `*.partial`, `*.bad`) and every
   Windows device name are refused in a signed list. A release build that names the loopback test host fails unless
   `OPENFRAME_TEST_BUILD=1` (set only by the test scripts). A pause takes effect within a quarter second; Remove and
   a download cannot run at the same time.
7. **Test support.** Development tools that need the real models without the network (the AI evaluation harness, the
   ignored real-model tests) install the pinned files the way a finished download leaves them, with a key that lives
   only in that process (`openframe-ai` feature `test-support`, never enabled by the app).

## Alternatives rejected

| Alternative | Reason |
|---|---|
| Keep the bundled AI (ADR-0025) | Makes the installer about 1.1 GB. The owner chose a small installer. |
| Split into an app file plus a separate offline AI file on the same site | Still two files to fetch and verify, and it keeps the offline-first first-run promise only if the user downloads both. Not chosen by the owner. |
| Download missing parts silently as a repair | Silent network use breaks the security standard. The user clicks, and reinstalling is the repair for a damaged install. |
| Download the runtime too | Launching a downloaded executable is the reviewer-sensitive part of the store policy (ADR-0020). The runtime stays in the package. |
