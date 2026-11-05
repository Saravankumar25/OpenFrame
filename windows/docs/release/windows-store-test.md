# Windows Store manual test

Status: written 2026-11-03 for the first Store submission. Nothing here has been run yet, because the app record in Partner
Center does not exist yet. The Microsoft Store has no simulator for `StoreContext`, so the mapping and every refresh trigger are
tested with test doubles in the code (package file 03, W7 item 1). This page is the manual part (W7 item 2): a person with a
clean Windows 11 PC and a real Store submission.

Sources: [package file 02](package-2026-11-03/02_TRIAL_AND_LICENSING_SPEC.md) (wording and states),
[package file 03](package-2026-11-03/03_WINDOWS_TASKS.md) (W3 to W7), [package file 05](package-2026-11-03/05_UPDATE_SAFETY_AND_STANDARDS.md)
(update safety), [27-microsoft-store.md](../engineering/27-microsoft-store.md), [ADR-0039](../adr/ADR-0039-store-release-licensing.md).
Check the wording in a row against the screen. If the screen and this page disagree, the code is right: fix this page.

Record each run at the bottom (date, package version, tester, result per row). Do not tick a row you did not do.

## Before you start

1. A package built by the real pipeline (`node scripts/build-msix.mjs --store`) with the real identity, uploaded to Partner Center.
   The version is higher than any earlier submission (`packaging/msix/submissions.json`).
2. In Partner Center, the submission is limited: private or hidden, or a limited audience, as the options read on the day. Only
   the tester accounts can find it. Free trial is set to 7 days in Pricing and availability.
3. A clean Windows 11 PC (a real PC or a full virtual machine; use Windows Sandbox only for the clean first launch check, not for purchases) signed in
   with a Microsoft account that is on the audience list. No developer tools installed. No Straft folders in Documents.
4. A second tester account for the refund and reinstall rows, if the first account is already used up.
5. Write down: Windows build, package version, whether the account had owned the app before.

## Rows

Tick each box only after you saw it.

### 1. Install and first start

- [ ] The Store page shows the name, the listing text and the screenshots of the real app. The listing says an AI model is
  downloaded after install, on a click.
- [ ] Install from the Store. No administrator prompt. Start it from Start. The icon on the tile, the taskbar and Alt+Tab is the
  Straft logo.
- [ ] The app opens on Application Home with no account screen and no sign-in.
- [ ] If WebView2 is missing (rare on Windows 11), the app explains it and offers the download page. Do not break WebView2 on
  purpose on the main test PC; use a spare VM.

### 2. Trial start and the chip

- [ ] A person who installs the app gets the Store's free trial. The chip on Application Home reads `Trial ends in 7 days`.
- [ ] Open or create a project. The same chip is in the project top bar. It is the same words.
- [ ] The button beside the chip reads `Buy full version`.
- [ ] Narrator reads `Free trial. Ends in 7 days.` on the chip. The chip is reachable with the keyboard.
- [ ] Wording by day, when you can observe the day: `Trial ends in N days` for N from 2 to 7, `Trial ends in 1 day` for N = 1.
  The chip is amber for N = 3, 2, 1, and Narrator adds `Your trial ends soon.` (for 1 day: `Free trial, last day. Your trial ends
  within 24 hours.`). The real trial runs in real time. Days 5 to 7 are checked live only if the tester waits that long;
  otherwise they stay covered by the test double (matrix row 1 and 2 below).

### 3. Reminders

- [ ] On trial days 5, 6 and 7 the reminder appears, once per calendar day, on Application Home and over an open project.
  Not on days 1 to 4.
- [ ] Headline `Your free trial ends in 3 days` or `Your free trial ends in 2 days`. On the last day: `Your free trial ends today` when
  the end falls on today's date, otherwise `Your free trial ends tomorrow`.
- [ ] Under the headline, one line with the real end moment, in this form: `It ends on Thu 5 Nov at 9:42 PM.`
- [ ] Buttons `Buy full version` and `Continue trial`. Closing the popup counts as `Continue trial`.
- [ ] After `Continue trial`, restart the app on the same day: the reminder does not come back.

### 4. Buy

- [ ] `Buy full version` opens the Microsoft Store purchase window. The price is shown by the Store. The app shows no price.
- [ ] Complete the purchase. Without a restart the chip, the button and any reminder disappear.
- [ ] Then, once each and in this order: the data note, the file safety tutorial, the welcome popup. Restart the app: none of them
  comes back.
- [ ] Buy on a different day (day 1, a middle day, the last day) on a different account and see the same result.

### 5. Refund

There is no refund button in the app. Refund through Microsoft support, or with a test account whose purchase you can cancel
in the Partner Center tools available on the day. Check the current process before you start.

- [ ] Refund while the app is open: the app locks. Title `Your purchase is no longer active`. Body: `The store no longer shows a
  purchase for this account, so Straft Studio is locked. Your projects are still on this computer. If this is a mistake,
  choose Restore purchase or contact us.` (On Windows the button is `Check again`.)
- [ ] The lock cannot be closed and Escape does nothing.
- [ ] Refund while the app is closed: on next start the app shows the refunded lock, and the projects are untouched.
- [ ] After the refund is reversed or a new purchase is made, `Check again` unlocks the app with the work exactly as it was.

### 6. Lock while the app is open

Use a spare account on the last day of its trial, or the test double route in the code if the trial cannot be reached.

- [ ] While the trial runs out with the app open and a change typed but not saved: the text is on disk after the lock appears
  (reopen the project after buying to confirm).
- [ ] Title `Your free trial has ended`. Body: `Buy the full version to keep using Straft Studio. Your projects are safe on this
  computer. Buying unlocks them again exactly as you left them.`
- [ ] Buttons `Buy full version` and `Check again`, and the link `Need help? Contact us`. The lock cannot be closed.
- [ ] A trial that ended before launch is stopped by the Store with its own dialog. Straft does not draw that one.

### 7. Contact us

- [ ] The About screen has `Contact us`. The lock screen and the `Checking your purchase` screen have `Need help? Contact us`.
- [ ] Choosing it opens `https://straftstudio.com/contact/` in the default browser, also while the app is locked.
- [ ] The page shows `support@straftstudio.com`. The app itself sends nothing (watch the network if you have a tool for it).

### 8. Unreadable licence

- [ ] Disconnect the network and start the app after a purchase: the app keeps the cached state and does not lock a buyer.
- [ ] A new install with no network and nothing cached: the app shows `Checking your purchase`. The screen can be closed.
  Editing stays locked while the notice is named. The top bar offers `Check purchase` after closing. With the network back,
  `Check purchase` clears it.
  Opening and exporting a project work; a change to the project (a new card, a typed line) is refused with `Straft could not
  check your purchase, so editing is paused.` (ADR-0039, enforced in Rust, not only on the screen).
- [ ] Copy the installed `straft-desktop.exe` out of its package and start it: it still shows `Checking your purchase` and never an open editor.

### 9. Reinstall, update, uninstall

- [ ] Uninstall from Settings > Apps. Projects and the Global Idea Vault stay in `Documents\Straft Studio`. The package data folder
  under `%LOCALAPPDATA%\Packages\<family>` is gone, so settings and downloaded AI files are removed.
- [ ] Reinstall from the Store. A buyer sees no chip. The first-run notices (data note, tutorial, welcome) show again only
  because the local flags were removed with the data folder. This is expected (package file 02, section 5). Projects open with
  `Open project`; the recent list is empty.
- [ ] Update to the next version: follow [update-simulation.md](update-simulation.md).

### 10. AI download

- [ ] `Download Straft AI` is a click. Nothing downloads before the click. After the download the AI works with the network
  off.
- [ ] A failed or cancelled download leaves the rest of the app working.

## Matrix rows that cannot be automated

These are the rows of package file 02, section 10, that need a person, a real Store or a screen reader. The other rows are
automated in Rust and in the UI tests.

| Row | What | Where it is done |
|---|---|---|
| 1 | Chip text for N = 7 down to 1, amber at 3, 2, 1 | Automated with the test double. Live check of day 1 above; later days only if the tester waits |
| 2 | Reminder only on days 5, 6, 7, once per day, on Home and over a project | Automated with the test double. Section 3 above for the live screen |
| 3 | Buy on day 1, day 4, day 7: chip, button and reminder gone at once, then notices once each in order | Section 4 above, with real purchases |
| 4 | Trial ends while the app is open: save flushed, lock, cannot be closed, Contact us opens the browser | Section 6 above |
| 5 | Refund while open: lock with the refunded wording | Section 5 above |
| 6 | Unreadable licence: `NeedsOnlineCheck`, or the cached state is kept | Section 8 above |
| 8 | Update simulation: flags, reminder day and projects intact | [update-simulation.md](update-simulation.md) |
| 9 | Keyboard and screen reader (Narrator) on every new or changed screen, contrast in all 15 themes | Walk each screen above with Narrator on, in at least a light and a dark theme |

Row 11 (SKU guard, ADR-0039 item 3): on a real Store install, confirm `GetAppLicenseAsync` returns a non-empty `SkuStoreId` for
both a trial and a purchase (if it were empty, buyers would see the check screen). On a sideloaded or dev-signed package, confirm
the app does not read as purchased. The guard applies to active licences only: after a refund, confirm the lock screen shows even if
the SKU id reads empty (an inactive licence is always an answer).

Row 7 (internal build shows no screen) is not a Store test: it belongs to the internal installer check.
Row 10 (writing standard) runs as `npm run check:writing`.

## Run record

| Date | Package version | Tester | Windows build | Rows done | Result and notes |
|---|---|---|---|---|---|
| (none yet) | | | | | |
