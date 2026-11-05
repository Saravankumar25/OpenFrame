# Trial, purchase and first-run safety screens

Spec: `tests/e2e/specs/15-trial-first-run.test.mjs`

Tags: `@e2e-data-root: own`, `@e2e-debug-only: sets the licence state by hand through entitlement.debug_set, which a release build never registers`

Before: an empty data folder of its own. The app is the internal test build (no trial, no chip, no Buy, no popup).

"Set the licence to X" below means: set the store channel and licence state by hand with the test-only switch of the debug build, then reload the page so the app asks the core again, the way a launch would. "Reset notices" clears the record of once-only screens already shown.

"No price" means no visible text has a currency sign ($, €, £, ₹, ¥) or the words "price" or "prices". The reminder itself says "Pay once, own forever. No subscription."

## The internal build shows no chip, no Buy and no popup; a store trial shows the chip

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Night Ferry' as the title, click Create.
2. Wait about 1.5 seconds.
3. Check: there is no Free trial chip.
4. Check: there is no Buy full version button.
5. Check: no dialog is open.
6. Set the licence to Microsoft Store, trial active, 7 days left, and reset notices.
7. Check: the Free trial chip shows "Trial ends in 7 days", is not amber, and is read aloud as "Free trial. Ends in 7 days."
8. Check: a Buy full version button shows.
9. Check: no dialog is open.
10. Check: no price anywhere on the screen.
11. Check: the screen is quiet.

## The day 5 reminder shows once that day; Buy says the store is not connected

1. Set the licence to Microsoft Store, trial active, 3 days left, and reset notices.
2. Check: the "Trial reminder" dialog shows with "Your free trial ends in 3 days".
3. Check: the dialog shows no price.
4. Check: the chip shows "Trial ends in 3 days", is amber, and is read aloud as "Free trial. Ends in 3 days. Your trial ends soon."
5. Check: the dialog shows "Pay once, own forever. No subscription."
6. Check: the screen is quiet.
7. Click Continue trial in the dialog.
8. Check: the dialog closes.
9. Set the licence to Microsoft Store, trial active, 3 days left (no reset).
10. Wait about 1.5 seconds.
11. Check: no dialog is open (no second reminder on the same day).
12. Click Buy full version.
13. Check: the screen shows "The Microsoft Store connection is not available in this build."
14. Check: no dialog is open.

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
12. Check: the screen shows "Uninstalling Straft Studio removes its private data on this computer (settings and downloaded AI files), but not your projects in the root folder."
13. Check: the screen is quiet.
14. Click OK, I understand.
15. Check: the dialog "File safety tutorial" shows.
16. Press Esc.
17. Check: the tutorial is still open.
18. Click Next. Check: the screen shows "Step 2 of 4".
19. Click Next. Check: the screen shows "Step 3 of 4".
20. Click Next. Check: the screen shows "Step 4 of 4".
21. Check: Finish is disabled.
22. Check: the dialog has exactly three checkboxes. Tick all three.
23. Check: Finish becomes enabled.
24. Check: the screen is quiet.
25. Click Finish.
26. Check: the dialog "Welcome to the Straft Studio community" shows with "Be a part of our community." and eight account buttons: Instagram, YouTube, X, Facebook, LinkedIn, Reddit, Discord and Threads
27. Check: the screen is quiet.
28. Click OK in the dialog.
29. Check: the dialog closes.
30. Set the licence to Microsoft Store, purchased, again.
31. Wait about 1.5 seconds.
32. Check: no dialog is open and there is no trial chip.

## A purchase check that cannot answer can be closed, and the project stays open

1. Set the licence to Microsoft Store, needs an online check.
2. Check: the dialog "Checking your purchase" shows.
3. Check: the screen shows "Your projects stay open and can be exported. Nothing is deleted."
4. Check: the dialog has a Need help? Contact us button.
5. Click Not now in the dialog.
6. Check: the dialog closes and the Workspaces navigation shows.
7. Click Check purchase in the Licence group.
8. Check: the dialog "Checking your purchase" shows again.
9. Press Esc.
10. Check: the dialog closes.
11. Check: the screen is quiet.

## The chip and the reminder show on Application Home too

1. Set the licence to Microsoft Store, trial active, 3 days left, and reset notices.
2. Check: the "Trial reminder" dialog shows. Click Continue trial.
3. Close the project through the project menu. Check: Application Home shows.
4. Check: the Free trial chip shows "Trial ends in 3 days", is amber, and is read aloud as "Free trial. Ends in 3 days. Your trial ends soon."
5. Check: a Buy full version button shows.
6. Set the licence to Microsoft Store, trial active, 2 days left, and reset notices, with Application Home open.
7. Check: the "Trial reminder" dialog shows on Application Home with "Your free trial ends in 2 days".
8. Check: the screen is quiet.
9. Click Continue trial. Check: the dialog closes.
10. Open the project "Night Ferry" again.

## The lock keeps text typed just before it, and has the contact link

1. Set the licence to Microsoft Store, trial active, 7 days left.
2. Open the Story workspace, click the Story page and type 'Typed just before the lock.' without waiting.
3. Without a reload, change the licence to trial ended (as a store check does while the app is open).
4. Check: the dialog "Your free trial has ended" shows with a Need help? Contact us button.
5. Wait about 1.5 seconds.
6. Without a reload, change the licence back to trial active, 7 days left.
7. Check: the dialog closes.
8. Ask the core for the Story page. Check: its text contains 'Typed just before the lock.'
9. Check: the screen is quiet.

## Mac App Store: an ended trial locks the app with Buy full version, Restore purchase and the contact link

1. Set the licence to Mac App Store, trial ended. Do not wait for the project screen.
2. Check: the dialog "Your free trial has ended" shows.
3. Check: the dialog has exactly three buttons, in order: Buy full version, Restore purchase, Need help? Contact us.
4. Press Esc, then press Ctrl+K.
5. Check: the only open dialog is still "Your free trial has ended".
6. Ask the core for the current project directly, below the screen.
7. Check: the core refuses with the error "permission.licence_required".
8. Check: the dialog shows no price.
9. Click Buy full version in the dialog.
10. Check: the screen shows "The App Store connection is not available in this build."

## Microsoft Store: an ended trial locks the app with Buy full version, Check again and the contact link

1. Set the licence to Microsoft Store, trial ended. Do not wait for the project screen.
2. Check: the dialog "Your free trial has ended" shows.
3. Check: the dialog has exactly three buttons, in order: Buy full version, Check again, Need help? Contact us.
4. Press Esc, then press Ctrl+K.
5. Check: the only open dialog is still "Your free trial has ended".
6. Ask the core for the current project directly, below the screen.
7. Check: the core refuses with the error "permission.licence_required".
8. Check: the dialog shows no price.
9. Click Buy full version in the dialog.
10. Check: the screen shows "The Microsoft Store connection is not available in this build."

## The Mac first launch screen cannot be closed and shows no price

1. Set the licence to Mac App Store, trial not started.
2. Check: the dialog "Welcome to Straft Studio" shows.
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
