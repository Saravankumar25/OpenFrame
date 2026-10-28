# Mac App Store readiness

> Written 2026-10-27 on branch `mac/windows-sync`. A read-only review of the Swift shell, the Rust core and the packaging
> files against Apple's App Review Guidelines, the App Sandbox, the hardened runtime, the privacy manifest rules and export
> compliance. Only the files in `apps/macos/packaging/` were changed. Nothing was built, signed or uploaded. Publishing is
> Chapter 3 ([roadmap](../roadmap/README.md)); signing, notarisation, App Store Connect and the Apple Developer account are
> owner tasks. Related: [ADR-0033](../adr/ADR-0033-mac-ai-engine-helper-process-metal.md),
> [ADR-0023](../adr/ADR-0023-store-trial-and-internal-installer.md), [ADR-0024](../adr/ADR-0024-internal-team-builds.md),
> [mac-internal-release-gate.md](mac-internal-release-gate.md), [doc 38](../engineering/38-mac-app-store-trial.md),
> [doc 40 section 7](../engineering/40-storage-locations-and-recovery.md).

Status words:

- **Ready**: the code and files already meet the rule. A sandboxed build must still confirm it.
- **Fixed now**: wrong in `apps/macos/packaging/` and corrected in this change.
- **Not built**: work the store build needs that does not exist yet (code, script or design).
- **Owner task**: only the owner can do or decide it.

## 1. Entitlements against what the app does

| Requirement | Evidence in the code | Status |
|---|---|---|
| App Sandbox on the store build | `OpenFrameStudio-store.entitlements` sets `app-sandbox` | Ready |
| Files the person picks (`files.user-selected.read-write`) | `NSOpenPanel` and `NSSavePanel` in 12 source files, `.fileImporter` in the Idea Vault; paths go to the core in the same process, which keeps the panel's access | Ready |
| App-scope bookmarks (`files.bookmarks.app-scope`) | Built for linked files only: `LinkedFileBookmarks` in `OpenFrameUI/ProjectPages/ProjectPagesSupport.swift` stores `.withSecurityScope` bookmarks in user defaults. Bookmarks for a library or project folder outside the container are not built; the store build hides Choose new location, Locate, Move and Change Folder (`StorageRules.canRelocate`) | Ready for linked files; folder bookmarks Not built |
| Old comment said bookmarks also reopen projects | The store entitlements comment now says linked files only | Fixed now |
| Microphone (`device.audio-input`) | `AVAudioRecorder` and `AVCaptureDevice.requestAccess(for: .audio)` in `IdeaVault/VoiceRecording.swift`, only on Record | Ready |
| Printing (`print`) | `NSPrintOperation` in `Interchange/PrintPreviewModel.swift`. A sandboxed app without `com.apple.security.print` cannot print | Fixed now (key added) |
| Outgoing network (`network.client`) | Model download with `reqwest` to `huggingface.co` and `hf.co` only (`openframe-ai/src/fetch.rs`), started by the person; trial check with `URLSession` to the licence server (`Store/LicenceServerClient.swift`, store build only); the core's calls to the engine on 127.0.0.1 | Ready; comment now names all three |
| Licence server address | Both addresses are `.invalid` placeholders (`licenceServerURL`, `licenceServerSandboxURL`) | Owner task (deploy and set) |
| Loopback listener for the engine (`network.server`) | `llama-server --host 127.0.0.1 --port <free port>` with a random API key (`openframe-ai/src/supervisor.rs`) | Ready. Optional hardening: a Unix socket inside the container would remove this key |
| Helper inherits the sandbox | `llama-server-store.entitlements` has only `app-sandbox` and `inherit`, as Apple requires | Ready |
| Internal build entitlements | Hardened runtime plus `device.audio-input` only, no DYLD or library-validation exception | Ready |
| `application-identifier` and `team-identifier` | Added from the provisioning profile at signing; not written in the repository | Owner task |

## 2. Privacy manifest

Apple's required-reason APIs were checked in the shipped Swift targets and in the symbols the Rust core library imports
(`nm -u` on `libopenframe_ffi.a`): `stat`, `fstat`, `lstat`, `fstatat`, `statfs`, `fstatfs`, `statvfs`, `getfsstat`,
`mach_absolute_time`, `sysctl`. The end-to-end driver uses `systemUptime` but is never in the app bundle.

| Requirement | Evidence | Status |
|---|---|---|
| Tracking false, no tracking domains | No analytics, no ad or tracking SDK | Ready |
| File timestamp: `C617.1`, `3B52.1` | Rust `metadata().modified()`, SQLite `fstat`; files in the container or picked by the person | Ready |
| Disk space: `E174.1` | `statvfs` in `openframe-platform/src/drives.rs` before a model download or a library move | Ready |
| Disk space: `85F4.1` (shown to the person) | `AILogic.swift` shows "free on this drive" in AI settings | Fixed now (reason added) |
| System boot time: `35F9.1` | The core links `sysinfo`, which imports `mach_absolute_time` and reads the boot time sysctl | Fixed now (category added) |
| User defaults: `CA92.1` | `@AppStorage` and `UserDefaults.standard` for the app's own settings and bookmarks | Ready |
| Active keyboards | Not used | Ready |
| Collected data | The licence server logs Apple original transaction ids per check (`services/licence-server/src/http.rs`). Declared as purchase history, linked, app functionality, no tracking | Fixed now; owner confirms or drops the ids from the log |
| Old comment said no server except the model download | Comment now names the licence check | Fixed now |
| Third-party SDK manifests | No SDK from Apple's list is used | Ready |

## 3. Info.plist

The store build has no script yet. These findings are for the internal script `apps/macos/scripts/build-release-app.sh` and
what a store script must do differently.

| Requirement | Evidence | Status |
|---|---|---|
| Usage text for every protected resource | `NSMicrophoneUsageDescription` present in both scripts; no camera, contacts, photos, location or calendar use | Ready |
| Bundle identifier | Internal `studio.openframe.mac`; doc 38 and the StoreKit products use `com.openframe.studio` | Owner task (pick one; the store record and product ids must match) |
| `CFBundleVersion` | Internal value is `<version>.<commit>`, for example `0.1.0.6b1b25c0`. The store accepts only up to three numbers separated by dots, rising with each upload | Not built (store script must write a plain build number) |
| `CFBundleShortVersionString` | `0.1.0` from `Cargo.toml` | Ready |
| `LSApplicationCategoryType` | `public.app-category.productivity` | Ready; owner may pick another |
| Minimum macOS | `14.0`; Package.swift targets macOS 14; newer APIs are behind `#available(macOS 15, *)` and `15.4` | Ready |
| `ITSAppUsesNonExemptEncryption` | Missing. See section 6 | Not built (owner answer first) |
| Xcode build keys (`DTXcode`, `DTSDKName`, `DTPlatformName`, `CFBundleSupportedPlatforms`) | The app is built with `swift build` and a written Info.plist; App Store Connect refuses uploads without a released Xcode and SDK. This Mac has the Command Line Tools only | Owner task (install Xcode) and Not built (archive step) |
| Copyright line | Says "internal build"; store build needs the company name | Not built |
| App icon | `AppIcon.icns` holds 16 to 512 at 1x and 2x, including 1024 pixels | Ready |
| Privacy manifest location | Copied to `Contents/Resources/PrivacyInfo.xcprivacy` | Ready |

## 4. App Review risks

| Rule | Evidence | Status |
|---|---|---|
| Data stays in the container (2.5.2) | `directories` reads `$HOME`, which the sandbox points at the container, so `~/OpenFrame` and Application Support resolve inside it; doc 40 section 7 records this | Ready; confirm on a sandboxed build |
| Nothing written next to the app | `app_resources_dir()` only reads `Contents/Resources` | Ready |
| No downloaded code (2.4.5, 2.5.2) | The engine and its libraries ship inside the app. The download fetches only `<model id>.gguf` weights from a pinned Hugging Face commit, checked by size and SHA-256 (`download.rs`, `install.rs`). Importing a model copies it into app storage | Ready; explain in the review notes |
| Child processes do not outlive the app (2.4.5) | Kill-on-close guard and a `/bin/sh` watcher that ends the engine group within a second of the app ending (`openframe-platform/src/process.rs`) | Ready; confirm the watcher and its signal work inside the sandbox |
| Opening links and revealing files | Rust runs `/usr/bin/open` (`openframe-platform/src/shell.rs`); Swift already uses `NSWorkspace` in other screens | Not built: route open and reveal through `NSWorkspace` on the store build, or prove `/usr/bin/open` works in the sandbox |
| Team password gate absent from the store build | Not applicable since 2026-10-27: the team password gate was removed from every build (owner decision). `OPENFRAME_DISTRIBUTION=store` still selects the store channel | Ready (code); store build script Not built |
| Trial and purchase through StoreKit only (3.1.1) | StoreKit 2 bridge and runtime in `OpenFrameUI/Store/`; products `7-day Trial` (zero price, follows Apple's naming) and `Lifetime Access` in `OpenFrameStudio.storekit` | Ready |
| Trial sheet says length, what stops and the price before it starts (3.1.1) | Mock 187 shows no price and no Not now; doc 38 section 10 leaves this open | Owner task (decision already open) |
| No external payment links | No purchase or website link in the app; only fixed social links on the licence screens | Ready |
| Account deletion (5.1.1(v)) | No accounts | Not applicable |
| Sign-in required | None | Ready |
| Restore purchases | Restore purchase button and `AppStore.sync()` | Ready |
| Social links on the licence screen | A fixed list of social sites, https only (`LicenceRules.isFixedSocialURL`) | Ready |

## 5. The AI helper under the hardened runtime and the store signature

| Requirement | Evidence | Status |
|---|---|---|
| Every Mach-O signed with the team identity, sandbox and `inherit` on `llama-server` | The internal build seals the engine as resources and never signs it; the libraries carry only upstream ad hoc signatures (ADR-0033 decision 6) | Not built |
| Code in a code location | The engine sits in `Contents/Resources/ai/runtimes/<id>/`. Apple wants executables in `Contents/MacOS` or `Contents/Helpers` and libraries in `Contents/Frameworks`; upload checks scan every executable for the sandbox key | Not built (move it, update `app_resources_dir` users and the lock file layout) |
| Byte hashes against signing | The core re-hashes every engine file before each launch against `packaging/ai/ai-assets.lock.json`. Signing changes the bytes, and Apple signs again for distribution, so the pinned hashes will fail and the AI will refuse to start | Not built: on the store build check the helper by its code signature (team identifier and designated requirement) and keep byte hashes for non-code files only |
| Library validation | With the hardened runtime on the helper, every dylib must carry the same team identifier. Signing all eleven files with one identity meets it; no `disable-library-validation` exception is needed | Not built (comes with the signing step) |
| Unused RPC backend | `libggml-rpc.0.dylib` ships with the engine; the core never passes `--rpc` | Ready; dropping it is optional |

## 6. Export compliance

The app uses: TLS through `rustls` (in `reqwest`, for the model download), TLS through `URLSession` (licence check), SHA-256
for file integrity, Ed25519 signature checks on licence answers. No algorithm is proprietary. No user data is encrypted by OpenFrame.

- Signatures, hashes and password checks are authentication and integrity, which are exempt.
- `rustls` is standard encryption shipped in the app, not the operating system's. The cautious App Store Connect answer is
  therefore "standard encryption algorithms instead of, or in addition to, Apple's operating system", mass market, with the
  France question answered as Apple asks.
- Simpler path: switch `reqwest` on macOS from `rustls-tls` to `native-tls` (Apple's Security framework). Then all
  encryption is Apple's or exempt, and `ITSAppUsesNonExemptEncryption` can be `NO`.

Which answer applies is the owner's to confirm, with legal advice if in doubt. Status: Owner task.

## 7. Owner checklist to publish

1. Enrol in the Apple Developer Program as the company (Account Holder). Accept the Paid Apps agreement and finish tax and
   banking.
2. Install a released Xcode. Create the certificates Apple Distribution and Mac Installer Distribution.
3. Register the bundle identifier (decide `com.openframe.studio` or `studio.openframe.mac`, section 3) and create a Mac App
   Store provisioning profile for it.
4. Create the App Store Connect record: macOS, price Free, category, age rating answers, support and privacy policy
   addresses, and the in-app purchases from doc 38 section 10 (product ids must match `OpenFrameStudio.storekit`).
5. Deploy the two licence servers (Production and Sandbox) and replace the two `.invalid` addresses and the public keys.
6. Approve the store build work in sections 3 to 5 (build number, Xcode archive, helper location, helper signing, signature
   check instead of byte hashes, `NSWorkspace` for open and reveal), then the build itself.
7. Screenshots: at least one, at 1280 x 800, 1440 x 900, 2560 x 1600 or 2880 x 1800, from real screens.
8. App Privacy answers: no tracking; purchase history collected for app functionality and linked, unless the licence server
   stops logging transaction ids, in which case "no data collected". Audio from voice notes stays on the Mac and is not
   collected.
9. Export compliance answer from section 6, and `ITSAppUsesNonExemptEncryption` set to match.
10. Decide the trial sheet wording against guideline 3.1.1 (doc 38 section 10 item 5).
11. Review notes, for example: "OpenFrame Studio is a writing and production app. Its optional assistant runs fully on the
    Mac. The app bundle contains the llama.cpp engine (`llama-server` and its libraries, MIT licence), signed with our team
    identity and the App Sandbox with `com.apple.security.inherit`. The app starts it as a child process that listens only on
    127.0.0.1 with a random key and ends it when the app quits. When the reviewer chooses to download the assistant, the app
    fetches model weights (GGUF data files, no executable code) from huggingface.co at a pinned commit and checks each file's
    SHA-256. Everything else in the app works without the download. The 7-day trial is a zero-price non-consumable; the app
    checks it with our server, sending only Apple's signed transaction records." Add sandbox tester details and how long the
    download takes.
12. Upload through Xcode Organizer or Transporter, test in TestFlight with the sandbox testers, then submit the build and both
    in-app purchases together.

## 8. Not done in this review

- No sandboxed build was made, so no claim above was proven on a running sandboxed app.
- Apple's guideline text was checked from current knowledge, not fetched on the day. Read the live guidelines before
  submission.
- The licence server and the trial sheet were not reviewed beyond what this table names.
