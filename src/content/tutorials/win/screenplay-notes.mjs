export default {
  "id": "screenplay-notes",
  "available": true,
  "reason": "",
  "path": "writing",
  "order": 5,
  "feature": "screenplay",
  "title": "Comments and private scene notes",
  "purpose": "Comment on script text and keep a private note on a scene.",
  "difficulty": "Intermediate",
  "minutes": 6,
  "prerequisites": [
    "A screenplay is open in the editor. On Project Home, click \"Continue screenplay\" to open it."
  ],
  "objectives": [
    "Add a comment to selected text",
    "Open Writing Room Mode",
    "Add a private note that only you see"
  ],
  "steps": [
    {
      "screen": "244-sp-editor-standard",
      "action": "drag",
      "target": "[data-id=\"01a15b74-8187-75c2-a21e-f48d9f1e3718\"]",
      "label": "Select some words in the script",
      "text": null,
      "key": null,
      "body": "In the open screenplay, select some words in the script."
    },
    {
      "screen": "260-sp-comment-float-button",
      "action": "click",
      "target": ".spx-cmt-btn",
      "label": "Click the Comment button that appears",
      "text": null,
      "key": null,
      "body": "A floating Comment button appears. Click it."
    },
    {
      "screen": "261-sp-comment-composer-text",
      "action": "view",
      "target": null,
      "label": "Write the comment in the Comments panel and click Comment",
      "text": null,
      "key": null,
      "body": "The Comments panel opens with a composer labelled for the scene and quoting your selected words. Write the comment and click Comment. Each comment then has Reply and Resolve."
    },
    {
      "screen": "244-sp-editor-standard",
      "action": "click",
      "target": "button[aria-label=\"View and more\"]",
      "label": "Open View and more",
      "text": null,
      "key": null,
      "body": "Private notes live in Scene Notes. Click \"View and more\" in the toolbar."
    },
    {
      "screen": "268-sp-view-menu",
      "action": "click",
      "target": "role=menuitem[name=\"Writing Room Mode\"]",
      "label": "Choose Writing Room Mode",
      "text": null,
      "key": null,
      "body": "Choose Writing Room Mode. Panels open as tabs on the right."
    },
    {
      "screen": "277-sp-writing-room-beat-board",
      "action": "click",
      "target": "role=button[name=\"Beat Board\"s]",
      "label": "Panels open as tabs on the right. Close Beat Board",
      "text": null,
      "key": null,
      "body": "The Beat Board tab is open. Close it to make room."
    },
    {
      "screen": "279-sp-writing-room-no-panel",
      "action": "click",
      "target": "role=button[name=\"Scene Notes\"]",
      "label": "Open Scene Notes",
      "text": null,
      "key": null,
      "body": "With no panel open, the page stays centred. Click Scene Notes."
    },
    {
      "screen": "280-sp-panel-scene-notes",
      "action": "click",
      "target": "role=button[name=\"Add private note\"]",
      "label": "Click Add private note and type",
      "text": null,
      "key": null,
      "body": "Scene Notes has a \"Private note\" marked \"Only you\" and a \"Scene note\". Click \"Add private note\" and type."
    },
    {
      "screen": "282-sp-panel-private-note-composer",
      "action": "view",
      "target": null,
      "label": "Only you see private notes. Click Save",
      "text": null,
      "key": null,
      "body": "Only you see private notes. Click Save. Notes are not printed and are left out of PDF, FDX, Fountain and DOCX exports and review packages by default."
    }
  ],
  "result": "A shared comment on script text and a private note on a scene.",
  "mistakes": [
    "Putting private thoughts in a comment. Comments are shared review notes. Use a private note for anything only you should see.",
    "Looking for the notes under Standard Mode only. In Writing Room Mode, open the Scene Notes tab on the right."
  ],
  "troubleshooting": [
    [
      "The Comment button does not show",
      "Select text in the script first. The button appears next to the selection."
    ],
    [
      "I cannot see my notes in the script",
      "Tick \"Show notes in the script\" in Scene Notes, or use \"Hide notes in the script\" in View and more."
    ]
  ],
  "related": [
    "write-screenplay",
    "drafts-revisions",
    "export-script",
    "tasks"
  ],
  "next": "drafts-revisions"
};
