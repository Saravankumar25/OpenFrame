# Mac App Store trial and purchase

> Written 2026-10-06 on a Windows machine with no Mac, no Xcode and no Apple developer account, and updated the same day with the
> owner's corrections (trial origin, server checks, one wire format, lease and revalidation, threat boundary). The Swift bridge has
> never been compiled, and nothing here has run against Apple's sandbox. Section 14 separates what is proven on this machine from
> what is EVIDENCE PENDING.
> Decision behind it: the owner decision of 2026-10-06 "How the Mac App Store build unlocks after the trial" in
> [owner-decisions.md](../implementation/owner-decisions.md), under [ADR-0018](../adr/ADR-0018-distribution-licensing-commerce.md).
> Shared entitlement rules: [36-entitlement-and-trial.md](36-entitlement-and-trial.md). Platform code rules:
> [37-platform-boundary.md](37-platform-boundary.md).
> Updated 2026-10-07 for the owner decisions of that day: after the trial or a refund the whole app locks until a purchase or a
> restore (owner decision 2026-10-07, ADR-0022, which supersedes ADR-0018 decision 5 for TrialEnded and Revoked on the Mac), and
> the app never names a price, a currency or a pricing model (ADR-0023). A failed online check still keeps opening and exporting
> (ADR-0018 decision 6, unchanged).

## 1. Decision and status

The Mac App Store build is a free download. Two in-app purchases control what the user may do:

| Product | Product id | Type | Price | Family Sharing |
|---|---|---|---|---|
| 7-day Trial | `com.openframe.studio.trial7` | Non-Consumable | Free (zero price) | Off |
| Lifetime Access | `com.openframe.studio.lifetime` | Non-Consumable | Set by the owner (open) | Off |

There is no subscription. The rules from the owner decision and its corrections:

- Only StoreKit 2 transactions that StoreKit reports as `.verified` count.
- The trial starts at the Apple `purchaseDate` of the verified zero-price "7-day Trial" transaction and ends 168 hours later. The
  licence server computes this with its own UTC clock. `AppTransaction.originalPurchaseDate` (the day the app was first
  downloaded) is never used. The Mac's clock is never used.
- Apple's `appTransactionID` links an Apple Account to this app. It is optional: Apple provides it on macOS 15.4 and later only.
- During the trial the app needs a successful online check at launch, after the Mac wakes, when StoreKit reports a change, and
  whenever the lease of the last check runs out (6 hours of use by default). Without one, editing locks; opening and exporting
  stay (ADR-0018 decision 6).
- When the trial has ended without a purchase, or a purchase is refunded or revoked, the whole app locks: opening, editing and
  exporting wait for a purchase or a restore (owner decision 2026-10-07, ADR-0022). Nothing is deleted.
- Lifetime Access comes from verified `Transaction.currentEntitlements` only, works offline, restores on its own on a new Mac or
  after a reinstall, and a refund (revocation) is honoured.
- Apple's private keys and OpenFrame's signing key are never in the app.
- Sandbox and production are separate: two server deployments with two signing keys.

Status of each part:

| Part | Where | State |
|---|---|---|
| Swift StoreKit 2 bridge (C interface) | `apps/desktop/src-tauri/macos/StoreKitBridge/` | Written, never compiled |
| Rust wrapper of the bridge | `apps/desktop/src-tauri/src/platform/` | Owned by the desktop shell work; not covered by this document's tests |
| Mac App Store adapter, policy and wire contract | `crates/openframe-entitlement/src/mac/` | Built; 44 tests with a fake StoreKit bridge and a fake server pass on Windows (2026-10-06) |
| Licence server | `services/licence-server/` | Built; 33 tests pass on Windows (`cargo test`, 2026-10-06); never deployed |
| Shared wire fixture | `services/licence-server/tests/fixtures/mac-validate-v1.json` | Written by the server's test key; replayed byte for byte by the server and verified by the adapter |
| Trial sheet, Settings, Check again and Restore Purchases buttons | app | Not built |

## 2. Products and rules in detail

| Situation | Answer |
|---|---|
| No trial product bought yet | TrialNotStarted. The app shows the trial sheet (section 10). Opening and exporting work; editing waits until the user starts the trial. |
| Trial bought, check confirmed, lease running, less than 168 hours since Apple's purchase date | TrialActive with the seconds left from the server, counted down on the monotonic session clock. |
| Trial bought, 168 hours or more since Apple's purchase date | TrialEnded: **the whole app locks** (owner decision 2026-10-07, ADR-0022). The lock screen offers only **Buy full version** (Apple's payment sheet) and **Restore purchase** and cannot be closed; the Rust core refuses every project operation, so opening, editing and exporting all wait for a purchase or a restore. Nothing is deleted. |
| Trial running and the lease of the last check has run out | NeedsOnlineCheck with the "check again" message. Editing locks until a new check succeeds. |
| Trial bought and no confirmed check in this session (offline launch, server down, an answer that does not verify) | NeedsOnlineCheck. Editing locks; the trial is neither restarted nor extended. |
| Lifetime Access verified and not revoked | Purchased. Works offline, because StoreKit verifies its cached transactions on the Mac. A failed server check never takes it away. |
| Lifetime Access or the trial revoked (refund) | Revoked: **the whole app locks**, as for TrialEnded (ADR-0022). A revocation StoreKit reports locks at once, with or without the server. Nothing is deleted. |
| A Mac build without the StoreKit bridge wired (every build today) | NeedsOnlineCheck with "The App Store connection is not available in this build.": opening and exporting work, editing would lock, nothing is granted. This is a gap in the build, not a state a user of a finished build sees. |

**Trial start.** The server takes the trial transaction's Apple `purchaseDate`. When the same signed transaction carries an
earlier `originalPurchaseDate` (the original transaction date), the earlier one is used, so a restore can never move the start
later. Both are fields of the trial transaction. The app transaction's `originalPurchaseDate` is not even parsed by the server,
and the bridge does not carry it. Because Apple keeps the trial purchase on the Apple Account, a reinstall or a second Mac signed
in to the same Apple Account gets the same start and the same end.

**What the app accepts.** The verdict must name the earliest trial transaction StoreKit lists and must not end later than that
transaction's Apple purchase date plus 168 hours. An earlier end (from an earlier original purchase date) only shortens the trial
and is accepted. Its seconds left must agree with its own end and the server's clock within one second.

**Lease.** Each answer carries `leaseSeconds`: 6 hours by default, never more than the seconds left. The app counts down from that
answer for at most `min(leaseSeconds, remainingSeconds)` of monotonic time. When the trial time is used up first the state is
TrialEnded; when only the lease is used up the state is NeedsOnlineCheck with the reason LeaseExpired.

**Why no sequence of events gives more than 168 hours.** Every confirmed answer at server time `s` grants a window of at most
`trialEnd - s`. A new answer replaces the old one, so windows never overlap, and the monotonic clock never runs faster than the
server's clock (on the Mac it stops during sleep). The windows therefore add up to at most `trialEnd - firstCheck`, which is at
most 168 hours. Sleep, relaunch, network loss and StoreKit changes in any order do not change this; the property test in
section 15 runs such sequences.

## 3. Architecture

```text
 Mac (App Sandbox)                                          OpenFrame server (one per environment)
 +--------------------------------------------------+       +------------------------------------+
 | React webview                                    |       | POST /v1/mac/validate              |
 |   shows state, trial sheet, buttons              |       |   1. schema, nonce, environment    |
 |   (no StoreKit, no network)                      |       |   2. each JWS and the app JWS:     |
 |        | of_invoke (existing allow-listed op)    |       |      x5c chain to the pinned Apple |
 |        v                                         |       |      Root CA G3, ES256, bundle id, |
 | Rust core: openframe-entitlement                 |       |      environment, product, ids     |
 |   mac::MacAppStoreAdapter                        | HTTPS |   3. dates, duplicates, one        |
 |   revalidate(reason), lease, policy  ------------+------>|      appTransactionID              |
 |        ^                                         |<------+   4. trialEndsAt = purchase + 168h |
 |        | verified transactions                   |       |      remaining and lease from UTC  |
 | Desktop shell: StoreKitBridge (Rust)             |       |   5. Ed25519-signed answer         |
 |        | C ABI (JSON strings)                    |       |   no database, one log line        |
 |        v                                         |       +------------------------------------+
 | Swift: StoreKitBridge (StoreKit 2 only)  <-------+------> Apple App Store (purchase sheet,
 |   currentEntitlements, purchase, updates,        |        transactions signed with Apple's keys)
 |   AppTransaction, products                       |
 +--------------------------------------------------+
```

No Tauri command is added. Rule C3 in `scripts/architecture-lint.mjs` keeps the command list fixed, so the webview reaches
entitlement only through the existing `of_invoke` operation path. The bridge is a Rust module in the desktop shell behind
`#[cfg(target_os = "macos")]`; on Windows the adapter runs over `UnavailableBridge` and `UnavailableServer` and the state is
NeedsOnlineCheck.

## 4. Data flow

**A check** (`MacAppStoreAdapter::revalidate(reason)`; `refresh()` is the launch check):

1. The adapter calls `StoreKitBridge::current_entitlements`. Only verified, well-formed transactions are kept; a transaction from
   another environment than the build's is ignored and counted. If StoreKit does not answer, the list from earlier in this session
   stays, so a verified Lifetime keeps working.
2. Lifetime verified and not revoked: Purchased, and no server call. A revoked lifetime or trial transaction: Revoked at once.
3. A trial transaction exists: the adapter takes a fresh 32-byte nonce from the system's secure random generator (a repeated or
   all-zero nonce is refused and nothing is sent), adds the `AppTransaction` JWS when StoreKit returns one for this environment,
   and posts the request (section 7).
4. The adapter checks the answer: size, the Ed25519 signature over the domain prefix and the payload bytes, the schema and
   believable values, the nonce, then the environment. Any failure counts as "could not confirm".
5. A confirmed answer replaces the previous one and starts its lease on the monotonic clock. A failed check keeps the previous
   confirmed answer, which goes on counting down until its lease ends. It is never restarted or extended.

**When the shell calls it.**

| Reason | When |
|---|---|
| `Launch` | At start-up, on a background thread so it does not hold up the window. |
| `Resume` | After the Mac wakes, or when the window comes back after a long idle. |
| `StoreKitChanged` | For every verified `Transaction.updates` element and for an App Store account change. A revocation locks at once. |
| `LeaseExpired` | When `time_until_check()` reaches zero, and when the user chooses Check again. |

**Messages** (`online_check_message`). Every one ends with "Your projects stay open and can be exported."

| Reason | Message |
|---|---|
| `LeaseExpired` | OpenFrame needs to check your free trial again. Connect to the internet, then choose Check again. |
| `NoConnection` (the last confirmed state was a running trial) | OpenFrame needs the internet to continue your free trial. Connect and open OpenFrame again. |
| `NoConnection` | OpenFrame needs the internet to check your free trial. Connect and open OpenFrame again. |
| `CouldNotConfirm` | OpenFrame could not confirm your free trial with the App Store. Try again later. |
| `AppStoreUnavailable` | OpenFrame could not reach the App Store (or could not confirm your purchase). Check that you are signed in to the App Store, then open OpenFrame again. |

The cache file `<app data>/entitlement/mac-cache.json` only picks the words of these messages. It holds no id, date or JWS, is
never read to decide a state, and is not rewritten when nothing changed.

**Purchase.** `purchase_trial` and `purchase_lifetime` call `StoreKitBridge::purchase`. `Success` with a verified, well-formed
transaction for the right product and environment is added and checked with the server at once. `Pending` (Ask to Buy) grants
nothing and shows the trial as waiting. `Cancelled` changes nothing. A trial that already exists is never bought again.

**Restore Purchases.** `restore` calls `AppStore.sync()` and then the launch check. Automatic restore needs no button:
`currentEntitlements` already holds the Apple Account's purchases on a new Mac.

## 5. The StoreKit bridge

Swift package `apps/desktop/src-tauri/macos/StoreKitBridge/`, StoreKit 2 only, macOS 13 or later. Every function returns a
`strdup` JSON string that Rust frees with `storekit_free` exactly once. Every call blocks with a timeout (30 seconds for queries,
600 seconds for a purchase) and refuses to run on the main thread.

| C function | Returns |
|---|---|
| `storekit_current_entitlements()` | `{ok, transactions, unverifiedCount, skippedCount}` |
| `storekit_purchase(product_id)` | `{ok, outcome, transaction?}`, outcome `purchased`, `pending`, `cancelled`, `unverified` or `unknown` |
| `storekit_app_transaction()` | `{ok, verified, appTransactionID?, environment?, jwsRepresentation?}` |
| `storekit_environment()` | `{ok, environment}` from the verified `AppTransaction` |
| `storekit_products(ids_json)` | `{ok, products: [{id, displayName, displayPrice}]}`, for the price on the trial sheet only |
| `storekit_start_updates(callback)` | 1 when the `Transaction.updates` listener runs |
| `storekit_free(ptr)` | releases a returned string |

The core's contract is `mac::StoreKitBridge` with `mac::VerifiedTransaction` and `mac::AppTransactionInfo`. Each transaction
carries `productId`, `transactionId`, `originalTransactionId`, `appTransactionId` (optional: `None` before macOS 15.4),
`purchaseDateMs` and `revocationDateMs` (Apple's dates in Unix milliseconds), `environment` and the raw `signedJws`.
`AppTransactionInfo` carries the optional `appTransactionId`, the environment and the JWS, and no date: the app's first download
date is not the trial start. The bridge never compares a date with the current time and makes no entitlement decision.
Family-shared and unverified transactions are skipped and counted, never passed on.

## 6. The licence server

A separate Cargo project in `services/licence-server/`. It is not a member of the root workspace (its manifest has an empty
`[workspace]` table). It uses axum, rustls with the ring provider, x509-parser and ring for the Apple chain, and ed25519-dalek for
the answer. No database.

**Request checks** (`src/answer.rs`, `evaluate`): body at most 64 KiB; unknown fields refused; `schema` is 1; `nonce` is exactly
64 lowercase hex characters; `environment` is the one this deployment serves, spelled `sandbox` or `production` (`xcode` and
anything else is refused); at most 16 transactions.

**Checks on each signed record** (`src/apple.rs`), for every transaction JWS and for the `AppTransaction` JWS when present: size
at most 16 KiB; three JWS parts; header `alg` is `ES256`; `x5c` has exactly three certificates; the third is byte for byte the
pinned Apple Root CA G3; the intermediate names the root as issuer, is signed by it, is a CA and carries Apple's intermediate
marker `1.2.840.113635.100.6.2.1`; the leaf names the intermediate as issuer, is signed by it, is not a CA and carries Apple's
marker `1.2.840.113635.100.6.11.1`; the ES256 signature verifies with the leaf key; only then is the payload parsed; all three
certificates are valid at the payload's `signedDate`; `bundleId` is `com.openframe.studio`; the environment (`environment` in a
transaction, `receiptType` in the app transaction) is the deployment's. For a transaction also: `productId` is one of the two
products; `transactionId` and `originalTransactionId` are 1 to 64 decimal digits; `appTransactionId`, when present, is 1 to 128
characters of `A-Z a-z 0-9 - _ .`; `inAppOwnershipType` is `PURCHASED`; `type` is `Non-Consumable`.

**Checks across the request:** no `transactionId` twice; every `appTransactionId` present (in the transactions and in the app
transaction) is the same, since one request comes from one Apple Account and app, while records without one (macOS before 15.4)
are accepted; `purchaseDate` and `originalPurchaseDate` are not more than 5 minutes ahead of the server clock and not before
`APP_STORE_RELEASE_PLACEHOLDER_MS`; a `revocationDate` is not before the purchase and not in the future. Any failing check refuses
the whole request.

**Decision** (`decide`). `lifetime`: a lifetime transaction without `revocationDate`. Trial: the earliest non-revoked trial
transaction by start, then transaction id; `trialEndsAtMs = start + 168 h`; `remainingSeconds = max(0, trialEndsAtMs - server
now) / 1000`, rounded down; `leaseSeconds = min(6 h, remainingSeconds)`; `appleTransactionId` is Apple's `originalTransactionId`.
`revoked`: no lifetime and any transaction carries a `revocationDate`.

**Errors.** 400 `bad_request`, `unsupported_schema`, `missing_nonce`, `bad_nonce`, `wrong_environment`, `too_many_transactions`;
413 for a body over 64 KiB; 422 `malformed_transaction`, `unsupported_algorithm`, `bad_certificate_chain`, `bad_signature`,
`bad_payload`, `wrong_bundle_id`, `wrong_environment`, `unknown_product`, `bad_transaction_id`, `not_purchased`,
`duplicate_transaction`, `bad_purchase_date`, `app_transaction_mismatch`; 429 `rate_limited`. Errors are never signed; the app
reads every one as "could not confirm".

**Rate limit.** A token bucket per client address in memory: 10 requests at once, then one every 6 seconds. IPv6 addresses are
grouped by /64. At most 100 000 addresses are tracked.

**Logging.** One line per request: `validate outcome=<code> transactions=[<original transaction ids>]`. No address, no nonce, no
payload, no e-mail. Nothing else is stored.

## 7. Wire format version 1 and the shared fixture

The adapter is the consumer and defines the format (`crates/openframe-entitlement/src/mac/server.rs`); the server follows it.

Request, `POST /v1/mac/validate`, `Content-Type: application/json`:

```json
{
  "schema": 1,
  "nonce": "<64 lowercase hex characters: 32 random bytes>",
  "environment": "sandbox | production",
  "appTransactionJws": "<AppTransaction JWS> | null",
  "transactionJws": ["<transaction JWS>", "..."]
}
```

Response 200:

```json
{ "payload": "<base64url, no padding, of the payload JSON bytes>", "signature": "<base64url, no padding, Ed25519>" }
```

Payload JSON:

```json
{
  "schema": 1,
  "nonce": "<the request nonce>",
  "environment": "sandbox | production",
  "serverTimeMs": 1790000000000,
  "trial": { "trialEndsAtMs": 1790518400000, "remainingSeconds": 518400, "appleTransactionId": "2000000000000001", "leaseSeconds": 21600 },
  "lifetime": false,
  "revoked": false
}
```

`trial` is `null` when no valid trial transaction was sent. The signature covers the bytes `openframe-mac-licence-response-v1`,
one zero byte, then the payload bytes exactly as sent. The app verifies the signature over those bytes (strict Ed25519) before it
parses them, so no canonical JSON form is needed. The domain prefix keeps a signature made for another OpenFrame file, such as
the installer licence, from passing as an answer.

**Shared fixture.** `services/licence-server/tests/fixtures/mac-validate-v1.json` holds one request (a trial transaction JWS and
an app transaction JWS from a test certificate chain, a fixed nonce) and the response the server signed with its test-only key,
plus that key's public half and the test root. The server test
`the_shared_fixture_request_produces_the_shared_fixture_response_byte_for_byte` replays the request and must produce the same
response. The entitlement test `the_shared_wire_fixture_parses_and_verifies_against_the_servers_test_key` parses the request with
the adapter's own types, serializes it back unchanged, verifies the response with the public key and the nonce, and runs the
policy on it. The test key lives only in `services/licence-server/src/tests.rs`. To regenerate after a format change:
`cargo test -- --ignored write_the_shared_wire_fixture` in `services/licence-server`.

## 8. Threat model

| Attack | Control that stops it | Test |
|---|---|---|
| Turn the Mac's clock back or forward | The end is Apple's signed purchase date plus 168 hours against the server's UTC clock. The request has no time field, an extra field is refused, and the mac module never reads the wall clock. | `clock_rolled_back_or_forward_changes_nothing_only_the_monotonic_session_clock_counts`, `an_unknown_schema_and_a_client_clock_field_are_refused`, `the_mac_module_never_reads_the_wall_clock` |
| Count the trial from the app's first download | The server never parses `AppTransaction.originalPurchaseDate`; the bridge does not carry it. | `the_trial_starts_at_the_trial_purchase_never_at_the_apps_first_download` |
| Restore to move the trial start later | The start is the earlier of the trial transaction's `purchaseDate` and `originalPurchaseDate`. | `a_restore_with_a_later_purchase_date_cannot_move_the_trial_start` |
| Buy the trial again for a new start | The earliest trial transaction decides on both sides; the app never buys an owned trial again. | `repeated_trial_purchase_uses_the_earliest_apple_date_and_never_extends`, `the_earliest_trial_transaction_decides` |
| Keep the app open or asleep past the end | The lease bounds each answer; windows never overlap; the monotonic clock stops in sleep. | `app_kept_open_past_expiry_ends_the_trial_during_the_session`, `lease_expiry_during_a_session_locks_editing_until_a_new_check_succeeds`, `total_usable_time_across_any_sequence_of_leases_never_exceeds_168_hours` |
| Go offline after one check | Editing continues only until the lease ends, then locks until a check succeeds. | `revalidation_after_resume_with_the_server_unreachable_keeps_the_countdown_until_the_lease_ends_then_locks`, `network_blocked_during_the_trial_needs_an_online_check_with_no_extension_and_no_restart` |
| Delete the app and reinstall, or restore an old cache | The trial lives on the Apple Account; the cache never grants. | `reinstall_or_deleted_app_data_brings_the_same_state_back_from_the_store_and_server`, `backup_or_vm_rollback_of_the_cache_never_grants`, `cache_copied_from_another_mac_never_grants` |
| A second Mac with the same Apple Account | Same trial transaction, same end. | `same_apple_account_on_a_second_mac_gets_the_same_trial_end` |
| Mix records from two Apple Accounts in one request | Every `appTransactionId` present must match. | `app_transaction_ids_must_match_across_transactions_and_the_app_transaction` |
| Forge a transaction with a self-made chain, or splice Apple's root in | The chain must end in the pinned root byte for byte, with Apple's markers, checked link by link. | `a_chain_from_another_root_or_without_apples_marker_is_refused` |
| Sign with another key, edit a signed payload, `alg: none` | ES256 with the leaf key over header and payload; only ES256. | `a_signature_from_another_key_is_refused`, `a_tampered_payload_is_refused`, `family_shared_unknown_products_and_other_algorithms_are_refused`, `tampered_jws_is_rejected_and_never_reads_as_trial_not_started` |
| A transaction from another app or with a made-up id | `bundleId`, product id and digit-only transaction ids; duplicates refused. | `a_wrong_bundle_id_is_refused`, `bad_or_duplicate_transaction_ids_are_refused` |
| A purchase dated in the future or before release | Server clock plus 5 minutes skew; the release placeholder. | `a_purchase_date_in_the_future_or_before_the_release_placeholder_is_refused` |
| Sandbox purchases used against production, or the reverse | Separate deployments and keys; the environment must match in the request, every transaction, the app transaction and the answer; `Xcode` refused. | `sandbox_data_is_refused_in_production_and_production_data_in_sandbox`, `sandbox_transaction_in_a_production_build_is_ignored` |
| Family Sharing | Off in App Store Connect, skipped by the bridge, refused by the server. | `family_shared_unknown_products_and_other_algorithms_are_refused` |
| An unverified StoreKit transaction | Never passed on; checked again in the adapter. | `corrupt_or_unverified_transaction_never_reaches_the_policy` |
| Replay an old answer, edit the answer, a fake server | Fresh nonce in the signed payload; Ed25519 with the embedded public key; domain prefix. | `tampered_forged_or_replayed_server_response_is_refused`, `an_edited_answer_or_an_answer_from_another_key_fails_the_apps_check` |
| Refund and keep using | StoreKit's revocation locks at once; the server reports `revoked`. | `a_storekit_change_that_reports_a_revocation_locks_at_once`, `refund_or_revocation_ends_the_unlock_and_locks_open_and_export_too`, `a_revoked_trial_gives_no_trial_and_revoked` |
| Lose a paid unlock because the server is down | Lifetime comes from StoreKit only. | `revalidation_never_downgrades_a_verified_lifetime_when_the_server_or_storekit_is_unreachable` |
| Flood the server | Body cap, at most 16 transactions, header read timeout, token bucket per address. | `an_oversized_body_is_refused_before_parsing`, `too_many_transactions_are_refused`, `too_many_requests_from_one_address_are_limited` |
| Steal the signing key from the app | The app holds only public keys. | Design |
| Learn personal data from the server | The server receives Apple's transaction records only and logs only transaction ids. | Code review of `src/http.rs` |

## 9. What this design cannot prevent

- **A different Apple Account.** A user who signs in to the App Store with an entirely different Apple Account is a new App Store
  customer. That account gets a new `appTransactionID` and can buy the zero-price trial again, so it gets a new 7-day trial.
  StoreKit's account-level entitlement logic cannot perfectly prevent this, and this design does not try to link accounts.
- **No device fingerprinting.** OpenFrame does not use hardware identifiers, MAC addresses, serial numbers or any other device
  fingerprint to tie a trial to a Mac. They would be personal data, are partly unavailable in the App Sandbox, and would conflict
  with the project's privacy rules.
- **DeviceCheck and App Attest.** If they are investigated later, they are supplementary only: detected at runtime, used when
  present, and never required on macOS. A Mac without them must get exactly the behaviour described here.
- **A patched app binary.** A copy patched to skip the check is outside the Mac App Store and its signature. Not stopped by this
  design.
- **Usage inside one lease while offline.** After a successful check the app may keep editing offline until the lease ends (at
  most 6 hours of use), even if the trial ends on the server's clock during a long sleep in between. The total over the whole
  trial still never exceeds 168 hours of use (section 2).
- **Certificate revocation.** The server does not check OCSP for Apple's leaf certificate.

## 10. App Store Connect setup (owner)

Do these in App Store Connect with the Account Holder or Admin role. Exact screen names change; follow Apple's current help pages.

1. **Agreements, Tax and Banking.** Accept the Paid Apps agreement and finish tax and banking. In-app purchases do not work
   without it, even at zero price.
2. **App record.** Bundle id `com.openframe.studio`, platform macOS, price of the app itself: Free.
3. **7-day Trial.** In-App Purchases, create: type Non-Consumable, reference name `7-day Trial`, product id
   `com.openframe.studio.trial7`, price Free (the zero price point), Family Sharing off, display name `7-day Trial`, a short
   description such as "Use every feature for 7 days. Free.", a review screenshot of the trial sheet and a review note that
   explains the trial.
4. **Lifetime Access.** Create: type Non-Consumable, reference name `Lifetime Access`, product id `com.openframe.studio.lifetime`,
   the price you choose, Family Sharing off, display name `Lifetime Access`, review screenshot and note.
5. **Before you start sheet.** Apple's review guideline 3.1.1 allows a free trial through a zero-price non-consumable and asks the
   app to say, before the trial starts, how long it lasts, what stops working after it, and what the user must pay to continue.
   Check the current wording of the guideline before submission. The sheet copy (prices come from StoreKit, never typed in code):

   > **Try OpenFrame Studio free for 7 days**
   >
   > The trial is free. Starting it charges nothing.
   >
   > It lasts 7 days (168 hours) from the moment you start it, on every Mac signed in to your Apple Account.
   >
   > When it ends, editing, creating, importing and the assistant stop working. You can still open, export and print every
   > project.
   >
   > To keep editing, buy Lifetime Access for {price}. It is a one-time purchase, not a subscription.
   >
   > During the trial, OpenFrame checks your trial online when it starts, after your Mac wakes, and at least every 6 hours while
   > you use it. The check sends Apple's purchase records for OpenFrame and nothing from your projects.
   >
   > Buttons: **Start free trial**, **Buy Lifetime Access for {price}**, **Restore Purchases**, **Not now**

   Update 2026-10-07: the owner's mock 187 replaced this sheet in the app. It offers **Start free trial**, **Buy full version**
   and **Restore purchase**, shows no price and has no "Not now". ADR-0023 makes "no price, currency or pricing model anywhere in
   the app" a rule: Apple's own payment sheet shows the price. The sheet copy above (with `{price}`, "Lifetime Access",
   "one-time purchase, not a subscription" and "You can still open, export and print every project") is SUPERSEDED by
   ADR-0022 and ADR-0023. Whether the app's screen satisfies guideline 3.1.1 is open for the owner; see
   [36-entitlement-and-trial.md](36-entitlement-and-trial.md) section 10.7.

6. **Sandbox testers.** Users and Access, Sandbox, add at least three testers with addresses that are not real Apple Accounts. Use
   one for the trial path, one for Lifetime Access, one for refunds. Clearing a tester's purchase history lets you run the trial
   again.
7. **TestFlight.** Upload a build, add internal testers, and test the purchases there. TestFlight purchases are in the Sandbox
   environment, so the app must reach the sandbox server deployment. App Review also uses the sandbox, so the released build
   must keep both public keys and both server addresses and choose by the verified `AppTransaction` environment.
8. **Submit both in-app purchases with the first build** that uses them.
9. **Release placeholder.** Set `APP_STORE_RELEASE_PLACEHOLDER_MS` in `services/licence-server/src/answer.rs` to a date no later
   than the first sandbox purchase of the trial product (for example the day the in-app purchases were created). It is
   2026-01-01 until then.

## 11. Wiring the bridge into the app (desktop shell)

1. Build the Swift package on an Apple silicon Mac: `swift build -c release --arch arm64` in
   `apps/desktop/src-tauri/macos/StoreKitBridge/`, and link it from `apps/desktop/src-tauri/build.rs` on macOS only
   (`StoreKitBridge`, the `StoreKit` framework and the Swift runtime search paths).
2. Implement `mac::StoreKitBridge` over the C functions, `mac::LicenceServer` over HTTPS with a bounded timeout, and
   `mac::NonceSource` with the system's secure random generator, and hand them to the core as `mac::MacInputs` with the
   environment's response public key.
3. Call `revalidate(RevalidateReason::Launch)` on a background thread at start-up, `Resume` on wake and window return after a long
   idle, `StoreKitChanged` from the `Transaction.updates` listener and on an App Store account change, and `LeaseExpired` when
   `time_until_check()` runs out or the user chooses Check again.
4. Add the App Sandbox network client entitlement (`com.apple.security.network.client`) and disclose the check in the privacy
   policy and the store listing.

## 12. Server deployment

| Variable | Meaning |
|---|---|
| `OPENFRAME_LICENCE_SIGNING_KEY` | Required. base64url (no padding) of the 32-byte Ed25519 seed. |
| `OPENFRAME_APPLE_ENVIRONMENT` | Required. `Production` or `Sandbox`. |
| `OPENFRAME_BIND` | Listen address. Default `127.0.0.1:8787`. |
| `OPENFRAME_TLS_CERT`, `OPENFRAME_TLS_KEY` | PEM paths. Required on any address other than loopback; the server refuses to start without them. |

- **Two deployments.** One for `Production`, one for `Sandbox`, each with its own signing key and its own address. The app
  embeds both public keys and picks by the verified `AppTransaction` environment.
- **The signing key.** Generate each seed offline. Keep it in the hosting platform's secret store and inject it as the
  environment variable at start. Never put it in the repository, an image or a log. The server prints its public key at start so
  you can check it matches the key in the app. The test key in `src/tests.rs` is for tests only and must never be deployed.
  Changing a key needs an app update, because the app embeds the public key.
- **TLS.** The server terminates TLS itself with rustls, so the rate limit sees the real client address. If you put a load balancer
  in front, the rate limit must be changed to read the balancer's client address header first.
- **Server clock.** The server's UTC clock decides every trial. Keep NTP running on the host.
- **Logs.** Keep the one-line logs for a short, stated period. They hold transaction ids only.
- **Apple root.** The pinned root is Apple Root CA G3, valid until 2039-04-30. Its SHA-256 fingerprint is in `src/apple.rs` and
  the server refuses to start if the embedded certificate does not match it. **Owner check:** compare the fingerprint with the one
  published at <https://www.apple.com/certificateauthority/> before the first production deployment. If Apple announces a new
  root, add it next to the old one, deploy, and remove the old one after Apple stops using it. The app does not change.
- **Health.** There is no health endpoint yet; a TCP check on the port is enough to start with.

## 13. Manual test procedure on a Mac

Use a TestFlight build or a development build signed for sandbox, the sandbox server deployment, and the sandbox testers from
section 10. Record each result in the release evidence.

1. **First launch.** The first-launch screen (mock 187) offers the 7-day free trial, Buy full version and Restore purchase, with no
   price (see the update under section 10, step 5).
2. **Start the trial.** Sign in with the trial tester when asked. The app shows 7 days left. The server log shows one `ok` line
   with the trial's original transaction id.
3. **Clock changes.** Quit. Set the Mac's date 10 days forward, launch: still the same days left. Set it 10 days back, launch: still
   the same. Restore automatic time.
4. **Reinstall.** Delete the app and its container, reinstall, launch with the same tester: the same end time, no new trial.
5. **Second Mac.** Sign in to the same tester on another Mac, launch: the same end time.
6. **Refund.** Buy Lifetime Access with the refund tester, check that editing works, then request a refund for that purchase
   from Apple's sandbox tools. Confirm that the app moves to Revoked (the lock screen with only Buy full version and Restore
   purchase; no project opens or exports) while it runs and after a relaunch. If the sandbox does not grant refunds on demand, record that and test revocation with StoreKit testing in Xcode
   against the bridge only.
7. **Offline launch.** During the trial, turn off the network and launch: the connectivity message shows, open and export work.
   Turn the network on and choose Check again: the trial state returns. After a lifetime purchase, launch offline: Purchased.
8. **Lease.** With a sandbox server deployment configured for a short lease, keep the app open past it with the network off:
   editing locks with the "check again" message; turn the network on and choose Check again: editing returns.
9. **Sleep.** Start a check, sleep the Mac for an hour with the network off, wake: editing continues until the lease ends.
10. **Trial end.** Use a tester whose trial started more than 168 hours ago (or wait): TrialEnded, the lock screen with only Buy
    full version and Restore purchase, which cannot be closed; no project opens or exports. Buy, then confirm every project opens
    exactly as it was.
11. **Ask to Buy.** With a sandbox tester set up for Ask to Buy, buy Lifetime Access: the app says the purchase waits for approval
    and nothing unlocks. Approve it: the purchase arrives through `Transaction.updates` and the app unlocks.
12. **Cancel.** Close the purchase sheet: nothing changes and no error shows.
13. **Server down.** Stop the sandbox server during the trial: the connectivity message after the lease, never a crash or a hang.
14. **macOS older than 15.4.** On such a Mac, the trial check works without `appTransactionID`.

## 14. Proven here and EVIDENCE PENDING

**Proven on this machine** (Windows 11, `cargo test` on 2026-10-06, code and tests only, with fakes and a locally generated
certificate chain):

- The adapter, policy and wire contract: 44 tests in `crates/openframe-entitlement/src/mac/`, including lease expiry, revalidation
  with the server unreachable, revocation through a StoreKit change, a Lifetime that never downgrades, the 168-hour bound under
  random sleeps, relaunches and network loss, and the source scan that the module never reads the wall clock.
- The licence server: 33 tests, covering every check in section 6, the lease, the trial origin, and the signed answer.
- One wire format: the shared fixture is reproduced byte for byte by the server and parsed and verified by the adapter.

**EVIDENCE PENDING** (nothing below has been run or seen):

- The Swift bridge compiled and run; its API use (for example `AppTransaction.appTransactionID` on macOS 15.4, `purchase()` on
  macOS) checked against the current SDK.
- A real StoreKit sandbox purchase of the trial and of Lifetime Access; TestFlight; Restore Purchases; a refund or revocation;
  a reinstall; a second Mac with the same Apple Account; Ask to Buy.
- A real Apple-signed transaction and app transaction verified by the server, including the exact payload field names
  (`appTransactionId`, `receiptType`) on current Apple data.
- Production validation with production keys.
- The server deployed: host, TLS certificate, both signing keys, both public keys embedded in the app.
- The owner's comparison of the Apple Root CA G3 fingerprint with Apple's published one.
- The release placeholder date set by the owner (section 10, step 9).
- Apple review's acceptance of this trial method under the review guidelines current at submission.

## 15. Tests

`crates/openframe-entitlement/src/mac/` (44): `bridge.rs` 3, `server.rs` 6, `policy.rs` 6, `tests.rs` 29. The new ones for the
owner's corrections: `a_missing_app_transaction_id_is_well_formed_because_apple_sends_it_from_macos_15_4_only`,
`a_verdict_is_good_for_its_lease_and_never_beyond_the_seconds_left`,
`lease_expiry_during_a_session_locks_editing_until_a_new_check_succeeds`,
`revalidation_after_resume_with_the_server_unreachable_keeps_the_countdown_until_the_lease_ends_then_locks`,
`a_storekit_change_that_reports_a_revocation_locks_at_once`,
`revalidation_never_downgrades_a_verified_lifetime_when_the_server_or_storekit_is_unreachable`,
`total_usable_time_across_any_sequence_of_leases_never_exceeds_168_hours`,
`the_shared_wire_fixture_parses_and_verifies_against_the_servers_test_key`.

`services/licence-server` (33, plus the ignored fixture writer `write_the_shared_wire_fixture`): `apple.rs` 3, `ratelimit.rs` 3,
`tests.rs` 27. The decision: `a_running_trial_ends_168_hours_after_apples_purchase_date`,
`an_expired_trial_has_zero_seconds_left_and_a_zero_lease`, `the_lease_is_six_hours_and_never_beyond_the_remaining_trial`,
`a_restore_with_a_later_purchase_date_cannot_move_the_trial_start`,
`the_trial_starts_at_the_trial_purchase_never_at_the_apps_first_download`, `the_earliest_trial_transaction_decides`,
`a_lifetime_purchase_unlocks`, `a_revoked_lifetime_purchase_is_revoked_and_not_lifetime`,
`a_revoked_trial_gives_no_trial_and_revoked`, `no_transactions_means_no_trial_and_no_purchase`. The signed records:
`a_wrong_bundle_id_is_refused`, `sandbox_data_is_refused_in_production_and_production_data_in_sandbox`,
`a_signature_from_another_key_is_refused`, `a_tampered_payload_is_refused`,
`a_chain_from_another_root_or_without_apples_marker_is_refused`,
`family_shared_unknown_products_and_other_algorithms_are_refused`, `bad_or_duplicate_transaction_ids_are_refused`,
`app_transaction_ids_must_match_across_transactions_and_the_app_transaction`,
`a_missing_app_transaction_id_is_accepted_because_apple_sends_it_from_macos_15_4_only`,
`a_purchase_date_in_the_future_or_before_the_release_placeholder_is_refused`. The request: `an_oversized_body_is_refused_before_parsing`,
`too_many_transactions_are_refused`, `a_missing_or_malformed_nonce_is_refused`,
`an_unknown_schema_and_a_client_clock_field_are_refused`. The answer:
`an_edited_answer_or_an_answer_from_another_key_fails_the_apps_check`, `too_many_requests_from_one_address_are_limited`,
`the_shared_fixture_request_produces_the_shared_fixture_response_byte_for_byte`. In `apple.rs`:
`the_embedded_apple_root_matches_its_fingerprint`,
`transaction_ids_are_bounded_digits_and_app_transaction_ids_are_bounded_tokens`,
`the_wire_spelling_is_lower_case_and_refuses_xcode`.
