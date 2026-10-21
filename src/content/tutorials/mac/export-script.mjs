export default {
  "id": "export-script",
  "available": true,
  "reason": "",
  "path": "writing",
  "order": 8,
  "feature": "import-export",
  "title": "Export your screenplay",
  "purpose": "Open the Export Screenplay sheet and choose a format.",
  "difficulty": "Beginner",
  "minutes": 3,
  "prerequisites": [
    "A screenplay with some text is open."
  ],
  "objectives": [
    "Open Export Screenplay from Screenplay Tools",
    "Choose a format",
    "Know the options before you export"
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
      "label": "Choose Import and Export, then Export Screenplay…",
      "text": null,
      "key": null,
      "body": "Choose Import and Export, then Export Screenplay.... The File menu also has Export Screenplay... and Export PDF...."
    },
    {
      "screen": "079-screenplay-tools-export-screenplay",
      "action": "click",
      "target": "[data-id=\"export-format-fdx\"]",
      "label": "Choose a format, here Final Draft (.fdx)",
      "text": null,
      "key": null,
      "body": "The sheet asks what you are exporting, which version and in which format. The formats are PDF, Final Draft (.fdx), Fountain and DOCX, with Additional formats... for plain text. Choose Final Draft (.fdx) here."
    },
    {
      "screen": "166-screenplay-export-fdx",
      "action": "view",
      "target": null,
      "label": "Check the options, then click Export… and choose where to save",
      "text": null,
      "key": null,
      "body": "Check the options: Entire draft or Selected scenes, Clean script or Revision-marked version, and the Title page, Scene numbers, Revision info and Include notes boxes. Click Preview... to see it, then Export... and choose where to save."
    }
  ],
  "result": "You know how to set the format and options. Export... then opens a macOS Save panel for the location.",
  "mistakes": [
    "Exporting without checking the options. Look at Scope, Title page and Scene numbers first.",
    "Expecting private notes in the file. The sheet says \"Private notes excluded.\"",
    "Thinking Export changes the project. It creates a snapshot and your project does not change."
  ],
  "troubleshooting": [
    [
      "Export... is not available.",
      "Check that the screenplay has text and that a format is selected."
    ],
    [
      "I need plain text.",
      "Click Additional formats... on the sheet."
    ],
    [
      "I want only a few scenes.",
      "Choose Selected scenes under Scope."
    ]
  ],
  "related": [
    "import-script",
    "drafts-revisions",
    "screenplay-notes"
  ],
  "next": "pages-templates"
};
