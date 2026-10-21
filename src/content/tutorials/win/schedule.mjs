export default {
  "id": "schedule",
  "available": true,
  "reason": "",
  "path": "prep",
  "order": 4,
  "feature": "scheduling",
  "title": "Create a shooting day",
  "purpose": "Add a shooting day to the stripboard with a date and notes, then open it.",
  "difficulty": "Intermediate",
  "minutes": 5,
  "prerequisites": [
    "A project is open with scenes broken down.",
    "You are in Production."
  ],
  "objectives": [
    "Open the Schedule tab",
    "Create a shooting day with a date and notes",
    "Open the new day"
  ],
  "steps": [
    {
      "screen": "434-prod-overview",
      "action": "click",
      "target": "role=button[name=\"Schedule\"]",
      "label": "In Production, open the Schedule tab",
      "text": null,
      "key": null,
      "body": "In Production, click the Schedule tab. The Board view shows the Unscheduled pool and the shooting days. The Board, List and Calendar switch changes the view."
    },
    {
      "screen": "533-prod-schedule-board",
      "action": "click",
      "target": "role=button[name=\"Create Shooting Day\"]",
      "label": "Click Create Shooting Day",
      "text": null,
      "key": null,
      "body": "Click Create Shooting Day. A dialog opens."
    },
    {
      "screen": "540-prod-schedule-create-day-dialog",
      "action": "type",
      "target": "#of-day-notes",
      "label": "Pick the shooting date and type day notes",
      "text": "Tower climb and rooftop, night shoot",
      "key": null,
      "body": "Pick the Shooting date, which is optional because a day can exist without a date, and type the Day notes, for example \"Tower climb and rooftop, night shoot\"."
    },
    {
      "screen": "541-prod-schedule-create-day-filled",
      "action": "click",
      "target": "role=button[name=\"Create Day\"]",
      "label": "Click Create Day",
      "text": null,
      "key": null,
      "body": "The date and notes are in. Click Create Day."
    },
    {
      "screen": "542-prod-schedule-day-created",
      "action": "click",
      "target": "role=button[name=\"Shoot Day 4 · Thu 12 Nov 2026\"]",
      "label": "Shoot Day 4 is added to the stripboard; click it",
      "text": null,
      "key": null,
      "body": "The new day, here Shoot Day 4, joins the stripboard. Click it to open the day."
    },
    {
      "screen": "543-prod-schedule-day-detail-empty",
      "action": "view",
      "target": null,
      "label": "The day opens, ready for scenes, breaks and times",
      "text": null,
      "key": null,
      "body": "The day opens ready for scenes, breaks and times. Move a scene in from a strip menu (Scene N actions, then MOVE TO a shoot day) or by dragging the strip. The app never moves scenes by itself."
    }
  ],
  "result": "A new shooting day on the stripboard, open and ready for scenes. Schedule settings holds the target day length and the Draft or Finalized status.",
  "mistakes": [
    "Waiting for the app to place scenes. There is no automatic optimizer. You decide where scenes go.",
    "Leaving the date out and forgetting it. A day without a date still works, but Edit date from the day menu fixes it later.",
    "Ignoring the warnings chip. Open it to see conflicts before the shoot."
  ],
  "troubleshooting": [
    [
      "Create Day does nothing.",
      "Check that a date or notes are filled if you intended them. A day can be created with neither."
    ],
    [
      "A scene will not drag.",
      "Use the strip menu: open Scene N actions and choose a Shoot Day under MOVE TO."
    ],
    [
      "The chip says there are warnings.",
      "Click the warnings chip to open the Schedule assistance drawer and read each card."
    ]
  ],
  "related": [
    "breakdown",
    "cast-crew",
    "call-sheet",
    "locations"
  ],
  "next": "call-sheet"
};
