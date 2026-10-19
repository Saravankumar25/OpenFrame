# Relaunch keeps the work

Spec: `tests/e2e/specs/02-restart.test.mjs`

Tags: none (shared journey data).

Before: `01-journey` has run on the shared journey data, ending with "Night Bus" closed cleanly. The app is started again on the same data.

## Reopen the project and find the screenplay text, the story and the call sheet

1. Start the app.
2. Check: the screen shows "Your projects".
3. Click the project row whose name starts with "Open Night Bus".
4. Check: the project opens with the Workspaces navigation.
5. Check: after about 1 second, no recovery question shows (no "recovery state" text), because the last session closed cleanly.
6. Press Ctrl+K and type 'there that night'.
7. Check: a result containing "INT. POLICE STATION" shows.
8. Click that result.
9. Check: the screenplay contains the line "I was there that night."
10. Click Story in the Workspaces navigation.
11. Check: the board shows exactly two scene cards.
12. Click Call Sheets in the Workspaces navigation.
13. Check: the screen shows "Day 1".
14. Check: no console errors or warnings were recorded.

## Notes

- On the Mac, Ctrl+K is Cmd+K.
