# Trial, purchase and first-run safety screens

Spec: `tests/e2e/specs/15-trial-first-run.test.mjs`

Tags: `@e2e-data-root: own`, `@e2e-debug-only: sets the licence state by hand through entitlement.debug_set, which a release build never registers`

Before: an empty data folder of its own. The app is the internal test build (no trial, no chip, no Buy, no popup).

"Set the licence to X" below means: set the store channel and licence state by hand with the test-only switch of the debug build, then reload the page so the app asks the core again, the way a launch would. "Reset notices" clears the record of once-only screens already shown.

"No price" means no visible text has a currency sign ($, €, £, ₹, ¥) or the words "price", "prices", "pay once", "subscription" or "lifetime".

## The internal build shows no chip, no Buy and no popup; a store trial shows the chip

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Night Ferry' as the title, click Create.
2. Wait about 1.5 seconds.
3. Check: there is no Free trial chip.
4. Check: there is no Buy full version button.
5. Check: no dialog is open.
6. Set the licence to Microsoft Store, trial active, 7 days left, and reset notices.
7. Check: the Free trial chip shows "Trial: 7 days left", is not amber, and is read aloud as "Free trial, 7 days left."
8. Check: a Buy full version button shows.
9. Check: no dialog is open.
10. Check: no price anywhere on the screen.
11. Check: the screen is quiet.

## The day 5 reminder shows once that day; Buy says the store is not connected

1. Set the licence to Microsoft Store, trial active, 3 days left, and reset notices.
2. Check: the "Trial reminder" dialog shows with "Your free trial ends in 3 days".
3. Check: the dialog shows no price.
4. Check: the chip shows "Trial: 3 days left", is amber, and is read aloud as "Free trial, 3 days left. Your trial ends soon."
5. Check: the screen is quiet.
6. Click Continue trial in the dialog.
7. Check: the dialog closes.
8. Set the licence to Microsoft Store, trial active, 3 days left (no reset).
9. Wait about 1.5 seconds.
10. Check: no dialog is open (no second reminder on the same day).
11. Click Buy full version.
12. Check: the screen shows "The Microsoft Store connection is not available in this build."
13. Check: no dialog is open.

## After a purchase: the data note, the 4-step tutorial and the welcome show once, work by keyboard, and cannot be skipped

1. Set the licence to Microsoft Store, purchased.
2. Check: the dialog "Your data, your responsibility" shows.
3. Check: the trial chip is gone.
4. Check: OK, I understand is disabled.
5. Press Esc.
6. Check: the data note is still open.
7. Click the region "Six points about your data" and press End to read to the end.
8. Check: the checkbox in the dialog becomes enabled.
9. Tick the checkbox.
10. Check: OK, I understand becomes enabled.
11. Check: the screen shows "Show in File Explorer".
12. Check: the screen is quiet.
13. Click OK, I understand.
14. Check: the dialog "File safety tutorial" shows.
15. Press Esc.
16. Check: the tutorial is still open.
17. Click Next. Check: the screen shows "Step 2 of 4".
18. Click Next. Check: the screen shows "Step 3 of 4".
19. Click Next. Check: the screen shows "Step 4 of 4".
20. Check: Finish is disabled.
21. Check: the dialog has exactly three checkboxes. Tick all three.
22. Check: Finish becomes enabled.
23. Check: the screen is quiet.
24. Click Finish.
25. Check: the dialog "Welcome to the OpenFrame Studio family" shows with "Be a part of our community."
26. Check: the screen is quiet.
27. Click OK in the dialog.
28. Check: the dialog closes.
29. Set the licence to Microsoft Store, purchased, again.
30. Wait about 1.5 seconds.
31. Check: no dialog is open and there is no trial chip.

## A purchase check that cannot answer can be closed, and the project stays open

1. Set the licence to Microsoft Store, needs an online check.
2. Check: the dialog "Checking your purchase" shows.
3. Check: the screen shows "Your projects stay open and can be exported. Nothing is deleted."
4. Click Not now in the dialog.
5. Check: the dialog closes and the Workspaces navigation shows.
6. Click Check purchase in the Licence group.
7. Check: the dialog "Checking your purchase" shows again.
8. Press Esc.
9. Check: the dialog closes.
10. Check: the screen is quiet.

## Mac App Store: an ended trial locks the app with only Buy full version and Restore purchase

1. Set the licence to Mac App Store, trial ended. Do not wait for the project screen.
2. Check: the dialog "Your free trial has ended" shows.
3. Check: the dialog has exactly two buttons, in order: Buy full version, Restore purchase.
4. Press Esc, then press Ctrl+K.
5. Check: the only open dialog is still "Your free trial has ended".
6. Ask the core for the current project directly, below the screen.
7. Check: the core refuses with the error "permission.licence_required".
8. Check: the dialog shows no price.
9. Click Buy full version in the dialog.
10. Check: the screen shows "The App Store connection is not available in this build."

## Microsoft Store: an ended trial locks the app with only Buy full version and Restore purchase

1. Set the licence to Microsoft Store, trial ended. Do not wait for the project screen.
2. Check: the dialog "Your free trial has ended" shows.
3. Check: the dialog has exactly two buttons, in order: Buy full version, Restore purchase.
4. Press Esc, then press Ctrl+K.
5. Check: the only open dialog is still "Your free trial has ended".
6. Ask the core for the current project directly, below the screen.
7. Check: the core refuses with the error "permission.licence_required".
8. Check: the dialog shows no price.
9. Click Buy full version in the dialog.
10. Check: the screen shows "The Microsoft Store connection is not available in this build."

## The Mac first launch screen cannot be closed and shows no price

1. Set the licence to Mac App Store, trial not started.
2. Check: the dialog "Welcome to OpenFrame Studio" shows.
3. Check: the screen shows "7 days. Everything included. No credit card."
4. Check: no price anywhere on the screen.
5. Press Esc.
6. Check: the dialog is still open.
7. Check: the screen is quiet.
8. Set the licence to direct installer, internal build, so the app is left as the internal build.

## Notes

- On the Mac, Ctrl+K is Cmd+K.
- The stores' own purchase windows are not tested; there is no store behind these states.
- This file is skipped on the release build, which refuses the hand-set licence switch.

## Platform notes

- "Show in File Explorer" is the Windows wording in the data note. Mac: Finder; the label is to decide in phase M-track.
- How XCUITest sets the licence states by hand is to decide in phase M-track.
