# Mac store test checklist (manual)

Automated: `crates/straft-entitlement/src/mac/tests.rs` (every rule and G1 to G4), `crates/straft-ffi/src/store.rs`,
`apps/macos/Tests/StraftUITests/Store*.swift` and the E2E store scenario. What needs a real App Store is below. It needs
the owner's developer account and a signed sandbox build. None of these have run yet.

With Xcode and `apps/macos/packaging/StraftStudio.storekit` (`SKTestSession`, a time rate to run 7 days in minutes):

1. Start the trial, watch the chip count 7 down to 1, amber at 3, 2 and 1.
2. Reminder on days 5, 6 and 7, once per day, on Application Home and over a project.
3. Refund in the transaction manager while open: lock with the refund wording. Restore purchase and Contact us work.
4. Ask to Buy: plain words, nothing granted, unlocks by itself once approved.
5. Interrupted purchase: nothing granted, buying again works.

With a sandbox Apple Account on a signed build:

6. **Rule G4, must be confirmed:** set the clock a year ahead, open the app, set it right, choose Check again. Confirm that
   `AppTransaction.refresh()` carries a fresh `signedDate` and the lock clears. If not, set
   `G4_USES_FRESH_SIGNATURE = false` in `crates/straft-entitlement/src/mac/adapter.rs` and record it in ADR-0040.
7. Confirm the zero-price item exists in App Store Connect. If no free price point exists, see ADR-0040 decision 8.
8. Reinstall: delete the app, install again. The Keychain record keeps the trial start. The purchase and flags behave.
9. Delete the app and its container. Same as above, and the data note shows again only for a buyer.
10. Clock back by a day during a trial: clock screen, cannot be closed, clears within 5 seconds of fixing the clock.
11. Clock forward, then back to the right time: covered by item 6.
12. Airplane mode on day 7: the trial still ends at its moment, and a buyer stays unlocked.
13. Refund a real sandbox purchase: `Transaction.latest(for:)` shows it and the app locks with the refund wording.
14. Second Mac, same Apple Account: the trial start comes from Apple, Restore purchase finds a purchase.
15. Second Apple Account on another Mac or macOS user: a new trial, which is accepted. On the same Mac and macOS user the
    Keychain keeps the earlier start (ADR-0040).
16. Restore purchase on a new Mac: data note, tutorial and welcome show because the local flags are missing.
17. Update: install build N, use it through a trial, install N+1 over it. Projects, settings, appearance, recents, notice
    flags, AI runtime and models and the licence state are all intact.
18. Kill the app mid-edit: the recovery dialog offers the work and nothing is lost.
19. VoiceOver reads the chip, the lock, the clock and the first-launch screens. Keyboard reaches every button.
20. Contact us from every screen opens the page in the browser. The app sends nothing.
21. Package: `xcrun altool --validate-app` passes. App Sandbox on, `PrivacyInfo.xcprivacy` present, helper signed.
