export default {
  "id": "drafts-revisions",
  "available": true,
  "reason": "",
  "path": "writing",
  "order": 6,
  "feature": "drafts-revisions",
  "title": "Start a new draft and compare it",
  "purpose": "Create a new draft, change a few lines and compare it with the original side by side.",
  "difficulty": "Intermediate",
  "minutes": 6,
  "prerequisites": [
    "A screenplay with some text is open."
  ],
  "objectives": [
    "Create a new draft",
    "Edit it without touching the source draft",
    "Open Compare to see every change"
  ],
  "steps": [
    {
      "screen": "034-home-import-toast",
      "action": "click",
      "target": "[data-id=\"draft-menu\"]",
      "label": "Click the Draft menu at the bottom of the page",
      "text": null,
      "key": null,
      "body": "At the bottom of the screenplay page, click the Draft menu."
    },
    {
      "screen": "080-screenplay-draft-menu",
      "action": "click",
      "target": "[data-id=\"menu-item-2\"]",
      "label": "Choose New Draft…",
      "text": null,
      "key": null,
      "body": "The menu lists your drafts, New Draft..., Manage Drafts... and Renumber Scenes. Choose New Draft...."
    },
    {
      "screen": "081-screenplay-draft-new-draft",
      "action": "type",
      "target": "[data-id=\"new-draft-name\"]",
      "label": "Name the draft",
      "text": "Rewrite A",
      "key": null,
      "body": "The New Draft sheet has Draft name and an optional Note. It says the source draft stays exactly as it is. Type a name for the draft."
    },
    {
      "screen": "135-screenplay-new-draft-named",
      "action": "click",
      "target": "role=button[name=\"Create Draft\"]",
      "label": "Click Create Draft",
      "text": null,
      "key": null,
      "body": "Click Create Draft."
    },
    {
      "screen": "136-screenplay-second-draft-created",
      "action": "type",
      "target": "[data-id=\"screenplay-editor\"]",
      "label": "Rewrite a few lines in the new draft",
      "text": "A red one.",
      "key": null,
      "body": "The new draft opens and the Draft menu shows its name. Rewrite a few lines in it."
    },
    {
      "screen": "137-screenplay-second-draft-edited",
      "action": "click",
      "target": "[data-id=\"compare-button\"]",
      "label": "Click Compare",
      "text": null,
      "key": null,
      "body": "Click Compare in the top bar."
    },
    {
      "screen": "145-screenplay-compare-text-only",
      "action": "view",
      "target": null,
      "label": "Both drafts side by side, with every change listed",
      "text": null,
      "key": null,
      "body": "Both drafts show side by side with Before and After pop-ups. A Changes list names each scene with changed lines. Previous change and Next change step through them, and Done closes Compare."
    }
  ],
  "result": "You have a second draft with your edits, and a side by side view of what changed.",
  "mistakes": [
    "Editing the original draft by accident. Check the Draft menu shows the new draft before you type.",
    "Skipping the name. A clear draft name makes Compare easier to read.",
    "Expecting a locked draft here. Locking is a separate step in Screenplay Tools."
  ],
  "troubleshooting": [
    [
      "Compare shows nothing changed.",
      "Check Before and After point at different drafts, and that you typed in the new one."
    ],
    [
      "I cannot see my other draft.",
      "Use the Draft menu to switch, or Manage Drafts... for the full list."
    ],
    [
      "The comparison is hard to read.",
      "Switch the Comparison view between Text-only, Scene-only and Text inline."
    ]
  ],
  "related": [
    "write-screenplay",
    "screenplay-notes",
    "export-script"
  ],
  "next": "import-script"
};
