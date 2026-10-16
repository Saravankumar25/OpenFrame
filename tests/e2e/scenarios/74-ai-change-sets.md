# AI Change Sets: review, apply, undo, reject, out of date

Spec: `tests/e2e/specs/74-ai-change-sets.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. The app carries the real bundled model with fixed AI sampling. The app starts for the first time.

"Ask Q" is a shared step in the README of this folder. "The card" is the newest "Proposed Changes" card in the AI Assistant; "its state" is the status line under it. "Go to Tasks" means: close the AI Assistant if it is open (Close AI Assistant), open the project menu, choose Notes & Tasks, and wait for the Tasks section. "The open tasks" are the tasks listed in the Tasks section. The effects are checked in Notes & Tasks, never in the wording of the answer.

## A requested change arrives as a proposal, and nothing changes before Apply

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Night Bus' as the title, click Create.
2. Go to Tasks. Check: there are no open tasks.
3. Click Assistant.
4. Ask 'Create a task to scout the railway station.'
5. Check: the result is a Proposal.
6. Check: a Proposed Changes card shows, and it names "Notes & Tasks".
7. Check: the card offers Apply Changes and Reject.
8. Check: the card says how many changes it makes ("1 change" or "N changes"), at least one. Call that number N.
9. Go to Tasks. Check: there are still no open tasks.

## Apply makes the reviewed change; one Undo reverses all of it; Redo brings it back

1. Click Assistant.
2. Click Apply Changes on the card.
3. Check: within 20 seconds the card's state reads "Applied N change." or "Applied N changes."
4. Go to Tasks.
5. Check: there are exactly N open tasks, and each one mentions "railway station".
6. Move focus off any text field and press Ctrl+Z.
7. Check: there are no open tasks (one Undo reversed the whole Change Set).
8. Press Ctrl+Y.
9. Check: there are N open tasks again.

## Reject leaves the project unchanged

1. Go to Tasks. Note the open tasks.
2. Click Assistant.
3. Ask 'Create a task to book the bus for the night shoot.'
4. Check: the result is a Proposal.
5. Click Reject on the card.
6. Check: the card's state reads "Rejected. Nothing was changed."
7. Go to Tasks. Check: the open tasks are the same as before.

## A proposal whose target changed is never applied blindly and must be checked again

1. Go to Tasks.
2. Click New Task. Type 'Confirm pier permit' in the task title field. Click Add Task.
3. Check: the screen shows "Added task “Confirm pier permit”".
4. Click Assistant.
5. Ask 'Mark the task Confirm pier permit as done.'
6. Check: the result is a Proposal.
7. Go to Tasks.
8. Tick the box "Mark “Confirm pier permit” done" by hand.
9. Check: the screen shows "Completed task “Confirm pier permit”".
10. Click Assistant.
11. Click Apply Changes on the card.
12. Check: within 20 seconds the card is marked out of date.
13. Check: the card no longer offers Apply Changes.
14. Check: the card offers Re-check & Review.

## Notes

- On the Mac, Ctrl+Z and Ctrl+Y use Cmd instead.
- Every Ask waits up to 4 minutes.
- The AI Assistant is closed before Notes & Tasks because the docked panel sits over the right of the page, where the Tasks column is.
