export default {
  "id": "cast-crew",
  "available": true,
  "reason": "",
  "path": "prep",
  "order": 2,
  "feature": "cast-crew-locations",
  "title": "Assign an actor to a character",
  "purpose": "Link a real person to a character who still needs casting.",
  "difficulty": "Beginner",
  "minutes": 3,
  "prerequisites": [
    "A project is open with characters in the script.",
    "You are in Production > Cast & Crew."
  ],
  "objectives": [
    "Find a character that needs casting",
    "Type the actor name",
    "Choose Primary actor or Alternate"
  ],
  "steps": [
    {
      "screen": "469-prod-cast",
      "action": "click",
      "target": "tr:has-text(\"NEWSREADER\") >> role=button[name=\"Assign\"]",
      "label": "Click Assign next to a character that needs casting",
      "text": null,
      "key": null,
      "body": "In Production, open Cast & Crew. The Cast list has PERSON, CHARACTER, CONTACT, AVAILABILITY, NOTE and SCENES. A character with no actor reads \"Needs casting\". Click Assign in that row."
    },
    {
      "screen": "470-prod-cast-assign-dialog",
      "action": "type",
      "target": "#as-person",
      "label": "Type the actor's name and choose the character they play",
      "text": "Lata Nair",
      "key": null,
      "body": "The Add Person dialog says \"Link the real person to the character they play.\" Type the actor name in Person *, and pick the character from \"Choose a character\"."
    },
    {
      "screen": "471-prod-cast-assign-alternate-banner",
      "action": "view",
      "target": null,
      "label": "Pick Primary actor or Alternate / double, then click Assign",
      "text": null,
      "key": null,
      "body": "Under Casting, choose Primary actor or Alternate / double. If the character already has a primary actor, a banner tells you the new person becomes an alternate. Click Assign."
    }
  ],
  "result": "The actor is linked to the character, and the scene count shows in the Cast list. There is no payroll or contract in this page.",
  "mistakes": [
    "Choosing Primary actor when you meant a double. Pick Alternate / double for a second person.",
    "Typing contact details and expecting them in exports. Contact is private to Owners and Editors and is never put in exports.",
    "Adding a character that is already in the script. Use \"Choose a character\" first, and \"Create a new character\" only when it is not in the script yet."
  ],
  "troubleshooting": [
    [
      "No character is listed to choose.",
      "Add the character in the screenplay or use \"Create a new character\" in the dialog."
    ],
    [
      "The actor is on the wrong character.",
      "Right-click the cast row and choose Assign Character to change it."
    ],
    [
      "I cannot see an archived person.",
      "Turn on Show archived above the list."
    ]
  ],
  "related": [
    "breakdown",
    "locations",
    "schedule",
    "characters"
  ],
  "next": "locations"
};
