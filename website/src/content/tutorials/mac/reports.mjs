export default {
  "id": "reports",
  "available": true,
  "reason": "",
  "path": "shoot",
  "order": 3,
  "feature": "reports-budget",
  "title": "Read production reports",
  "purpose": "Switch between the production reports and find the filter, snapshot and export tools.",
  "difficulty": "Beginner",
  "minutes": 4,
  "prerequisites": [
    "A project with a breakdown and, ideally, a schedule."
  ],
  "objectives": [
    "Open the report list",
    "Move between reports",
    "Save a snapshot or export"
  ],
  "steps": [
    {
      "screen": "408-schedule-reports-scene",
      "action": "click",
      "target": "[data-id=\"reports-type-location\"]",
      "label": "In Production, Reports, the Scene Report shows. Click Location Report",
      "text": null,
      "key": null,
      "body": "Open Production, then Reports. The Scene Report shows first. Click Location Report in the report list."
    },
    {
      "screen": "409-schedule-reports-location",
      "action": "click",
      "target": "[data-id=\"reports-type-cast-scene\"]",
      "label": "Click Cast Scene Report",
      "text": null,
      "key": null,
      "body": "The Location Report lists the places and their scenes. Click Cast Scene Report."
    },
    {
      "screen": "410-schedule-reports-cast-scene",
      "action": "click",
      "target": "[data-id=\"reports-type-prop\"]",
      "label": "Click Prop Report",
      "text": null,
      "key": null,
      "body": "The Cast Scene Report shows which cast appear in which scenes. Click Prop Report."
    },
    {
      "screen": "411-schedule-reports-prop",
      "action": "view",
      "target": null,
      "label": "Each report is built live from the breakdown and the schedule. Use the filter, Save snapshot or Export",
      "text": null,
      "key": null,
      "body": "Each report is built live from the breakdown and the schedule. The list also holds the Schedule Report and Breakdown Completeness. Use the Filter report rows field, Save snapshot to keep a copy, or Export to make a file."
    }
  ],
  "result": "You can move between the six reports and keep a snapshot or export.",
  "mistakes": [
    "Reading a report built from an old draft. Check the Source line at the top.",
    "Expecting a live report to stay as it was. Click Save snapshot to keep one."
  ],
  "troubleshooting": [
    [
      "A report looks empty.",
      "Reports come from the breakdown and the schedule. Tag elements and place scenes first."
    ],
    [
      "I need the report as a file.",
      "Click Export and pick where to save."
    ]
  ],
  "related": [
    "budget",
    "sides",
    "breakdown"
  ],
  "next": "budget"
};
