export default {
  "id": "backup-restore",
  "available": true,
  "reason": "",
  "path": "data",
  "order": 1,
  "feature": "backups-recovery",
  "title": "Back up a project and restore it",
  "purpose": "Create a backup file, then restore it as a copy without touching the original.",
  "difficulty": "Beginner",
  "minutes": 8,
  "prerequisites": [
    "A project is open.",
    "A place to save the backup, ideally another disk."
  ],
  "objectives": [
    "Create a backup with a label",
    "Find the backup file",
    "Restore a backup as a copy"
  ],
  "steps": [
    {
      "screen": "652-menu-file-with-project",
      "action": "click",
      "target": "[data-id=\"menu-item-13\"]",
      "label": "With the project open, choose File > Back Up Project…",
      "text": null,
      "key": null,
      "body": "With the project open, choose File > Back Up Project in the menu bar."
    },
    {
      "screen": "658-package-back-up-project",
      "action": "type",
      "target": "[data-id=\"backup-label\"]",
      "label": "Type an optional label. Use Browse… to choose where to save, ideally another disk",
      "text": "before-rewrite",
      "key": null,
      "body": "The Create Backup sheet says \"A recoverable copy of supported project state.\" Type an optional label. Use Browse to choose where to save, ideally another disk."
    },
    {
      "screen": "659-package-backup-ready",
      "action": "click",
      "target": "[data-id=\"backup-create\"]",
      "label": "Click Create Backup",
      "text": null,
      "key": null,
      "body": "Check what this backup contains. Tick the check box to copy linked external files in if you want a portable backup. Click Create Backup."
    },
    {
      "screen": "661-package-backup-created",
      "action": "view",
      "target": null,
      "label": "Backup created. Show in folder reveals the .ofbackup file",
      "text": null,
      "key": null,
      "body": "The sheet says Backup created. Show in folder reveals the .ofbackup file. Click Close."
    },
    {
      "screen": "652-menu-file-with-project",
      "action": "click",
      "target": "[data-id=\"menu-item-21\"]",
      "label": "Choose File > Restore Backup or Package…",
      "text": null,
      "key": null,
      "body": "To restore, choose File > Restore Backup or Package in the menu bar. The empty Application Home has the same button."
    },
    {
      "screen": "684-package-restore",
      "action": "click",
      "target": "[data-id=\"open-project-choose\"]",
      "label": "Click Choose Package… and pick the .ofbackup or .ofproject file",
      "text": null,
      "key": null,
      "body": "The Open Project Package sheet accepts a .ofproject Full Project Package or a .ofbackup Backup Package. Click Choose Package and pick the file. A damaged file is refused."
    },
    {
      "screen": "686-package-restore-collision",
      "action": "click",
      "target": "[data-id=\"open-project-confirm\"]",
      "label": "The project already exists here, so keep Open as Copy and click Open as Copy",
      "text": null,
      "key": null,
      "body": "If the project already exists here, the sheet says so and shows Package, Project, Created and Size. Keep Open as Copy and click Open as Copy. Importing never silently replaces the current project."
    },
    {
      "screen": "687-package-restore-opened-copy",
      "action": "view",
      "target": null,
      "label": "The backup opens as Night Bus (copy). The original project is untouched",
      "text": null,
      "key": null,
      "body": "The backup opens as a copy of the project. The original project is untouched."
    }
  ],
  "result": "A saved .ofbackup file and a restored copy of the project.",
  "mistakes": [
    "Saving the backup on the same disk only. Choose another disk if you can.",
    "Choosing Replace Existing After Backup when a copy would do. Open as Copy is the safe default."
  ],
  "troubleshooting": [
    [
      "The package is refused.",
      "The file may be damaged or not a .ofproject or .ofbackup file. Try another copy."
    ],
    [
      "I cannot find the backup file.",
      "Click Show in folder on the Backup created screen, or look in the folder you chose with Browse."
    ]
  ],
  "related": [
    "recently-deleted",
    "files",
    "settings"
  ],
  "next": "recently-deleted"
};
