export default {
  "id": "manage-projects",
  "available": true,
  "reason": "",
  "path": "getting-started",
  "order": 4,
  "feature": null,
  "title": "Pin, archive and restore projects",
  "purpose": "Keep Application Home tidy by pinning projects you use most and archiving finished ones.",
  "difficulty": "Beginner",
  "minutes": 3,
  "prerequisites": [
    "Application Home lists at least two projects."
  ],
  "objectives": [
    "Pin a project to the top",
    "Archive a project without deleting it",
    "Restore an archived project"
  ],
  "steps": [
    {
      "screen": "061-home-two-projects",
      "action": "click",
      "target": "[data-id=\"project-card-actions-Night Bus\"]",
      "label": "Open Actions for Night Bus and choose Pin",
      "text": null,
      "key": null,
      "body": "Every project card has an Actions button, and the same items appear on Control-click. The menu has Open, Rename..., Duplicate Project, Pin, Archive, Remove from Recent and Delete Project.... Open Actions on a card and choose Pin. It moves to a PINNED group above RECENT PROJECTS."
    },
    {
      "screen": "064-home-pinned",
      "action": "click",
      "target": "[data-id=\"project-card-actions-Empty Short\"]",
      "label": "Open Actions for Empty Short and choose Archive",
      "text": null,
      "key": null,
      "body": "Open Actions on another card and choose Archive. A notice says it was archived and is not deleted."
    },
    {
      "screen": "065-home-one-archived",
      "action": "click",
      "target": "[data-id=\"home-open-archived\"]",
      "label": "Click Open Archived",
      "text": null,
      "key": null,
      "body": "The toolbar button now reads Open Archived. Click it."
    },
    {
      "screen": "066-home-archived-page",
      "action": "click",
      "target": "button[aria-label=\"Restore Empty Short\"]",
      "label": "Click Restore",
      "text": null,
      "key": null,
      "body": "The Archived projects page shows each archived project with Restore and Open. Click Restore on the one you want back."
    },
    {
      "screen": "067-home-archived-empty",
      "action": "view",
      "target": null,
      "label": "The project is back in your projects",
      "text": null,
      "key": null,
      "body": "A notice says the project is back in your projects. Nothing was lost along the way."
    }
  ],
  "result": "One project is pinned, and an archived project has been restored to your list.",
  "mistakes": [
    "Choosing Delete Project... when you meant Archive. Delete asks you to type the project name first, so read the sheet before confirming.",
    "Thinking Archive removes the project. It stays on this Mac and Restore brings it back.",
    "Using Remove from Recent to clean up. That only drops the card from the list; archive is the tidy way to hide finished work."
  ],
  "troubleshooting": [
    [
      "I cannot find an archived project.",
      "Click Open Archived in the toolbar. It shows how many are archived."
    ],
    [
      "The Actions menu looks different.",
      "Control-click on the card gives the same menu as the Actions button."
    ],
    [
      "I renamed the wrong project.",
      "Open Actions and choose Rename... again to set the title back."
    ]
  ],
  "related": [
    "create-project",
    "recently-deleted",
    "backup-restore"
  ],
  "next": "idea-vault"
};
