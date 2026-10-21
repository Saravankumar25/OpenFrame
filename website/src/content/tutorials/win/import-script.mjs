export default {
  "id": "import-script",
  "available": true,
  "reason": "",
  "path": "writing",
  "order": 7,
  "feature": "import-export",
  "title": "Import an existing script",
  "purpose": "Bring a script into a project and check the preview before anything is created.",
  "difficulty": "Beginner",
  "minutes": 5,
  "prerequisites": [
    "A project is open. You have a script as PDF, Final Draft (.fdx), Fountain, TXT or DOCX, or you use a sample script."
  ],
  "objectives": [
    "Open Import Screenplay",
    "Choose a source",
    "Read the preview and warnings",
    "Finish the import"
  ],
  "steps": [
    {
      "screen": "013-shell-project-home",
      "action": "click",
      "target": "button[aria-label=\"Create\"]",
      "label": "Click + (Create)",
      "text": null,
      "key": null,
      "body": "On Project Home, click the + (Create) button."
    },
    {
      "screen": "015-shell-create-menu",
      "action": "click",
      "target": "role=menuitem[name=\"Import Screenplay…\"]",
      "label": "Choose Import Screenplay…",
      "text": null,
      "key": null,
      "body": "Choose \"Import Screenplay…\". You can also find it in \"View and more\" or on an empty Screenplay page."
    },
    {
      "screen": "009-sp-import-source",
      "action": "click",
      "target": "role=radio[name=\"Example Feature\"]",
      "label": "Choose a file with Browse…, paste text, or open a sample script",
      "text": null,
      "key": null,
      "body": "The Import Screenplay dialog has three steps: \"Select source\", \"Preview interpretation\" and \"Import\". Choose a file with \"Browse…\", paste text, or open a sample script."
    },
    {
      "screen": "010-sp-import-sample-chosen",
      "action": "click",
      "target": "role=dialog >> role=button[name=\"Preview\"]",
      "label": "Click Preview",
      "text": null,
      "key": null,
      "body": "Click Preview. Nothing is created yet."
    },
    {
      "screen": "011-sp-import-preview",
      "action": "click",
      "target": "role=dialog >> role=button[name=\"Import\"s]",
      "label": "Check scenes, characters and warnings, then click Import",
      "text": null,
      "key": null,
      "body": "The preview shows the source, scene count, characters, pages and any warnings. Check them, choose \"Import as\" and the draft name, then click Import."
    },
    {
      "screen": "012-sp-import-complete",
      "action": "view",
      "target": null,
      "label": "Import complete. Nothing else in your project was changed",
      "text": null,
      "key": null,
      "body": "An \"Import complete\" summary lists the imported scenes, detected characters and warnings. Nothing else in your project was changed."
    }
  ],
  "result": "A script in your project, imported as a new screenplay or a new draft.",
  "mistakes": [
    "Expecting the import to replace your screenplay. It arrives as a new screenplay or a new draft.",
    "Skipping the warnings. Read \"Show warnings\" before you import, because some details are adjusted on the way in."
  ],
  "troubleshooting": [
    [
      "I see \"Import failed\"",
      "Your current project was not changed. Check the text is a screenplay and try again."
    ],
    [
      "I want to keep the original file",
      "Tick \"Keep the original file in Project Files\" in the preview."
    ]
  ],
  "related": [
    "export-script",
    "write-screenplay",
    "drafts-revisions",
    "files"
  ],
  "next": "export-script"
};
