export default {
  "id": "idea-vault",
  "available": true,
  "reason": "",
  "path": "writing",
  "order": 1,
  "feature": "idea-vault",
  "title": "Capture an idea in the Idea Vault",
  "purpose": "Save a quick idea as a card in the Project Idea Vault.",
  "difficulty": "Beginner",
  "minutes": 3,
  "prerequisites": [
    "A project is open and you are in Idea Vault in the sidebar."
  ],
  "objectives": [
    "Use Quick Capture",
    "Save an idea as a card",
    "Know where the card is stored"
  ],
  "steps": [
    {
      "screen": "252-vault-add-quote",
      "action": "click",
      "target": "[data-id=\"vault-quick-capture\"]",
      "label": "Click Quick Capture, or press Shift+Cmd+N",
      "text": null,
      "key": null,
      "body": "In the Idea Vault toolbar, click Quick Capture, or press Shift+Cmd+N. Add holds the longer list: Note, Image..., URL..., File..., Audio / Voice Note..., Audio File..., Video..., Quote and Quick Capture."
    },
    {
      "screen": "253-vault-add-quick-capture",
      "action": "type",
      "target": "[data-id=\"quick-capture-idea\"]",
      "label": "Type the idea",
      "text": "A bus that never reaches its stop",
      "key": null,
      "body": "The Quick Capture sheet says it is saving to your project and the Project Idea Vault. Type the idea. Return saves and Shift+Return starts a new line."
    },
    {
      "screen": "254-vault-quick-capture-typed",
      "action": "click",
      "target": "role=button[name=\"Save\"]",
      "label": "Click Save, or press Return",
      "text": null,
      "key": null,
      "body": "Click Save, or press Return. Esc cancels instead."
    },
    {
      "screen": "255-vault-saved-notice",
      "action": "view",
      "target": null,
      "label": "The idea is saved as a card in the Project Idea Vault",
      "text": null,
      "key": null,
      "body": "A notice reads \"Saved to the Project Idea Vault\" with Undo. The idea is now a card in the vault, next to your other ideas."
    }
  ],
  "result": "Your idea is saved as a card in the Project Idea Vault.",
  "mistakes": [
    "Pressing Return when you wanted a second line. Use Shift+Return for a new line.",
    "Saving to the wrong vault. The Which Idea Vault switch chooses This Project or Global; check it before you capture.",
    "Not noticing a card left open in the inspector. Close it or click another card; it does not change your new idea."
  ],
  "troubleshooting": [
    [
      "I cannot see my idea after saving.",
      "Check the Which Idea Vault switch and the View switch (Grid, Card, List, Folder), or search for a word from it."
    ],
    [
      "I saved by mistake.",
      "Click Undo in the notice straight away."
    ],
    [
      "Image... and File... open a macOS panel.",
      "That is the standard Open panel. Choose the file and the card is added."
    ]
  ],
  "related": [
    "story-board",
    "moodboard",
    "recently-deleted"
  ],
  "next": "story-board"
};
