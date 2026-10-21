export default {
  "id": "backup-restore",
  "available": true,
  "reason": "",
  "path": "data",
  "order": 1,
  "feature": "backups-recovery",
  "title": "Restore a backup or project package",
  "purpose": "Open the restore dialog from the projects screen and choose a backup or package file.",
  "difficulty": "Beginner",
  "minutes": 3,
  "prerequisites": [
    "You have a .ofbackup or .ofproject file on this computer or another disk."
  ],
  "objectives": [
    "Return to the projects screen",
    "Open Restore Backup or Package",
    "Choose the package file"
  ],
  "steps": [
    {
      "screen": "074-apphome-archived-empty",
      "action": "click",
      "target": "role=button[name=\"Back to projects\"]",
      "label": "On the projects screen",
      "text": null,
      "key": null,
      "body": "Click Back to projects to return to the projects screen. To make a backup instead, open the project menu and under BACKUP & PORTABILITY choose Create Backup. Export Full Project is in the same menu."
    },
    {
      "screen": "051-apphome-projects",
      "action": "click",
      "target": "[title=\"Restore a backup or open a project package\"]",
      "label": "Click Restore Backup or Package…",
      "text": null,
      "key": null,
      "body": "On the projects screen, click Restore Backup or Package."
    },
    {
      "screen": "075-apphome-restore-package-dialog",
      "action": "view",
      "target": null,
      "label": "Click Choose Package… and pick the .ofbackup or .ofproject file",
      "text": null,
      "key": null,
      "body": "The Open Project Package dialog opens. It takes a .ofproject (Full Project Package) or a .ofbackup (Backup Package). Click Choose Package, pick the file, and the package is validated before anything is created."
    }
  ],
  "result": "A backup or full project package opened as a project. A restore never overwrites your active project.",
  "mistakes": [
    "Keeping backups on the same disk as the project. The Create Backup dialog suggests another disk if you can.",
    "Skipping the portable option. \"Copy linked external files into the backup\" makes a backup that carries linked files.",
    "Looking for real-time sharing. Sharing is by files only."
  ],
  "troubleshooting": [
    [
      "The package will not open.",
      "It is checked before anything is created. Confirm the file ends in .ofproject or .ofbackup and is not damaged."
    ],
    [
      "I cannot find the restore button.",
      "It is on the projects screen, not inside a project. Click Back to projects."
    ],
    [
      "Where do I create the backup?",
      "Open the project menu at the top left and choose Create Backup under BACKUP & PORTABILITY."
    ]
  ],
  "related": [
    "recently-deleted",
    "manage-projects",
    "files",
    "settings"
  ],
  "next": "recently-deleted"
};
