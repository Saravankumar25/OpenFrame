export default {
  "id": "project-tour",
  "available": true,
  "reason": "",
  "path": "getting-started",
  "order": 3,
  "feature": null,
  "title": "Find your way around a project",
  "purpose": "Use the sidebar, the project menu, the Create menu and search to move around a project.",
  "difficulty": "Beginner",
  "minutes": 4,
  "prerequisites": [
    "A project is open on its Home page."
  ],
  "objectives": [
    "Name the sidebar sections",
    "Open the project menu",
    "Create anything from the + button",
    "Search the whole project with Ctrl+K"
  ],
  "steps": [
    {
      "screen": "013-shell-project-home",
      "action": "click",
      "target": "button[aria-label=\"Project: Night Bus. Switch or manage projects\"]",
      "label": "The project name at the top opens the project menu",
      "text": null,
      "key": null,
      "body": "The sidebar on the left lists Home, Idea Vault, Story, Screenplay, Breakdown, Production, Call Sheets and Files. Click the project name at the top to open the project menu."
    },
    {
      "screen": "014-shell-project-menu",
      "action": "click",
      "target": "button[aria-label=\"Create\"]",
      "label": "The + button creates anything: ideas, acts, scene cards, characters and more",
      "text": null,
      "key": null,
      "body": "The project menu holds your recent projects, Undo and Redo, Project settings, Notes & Tasks, Activity, Recently Deleted, Templates and Appearance. Next to the search box, click the + button to create something."
    },
    {
      "screen": "015-shell-create-menu",
      "action": "click",
      "target": "button[aria-label=\"Search this project\"]",
      "label": "Search the whole project from the top bar (Ctrl+K)",
      "text": null,
      "key": null,
      "body": "The CREATE menu starts new items: New Idea, New Act, New Sequence, New Beat, New Scene Card, New Character, New Location, New Shot List, New Moodboard and more. Now click the search box at the top, or press Ctrl+K."
    },
    {
      "screen": "018-shell-search-empty",
      "action": "type",
      "target": "input[aria-label=\"Search\"]",
      "label": "Type a name or word",
      "text": "Lakshmi",
      "key": null,
      "body": "In the Search dialog, type a name or word. The placeholder lists what it covers: scenes, ideas, characters, locations and files. Use the scope to search This project, or add the Global Idea Vault."
    },
    {
      "screen": "019-shell-search-results",
      "action": "view",
      "target": null,
      "label": "Results from every section, each labelled with its type",
      "text": null,
      "key": null,
      "body": "Results come from every section, and each result is labelled with its type. Press Esc to close the search."
    }
  ],
  "result": "You can reach any part of a project from the sidebar, the project menu, the + button or Ctrl+K search.",
  "mistakes": [
    "Looking for a separate command palette. OpenFrame has Ctrl+K search instead.",
    "Hunting through sections for something you just made. Search the project by name and open the result."
  ],
  "troubleshooting": [
    [
      "Search finds nothing",
      "Check the scope. \"This project\" does not include the Global Idea Vault unless you add it."
    ],
    [
      "The sidebar is only icons",
      "The sidebar is collapsed. Use Collapse at the bottom of the sidebar to switch it back."
    ]
  ],
  "related": [
    "create-project",
    "manage-projects",
    "idea-vault",
    "recently-deleted"
  ],
  "next": "manage-projects"
};
