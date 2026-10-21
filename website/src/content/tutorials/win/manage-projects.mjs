export default {
  "id": "manage-projects",
  "available": true,
  "reason": "",
  "path": "getting-started",
  "order": 4,
  "feature": null,
  "title": "Pin, archive and restore projects",
  "purpose": "Use a project card menu to pin, archive and restore a project without deleting it.",
  "difficulty": "Beginner",
  "minutes": 3,
  "prerequisites": [
    "Application Home shows at least one project. In this example \"Empty Short\" is already pinned."
  ],
  "objectives": [
    "Open the menu on a project card",
    "Archive a project",
    "Open the archived view",
    "Restore a project"
  ],
  "steps": [
    {
      "screen": "060-apphome-pinned-section",
      "action": "click",
      "target": "button[aria-label=\"More actions for Empty Short\"]",
      "label": "Pinned projects sit at the top. Click the ... button on a card",
      "text": null,
      "key": null,
      "body": "On Application Home, pinned projects sit at the top under PINNED. Click the ... button on a project card, or right-click the card, to open its menu."
    },
    {
      "screen": "061-apphome-card-menu-pinned",
      "action": "click",
      "target": "role=menuitem[name=\"Archive\"]",
      "label": "The menu has Rename, Duplicate Project, Pin or Unpin, Archive. Click Archive",
      "text": null,
      "key": null,
      "body": "The menu has Open, Rename (F2), Duplicate Project, Pin or Unpin, \"Reveal in File Manager\", Archive and \"Delete Project…\". Click Archive."
    },
    {
      "screen": "065-apphome-toast-archived",
      "action": "click",
      "target": "role=button[name=\"Open Archived\"]",
      "label": "Archived, not deleted. Click Open Archived",
      "text": null,
      "key": null,
      "body": "A message says the project was archived and is not deleted. Click \"Open Archived\" to see archived projects."
    },
    {
      "screen": "067-apphome-archived-view",
      "action": "click",
      "target": "role=button[name=\"Restore\"]",
      "label": "Click Restore to bring it back",
      "text": null,
      "key": null,
      "body": "The \"Archived projects\" view says they stay on your computer and can be restored at any time. Click Restore on the project."
    },
    {
      "screen": "073-apphome-toast-restored",
      "action": "view",
      "target": null,
      "label": "The project is back in your projects",
      "text": null,
      "key": null,
      "body": "A message says the project is back in your projects."
    }
  ],
  "result": "You can keep Application Home tidy by pinning and archiving, and bring any archived project back.",
  "mistakes": [
    "Using Delete Project when you only want it out of the way. Archive keeps the project on your computer.",
    "Expecting Delete Project to be one click. It asks you to type the project title first."
  ],
  "troubleshooting": [
    [
      "A project disappeared from Application Home",
      "Click Open Archived. Archived projects are listed there and Restore brings them back."
    ],
    [
      "I cannot find Pin",
      "Pin and Unpin are in the card menu. The menu shows Unpin on a project that is already pinned."
    ]
  ],
  "related": [
    "create-project",
    "project-tour",
    "backup-restore",
    "recently-deleted"
  ],
  "next": "idea-vault"
};
