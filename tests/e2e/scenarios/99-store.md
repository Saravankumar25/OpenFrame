# 99-store: trial, purchase, lock and clock screens (Mac App Store build)

Runs over a scripted App Store (`ScriptedStoreKitSource`, debug builds on a test folder only). Real StoreKit needs a signed
app: see `docs/release/mac-store-test.md`. Mocks 187 to 201 in the release package.

| Case | What it proves |
|---|---|
| `first-launch-trial-chip-on-home-and-project-then-buy` | The first launch states the trial, what stops at the end and the price from the store; it cannot be closed; Contact us opens the address; the chip and Buy show on Home and in a project; buying removes them at once; note, tutorial and welcome once and in order, never after a restart |
| `reminder-on-home-on-day-five-once-per-day` | No reminder on day 4; on day 5 it shows on Home with the real end moment; not again the same day |
| `trial-ends-while-open-keeps-unsaved-text-and-locks` | Text typed just before the end is stored, the lock cannot be closed, Restore keeps it, Buy lifts it and the text is there |
| `refund-while-open-locks-with-the-refund-words` | A refund while open locks with the refund wording |
| `clock-turned-back-locks-and-clears-by-itself` | A clock turned back locks, cannot be closed and clears by itself once the clock is right |
| `first-launch-offline-shows-the-check-screen` | No store on first launch: the check screen, no trial offered, Try again finds the store |
| `offline-relaunch-keeps-the-trial-and-the-buyer` | A known trial keeps counting and a buyer stays unlocked with no store |
