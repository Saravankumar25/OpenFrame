export default {
  "id": "call-sheet",
  "available": true,
  "reason": "",
  "path": "shoot",
  "order": 1,
  "feature": "call-sheets",
  "title": "Make a call sheet for a shooting day",
  "purpose": "Create a call sheet from a shooting day, add a section and export it as a PDF.",
  "difficulty": "Intermediate",
  "minutes": 8,
  "prerequisites": [
    "A schedule with at least one shooting day."
  ],
  "objectives": [
    "Create a call sheet from a day",
    "Add an optional section",
    "Open the Export Call Sheet sheet"
  ],
  "steps": [
    {
      "screen": "416-callsheets-empty",
      "action": "click",
      "target": "[data-id=\"callsheet-create\"]",
      "label": "In Call Sheets, click Create Call Sheet",
      "text": null,
      "key": null,
      "body": "Open Call Sheets in the sidebar. An empty project says \"Create a call sheet from a shooting day.\" Click Create Call Sheet."
    },
    {
      "screen": "417-callsheet-create-sheet",
      "action": "click",
      "target": "[data-id=\"callsheet-create-confirm\"]",
      "label": "Pick the shooting day and template, then click Create Call Sheet",
      "text": null,
      "key": null,
      "body": "In the Create Call Sheet sheet, pick the Source shooting day and a Template, then click Create Call Sheet. The sheet lists what is filled in automatically. You add call times, weather and day-specific notes."
    },
    {
      "screen": "420-callsheet-editor",
      "action": "click",
      "target": "[data-id=\"callsheet-add-section\"]",
      "label": "The call sheet fills in from the schedule. Type call times and contacts in the fields, then click Add Section",
      "text": null,
      "key": null,
      "body": "The call sheet fills in from the schedule. Values the app cannot know show Needs input. Type call times and contacts in the fields, then click Add Section."
    },
    {
      "screen": "421-callsheet-section-menu",
      "action": "click",
      "target": "[data-id=\"menu-item-0\"]",
      "label": "Choose Weather",
      "text": null,
      "key": null,
      "body": "The menu offers Weather, Extras, Department notes, Advance schedule, Crew, Walkie channels and contacts, Reference images, Attachments, Special notes and Custom field. Choose Weather."
    },
    {
      "screen": "422-callsheet-section-weather",
      "action": "click",
      "target": "[data-id=\"callsheet-export\"]",
      "label": "Click Export PDF",
      "text": null,
      "key": null,
      "body": "The Weather section is added to the sheet. Fill it in, then click Export PDF."
    },
    {
      "screen": "428-callsheet-export",
      "action": "view",
      "target": null,
      "label": "The Export Call Sheet sheet creates the PDF snapshot to send. Click Export… and choose where to save",
      "text": null,
      "key": null,
      "body": "The Export Call Sheet sheet creates the PDF snapshot to send. It has a check box to include the added sections. Click Export and choose where to save the file."
    }
  ],
  "result": "A call sheet for one shooting day, ready to export as a PDF.",
  "mistakes": [
    "Sending the sheet while fields still say Needs input. Fill those fields first.",
    "Forgetting to refresh after a schedule change. Use Refresh from schedule."
  ],
  "troubleshooting": [
    [
      "The Create Call Sheet button is not active.",
      "You need a shooting day first. Make one in Schedule."
    ],
    [
      "I want to freeze the version I sent.",
      "Click Finalize. Final freezes the document as a snapshot."
    ]
  ],
  "related": [
    "sides",
    "schedule",
    "reports"
  ],
  "next": "sides"
};
