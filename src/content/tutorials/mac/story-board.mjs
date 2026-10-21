export default {
  "id": "story-board",
  "available": true,
  "reason": "",
  "path": "writing",
  "order": 2,
  "feature": "story-board",
  "title": "Build a Beat Board",
  "purpose": "Add an act, a sequence and a scene card to an empty Beat Board.",
  "difficulty": "Beginner",
  "minutes": 5,
  "prerequisites": [
    "A project is open and you are in Story in the sidebar."
  ],
  "objectives": [
    "Add an act",
    "Add a sequence inside it",
    "Add a scene card to the sequence"
  ],
  "steps": [
    {
      "screen": "177-story-board-empty",
      "action": "click",
      "target": "[data-id=\"story-add-menu\"]",
      "label": "Click Add",
      "text": null,
      "key": null,
      "body": "Story has four views: Beat Board, Outline, Characters and Timeline. The empty Beat Board reads \"Your Beat Board is empty.\" Click Add in the toolbar."
    },
    {
      "screen": "178-story-add-menu",
      "action": "click",
      "target": "[data-id=\"menu-item-0\"]",
      "label": "Choose Act",
      "text": null,
      "key": null,
      "body": "The Add menu offers Act, Sequence, Beat, Scene Card and From Idea Vault.... Choose Act."
    },
    {
      "screen": "184-story-add-act-inline",
      "action": "type",
      "target": "[data-id=\"inline-Act title\"]",
      "label": "Name the act and press Return",
      "text": "Act One: The last bus",
      "key": null,
      "body": "The act is named right on the board, not in a sheet. Type a name for the act and press Return."
    },
    {
      "screen": "185-story-board-one-act",
      "action": "click",
      "target": "a[aria-label=\"Add Sequence\"]",
      "label": "Click Add Sequence in the act",
      "text": null,
      "key": null,
      "body": "The act appears with Add Scene, Add Sequence and Add Beat. Click Add Sequence."
    },
    {
      "screen": "186-story-add-sequence-inline",
      "action": "type",
      "target": "[data-id=\"inline-Sequence name\"]",
      "label": "Name the sequence and press Return",
      "text": "Missing the bus",
      "key": null,
      "body": "Name the sequence inline and press Return."
    },
    {
      "screen": "187-story-sequence-empty",
      "action": "click",
      "target": "a[aria-label=\"Add scene\"]",
      "label": "Click Add scene in the sequence",
      "text": null,
      "key": null,
      "body": "The empty sequence offers Add scene. Click it."
    },
    {
      "screen": "188-story-add-card-inline",
      "action": "view",
      "target": null,
      "label": "A new scene card waits for a short description",
      "text": null,
      "key": null,
      "body": "A new scene card waits for a short description. The board shows no scene numbers; they come from the screenplay."
    }
  ],
  "result": "Your Beat Board has an act, a sequence inside it and a blank scene card ready for text.",
  "mistakes": [
    "Looking for a sheet to name the act. Names are typed inline on the board.",
    "Using Add Scene when you meant Add Sequence. A sequence groups scenes inside an act.",
    "Putting cards you are unsure about in the story. Add them to the Parking Lot instead."
  ],
  "troubleshooting": [
    [
      "I cannot find the Beat Board.",
      "Click Story in the sidebar, then Beat Board in the segmented control."
    ],
    [
      "I pressed Esc and lost my name.",
      "Click the act title and type again."
    ],
    [
      "Where do scene numbers come from?",
      "From the screenplay. Build Screenplay turns cards into screenplay scenes."
    ]
  ],
  "related": [
    "idea-vault",
    "characters",
    "write-screenplay"
  ],
  "next": "characters"
};
