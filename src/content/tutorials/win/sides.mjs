export default {
  "id": "sides",
  "available": true,
  "reason": "",
  "path": "shoot",
  "order": 2,
  "feature": "sides-daily-view",
  "title": "Prepare sides for one actor",
  "purpose": "Build the script pages one actor needs for a shooting day.",
  "difficulty": "Intermediate",
  "minutes": 4,
  "prerequisites": [
    "A project is open with a scheduled shooting day.",
    "Cast are assigned to characters."
  ],
  "objectives": [
    "Open the Sides tab",
    "Choose a shooting day and selected cast",
    "Preview the pages"
  ],
  "steps": [
    {
      "screen": "597-prod-daily-day-1",
      "action": "click",
      "target": "role=button[name=\"Sides\"]",
      "label": "From the Daily View, open the Sides tab",
      "text": null,
      "key": null,
      "body": "In Production, the Daily View asks \"What do I need to know for this day?\" From there, click the Sides tab. It asks \"What script pages does this team need for this shooting day?\""
    },
    {
      "screen": "602-prod-sides-setup",
      "action": "click",
      "target": "select[aria-label=\"Shooting day\"]",
      "label": "Choose the shooting day, here Shoot Day 4",
      "text": null,
      "key": null,
      "body": "Under SETUP, open the \"Shooting day\" list and choose a day, here Shoot Day 4. Each scene of that day appears with a tick box."
    },
    {
      "screen": "603-prod-sides-day-4",
      "action": "click",
      "target": "role=radio[name=\"Selected cast\"]",
      "label": "Under For, choose Selected cast",
      "text": null,
      "key": null,
      "body": "Under For, choose Selected cast instead of All cast. Until you tick someone, the app says to choose a shooting day, a scene and a cast member to preview."
    },
    {
      "screen": "604-prod-sides-selected-cast-none",
      "action": "click",
      "target": "label:has-text(\"Ananya Rao (LAKSHMI)\") input",
      "label": "Tick the actor who needs sides",
      "text": null,
      "key": null,
      "body": "Tick the actor who needs the pages, here Ananya Rao (LAKSHMI)."
    },
    {
      "screen": "605-prod-sides-selected-cast",
      "action": "view",
      "target": null,
      "label": "The preview shows that actor's pages; Save Sides or Export PDF",
      "text": null,
      "key": null,
      "body": "The preview shows that actor's pages. Save Sides keeps a snapshot, and Export PDF makes the PDF through the Windows Save dialog."
    }
  ],
  "result": "A preview of the pages for one actor on one day. Saved sides are snapshots, so later script edits do not change them.",
  "mistakes": [
    "Expecting saved sides to update after a script edit. They are snapshots. Make new sides for the new draft.",
    "Forgetting to tick the scenes. Each scene needs its tick box.",
    "Leaving For on All cast when you meant one actor."
  ],
  "troubleshooting": [
    [
      "The preview is empty.",
      "Choose a shooting day, at least one scene and a cast member. The app says so when something is missing."
    ],
    [
      "I need the cover page.",
      "Turn on \"Include call-sheet cover page\" in SETUP."
    ],
    [
      "I want to leave a saved snapshot.",
      "Choose Back to live preview."
    ]
  ],
  "related": [
    "call-sheet",
    "reports",
    "schedule",
    "drafts-revisions"
  ],
  "next": "reports"
};
