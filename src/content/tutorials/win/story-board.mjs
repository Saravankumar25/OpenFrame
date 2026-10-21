export default {
  "id": "story-board",
  "available": true,
  "reason": "",
  "path": "writing",
  "order": 2,
  "feature": "story-board",
  "title": "Plan your story on the Beat Board",
  "purpose": "Use the Beat Board to arrange acts, sequences, beats and scene cards.",
  "difficulty": "Beginner",
  "minutes": 5,
  "prerequisites": [
    "A project is open with some story content, or an empty Beat Board you can add to."
  ],
  "objectives": [
    "Open the Board tab in Story",
    "Add acts, sequences, beats and scene cards",
    "Open a scene card"
  ],
  "steps": [
    {
      "screen": "169-story-page-filled",
      "action": "click",
      "target": "role=tab[name=\"Board\"]",
      "label": "In Story, open the Board tab",
      "text": null,
      "key": null,
      "body": "Click Story in the sidebar. The tabs are Story, Board, Outline, Characters and Timeline. Open the Board tab."
    },
    {
      "screen": "171-story-board-filled",
      "action": "click",
      "target": "main >> role=button[name=\"Add\"s]",
      "label": "Acts are columns, sequences group scene cards, beats mark turns. Add creates any of them",
      "text": null,
      "key": null,
      "body": "The Beat Board shows acts as columns, with sequences grouping scene cards and beats marking turns. Click Add to create any of them."
    },
    {
      "screen": "172-story-board-add-menu",
      "action": "key",
      "target": null,
      "label": "Press Esc to close the menu",
      "text": null,
      "key": "Esc",
      "body": "The Add menu lists Act, Sequence, Beat, Scene Card (Ctrl+N) and \"From Idea Vault…\". Press Esc to close it."
    },
    {
      "screen": "171-story-board-filled",
      "action": "rightclick",
      "target": "div[aria-label=\"Scene Card: Lakshmi steps off the night bus\"]",
      "label": "Right-click a scene card",
      "text": null,
      "key": null,
      "body": "Right-click a scene card to open its menu."
    },
    {
      "screen": "183-story-board-card-context-menu",
      "action": "click",
      "target": "role=menuitem[name^=\"Open\"][name$=\"Enter\"]",
      "label": "Click Open",
      "text": null,
      "key": null,
      "body": "The menu has Open, Duplicate (Ctrl+D), \"Move…\", \"Send to Parking Lot\", \"Convert to Screenplay Scene\", \"Copy as Beat\" and Delete. Click Open."
    },
    {
      "screen": "190-story-card-drawer",
      "action": "view",
      "target": null,
      "label": "The card opens: scene heading, short and long description, notes",
      "text": null,
      "key": null,
      "body": "The card opens with Scene heading, Short description, Long description and Notes. A heading such as \"EXT. OLD RAILWAY STATION - NIGHT\" is needed before a card becomes a screenplay scene."
    }
  ],
  "result": "Your story laid out in acts, sequences, beats and cards, with each card ready to become a screenplay scene.",
  "mistakes": [
    "Looking for a separate story board. In OpenFrame the Story area is the Beat Board. Storyboards with panels are in Production.",
    "Converting a card to a scene without a scene heading. Fill in the Scene heading first."
  ],
  "troubleshooting": [
    [
      "I deleted a card",
      "Press Ctrl+Z, or open Recently Deleted from the project menu. Deleted items stay there for 30 days."
    ],
    [
      "I want to see the order before writing",
      "Use Build Screenplay. It previews the card order before anything is created."
    ]
  ],
  "related": [
    "characters",
    "idea-vault",
    "write-screenplay",
    "storyboard"
  ],
  "next": "characters"
};
