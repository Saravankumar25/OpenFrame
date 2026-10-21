export default {
  "id": "project-tour",
  "available": true,
  "reason": "",
  "path": "getting-started",
  "order": 3,
  "feature": null,
  "title": "Find your way around a project",
  "purpose": "Learn the sidebar and use Search to jump to a scene, a command or a section.",
  "difficulty": "Beginner",
  "minutes": 4,
  "prerequisites": [
    "A project is open. Search finds scenes best once the project has a screenplay."
  ],
  "objectives": [
    "Return to Project Home from any section",
    "Open Search with Cmd+K",
    "Jump to a scene by typing a word from it"
  ],
  "steps": [
    {
      "screen": "034-home-import-toast",
      "action": "click",
      "target": "[data-id=\"sidebar-home\"]",
      "label": "Click Home in the sidebar",
      "text": null,
      "key": null,
      "body": "The sidebar lists Home, Idea Vault, Story, Screenplay, Breakdown, Production, Call Sheets and Files. Click Home to return to Project Home from any section."
    },
    {
      "screen": "035-project-home",
      "action": "click",
      "target": "[data-id=\"project-search-button\"]",
      "label": "Click Search, or press Cmd+K",
      "text": null,
      "key": null,
      "body": "Project Home shows QUICK ACCESS, Project Files and RECENT. Click Search in the toolbar, or press Cmd+K, to open the \"Search and commands\" palette."
    },
    {
      "screen": "051-search-palette-project",
      "action": "type",
      "target": "[data-id=\"palette-field\"]",
      "label": "Type a name or a word from the script",
      "text": "kiran",
      "key": null,
      "body": "Type a name or a word from the script into the palette field. Empty, the palette lists commands such as \"Go to Story\". Typing a character name lists the scenes that mention it."
    },
    {
      "screen": "052-search-palette-results",
      "action": "view",
      "target": null,
      "label": "Matching scenes and commands appear; pick one to jump there",
      "text": null,
      "key": null,
      "body": "Matching scenes and commands appear. Pick one to jump there. Press Esc to close the palette without choosing."
    }
  ],
  "result": "You can move between sections from the sidebar and jump anywhere with Search.",
  "mistakes": [
    "Searching only for commands. The palette also finds words inside your scenes.",
    "Hunting the menu bar for navigation. The Go menu lists every section, but the sidebar is quicker.",
    "Forgetting the shortcut list. Help > Keyboard Shortcuts shows every shortcut."
  ],
  "troubleshooting": [
    [
      "Search shows no scenes.",
      "The project may have no screenplay yet, or the word is not in it. Try a shorter word."
    ],
    [
      "The sidebar is gone.",
      "Click Hide Sidebar in the toolbar again to bring it back."
    ],
    [
      "I am lost in a section.",
      "Click Home in the sidebar, or All Projects in the toolbar to return to Application Home."
    ]
  ],
  "related": [
    "create-project",
    "manage-projects",
    "settings"
  ],
  "next": "manage-projects"
};
