export default {
  "id": "idea-vault",
  "available": true,
  "reason": "",
  "path": "writing",
  "order": 1,
  "feature": "idea-vault",
  "title": "Collect ideas in the Idea Vault",
  "purpose": "Capture an idea fast with Quick Capture and group ideas in a collection.",
  "difficulty": "Beginner",
  "minutes": 5,
  "prerequisites": [
    "A project is open."
  ],
  "objectives": [
    "Add an item from the Add menu",
    "Use Quick Capture (Ctrl+Shift+N)",
    "Create a collection"
  ],
  "steps": [
    {
      "screen": "091-vault-grid-all-items",
      "action": "click",
      "target": "main >> role=button[name=\"Add\"s]",
      "label": "Click Add to choose what to collect",
      "text": null,
      "key": null,
      "body": "Open Idea Vault from the sidebar. It holds everything collected for the film; nothing in it is classified or synced to the screenplay. Click Add to choose what to collect."
    },
    {
      "screen": "102-vault-add-menu",
      "action": "click",
      "target": "role=menuitem[name^=\"Quick Capture\"]",
      "label": "Pick Quick Capture for a fast idea",
      "text": null,
      "key": null,
      "body": "The Add menu offers Note, Image, URL, File, \"Audio / Voice Note\", \"Audio file…\", Video, Quote, Sketch and Quick Capture. Pick Quick Capture for a fast idea."
    },
    {
      "screen": "129-vault-quick-capture-dialog",
      "action": "type",
      "target": "textarea[aria-label=\"Idea\"]",
      "label": "Type the idea",
      "text": "A bus that only runs when someone is waiting for it.",
      "key": null,
      "body": "The Quick Capture dialog saves to the Project Idea Vault. Type the idea in the box that says \"Type the idea before it's gone…\"."
    },
    {
      "screen": "130-vault-quick-capture-filled",
      "action": "click",
      "target": "role=dialog >> role=button[name=\"Save\"]",
      "label": "Click Save (or press Enter)",
      "text": null,
      "key": null,
      "body": "Click Save, or press Enter. Shift+Enter starts a new line and Esc cancels."
    },
    {
      "screen": "131-vault-quick-capture-saved-toast",
      "action": "click",
      "target": "role=button[name=\"New Collection\"]",
      "label": "Organise: click New Collection in the side panel",
      "text": null,
      "key": null,
      "body": "A toast says \"Saved to the Project Idea Vault\" with Undo. To organise, click New Collection in the side panel."
    },
    {
      "screen": "138-vault-new-collection-dialog",
      "action": "type",
      "target": "#vv-name",
      "label": "Name the collection and click Create",
      "text": "Scratch collection",
      "key": null,
      "body": "In the New Collection dialog, type a name in the \"Name *\" box and click Create. An empty name shows \"Name is required.\""
    },
    {
      "screen": "140-vault-collection-empty",
      "action": "view",
      "target": null,
      "label": "The collection is ready in the side panel",
      "text": null,
      "key": null,
      "body": "The collection appears in the side panel under COLLECTIONS, ready for items. You can also use FOLDERS and TAGS there."
    }
  ],
  "result": "An idea saved in the Project Idea Vault and a collection ready to hold related items.",
  "mistakes": [
    "Saving an idea to the wrong place. Quick Capture shows where it saves, here \"Project Idea Vault\". The Global Idea Vault is separate.",
    "Waiting to write the idea properly. Quick Capture is for catching it first. Sort it into a collection later."
  ],
  "troubleshooting": [
    [
      "I saved an idea by mistake",
      "Click Undo on the toast. A deleted item also waits in Recently Deleted."
    ],
    [
      "Image, File, Audio file and Video open a file chooser",
      "They use the Windows file picker. Pick the file there and it joins the vault."
    ]
  ],
  "related": [
    "story-board",
    "moodboard",
    "recently-deleted",
    "project-tour"
  ],
  "next": "story-board"
};
