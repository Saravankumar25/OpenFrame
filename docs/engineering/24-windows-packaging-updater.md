# 24 — Windows Packaging, Signing & Updater

> **Status legend:** ✅ implemented · 🚧 in development · 📋 planned · ⛔ blocked on an open decision.
> Source: `apps/desktop/src-tauri/tauri.conf.json`, `.github/workflows/release.yml`, `scripts/release-validate.mjs`.

## 1. Installers ✅ (configured)

| Property | Value |
|---|---|
| Targets | NSIS `.exe` (primary) and WiX `.msi` (`bundle.targets = ["nsis","msi"]`) |
| Product name / identifier | `OpenFrame Studio` / `studio.openframe.desktop` |
| Install mode (NSIS) | `currentUser`: no admin rights, installs under `%LOCALAPPDATA%\Programs` |
| MSI | WiX, `en-US`. MSI is per-machine by Windows Installer convention, for managed deployment. |
| WebView2 | `webviewInstallMode: downloadBootstrapper (silent)`. Windows 11 ships WebView2, and the bootstrapper downloads it only if it is missing (the only network access during install). 📋 Offline installer variant: `embedBootstrapper` or `offlineInstaller` if users without internet must install on machines lacking WebView2. |
| Icons | `apps/desktop/src-tauri/icons/*` |
| Language selector | Off (English UI in v1) |
| Architecture | x64 (`x86_64-pc-windows-msvc`). ARM64 📋 later. |

Build locally with `npm run bundle` (unsigned) → `target/release/bundle/{nsis,msi}/`.

**Microsoft Store channel:** a separate MSIX package (`OpenFrame-Studio-<ver>-x64.msix`) is built from the
same executable with `OPENFRAME_DISTRIBUTION=store` and `scripts/build-msix.mjs`. Microsoft signs it and the Store
updates it. See [27-microsoft-store.md](27-microsoft-store.md).

**User data is never inside the install directory.** Uninstalling removes the program only. Projects
(`Documents\OpenFrame`) and app data (`%LOCALAPPDATA%\OpenFrame`) remain. 📋 The NSIS uninstaller should offer an
explicit, unchecked-by-default "Also remove OpenFrame settings and logs" option. It must never offer to remove
projects.

## 2. Code signing ⛔ (certificate provider not selected, Engineering Index §7)

- **Authenticode** signs the app executable, NSIS installer and MSI with SHA-256 digests and an RFC 3161 timestamp.
- CI flow (`release.yml`): the base64 `.pfx` from secret `WINDOWS_CERTIFICATE` (+ `WINDOWS_CERTIFICATE_PASSWORD`) is
  imported into `Cert:\CurrentUser\My` on the runner. The temp file is deleted immediately. The thumbprint is passed
  via `tauri build --config <release-only json>` (`bundle.windows.certificateThumbprint`, `digestAlgorithm: sha256`,
  `timestampUrl` from variable `WINDOWS_TIMESTAMP_URL`, `tsp: true`). The certificate is removed in an `always()`
  step.
- **Verification (fail closed):** after the build, `Get-AuthenticodeSignature` must report `Valid` for every
  `.exe`/`.msi` and for `openframe-desktop.exe`. Otherwise the job fails.
- If the chosen provider uses a cloud HSM (EV/OV certificates now require hardware-protected keys), replace the
  `.pfx` import with the provider's signing tool via `bundle.windows.signCommand`. The verification step stays.
- Never committed: `*.pfx`, `*.p12`, `*.key` and `.env*` are gitignored, and `release-validate` fails if key files
  are present in the tree.

## 3. Updater ⛔📋 (not integrated yet)

Policy (Engineering Index): **notify the user; download and install only after explicit approval.** No silent
updates, and no update checks when the user has disabled them or is offline.

**Direct channel only.** Store builds (`OPENFRAME_DISTRIBUTION=store` → `cfg(openframe_store)`) must not contain
the updater. Register the plugin under `#[cfg(not(openframe_store))]` and only when
`distribution::Distribution::in_app_updates_allowed()` is true. That function is false for Store builds and for
any process running with MSIX package identity. `release-validate --store` enforces the cfg (doc 27 §5.1).

Plan:

1. Add `tauri-plugin-updater` with `plugins.updater.pubkey` (minisign public key, committed) and HTTPS `endpoints`
   pointing at the OpenFrame release host (hostname ⛔ open decision). `release-validate` already fails until both
   are configured.
2. `bundle.createUpdaterArtifacts: true` (injected in the release config): Tauri signs the installer with
   `TAURI_SIGNING_PRIVATE_KEY` (+ password) from CI secrets and emits `.sig` files, which are uploaded with the release.
3. The update manifest (`latest.json`: version, notes, pub_date, per-platform URL + signature) is generated from the
   reviewed draft release and published only after human approval.
4. In-app flow: a check at most once per day (and on "Check for updates") → a non-modal notice → the user clicks
   Download → the download is verified against the pinned key by the plugin → "Restart to update" → before
   installing, the open project is closed cleanly (checkpoint) so no work is at risk.
5. The update check is a network egress and must be disclosed in settings (PRD L298 egress rule). It sends no
   project data or identifiers beyond what an HTTPS GET of the manifest implies.
6. Key rotation: document a procedure before 1.0. Losing the updater key strands existing installs.

## 4. Release artifacts (per tag)

`OpenFrame Studio_<ver>_x64-setup.exe`, `OpenFrame Studio_<ver>_x64_en-US.msi`, updater `.sig` files,
`sbom-rust-*.cdx.json`, `sbom-npm.cdx.json`, `THIRD_PARTY_LICENSES.md` and `SHA256SUMS.txt`. Everything is uploaded
as a workflow artifact and attached to a **draft** GitHub release.

## 5. Blockers before the first public installer

(Direct channel. The Microsoft Store channel needs neither the certificate nor the updater. Its blockers are
listed in doc 27 §10.)

1. Open-source license selected (LICENSE-PENDING.md). The installer license page must show it.
2. Code-signing certificate provider chosen; secrets and `WINDOWS_TIMESTAMP_URL` configured.
3. Updater plugin, public key and release host configured.
4. Full license texts bundled with the installer (an inventory alone is not sufficient). 📋 Generate a
   `licenses/` folder via `cargo about` + an npm license collector at release time and include it as a bundle resource.
