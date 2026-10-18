# Mac phase M9: security review of the Mac surface

> 2026-10-18. Branch `mac/m1-toolchain`, HEAD `d69efa0`. Read-only review: no code was changed, nothing was built or
> run except read-only inspection of the existing `release-out/mac/OpenFrame Studio.app` (`codesign -dv`, `otool -L`,
> `strings`, `xattr`). The shared Rust core was reviewed for Windows already (gate rows S1 to S15 in
> [windows-internal-release-gate.md](../../windows-internal-release-gate.md),
> [security-mapping.md](../w9/security-mapping.md)); this review covers only what the Mac adds. Every finding below was
> checked in the code at the line given.

## Findings

| Id | Area | Severity | File:line | What an attacker does | Recommended fix |
|---|---|---|---|---|---|
| MAC-01 | Paths and panels (Story attachments) | High | `apps/macos/Sources/OpenFrameUI/Story/StoryComponents.swift:241` | Sends a project or package whose story card has an attachment. The button calls `NSWorkspace.shared.open(URL(fileURLWithPath: a.asset.path))` with the path straight from the project database (a linked `external_path` can be any local file). This skips everything the core's `open_asset` does: the team gate and store lock check, the fresh lookup by asset id, and the "show executable types in Finder instead of opening them" rule (PKG-01). One click opens whatever file the sender named. | Call `core.openAsset(a.asset.id)` like every other open button (Idea Vault, Files). Add a grep check in `apps/macos/scripts/test.sh` that fails on `NSWorkspace.shared.open(` with a file URL outside the core adapter. |
| MAC-02 | Paths and panels (core rule used by the Mac) | High | `crates/openframe-security/src/lib.rs:324` (list), `:401` (`is_dangerous_to_open`); used by `crates/openframe-ffi/src/shell.rs:81` | The list of types that are shown in Finder instead of opened is Windows-only. On the Mac `open_asset` hands `/usr/bin/open` any file whose extension is not on it, so an attachment named `.fileloc`, `.webloc`, `.inetloc` (open another file, app or URL scheme), `.terminal`, `.command`, `.tool` (Terminal), `.pkg`/`.mpkg` (Installer), `.scpt`/`.applescript`/`.scptd`, `.workflow`, `.app`, `.dmg`, `.mobileconfig`, `.zsh`/`.bash`/`.csh` is opened by its handler. This is the Mac form of PKG-01, which was High on Windows. | Add the macOS launchable types to `EXECUTABLE_EXTENSIONS` (or a Mac list under `cfg(target_os = "macos")`), plus `.prefpane`, `.saver`, `.osax`, `.plugin`, `.bundle`, `.kext`, `.dylib`. Add a test like `executables_are_never_shell_opened` for the Mac names. |
| MAC-03 | Packaging, code injection | Medium | `apps/macos/scripts/build-release-app.sh:106`; `apps/macos/E2E/run.sh:39`, `:54` | The internal app is signed ad hoc without the hardened runtime (checked on the built app: `flags=0x2(adhoc)`, no `runtime` flag). Any program running as the same user can start it with `DYLD_INSERT_LIBRARIES=evil.dylib`; the library runs inside OpenFrame and inherits OpenFrame's privacy grants (microphone for voice notes, the Documents folder that holds the projects), which the attacker's own program would have to ask the person for. It can also call the core directly after the person types the team password. `E2E/run.sh` depends on this injection, which is why the shipped build is left open. | Sign the shipped app with `--options runtime` and an entitlements file holding only `com.apple.security.device.audio-input` (needed for the microphone under the hardened runtime). Do not add `com.apple.security.cs.allow-dyld-environment-variables` or `disable-library-validation` to the shipped app; it links only system libraries (`otool -L`), so library validation costs nothing. For tests, `run.sh` copies the app to its work folder and re-signs only the copy without the runtime (or with `allow-dyld-environment-variables`), then injects the driver into the copy. The store build (Chapter 3) uses the hardened runtime and App Sandbox with no DYLD exception. Leave the engine (`Resources/ai`) as it is for the internal build: it starts with an empty environment (`crates/openframe-ai/src/supervisor.rs:600`), so DYLD variables never reach it. |
| MAC-04 | Packaging, Gatekeeper | Medium | `apps/macos/scripts/build-release-app.sh:77` to `:100` (Info.plist) | Files OpenFrame writes from a received package or project (managed assets, restored packages) carry no `com.apple.quarantine` flag, because the Info.plist lacks `LSFileQuarantineEnabled`. When such a file is later opened (MAC-01, MAC-02, or by the person in Finder), Gatekeeper does not check it as a download. | Add `<key>LSFileQuarantineEnabled</key><true/>` to the internal and store Info.plist, so every file the app creates is quarantined. Files the person saves as exports get the flag too, which is harmless for PDFs and text. |
| MAC-05 | Paths and panels (character avatars) | Low | `apps/macos/Sources/OpenFrameUI/Story/StoryCharacters.swift:299` | A received project sets a character image to a linked path. The avatar calls `NSImage(contentsOfFile:)` on that path: it reads a local image outside the managed folders (the core's `asset_file` refuses that for previews, `crates/openframe-ffi/src/shell.rs:67`) and decodes it at full size with no limit, so a very large image can use a lot of memory. Nothing leaves the Mac. | Load avatars through `core.assetFile(id)` and a thumbnail decode with `kCGImageSourceThumbnailMaxPixelSize`, as `IdeaVault/VaultThumbnails.swift:28` already does. |
| MAC-06 | Team gate | Low | `crates/openframe-entitlement/src/team_gate/mod.rs:225` to `:237`; `apps/macos/Sources/OpenFrameUI/App/OpenFrameStudioApp.swift:19` | The wrong-try wait is kept in `entitlement/team-gate.json` under the app data folder. Deleting that file, or starting the app with `OPENFRAME_DATA_ROOT` set to a new folder (honoured by release builds, as on Windows), resets the count to zero, so the waits after 5 wrong tries do not slow guessing. This is not a bypass: the password is still checked in the core for every operation. The Argon2id hash is also readable in the binary (`strings` finds it), which ADR-0024 already accepts. | Record in ADR-0024 that the wait is a convenience, not a limit, or keep a second copy of the count outside the data root (for example in the user defaults of `studio.openframe.mac`) and use the larger of the two. |
| MAC-07 | Packaging (scan script) | Low | `scripts/scan-installer.mjs:33` to `:37`; `apps/macos/scripts/build-dmg.sh:26`, `:54` | Not an attack. `walk` uses `fs.statSync`, which follows links. The disk image holds the `Applications` link to `/Applications`, so the scan of the mounted image walks every app installed on the build Mac. The scan can fail on files it cannot read, take very long, or report findings that are not in the image, and a link loop would never end. | Use `fs.lstatSync` in `walk` and skip symbolic links (report them by name instead). |
| MAC-08 | Packaging (build machine) | Low | `apps/macos/scripts/build-release-app.sh:30`, `:50` to `:51` | The release build copies the sources to the fixed folder `/private/tmp/openframe-release-stage/macos` and builds there. Another account on the same Mac can create that folder first and change files in it during the build, so the built app carries its code. Only matters on a shared build Mac. | Create the stage with `mktemp -d` (or under `.build-cache`, which is inside the owner's checkout) and keep the remap prefix pointing at it. |
| MAC-09 | Paths and panels (security-scoped bookmarks) | Low | `apps/macos/Sources/OpenFrameUI/ProjectPages/ProjectPagesSupport.swift:205` to `:236` | Matters only in the sandboxed store build. Bookmarks are kept in one app-wide table keyed by file path. A project from someone else that links an asset to the same path as a file the person linked in another project gets that access while the core works on it. | Key the table by project id and asset id, and drop a bookmark when its asset is unlinked or its project is deleted. |
| MAC-10 | FFI boundary | Info | `crates/openframe-ffi/src/lib.rs:325`; `crates/openframe-ffi/src/shell.rs:141` | `open_url` does not check the team gate, unlike `open_asset` and `reveal_location`. Before the password is typed the shell could open an http or https link; no screen does this and only `web_link` URLs pass. | Call `core.entitlement.check_operation("shell.open_url")` first, for parity with the other shell requests. |
| MAC-11 | AI engine | Info | `crates/openframe-ai/src/supervisor.rs:583` (verify), `:591` (spawn) | The app copied to `/Applications` is owned by the person, so a program running as that person could swap `llama-server` between the hash check and the start. That program can already run its own code as the person; the gain would be the app's privacy grants, which MAC-03 gives more easily. | No change for Chapter 1. With the hardened runtime and a Developer ID signature (Chapter 3), check the engine's code signature as well as its hashes. |
| MAC-12 | Packaging (store build, Chapter 3) | Info | `apps/macos/packaging/llama-server-store.entitlements`; `packaging/ai/ai-assets.lock.json:276` | Not an attack. The store build must sign `llama-server` and its libraries with the inherit entitlement, which rewrites their bytes; the pinned SHA-256 values (checked before every start) then no longer match and the engine refuses to start. Apple also expects helper programs under `Contents/MacOS` or `Contents/Helpers`, not `Contents/Resources`. | When the Chapter 3 plan is written: pin the hashes of the signed files (or verify the Developer ID signature instead), and place the engine where App Store validation accepts it. |

No Critical findings.

## 1. FFI boundary

Checked `crates/openframe-ffi/src/lib.rs`, `crates/openframe-ffi/build.rs`, `crates/openframe-ffi/src/shell.rs` and
`apps/macos/Sources/OpenFrameCore/Core.swift`. Passed:

- One door: every operation goes through `CoreHandle::invoke`, which runs `AppCore::dispatch` and its entitlement and
  permission checks. The extra exports are `open_asset`, `asset_file`, `reveal_location` (each checks the team gate and
  store lock first and takes an id, not a path, except `exported`, which only accepts files OpenFrame wrote this
  session), `open_url` (only `web_link` URLs; gate check missing, MAC-10), `cancel_task`, `power_event`,
  `request_store_check` and `stop`.
- `asset_file` answers only files inside the managed `assets/` folders, after `canonicalize`, so links out of the folder
  are refused (test `a_path_outside_the_managed_folders_is_not_inside`).
- Isolated data root: refused unless absolute and creatable, never falling back to real data (lib.rs:130). Deleted
  projects of a test run go to its own `recycle-bin`.
- Test switches: `test_team_password_hash` and `test_store_response_key_hex` are honoured only when
  `cfg!(debug_assertions)` and an isolated root are both true (lib.rs:174 to 183, test
  `the_test_gate_hash_is_honoured_only_by_a_debug_build_on_a_test_folder`). `entitlement_debug_controls` follows the same
  rule. The release profile (and `perf`, which inherits it) has no `debug-assertions` override in `Cargo.toml`.
- Team gate hash: `build.rs` embeds an Argon2id hash with a random 16-byte salt from `getrandom`; an internal release build
  without `OPENFRAME_TEAM_PASSWORD`, or with fewer than 8 characters, fails (`build_plan`). A store build
  (`OPENFRAME_DISTRIBUTION=store`) embeds nothing and sets the store channel; any other value fails the build. The
  password is never printed and the hash file is removed before every run of the script.
- `OPENFRAME_TEST_MODEL_SOURCE` must be `http://127.0.0.1:<port>` and needs `OPENFRAME_TEST_BUILD=1`.
- Errors: the Swift side shows only `message`; `detail` stays in diagnostics. `invoke` logs the operation name and error
  code, never arguments, so the team password typed into `team_gate.unlock` is not logged.

## 2. Team gate on the Mac

Checked `LaunchOptions` (`OpenFrameStudioApp.swift:16`), `AppModel.swift`, `TeamGateView.swift`,
`crates/openframe-entitlement/src/team_gate/` and `crates/openframe-application/src/modules/entitlement.rs`. Passed:

- The gate lives in the core. `check_operation` refuses every operation except `team_gate.status`, `team_gate.unlock`
  and `appearance.get` until the password is right (entitlement.rs:567). The Swift phase machine only decides which
  screen to show, so skipping the screen gives nothing.
- The unlocked state is an `AtomicBool` in memory; nothing on disk, in user defaults or in the data folder can mark the
  gate open. `team-gate.json` holds only the wrong-try count, and a damaged file starts a wait (but a deleted one does
  not, MAC-06).
- Environment: `LaunchOptions` reads only `OPENFRAME_DATA_ROOT` (absolute or the app stops). It never sets
  `testTeamPasswordHash`, so no environment variable, launch argument or user default reaches the test hash. Even if it
  did, a release core ignores it.
- `OPENFRAME_E2E_*` variables are read only by the driver library, which is not in the app. The driver types the real
  password from `OPENFRAME_E2E_TEAM_PASSWORD` into the password field; it does not bypass the gate.
- `startup_maintenance` does nothing while the gate is shut. The password field is a `SecureField` and is cleared after
  a wrong try.

## 3. End-to-end driver

Checked `Package.swift`, `Sources/OpenFrameE2EDriver`, `Sources/OpenFrameE2EShim/shim.c`, `E2E/run.sh` and
`scripts/build-release-app.sh`. Passed:

- The driver is a separate dynamic library product. `OpenFrameStudio` depends only on `OpenFrameUI`. The release script
  builds the driver but copies only the executable, the SwiftPM `*.bundle` resource folders, the privacy manifest, the
  icon and the engine into the app.
- On the existing `release-out/mac/OpenFrame Studio.app`: no `.dylib` outside `Resources/ai`, no file named `*E2E*`, no
  `OPENFRAME_E2E` string in the executable, and `otool -L` lists only system libraries.
- The shim's constructor runs the driver only when `OPENFRAME_E2E_SCENARIO` is set. The driver opens no socket or network
  connection.

Not passed: the shipped app accepts `DYLD_INSERT_LIBRARIES` (MAC-03). Recommendation in short:

- Internal `.dmg`: hardened runtime on the app, entitlement `com.apple.security.device.audio-input` only, library
  validation left on, no `allow-dyld-environment-variables`. The test runner re-signs a private copy for injection.
- Store build: hardened runtime and App Sandbox, the entitlements already in `packaging/OpenFrameStudio-store.entitlements`,
  no DYLD or library validation exception; the engine helper signed with `llama-server-store.entitlements` (see MAC-12).

## 4. File paths and panels in Swift

Checked every `NSOpenPanel`, `NSSavePanel`, `fileImporter`, `NSWorkspace` call and every place a `URL(fileURLWithPath:)`
is built. Passed:

- Panels return paths that the core validates (`user_path`, `validate_output_file`, protected folders, confinement).
  Swift never joins a user or file name onto a folder: the only `appendingPathComponent` calls use fixed names or a
  UUID (`VoiceRecording.swift:61`, `PrintPreviewModel.swift:60` and `:116`, both under the per-user temporary folder).
- "Reveal in Finder" buttons (`ScheduleSheets.swift:596`, `VisualComponents.swift:462`, `CallSheetExport.swift:188`) only
  reveal export results; revealing never runs a file.
- Idea Vault thumbnails, audio and video go through `core.assetFile`, so only managed files are read, and thumbnails are
  decoded with a pixel limit (`VaultThumbnails.swift:33`).
- The Idea Vault microphone link opens only the fixed `x-apple.systempreferences:` address (`VaultSheets.swift:237`).
- `ExternalAccess` creates security-scoped bookmarks only from URLs the person picked and holds access only while the
  core works. Keying is weak for the store build (MAC-09).

Not passed: MAC-01 (Story attachments) and MAC-05 (character avatars) read or open a path from the project directly.

## 5. Process control and the AI engine

Checked `crates/openframe-platform/src/process.rs` (macOS module), `crates/openframe-ai/src/supervisor.rs`,
`crates/openframe-ai/src/engines.rs`, `packaging/ai/fetch-macos-runtime.sh` and `crates/openframe-platform/src/shell.rs`.
Passed:

- The engine folder comes only from the app bundle: `Bundle.main.resourceURL/ai`, and the core drops a relative path
  (test `a_relative_ai_folder_is_ignored`). There is no fallback to a writable folder; downloaded models go to the app
  data folder, but programs do not.
- Before every start, every engine file is hashed against the lock, links in the folder are refused, and an added
  program file (`.dylib`, `.so`, `.metallib`, a name without an extension) stops the start. The bundle copy on disk is
  unchanged by `codesign --deep` (the engine sits in `Resources`, which is sealed, not re-signed; `llama-server` keeps its
  linker ad hoc signature).
- The engine starts with an empty environment plus `TMPDIR` and `HOME` (and Windows names), so `DYLD_*`, `LLAMA_*`,
  `GGML_*` and `PATH` never reach it. It listens on `--host 127.0.0.1` on a free loopback port with a random 32-byte API
  key.
- macOS process control: the child leads its own process group; the guard signals the group only while the pid still
  leads it; the startup sweep kills a recorded engine only when pid, start time and program path all match.
- Quarantine: the fetch script clears extended attributes on the engine folder at build time, checks the archive and
  every file against the lock, refuses `..` and absolute names in the archive, and only runs `--version`. The `.dmg`
  read-me tells the team to use Control-click Open.
- `/usr/bin/open` is called with a fixed path and `--` before the target, so a target cannot be read as an option.

## 6. Packaging

Checked `build-release-app.sh`, `build-dmg.sh`, both entitlements files, `PrivacyInfo.xcprivacy`, the Info.plist keys and
`scripts/scan-installer.mjs`. Passed:

- The release script refuses to run without `OPENFRAME_TEAM_PASSWORD`, checks that the core carries an `$argon2id$` hash,
  remaps build paths out of the binary, strips the executable and runs the secret, personal path, user name and
  plain-password scan on the app; `build-dmg.sh` verifies the signature and scans the mounted image again.
- Info.plist: only `NSMicrophoneUsageDescription` asks for a privacy grant. No URL schemes, document types or services are
  registered, so other programs cannot send the app links or files. No ATS exceptions.
- Store entitlements are the least the store build needs (sandbox, user-selected files, app-scope bookmarks, network
  client for the model download, network server for the loopback engine, audio input). The engine helper has sandbox
  and inherit only.
- The privacy manifest declares no tracking and no collected data, with reasons for file dates, disk space and user
  defaults.

Not passed: no hardened runtime (MAC-03), no `LSFileQuarantineEnabled` (MAC-04), the scan follows the `Applications`
link (MAC-07), the predictable staging folder (MAC-08), and the Chapter 3 signing note (MAC-12).

## 7. Web links and AI answer rendering

Checked `Sources/OpenFrameUI/AI/` and every `openURL` caller. Passed:

- All web links go through `core.openURL`, which allows only `http` and `https` with no user part, no spaces, quotes,
  angle brackets or control characters, and at most 4,096 characters (`web_link`), and only after a click (Idea Vault
  item "Open", moodboard "Open Link"). Nothing opens a string from an AI answer.
- AI answers are shown with `Text(String)`, which SwiftUI does not read as Markdown, so there are no live links. There is
  no `AttributedString(markdown:)`, `NSAttributedString` HTML, `WKWebView`, `AsyncImage` or remote image anywhere in the
  Mac interface.
- AI navigation (`AIRoute`) maps a fixed list of workspace names; unknown names are ignored.
- "Report AI output" writes a JSON file only to the place the person picks in a save panel.

## What was not done

- Nothing was built, run or tested; the findings come from reading the code and inspecting the existing release app.
- `check_user_path` and `is_local_disk_path` were not re-reviewed for Mac volume paths (`/Volumes/...`, SMB mounts); the
  Windows review covered UNC and mapped drives only. Worth a follow-up with MAC-02.
- `.terminal` and `.fileloc` behaviour on macOS 14 and 15 was not tried by hand; MAC-02 rests on these types being handed
  to another program, which is enough to keep them out of a one-click open.

## Fixes applied on 2026-10-18

| Finding | Status | Change |
|---|---|---|
| MAC-01 | Fixed | Story card attachments open through `core.openAsset`, like every other open button (`apps/macos/Sources/OpenFrameUI/Story/StoryComponents.swift`). |
| MAC-02 | Fixed | `is_dangerous_to_open` also refuses the macOS types that run, install or follow a link (`.command`, `.terminal`, `.fileloc`, `.webloc`, `.inetloc`, `.pkg`, `.scpt`, `.workflow`, `.dmg`, `.mobileconfig`, `.app` and more), on every platform because the architecture lint keeps operating-system checks out of feature crates. Test: `mac_files_that_run_or_open_other_things_are_shown_not_opened`. |
| MAC-03 | Fixed | The internal app is signed with the hardened runtime and one entitlement, the microphone (`apps/macos/packaging/OpenFrameStudio-internal.entitlements`); no DYLD or library-validation exception. The end-to-end runner tests a private copy re-signed without the hardened runtime and prints the hashes of both. The engine files keep their pinned bytes and are not re-signed. |
| MAC-04 | Fixed | `LSFileQuarantineEnabled` is set in the Info.plist. |
| Scanner follows the image's Applications link | Fixed | `scripts/scan-installer.mjs` does not follow links. |
| Fixed staging folder in /private/tmp | Fixed | The release build stages in the per-user temporary folder (mode 700). |
| Other Low and Info findings | Open | Recorded for Chapter 3 (store build) or as accepted for the internal build; none blocks the internal disk image. |
