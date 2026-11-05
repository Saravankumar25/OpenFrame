# 02. Trial and licensing: one behaviour for both platforms

Both sessions implement this file. Platform differences are in 03 (Windows) and 04 (Mac). The Rust core decides; the shells only ask and show (this is how the code works today and it stays that way).

## 1. Decisions already made by the owner (do not reopen)

| # | Decision |
|---|---|
| D1 | Trial is 7 days (`TRIAL_DAYS = 7`). |
| D2 | Windows: the Microsoft Store owns the trial clock, the purchase and the expiry. The app never runs its own authoritative timer. |
| D3 | Mac: the trial is computed offline by the app from Apple-signed StoreKit data. No licence server. No device binding. Different Apple Accounts get separate trials; this loophole is accepted. |
| D4 | Purchase is a lifetime licence. Windows: the Store app purchase. Mac: the paid non-consumable. Apple and Microsoft handle all payments. |
| D5 | A buyer, on any day, never sees the chip, the Buy button or a reminder again. |
| D6 | The chip says "Trial ends in N days" and sits on Application Home and on every project screen. |
| D7 | Reminders only on trial days 5, 6 and 7 (3, 2, 1 days left), at most once per calendar day. |
| D8 | After purchase, once and in order: data note, file safety tutorial, welcome popup. |
| D9 | Contact: one in-app "Contact us" link to `https://straftstudio.com/contact/`. No form in the app. |
| D10 | Social addresses: the owner sent the real ones on 2026-11-05 (file 07). |
| D11 | Internal builds (`DirectInstaller`) never show any licence screen. Unchanged. |
| D12 | The lock after trial end or refund stays a full lock that cannot be closed (existing `locks_the_app`). It must never lose unsaved work. |

## 2. States and what shows

The existing `EntitlementState` set stays. One state is added (Mac only): `ClockChanged`.

| State | Windows | Mac | Chip and Buy | Screen due |
|---|---|---|---|---|
| `TrialActive{days_left}` | yes | yes | chip + Buy | Reminder when `days_left <= 3`, once per day |
| `TrialEnded` | only if the trial ends while the app is open (the Store blocks launch afterwards) | yes | none | Lock (trial ended) |
| `Purchased` | yes | yes | none | data note, tutorial, welcome, once each, in order |
| `Revoked` | refund detected while open | refund or revocation | none | Lock (refunded) |
| `NeedsOnlineCheck` | licence unreadable and nothing cached | store unreachable and nothing cached | none | Check screen (closable); editing locked, as today |
| `TrialNotStarted` | never | first launch | none | Mac first-launch choice |
| `ClockChanged` (new) | never | clock behind a date the app already trusted | none | Clock lock (cannot be closed) |
| `InternalBuild` | yes | yes | none | nothing |

Rules for ordering and the "once" flags stay as in `notices::due` and `record_notice`. Do not weaken them. Add the new state to `locks_the_app`, `permits`, `summary`, the generated TypeScript types (`ts-rs`) and the Swift types (`gen-swift-types.mjs`), and to `notices::due` (Clock lock beats everything except internal build).

## 3. The chip

- Visible only in `TrialActive`. Never in any other state, never for a buyer.
- Text: `Trial ends in N days` for N from 2 to 7, and `Trial ends in 1 day` for N = 1. `N = ceil(remaining time / 24 h)`, clamped to 1..7 (this is how Windows and Mac already compute `days_left`; keep it).
- Amber when `N <= 3` (`LAST_DAYS`). Screen reader text: `Free trial. Ends in N days.` and, when amber, add `Your trial ends soon.` For N = 1: `Free trial, last day. Your trial ends within 24 hours.`
- Beside it: the button `Buy full version`. Windows opens the Microsoft Store purchase window. Mac opens Apple's purchase sheet.
- **Placement (this is a gap today).** The chip exists only in the project shell top bar (`TrialCorner` inside `.tb-actions` in `Shell.tsx`; `TrialCorner.swift` on the Mac). `AppHome.tsx` has no licence code. Mount the same component on Application Home as well (top right, same position and styling) so the chip is visible on the first screen the user sees. Do not duplicate the logic: one component, mounted in both places. Check whether Home renders inside the shell; if it does not, mount it in the Home header.
- Update the existing tests that assert `Trial: N days left` (for example `licensing.test.tsx`, Swift `LicenceRules` tests) to the new wording.

## 4. Reminders

- Shown when `days_left` is 3, 2 or 1 (trial days 5, 6, 7), at most once per local calendar day, never after purchase. Existing `REMINDER_DAYS_LEFT = 3` and `reminder_shown_on` stay.
- **Where.** Over a project or on Application Home. Today the Mac model shows a reminder only over an open project (`projectOpen` gate in `LicenceModel.swift`) and the Windows shell mounts reminders through `LicenceScreens`. Remove the project-only gate on the Mac and make sure Windows also shows it on Home.
- Headline: `Your free trial ends in N days` (N = 2 or 3). For N = 1 use `Your free trial ends today` if the end moment falls on today's local date, otherwise `Your free trial ends tomorrow`. Under it, one line with the real end moment from the core: `It ends on Thu 5 Nov at 9:42 PM.` This needs the core to expose the trial end moment (`trial_ends_at`, local time formatted by the shell). Windows computes it as now plus `TrialTimeRemaining`; Mac from the verified start plus 168 hours. If you cannot expose it cheaply, say so in the handoff and keep the headline only.
- Buttons: `Buy full version` and `Continue trial`. Closing the popup counts as `Continue trial`.
- Clock safety (already in `notices.rs`): a stored reminder day later than today counts as shown, so moving the clock never shows it twice. Keep that.

## 5. The buyer path

Unchanged from the existing code and mocks 193 to 198, with these checks:

- After any purchase (day 1 or day 6), the state becomes `Purchased` and the chip, button and reminders disappear at once, with no restart. On Windows this comes from `OfflineLicensesChanged` and the purchase result; on the Mac from `Transaction.updates` and the purchase result.
- A buyer on a new computer who restores the purchase goes through the data note, tutorial and welcome again only if the local flags are missing (new computer, new data folder). That is correct and expected.
- Reinstall or update never resets the flags (they live in `entitlement/notices.json` in the app data folder, which survives updates; see file 05).

## 6. The lock (both platforms)

- Trigger: `TrialEnded` or `Revoked`. Enforced in the Rust core (`check_operation` in `Registry::dispatch`): every project operation is refused underneath the lock, the screen cannot be closed, Escape does nothing, no route into projects. Keep.
- **No data loss.** Before the lock screen appears, the app must finish any pending save (flush the project writer and checkpoint, the same path used when closing a project). Add a test: type text, expire the trial in the test double, assert the text is on disk after the lock. If the existing code already does this, add the test only and say so.
- Allowed while locked (existing `ALLOWED_WHILE_LOCKED`): `app.info`, `appearance.get` (`team_gate.status` was removed with the team password, ADR-0043). Opening a link in the browser is a shell action, not a core operation, and must keep working so Contact us works.
- Buttons on the lock screen: `Buy full version`, `Restore purchase` (Windows: `Check again`), and a small text link `Need help? Contact us` (opens the contact page). This adds one link to the earlier design, which had none. Reason: a locked person with a store problem needs a way to reach support.
- Copy (both platforms, use "this computer" on Windows and "this Mac" on the Mac):
  - Trial ended: title `Your free trial has ended`. Body: `Buy the full version to keep using Straft Studio. Your projects are safe on this computer. Buying unlocks them again exactly as you left them.`
  - Refunded or removed: title `Your purchase is no longer active`. Body: `The store no longer shows a purchase for this account, so Straft Studio is locked. Your projects are still on this computer. If this is a mistake, choose Restore purchase or contact us.`
  - Windows detail: a trial that ended before launch is blocked by the Microsoft Store itself with the Store's own dialog. Straft does not draw that one (mock 186 shows the in-app case only).

## 7. Check screen (unchanged behaviour, one added link)

`NeedsOnlineCheck` shows `Checking your purchase`, can be closed, editing stays locked while it is named, the top bar offers `Check purchase` after closing. Add `Need help? Contact us` under the buttons. Never block the app because a read failed; never lock a buyer because the network is down (file 04, rule R3 for the Mac; file 03, section W3 for Windows).

## 8. Prices

- Windows: no price in the app. The Microsoft Store purchase window shows it.
- Mac: Apple's App Review Guideline 3.1.1 requires a "XX-day Trial" free item, and that the app clearly identifies the trial length, what stops working when it ends, and any later charge, before the trial starts. So the Mac first-launch screen and the lock screen show the price returned by StoreKit (`Product.displayPrice`, already localised and tax-aware). It is read at run time. It is never typed in code, strings, mocks or the StoreKit config's text. Mock 187 and 190 show the token `{price from the App Store}`.

## 9. Contact, feedback and socials

See file 07. In short: `Contact us` opens the website contact page. The welcome popup shows the eight real social addresses in every build.

## 10. Test matrix (automate in Rust and in the UI tests; the manual rest goes in `docs/release/`)

Both platforms:
1. Day 1 to day 7: the chip text for N = 7..1, amber at 3, 2, 1.
2. Reminder: shown on days 5, 6, 7 only, once per calendar day, not on days 1 to 4, not after purchase, not twice when the clock moves back and forth, shown on Home and over a project.
3. Buy on day 1, day 4, day 7: chip, button and reminder gone at once, then data note, tutorial, welcome once each in order, never again after restart.
4. Trial ends while the app is open: pending save flushed, lock appears, cannot be closed, Buy and Restore work, Contact us opens the browser.
5. Refund while open: lock with the refunded wording.
6. Unreadable licence with nothing cached: `NeedsOnlineCheck`; with a cached `Purchased` or `TrialActive`: keeps the cached state, no lock.
7. Internal build: no screen, no chip.
8. Update simulation: install N, use the app through the trial, install N+1 over it, the flags, reminder day and projects are intact (file 05).
9. Accessibility: every new or changed screen is reachable and readable by keyboard and screen reader (Narrator, VoiceOver); the chip has the spoken text above.
10. Writing standard gate passes on all new text.

Mac only (file 04 has the full table): clock rollback, offline relaunch, restore on a new Mac, second Apple Account, refund, StoreKit transaction manager tests with `.storekit` and `SKTestSession`.
Windows only (file 03): unit tests against the `StoreLicenceSource` test double for every mapping row; manual Store test with a private or hidden submission.
