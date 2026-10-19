# Appearance and themes

Spec: `tests/e2e/specs/60-appearance.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. The app starts for the first time, and the window is made 1440 by 900 pixels.

"The theme", "the mode" and "the preference" are the active theme id, the mode drawn (light or dark) and the chosen mode (light, dark or system), as the app root reports them. "The background colour" is the colour the window background is really drawn in.

The theme to paste ("Harbour Night"):

```json
{"formatVersion":1,"id":"harbour-night","name":"Harbour Night","description":"Blue-grey surfaces with a lamp-orange accent.","author":"E2E","seeds":{"neutralHue":230,"neutralTint":0.02,"accentHue":55,"accentChroma":0.14,"accentLightnessNudge":0}}
```

The hostile theme to paste (a name that is markup, and a colour that tries to break out of its rule):

```json
{"formatVersion":1,"id":"evil-theme","name":"<img src=x onerror=alert(1)>","seeds":{"neutralHue":230,"neutralTint":0.02,"accentHue":55,"accentChroma":0.14,"accentLightnessNudge":0},"overrides":{"light":{"bg":"#fff} body{display:none"}}}
```

"Paste theme X" means: click Paste Theme…, paste X into the paste box, then click Check Theme in the dialog.

All steps below are one scenario.

## Mode, theme, import, rename, restart and delete

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Harbour Lights' as the title, click Create.
2. Check: the theme is "openframe-default" and the preference is "system".
3. Check: the top bar shows the OpenFrame logo.
4. Open the project menu and choose Appearance.
5. Check: the "Theme" choice shows.
6. Click Dark.
7. Check: the mode is dark, and each part of the background colour (red, green, blue) is below 80.
8. Click Light.
9. Check: the mode is light, and the background colour is red 243, green 241, blue 236.
10. Check: the screen is quiet.
11. Click the theme card that starts with "Midnight Reel".
12. Check: the theme is "midnight-reel" and that card is selected.
13. Paste theme the hostile theme.
14. Check: the screen shows "This theme cannot be used. Nothing was changed."
15. Check: Save and Apply is disabled.
16. Click Cancel in the dialog.
17. Check: the theme is still "midnight-reel" and the page body is still shown.
18. Paste theme Harbour Night.
19. Check: the screen shows "This theme is readable in Light and Dark."
20. Check: a "Dark preview" shows.
21. Click Save and Apply in the dialog.
22. Check: the theme is "harbour-night".
23. Check: My Themes lists "Harbour Night".
24. Click Rename Harbour Night.
25. Clear the name field and type 'Harbour Lamps'.
26. Click Rename in the dialog.
27. Check: My Themes lists "Harbour Lamps".
28. Click Dark. Check: the mode is dark.
29. Open the project menu and choose Close project.
30. Quit the app and start it again on the same data. Make the window 1440 by 900 pixels.
31. Check: the theme is "harbour-night" and the mode is dark.
32. On Application Home, click Appearance.
33. Check: a dialog with the "Theme" choice shows.
34. Click Delete Harbour Lamps.
35. Click Delete Theme in the dialog.
36. Check: the theme is "openframe-default" at once.
37. Click System.
38. Check: the preference is "system".
39. Click Done in the dialog.
40. Check: the screen is quiet.

## Platform notes

- System mode follows the Windows light or dark setting. Mac: the macOS appearance setting.
