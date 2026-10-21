export default {
  "id": "moodboard",
  "available": true,
  "reason": "",
  "path": "visual",
  "order": 1,
  "feature": "moodboards",
  "title": "Make a moodboard for your film",
  "purpose": "Create a moodboard in Production so images, notes and links for the look of the film have a home.",
  "difficulty": "Beginner",
  "minutes": 3,
  "prerequisites": [
    "A project is open.",
    "You are in the Production workspace."
  ],
  "objectives": [
    "Open the Moodboards tab",
    "Name a new board",
    "See the empty board that is ready for references"
  ],
  "steps": [
    {
      "screen": "434-prod-overview",
      "action": "click",
      "target": "role=button[name=\"Moodboards\"]",
      "label": "In Production, open the Moodboards tab",
      "text": null,
      "key": null,
      "body": "Open Production from the left rail and click the Moodboards tab. The page asks \"What should this film feel like?\"."
    },
    {
      "screen": "480-prod-moodboard",
      "action": "click",
      "target": "role=button[name=\"New Moodboard\"]",
      "label": "Click New Moodboard",
      "text": null,
      "key": null,
      "body": "Click New Moodboard at the top right, next to Export. A dialog opens."
    },
    {
      "screen": "499-prod-moodboard-new-dialog",
      "action": "click",
      "target": "role=button[name=\"Overall Look\"]",
      "label": "Pick a suggested name, or type your own board name",
      "text": null,
      "key": null,
      "body": "In the dialog, \"Board name *\" takes the name. Click a suggestion chip such as Overall Look, or type your own name. \"Reference for a scene\" can stay on \"No particular scene\"."
    },
    {
      "screen": "501-prod-moodboard-new-filled",
      "action": "click",
      "target": "role=button[name=\"Create Moodboard\"]",
      "label": "Click Create Moodboard",
      "text": null,
      "key": null,
      "body": "The name is now in the box. Click Create Moodboard. If the name is empty you see \"Give the board a name.\" and nothing is created."
    },
    {
      "screen": "502-prod-moodboard-empty-canvas",
      "action": "view",
      "target": null,
      "label": "The new board opens, empty, ready for images, notes and links",
      "text": null,
      "key": null,
      "body": "The new board opens empty. The toolbar has Add image, Add note and Add link. Add image offers \"From file...\" and \"From Idea Vault...\". Right-click a board in the list on the left for Rename, Board notes, Move up, Move down and Delete."
    }
  ],
  "result": "A named moodboard that opens as an empty canvas, ready for images, notes and links. Export on the Moodboards page makes a PDF.",
  "mistakes": [
    "Clicking Create Moodboard with an empty name. Type a name or pick a chip first.",
    "Expecting the Idea Vault item to move. Add image from the Idea Vault copies the picture onto the board and leaves the vault item as it was.",
    "Looking for a Delete button on the canvas. Boards are deleted from the right-click menu in the board list."
  ],
  "troubleshooting": [
    [
      "\"Give the board a name.\" appears.",
      "The Board name field is empty. Click a chip or type a name, then click Create Moodboard again."
    ],
    [
      "The Moodboards tab is not in Production.",
      "Open a project first. Moodboards is one of the tabs inside Production, not a separate page."
    ],
    [
      "Add image \"From file...\" shows a Windows file picker.",
      "That is expected. Choose the image in the picker and it is copied into the project."
    ]
  ],
  "related": [
    "idea-vault",
    "shot-list",
    "storyboard",
    "project-tour"
  ],
  "next": "shot-list"
};
