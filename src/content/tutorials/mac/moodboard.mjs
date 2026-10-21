export default {
  "id": "moodboard",
  "available": true,
  "reason": "",
  "path": "visual",
  "order": 1,
  "feature": "moodboards",
  "title": "Build a moodboard for the film",
  "purpose": "Create a moodboard and fill it with notes and links that say what the film should feel like.",
  "difficulty": "Beginner",
  "minutes": 6,
  "prerequisites": [
    "A project is open.",
    "You know where the Production view is in the sidebar."
  ],
  "objectives": [
    "Create a named moodboard",
    "Add a note and mark it internal",
    "Select and edit a tile in the inspector"
  ],
  "steps": [
    {
      "screen": "517-plan-moodboard-new-sheet",
      "action": "click",
      "target": "[data-id=\"moodboard-suggestion-Overall Look\"]",
      "label": "In Production, Moodboards, click New Moodboard. In the sheet, click the suggestion Overall Look",
      "text": null,
      "key": null,
      "body": "Open Production in the sidebar, use the section pop-up at the top to choose Moodboards, then click New Moodboard. A sheet drops down from the window title bar. Click the suggestion Overall Look, or type any name in Board name."
    },
    {
      "screen": "519-plan-moodboard-new-suggestion",
      "action": "click",
      "target": "[data-id=\"moodboard-new-sheet-primary\"]",
      "label": "Click Create Moodboard",
      "text": null,
      "key": null,
      "body": "Leave Reference for a scene on No particular scene unless the board belongs to one scene. Click Create Moodboard. A message confirms the new board and offers Undo."
    },
    {
      "screen": "520-plan-moodboard-canvas-empty",
      "action": "click",
      "target": "[data-id=\"moodboard-add-note\"]",
      "label": "The empty board opens. Click Add Note",
      "text": null,
      "key": null,
      "body": "The empty board opens with Add Image, Add Note, Add Link and Export in its toolbar. Click Add Note."
    },
    {
      "screen": "523-plan-moodboard-note-sheet",
      "action": "type",
      "target": "[data-id=\"new-note\"]",
      "label": "Type the note",
      "text": "Faces lit only by phone screens",
      "key": null,
      "body": "The Add note sheet opens. Type your note in the Note field."
    },
    {
      "screen": "525-plan-moodboard-note-private",
      "action": "click",
      "target": "[data-id=\"moodboard-note-sheet-primary\"]",
      "label": "Click Add Note",
      "text": null,
      "key": null,
      "body": "Tick the check box \"Internal note, left out of exports unless selected\" if the note is only for the team, then click Add Note."
    },
    {
      "screen": "526-plan-moodboard-canvas-items",
      "action": "click",
      "target": "[data-id=\"moodboard-tile-01a15bae-5857-779e-81b6-74d49a50902f\"]",
      "label": "Notes and links sit on the canvas. Select a tile to edit it",
      "text": null,
      "key": null,
      "body": "Your notes and links sit on the canvas as tiles. Drag to move a tile, Shift-click to select several, and use the arrow keys to nudge. Click one tile to select it."
    },
    {
      "screen": "527-plan-moodboard-inspector",
      "action": "view",
      "target": null,
      "label": "The item opens in the inspector, with its note, the internal note switch and a caption",
      "text": null,
      "key": null,
      "body": "The inspector shows MOODBOARD ITEM with the note, the internal note check box and a Caption. Edit them here. When the board is ready, click Export to make a PDF."
    }
  ],
  "result": "A named moodboard with notes on its canvas, ready to export as a PDF.",
  "mistakes": [
    "Leaving the internal note box unticked on a private note. Tick it, or the note can appear in the export.",
    "Closing the New Moodboard sheet without clicking Create Moodboard. No board is made until you click it."
  ],
  "troubleshooting": [
    [
      "I cannot find New Moodboard.",
      "Open Production in the sidebar, then choose Moodboards from the section pop-up at the top. Go > Production Tab also reaches it."
    ],
    [
      "I made a board by mistake.",
      "Click Undo in the message that appears right after you create it."
    ]
  ],
  "related": [
    "shot-list",
    "storyboard",
    "idea-vault"
  ],
  "next": "shot-list"
};
