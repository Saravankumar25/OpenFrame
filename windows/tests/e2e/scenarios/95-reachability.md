# Reachability from the feature status file

Spec: `tests/e2e/specs/95-reachability.test.mjs`

Tags: none (shared journey data).

Before: the journey and relaunch scenarios have run on the shared journey data (the demo project "Night Bus"). The app starts on that data and the window is made 1440 by 900 pixels.

The routes come from `scripts/check-reachability.mjs`, which reads every feature row in `docs/implementation/feature-status.md` at Reachable or above and returns the screen routes they name, each one already checked against the source. The list therefore changes with the feature status file; this file describes how each kind of route is visited.

"Visit route R" means:

- "shell": check the Workspaces navigation shows.
- "search-overlay": press Ctrl+K, check a dialog shows, press Esc.
- "ai-panel": press Ctrl+J, check the AI Assistant opens, press Ctrl+J, check it closes.
- "project-menu": click the project button (named "Project: ..."), check a menu shows, press Esc.
- A project page (trash, activity, notes, settings): open the project menu and choose Recently Deleted, Activity, Notes & Tasks or Project settings.
- A workspace (home, vault, story, screenplay, breakdown, production, callsheets, files): click Home, Idea Vault, Story, Screenplay, Breakdown, Production, Call Sheets or Files in the Workspaces navigation.
- A route with a sub-part ("workspace/sub"): first visit the workspace or page, then click the sub-part's button:
  - production: Overview, Catalog, Locations, Cast & Crew, Moodboards, Storyboards, Shot Lists, Schedule, Daily View, Sides, Reports, Budget.
  - story: Board, Outline, Characters, Timeline, Episodes.
  - screenplay: Compare, Review.
  - vault: Recently Deleted (this one replaces the page, so check a Back to Vault button shows instead).
  - settings: Templates, Offline AI, Writing, Keyboard shortcuts, Advanced, Appearance.
  - After the click, except for screenplay sub-parts and the vault page switch, check the clicked tab is announced as current (selected or current).
- Any route with no recipe above fails the scenario, so a new route in the feature status file needs a recipe added to the spec and to this list.

After every visit, check no console errors or warnings were recorded.

## The feature status routes are valid against the source

1. Read the routes from the feature status file.
2. Check: the route check reports no errors.
3. Check: there are at least 20 routes.

## Every route opens in the real app

1. Check: the screen shows "Your projects".
2. Click the project row "Open Night Bus".
3. Check: the Workspaces navigation shows.
4. If an enabled Keep Saved Version button shows within 4 seconds, click it.
5. Check: no console errors or warnings were recorded.
6. Visit every route except "app-home" and the routes that exist only in an episodic project ("story/episodes").
7. Open the project menu and choose Close project.
8. Check: the screen shows "Your projects", and no console errors or warnings were recorded (this is the "app-home" route).
9. If the routes include "story/episodes":
   1. Click New Project.
   2. Type 'Reach Series' in the project title field.
   3. Click Episodic / Series, then click Create in the dialog.
   4. Check: the Workspaces navigation shows.
   5. Visit "story/episodes".
   6. Check: no console errors or warnings were recorded.
   7. Open the project menu and choose Close project.
   8. Check: the screen shows "Your projects".

## Notes

- On the Mac, Ctrl+K and Ctrl+J use Cmd instead.
- The episodic project "Reach Series" is created in the shared journey data and stays there.
