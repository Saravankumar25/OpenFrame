export default {
  "id": "export-script",
  "available": true,
  "reason": "",
  "path": "writing",
  "order": 8,
  "feature": "import-export",
  "title": "Export a script and preview the pages",
  "purpose": "Choose a format and scenes, then preview the pages before you export.",
  "difficulty": "Beginner",
  "minutes": 5,
  "prerequisites": [
    "A screenplay is open in the editor."
  ],
  "objectives": [
    "Open Export Screenplay",
    "Pick a format",
    "Export the entire draft or selected scenes",
    "Preview the pages"
  ],
  "steps": [
    {
      "screen": "244-sp-editor-standard",
      "action": "click",
      "target": "role=button[name=\"Export\"s]",
      "label": "Click Export in the screenplay toolbar",
      "text": null,
      "key": null,
      "body": "In the screenplay toolbar, click Export."
    },
    {
      "screen": "324-sp-export-dialog",
      "action": "click",
      "target": "role=button[name=\"Additional formats…\"]",
      "label": "Pick a format: PDF, Final Draft (.fdx), Fountain, DOCX. Additional formats… shows more",
      "text": null,
      "key": null,
      "body": "The Export Screenplay dialog asks which version, which format and where it is saved. Formats are PDF, Final Draft (.fdx), Fountain and DOCX. Click \"Additional formats…\" to see more, including Plain text (.txt)."
    },
    {
      "screen": "325-sp-export-more-formats",
      "action": "click",
      "target": "role=dialog >> text=Selected scenes",
      "label": "Choose Entire draft or Selected scenes",
      "text": null,
      "key": null,
      "body": "Under SCOPE, choose \"Entire draft\" or \"Selected scenes\"."
    },
    {
      "screen": "326-sp-export-selected-none",
      "action": "click",
      "target": "role=group[name=\"Scenes to export\"] >> label >> nth=0",
      "label": "Tick the scenes you want",
      "text": null,
      "key": null,
      "body": "With Selected scenes, tick the scenes you want."
    },
    {
      "screen": "327-sp-export-selected-scenes",
      "action": "click",
      "target": "role=dialog >> role=button[name=\"Preview…\"]",
      "label": "Click Preview… to check the pages",
      "text": null,
      "key": null,
      "body": "Click \"Preview…\" to check the pages."
    },
    {
      "screen": "328-sp-print-preview",
      "action": "view",
      "target": null,
      "label": "Print or Export PDF from the preview; Export… saves the chosen format",
      "text": null,
      "key": null,
      "body": "In Print Preview you can Print or Export PDF, go \"Back to Export Options\", or Close. \"Export…\" in the dialog saves the chosen format."
    }
  ],
  "result": "A previewed export of the whole script or just the scenes you chose.",
  "mistakes": [
    "Exporting before checking the pages. Use \"Preview…\" first.",
    "Expecting private notes in the file. The dialog says \"Private notes excluded.\""
  ],
  "troubleshooting": [
    [
      "I need a clean script or marked pages",
      "The dialog offers \"Clean script\" and \"Revision-marked version\"."
    ],
    [
      "I worry export changes my project",
      "The dialog says export creates a snapshot; your project does not change."
    ]
  ],
  "related": [
    "import-script",
    "drafts-revisions",
    "screenplay-notes",
    "sides"
  ],
  "next": "pages-templates"
};
