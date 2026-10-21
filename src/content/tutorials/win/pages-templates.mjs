export default {
  "id": "pages-templates",
  "available": true,
  "reason": "",
  "path": "writing",
  "order": 9,
  "feature": null,
  "title": "Project notes and templates",
  "purpose": "Write a project note in Notes & Tasks and save your own template.",
  "difficulty": "Beginner",
  "minutes": 6,
  "prerequisites": [
    "A project is open on its Home page."
  ],
  "objectives": [
    "Open Notes & Tasks",
    "Write a note",
    "Open Templates",
    "Create a template"
  ],
  "steps": [
    {
      "screen": "013-shell-project-home",
      "action": "click",
      "target": "button[aria-label=\"Project: Night Bus. Switch or manage projects\"]",
      "label": "Open the project menu",
      "text": null,
      "key": null,
      "body": "OpenFrame has no separate Pages documents. Project notes live in Notes & Tasks. Click the project name at the top to open the project menu."
    },
    {
      "screen": "014-shell-project-menu",
      "action": "click",
      "target": "role=menuitem[name=\"Notes & Tasks\"]",
      "label": "Choose Notes & Tasks",
      "text": null,
      "key": null,
      "body": "Choose \"Notes & Tasks\". You can also reach it from Project Home."
    },
    {
      "screen": "717-notes-filled",
      "action": "click",
      "target": "role=button[name=\"New Note\"]",
      "label": "Click New Note",
      "text": null,
      "key": null,
      "body": "The page has New Note, New Task and \"Search notes\", with notes on the left and Tasks on the right. Click New Note."
    },
    {
      "screen": "722-notes-new-note-editor",
      "action": "type",
      "target": "#ne-title",
      "label": "Give it a title and write the note",
      "text": "Tour note on the radio tower",
      "key": null,
      "body": "Give the note a title in the title box, then write the note."
    },
    {
      "screen": "723-notes-new-note-filled",
      "action": "view",
      "target": null,
      "label": "The note saves as you type and joins the list",
      "text": null,
      "key": null,
      "body": "The note saves as you type and joins the list. Deleting it moves it to Recently Deleted. Next, open the project menu again and choose Templates."
    },
    {
      "screen": "749-templates-page",
      "action": "click",
      "target": "role=button[name=\"Create Template\"]",
      "label": "Built-in templates are listed. Click Create Template",
      "text": null,
      "key": null,
      "body": "Built-in templates are listed, such as Feature Film Screenplay, Indie Call Sheet and Shot List. Click Create Template."
    },
    {
      "screen": "752-templates-create-dialog",
      "action": "type",
      "target": "#tp-name",
      "label": "Name it, pick a type and where to keep it",
      "text": "Tour global call sheet",
      "key": null,
      "body": "In the Create Template dialog, type a name in \"Name *\", pick a Type and choose where to keep it: This project or Every project."
    },
    {
      "screen": "754-templates-create-filled",
      "action": "click",
      "target": "role=dialog >> role=button[name=\"Create\"s]",
      "label": "Click Create",
      "text": null,
      "key": null,
      "body": "Click Create."
    },
    {
      "screen": "755-templates-created-toast",
      "action": "view",
      "target": null,
      "label": "Your template joins the list",
      "text": null,
      "key": null,
      "body": "A toast confirms it and your template joins the list. Each card has Preview, and your own also have Edit and Delete."
    }
  ],
  "result": "A saved project note and your own reusable template.",
  "mistakes": [
    "Leaving the template name empty. The dialog says \"Give the template a name.\"",
    "Putting ideas in Notes & Tasks. Use the Idea Vault for ideas and notes for anything that belongs elsewhere."
  ],
  "troubleshooting": [
    [
      "I cannot find a deleted note",
      "Open Recently Deleted from the project menu. Deleted items stay 30 days."
    ],
    [
      "I cannot find Templates",
      "It is in the project menu and is also a tab of Project settings."
    ]
  ],
  "related": [
    "idea-vault",
    "tasks",
    "settings",
    "project-tour"
  ],
  "next": "ai-assistant-setup"
};
