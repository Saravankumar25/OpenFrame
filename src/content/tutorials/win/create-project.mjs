export default {
  "id": "create-project",
  "available": true,
  "reason": "",
  "path": "getting-started",
  "order": 2,
  "feature": null,
  "title": "Create your first project",
  "purpose": "Create a project with a title and a type, and open its Home page.",
  "difficulty": "Beginner",
  "minutes": 3,
  "prerequisites": [
    "OpenFrame Studio is open on Application Home."
  ],
  "objectives": [
    "Open the New Project dialog",
    "Choose Feature Film or Short Film",
    "Know what the optional details are",
    "Open the new project"
  ],
  "steps": [
    {
      "screen": "003-app-home-empty",
      "action": "click",
      "target": "role=button[name=\"Create Project\"]",
      "label": "Click Create Project",
      "text": null,
      "key": null,
      "body": "On Application Home, click Create Project. The New Project dialog opens and asks \"What is this project called and what type is it?\"."
    },
    {
      "screen": "004-app-new-project-dialog",
      "action": "type",
      "target": "#np-title",
      "label": "Type a title and choose Feature Film or Short Film",
      "text": "Empty Short",
      "key": null,
      "body": "Type a title in the \"Title *\" box. Under \"Type *\" choose Feature Film (the default) or Short Film; this example uses Short Film."
    },
    {
      "screen": "006-app-new-project-filled",
      "action": "click",
      "target": "role=dialog >> summary",
      "label": "Optional details are there if you want them",
      "text": null,
      "key": null,
      "body": "Click OPTIONAL DETAILS if you want more. It opens Language, Genre, Creator and the folder the project is saved in, with a \"Change…\" button for the folder."
    },
    {
      "screen": "007-app-new-project-optional-details",
      "action": "click",
      "target": "role=dialog >> role=button[name=\"Create\"s]",
      "label": "Click Create",
      "text": null,
      "key": null,
      "body": "Click Create. If the title is empty you see \"A title is required.\" and nothing is made."
    },
    {
      "screen": "008-home-new-project",
      "action": "view",
      "target": null,
      "label": "The new project opens on its Home page",
      "text": null,
      "key": null,
      "body": "The new project opens on its Home page, which says \"Start with your ideas, build your story, or open a screenplay.\" The cards offer Open Idea Vault, Build Story and Write/Import Screenplay."
    }
  ],
  "result": "A new project, saved in the folder you chose, open on its Home page.",
  "mistakes": [
    "Skipping the title. The Title field is required and the dialog shows \"A title is required.\" until you fill it.",
    "Opening the optional details and forgetting the folder. Check \"Saved in\" and use \"Change…\" if you want a different place."
  ],
  "troubleshooting": [
    [
      "I do not see Create Project",
      "That button is on the empty Application Home. Once you have projects, use New Project (Ctrl+N) instead."
    ],
    [
      "I picked the wrong type",
      "Click Cancel and start again. The type is chosen in this dialog, so check Feature Film or Short Film before you click Create."
    ]
  ],
  "related": [
    "first-launch",
    "project-tour",
    "manage-projects"
  ],
  "next": "project-tour"
};
