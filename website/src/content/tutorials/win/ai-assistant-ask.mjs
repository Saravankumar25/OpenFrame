export default {
  "id": "ai-assistant-ask",
  "available": true,
  "reason": "",
  "path": "ai",
  "order": 2,
  "feature": "ai-assistant",
  "title": "Ask the assistant and review changes",
  "purpose": "Ask a question, check its sources, and apply a proposed change only when you choose.",
  "difficulty": "Intermediate",
  "minutes": 7,
  "prerequisites": [
    "The offline assistant is set up and shows \"AI Ready\" (the one-time download of about 1.2 GB, which you start).",
    "A project is open."
  ],
  "objectives": [
    "Ask a question and read the sources",
    "Ask for a change",
    "Review a Proposed Changes card",
    "Apply it, and undo it if needed"
  ],
  "steps": [
    {
      "screen": "013-shell-project-home",
      "action": "click",
      "target": "button[aria-label=\"Assistant\"]",
      "label": "Click the Assistant button",
      "text": null,
      "key": null,
      "body": "The assistant is optional and works offline once set up. Click the Assistant button in the top bar."
    },
    {
      "screen": "374-ai-panel-ready",
      "action": "type",
      "target": "textarea[aria-label=\"Ask or command\"]",
      "label": "Type a question",
      "text": "List the characters.",
      "key": null,
      "body": "The AI Assistant panel shows \"AI Ready\" and a Scope, such as \"Using: Whole Project\". Type a question in the \"Ask or command…\" box."
    },
    {
      "screen": "381-ai-question-typed",
      "action": "click",
      "target": "role=button[name=\"Send\"s]",
      "label": "Click Send",
      "text": null,
      "key": null,
      "body": "Click Send. The panel shows \"Working on it on this computer…\" while it answers."
    },
    {
      "screen": "383-ai-answer-characters",
      "action": "click",
      "target": "text=See sources",
      "label": "The answer comes from your project. See sources shows what it used",
      "text": null,
      "key": null,
      "body": "The answer comes from your project. Click \"See sources\" to see what it used."
    },
    {
      "screen": "384-ai-answer-characters-sources",
      "action": "view",
      "target": null,
      "label": "Sources list the scenes and steps behind the answer",
      "text": null,
      "key": null,
      "body": "Sources list the scenes and steps behind the answer. Next, to try a change, open the screenplay and click the Assistant button there."
    },
    {
      "screen": "244-sp-editor-standard",
      "action": "click",
      "target": "button[aria-label=\"Assistant\"]",
      "label": "In the Screenplay, click the Assistant button",
      "text": null,
      "key": null,
      "body": "In the Screenplay, click the Assistant button. The panel opens over your script."
    },
    {
      "screen": "385-ai-panel-screenplay",
      "action": "type",
      "target": "textarea[aria-label=\"Ask or command\"]",
      "label": "Ask for a change and click Send",
      "text": "Create a task to book the bus for the night shoot.",
      "key": null,
      "body": "Ask for a change and click Send. The answer says \"I prepared a task. Nothing has been added yet.\""
    },
    {
      "screen": "391-ai-proposed-changes-second",
      "action": "click",
      "target": "role=button[name=\"Apply Changes\"]",
      "label": "Review the Proposed Changes. Nothing is added until you apply",
      "text": null,
      "key": null,
      "body": "The Proposed Changes card shows what is affected and the impact. Nothing is added until you apply it. Click \"Apply Changes\", or Reject to change nothing."
    },
    {
      "screen": "392-ai-proposal-applied",
      "action": "view",
      "target": null,
      "label": "Applied as one step you can undo",
      "text": null,
      "key": null,
      "body": "It says \"Applied 1 change. Use Undo to reverse it.\" The change is one step you can undo."
    }
  ],
  "result": "An answer with sources, and a change that was only made when you clicked Apply Changes.",
  "mistakes": [
    "Assuming the assistant edits your project on its own. It only prepares changes. Nothing changes until you click Apply Changes.",
    "Trusting an answer without checking. Open \"See sources\" to see what it used."
  ],
  "troubleshooting": [
    [
      "I applied something I did not want",
      "Click Undo on the toast or press Ctrl+Z. Applied changes are one undoable step."
    ],
    [
      "I clicked Reject",
      "The panel says \"Rejected. Nothing was changed.\" Ask again with more detail."
    ]
  ],
  "related": [
    "ai-assistant-setup",
    "write-screenplay",
    "tasks",
    "backup-restore"
  ],
  "next": "backup-restore"
};
