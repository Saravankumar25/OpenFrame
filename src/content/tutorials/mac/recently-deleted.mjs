export default {
  "id": "recently-deleted",
  "available": true,
  "reason": "",
  "path": "data",
  "order": 2,
  "feature": "backups-recovery",
  "title": "Restore a deleted item",
  "purpose": "Find a deleted item in Recently Deleted and bring it back.",
  "difficulty": "Beginner",
  "minutes": 3,
  "prerequisites": [
    "A project is open with at least one deleted item."
  ],
  "objectives": [
    "Open Recently Deleted",
    "Restore an item",
    "Know how long items stay"
  ],
  "steps": [
    {
      "screen": "035-project-home",
      "action": "click",
      "target": "button[aria-label=\"Recently Deleted\"]",
      "label": "On Project Home, click Recently Deleted",
      "text": null,
      "key": null,
      "body": "On Project Home, click Recently Deleted. The same page opens from the project menu or Go > Recently Deleted. Deleted items stay here for 30 days, then they are removed for good."
    },
    {
      "screen": "613-pages-deleted-items",
      "action": "click",
      "target": "[data-id=\"restore-Old catering list\"]",
      "label": "Deleted items wait here for 30 days. Click Restore next to Old catering list",
      "text": null,
      "key": null,
      "body": "The list shows each item with its type, when it was deleted, when it will be removed and where it was. The This project and Global Idea Vault switch changes the list. Click Restore next to the item you want back."
    },
    {
      "screen": "617-pages-deleted-restored",
      "action": "view",
      "target": null,
      "label": "The note goes back to Notes & Tasks. The message offers Undo",
      "text": null,
      "key": null,
      "body": "The item goes back to where it was, and a message offers Undo. Delete permanently asks a second time, and Keep in Recently Deleted cancels it."
    }
  ],
  "result": "A deleted item restored to its original place.",
  "mistakes": [
    "Waiting more than 30 days. After that the item is gone for good.",
    "Choosing Delete permanently by mistake. It asks twice; read the second question."
  ],
  "troubleshooting": [
    [
      "My item is not in the list.",
      "Check the This project and Global Idea Vault switch, and try Search deleted items."
    ],
    [
      "I restored the wrong item.",
      "Click Undo in the message that appears after the restore."
    ]
  ],
  "related": [
    "backup-restore",
    "tasks",
    "files"
  ],
  "next": "files"
};
