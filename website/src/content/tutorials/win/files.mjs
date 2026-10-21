export default {
  "id": "files",
  "available": true,
  "reason": "",
  "path": "data",
  "order": 3,
  "feature": null,
  "title": "Organise project files in a folder",
  "purpose": "Create a folder in Files so reference documents sit together with the project.",
  "difficulty": "Beginner",
  "minutes": 3,
  "prerequisites": [
    "A project is open."
  ],
  "objectives": [
    "Open Files",
    "Create a folder",
    "See where files go next"
  ],
  "steps": [
    {
      "screen": "013-shell-project-home",
      "action": "click",
      "target": "role=button[name=\"Files\"]",
      "label": "In the left rail, open Files",
      "text": null,
      "key": null,
      "body": "Open Files in the left rail. It asks \"What files belong with this film?\" There are New Folder and Add File buttons, List and Grid views and Search files."
    },
    {
      "screen": "690-files-list",
      "action": "click",
      "target": "role=button[name=\"New Folder\"]",
      "label": "Click New Folder",
      "text": null,
      "key": null,
      "body": "Click New Folder. A dialog opens."
    },
    {
      "screen": "707-files-new-folder-dialog",
      "action": "type",
      "target": "#fo-name",
      "label": "Type a folder name",
      "text": "Tour Folder",
      "key": null,
      "body": "Type a folder name, here \"Tour Folder\"."
    },
    {
      "screen": "708-files-new-folder-filled",
      "action": "click",
      "target": "role=button[name=\"Create\"]",
      "label": "Click Create",
      "text": null,
      "key": null,
      "body": "The name is in the box. Click Create."
    },
    {
      "screen": "709-files-folder-created",
      "action": "view",
      "target": null,
      "label": "The new folder opens, ready for files",
      "text": null,
      "key": null,
      "body": "The new folder opens and says it is empty. Drag files onto it from Explorer, or use Add File with \"Copy into project...\" or \"Link to file...\"."
    }
  ],
  "result": "A folder in the project file cabinet. Files copied into the project travel with it, and deleted files go to Recently Deleted.",
  "mistakes": [
    "Linking a file and then moving it. A linked file can be relinked, but a copy stays inside the project.",
    "Renaming a file and expecting the original to change. The name shown in OpenFrame does not rename the file itself.",
    "Deleting a folder to clear it. Check what is inside first."
  ],
  "troubleshooting": [
    [
      "Add File opens a Windows picker.",
      "Choose the file there. Copy into project stores a copy, and Link to file keeps it where it is."
    ],
    [
      "A file moved and shows as missing.",
      "Open the file in the details drawer and relink it."
    ],
    [
      "Created folder by mistake.",
      "Click Undo in the toast or use Delete from the folder list."
    ]
  ],
  "related": [
    "recently-deleted",
    "backup-restore",
    "settings",
    "idea-vault"
  ],
  "next": "themes-appearance"
};
