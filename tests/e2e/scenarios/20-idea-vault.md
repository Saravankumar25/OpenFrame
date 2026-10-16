# Idea Vault basics

Spec: `tests/e2e/specs/20-idea-vault.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. The app starts for the first time.

"The Vault cards" means the cards in the list "Idea Vault items", read by their accessible names.

## Quick Capture adds an idea to the project's Vault

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Lantern Coast' as the title, click Create.
2. Click Idea Vault in the Workspaces navigation.
3. Click Quick Capture.
4. Type 'A lighthouse keeper collects letters that never arrived.' in the Idea field.
5. Click Save in the dialog.
6. Check: the screen shows "Saved to the Project Idea Vault".
7. Check: the Vault has exactly one card, and its name starts with "A lighthouse keeper".
8. Check: the screen is quiet.

## Give the idea a title in its drawer, then undo and redo

1. Note the name of the one card.
2. Double-click the card.
3. Check: the drawer opens with its title field.
4. Type 'Lighthouse keeper' in the title field.
5. Check: the screen shows "Saved automatically".
6. Click Close in the drawer. Check: the drawer closes.
7. Check: a Vault card is named "Lighthouse keeper".
8. Move focus off any text field and press Ctrl+Z.
9. Check: the card has its earlier name again.
10. Press Ctrl+Y.
11. Check: a card is named "Lighthouse keeper" again.
12. Check: the screen is quiet.

## Delete with the keyboard, undo, redo, then restore from Recently Deleted

1. Click the card "Lighthouse keeper".
2. Press Delete.
3. Check: the screen shows "Deleted “Lighthouse keeper”. You can restore from Recently Deleted."
4. Check: the Vault has no cards.
5. Press Ctrl+Z.
6. Check: the card "Lighthouse keeper" is back.
7. Press Ctrl+Y.
8. Check: the Vault has no cards.
9. Click Recently Deleted.
10. Check: the screen shows "Items you deleted can be restored."
11. Click Restore in the table row that contains "Lighthouse keeper".
12. Click Back to Vault.
13. Check: the card "Lighthouse keeper" is back.
14. Check: the screen is quiet.

## After a relaunch, the titled idea and its text are kept

1. Open the project menu and choose Close project.
2. Quit the app and start it again on the same data.
3. Click the project row "Open Lantern Coast". Check: no recovery question shows.
4. Click Idea Vault in the Workspaces navigation.
5. Check: the card "Lighthouse keeper" shows.
6. Double-click the card.
7. Check: the body field reads exactly "A lighthouse keeper collects letters that never arrived."
8. Click Close in the drawer.
9. Check: the screen is quiet.
10. Open the project menu and choose Close project.

## Notes

- On the Mac, Ctrl+Z and Ctrl+Y use Cmd instead.
