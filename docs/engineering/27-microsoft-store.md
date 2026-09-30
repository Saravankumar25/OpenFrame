# 27 — Microsoft Store (MSIX) distribution

> **Status legend:** ✅ implemented · 🚧 in development · 📋 planned · ⛔ blocked on the product owner.
> Sources: `packaging/msix/*`, `scripts/build-msix.mjs`, `scripts/release-validate.mjs --store`,
> `.github/workflows/release.yml` (jobs `validate-store`, `store`), `apps/desktop/src-tauri/build.rs`,
> `apps/desktop/src-tauri/src/distribution.rs`, `docs/store/{privacy-policy,listing}.md`.
> Microsoft requirements were checked on 2026-09-30 against Microsoft Learn (Store Policies v7.20,
> MSIX submission, screenshots and images, "How packaged desktop apps run on Windows", WebView2
> distribution) and the Windows developer blog (developer registration fees).

## 1. Channels

| | Direct download (doc 24) | Microsoft Store (this doc) |
|---|---|---|
| Artifact | NSIS `.exe` + WiX `.msi` | `OpenFrame-Studio-<ver>-x64.msix` |
| Code signing | Our Authenticode certificate (⛔ provider not chosen) | **Microsoft signs the package** during Store ingestion. No certificate or signing secret is needed. |
| Updates | In-app updater (📋 not integrated yet) | Microsoft Store only. The in-app updater is compiled out. |
| Install | Per user (NSIS) / per machine (MSI) | Per user, no admin rights, clean uninstall |
| Build flag | `OPENFRAME_DISTRIBUTION` unset or `direct` | `OPENFRAME_DISTRIBUTION=store` |

The Store can also list a traditional MSI/EXE by URL (Store policy 10.2.9). That path still needs our
own trusted code-signing certificate, a silent standalone installer and a versioned download URL, so
it doesn't remove the certificate blocker. It is not used.

## 2. Files

| Path | Purpose |
|---|---|
| `packaging/msix/AppxManifest.template.xml` | Manifest template: `Windows.Desktop` target, `MinVersion 10.0.17763.0` (Windows 10 1809), `MaxVersionTested 10.0.26100.0`, `EntryPoint="Windows.FullTrustApplication"`, capabilities `runFullTrust` + `internetClient`, visual elements, optional file-type association. |
| `packaging/msix/store.config.json` | Partner Center identity (placeholders `REPLACE_…`), display name, description, privacy policy URL, OS versions, file-type association switch (off). Environment variables override it (`MSIX_IDENTITY_NAME`, `MSIX_PUBLISHER`, `MSIX_PUBLISHER_DISPLAY_NAME`, `MSIX_DISPLAY_NAME`, `STORE_PRIVACY_POLICY_URL`). |
| `packaging/msix/Assets/*.png` | 77 visual assets generated from `apps/desktop/src-tauri/icons/source.png`: Square44x44 (app list), Square71x71, Square150x150, Square310x310, Wide310x150, StoreLogo and SplashScreen at scales 100/125/150/200/400, plus Square44x44 target sizes 16–256 (plated, `altform-unplated`, `altform-lightunplated`) for the taskbar and Start. Committed so they can be hand-tuned. Regenerate with `node scripts/build-msix.mjs --generate-assets`. |
| `packaging/msix/generate-assets.ps1` | System.Drawing resizer used by `--generate-assets`. Tiles center the icon at 66% (splash 50%) on transparency. |
| `scripts/build-msix.mjs` | Stage → manifest → `makepri` (`resources.pri` maps the unqualified names in the manifest to the scale/target-size files) → `makeappx pack` → optional dev signing, bundle, loose registration. |
| `docs/store/privacy-policy.md` | Privacy policy draft (host it, then configure its URL). |
| `docs/store/listing.md` | Store listing text, features, keywords, screenshot plan. |

## 3. Building

Prerequisites: Windows 10/11 SDK with "App packaging tools" (`makeappx.exe`, `makepri.exe`,
`signtool.exe` under `C:\Program Files (x86)\Windows Kits\10\bin\<ver>\x64`; the newest version is
picked automatically, or set `WINDOWS_SDK_BIN_DIR`). GitHub `windows-latest` runners include it.

```powershell
# Store upload package (unsigned; requires the Partner Center identity in store.config.json)
$env:OPENFRAME_DISTRIBUTION = 'store'
npm --workspace apps/desktop run tauri -- build --no-bundle      # target\release\openframe-desktop.exe
node scripts/build-msix.mjs --store                             # target\msix\OpenFrame-Studio-0.1.0-x64.msix
node scripts/release-validate.mjs --store --report-only

# Local test package: test identity "OpenFrameStudio.LocalTest", signed with a throwaway certificate
node scripts/build-msix.mjs --dev                               # target\msix\OpenFrame-Studio-0.1.0-x64-localtest.msix
```

- The version comes from `tauri.conf.json`. `X.Y.Z` becomes `X.Y.Z.0`, because the Store reserves
  the fourth part. Pre-release suffixes are rejected. Every Store submission needs a higher version
  than the last one.
- `--store` refuses an executable that wasn't built with `OPENFRAME_DISTRIBUTION=store` (it checks
  the embedded `openframe-distribution-channel:store` marker). This keeps a build with updater code
  out of the Store.
- `--bundle` also writes a `.msixbundle`. That becomes useful once an ARM64 package exists. For
  x64 only, upload the `.msix` directly. `.msixupload` is only needed to ship symbol files, and
  OpenFrame doesn't upload symbols.
- **Local test signing:** `--dev` creates a self-signed code-signing certificate in memory, with a
  subject equal to the test publisher, and writes it only to `target/msix/dev-cert/` (gitignored,
  never committed). The script never imports it into a certificate store. To install the test package,
  trust it yourself on a development machine. This changes machine trust, so it needs an elevated
  prompt:
  `Import-Certificate -FilePath target\msix\dev-cert\OpenFrame-LocalTest.cer -CertStoreLocation Cert:\LocalMachine\TrustedPeople`,
  then `Add-AppxPackage target\msix\OpenFrame-Studio-<ver>-x64-localtest.msix`. Alternatively, with
  Windows **Developer Mode** on, `node scripts/build-msix.mjs --dev --register` registers the staged
  folder without any certificate. Rebuilding replaces the staged folder, so register again afterwards.
  Remove the test app with `Get-AppxPackage OpenFrameStudio.LocalTest | Remove-AppxPackage`. Remove
  the certificate from Trusted People when you are done.
- CI: `release.yml` runs `validate-store` (`release-validate.mjs --store` + `cargo deny`), then
  `store` (full verification → Store-channel `tauri build --no-bundle` → `build-msix.mjs --store` →
  artifact `openframe-<tag>-msstore-x64` with the MSIX, `msix-build.json`, `THIRD_PARTY_LICENSES.md`
  and `SHA256SUMS.txt`). No secrets are used. The repository variable `RELEASE_CHANNELS`
  (`direct,store` by default) or the `channels` dispatch input selects the channels. The MSIX is
  not attached to the GitHub release. It is uploaded to Partner Center by a person.

## 4. Manifest decisions

| Item | Decision |
|---|---|
| Identity `Name`, `Publisher`, `PublisherDisplayName` | Placeholders filled from Partner Center. `Publisher` is the `CN=<GUID>` shown under Product identity. It must match exactly, or upload fails. |
| `DisplayName` | Must equal the name reserved in Partner Center (`OpenFrame Studio`). |
| Target | `Windows.Desktop`, `MinVersion 10.0.17763.0`, `MaxVersionTested 10.0.26100.0`, x64. No `uap10` attributes, because they need build 19041. |
| Entry point | `Windows.FullTrustApplication`: a Desktop Bridge (packaged classic) app running `openframe-desktop.exe` at medium integrity. |
| Capabilities | `runFullTrust` (restricted, required for every packaged Win32 app; justification in §7). `internetClient` is declarative only for full-trust apps and describes the optional model/runtime downloads and update checks. **No other capability.** `release-validate --store` fails if the set changes. |
| File-type association | Off (`fileTypeAssociations.enabled=false`). Projects are folders, so they can't be associated. Exchange packages (`.ofstory`, `.ofschedule`, `.ofshots`) can be associated once the app handles file activation. The switch generates `uap:FileTypeAssociation`. |
| Languages | `en-US` (the v1 UI is English). |
| WebView2 | Not declarable as a Store package dependency. See §5.5. |

## 5. Runtime behaviour in Store builds (MSIX compatibility)

### 5.1 Channel flag and updater ✅
`build.rs` reads `OPENFRAME_DISTRIBUTION` (`direct` default, `store`, anything else fails the build),
exports it as `env!("OPENFRAME_DISTRIBUTION")` and sets `cfg(openframe_store)` for Store builds.
`distribution.rs` combines this with runtime detection of MSIX package identity
(`GetCurrentPackageFamilyName`). `Distribution::in_app_updates_allowed()` is true only for a direct
build running unpackaged. A sideloaded MSIX of a direct build also never self-updates, because its
install folder is read-only and an NSIS update would install a second copy. No updater plugin exists
today. When one is added (doc 24 §3), it must be compiled under `#[cfg(not(openframe_store))]` and
registered only when `in_app_updates_allowed()` is true. `release-validate --store` fails if
`lib.rs` references `tauri_plugin_updater` without that cfg. The startup log records the channel,
packaging state and updater decision.

### 5.2 App data folder ✅ (decision)
MSIX (Windows 10 1903 and later) redirects *new* files that a packaged full-trust app creates under
`%LOCALAPPDATA%` to a private per-package location. It merges them back only for processes inside
the package. Explorer and other processes don't see them at the original path, and the files are
deleted on uninstall. With the default path, "Open logs folder" would show an empty or missing
folder.

**Decision:** when the process has package identity, `AppConfig.app_data_dir` becomes
`%LOCALAPPDATA%\Packages\<PackageFamilyName>\LocalState\OpenFrame`, the Win32 path of
`ApplicationData.Current.LocalFolder`. It is a real, non-virtualized path, so the app, Explorer and
child processes see the same files. It needs no extra capability (the alternative, disabling
virtualization, needs the restricted `unvirtualizedResources` capability), and Windows removes it on
uninstall (Store policy 10.2.7 clean uninstall). Unpackaged builds keep `%LOCALAPPDATA%\OpenFrame`.
Implemented in the Tauri adapter (`lib.rs` overrides the field); the foundation crates are unchanged.

Consequence: a user who moves from the direct build to the Store build starts with empty settings
and an empty recent-projects list, and must download AI components again. Their projects and Global
Idea Vault are unaffected (§5.3). They reopen projects with *Open project*. 📋 An optional one-time
import of `%LOCALAPPDATA%\OpenFrame\` settings and recents could be added later.

Tauri's WebView2 profile (`%LOCALAPPDATA%\studio.openframe.desktop\EBWebView`) stays on the
default path. It is virtualized into the package's private storage, which is harmless because only
the app's own WebView2 processes use it.

### 5.3 Projects and Global Idea Vault in Documents ✅
MSIX does not virtualize Documents (`FOLDERID_Documents`). Projects (`Documents\OpenFrame\Projects`)
and the Global Idea Vault (`Documents\OpenFrame\Global Idea Vault`) are real files that other apps
and backups can see. They survive uninstall, as the privacy policy promises. File dialogs,
`opener` open/reveal and the asset protocol scope (project folder + vault) work unchanged.

### 5.4 Local AI runtime (llama.cpp) 📋 (AI module not merged yet)
Downloaded runtime executables and GGUF models go under `app_data_dir`, which is the package
`LocalState` folder in Store builds. A full-trust (medium IL) packaged app may start executables
from any folder the user can read. The child process runs with the package's identity, and because
the path is real (§5.2), no virtualization mismatch can occur. Requirements for the AI module:
- Store policy 10.2.2 forbids using downloaded code to change or extend the app's *described*
  functionality. The AI assistant and its on-demand runtime/model download must therefore be
  described in the listing (`docs/store/listing.md` **[AI]** lines). The download must be
  user-initiated, come from the configured HTTPS host and be verified (SHA-256 pinned in the app)
  before it runs.
- Store policy 11.16 (live generative AI): disclose generative AI in the listing metadata and in
  Partner Center, and give users a way to report inappropriate output to the developer (the listing
  draft has a placeholder).
- Never run the runtime from the package install folder (read-only) or from a temp folder.

### 5.5 WebView2 ✅
The Evergreen WebView2 Runtime is part of Windows 11 and is present on nearly all Windows 10
devices. MSIX can't run the WebView2 bootstrapper that the NSIS/MSI installers use, and the Store
has no WebView2 package dependency. Before creating any window, `distribution::ensure_webview2()`
checks `tauri::webview_version()`. If the runtime is missing, the app shows a native message box
that explains the problem and offers to open Microsoft's WebView2 download page, instead of
crashing silently (release builds have no console). The listing states the requirement.

### 5.6 Single instance ✅ (expected; confirm on an installed package, §8)
`tauri-plugin-single-instance` uses a named mutex and window messaging within the user's session.
Neither is affected by MSIX. A second launch from Start focuses the running window.

### 5.7 No admin rights ✅
MSIX installs per user and the app never asks for elevation. `makeappx` packages don't use
installers, services or drivers. Product declaration "depends on non-Microsoft drivers or NT
services" = No.

## 6. Partner Center submission, step by step

1. **Developer account** ⛔ owner. Register at <https://storedeveloper.microsoft.com> (Partner
   Center). Registration is free for individual developers (since September 2025) and, per
   Microsoft's May 2026 announcement, for company accounts as well. Individuals verify their
   identity with a government ID and a selfie. Company accounts go through business verification.
   Decide which account type publishes. The publisher display name appears on the Store page.
2. **Reserve the name.** Apps and games → New product → *MSIX or PWA app* → reserve
   **OpenFrame Studio**.
3. **Copy the identity.** Product management → *Product identity*: copy `Package/Identity/Name`,
   `Package/Identity/Publisher` (`CN=…`) and `Package/Properties/PublisherDisplayName` into
   `packaging/msix/store.config.json` and commit them (they are public), or set the repository
   variables `MSIX_IDENTITY_NAME`, `MSIX_PUBLISHER` and `MSIX_PUBLISHER_DISPLAY_NAME`.
4. **Privacy policy** ⛔ owner. Finish `docs/store/privacy-policy.md` (fill in the placeholders and
   confirm it matches the build) and host it at a stable public HTTPS URL. Set `privacyPolicyUrl` or
   `STORE_PRIVACY_POLICY_URL`. Store policy 10.5.1 requires a privacy policy for every Win32
   (Desktop Bridge) product, even one that collects no personal information.
5. **Build.** Push tag `vX.Y.Z`. The `store` job produces the artifact
   `openframe-<tag>-msstore-x64`. Run WACK on it (§8).
6. **Start a submission.**
   - *Pricing and availability:* markets (default: all), audience public, free, release schedule.
   - *Properties:* category **Photo & video** (alternative: Productivity), privacy policy URL,
     website, support contact, system requirements (x64; memory/disk for the AI assistant),
     product declarations.
   - *Age ratings:* answer the IARC questionnaire honestly. OpenFrame has no violence, no sharing of
     user content with other users, no chat, no purchases and no location use. Content is written by
     the user and stays on the device. If the AI assistant ships, answer the generative-AI questions
     accordingly. The expected result is a general-audience rating, but the questionnaire decides.
   - *Packages:* upload `OpenFrame-Studio-<ver>-x64.msix`. Device families: Windows 10/11 Desktop.
   - *Store listings (en-US):* text from `docs/store/listing.md`, 4 to 8 screenshots (§9), and the
     recommended 300 × 300 app tile icon.
   - *Submission options:* restricted capability justification (§7). Notes for certification:
     "No account or sign-in is required. Create a project from the home screen (New project).
     Optional AI components are downloaded only when the user starts the download in the app's AI
     settings; the app works fully without them."
7. **Submit for certification.** Certification usually takes a few business days. Fix any report
   items and resubmit with a higher version.
8. **Updates:** each new version is a new submission with a higher `X.Y.Z` (package `X.Y.Z.0`).
   The Store delivers it to users. 📋 Optional automation later: the Microsoft Store Developer CLI
   (`msstore`) or the Store submission API with an Entra ID app registration (the owner creates it).

## 7. Restricted capability justification (paste into Submission options)

> **runFullTrust:** OpenFrame Studio is a desktop application (Rust + Microsoft Edge WebView2,
> built with the Tauri framework) packaged with MSIX through the Desktop Bridge. `runFullTrust` is
> required to run its Win32 executable (`EntryPoint="Windows.FullTrustApplication"`). The app reads
> and writes the user's project folders in Documents that the user chooses, opens files and folders
> in Explorer at the user's request, and optionally runs a local AI inference engine (llama.cpp)
> that the user downloads from inside the app. It installs no drivers or services, requires no
> administrator rights, collects no telemetry and doesn't require an account.

## 8. Validation: Windows App Certification Kit (WACK)

WACK ships with the Windows SDK (`C:\Program Files (x86)\Windows Kits\10\App Certification Kit\`).
It installs the package, so it needs **an elevated prompt** and a package it can install. That
means either the dev-signed build with its test certificate trusted (§3), or a Store build re-signed
with that certificate. Partner Center runs equivalent checks on upload.

```powershell
# elevated PowerShell
$kit = 'C:\Program Files (x86)\Windows Kits\10\App Certification Kit'
& "$kit\appcert.exe" reset
& "$kit\appcert.exe" test -appxpackagepath "$PWD\target\msix\OpenFrame-Studio-0.1.0-x64-localtest.msix" -reportoutputpath "$PWD\target\msix\wack-report.xml"
# or interactively: "$kit\appcertui.exe" → Validate Store App → select the .msix
```

Review `wack-report.xml`. Every required test must pass: package sanity, manifest, app launch,
crash and hang, blocked executables, binary analyzer (ASLR, DEP, HighEntropyVA; Rust MSVC builds
set these by default), supported APIs (informational for full-trust apps), and branding (default
images). If the binary analyzer flags Control Flow Guard, add `-C control-flow-guard` for release
builds (📋 decide).

Also check manually on the installed package (§3): launch from Start. The Start tile, taskbar and
Alt+Tab icons must show the OpenFrame logo, not a placeholder. Create a project (it must appear under
`Documents\OpenFrame\Projects`), open the Global Idea Vault and add an image. Then check that
`%LOCALAPPDATA%\Packages\<family>\LocalState\OpenFrame\logs` holds today's log in Explorer. Once
Help → Open logs folder exists (doc 26 §3), it must open that folder. Launch
a second time: the running window must come to the front. Uninstall, then check that the projects
remain and `…\Packages\<family>` is gone.

## 9. Screenshots and images

- Desktop screenshots: **at least 1, recommended 4 to 8, up to 10**. PNG, **1366 × 768 or larger**
  (up to 3840 × 2160), up to 50 MB each, landscape or portrait, optional caption up to 200
  characters. Keep key content in the top two-thirds. No extra logos or marketing overlays. Plan
  in `docs/store/listing.md`.
- The 1:1 app tile icon (300 × 300) is strongly recommended. Otherwise the Store uses the package
  logo. Poster and box art are for games. 16:9 super hero art (1920 × 1080, no text) is optional.
- In-package logos come from `packaging/msix/Assets` (§2).

## 10. Blockers only the owner can clear ⛔

1. **Partner Center developer account** and identity verification (§6.1).
2. **Name reservation and Product identity values** (Name, Publisher, PublisherDisplayName) (§6.2–3).
3. **Final open-source license**. `release-validate` fails while `LICENSE-PENDING.md` exists, and
   the listing's copyright and license fields depend on it.
4. **Privacy policy hosting URL** and the placeholders in it: publisher name, contact, model
   download host and its log retention (§6.4).
5. Listing details: support contact or website, the screenshots themselves, category choice, and
   the IARC questionnaire answers (the account holder must submit them).
6. For AI builds: the model distribution host (`MODEL_DISTRIBUTION_BASE_URL`) and a
   report-inappropriate-content channel (policy 11.16).

Not blockers for the Store: the code-signing certificate and the updater keys, which are needed
only for the direct channel.

## 11. Verification record

Run on 2026-09-30 (Windows 11 26200, Windows SDK 10.0.26100.0, non-admin account, Developer Mode off):

| Step | Result |
|---|---|
| `OPENFRAME_DISTRIBUTION=store` `tauri build --no-bundle` (release profile) | ✅ `openframe-desktop.exe` 18.2 MB. It contains the `openframe-distribution-channel:store` marker. |
| `build-msix.mjs --store` with placeholder identity | ✅ Refused (identity incomplete), as designed |
| `build-msix.mjs --store` with a non-Store executable | ✅ Refused (channel check) |
| `build-msix.mjs --store --bundle` with a well-formed test identity | ✅ `makepri new` + `makeappx pack` succeeded (manifest schema and referenced files validated). `makeappx bundle` succeeded. |
| `makepri dump` of `resources.pri` | ✅ `Square44x44Logo.png` resolves to 47 candidates (5 scales + 42 target sizes). Every other logo resolves to its 5 scales. |
| `build-msix.mjs --dev` | ✅ Signed with an in-memory self-signed certificate (`signtool`, SHA256). `Get-AuthenticodeSignature` shows the signer as expected, and the chain is untrusted, as expected. |
| `Add-AppxPackage` of the dev package | ⛔ `0x800B0109` (root not trusted). Trusting the test certificate needs an elevated change to the machine trust store. The release engineer deliberately did not make it. |
| WACK (`appcert.exe`) | ⛔ Requires elevation ("The requested operation requires elevation"). Not run. |
| Staged `openframe-desktop.exe` launched unpackaged | ✅ Main window "OpenFrame Studio" opened. The log shows `channel=store packaged=false in_app_updates=false` and WebView2 154.0.4258.37 detected. |
| `release-validate --store --report-only` | Fails only on owner blockers: license, Partner Center identity, privacy policy URL. Version, assets, manifest capabilities and the no-updater check pass. |

Still to verify on a machine where the owner trusts the test certificate or enables Developer Mode:
install, the packaged launch (the startup log must show `packaged=true` and the `LocalState` data
path), Start/taskbar icons, the single instance, projects and vault in Documents, uninstall
cleanup, and a WACK run (§8).
