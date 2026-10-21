export default {
  "id": "tasks",
  "available": true,
  "reason": "",
  "path": "shoot",
  "order": 5,
  "feature": null,
  "title": "Add a task with a due date",
  "purpose": "Create a task in Notes & Tasks with a due date and mark it done later.",
  "difficulty": "Beginner",
  "minutes": 4,
  "prerequisites": [
    "A project is open."
  ],
  "objectives": [
    "Open Notes & Tasks",
    "Create a task with a due date",
    "Mark a task done"
  ],
  "steps": [
    {
      "screen": "588-pages-notes-two",
      "action": "click",
      "target": "[data-id=\"new-task\"]",
      "label": "In Notes & Tasks, click New Task",
      "text": null,
      "key": null,
      "body": "Open Notes & Tasks from the Project Home button, the project menu or Go > Notes and Tasks. The page says \"Small things that do not belong anywhere else.\" Click New Task."
    },
    {
      "screen": "589-tasks-new-task",
      "action": "type",
      "target": "[data-id=\"task-title\"]",
      "label": "Type the task",
      "text": "Book the lamp rig",
      "key": null,
      "body": "The New Task drawer opens. Type the task in the Task field."
    },
    {
      "screen": "590-pages-tasks-titled",
      "action": "click",
      "target": "[data-id=\"task-has-due\"]",
      "label": "Tick Set a due date and pick the date",
      "text": null,
      "key": null,
      "body": "Tick Set a due date and pick the date with the date picker."
    },
    {
      "screen": "591-pages-tasks-due",
      "action": "click",
      "target": "[data-id=\"task-save\"]",
      "label": "Choose an owner from Owner if needed, then click Add Task",
      "text": null,
      "key": null,
      "body": "Choose an owner from the Owner pop-up if needed. Related to can link a scene, location, character, shooting day or call sheet. Completing the task never changes the related item. Click Add Task."
    },
    {
      "screen": "594-notes-and-tasks",
      "action": "view",
      "target": null,
      "label": "The task is in the Tasks list with its due date. Tick its box to mark it done",
      "text": null,
      "key": null,
      "body": "The task appears in the Tasks list with its due date. Tick its check box to mark it done. The filter shows Open, Done or All."
    }
  ],
  "result": "A task with a due date in your Tasks list.",
  "mistakes": [
    "Using a task as a note. Use New Note for text that is not a to-do.",
    "Expecting a linked item to change when the task is done. It stays as it was."
  ],
  "troubleshooting": [
    [
      "I cannot find Notes & Tasks.",
      "Use the Project Home button, the project menu item, or the Go menu in the menu bar."
    ],
    [
      "A finished task vanished from the list.",
      "The filter is on Open. Switch it to Done or All."
    ]
  ],
  "related": [
    "files",
    "call-sheet",
    "recently-deleted"
  ],
  "next": "backup-restore"
};
