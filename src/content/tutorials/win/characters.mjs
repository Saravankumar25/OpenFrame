export default {
  "id": "characters",
  "available": true,
  "reason": "",
  "path": "writing",
  "order": 3,
  "feature": "story-board",
  "title": "Add and edit characters",
  "purpose": "Open the Characters list, add a character and edit a character's details.",
  "difficulty": "Beginner",
  "minutes": 4,
  "prerequisites": [
    "A project is open."
  ],
  "objectives": [
    "Open the Characters tab",
    "Start a new character",
    "Handle the duplicate warning",
    "Edit name, role and description"
  ],
  "steps": [
    {
      "screen": "169-story-page-filled",
      "action": "click",
      "target": "role=tab[name=\"Characters\"]",
      "label": "In Story, open the Characters tab",
      "text": null,
      "key": null,
      "body": "In Story, open the Characters tab. Each row shows initials, a name, a role (Lead, Supporting or Minor) and a scene count."
    },
    {
      "screen": "216-story-characters-list",
      "action": "click",
      "target": "role=button[name=\"Add Character\"]",
      "label": "Click Add Character",
      "text": null,
      "key": null,
      "body": "Click Add Character. The New Character dialog opens."
    },
    {
      "screen": "219-story-new-character-dialog",
      "action": "type",
      "target": "#nc-name",
      "label": "Type a name",
      "text": "Lakshmi",
      "key": null,
      "body": "Type the name in the \"Name *\" box. Other fields are \"Role / title\", Image and \"Short description\"."
    },
    {
      "screen": "220-story-new-character-duplicate-warning",
      "action": "click",
      "target": "role=dialog >> role=button[name=\"Cancel\"]",
      "label": "OpenFrame warns about a possible match. Cancel and use the existing character",
      "text": null,
      "key": null,
      "body": "If the name matches one you have, OpenFrame warns \"Possible match\" and offers \"Use Existing\" or \"Create Another Character Anyway\". Here, Cancel and use the existing character."
    },
    {
      "screen": "216-story-characters-list",
      "action": "click",
      "target": "role=button[name=\"Edit\"s]",
      "label": "Click Edit to fill in the details",
      "text": null,
      "key": null,
      "body": "Select a character in the list and click Edit to fill in the details."
    },
    {
      "screen": "221-story-edit-character-dialog",
      "action": "view",
      "target": null,
      "label": "Name, role and short description, saved with Save",
      "text": null,
      "key": null,
      "body": "The Edit dialog has Name, \"Role / title\" and \"Short description\". Click Save when you are done. Renaming does not change screenplay text; use Find & Replace in the screenplay for that."
    }
  ],
  "result": "A character list you can add to and edit, with a warning that stops accidental duplicates.",
  "mistakes": [
    "Creating a second copy of a character. If you see \"Possible match\", choose \"Use Existing\".",
    "Expecting a rename to change the script. Use Find & Replace in the screenplay for that."
  ],
  "troubleshooting": [
    [
      "The Create Character button does nothing",
      "The \"Name *\" field is required. Type a name first."
    ],
    [
      "I want to link a character to a scene",
      "Select the character, then use \"Scene Card to link\" and click Link in the detail panel."
    ]
  ],
  "related": [
    "story-board",
    "write-screenplay",
    "cast-crew",
    "breakdown"
  ],
  "next": "write-screenplay"
};
