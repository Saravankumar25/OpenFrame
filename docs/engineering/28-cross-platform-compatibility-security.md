# OpenFrame Studio: Strong Windows + macOS Compatibility and Security Plan

> **Repository state 2026-10-06.** This document is a plan. What exists in code today is written in [37-platform-boundary.md](37-platform-boundary.md): the `openframe-platform` crate (app folders, atomic file replace, file existence check, managed child processes, package identity, keyboard labels), lint rule R6 and its allow-list, which parts are verified on Windows, and which parts compile for macOS but are unverified without Mac hardware. Entitlement and trial rules are in [36-entitlement-and-trial.md](36-entitlement-and-trial.md). Where this plan and document 37 disagree about what is built, document 37 wins.

> **Repository integration (2026-10-05).** Promoted from the supplied OS compatibility plan. **Scope is set by
> [ADR-0017](../adr/ADR-0017-cross-platform-windows-and-macos.md) (Accepted 2026-10-06):** Windows 11 only and Apple silicon only; three release files (Microsoft Store `.msix`, Windows installer `.exe`, Mac App Store `.pkg`) from one codebase with a Windows adapter and a macOS adapter. The macOS work is on the release path, not after it. Where the supplied text below mentions Intel, x86_64 or macOS x64, the owner decision wins: Apple silicon (arm64) only. Developer ID, DMG and direct-download material is reference only, because the Mac App Store package is the only Mac release file decided.
> Sections on Windows paths, network shares, SQLite durability, atomic replace, WebView policy, secrets, logging and the threat
> model apply to Windows now. External citations in the source were removed; re-check Apple and Microsoft requirements when the work starts.

**Target:** production-grade desktop application for Windows and macOS, with **Windows and macOS as equal first-class production targets**\
**Baseline:** OpenFrame `main` HEAD
`d30b5889ab5edf9f69d60f8e6d600cb11d4921a0`

------------------------------------------------------------------------

# 1. Product requirement

OpenFrame must not be treated as:

> "a Windows application that happens to compile on Mac."

It must be:

> **one product with one canonical application architecture and two
> hardened platform adapters.**
>
> **Windows and macOS are equal first-class production targets. Neither is a
> secondary, fallback, or port-only platform.**

The product invariant is:

``` text
same project
same domain rules
same command semantics
same permissions
same undo/redo semantics
same recovery guarantees
same AI safety boundary
same file format
same user-visible intent
```

while allowing:

``` text
Windows-specific packaging/process APIs
macOS-specific packaging/process APIs
```

where the operating system requires them.

Tauri officially supports macOS and Windows desktop development; its
current prerequisites document calls out Xcode/Command Line Tools for
macOS and Microsoft C++ Build Tools plus WebView2 for Windows.


------------------------------------------------------------------------

# 2. Compatibility architecture

Use:

``` text
                    ┌─────────────────┐
                    │ Application     │
                    │ domain/modules  │
                    └────────┬────────┘
                             │
                     PlatformServices
                             │
              ┌──────────────┴──────────────┐
              │                             │
      WindowsPlatform                 MacOSPlatform
              │                             │
        Win32 / Windows                macOS APIs
        WebView2                       WebKit/WebView
        Job Objects                    process groups
        Authenticode                   Developer ID
        NSIS/MSI/MSIX                 DMG/pkg/App Store
```

The application layer must never know which OS it is running on.

------------------------------------------------------------------------

# 3. Platform abstraction contract

Create a small platform service boundary.

``` rust
pub trait PlatformServices: Send + Sync {
    fn platform(&self) -> Platform;
    fn app_paths(&self) -> AppPaths;
    fn open_path(&self, path: &Path) -> AppResult<()>;
    fn reveal_path(&self, path: &Path) -> AppResult<()>;
    fn spawn_managed_process(
        &self,
        spec: ProcessSpec,
    ) -> AppResult<ManagedProcess>;
    fn terminate_process(
        &self,
        process: &ManagedProcess,
    ) -> AppResult<()>;
}
```

Keep this interface intentionally small.

Do not turn it into a dumping ground for business logic.

------------------------------------------------------------------------

# 4. Windows requirements

## 4.1 Runtime

Current project baseline is Windows 11.

Required:

-   x64 release
-   WebView2
-   MSVC Rust target
-   Windows C/C++ build tooling for development
-   current supported WebView2 runtime

Windows packaging already defines NSIS and MSI targets.

## 4.2 Paths

Use Windows known-folder APIs through Tauri/platform services.

Do not assume:

``` text
C:\Users\<name>\
```

because:

-   usernames vary
-   drive letters vary
-   corporate redirection exists
-   OneDrive/Documents redirection exists
-   permissions vary

Project data should remain user-owned.

## 4.3 File opening

Never launch a file from a path supplied by React.

Existing model is correct:

``` text
asset id
→ Rust resolves canonical path
→ validates scope
→ OS opener
```

For risky/executable file types:

``` text
reveal
```

instead of automatic launch.

## 4.4 Windows installer

Direct distribution:

-   NSIS
-   MSI

Store distribution:

-   MSIX

The existing release plan uses Authenticode for direct packages and
Microsoft signing for Store packages.

Microsoft's current MSIX guidance recommends trusted production signing;
Microsoft Store packages are signed by Microsoft during submission.


------------------------------------------------------------------------

# 5. macOS requirements

## 5.1 Architectures

Target (owner decision 2026-10-06):

``` text
Apple Silicon: arm64
```

There is no Intel (x86_64) build and no universal distribution.

CI must execute arm64 behavior on Apple silicon runners.

## 5.2 Minimum supported macOS

Choose and document a minimum supported macOS version based on the
Tauri/WebKit/runtime dependency set.

Do not promise a version simply because Rust compiles against it.

The release pipeline must validate:

``` text
minimum supported macOS
current macOS
current Apple Silicon
```

------------------------------------------------------------------------

# 6. macOS signing and notarization

For distribution outside the Mac App Store:

-   sign distributed executables
-   use Developer ID
-   enable Hardened Runtime
-   use secure timestamps
-   notarize
-   staple the notarization ticket where applicable
-   verify Gatekeeper behavior

Apple states that macOS notarization requires code signing, a Developer
ID certificate, Hardened Runtime, a secure timestamp, and appropriate
entitlements. 

Apple's current guidance also describes Developer ID as the certificate
mechanism for software distributed outside the Mac App Store.


### Required release validation

``` bash
codesign --verify --deep --strict --verbose
spctl --assess --type execute
xcrun stapler validate
```

Also inspect:

``` bash
codesign -d --entitlements :- OpenFrame.app
```

The entitlement set should be minimal.

------------------------------------------------------------------------

# 7. Hardened Runtime

Do not disable protections merely to make a component work.

Apple's Hardened Runtime blocks classes of runtime attacks and requires
explicit exceptions when a legitimate capability needs them. Apple
specifically warns that exceptions should be used only when necessary.


For OpenFrame:

-   avoid JIT-dependent components unless unavoidable
-   do not enable unsigned executable memory without a demonstrated need
-   do not enable arbitrary dynamic library loading
-   do not disable library validation unless a real plugin requirement
    exists
-   never add entitlements just because a third-party library suggests
    them without verification

------------------------------------------------------------------------

# 8. AI sidecar security on macOS

This is one of the most important portability areas.

Current Windows design includes:

-   managed `llama.cpp`
-   localhost binding
-   random port
-   per-launch key
-   controlled process creation
-   kill-on-close
-   artifact verification

The macOS implementation must achieve the same security properties.

### Required invariant

``` text
OpenFrame
  │
  ├─ launches trusted sidecar
  │
  ├─ sidecar listens only on 127.0.0.1
  │
  ├─ random port
  │
  ├─ random authentication key
  │
  ├─ no shell
  │
  ├─ executable comes from trusted managed location
  │
  ├─ hash verified
  │
  └─ process is terminated with OpenFrame
```

Never weaken this on macOS because process APIs are different.

------------------------------------------------------------------------

# 9. Process isolation strategy

## Windows

Use:

-   Job Object
-   `CREATE_NO_WINDOW`
-   explicit process handle tracking
-   startup orphan scan
-   kill-on-close semantics

## macOS

Use:

-   process group/session
-   explicit PID tracking
-   startup orphan detection
-   termination escalation
-   signed executable location
-   no shell
-   inherited environment minimized
-   controlled working directory
-   localhost only

The exact mechanism may differ. The security property must not.

------------------------------------------------------------------------

# 10. Filesystem security

Every filesystem operation must answer:

``` text
Who chose this path?
Who validated it?
Can it escape its allowed root?
Can it be a network location?
Can it be a symlink?
Can it be executable?
Can it overwrite an unrelated file?
Can it cause data loss?
```

## Project-relative operations

Use canonical roots:

``` text
project_root
assets_root
cache_root
```

Resolve and validate before use.

## User-selected paths

A dialog may return a path, but the path is still untrusted.

Validate in Rust.

Never trust frontend checks.

------------------------------------------------------------------------

# 11. Windows network-share behavior

The existing application deliberately refuses UNC/network-share project
locations.

Preserve this.

Examples:

``` text
\\server\share\project.openframe
```

must not become a silently supported project root.

Reason:

-   locking semantics
-   SQLite behavior
-   network failures
-   data corruption risk
-   unexpected network access
-   security implications

If future network storage support is ever required, it must be a
separate explicit architecture decision.

------------------------------------------------------------------------

# 12. macOS network volumes

Apply the same principle.

Do not silently assume:

``` text
/Volumes/NetworkShare/Project.openframe
```

is safe for a live SQLite project.

Detect/deny unsupported remote filesystem characteristics rather than
merely checking the path string.

If network volumes are unsupported:

``` text
clear error
+
no writes
+
no migration
```

The user must never discover the policy after data is damaged.

------------------------------------------------------------------------

# 13. SQLite durability

SQLite behavior is central to product safety.

For both OSes:

-   identical journal configuration
-   explicit busy timeout
-   integrity checks
-   transactional migrations
-   atomic backup
-   no concurrent writer
-   one application writer
-   read connection separated from writer where appropriate
-   no database copied while an unsafe write is occurring

Test:

``` text
normal save
crash during save
crash during migration
disk full
permission denied
drive removed
external volume removed
process killed
```

------------------------------------------------------------------------

# 14. Atomic file replacement

Windows and macOS differ in filesystem semantics.

Create:

``` rust
replace_file_atomic(old, new)
```

as a platform service.

Required guarantees:

1.  original remains valid until replacement is ready
2.  partial temporary output is never treated as final
3.  crash recovery can identify incomplete replacements
4.  permission metadata is handled deliberately
5.  errors become `storage.*` codes

Do not duplicate this logic in exporters.

------------------------------------------------------------------------

# 15. Project portability

A project should be portable between Windows and macOS where the product
format permits it.

Do not store:

``` text
C:\Users\Badri\...
```

or:

``` text
/Users/badri/...
```

as canonical project identity.

Store:

-   relative asset references
-   stable IDs
-   logical paths
-   metadata
-   explicit external-link state

External linked files should remain external and be revalidated on the
destination system.

------------------------------------------------------------------------

# 16. Path normalization

Never compare paths using string equality alone.

Platform-aware rules:

### Windows

Normalize:

-   case behavior where applicable
-   drive prefixes
-   separators
-   UNC rules
-   reserved names

### macOS

Handle:

-   `/Volumes`
-   Unicode normalization considerations
-   case-sensitive and case-insensitive filesystems
-   symlinks

The project format should avoid depending on filesystem case
sensitivity.

------------------------------------------------------------------------

# 17. Unicode compatibility

OpenFrame is a creative application and must support international
screenplay content.

Test:

-   Unicode filenames
-   Devanagari
-   Telugu
-   Hindi
-   accented Latin
-   CJK
-   Arabic where supported by editor/layout
-   emoji where permitted
-   mixed scripts

Test both:

``` text
Windows filesystem
macOS filesystem
```

and both:

``` text
screenplay storage
PDF export
DOCX export
Fountain/FDX
search
AI retrieval
```

------------------------------------------------------------------------

# 18. WebView security

Maintain:

``` text
script-src 'self'
object-src 'none'
frame-src 'none'
connect-src ipc:
```

No arbitrary HTTP from the UI.

No:

-   `eval`
-   `innerHTML`
-   `dangerouslySetInnerHTML`
-   unrestricted window opening
-   direct filesystem APIs

User URLs must continue through the Rust URL policy.

------------------------------------------------------------------------

# 19. macOS WebKit considerations

Do not assume Chromium/WebView2 behavior equals WebKit behavior.

Test:

-   clipboard
-   drag and drop
-   text selection
-   IME
-   keyboard shortcuts
-   focus
-   composition events
-   file dialogs
-   media
-   fonts
-   scrollbars
-   wheel gestures
-   context menus
-   editor behavior

The screenplay editor is particularly sensitive.

------------------------------------------------------------------------

# 20. Keyboard compatibility

Windows:

``` text
Ctrl
Alt
Shift
```

macOS:

``` text
Command
Option
Control
Shift
```

Do not hard-code Windows shortcut labels into components.

Define semantic commands:

``` text
save
undo
redo
find
replace
next scene
previous scene
new scene
```

Map them by platform.

------------------------------------------------------------------------

# 21. Window behavior

Abstract:

-   minimize
-   maximize
-   fullscreen
-   close
-   restore
-   window state
-   multi-window policy

Do not store platform-specific window state in project data.

Window state is app/user preference, not project content.

------------------------------------------------------------------------

# 22. Notifications and dialogs

Keep product behavior consistent while using native platform
presentation.

Do not make critical confirmation dependent on one platform's native
wording.

For example:

``` text
Permanent delete?
```

must mean the same thing on both systems.

------------------------------------------------------------------------

# 23. Permissions

OpenFrame should request the minimum OS privileges.

Windows:

-   current-user installation where possible
-   project files under user-owned directories
-   no unnecessary elevation

macOS:

-   avoid unnecessary privacy entitlements
-   use user-selected file access
-   avoid broad system access
-   if sandboxing is introduced, design security-scoped resource
    handling explicitly

Do not request camera, microphone, contacts, location or other sensitive
access unless a product feature truly requires it.

------------------------------------------------------------------------

# 24. Installer architecture

## Windows direct

``` text
signed executable
↓
signed NSIS/MSI
↓
timestamp
↓
signature verification
↓
release artifact
```

## Windows Store

``` text
MSIX
↓
Partner Center
↓
Microsoft signing
```

## macOS direct

``` text
.app
↓
Developer ID signing
↓
Hardened Runtime
↓
notarization
↓
staple
↓
DMG/PKG distribution
```

Do not share one packaging script with OS-specific assumptions hidden
inside.

------------------------------------------------------------------------

# 25. Updater architecture

The current Windows updater plan is approval-based.

Keep the same product rule on macOS:

``` text
check
→ tell user
→ user approves
→ download
→ verify signature
→ close project safely
→ update
→ restart
```

No silent update.

Store/App Store channels should use their platform-native update
mechanism.

Direct channels use OpenFrame's signed updater mechanism.

------------------------------------------------------------------------

# 26. Update rollback

Every update must have:

-   version
-   channel
-   artifact hash
-   signature
-   release metadata
-   minimum supported version if required

Before update:

``` text
save/checkpoint
close project
verify no active mutation
```

If update fails:

``` text
old application remains usable
```

Do not replace the running executable in an unsafe way.

------------------------------------------------------------------------

# 27. Security boundary for updates

Never allow:

``` text
project data
→ update server
```

Update requests should not contain project content.

Manifest is untrusted until signature verification succeeds.

The embedded/development signing key must never be trusted for
production.

------------------------------------------------------------------------

# 28. AI model distribution

Models are executable-adjacent assets and must be treated as untrusted
downloads.

Required:

``` text
manifest HTTPS
↓
signature verification
↓
artifact SHA-256
↓
size validation
↓
temporary download
↓
hash
↓
atomic install
```

Never execute a model/sidecar simply because its filename matches.

------------------------------------------------------------------------

# 29. Package/import security

Existing security review already established strong controls around:

-   ZIP traversal
-   absolute paths
-   UNC paths
-   symlinks
-   XML entities
-   PDF bombs
-   oversized parsing
-   schema injection
-   SQLite triggers/views
-   SQL identifier injection

These controls must remain cross-platform.

Add OS-specific tests for:

-   Windows drive paths
-   UNC
-   Windows device paths
-   macOS volume paths
-   symlinks
-   case collisions
-   Unicode normalization

------------------------------------------------------------------------

# 30. Secrets

Never store:

-   signing private keys
-   updater private keys
-   passwords
-   API credentials

in source.

CI secrets:

``` text
Windows certificate
Windows certificate password
Tauri updater private key
macOS signing credentials
Apple notarization credentials
```

must exist only in the CI secret manager or secure signing
infrastructure.

The repository may contain public verification keys only.

------------------------------------------------------------------------

# 31. Dependency security

Release blocking:

``` text
cargo deny
cargo audit
npm audit --omit=dev
license inventory
SBOM
```

Parser/runtime changes require targeted security tests.

Never suppress an advisory solely because:

> "it does not happen on my machine."

Document why a dependency is safe or replace it.

------------------------------------------------------------------------

# 32. Logging

Logs remain local.

Allowed:

``` text
operation
error code
duration
task id
version
platform
```

Forbidden:

``` text
screenplay text
private notes
contacts
AI context
full file contents
auth keys
tokens
```

Add automated log-redaction tests.

------------------------------------------------------------------------

# 33. Threat model

Primary threats:

  Threat                        Control
  ----------------------------- -------------------------------------------------
  compromised webview content   strict CSP + IPC allow-list
  malicious project             schema screening + integrity checks
  malicious package             parser sandboxing/limits + transactional import
  malicious path                path confinement
  malicious model download      signed manifest + hash
  AI prompt injection           untrusted-data boundary + tool constraints
  AI privilege escalation       capability checks + Change Sets
  local sidecar hijack          random port + per-launch key
  updater tampering             signed artifacts
  release tampering             code signing + notarization
  accidental data loss          SQLite transactions + undo + recovery
  platform regression           OS matrix + E2E

------------------------------------------------------------------------

# 34. Security properties that must be identical across OSes

The following are product-level invariants:

``` text
No arbitrary UI filesystem access
No arbitrary UI network access
No arbitrary shell execution
No automatic AI mutation
No permission bypass
No private-note leakage
No silent project migration
No unsafe project opening
No unverified model execution
No unsigned production updater
No content in logs
```

If Windows and macOS differ, the difference must be documented as an OS
implementation detail, not a weaker security policy.

------------------------------------------------------------------------

# 35. Cross-platform test matrix

## Unit

Run on:

``` text
Windows x64
macOS arm64
```

## Integration

Test:

-   project creation
-   open/close
-   migration
-   autosave
-   recovery
-   undo/redo
-   trash
-   files
-   search
-   AI
-   import/export

## E2E

Minimum journey:

``` text
launch
→ create project
→ create story
→ create scene
→ write screenplay
→ breakdown
→ schedule
→ call sheet
→ save
→ close
→ relaunch
→ reopen
→ edit
→ undo
→ redo
→ export
→ re-import
```

Run the same scenario on both OS families.

------------------------------------------------------------------------

# 36. Failure injection matrix

Test:

### Storage

-   disk full
-   permission denied
-   path disappears
-   external volume removed

### Process

-   AI sidecar crashes
-   AI sidecar hangs
-   OpenFrame killed during save
-   OpenFrame killed during migration

### Network

-   unavailable
-   manifest unavailable
-   model download interrupted
-   update download interrupted

### Files

-   malformed ZIP
-   malformed XML
-   malformed PDF
-   enormous file
-   symlink
-   path traversal
-   Unicode path

The product must fail safely rather than partially succeed.

------------------------------------------------------------------------

# 37. Release checklist

## Windows

-   [ ] x64 build
-   [ ] WebView2 behavior
-   [ ] Authenticode signature
-   [ ] timestamp
-   [ ] NSIS
-   [ ] MSI
-   [ ] Microsoft Store MSIX
-   [ ] SmartScreen behavior tested
-   [ ] uninstall preserves projects
-   [ ] updater tested

## macOS

-   [ ] arm64 (Apple silicon only)
-   [ ] minimum supported macOS
-   [ ] Developer ID Application signing
-   [ ] Hardened Runtime
-   [ ] minimal entitlements
-   [ ] notarization
-   [ ] stapling
-   [ ] Gatekeeper test
-   [ ] DMG/PKG
-   [ ] updater tested
-   [ ] quarantine test

------------------------------------------------------------------------

# 38. Implementation sequence without disruption

## Route current Windows behavior through the platform abstraction (no behavior change)

Introduce:

``` text
PlatformServices
AppPaths
ProcessSpec
ManagedProcess
```

and route current Windows behavior through the abstraction.

## Add macOS to CI

Add macOS:

``` text
cargo test
cargo clippy
frontend typecheck
eslint
vitest
bindings
```

## Move filesystem work behind platform services

Move:

-   paths
-   open/reveal
-   atomic replacement
-   temp directories

behind platform services.

## Port the AI sidecar lifecycle

Port AI lifecycle with identical security properties.

## Add macOS packaging and signing

Add:

-   macOS signing
-   notarization
-   distribution artifact
-   verification

## Run the full end-to-end journeys on both platforms

Run identical product journeys.

Only after these gates pass should macOS be called release-ready.

------------------------------------------------------------------------

# 39. Definition of "never break"

"Never break" cannot mean mathematically zero defects.

For OpenFrame it should mean:

> **No known release may violate a protected data, security,
> compatibility, or recovery invariant.**

Release must be blocked by:

-   data corruption
-   migration failure
-   private-note leak
-   AI privilege bypass
-   unsigned/unverifiable update
-   broken installer
-   failed notarization
-   OS-specific canonical data divergence
-   project reopen failure
-   import/export semantic loss
-   crash recovery failure

------------------------------------------------------------------------

# 40. Final platform principle

Do not create:

``` text
OpenFrame Windows
OpenFrame Mac
```

Create:

``` text
OpenFrame
  ├─ shared product/domain
  ├─ shared project format
  ├─ shared security invariants
  ├─ shared AI architecture
  ├─ shared UI
  ├─ Windows adapter
  └─ macOS adapter
```

That is the architecture most likely to let the current ecosystem grow
without turning every future feature into two implementations.
