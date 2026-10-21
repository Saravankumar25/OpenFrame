export default {
  "id": "schedule",
  "available": true,
  "reason": "",
  "path": "prep",
  "order": 4,
  "feature": "scheduling",
  "title": "Build a shooting schedule",
  "purpose": "Create a schedule, add shooting days, place scenes in them and read the warnings.",
  "difficulty": "Intermediate",
  "minutes": 10,
  "prerequisites": [
    "A project with a screenplay draft.",
    "Breakdown, cast and locations help the warnings, but are not required."
  ],
  "objectives": [
    "Create a shooting schedule",
    "Add shooting days",
    "Move scenes into days",
    "Read the schedule warnings"
  ],
  "steps": [
    {
      "screen": "357-schedule-empty",
      "action": "click",
      "target": "[data-id=\"create-shooting-schedule\"]",
      "label": "In Production, Schedule, click Create Shooting Schedule",
      "text": null,
      "key": null,
      "body": "Open Production, then Schedule. An empty project says \"Create a shooting schedule from your screenplay.\" Click Create Shooting Schedule."
    },
    {
      "screen": "358-schedule-create-sheet",
      "action": "click",
      "target": "[data-id=\"create-schedule-confirm\"]",
      "label": "Check the screenplay source and click Create Schedule",
      "text": null,
      "key": null,
      "body": "The Create Shooting Schedule sheet shows the screenplay source. Check it and click Create Schedule. All scenes start in the Unscheduled pool."
    },
    {
      "screen": "359-schedule-board-unscheduled",
      "action": "click",
      "target": "[data-id=\"create-shooting-day\"]",
      "label": "All 10 scenes start in Unscheduled. Click Create Shooting Day",
      "text": null,
      "key": null,
      "body": "The board shows the Unscheduled pool. Click Create Shooting Day."
    },
    {
      "screen": "360-schedule-add-day-sheet",
      "action": "click",
      "target": "[data-id=\"day-sheet-confirm\"]",
      "label": "Enter a shooting date (YYYY-MM-DD, optional) and click Create Day. Repeat for each day",
      "text": null,
      "key": null,
      "body": "In the Create Shooting Day sheet, enter a shooting date in the form YYYY-MM-DD. The date is optional. Click Create Day. Repeat for each day."
    },
    {
      "screen": "362-schedule-board-three-days",
      "action": "click",
      "target": "[data-id=\"strip-actions-1\"]",
      "label": "Three days are on the board. Open the actions of the Scene 1 strip",
      "text": null,
      "key": null,
      "body": "Three days are on the board. Open the actions of the Scene 1 strip."
    },
    {
      "screen": "365-schedule-strip-menu",
      "action": "click",
      "target": "[data-id=\"menu-item-2\"]",
      "label": "Choose Move to and pick a shooting day, or drag the strip onto a day",
      "text": null,
      "key": null,
      "body": "Choose Move to and pick a shooting day, or drag the strip onto a day. A message confirms the move. No automatic tool ever moves scenes for you."
    },
    {
      "screen": "366-schedule-board",
      "action": "click",
      "target": "[data-id=\"warnings-button\"]",
      "label": "The scenes sit in their days. Click the warnings button",
      "text": null,
      "key": null,
      "body": "The scenes now sit in their days. Click the warnings button in the toolbar."
    },
    {
      "screen": "389-schedule-warnings",
      "action": "view",
      "target": null,
      "label": "Schedule assistance lists practical warnings, such as an actor needed at two locations, with Open affected item, Move scene and Keep anyway",
      "text": null,
      "key": null,
      "body": "Schedule assistance lists practical warnings, such as an actor needed at two locations. Each one offers Open affected item, Move scene and Keep anyway. Warnings never block you and nothing is fixed automatically."
    }
  ],
  "result": "A shooting schedule with days, scenes placed in them, and warnings you have read.",
  "mistakes": [
    "Treating warnings as errors. They are advice; you can use Keep anyway.",
    "Waiting for the app to arrange scenes. You place scenes yourself."
  ],
  "troubleshooting": [
    [
      "Create Shooting Day asks for a date I do not have.",
      "The date is optional. Leave it empty and add it later."
    ],
    [
      "I do not see my scenes on the board.",
      "Scenes you have not placed stay in the Unscheduled pool. Check the filters too."
    ]
  ],
  "related": [
    "cast-crew",
    "call-sheet",
    "locations"
  ],
  "next": "call-sheet"
};
