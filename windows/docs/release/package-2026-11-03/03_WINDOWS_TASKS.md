# 03. Windows tasks (Microsoft Store, MSIX)

Read 02 first. This file is only what is specific to Windows. Everything below was checked against the real code on 2026-11-02 and against Microsoft's documentation on 2026-11-03.

## State of the real code today

- `MicrosoftStoreAdapter` exists over a `StoreLicenceSource` trait (`app_licence`, `request_purchase`) in `crates/straft-entitlement/src/adapters.rs`, with the mapping in `store_licence_state`: active non-trial gives `Purchased`; active trial gives `TrialActive` with `ceil(seconds / 86400)` clamped to 1..7; inactive gives `TrialEnded`; unreachable gives `NeedsOnlineCheck`.
- **The real WinRT `StoreContext` calls are not written.** `src-tauri/src/lib.rs` (around lines 506 to 510) has a comment saying the Store calls are not wired until the Partner Center identity exists. Everything above runs only against test doubles.
- `EntitlementService` (`modules/entitlement.rs`) evaluates at Launch, Restore and Revalidate. For non-Mac channels `lease()` is `None`, so on Windows the state is read once and then served from memory. A trial that ends while the app stays open is only noticed at the next launch. (The UI comment in `licence.ts` says exactly this.)
- A refund maps to `TrialEnded`, which shows the wrong words.
- Store builds already compile out the in-app updater and use the package's own data folder (`distribution.rs`). Keep.

## Facts from Microsoft's documentation (confirm again when you build)

- Get the licence with `StoreContext.GetDefault()` then `GetAppLicenseAsync()`. `StoreAppLicense` has `IsActive`, `IsTrial`, `IsTrialOwnedByThisUser`, `TrialTimeRemaining`, `TrialUniqueId`, `ExpirationDate`, `SkuStoreId`, `ExtendedJsonData`.
- `StoreContext.OfflineLicensesChanged` fires when the licence changes while the app runs (trial expiry, purchase in the Store).
- If the trial expired before launch, the app does not launch; the Store shows its own purchase dialog. If it expires while the app runs, the app should show a message with a prompt to buy.
- A desktop app (Win32, Tauri) **must** tell the `StoreContext` which window owns modal dialogs (`IInitializeWithWindow::Initialize(hwnd)`), or the object returns inaccurate data or errors.
- Partner Center, Pricing and availability, Free trial: choose **Time-limited**, then **7 days**. (Options are 1, 7, 15 or 30 days. The owner sets this when the app record exists.)
- Writes to the package install folder are not allowed (read-only). App data belongs in the package's own data folder.

## Tasks

### W1. Write the real Store source
- Implement `StoreLicenceSource` for real in `src-tauri` (platform boundary; the Rust core must not depend on WinRT). Use the `windows` crate (`Services_Store`, `Win32_UI_Shell` for `IInitializeWithWindow`). Use the Tauri main window's HWND.
- `app_licence()`: `GetAppLicenseAsync()`, then return `{ is_active, is_trial, trial_seconds_remaining }`. For the trial seconds use `TrialTimeRemaining`; if it is zero or missing while `IsTrial` is true, fall back to `ExpirationDate - now`, never to a locally kept date.
- `request_purchase()`: `GetStoreProductForCurrentAppAsync()` then the product's `RequestPurchaseAsync()` (confirm exact names in the crate). Map `StorePurchaseStatus`: `Succeeded` and `AlreadyPurchased` give purchased; `NotPurchased` gives a calm "Not purchased" outcome with no error; `NetworkError` and `ServerError` give plain-words messages (never a raw HRESULT on screen). Re-read the licence after any result.
- Run all calls off the UI thread with a timeout (suggest 20 s) and map a timeout to "could not reach the store", never to a lock.
- Unpackaged or internal builds: the source reports unavailable; the channel there is `DirectInstaller` and no screen shows (unchanged).
- Keep the test double and add tests for every mapping row in W3.

### W2. Keep the state fresh (the gap)
Make `EntitlementService` re-read the licence on Windows. Triggers:
1. Launch and Restore (existing).
2. `OfflineLicensesChanged` (register once after the first read; handler calls the same revalidate).
3. Window comes to the front, if the last read was more than 5 minutes ago.
4. Every 15 minutes while the app runs (add a Windows interval to `lease()` or a separate refresh task; do not add a fake lease).
5. At local midnight (the day count changes), and at the exact expiry moment (`now + TrialTimeRemaining`) so a trial ending while the app is open locks within seconds.
Each read updates the core's cached state; the UI reads the cache as today (`useLicenceNotice` refetch on focus and every 5 minutes is fine). Do not poll the Store from the UI. Do not make a second authoritative clock: the local timers only decide when to ask the Store again.

### W3. State mapping (replace the refund bug)
Store the last known good state in `entitlement/notices.json` as a new optional field (`lastState`: `trial` or `purchased`; defaults keep old files valid because the record uses `#[serde(default)]`).

| Store says | Last known | Result |
|---|---|---|
| active, not trial | any | `Purchased` |
| active, trial | any | `TrialActive{days_left}` with `ceil` clamped 1..7 |
| not active | `trial` or none | `TrialEnded` |
| not active | `purchased` | `Revoked` (refund wording from file 02, section 6) |
| call failed or unreachable | `purchased` | stay `Purchased` (never lock a buyer for a network error) |
| call failed or unreachable | `trial` | keep `TrialActive` and recompute days from the last good remaining time minus monotonic time elapsed; never extend |
| call failed or unreachable | none | `NeedsOnlineCheck` (editing locked, closable screen, as today) |

Add `trial_ends_at` to the status if it is cheap (file 02, section 4).

### W4. Chip, reminders, lock (shared spec)
Implement file 02 sections 3, 4, 6, 7. The chip is mounted on Application Home and in the shell. Update `trialChip` wording and the tests.

### W5. Buy flow
`entitlement.buy` calls W1's `request_purchase`. After success: re-read the licence, invalidate queries (the UI already does `invalidateQueries()` after a store change), and let `notices::due` start the data note, tutorial, welcome sequence.

### W6. Packaging for the Store (MSIX)
Files: `packaging/msix/store.config.json`, `AppxManifest.template.xml`, `scripts/build-msix.mjs`, `.github/workflows/release.yml`.
- Fill what is known: `displayName` "Straft Studio", `privacyPolicyUrl` `https://straftstudio.com/legal/privacy/`, `supportUrl` `https://straftstudio.com/contact/`.
- Leave as placeholders (file 06): identity `name`, `publisher` (`CN=...`), `publisherDisplayName`. Keep the existing guard: any value matching `/REPLACE_/` blocks a `--store` build. Add the same guard for the other placeholders in file 06.
- The package version must always increase between submissions. Partner Center reserves the last (revision) part, so use `Major.Minor.Build.0` for Store packages; confirm in Partner Center when the app record exists, and make `build-msix.mjs` produce it.
- Never change the identity name or publisher after the first submission. An update with a different identity is a different app and will not update the old one.
- Capabilities: keep the minimum (`runFullTrust` for the desktop bridge, network client for the AI download and Store calls). Do not add broader capabilities.
- WebView2: a Store package cannot run the WebView2 bootstrapper. `ensure_webview2()` already explains and links to the download. Keep. Document that Windows 11 includes it.
- Release workflow: the Store channel needs no signing secret (Microsoft signs the MSIX). Keep `RELEASE_CHANNELS` default and the `MSIX_*` repository variables. Add a `msixbundle` step when ARM64 lands (W8).
- Run the Windows App Certification Kit on the built MSIX in CI or document it as a manual release step, and fix what it finds.

### W7. Testing against the real Store
There is no Store simulator for `StoreContext`. Do both:
1. Unit and integration tests with the test double for every row of W3 and every refresh trigger of W2.
2. A manual checklist in `docs/release/windows-store-test.md`: submit a package to Partner Center with limited audience (private or hidden, per Partner Center options at that time), install it from the Store on a clean PC, and walk through: trial start, chip and reminder wording, buy, refund (ask Microsoft support or use a test account), reinstall, update to the next version, uninstall and reinstall (data folder behaviour, file 05).

### W8. Windows ARM64 (the owner wants x64 and ARM64)
Real code today is x64 only: `AppxManifest.template.xml` has `ProcessorArchitecture="x64"`; `ai-assets.lock.json` lists Windows runtimes for `windows-x86_64` only (CPU and Vulkan); `build-msix.mjs` targets `windows-x86_64`; the release workflow builds x64.
- Native ARM64 needs: Rust target `aarch64-pc-windows-msvc`; an ARM64 llama.cpp engine in `ai-assets.lock.json` (CPU first; do not assume Vulkan); an ARM64 `AppxManifest` (or one manifest per architecture); a `.msixbundle` or two packages in one submission; a CI runner or cross toolchain; a real ARM64 Windows 11 PC for the smoke test.
- **Gate:** do not ship an ARM64 package that has not run on real ARM64 hardware. If hardware or the engine is not ready, ship x64 only. An x64 package installs on Windows 11 ARM PCs and runs under Windows' x64 emulation (slower AI, everything else works); confirm in Partner Center's package validation. Record the decision in the handoff.
- Do the x64 release path first. ARM64 is a follow-up item with its own acceptance test, not a blocker.

### W9. Remove the retired Mac-server leftovers on the Windows tree
`windows/services/licence-server` and the `.invalid` licence URLs in `src-tauri/src/platform/mac_inputs.rs` and `scripts/app-addresses.json` belong to the design that is gone (file 04, M1). Remove or retire them in this tree too and keep `app-addresses` and the governance tests passing.

### W10. Done when
- Real `StoreContext` source exists and is covered by the doubles; the manual Store checklist is written.
- The state refreshes by every trigger in W2; the refund shows the right words.
- Chip on Home and in projects; reminders on Home and over a project; updated tests.
- Packaging guard blocks placeholders; x64 MSIX builds with the real identity once supplied; ARM64 decision recorded.
- Update-safety items for Windows in file 05 are done and tested.
