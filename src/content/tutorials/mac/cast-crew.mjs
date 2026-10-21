export default {
  "id": "cast-crew",
  "available": true,
  "reason": "",
  "path": "prep",
  "order": 2,
  "feature": "cast-crew-locations",
  "title": "Cast actors and add crew",
  "purpose": "Link a real person to a character, then add a crew member with a role.",
  "difficulty": "Beginner",
  "minutes": 7,
  "prerequisites": [
    "A project with characters in the screenplay."
  ],
  "objectives": [
    "Assign a person to a character",
    "Add a crew member with a role and department",
    "Read a person in the inspector"
  ],
  "steps": [
    {
      "screen": "500-plan-cast-needs-casting",
      "action": "click",
      "target": "a[aria-label=\"Assign KIRAN\"]",
      "label": "In Production, Cast & Crew, Cast, click Assign next to KIRAN",
      "text": null,
      "key": null,
      "body": "Open Production, then Cast & Crew, and stay on the Cast switch. Characters from the script appear under Needs casting, each with an Assign button. Click Assign next to a character."
    },
    {
      "screen": "501-plan-cast-assign-sheet",
      "action": "type",
      "target": "[data-id=\"as-person\"]",
      "label": "Type the actor name",
      "text": "Meera Iyer",
      "key": null,
      "body": "The Add Person sheet says \"Link the real person to the character they play.\" Type the actor name in the Person field."
    },
    {
      "screen": "502-plan-cast-assign-filled",
      "action": "click",
      "target": "[data-id=\"assign-submit\"]",
      "label": "Keep Primary actor and click Assign",
      "text": null,
      "key": null,
      "body": "Keep Casting on Primary actor, or choose Alternate / double, then click Assign. A message confirms who plays the character and offers Undo."
    },
    {
      "screen": "503-plan-cast-linked-one",
      "action": "view",
      "target": null,
      "label": "Meera Iyer now plays KIRAN. DEVI still needs casting",
      "text": null,
      "key": null,
      "body": "The person now plays that character. Characters still without an actor stay under Needs casting. Select a cast row to open CAST MEMBER in the inspector, where Contact stays private to Owners and Editors."
    },
    {
      "screen": "510-plan-crew-add-empty",
      "action": "type",
      "target": "[data-id=\"nc-name\"]",
      "label": "In Production, Cast & Crew, switch to Crew and click Add Person. Type the name and the role",
      "text": "Imran Sheikh",
      "key": null,
      "body": "Now the crew. Switch to Crew and click Add Person. The sheet says \"No payroll, no contracts.\" Type the person name and the role."
    },
    {
      "screen": "511-plan-crew-add-filled",
      "action": "click",
      "target": "[data-id=\"nc-submit\"]",
      "label": "Pick a Department and add contact details if you like, then click Add to Crew",
      "text": null,
      "key": null,
      "body": "Pick a Department and add contact details if you like, then click Add to Crew."
    },
    {
      "screen": "512-plan-crew-open",
      "action": "view",
      "target": null,
      "label": "The crew member opens in the inspector with Role, Department, Contact and availability",
      "text": null,
      "key": null,
      "body": "The crew member opens in the inspector with Role, Department, Contact and availability. Add Dates records when the person can work."
    }
  ],
  "result": "A cast member linked to a character and a crew member with a role and department.",
  "mistakes": [
    "Typing a character name that is not in the script. Use \"Create a new character\" only for a character the script does not have yet.",
    "Putting private phone numbers in Notes. Use the Contact field, which is limited to Owners and Editors."
  ],
  "troubleshooting": [
    [
      "No characters appear under Needs casting.",
      "Characters come from the script. Add or tag them first."
    ],
    [
      "The Crew list is empty.",
      "That is normal for a new project. Click Add Person to add the first one."
    ]
  ],
  "related": [
    "locations",
    "schedule",
    "characters"
  ],
  "next": "locations"
};
