# 25: CI/CD & Build Reproducibility

> **Current decisions (2026-10-15).** CI tests the AI download against a test manifest and a local host, not the model files. New gates: secret scan, link check, installer string scan ([ADR-0030](../adr/ADR-0030-verification-ladder-and-release-flow.md)), and the AI evaluation harness as a model gate ([ADR-0028](../adr/ADR-0028-ai-quality-program-and-harness.md)). The Tauri Mac compile job is retired in Mac phase M1 ([ADR-0027](../adr/ADR-0027-rust-core-two-ui-shells.md)).

> **Status legend:** ✅ implemented · 📋 planned.
> Source: `.github/workflows/{ci.yml,mac-core.yml,release.yml}`, `deny.toml`, `scripts/*.mjs`, `Cargo.lock`, `package-lock.json`.

## 1. Pipelines

### `ci.yml` ✅ (push to `master`/`main`, every pull request, manual)

| Job | Runner | Steps | Gating |
|---|---|---|---|
| `rust` | `windows-latest` | stable toolchain (rustfmt, clippy) → rust-cache → `cargo fmt --all -- --check` → placeholder `apps/desktop/dist` (needed by `tauri::generate_context!`) → `cargo clippy --workspace --all-targets --locked -- -D warnings` → `cargo test --workspace --locked` → **no drift** in `apps/desktop/src/ipc/generated` (ts-rs regenerates the types during tests) and in `docs/engineering/ai-tool-coverage.md` (regenerated from the Registry) | Blocking |
| `web` | `windows-latest` | Node 20 + npm cache → `npm ci` → `npm run typecheck` (tsc) → ESLint `--max-warnings 0` → Vitest → `npm --workspace apps/desktop run build:web` | Blocking |
| `governance` | `windows-latest` | Node 20 → `scripts/architecture-lint.mjs --list` (webview imports, single IPC, no webview network, capabilities, CSP, command allow-list, SQLite write sites, AI apply boundary, listeners, `unsafe`, process spawn) → `scripts/check-migrations.mjs` (linear numbering, embedded list, freeze checksums) → `scripts/check-feature-status.mjs` (every in-scope row has a source, a build area, a UI route, tests, a status and an ADR-0030 proof level its evidence supports) → reachability, plan evidence, writing standard → `scripts/check-links.mjs` (document links, and every web address in the shipped app source on the allow-list `scripts/app-addresses.json`) → `scripts/check-secrets.mjs` (secret patterns and secret files in the working tree and every commit) → `node --test scripts/tests/governance.test.mjs` (each check catches a planted violation) | **Blocking** (build plan: feature status check in CI, architecture lint, migration policy) |
| `e2e` | `windows-latest` | rust toolchain + cache → Node + `npm ci` → `npm run test:e2e` (verifies and stages the bundled Offline AI from `OPENFRAME_AI_ASSETS_DIR`, builds the real debug app with it, installs msedgedriver matching the runner's WebView2 and tauri-driver, runs `tests/e2e/specs`: lifecycle journey, relaunch, the AI specs with the real model, reachability generated from the feature status file) → failure screenshots uploaded. Fails closed until the runner has the pinned AI files for today's bundled build; after the download flow is rebuilt, CI tests the download against a test manifest and host (ADR-0026) | **Blocking**; the same job runs on the release tag in `release.yml` and `build` / `store` need it (build plan: end-to-end tests in CI) |
| `supply-chain` | `windows-latest` | `cargo install cargo-deny` → `cargo deny --locked check` (advisories, licenses, bans, sources) → `cargo install cargo-audit` → `cargo audit` → `npm ci` → `npm audit --omit=dev --audit-level=moderate` → `license-inventory --check` + `THIRD_PARTY_LICENSES.md` must be current | **Blocking** since 2026-10-05 (build plan: supply-chain checks); also a hard gate in releases. An advisory may be ignored only in `deny.toml` with id, reason and tracking reference. |

Locally, `npm run verify` runs format, lint, typecheck, `npm run check:governance` and tests. On a 16 GB machine set
`CARGO_BUILD_JOBS=4` (or 8) and do not run two cargo builds at once; the full test build otherwise exhausts memory.

Concurrency cancels superseded runs per ref. Permissions are `contents: read`.

### `mac-core.yml` (written 2026-10-17, has not run yet)

Triggered by a pull request or a push to `main` that changes `crates/**`, `apps/macos/**`, `Cargo.lock`, `Cargo.toml` or the
workflow itself, and by hand. macOS runner minutes cost ten times Windows minutes, hence the path filter.

| Job | Runner | Steps | Gating |
|---|---|---|---|
| `mac-core` | `macos-latest` (Apple silicon), 60 minute timeout | Rust 1.98.1 with `aarch64-apple-darwin`, rust-cache (same pinned actions as `ci.yml`) → Swift and Xcode versions → `cargo test --workspace --exclude openframe-desktop --locked` → `bash apps/macos/scripts/build-core.sh` → `bash apps/macos/scripts/test.sh` | Not a required check yet: make it required once it has passed |

It proves that the shared core builds and passes its tests on Apple silicon and that the Swift package of the native Mac app
builds and tests against it. It does not sign, package or start the app. The earlier `mac.yml` ("Mac build check", Tauri shell
and StoreKit bridge) is superseded and retired: see the archive note and
[chapter-1b-mac-phases.md](../roadmap/chapter-1b-mac-phases.md).

### `release.yml` ✅ (tag `v*.*.*`, or manual with an existing tag)

1. **validate** (fail closed): `scripts/release-validate.mjs --tag` checks versions and tag, license selected,
   signing secrets present, updater configured, license inventory clean and current, and the bundled Offline AI build
   inputs matching `packaging/ai/ai-assets.lock.json` for today's bundled build (ADR-0026 will replace this with the manifest and host checks). Then `cargo deny check` (blocking).
2. **build** (needs validate): full verification (fmt, clippy, tests, tsc, eslint, vitest) → Authenticode import →
   release-only Tauri config → `prepare-ai-assets.mjs` → `tauri build --bundles nsis,msi --config src-tauri/tauri.bundled-ai.conf.json` with
   updater artifacts → **signature verification**
   → CycloneDX SBOMs (`cargo cyclonedx`, `@cyclonedx/cyclonedx-npm --omit dev`) → license inventory → `SHA256SUMS.txt`
   → workflow artifact → **draft** GitHub release (`contents: write` for this job only) → certificate removed
   (`always()`).

3. **validate-store → store** (Microsoft Store channel, doc 27): `release-validate.mjs --store` (Partner Center
   identity, `X.Y.Z.0` version, MSIX assets, manifest capabilities, privacy policy URL, no updater in Store builds)
   + `cargo deny` → full verification → `prepare-ai-assets.mjs` → `tauri build --no-bundle` with the bundled-AI overlay
   and `OPENFRAME_DISTRIBUTION=store` → `build-msix.mjs --store` (verifies and copies the bundled AI into the layout, makepri
   + makeappx, unsigned) → artifact `openframe-<tag>-msstore-x64`. It uses no
   secrets. Variables: `MSIX_IDENTITY_NAME`, `MSIX_PUBLISHER`, `MSIX_PUBLISHER_DISPLAY_NAME`, `STORE_PRIVACY_POLICY_URL`
   (or commit them in `packaging/msix/store.config.json`). `RELEASE_CHANNELS` (`direct,store` by default) or the
   `channels` dispatch input selects which channels a run builds.

Required secrets (direct channel): `WINDOWS_CERTIFICATE`, `WINDOWS_CERTIFICATE_PASSWORD`, `TAURI_SIGNING_PRIVATE_KEY`,
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Variables: `WINDOWS_TIMESTAMP_URL`, `OPENFRAME_AI_ASSETS_DIR` (folder on the runner
holding the pinned Offline AI files for today's bundled build; also used by `e2e` and `store`). `MODEL_DISTRIBUTION_BASE_URL`
and `OPENFRAME_MANIFEST_PUBLIC_KEY` are gone with the download flow.
**None are configured today, so a tag push fails at `validate` by design.**

## 2. Local equivalents

| CI step | Local command |
|---|---|
| Everything blocking | `npm run verify` |
| Supply chain | `cargo install cargo-deny --locked; cargo deny check; npm audit --omit=dev; npm run licenses` |
| Release gate | `node scripts/release-validate.mjs --report-only` (Store: add `--store`) |
| Bundled AI inputs | `npm run ai:assets` (verify `.ai-assets` against the lock and stage it; `-- --check` verifies only) |
| Store package | `$env:OPENFRAME_DISTRIBUTION='store'; npm run build -- --no-bundle; node scripts/build-msix.mjs --store` (local test: `--dev`) |
| Prerequisites | `npm run setup` / `node scripts/setup.mjs --check-only` |

## 3. Reproducibility

- **Locked dependencies:** `Cargo.lock` + `--locked` on every cargo invocation in CI, and `package-lock.json` +
  `npm ci`. Lockfile changes are reviewed like code (CONTRIBUTING §5).
- **Toolchains:** Rust `stable` via `dtolnay/rust-toolchain` (MSRV 1.85 declared in `Cargo.toml`) and Node 20.
  📋 Pin an exact Rust version in `rust-toolchain.toml` before 1.0 so release builds are reproducible to the
  compiler version. Until then the release notes record `rustc -V`.
- **Deterministic outputs:** the license inventory has no timestamps (it only changes when dependencies change), and
  PDF exports are deterministic by design (ADR-0008). Generated bindings are committed and verified.
- **Release profile:** `lto = "thin"`, `codegen-units = 1`, `opt-level = 3`, `strip = "debuginfo"`. 📋 Keep PDB
  symbols as private build artifacts for crash analysis (not shipped).
- **Provenance:** 📋 add `actions/attest-build-provenance` for installers once the release host is chosen.
  SHA256SUMS and SBOMs are produced today.
- **Third-party actions:** only well-known actions (`actions/*`, `dtolnay/rust-toolchain`, `Swatinem/rust-cache`).
  📋 Pin them by commit SHA before the first public release.

## 4. Branch protection (to configure on the hosting repo) 📋

- `master` requires the `rust` and `web` checks and one approving review, and allows no force-push.
- Tags `v*` can be created by maintainers only.
- The `release` environment requires reviewer approval to expose signing secrets.
