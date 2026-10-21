export default {
  "id": "sides",
  "available": true,
  "reason": "",
  "path": "shoot",
  "order": 2,
  "feature": "sides-daily-view",
  "title": "Save and export sides for a day",
  "purpose": "Choose a shooting day, preview its script pages and save the sides as a snapshot.",
  "difficulty": "Beginner",
  "minutes": 5,
  "prerequisites": [
    "A schedule with a shooting day that has scenes."
  ],
  "objectives": [
    "Preview sides for a shooting day",
    "Add a cover page",
    "Save sides and open the snapshot"
  ],
  "steps": [
    {
      "screen": "401-schedule-sides",
      "action": "click",
      "target": "[data-id=\"sides-cover\"]",
      "label": "In Production, Sides, Shoot Day 1 is chosen with its scenes. Tick Include call-sheet cover page",
      "text": null,
      "key": null,
      "body": "Open Production, then Sides. The page asks \"What script pages does this team need for this shooting day?\". In Setup, pick the Shooting day and its scenes, and choose All cast or Selected cast. Tick Include call-sheet cover page."
    },
    {
      "screen": "404-schedule-sides-cover",
      "action": "click",
      "target": "[data-id=\"sides-save\"]",
      "label": "The preview adds the cover page. Click Save Sides",
      "text": null,
      "key": null,
      "body": "The preview adds the cover page. There is also a check box to show the draft or revision name. Click Save Sides."
    },
    {
      "screen": "406-schedule-sides-saved",
      "action": "click",
      "target": "[data-id=\"sides-saved-01a15ba9-b96f-70f8-b295-d391343ab26a\"]",
      "label": "The sides are saved as a snapshot. Click the saved entry to open it",
      "text": null,
      "key": null,
      "body": "The sides are saved as a snapshot and appear under Saved sides. Click the saved entry to open it."
    },
    {
      "screen": "407-schedule-sides-snapshot",
      "action": "view",
      "target": null,
      "label": "The saved sides open. Export PDF makes the file for the team",
      "text": null,
      "key": null,
      "body": "The saved sides open. Later script edits do not change them. Export PDF makes the file for the team, and Back to live preview returns to the current script."
    }
  ],
  "result": "Saved sides for one shooting day that you can export as a PDF.",
  "mistakes": [
    "Expecting a saved snapshot to update when the script changes. Save new sides after a revision.",
    "Leaving Selected cast on by accident. Check the For setting before saving."
  ],
  "troubleshooting": [
    [
      "No scenes are listed.",
      "Place scenes in a shooting day in Schedule first."
    ],
    [
      "The cover page is missing.",
      "Tick Include call-sheet cover page and preview again."
    ]
  ],
  "related": [
    "call-sheet",
    "reports",
    "schedule"
  ],
  "next": "reports"
};
