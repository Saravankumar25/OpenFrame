# 25 — CI/CD & Build Reproducibility

> **Status legend:** ✅ implemented · 📋 planned.
> Source: `.github/workflows/{ci.yml,release.yml}`, `deny.toml`, `scripts/*.mjs`, `Cargo.lock`, `package-lock.json`.

## 1. Pipelines

### `ci.yml` ✅ (push to `master`/`main`, every pull request, manual)

| Job | Runner | Steps | Gating |
|---|---|---|---|
| `rust` | `windows-latest` | stable toolchain (rustfmt, clippy) → rust-cache → `cargo fmt --all -- --check` → placeholder `apps/desktop/dist` (needed by `tauri::generate_context!`) → `cargo clippy --workspace --all-targets --locked -- -D warnings` → `cargo test --workspace --locked` → **no drift** in `apps/desktop/src/ipc/generated` (ts-rs regenerates the types during tests) | Blocking |
| `web` | `windows-latest` | Node 20 + npm cache → `npm ci` → `npm run typecheck` (tsc) → ESLint `--max-warnings 0` → Vitest → `npm --workspace apps/desktop run build:web` | Blocking |
| `supply-chain` | `windows-latest` | `cargo install cargo-deny` → `cargo deny --locked check` (advisories, licenses, bans, sources) → `npm ci` → `npm audit --omit=dev --audit-level=moderate` → `license-inventory --check` + `THIRD_PARTY_LICENSES.md` must be current | **Non-blocking** (`continue-on-error`), visible on every PR; blocking in releases |

Concurrency cancels superseded runs per ref. Permissions are `contents: read`.

### `release.yml` ✅ (tag `v*.*.*`, or manual with an existing tag)

1. **validate** (fail closed): `scripts/release-validate.mjs --tag` checks versions and tag, license selected,
   signing secrets present, updater configured, license inventory clean and current, and the model distribution URL
   (only once the AI module exists). Then `cargo deny check` (blocking).
2. **build** (needs validate): full verification (fmt, clippy, tests, tsc, eslint, vitest) → Authenticode import →
   release-only Tauri config → `tauri build --bundles nsis,msi` with updater artifacts → **signature verification**
   → CycloneDX SBOMs (`cargo cyclonedx`, `@cyclonedx/cyclonedx-npm --omit dev`) → license inventory → `SHA256SUMS.txt`
   → workflow artifact → **draft** GitHub release (`contents: write` for this job only) → certificate removed
   (`always()`).

3. **validate-store → store** (Microsoft Store channel, doc 27): `release-validate.mjs --store` (Partner Center
   identity, `X.Y.Z.0` version, MSIX assets, manifest capabilities, privacy policy URL, no updater in Store builds)
   + `cargo deny` → full verification → `tauri build --no-bundle` with `OPENFRAME_DISTRIBUTION=store` →
   `build-msix.mjs --store` (makepri + makeappx, unsigned) → artifact `openframe-<tag>-msstore-x64`. It uses no
   secrets. Variables: `MSIX_IDENTITY_NAME`, `MSIX_PUBLISHER`, `MSIX_PUBLISHER_DISPLAY_NAME`, `STORE_PRIVACY_POLICY_URL`
   (or commit them in `packaging/msix/store.config.json`). `RELEASE_CHANNELS` (`direct,store` by default) or the
   `channels` dispatch input selects which channels a run builds.

Required secrets (direct channel): `WINDOWS_CERTIFICATE`, `WINDOWS_CERTIFICATE_PASSWORD`, `TAURI_SIGNING_PRIVATE_KEY`,
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Variables: `WINDOWS_TIMESTAMP_URL`, `MODEL_DISTRIBUTION_BASE_URL` (AI builds).
**None are configured today, so a tag push fails at `validate` by design.**

## 2. Local equivalents

| CI step | Local command |
|---|---|
| Everything blocking | `npm run verify` |
| Supply chain | `cargo install cargo-deny --locked; cargo deny check; npm audit --omit=dev; npm run licenses` |
| Release gate | `node scripts/release-validate.mjs --report-only` (Store: add `--store`) |
| Store package | `$env:OPENFRAME_DISTRIBUTION='store'; npm --workspace apps/desktop run tauri -- build --no-bundle; node scripts/build-msix.mjs --store` (local test: `--dev`) |
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
