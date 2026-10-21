export default {
  "id": "drafts-revisions",
  "available": true,
  "reason": "",
  "path": "writing",
  "order": 6,
  "feature": "drafts-revisions",
  "title": "Compare drafts, lock and revise",
  "purpose": "Compare two drafts, lock a shooting draft and start a coloured revision.",
  "difficulty": "Intermediate",
  "minutes": 8,
  "prerequisites": [
    "A screenplay with two drafts, open in the editor."
  ],
  "objectives": [
    "Compare two drafts",
    "Lock a draft as the Shooting Draft",
    "Start a revision with a colour"
  ],
  "steps": [
    {
      "screen": "244-sp-editor-standard",
      "action": "click",
      "target": "role=button[name=\"Compare\"s]",
      "label": "Click Compare to see what changed between two drafts",
      "text": null,
      "key": null,
      "body": "In the screenplay toolbar, click Compare to see what changed between two drafts."
    },
    {
      "screen": "308-sp-compare-text-only",
      "action": "click",
      "target": "role=button[name=\"Back to script\"]",
      "label": "Changes are listed and marked side by side. Back to script",
      "text": null,
      "key": null,
      "body": "Before and After draft lists sit side by side, with a CHANGES list and a summary of scenes added, removed, changed and moved. Click \"Back to script\"."
    },
    {
      "screen": "244-sp-editor-standard",
      "action": "click",
      "target": "role=button[name=\"Lock\"s]",
      "label": "When the script is ready to shoot, click Lock",
      "text": null,
      "key": null,
      "body": "When the script is ready to shoot, click Lock."
    },
    {
      "screen": "312-sp-lock-dialog",
      "action": "click",
      "target": "role=button[name=\"Lock as Shooting Draft\"]",
      "label": "Check the details and click Lock as Shooting Draft",
      "text": null,
      "key": null,
      "body": "The dialog asks \"Lock this draft as the Shooting Draft?\" and lists the draft name, version, unresolved review notes and scene count. Click \"Lock as Shooting Draft\"."
    },
    {
      "screen": "313-sp-locked-draft",
      "action": "key",
      "target": "[data-id=\"01a15b74-8187-75c2-a21e-f48d9f1e3718\"]",
      "label": "Start typing in the locked draft",
      "text": null,
      "key": "X",
      "body": "A banner says it is a locked shooting draft that you can read and export freely. Start typing and OpenFrame offers a revision instead."
    },
    {
      "screen": "314-sp-locked-edit-dialog",
      "action": "click",
      "target": "role=button[name=\"Start Revision\"]",
      "label": "OpenFrame offers a revision instead. Click Start Revision",
      "text": null,
      "key": null,
      "body": "The message says \"This is your locked shooting draft. Create a revision?\" and \"Nothing has been changed yet.\" Click Start Revision."
    },
    {
      "screen": "315-sp-start-revision-dialog",
      "action": "click",
      "target": "role=radio[name=\"Pink\"]",
      "label": "Pick a colour and add a reason if you like",
      "text": null,
      "key": null,
      "body": "The Start Revision dialog says the locked draft never changes. Give it a Label, pick a Colour such as Pink, and add a reason if you like."
    },
    {
      "screen": "316-sp-start-revision-filled",
      "action": "click",
      "target": "role=dialog >> role=button[name=\"Start Revision\"]",
      "label": "Click Start Revision",
      "text": null,
      "key": null,
      "body": "With the label, colour and reason set, click Start Revision."
    },
    {
      "screen": "319-sp-revision-draft-editor",
      "action": "view",
      "target": null,
      "label": "You now write in Revision A; the locked draft never changes",
      "text": null,
      "key": null,
      "body": "You now write in the revision, shown in the draft pill as \"Revision A - Pink\". The locked draft never changes."
    }
  ],
  "result": "A locked shooting draft and a coloured revision you can write in safely.",
  "mistakes": [
    "Locking too early. A locked draft changes only through revisions, so lock when the script is ready to shoot.",
    "Editing the locked draft directly. Typing in it offers a revision, which keeps the locked draft intact."
  ],
  "troubleshooting": [
    [
      "I typed in a locked draft and a dialog appeared",
      "Nothing has been changed yet. Click Start Revision to keep writing, or cancel."
    ],
    [
      "I want a new draft without locking",
      "Use Drafts in the toolbar, then New Draft. It can start from the current draft or a previous named draft."
    ]
  ],
  "related": [
    "write-screenplay",
    "screenplay-notes",
    "export-script",
    "backup-restore"
  ],
  "next": "import-script"
};
