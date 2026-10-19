# Every built-in theme in Light and Dark

Spec: `tests/e2e/specs/91-appearance-themes.test.mjs`

Tags: none (shared journey data).

Before: `90-visual-baseline` has run on the shared journey data (the demo project "Night Bus"). The app starts on that data and the window is made 1440 by 900 pixels. At the end the appearance is set to OpenFrame Default, Light, and then to System mode, so later scenarios see the default look.

The 16 built-in themes (from the app's theme list): OpenFrame Default, Studio Amber, Slate Noir, Midnight Reel, Teal & Orange, Courier & Cream, Cutting Room, Velvet Curtain, Sunset Boulevard, Arctic Frame, Neon Reel, Location Scout, Desert Dune, Colorist Neutral, Silver Screen, High Contrast.

"Set the appearance to T, M" means: set theme T and mode M through the same operations Settings, Appearance uses, then check the app root reports theme T and mode M.

"Walk the 15 screens" means, with the same names as `90-visual-baseline`:

1. Check: the screen shows "Your projects". This is 01-application-home.
2. Click the project row "Open Night Bus". If an enabled Keep Saved Version button shows within 2.5 seconds, click it.
3. Click Home (02-project-home), Idea Vault (03-idea-vault), Story (04-story-board), then the Outline tab (05-story-outline), Screenplay (06-screenplay-editor), Breakdown (07-breakdown), Production (08-production-overview), then the Catalog tab (09-production-catalog) and the Schedule tab (10-production-schedule), Call Sheets (11-call-sheets), Files (12-files).
4. Open the project menu and choose Notes & Tasks (13-notes-and-tasks), then Recently Deleted (14-recently-deleted), then Project settings (15-project-settings).
5. Open the project menu and choose Close project. Check: the screen shows "Your projects".

"Check the screen is readable" means, after waiting about 0.5 seconds on a screen:

1. Check: the app root reports the mode that was set.
2. Check: the window background is a solid colour, not transparent.
3. Check: every visible text element (headings, paragraphs, muted text, buttons, chips, navigation, tabs, labels, table cells, cards) has a contrast of at least 4.5 to 1 against the colour drawn behind it, or 3 to 1 for large text (24 pixels, or 18.66 pixels and bold). Disabled controls, toasts, images, hidden parts and the screenplay and call sheet paper are skipped.
4. Check: the screenplay page and the call sheet paper are white with black text.
5. Save a screenshot for review to `tests/e2e/artifacts/themes/<theme>-<mode>/<screen>.png`.

## 16 themes in 2 modes on 15 screens are applied and readable; the paper stays white

1. For each of the 16 themes, and for Light and then Dark:
   1. Set the appearance to that theme and mode.
   2. Walk the 15 screens. On each screen, check the screen is readable.
2. Check: 480 screens were checked (16 themes, 2 modes, 15 screens).
3. Check: no readability problem was found, apart from the known exceptions of OpenFrame Default in Light mode: muted text, accent buttons and accent text (the owner chose to keep the default look as it is). Those exceptions are allowed only for that theme and mode.

## The theme and mode come back after a relaunch

1. Set the appearance to Velvet Curtain, Dark.
2. Note the window background colour.
3. Quit the app, wait about 1 second, and start it again on the same data. Make the window 1440 by 900 pixels.
4. Check: the screen shows "Your projects".
5. Check: the app root reports theme "velvet-curtain" and mode dark.
6. Check: the window background colour is the same as before the relaunch.

## Notes

- The appearance is set through the core, as Settings does, not by clicking the Settings screen. `60-appearance` covers the clicks.

## Platform notes

- The colours are read from the WebView2 page. Mac: the SwiftUI shell draws natively, so the contrast check needs another way to read rendered colours; to decide in phase M-track.
