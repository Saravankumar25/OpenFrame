export default {
  "id": "call-sheet",
  "available": true,
  "reason": "",
  "path": "shoot",
  "order": 1,
  "feature": "call-sheets",
  "title": "Create a call sheet from a shooting day",
  "purpose": "Make a call sheet from a shooting day and see how to add extra sections.",
  "difficulty": "Intermediate",
  "minutes": 5,
  "prerequisites": [
    "A project is open with at least one shooting day in the schedule."
  ],
  "objectives": [
    "Create a call sheet from a day",
    "See what the app fills in from the schedule",
    "Add a section"
  ],
  "steps": [
    {
      "screen": "636-cs-list",
      "action": "click",
      "target": "role=button[name=\"Create Call Sheet\"]",
      "label": "Click Create Call Sheet",
      "text": null,
      "key": null,
      "body": "Open Call Sheets in the left rail. The list has search and a filter for All, Draft, Final and Needs refresh. Click Create Call Sheet."
    },
    {
      "screen": "643-cs-create-dialog",
      "action": "click",
      "target": "select[aria-label=\"Source shooting day\"]",
      "label": "Choose the shooting day, here Shoot Day 4",
      "text": null,
      "key": null,
      "body": "The dialog asks for the \"Source shooting day *\". Use the list to choose the day, here Shoot Day 4. The dialog lists what is filled in automatically, such as title, date, day times, scene schedule, talent and locations."
    },
    {
      "screen": "644-cs-create-dialog-day-chosen",
      "action": "click",
      "target": "role=button[name=\"Create Call Sheet\"]",
      "label": "Click Create Call Sheet",
      "text": null,
      "key": null,
      "body": "With the day chosen, click Create Call Sheet. Anything nobody has entered reads Needs input."
    },
    {
      "screen": "645-cs-editor-draft",
      "action": "click",
      "target": "role=button[name=\"Add Section\"]",
      "label": "The draft opens filled from the schedule; click Add Section",
      "text": null,
      "key": null,
      "body": "The draft opens filled from the schedule. You add call times, weather and day-specific notes. Click Add Section."
    },
    {
      "screen": "647-cs-add-section-menu",
      "action": "view",
      "target": null,
      "label": "Add weather, crew, walkie channels, notes and more, then Finalize and Export PDF",
      "text": null,
      "key": null,
      "body": "The menu lists Weather, Extras, Department notes, Advance schedule, Crew, Walkie channels and contacts, Reference images, Attachments, Special notes and Custom field. Finalize needs a crew call, and Export PDF makes the PDF."
    }
  ],
  "result": "A draft call sheet linked to its shooting day. When the schedule changes later, the sheet shows Needs refresh with Review Update and Refresh from schedule.",
  "mistakes": [
    "Finalizing before a crew call is set. Finalize needs one.",
    "Editing the finished sheet and expecting the schedule to change. The call sheet follows the schedule, not the other way round.",
    "Ignoring Needs input. Those fields stay empty until someone fills them."
  ],
  "troubleshooting": [
    [
      "The sheet shows \"Needs refresh.\"",
      "The schedule changed after the sheet was made. Click Review Update, then Refresh from schedule."
    ],
    [
      "I need a change after finalizing.",
      "Use Start New Revision."
    ],
    [
      "Export PDF asks where to save.",
      "The Windows Save dialog opens. Pick a folder and file name."
    ]
  ],
  "related": [
    "schedule",
    "sides",
    "locations",
    "cast-crew"
  ],
  "next": "sides"
};
