# Closing the window keeps the last typed text

Spec: `tests/e2e/specs/04-close-keeps-typing.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. The app starts for the first time.

## Type, close the window within 100 ms by the close button and by Alt+F4, reopen, find the text

The rounds repeat 20 times by default (the runner setting `E2E_CLOSE_ROUNDS` changes the count). Each round closes the window once by its close button and once by Alt+F4.

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Last Words' as the title, click Create.
2. Check: the project opens and the window title is "Last Words - OpenFrame Studio".
3. Click Screenplay in the Workspaces navigation.
4. Click New Screenplay.
5. Check: the editor opens with at least one element.
6. Click the first element and type 'The last bus leaves at midnight.'
7. Check: the editor toolbar shows "Saved".
8. For round N from 1 to 20, and for each way of closing (the close button, then Alt+F4):
   1. Click the last element of the screenplay. Press Ctrl+End, then End, then Enter.
   2. Press Ctrl+2 so the new element is an Action.
   3. Type 'Round N ends by the close button.' (or 'Round N ends by Alt F4.' for the Alt+F4 close).
   4. Close the window by that way, within 100 ms of the last key.
   5. Check: the close was sent less than 100 ms after the last key.
   6. Check: within 15 seconds the app has ended.
   7. Start the app again on the same data.
   8. Check: Application Home shows.
   9. Click the project row "Open Last Words".
   10. Check: no recovery question shows.
   11. Click Screenplay in the Workspaces navigation.
   12. Check: a line of the screenplay ends with the sentence typed in this round.

## Notes

- On the Mac, Ctrl+End and Ctrl+2 use Cmd instead. The Alt+F4 close becomes Cmd+Q or Cmd+W; which one matches is to decide in phase M-track.

## Platform notes

- The close button and Alt+F4 are sent from outside the web view by a Windows helper (the system close command and the keyboard). Mac: the red close button and the keyboard shortcut; driving them from XCUITest is to decide in phase M-track.
