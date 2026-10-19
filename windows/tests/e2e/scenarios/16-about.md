# About card

Spec: `tests/e2e/specs/16-about.test.mjs`

Tags: `@e2e-data-root: own`, `@e2e-debug-only: sets the licence state by hand through entitlement.debug_set, which a release build never registers`

Before: an empty data folder of its own. The app is the internal test build, made without the team password, so no password screen shows.

"Set the licence to X" means: set the store channel and licence state by hand with the test-only switch of the debug build, then reload the page.

"No price" means no currency sign ($, €, £, ₹, ¥) and none of the words "price", "prices", "pay once", "subscription" or "lifetime".

## The internal build shows its version and says it is not for sale

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Quiet Pier' as the title, click Create.
2. Ask the core for the app version. Check: it starts with three numbers, like "1.2.3".
3. Open the project menu and choose Project settings.
4. Check: the About card shows "About OpenFrame Studio".
5. Check: the About card shows "Version " followed by the same version.
6. Check: the About card shows "Internal build, not for sale."
7. Check: the About card shows no price.
8. Check: there is no Buy full version button.
9. Check: the screen is quiet.

## A store build shows the version only

1. Set the licence to Microsoft Store, trial active, 7 days left.
2. Open the project menu and choose Project settings.
3. Check: the About card shows "About OpenFrame Studio" and a version.
4. Check: the About card does not show "Internal build".
5. Check: the About card shows no price.
6. Check: the screen is quiet.
7. Set the licence to direct installer, internal build.
8. Open the project menu and choose Project settings.
9. Check: the About card shows "Internal build, not for sale."

## Notes

- The About card heading is drawn in capitals by the stylesheet; the checks read the text as written.
- The team password screen is not part of this file. Rust and component tests cover it.
- This file is skipped on the release build.
