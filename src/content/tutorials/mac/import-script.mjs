export default {
  "id": "import-script",
  "available": true,
  "reason": "",
  "path": "writing",
  "order": 7,
  "feature": "import-export",
  "title": "Bring in an existing screenplay",
  "purpose": "Open the Import Screenplay sheet and see which sources it accepts.",
  "difficulty": "Beginner",
  "minutes": 3,
  "prerequisites": [
    "A project is open and you are in Screenplay in the sidebar."
  ],
  "objectives": [
    "Open Import Screenplay from Screenplay Tools",
    "Know the accepted formats",
    "Know that importing never replaces your current screenplay"
  ],
  "steps": [
    {
      "screen": "068-screenplay-editor-typing",
      "action": "click",
      "target": "[data-id=\"screenplay-tools-menu\"]",
      "label": "Click Screenplay Tools",
      "text": null,
      "key": null,
      "body": "In the screenplay page, click Screenplay Tools."
    },
    {
      "screen": "069-screenplay-tools-menu",
      "action": "click",
      "target": "[data-id=\"menu-item-13\"]",
      "label": "Choose Import and Export, then Import Screenplay…",
      "text": null,
      "key": null,
      "body": "Choose Import and Export, then Import Screenplay.... The same command is in the File menu, the toolbar New menu and on a new project's Write/Import Screenplay card."
    },
    {
      "screen": "078-screenplay-tools-import-screenplay",
      "action": "view",
      "target": null,
      "label": "Choose a file with Browse…, paste text, or open a sample, then click Preview",
      "text": null,
      "key": null,
      "body": "The sheet asks \"What existing script do you want to bring into OpenFrame?\" and shows three steps: Select source, Preview interpretation and Import. Choose a file with Browse..., paste text, or open a sample, then click Preview. It reads PDF, Final Draft (.fdx), Fountain, TXT and DOCX."
    }
  ],
  "result": "You know where to start an import and what it accepts. Preview, warnings and Import come after that on the same sheet.",
  "mistakes": [
    "Worrying about your current script. Importing never replaces your current screenplay.",
    "Skipping Preview. Check how the script was read before you import.",
    "Expecting a pasted script to need a file. You can paste screenplay text into the box instead."
  ],
  "troubleshooting": [
    [
      "The file will not open.",
      "Check it is PDF, .fdx, Fountain, TXT or DOCX. Browse... opens the standard macOS Open panel."
    ],
    [
      "The import looks wrong in Preview.",
      "Cancel and try another format of the same script, such as Fountain or .fdx."
    ],
    [
      "I imported by mistake.",
      "Click Undo in the toast \"Imported screenplay\" that appears after an import."
    ]
  ],
  "related": [
    "export-script",
    "write-screenplay",
    "drafts-revisions"
  ],
  "next": "export-script"
};
