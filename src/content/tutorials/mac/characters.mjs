export default {
  "id": "characters",
  "available": true,
  "reason": "",
  "path": "writing",
  "order": 3,
  "feature": "story-board",
  "title": "Add a character",
  "purpose": "Create a character in Story and see where they appear in the script.",
  "difficulty": "Beginner",
  "minutes": 3,
  "prerequisites": [
    "A project is open and you are in Story in the sidebar."
  ],
  "objectives": [
    "Open the Characters view",
    "Fill in the New Character sheet",
    "Open the character page"
  ],
  "steps": [
    {
      "screen": "177-story-board-empty",
      "action": "click",
      "target": "input[aria-label=\"Characters\"]",
      "label": "In Story, click Characters",
      "text": null,
      "key": null,
      "body": "In Story, click Characters in the segmented control."
    },
    {
      "screen": "230-story-characters-empty",
      "action": "click",
      "target": "button[aria-label=\"Add Character\"]",
      "label": "Click Add Character",
      "text": null,
      "key": null,
      "body": "The empty view says \"No characters yet.\" Click Add Character."
    },
    {
      "screen": "231-story-add-character-sheet",
      "action": "type",
      "target": "[data-id=\"story-new-character-name\"]",
      "label": "Type a name, and if you like a role and a short description",
      "text": "Kiran",
      "key": null,
      "body": "The New Character sheet has Name, Role / title, Image, Short description, Cancel and Create Character. Name is all you need. Type a name, and if you like a role and a short description."
    },
    {
      "screen": "232-story-add-character-filled",
      "action": "click",
      "target": "[data-id=\"story-create-character\"]",
      "label": "Click Create Character",
      "text": null,
      "key": null,
      "body": "Click Create Character. A notice offers Undo."
    },
    {
      "screen": "233-story-character-one",
      "action": "view",
      "target": null,
      "label": "The character page lists every scene the character appears in",
      "text": null,
      "key": null,
      "body": "The character page lists every scene the character appears in, found from the screenplay. It also has Relationships, linked scene cards and Notes."
    }
  ],
  "result": "Your character exists in Story with a page of its own.",
  "mistakes": [
    "Thinking you must fill every field. Only Name is required; role, image and notes are optional.",
    "Typing a near-duplicate name. The sheet warns about a possible match, so check before creating.",
    "Expecting scenes to appear at once. They show once the name appears in your screenplay."
  ],
  "troubleshooting": [
    [
      "Create Character does nothing.",
      "Check that the Name field has text."
    ],
    [
      "I made the character by mistake.",
      "Click Undo in the notice, or use More actions on the character page."
    ],
    [
      "The scenes list is empty.",
      "The character is not in the screenplay yet. Write a scene with that name in a Character element."
    ]
  ],
  "related": [
    "story-board",
    "write-screenplay",
    "cast-crew"
  ],
  "next": "write-screenplay"
};
