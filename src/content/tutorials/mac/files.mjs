export default {
  "id": "files",
  "available": true,
  "reason": "",
  "path": "data",
  "order": 3,
  "feature": null,
  "title": "Add a file to the project",
  "purpose": "Copy a file into the project so it stays with the film.",
  "difficulty": "Beginner",
  "minutes": 4,
  "prerequisites": [
    "A project is open.",
    "A file on your Mac, such as a permit letter or a reference."
  ],
  "objectives": [
    "Open Files",
    "Copy a file into the project",
    "Know the difference between stored and linked files"
  ],
  "steps": [
    {
      "screen": "558-files-empty",
      "action": "click",
      "target": "[data-id=\"add-file\"]",
      "label": "In Files, click Add File",
      "text": null,
      "key": null,
      "body": "Open Files in the sidebar. The page asks \"What files belong with this film?\". Click Add File. You can also drag files here from Finder, or click New Folder to organise them."
    },
    {
      "screen": "559-pages-files-add-menu",
      "action": "click",
      "target": "[data-id=\"menu-item-0\"]",
      "label": "Choose Copy into project… and pick the file in the Open panel",
      "text": null,
      "key": null,
      "body": "The menu offers Copy into project and Link to file. Choose Copy into project, then pick the file in the Open panel."
    },
    {
      "screen": "563-pages-files-one-file",
      "action": "view",
      "target": null,
      "label": "The file is listed as stored in the project, with Open and a More actions menu",
      "text": null,
      "key": null,
      "body": "The file is listed as stored in the project, with Open and a More actions menu. A linked file shows as Linked (external) and stays where it is on your Mac. Select a file to add Notes or choose a Folder."
    }
  ],
  "result": "A file stored inside the project and listed in Files.",
  "mistakes": [
    "Linking a file and then moving it. A linked file lives outside the project, so keep it in place.",
    "Skipping folders. Use New Folder to keep references together."
  ],
  "troubleshooting": [
    [
      "The Open panel does not show my file.",
      "Browse to the folder that holds it, or drag the file from Finder into the page."
    ],
    [
      "A linked file will not open.",
      "It was moved or removed. Copy it into the project instead."
    ]
  ],
  "related": [
    "backup-restore",
    "recently-deleted",
    "settings"
  ],
  "next": "themes-appearance"
};
