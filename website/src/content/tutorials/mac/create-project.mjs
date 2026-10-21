export default {
  "id": "create-project",
  "available": true,
  "reason": "",
  "path": "getting-started",
  "order": 2,
  "feature": null,
  "title": "Create your first project",
  "purpose": "Create a new project from Application Home and open its Project Home.",
  "difficulty": "Beginner",
  "minutes": 3,
  "prerequisites": [
    "OpenFrame Studio is open on Application Home (see Start OpenFrame Studio for the first time)."
  ],
  "objectives": [
    "Open the New Project sheet",
    "Give your project a title",
    "Create it and land on Project Home"
  ],
  "steps": [
    {
      "screen": "003-home-empty",
      "action": "click",
      "target": "[data-id=\"home-new-project\"]",
      "label": "Click Create Project",
      "text": null,
      "key": null,
      "body": "On Application Home, click Create Project. With projects already listed, the button is called New Project in the toolbar, and File > New Project... (Cmd+N) does the same."
    },
    {
      "screen": "011-new-project-sheet",
      "action": "type",
      "target": "[data-id=\"new-project-title\"]",
      "label": "Type a title for the film",
      "text": "Night Bus",
      "key": null,
      "body": "The New Project sheet slides down over Application Home. Type a title for your film in the Title field. Type, Genre, Written by and Location are optional."
    },
    {
      "screen": "012-new-project-filled",
      "action": "click",
      "target": "[data-id=\"new-project-create\"]",
      "label": "Click Create",
      "text": null,
      "key": null,
      "body": "Click Create. It stays off until the sheet has a title."
    },
    {
      "screen": "060-project-home-empty",
      "action": "view",
      "target": null,
      "label": "The new project opens on its Project Home",
      "text": null,
      "key": null,
      "body": "Your new project opens on its Project Home. Three starting cards help you begin: Open Idea Vault, Build Story and Write/Import Screenplay. The sidebar says \"Project stored locally on this Mac\"."
    }
  ],
  "result": "A new project exists on this Mac and is open on its Project Home.",
  "mistakes": [
    "Looking for a dialog. On the Mac the New Project form is a sheet that slides down from the window.",
    "Clicking Create before typing a title. The button stays off until the Title field has text.",
    "Choosing a location you cannot find later. Leave Location as it is, or use Choose... and note the folder."
  ],
  "troubleshooting": [
    [
      "Create is greyed out.",
      "Type a title in the Title field first."
    ],
    [
      "I cannot find the Create Project button.",
      "It shows only when no projects exist. Otherwise use New Project in the toolbar or Cmd+N on Application Home."
    ],
    [
      "My project is not where I expected.",
      "Your work is stored on this Mac. On an empty Application Home, Show in Finder opens the projects folder."
    ]
  ],
  "related": [
    "first-launch",
    "project-tour",
    "manage-projects"
  ],
  "next": "project-tour"
};
