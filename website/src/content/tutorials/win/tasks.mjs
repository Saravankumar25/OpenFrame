export default {
  "id": "tasks",
  "available": true,
  "reason": "",
  "path": "shoot",
  "order": 5,
  "feature": null,
  "title": "Add a task linked to a scene or item",
  "purpose": "Add a task with a due date and link it to the scene, location or catalog item it is about.",
  "difficulty": "Beginner",
  "minutes": 4,
  "prerequisites": [
    "A project is open."
  ],
  "objectives": [
    "Open Notes & Tasks",
    "Create a task with a due date",
    "Link it to a related item"
  ],
  "steps": [
    {
      "screen": "014-shell-project-menu",
      "action": "click",
      "target": "role=menuitem[name=\"Notes & Tasks\"]",
      "label": "In the project menu, choose Notes & Tasks",
      "text": null,
      "key": null,
      "body": "Open the project menu and choose Notes & Tasks. The page says \"Small things that do not belong anywhere else.\" It is also reachable from Project Home quick access."
    },
    {
      "screen": "717-notes-filled",
      "action": "click",
      "target": "role=button[name=\"New Task\"]",
      "label": "Click New Task",
      "text": null,
      "key": null,
      "body": "Click New Task. The task drawer opens."
    },
    {
      "screen": "725-notes-task-editor-new",
      "action": "type",
      "target": "#te-title",
      "label": "Type the task, set a due date and add notes",
      "text": "Tour task call the depot",
      "key": null,
      "body": "Type the Task * title, set a Due date and add Notes. A task needs a title, otherwise you see \"Give the task a short title.\""
    },
    {
      "screen": "727-notes-task-editor-filled",
      "action": "type",
      "target": "input[aria-label=\"Find a scene, location, character, shooting day or call sheet\"]",
      "label": "Under Related to, search for what the task is about",
      "text": "bus",
      "key": null,
      "body": "Under Related to, type a few letters of what the task is about in the search box. It finds scenes, locations, characters, shooting days and call sheets."
    },
    {
      "screen": "728-notes-task-editor-related-results",
      "action": "click",
      "target": "role=option[name^=\"Catalog Item\"]",
      "label": "Pick the related item",
      "text": null,
      "key": null,
      "body": "Pick the related item from the results, here the catalog item Night bus."
    },
    {
      "screen": "729-notes-task-editor-related-chosen",
      "action": "click",
      "target": "role=button[name=\"Add Task\"]",
      "label": "Click Add Task",
      "text": null,
      "key": null,
      "body": "The related item is attached. Click Add Task."
    },
    {
      "screen": "730-notes-task-added-toast",
      "action": "view",
      "target": null,
      "label": "The task joins the open tasks with its link and due date",
      "text": null,
      "key": null,
      "body": "The toast says the task was added and offers Undo. The task joins the open tasks with its link and due date. Completing a task never changes the related item."
    }
  ],
  "result": "An open task with a due date and a link to the item it concerns. Use the Owner filter and Open, Done and All to sort tasks.",
  "mistakes": [
    "Leaving the title empty. Add Task needs a short title.",
    "Expecting a done task to change the scene. Completing it does not touch the related item.",
    "Writing long notes as tasks. New Note on the same page is for notes."
  ],
  "troubleshooting": [
    [
      "The related search finds nothing.",
      "Type a different part of the name. It searches scenes, locations, characters, shooting days and call sheets."
    ],
    [
      "I added a task by mistake.",
      "Click Undo in the toast, or press Ctrl+Z."
    ],
    [
      "I cannot find the page.",
      "Open the project menu or the quick access on Project Home and choose Notes & Tasks."
    ]
  ],
  "related": [
    "recently-deleted",
    "budget",
    "schedule",
    "screenplay-notes"
  ],
  "next": "backup-restore"
};
