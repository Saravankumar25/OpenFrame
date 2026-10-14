# Entitlement and trial

> Written 2026-10-06 from the code in `crates/openframe-entitlement/src/` (then `lib.rs`, `trial.rs`, `licence.rs`, `adapters.rs`).
> Decisions behind it: [ADR-0018](../adr/ADR-0018-distribution-licensing-commerce.md). Plan area: "Payment and free trial in all
> three channels" in [masterplan.md](../implementation/masterplan.md). State of the feature: [feature-status.md](../implementation/feature-status.md)
> (rows PLAT-009, PLAT-012 to PLAT-014 and PLAT-015). Open decisions: [owner-decisions.md](../implementation/owner-decisions.md).
> Updated 2026-10-07: the trial, purchase and first-run safety screens from the owner's mocks 182 to 198 (section 10).
> Updated 2026-10-07 again for the owner decisions of that day: ADR-0022 (the Mac App Store build locks the whole app after the
> trial or a refund) and ADR-0023 (the Microsoft Store owns its 7-day trial and locks the same way; the Windows installer is an
> internal build that is never sold; no screen names a price, a currency or a pricing model). Sections 2.1, 3 (Windows rules),
> 4, 5 and 6 describe the local trial record and the installer licence file, which the core no longer uses. They are marked
> SUPERSEDED and kept as history.
> Updated 2026-10-07 a third time: `trial.rs`, `combine`, `licence.rs`, `DirectInstallerAdapter` and the `issuer` feature are
> removed from the crate with their tests (clean as you go, ADR-0023). The internal builds are now the Windows installer and the
> Mac disk image, behind a team password on every launch (ADR-0024, section 11). An About card in Settings shows the version and
> "Internal build, not for sale." on the internal builds.

## 1. Purpose

OpenFrame Studio is a paid app with a 7-day free trial and no credit card to start, sold through two stores. The entitlement
crate is the one place in the Rust core that answers three questions for the rest of the app:

1. Is the trial running, and how many days are left?
2. Is the app unlocked (purchased)?
3. What may the user do right now?

The crate never talks to a store from its core. Each release file has one adapter behind the `ChannelAdapter` trait (section 7).
Both stores own the trial and decide the whole state themselves (`ChannelAdapter::channel_state`); OpenFrame keeps no trial timer
of its own (ADR-0023). The Windows installer is an internal build for the owner's team: its state is InternalBuild and nothing
is asked, shown or locked.

Rules that hold in every channel. They are written at the top of `lib.rs` and checked by the crate tests:

- **Unknown never blocks.** If a store's adapter cannot answer at all, the state is Unknown and the user can open, edit and export.
- **Nothing is ever deleted.** In either store build an ended trial or a refund locks opening, editing and exporting until a
  purchase or a restore (ADR-0022 for the Mac, ADR-0023 for the Microsoft Store). The projects stay on disk untouched and open
  again exactly as they were.
- **A failed check never locks anyone out.** NeedsOnlineCheck locks editing only; opening and exporting stay (ADR-0018
  decision 6). A user who bought or is in a running trial keeps a cached licence (StoreKit entitlements on the Mac, the Store's
  offline licence on Windows), so a failed check does not change their state.
- **No price anywhere.** The app never names a price, a currency or a pricing model. "Buy full version" hands off to the store's
  own purchase window, which shows the price (ADR-0023).
- **No project data in any check.**

## 2. The eight states and what each permits

`Permission` has three values: Open (read an existing project), Export (export or print an existing project) and Edit (change
content: edit, create, import, use the assistant to propose changes). `EntitlementState::permits(channel, permission)` is the
single function that decides; `EntitlementState::locks_the_app(channel)` says when the core refuses every project operation.

| State | Meaning | Store builds: Open, Export | Store builds: Edit | Internal build |
|---|---|---|---|---|
| TrialActive (with `daysLeft`, 1 to 7) | The store's trial is running. | Yes | Yes | n/a |
| TrialEnded | The store says the trial is over and nothing was bought. | **Locked (whole app)** | **Locked** | n/a |
| Purchased | The store confirms a purchase. | Yes | Yes | n/a |
| Revoked | The store reported a refund or a revoked purchase. | **Locked (whole app)** | **Locked** | n/a |
| Unknown | The adapter could not answer at all. | Yes | Yes | n/a |
| NeedsOnlineCheck | No confirmed licence: on the Mac a trial not confirmed online, on the Microsoft Store no answer and no cached licence. A build with no store connection wired also reports it, a gap in the build and not a user state. | Yes | **Locked** | n/a |
| TrialNotStarted | Mac only. No trial has been started from the App Store yet. | Yes | **Locked** | n/a |
| InternalBuild | The Windows installer, never sold (ADR-0023). | n/a | n/a | Everything allowed |

**The store lock is enforced in Rust.** `EntitlementService::check_operation` runs inside `Registry::dispatch`, the one path every
operation takes (the interface, the assistant's change sets and package imports). While `locks_the_app` holds, every operation is
refused with `permission.licence_required` ("Buy OpenFrame Studio or restore your purchase to open your projects again. Nothing
is deleted.") except `entitlement.*` and `ALLOWED_WHILE_LOCKED` (`app.info` and `appearance.get`, which the lock screen needs to
draw, and `team_gate.status`, the first question the window asks on every launch; a store package answers that no
team password is needed, so the lock screen shows; all three touch no project). Editing alone is not yet gated per operation in NeedsOnlineCheck and TrialNotStarted: that is
the open plan item "Enforce Permission::Edit in the Rust operation path" (section 8).

### 2.1 How the adapter answer and the trial record combine (SUPERSEDED by ADR-0023)

> SUPERSEDED 2026-10-07 by ADR-0023: no channel combines a purchase answer with the local trial record any more. `combine` and
> `trial.rs` were removed from the crate the same day. The table is history.

The adapter returns a `PurchaseStatus`: Purchased, NotPurchased, Revoked or Unknown. `combine` turns it into a state:

| Adapter answer | Trial record | Result |
|---|---|---|
| Purchased | any | Purchased |
| Revoked | any | Revoked |
| NotPurchased | present | the trial state: TrialActive, or TrialEnded after day 7 |
| NotPurchased | absent | Unknown |
| Unknown | running trial | TrialActive (the user still sees the days left) |
| Unknown | ended trial, or no record | Unknown, never TrialEnded, because the user may have paid |

Each state has a plain-words summary for the Settings screen and the trial banner (`EntitlementState::summary`). Every summary
ends with a full stop and has no em dash. A test checks both.

## 3. Trial rules

> SUPERSEDED 2026-10-07 by ADR-0023 for the Windows channels: the Microsoft Store owns its trial and the installer is an internal
> build with no trial. The table below is the old local-record rule, kept as history. The current rules are "Mac App Store
> channel" and "Microsoft Store channel" below.

| Rule | What happens |
|---|---|
| Length | Exactly 7 days (`TRIAL_DAYS`), the same in all three channels. |
| Start | At first launch. The trial record is created the first time `load_and_observe` runs and finds no record. |
| Credit card | None. The trial needs no payment detail and no network. |
| Counting | In whole 24-hour days from the start time, not calendar days. The days left are rounded up: 6.5 days left shows as 7, the last partial day shows as 1. At exactly 7 days the trial has ended. |
| Offline | The trial counts without a network, because the count uses only the local clock and the local record. A purchase also keeps working offline in the installer channel, because the licence file is checked with a public key and no network call (section 6). |
| Clock turned back | The latest time the record has seen wins (`lastSeenMs`, which never decreases). Turning the clock back gives no days back. A clock set earlier than the start time shows the full 7 days, because no time has passed yet. |
| Clock turned forward | Time moves on and the days count. A large jump forward ends the trial, and the user can only undo that by deleting the record. The code accepts this on purpose: a forward jump that was ignored would give an unlimited trial. |
| Record edited, or not a record | The record is replaced by one that has already ended, never by a fresh trial. The user keeps opening and exporting. A purchase still unlocks the app, because the purchase answer comes from the channel and not from this file. |
| Record cannot be read right now (locked, no permission) | This is an error, not a tampered file. The record is not overwritten, because it may be fine. |
| After a refund | The state is Revoked, which locks editing and keeps Open and Export. This happens only when a channel adapter reports a revocation. No adapter reports one yet (section 7). |
| Purchase during the trial | The purchase answer wins over the trial record at once. |

### Mac App Store channel (decided 2026-10-06)

The Mac App Store build does not use the rules above. The local trial record (`trial.json`) is not used on the Mac: nothing in
section 4 starts, counts or extends a Mac trial. The owner decided the Mac method in chat on 2026-10-06 ([ADR-0018](../adr/ADR-0018-distribution-licensing-commerce.md), decisions 3 and 6). Full design: [38-mac-app-store-trial.md](38-mac-app-store-trial.md).

| Rule | What happens |
|---|---|
| Products | Apple's non-subscription trial model. A zero-price Non-Consumable "7-day Trial" (`com.openframe.studio.trial7`) and a separate paid Non-Consumable "Lifetime Access" (`com.openframe.studio.lifetime`). No auto-renewing subscription. Family Sharing is off. |
| Before the trial | The app shows the 7-day length and offers the trial, the purchase and restore. It names no price (ADR-0023); Apple's sheet shows it. The trial begins only after the verified free StoreKit transaction succeeds. |
| Source of truth | Only cryptographically verified StoreKit 2 transactions are authoritative. The installation date, the system clock, the time zone, UserDefaults, SQLite, files, Keychain and frontend state are never the source of truth. Local state is a cache. |
| Trial state | Built from the verified trial transaction: product id, Apple transaction id and the Apple-recorded purchase date. The trial starts at that transaction's `purchaseDate` (or its own earlier `originalPurchaseDate`), never at `AppTransaction.originalPurchaseDate`. Apple's `appTransactionID` only links the Apple Account to the app and is optional, because Apple provides it on macOS 15.4 and later only; the server requires every `appTransactionID` present in one request to match. |
| Expiry | Server-authoritative. A small licensing server validates the Apple-signed transaction and computes `trialEndsAt` as the Apple purchase time plus 168 hours in server UTC. Apple private keys and server secrets never live in the app. |
| During the trial | The app needs a successful online validation at launch. It receives the remaining seconds and a lease (6 hours by default, never more than the seconds left) and counts down with monotonic elapsed time in the session, not the wall clock. A validation is good for `min(lease, remaining)` of monotonic time. |
| Revalidation | The shell calls `revalidate` at launch, after the Mac wakes or the window returns after a long idle, when StoreKit reports a change (`Transaction.updates` or an account change), and when the lease runs out. If the server cannot be reached, the current countdown continues until its lease ends, then editing locks (NeedsOnlineCheck, "check again") until a check succeeds. A successful check takes the server's seconds left (Apple purchase date plus 168 hours minus server time) and never adds to them, so no sequence of leases, sleeps and relaunches gives more than 168 hours of use. A revocation StoreKit reports locks at once. |
| Validation fails | The app fails safe with a connectivity message: editing locks, opening and exporting stay. It never restarts or extends the trial. |
| After expiry | The whole app locks: opening, editing and exporting wait for a purchase or a restore (owner decision 2026-10-07, ADR-0022). The lock screen offers only **Buy full version** and **Restore purchase** and cannot be closed; the core refuses every project operation underneath it. Nothing is deleted. |
| Lifetime | Comes only from the verified paid non-consumable in StoreKit `currentEntitlements`. It is restored automatically after a reinstall or on a new Mac with the same Apple Account, and it works offline. A revalidation never takes it away because the server cannot be reached. A refund or revocation that StoreKit reports is honoured. |
| Enforcement | Central in the Rust core. UI paywalls are presentation only. Sandbox or TestFlight and production stay separated. |
| Offline | The trial needs the launch check and a new check whenever the lease runs out. This is the one exception to the offline rule (ADR-0018, decisions 4 and 6). |

Threats the Mac tests cover: clock rollback or forward, time zone changes, reinstall, deleted app data or Keychain, backup or VM
rollback, copied state, network blocking, a repeated trial purchase, another Mac with the same Apple Account, transaction or JWS
tampering, App Store sign-out, the app kept open past expiry, lease expiry, revalidation with the server unreachable, an
interrupted or deferred purchase, restore, refund or revocation, and corrupt or unverified transactions. What the design cannot
prevent (a different Apple Account gets a new trial; no device fingerprinting) is in section 9 of
[38-mac-app-store-trial.md](38-mac-app-store-trial.md).

### Microsoft Store channel (decided 2026-10-07, ADR-0023)

| Rule | What happens |
|---|---|
| Trial | Owned by the Store (Partner Center trial). OpenFrame keeps no timer and no `trial.json` for this channel. |
| Licence | `StoreContext.GetAppLicenseAsync`: IsActive, IsTrial and TrialTimeRemaining, read at launch, on "Restore purchase" and when the Store raises `OfflineLicensesChanged`. The call sits behind `adapters::StoreLicenceSource`, supplied by the Windows shell as `AppConfig::microsoft_store`. |
| Mapping (`store_licence_state`) | Active and not a trial: Purchased. Active trial: TrialActive with TrialTimeRemaining in whole days, rounded up, 1 to 7. Not active: TrialEnded. No answer and no cached licence: NeedsOnlineCheck. |
| After the trial | The whole app locks, as on the Mac: only **Buy full version** and **Restore purchase**, not closable, enforced in Rust. Nothing is deleted. |
| Buy | `StoreContext.RequestPurchaseAsync` with the app window's handle, so the Store's own purchase dialog opens over the app and shows the price. A closed dialog changes nothing and shows no failure; a network or server error comes back in plain words. |
| Offline | The Store answers from its cached offline licence, so a buyer or a running trial keeps working. No cached licence and no Store: NeedsOnlineCheck (editing locks, opening and exporting stay). |
| Not built yet | The real Store calls need the Partner Center product identity (owner task). Until the shell supplies them the channel reports NeedsOnlineCheck and Buy says "The Microsoft Store connection is not available in this build." The mapping, the lock and the purchase path are tested with a fake source (section 9). |

### The Windows installer (decided 2026-10-07, ADR-0023)

An internal build for the owner's team, never sold: no trial, no payment, no Buy, no reminder, no lock. The state is
InternalBuild, Settings > Licence and Settings > About say "Internal build, not for sale." and nothing can be restored. The
internal Mac disk image (ADR-0024) is the same channel (`Channel::DirectInstaller`, "Internal build" on screen): a Mac build made
without `OPENFRAME_DISTRIBUTION=store` is an internal build, a Mac build with it is the Mac App Store build. Both internal builds
ask the team password on every launch (section 11). The installer licence file code was removed on 2026-10-07.

### Not decided (owner decisions)

These items are not rules. They are open in [owner-decisions.md](../implementation/owner-decisions.md) and nothing in this crate
or this document decides them:

- **Price and currency.** Set in the stores; the app never shows them (ADR-0023).
- **Refund rules and refund text,** per store.

## 4. The trial record file (SUPERSEDED by ADR-0023)

> SUPERSEDED 2026-10-07: no channel reads or writes this file any more, and `trial.rs` was removed the same day. History only.

Path: `<app data>/entitlement/trial.json`, where `<app data>` is the folder from `openframe_platform::dirs::app_data_dir`
(see [37-platform-boundary.md](37-platform-boundary.md)). The file is written with the shared atomic replace
(`openframe_platform::atomic::replace_bytes`), so a crash never leaves a half-written record.

| Field | Meaning |
|---|---|
| `startedAtMs` | Unix milliseconds of the first launch in this channel. |
| `lastSeenMs` | The latest clock reading this record has seen. It never decreases. |
| `channel` | `microsoftStore`, `macAppStore` or `directInstaller`. |
| `check` | Hex SHA-256 over a fixed salt, `startedAtMs` and `lastSeenMs` as 8-byte little-endian numbers, and the channel name. |

How a read ends (`TrialStore::read`). At most 4096 bytes plus one are read, so a huge file is never loaded whole:

- **Missing:** first launch. A new record starts.
- **Io:** the file exists but could not be read now (locked, no permission). The record may be fine.
- **Unparseable:** the file is not JSON, has the wrong shape or is larger than 4096 bytes.
- **Tampered:** the file parses but `check` does not match, or a time is not believable (before 2025-01-01 or after about the
  year 2200), or `lastSeenMs` is earlier than `startedAtMs`.

`load_and_observe` reads the record, records the current clock reading and saves it back. It returns a `LoadedTrial` with the
record and a `persisted` flag:

- First launch starts a new record.
- Unparseable and Tampered files get a record that started 7 days before now, so the trial has ended. One warning is logged. The
  file is rewritten clean.
- An Io error is returned to the caller and the file is left alone.
- If the record cannot be written (for example the folder is blocked), the call still returns the record with `persisted: false`.
  The crate does not pretend it was saved. The caller decides how to treat a trial that cannot be remembered.

## 5. Honest limits of the trial record (SUPERSEDED by ADR-0023)

> SUPERSEDED 2026-10-07 with section 4.

- **The checksum is not a secret.** The salt is a constant in the source. The checksum detects accidental corruption and plain
  edits. It gives no protection against a person who recomputes it.
- **Deleting the file, or using a new user account, restarts the local count.** The Store channels do not need this file for the
  purchase answer, but the 7-day count itself lives here. Closing this gap is the open reinstall decision in section 3.
- **The record's `channel` field is not compared with the build's channel on read.** `load_and_observe` uses the channel only when
  it starts a new record.
- **A clock jumped far forward ends the trial** (section 3).

## 6. The licence file (installer channel, SUPERSEDED by ADR-0023)

> SUPERSEDED 2026-10-07: the installer is an internal build that is never sold. `DirectInstallerAdapter`, `licence.rs` and
> the `issuer` feature were removed the same day. History only.

Path: `<app data>/entitlement/licence.json`. The file is JSON with two text fields, both base64url without padding:

| Field | Meaning |
|---|---|
| `payload` | The claims as JSON bytes, kept exactly as they were signed. |
| `signature` | The 64-byte Ed25519 signature over the domain prefix `openframe-licence-v1` followed by a zero byte and then the payload bytes. |

The claims inside the payload:

| Claim | Meaning |
|---|---|
| `schema` | Claims format version. Must be 1. An unknown value is refused. |
| `product` | Must be `openframe-studio`. Any other value is refused. |
| `licenseeHash` | Hex SHA-256 of the buyer's e-mail address, trimmed and lower-cased (ASCII only, so both sides agree). Support can find a licence from it. The address itself is never stored in the file or the app. |
| `issuedAtMs` | Unix milliseconds when the licence was issued. Informational. |
| `licenceId` | A unique id, kept for refunds and a future signed denylist. |

Checks, in order (`DirectInstallerAdapter::check_file`, `licence::parse` and `licence::verify`):

1. At most 8 KiB plus one byte of the file is read. A larger file, a file that is not valid text, or text that is not a licence
   file is refused as unreadable. The decoded payload is also capped at 8 KiB.
2. The signature decodes and verifies against the embedded public key, with strict verification (non-canonical signatures are
   refused). The domain prefix stops a signature made for another OpenFrame file from being replayed as a licence.
3. Only after that is the payload parsed. The schema and the product must match.

The signature covers the stored payload bytes and not a re-serialisation, so a later change to field order or to the Rust types
never invalidates a licence already issued. There is no clock check: a wrong clock never refuses a genuine licence. The file holds
no screenplay or project data. The app holds only the public key. The issuing function exists only behind the `issuer` feature
and in tests, so the app binary does not contain it, and the private key never ships.

Limits that follow from the code:

- A licence has no expiry and is not bound to a computer. A valid file copied to another computer counts as a purchase there. The
  tests cover a licence signed by another key and an edited payload, not a valid licence copied between machines. Expiry and
  machine binding are owner decisions.
- Nothing yet checks `licenceId` against a revocation list, so a refund cannot make this adapter answer Revoked. The `schema` and
  `licenceId` claims leave room for a signed denylist without a new format. The denylist is an owner decision.
- The production public key does not exist yet. Until the owner generates the offline signing key, the installer adapter has no key
  and answers Unknown.

## 7. The adapters and their state

| Channel | Adapter | State today |
|---|---|---|
| Microsoft Store (`.msix`) | `MicrosoftStoreAdapter` over `StoreLicenceSource` | **Mapping, lock and purchase path built and tested with a fake source; not connected to the Store.** `channel_state` and `restore_channel_state` read the Store licence and map it with `store_licence_state` (section 3). `buy_full_version` runs the Store's purchase dialog. Without a source (`MicrosoftStoreAdapter::unavailable`, every build today) the state is NeedsOnlineCheck with the limitation "The Microsoft Store connection is not available in this build." and Buy returns that message. The real `GetAppLicenseAsync`, `RequestPurchaseAsync` and `OfflineLicensesChanged` calls in the Windows shell need the Partner Center product identity (owner task). |
| Mac App Store (`.pkg`) | `mac::MacAppStoreAdapter` | **Built and tested with fakes; not connected to Apple.** The policy and the adapter are in `crates/openframe-entitlement/src/mac/`, tested against a fake StoreKit bridge and a fake licensing server; design and test list in [38-mac-app-store-trial.md](38-mac-app-store-trial.md). Without the shell's `MacInputs` (every build today) `for_channel` builds it over `UnavailableBridge` and `UnavailableServer`: NeedsOnlineCheck with "The App Store connection is not available in this build.", nothing granted, nothing cached. That is a gap in the build, not a user state. |
| Internal builds (`.exe`, `.dmg`) | none | **Internal build** (ADR-0023, ADR-0024). `for_channel` returns no adapter and the core sets InternalBuild without asking anything. The team password gate is section 11. |

`for_channel` builds the adapter from the channel, the app data folder, optional `MacInputs` and an optional
`StoreLicenceSource`, and returns `None` for the internal builds. Every adapter implements `channel`, `purchase_status`,
`limitation`, `channel_state`, `restore_channel_state` and `buy_full_version`; only the Mac adapter implements `start_trial`.

`Channel` and `EntitlementState` are exported to TypeScript through ts-rs. `Permission` and `PurchaseStatus` stay in Rust.

## 8. What still has to be built

The crate is wired into `AppCore` through `EntitlementService` (`crates/openframe-application/src/modules/entitlement.rs`). It
builds the adapter for the configured channel, computes the state at start-up, on "Restore purchase" and when the shell asks again
(`refresh` for the Microsoft Store's `OfflineLicensesChanged`, `revalidate` on the Mac), serves it through `entitlement.status`
and the screen operations (section 10), and refuses every operation during the store lock (section 2). To finish the plan area:

- Enforce `Permission::Edit` per operation in the Rust operation path for NeedsOnlineCheck and TrialNotStarted (the store lock is
  already enforced; editing alone is not).
- Microsoft Store: the real `StoreLicenceSource` in the Windows shell (`GetAppLicenseAsync`, `RequestPurchaseAsync` with the window
  handle) and the `OfflineLicensesChanged` subscription that calls `refresh`, after the owner reserves the product in Partner
  Center; the Partner Center trial set to 7 days.
- Mac App Store: the real StoreKit 2 bridge in Swift, the HTTPS licensing server client and the system nonce source, handed to the
  core as `MacInputs`; the shell's calls to `revalidate`; the licensing server's deployment and its response keys; a check against
  Apple's current review rules before submission.
- Tests that are still missing: an end-to-end run in each store against the real store.

## 9. Tests in the crate

All 76 pass on Windows 11 with `cargo test -p openframe-entitlement` (run 2026-10-07 after the clean-up): the tests below, 11 in
`notices.rs` (section 10), 8 in `team_gate` (section 11) and the Mac module's, listed in
[38-mac-app-store-trial.md](38-mac-app-store-trial.md). The app's behaviour is tested in
`crates/openframe-application/tests/entitlement.rs`, `tests/trial_first_run.rs` and `tests/team_gate.rs`.

| File | Test |
|---|---|
| `lib.rs` | `the_internal_installer_build_never_locks_anything` |
| `lib.rs` | `both_stores_lock_everything_only_after_the_trial_ended_or_a_refund` |
| `lib.rs` | `only_ended_revoked_needs_online_check_and_not_started_lock_editing` |
| `lib.rs` | `summaries_are_plain_and_have_no_em_dash` |
| `adapters.rs` | `store_licence_answers_map_to_every_state` |
| `adapters.rs` | `microsoft_store_reads_the_store_licence_and_buys_through_the_store_dialog` |
| `adapters.rs` | `microsoft_store_without_the_store_calls_needs_a_check_and_cannot_sell` |
| `adapters.rs` | `mac_channel_without_inputs_needs_an_online_check_and_grants_nothing` |
| `adapters.rs` | `the_internal_builds_have_no_channel_to_ask` |

App-level tests for the lock and the channels (`crates/openframe-application/tests/entitlement.rs`):
`plat_009_the_internal_installer_build_is_never_sold_and_never_locks`,
`plat_009_the_microsoft_store_trial_comes_from_the_store_licence`,
`plat_009_the_microsoft_store_trial_over_locks_every_project_operation_until_bought`,
`plat_009_a_microsoft_store_check_that_cannot_answer_keeps_open_and_export`,
`plat_009_a_microsoft_store_build_without_the_store_calls_needs_a_check_and_cannot_sell`,
`plat_014_a_mac_refund_locks_open_edit_and_export_in_rust_and_a_purchase_unlocks`,
`plat_014_a_mac_trial_that_ended_locks_open_edit_and_export_and_a_restore_unlocks`,
`plat_014_mac_channel_without_the_store_bridge_needs_an_online_check_and_keeps_open_and_export`,
`plat_009_the_operations_the_lock_screen_needs_are_registered_and_touch_no_project`.

## 10. Trial, purchase and first-run safety screens

Written 2026-10-07 from the owner's mocks `Trial_Licensing_Mocks/182-lic-flow.html` to `198-lic-welcome.html` and their change
note, section 4 "Rules to implement", and updated the same day for ADR-0022 and ADR-0023. The Rust core decides which screen is
due; the interface only shows it. Code: `crates/openframe-entitlement/src/notices.rs`,
`crates/openframe-application/src/modules/entitlement.rs`, `apps/desktop/src/features/licensing/`. Feature rows PLAT-009
(screens in the app) and PLAT-015 (first-run safety screens).

### 10.1 The screens

| Mock | Screen | When | Can it be closed? |
|---|---|---|---|
| 183, 188 | Corner chip "Trial: N days left" and **Buy full version** in the top bar | Either store, every project screen while the state is TrialActive. Amber with the words "Your trial ends soon." for screen readers in the last 3 days; never colour alone. Gone after the purchase. Never on the internal build. After the check screen was closed with **Not now**, the top bar shows **Check purchase** to open it again. | Not a dialog |
| 184, 185, 189 | Trial reminder: day 5 "Your free trial ends in 3 days", day 6 "ends in 2 days", day 7 "ends today"; "Buy the full version to keep using OpenFrame Studio."; **Buy full version** and **Continue trial** | Either store, TrialActive with 3, 2 or 1 days left, a project open, at most once per calendar day | Yes (Continue trial or Escape) |
| 186, 190 | The lock: "Your free trial has ended" (or "Your purchase is no longer active" after a refund, naming the store) with only **Buy full version** and **Restore purchase**; "Your projects are safe on this Mac (this computer). Nothing is deleted." | Either store, TrialEnded or Revoked (ADR-0022, ADR-0023) | No: no close button, no Escape, no click outside, app shortcuts held back, focus trapped. The core refuses every project operation underneath it |
| 187 | Mac first launch: **Start free trial**, **Buy full version**, **Restore purchase** | Mac, TrialNotStarted | No |
| 191 | The store's own purchase window: Apple's payment sheet, or the Microsoft Store purchase dialog | Through `ChannelAdapter::buy_full_version`. OpenFrame never draws it; it shows the price. | The store's |
| 192 | "Checking your purchase" with **Not now**, **Buy full version** and **Try again**, naming the store; "Your projects stay open and can be exported. Nothing is deleted." | Either store, NeedsOnlineCheck | Yes (Not now, Escape or Close), ADR-0018 decision 6. Stays closed for the session while it is due |
| 193 | "Your data, your responsibility": six points, the real root folder, **Show in File Explorer** or **Show in Finder**, a tick box, **OK, I understand** | Purchased and not yet acknowledged | No. The box enables only after the list is scrolled to its end; OK only after the box is ticked |
| 194 to 197 | File safety tutorial, 4 steps; **Finish** needs all three boxes on step 4. Step 3 says how to install per channel: the App Store with the same Apple Account, the Microsoft Store with the same Microsoft account, or (internal build) just "Install OpenFrame Studio on the new laptop." | Purchased, note acknowledged, tutorial not finished | No |
| 198 | "Welcome to the OpenFrame Studio family" with the eight community accounts and **OK** | Purchased, tutorial finished, welcome not shown | Yes |

**No price anywhere (ADR-0023).** No screen names a price, a currency, or a pricing model ("pay once", "subscription",
"lifetime"). The button says "Buy full version" with no amount. The Vitest test
`no price, currency or pricing model on any licensing screen` renders the reminder, the lock (both stores, ended and refunded),
the check screen, the first launch, every tutorial step, the data note, the welcome, the chip and Settings > Licence and fails on
a currency symbol, a number with a currency code, or those words. Trial users never see the note, the tutorial or the welcome.
The internal installer build shows none of these screens.

While a mandatory screen is open the app's own Ctrl, Alt and Cmd shortcuts are held back; copy, cut, paste and select all go
through, so the root folder path can be copied.

### 10.2 The rule (`notices::due`)

1. **The internal installer build:** nothing, ever.
2. **Either store first:** TrialEnded and Revoked give the lock, NeedsOnlineCheck gives the purchase check, and on the Mac
   TrialNotStarted gives the first-launch screen. These win over everything else, so a refund after the purchase locks again.
3. **Purchased:** the data note, then the tutorial, then the welcome, each once.
4. **TrialActive with 3 days or fewer left** and a valid local day from the interface: a reminder, unless the stored reminder day
   is the same day or later. A later stored day means the clock was turned back, so it counts as shown.
5. Everything else: nothing.

Trial days come from `days_left`: day = 8 minus days left, so days 5, 6 and 7 are 3, 2 and 1 days left. The days come from the
store (TrialTimeRemaining on Windows, the Mac adapter's verified countdown on the Mac); a clock turned back gives no days back.

### 10.3 The notice record

Path: `<app data>/entitlement/notices.json`, written with the shared atomic replace.

| Field | Meaning |
|---|---|
| `reminderShownOn` | The local calendar day (`YYYY-MM-DD`) the last reminder was shown. |
| `dataNoteAcknowledged` | The data note was read to the end, ticked and confirmed. |
| `tutorialCompleted` | The tutorial was finished with all three boxes ticked. |
| `welcomeShown` | The welcome popup was closed. |

The record only decides whether a screen shows again. It grants and removes nothing, so it has no checksum: an edited file can
only hide a reminder or a first-run screen, never unlock the app. A missing file is a first run. A file that is locked, larger
than 4096 bytes, not JSON or the wrong shape is read as an empty record (the screens may show again) and never stops startup.
Unknown fields are ignored. A stored day that is not a real date is ignored. The service keeps the record in memory too: when the
file cannot be written (folder blocked, disk full) the change still holds for the session, so the user is never trapped on a
mandatory screen, and the screen may show again at the next launch. A reminder day that cannot be saved is reported to the
diagnostics only (`reportQuietly`), never on screen.

The local calendar day comes from the interface (`today`, `YYYY-MM-DD`), because "once per calendar day" follows the user's own
midnight. `CalendarDay::parse` accepts only real dates from 2000 to 9999; anything else shows no reminder and cannot be recorded.

### 10.4 Operations

All four are application level, hidden from the assistant (`HIDDEN_REASON`), and the three commands refuse any actor that is not
the person at this computer (`permission.denied` for the assistant and for packages). They keep working during the store lock.

| Operation | Does |
|---|---|
| `entitlement.notice` (query, `{ today? }`) | The status, the screen due now and `dataRoot`, the one folder to back up (`AppConfig::data_root`: the folder holding both the projects folder and the Global Idea Vault, `Documents/OpenFrame` by default). |
| `entitlement.record_notice` (command, `{ notice, today? }`) | Remembers a reminder day or a finished screen. Out of order is refused with `conflict.state`: a reminder only during the trial, the note only after a purchase, the tutorial only after the note, the welcome only after the tutorial. |
| `entitlement.buy` (command) | The store's own purchase window through `ChannelAdapter::buy_full_version`: Apple's payment sheet, or the Microsoft Store purchase dialog. The route is `store`; a failure comes back in plain words, a closed window changes nothing. Without a store connection the route is `unavailable` with "The App Store (Microsoft Store) connection is not available in this build." The internal build answers "This is an internal build and is not for sale." (the interface shows no Buy there). After any answer the interface asks every query again, so a lifted lock shows at once. |
| `entitlement.start_trial` (command) | Mac only: the zero-price trial product through `ChannelAdapter::start_trial`. The Microsoft Store starts its trial itself. |

"Show in File Explorer" and "Show in Finder" call the shell command `of_reveal_path` with the fixed kind `dataRoot`; the path never
comes from the webview. The welcome links open only through `of_open_url` and only for fixed https addresses on the networks'
own sites (`isFixedSocialUrl`). The owner has not sent the real handles yet, so every account has no address and none can be
opened; the tiles show the placeholder handle `@openframestudio`.

### 10.5 Hand-set state for end-to-end tests

`entitlement.debug_set { channel, state, resetNotices }` reports a chosen channel and state until the app closes. It is
registered only when `AppConfig::entitlement_debug_controls` is true, which the desktop shell sets only in a debug build running on
an isolated test data root (`entitlement_debug_controls(release, isolated)`). A release build never registers it (test
`release_builds_never_expose_the_hand_set_licence_state`), and the product catalog does not list it. A state set by hand has no
store behind it: Buy then says the store connection is not available. A hand-set lock is enforced like a real one.

### 10.6 What is verified and what is not

- The rule, the lock and the channel mapping: Rust tests in `notices.rs`, `adapters.rs`, `lib.rs`, `tests/entitlement.rs` and
  `tests/trial_first_run.rs` (section 9), with a fake Microsoft Store source and a fake StoreKit bridge.
- The screens: Vitest in `apps/desktop/src/features/licensing/licensing.test.tsx` and `workspaces/settings/Licence.test.tsx`.
- End to end: `tests/e2e/specs/15-trial-first-run.test.mjs` drives the internal build, the store states set by hand, the lock in
  both stores and the closable check screen. It was updated for ADR-0022 and ADR-0023 but not run in this change.
- Never run against a real store: the Microsoft Store calls need the Partner Center identity; the Mac screens were never run on a
  Mac or against the App Store.

### 10.7 Conflicts with earlier text, for the owner

- **The store lock replaces "open and export always work" for TrialEnded and Revoked in the stores.** Owner decision
  2026-10-07, ADR-0022 (Mac) and ADR-0023 (Microsoft Store), supersede ADR-0018 decision 5 for those states. NeedsOnlineCheck
  keeps opening and exporting (ADR-0018 decision 6, unchanged). The earlier interim change that made the Mac screens closable is
  withdrawn.
- **Buy on the installer.** The earlier placeholder ("Buying is not available in this build yet") is withdrawn: the installer is
  an internal build with no Buy at all (ADR-0023).
- **No price on any screen.** Section 10, step 5, of [38-mac-app-store-trial.md](38-mac-app-store-trial.md) quotes Apple's
  guideline 3.1.1 as asking the app to say, before a zero-price trial starts, what the user must pay to continue. ADR-0023 says
  the app never names a price. Check the guideline before submission; the owner decides.
- **Step 1 of the tutorial (mock 194) has a "Change folder" button.** There is no operation that moves the projects folder, so the
  button is not built.
- **About screen.** ADR-0023 asks for "Internal build, not for sale" in About and in Settings > Licence. There is no About screen
  yet; Settings > Licence shows it.

## 11. Team password gate of the internal builds (ADR-0024)

The internal Windows installer and the internal Mac disk image ask the team password on every launch, before anything opens.
Store packages carry neither the check nor a hash.

**Build.** `apps/desktop/src-tauri/build.rs` reads `OPENFRAME_TEAM_PASSWORD` on the build machine for a `direct` build (the
default `OPENFRAME_DISTRIBUTION`), hashes it with Argon2id (64 MiB, 3 passes, 1 lane, 16 random bytes of salt from the system's
secure random generator) and writes only the PHC string to `OUT_DIR/team_gate_hash.txt`, which the shell embeds under
`cfg(openframe_team_gate)` and hands to the core as `AppConfig::team_password_hash`. The rules are one shared file,
`crates/openframe-entitlement/src/team_gate/build_hash.rs`, which the build script includes by path and the crate compiles, so
the build and the check cannot disagree:

| Build | Result |
|---|---|
| `store` (either store) | No gate and no hash, whatever the variable says. The variable is never read. |
| `direct`, release profile, variable missing or empty | The build fails and names the variable, never a value. |
| `direct`, password shorter than 12 characters | The build fails. |
| `direct`, password set | Gate and hash embedded. |
| `direct`, debug profile, variable missing | No gate (development and the end-to-end suite; never shipped). |

While the variable is set the build script reruns on every build instead of asking Cargo to watch the variable, because Cargo
keeps the value of a watched variable in the target folder to compare later. The release workflow passes the
`OPENFRAME_TEAM_PASSWORD` secret to the internal build step only. Changing the password means changing the secret and building
again.

**Check.** `team_gate::TeamGate` in the Rust core. `verify_team_password` accepts only an Argon2id hash, refuses passwords over
1024 bytes without hashing them, recomputes the hash with the stored salt and costs and compares the outputs in constant time
(the `phc` crate's `Output` equality). While the gate is shut, `EntitlementService::check_operation` (inside
`Registry::dispatch`) refuses every operation with `permission.team_password_required` except `team_gate.status`,
`team_gate.unlock` and `appearance.get`, so no project, vault or setting opens, and the app start clean-up of the Global Idea
Vault waits until the gate opens. The shell's own commands follow the same rule: `of_open_asset` and `of_reveal_path` (even
for the data folder and the logs) are refused before the password. `team_gate.unlock` takes the password from the person at the computer only (never the
assistant or a package); its argument's `Debug` output never shows the password, and nothing logs it.

**Wrong tries.** After 5 wrong passwords in a row the next try waits 1 minute; each further run of 5 doubles the wait: 2, 4, 8,
16, then 30 minutes from then on. During a wait a try is refused without being checked and does not count. A right password
resets the count. The count lives in `<app data>/entitlement/team-gate.json` (numbers only) and survives a restart. It is saved
before the password is checked, so closing or killing the app during the check cannot undo a wrong try. A missing file means no
wrong tries; a damaged, oversized or unreadable file starts at the first 1-minute wait, never at zero. A clock turned back to
before a wait started restarts that wait from the earlier time (and saves it), so the wait still ends after its own length of real
time instead of when the clock catches up. A clock turned forward ends a wait early; ADR-0024 accepts that the gate slows casual
use and does not stop a person who holds the file and guesses the hash offline.

**Screen.** `apps/desktop/src/features/team-gate/TeamGate.tsx` wraps the whole app: a dialog that cannot be closed, with a
password field, the core's plain-words answer, and a countdown while a wait runs. Store builds pass straight through.

**Tests.** `crates/openframe-entitlement/src/team_gate/tests.rs` (9, test-only password and small costs, plus one hash with the
build costs): right and wrong, the waits 1, 2, 4, 8, 16, 30, 30, 30 minutes, restart keeps the count and the wait, a try counted
before the check, a damaged count file, a clock turned back (seen by the screen and seen by a try), the build hash shape, and the build rules (store build without a
gate, missing variable, short password). `crates/openframe-application/tests/team_gate.rs` (4): nothing opens before the
password, the waits and a restart through the core, a store build without a gate, and only the person at the computer can unlock.
`apps/desktop/src-tauri/src/lib.rs`: a store build embeds no hash. Vitest: `TeamGate.test.tsx` (4) and `About.test.tsx` (2). Not
run: a signed internal build with a real secret, and the end-to-end suite (it runs debug builds without the variable).
