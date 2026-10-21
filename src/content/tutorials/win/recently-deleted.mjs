export default {
  "id": "recently-deleted",
  "available": true,
  "reason": "",
  "path": "data",
  "order": 2,
  "feature": "backups-recovery",
  "title": "Undo a delete and use Recently Deleted",
  "purpose": "Delete a note, then bring it back from the message or the Recently Deleted page.",
  "difficulty": "Beginner",
  "minutes": 3,
  "prerequisites": [
    "A project is open."
  ],
  "objectives": [
    "Delete an item safely",
    "Use Undo in the message",
    "Know where deleted items wait"
  ],
  "steps": [
    {
      "screen": "722-notes-new-note-editor",
      "action": "type",
      "target": "#ne-title",
      "label": "Here a short note is written first",
      "text": "Tour note on the radio tower",
      "key": null,
      "body": "On Notes & Tasks, a new note is open in the editor. Type its title, here a short note about the radio tower."
    },
    {
      "screen": "723-notes-new-note-filled",
      "action": "click",
      "target": "role=button[name=\"Delete\"]",
      "label": "Click Delete",
      "text": null,
      "key": null,
      "body": "With the note filled in, click Delete."
    },
    {
      "screen": "724-notes-note-deleted-toast",
      "action": "view",
      "target": null,
      "label": "It moves to Recently Deleted; click Undo in the message to bring it back",
      "text": null,
      "key": null,
      "body": "The note moves to Recently Deleted. A message says \"Moved note ... to Recently Deleted\" with an Undo button. Click Undo to bring it back. Ctrl+Z also undoes."
    }
  ],
  "result": "A deleted item that comes back with Undo. Deleted items stay in Recently Deleted for 30 days, then they are removed for good.",
  "mistakes": [
    "Waiting for the message to disappear and then hunting. The item is still in Recently Deleted for 30 days.",
    "Clicking Delete Permanently by habit. It asks for a second confirmation with Keep in Recently Deleted, so read the dialog.",
    "Forgetting the Idea Vault. It has its own Recently Deleted view."
  ],
  "troubleshooting": [
    [
      "The Undo message went away.",
      "Open Recently Deleted from the project menu or Project Home quick access and click Restore."
    ],
    [
      "I do not see an item in the list.",
      "Check the switch: This project or Global Idea Vault. Use Search deleted items."
    ],
    [
      "Ctrl+Z did nothing.",
      "The project menu shows Undo and Redo with the action name. Use it if the shortcut is blocked."
    ]
  ],
  "related": [
    "backup-restore",
    "idea-vault",
    "files",
    "tasks"
  ],
  "next": "files"
};
