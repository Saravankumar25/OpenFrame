# Tauri Mac shell notes (history)

> Archived 2026-10-17 in Mac phase M1. This is history, not a requirement. The Tauri Mac shell was retired by
> [ADR-0027](../adr/ADR-0027-rust-core-two-ui-shells.md) decision 3: the Mac application is a native SwiftUI shell over the
> shared Rust core ([chapter-1b-mac-phases.md](../roadmap/chapter-1b-mac-phases.md)). The compile-only CI workflow
> `.github/workflows/mac.yml` ("Mac build check") was deleted and replaced by `.github/workflows/mac-core.yml` ("Mac core check").
> The text below was moved here from `docs/engineering/25-ci-cd.md` and `docs/engineering/37-platform-boundary.md` (sections 7
> and 8). It is not edited.
>
> The Swift StoreKit bridge source (`apps/desktop/src-tauri/macos/StoreKitBridge`) is kept untouched. It moves to the SwiftUI
> app in Chapter 3 and is not used by the internal build.

## What the retired workflow did

`mac.yml` ran on `macos-latest` (Apple silicon, 30 minute timeout) when `apps/desktop/src-tauri/**`,
`crates/openframe-platform/**`, `crates/openframe-entitlement/**` or the workflow changed. Steps: Rust 1.98.1 with
`aarch64-apple-darwin`, rust-cache, `swift build -c release --product StoreKitBridge`, a placeholder `apps/desktop/dist`,
`cargo check -p openframe-desktop`, `cargo test -p openframe-platform -p openframe-entitlement`, and
`cargo test -p openframe-desktop --lib`. It never ran on a real runner.

## From docs/engineering/25-ci-cd.md

### `mac.yml` (written 2026-10-06, has not run yet)

Triggered by a pull request or a push to `main` that changes `apps/desktop/src-tauri/**`, `crates/openframe-platform/**`,
`crates/openframe-entitlement/**` or the workflow itself, and by hand. macOS runner minutes cost ten times Windows minutes, hence the
path filter.

| Job | Runner | Steps | Gating |
|---|---|---|---|
| `mac-build` | `macos-latest` (Apple silicon), 30 minute timeout | Rust 1.98.1 with `aarch64-apple-darwin`, rust-cache (same pinned actions as `ci.yml`) → `swift build -c release --product StoreKitBridge` → placeholder `apps/desktop/dist` → `cargo check -p openframe-desktop --target aarch64-apple-darwin --locked` → `cargo test -p openframe-platform -p openframe-entitlement --target aarch64-apple-darwin --locked` → `cargo test -p openframe-desktop --lib --target aarch64-apple-darwin --locked` (links a binary against the Swift bridge, which `cargo check` does not) | Not a required check yet: make it required once it has passed (doc 37 section 8) |

It compiles and links the macOS shell (doc 37 section 8) and runs the crate and shell unit tests on macOS. It does not sign, package
or start the app and never talks to StoreKit or the App Store.


## From docs/engineering/37-platform-boundary.md

## 7. Next steps for Mac continuous integration

`ci.yml` runs on `windows-latest` only. `.github/workflows/mac.yml` (section 8) is a first, narrow macOS job: it builds the
StoreKit bridge and the desktop shell and runs the `openframe-platform`, `openframe-entitlement` and desktop unit tests on Apple
silicon. It has not run yet. Nothing in section 3 marked unverified can be called verified until it has. Still to do, in order:

1. Add a macOS job on an Apple silicon runner (check the runner label) that runs `cargo fmt --check`, `cargo clippy`,
   `cargo test -p openframe-platform` and `cargo test -p openframe-entitlement`. These crates need no web view, so they are the first
   to run and the quickest to check.
2. Widen the job to the whole Rust workspace, then the web tests and the bindings drift check. Fix what fails on macOS in the
   crates, never by adding `cfg` to feature code.
3. Run the three macOS tests that exist but have never run: `mac_data_folder_is_under_application_support`,
   `a_guarded_child_gets_sigterm_when_the_guard_is_dropped` and the shared atomic tests on APFS.
4. Add the Mac-only checks the plan names: App Sandbox entitlements, security-scoped access to folders the user picks, and an
   outgoing network limited to the model download.
5. Only after steps 1 to 4, move PLAT-008 up in [feature-status.md](../implementation/feature-status.md), with the run as evidence.

Mac hardware or a Mac runner and an Apple Developer account are owner tasks in
[owner-decisions.md](../implementation/owner-decisions.md).

## 8. macOS shell adapter: StoreKit bridge

The Mac App Store build connects StoreKit 2 and the licence server to the entitlement service through the desktop shell. Written
on 2026-10-06 on Windows. **Unverified: none of it has been compiled on a Mac, linked or run** until the Mac workflow
(`.github/workflows/mac.yml`) passes. The product rules are in [38-mac-app-store-trial.md](38-mac-app-store-trial.md).

| File | What it does |
|---|---|
| `apps/desktop/src-tauri/macos/StoreKitBridge/` | Swift package (StoreKit 2 only, macOS 13 or later). Exports C functions that return `strdup` JSON, and listeners on `Transaction.updates` and `Storefront.updates` that call back into Rust. |
| `apps/desktop/src-tauri/build.rs` | For a macOS target only: `swift build -c release --product StoreKitBridge` into `OUT_DIR`, then links `libStoreKitBridge.a`, the `StoreKit` and `Foundation` frameworks and the Swift runtime paths from `swift -print-target-info`. Allow-listed for rule R5: it starts `swift` at build time, never in the app. |
| `src/platform/storekit.rs` | The bridge's JSON contract and its mapping onto the `openframe-entitlement` types (`VerifiedTransaction`, `BridgeError` and the rest). Pure Rust, compiled on macOS and in tests on every system. |
| `src/platform/macos_storekit.rs` | The C ABI calls (bounded reads, each string freed once) and `StoreKitChannel`, the crate's `StoreKitBridge` implementation. Allow-listed for rule R4. |
| `src/platform/mac_inputs.rs` | Builds `MacInputs`: the bridge, an HTTPS licence server client (`reqwest` blocking over rustls, 10 second timeout, 64 KiB answer cap, no redirects), nonces from `OsRng` and the monotonic `InstantClock`. The two server addresses and the Ed25519 response key are placeholders the owner replaces. With the placeholder key no request is ever sent and the state stays NeedsOnlineCheck. |
| `src/platform/revalidation.rs` | One background thread runs every check: at launch, on resume or reopen, after sleep (wall clock ahead of the monotonic clock), when focus returns after more than 30 minutes, on a StoreKit transaction or storefront change, and when the lease ends. |

**How it is wired.** `lib.rs` sets `AppConfig::mac_inputs` from `platform::mac_inputs()`, which is `None` on Windows. The
`EntitlementService` keeps the inputs and builds the Mac adapter on the first check, off the main thread, because building it
asks StoreKit for the environment. Until that check answers, the state is NeedsOnlineCheck. `EntitlementService::revalidate`
runs the channel's restore check today, and `EntitlementService::lease` uses fixed intervals by state. Both switch to the Mac
adapter's `revalidate` and `time_until_check` once `ChannelAdapter` exposes them. The log carries the reason and the state name,
never an identifier. No Tauri command is added.

**Known limits.** The adapter takes one response key for both server deployments. The environment comes from the verified
`AppTransaction`, with a build default (Production for release builds) when StoreKit cannot answer. The App Sandbox
`com.apple.security.network.client` entitlement is not added yet; it belongs to the Mac App Store package area.
