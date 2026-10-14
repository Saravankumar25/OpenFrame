# ADR-0025: Offline AI bundled in every package

- **Status:** Accepted 2026-10-08 (owner, in chat). Permanent and cross-platform.
- **Date:** 2026-10-08
- **Supersedes:** [ADR-0020](ADR-0020-ai-runtime-packaging-per-channel.md) decision 1 where it says only model weights
  download after a click, and decisions 3 and 5; [ADR-0006](ADR-0006-local-ai-sidecar.md) decision 2 (delivery by
  **Download Offline AI** from a model host) and its rejected alternative "Bundling the model in the installer";
  [ADR-0013](ADR-0013-single-local-model-hybrid-rag-agent.md) decision 2 ("One download").
- **Keeps:** ADR-0020 decision 1 for the runtime (the `llama.cpp` runtime ships inside every package), decision 4
  (Report AI output); ADR-0006 decisions 1 and 4 to 7; ADR-0013 except decision 2.
- **Related:** [ADR-0017](ADR-0017-cross-platform-windows-and-macos.md) (supported platforms, Linux not a target);
  [ADR-0024](ADR-0024-internal-team-builds.md) (four release files); `crates/openframe-ai/`;
  `docs/implementation/masterplan.md` area 8; `docs/implementation/owner-decisions.md`

## Context

Until now the AI engine shipped inside each package (ADR-0020) but the Gemma 3 1B weights, the BGE small embedding
weights and the tokenizer were downloaded after the user clicked **Download Offline AI**. That needed a production model
host, a signed download list with an offline signing key, download resilience work, and a privacy policy entry for the
host. It also meant a fresh install could not use AI without a network connection, which contradicts the offline promise.

On 2026-10-08 the owner decided that AI must work right after install with no download and no setup, on every package
format, now and in the future.

## Decision

1. **Every supported package contains the complete offline AI.** That means the AI runtime, the tested Gemma 3 1B
   weights, the embedding weights, the tokenizer and every runtime asset the engine needs. This covers the Windows
   `.exe` and `.msix`, the macOS `.dmg` and `.app` (inside the `.pkg`), Linux AppImage, DEB and RPM if Linux is ever
   supported, and any future format.
2. **AI works right after install or first launch.** No download, no model host and no manual setup. AI stays optional
   for core work (ADR-0006 decision 7 is unchanged).
3. **Paths follow each platform.** Large files that never change (weights, tokenizer, runtime) live in the installed
   app resources. Writable AI data (indexes, logs, caches, health records) lives in the user's app data folder. One
   layout for all platforms is not allowed. The bundled AI must load under MSIX packaging and under the macOS App
   Sandbox, which both treat the installed resources as read-only.
4. **Integrity and compatibility are checked.** Each bundled model file has a pinned SHA-256 hash and size recorded at
   build time. The build fails when a hash does not match. The app checks the files before it first starts the engine
   and refuses to start AI, with a plain message and no change to the project, when a file is missing or altered. The
   bundled runtime, weights and tokenizer are tested together as one set.
5. **Never fetch at runtime.** The app never downloads weights, never silently downloads a missing part, and never
   offers a download as a repair. A damaged install is repaired by reinstalling the package.
6. **AI is part of install and upgrade.** Installing the app installs AI. Upgrading the app replaces the bundled AI as
   one unit. Uninstalling removes the bundled files with the app and follows each format's rule for the user's app data.
7. **Licensing.** Redistribution rights for the Gemma weights, the embedding model, the tokenizer and the runtime are
   verified before any package leaves the team. The Gemma terms review is an owner task and is now a redistribution
   check. Notices ship in the app and in the third-party notices file.
8. **Validation per format.** Install, offline inference, upgrade and uninstall are validated separately for each
   package format. No platform or format is claimed as supported until its package is built and tested this way.
9. **Linux.** Linux is not a target today (ADR-0017). If it becomes one, this rule applies to it with no new decision.

## Consequences

- **Installer size grows** by the weights, roughly 1.1 GB on top of the app (ADR-0013 measured the download at
  1,125,156,323 bytes including the runtime). The real size of each package must be measured and recorded when it is
  first built.
- **Clean as you go.** The change that bundles the weights must also remove, in the same change: the **Download
  Offline AI** flow and its screens, the download code (partial files, resume, disk-space preflight for downloads,
  quarantine of bad downloads), the model host setting (`MODEL_DISTRIBUTION_BASE_URL`), the manifest signing key items
  (`OPENFRAME_MANIFEST_PUBLIC_KEY`, the offline Ed25519 key, the release check that requires it), and their tests. The
  repository must not carry the download path and the bundled path side by side.
- The owner tasks "choose the model host" and "generate the production manifest key" are no longer needed.
- Store listings and certification notes no longer describe an optional AI download. The privacy policy no longer
  names a model download host. `internetClient` on the Microsoft Store package is justified by links and update checks
  only, or removed if nothing else needs it.
- Model updates ship as app updates, so the model version is part of the app version, like the runtime (ADR-0020).
- On the Mac the weights sit inside the signed app bundle, which App Sandbox and notarization accept as read-only
  resources.
- Documents that still describe the download (`docs/engineering/23-release-migration.md`,
  `docs/engineering/27-microsoft-store.md`, `docs/engineering/22-qa-test-strategy.md`, `docs/engineering/25-ci-cd.md`,
  `docs/store/privacy-policy.md`, the AI and runtime specifications) describe the code as it is today. They are updated
  in the change that removes the download. Where they disagree with this decision, this decision wins.

### Validation list per package format

Each format is validated on its own, on a clean machine, with the network turned off for the inference step:

| Format | Install | Offline inference | Upgrade | Uninstall | When |
|---|---|---|---|---|---|
| Windows `.exe` (internal) | yes | yes | yes | yes | now |
| macOS `.dmg` (internal) | yes | yes | yes | yes | later, on an Apple silicon Mac |
| Windows `.msix` (Microsoft Store) | yes | yes | yes | yes | later, store publishing step |
| macOS `.app` in `.pkg` (Mac App Store) | yes | yes | yes | yes | later, store publishing step |
| Linux AppImage, DEB, RPM | yes | yes | yes | yes | only if Linux becomes a target |

Each check records: the package size, that the pinned hashes match, that the first AI answer comes with no network,
that an upgrade keeps settings and projects and rebuilds the AI index when needed, and that uninstall removes the bundled
files and leaves user projects in place.

## Alternatives rejected

| Alternative | Reason |
|---|---|
| Keep the download after a click (ADR-0020) | AI would not work offline after install, and it needs a model host, a signing key and download code to run and secure |
| Bundle in some formats and download in others | Two delivery paths to build, test and secure; the owner chose one rule for every package |
| Download missing parts silently as a repair | Silent network use breaks the security standard; reinstalling is the repair |
