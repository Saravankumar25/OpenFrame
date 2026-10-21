export default {
  "id": "write-screenplay",
  "available": true,
  "reason": "",
  "path": "writing",
  "order": 4,
  "feature": "screenplay",
  "title": "Write in the screenplay editor",
  "purpose": "Open the screenplay, change element types and use character name suggestions.",
  "difficulty": "Beginner",
  "minutes": 5,
  "prerequisites": [
    "A project with a screenplay. An empty project offers \"New Screenplay\", \"Build from Beat Board\" and \"Import Screenplay\"."
  ],
  "objectives": [
    "Open the screenplay",
    "Read the element type bar",
    "Switch elements with Ctrl+1 to Ctrl+7",
    "Accept a character name suggestion"
  ],
  "steps": [
    {
      "screen": "013-shell-project-home",
      "action": "click",
      "target": "role=button[name=\"Continue screenplay\"]",
      "label": "Click Continue screenplay",
      "text": null,
      "key": null,
      "body": "On Project Home, click \"Continue screenplay\". The editor opens with the scene list on the left."
    },
    {
      "screen": "244-sp-editor-standard",
      "action": "click",
      "target": "button[aria-label=\"Element type: Scene Heading. Change element type\"]",
      "label": "The bar at the bottom shows the element type of the line you are in. Click it",
      "text": null,
      "key": null,
      "body": "The bar at the bottom shows the element type of the line you are in, for example \"Scene Heading · Enter → Action\". Click it to change the type."
    },
    {
      "screen": "257-sp-element-type-menu",
      "action": "key",
      "target": null,
      "label": "Pick a type or use Ctrl+1 to Ctrl+7. Press Esc to close",
      "text": null,
      "key": "Esc",
      "body": "The ELEMENT TYPE menu lists Scene Heading, Action, Character, Parenthetical, Dialogue, Transition and Shot. Ctrl+1 to Ctrl+7 do the same, and Ctrl+Enter inserts a page break. Press Esc to close the menu."
    },
    {
      "screen": "244-sp-editor-standard",
      "action": "type",
      "target": "[data-id=\"01a15b74-802c-7786-9f65-18081a818e07\"]",
      "label": "On a character line, start typing a name",
      "text": "LA",
      "key": null,
      "body": "On a Character line, start typing a name. Enter moves to the usual next element, so after Character it moves to Dialogue."
    },
    {
      "screen": "258-sp-suggestion-list",
      "action": "view",
      "target": null,
      "label": "OpenFrame suggests characters already in the script. Enter accepts",
      "text": null,
      "key": null,
      "body": "The suggestion list offers names already in the script, plus \"Create New\" for a new one. Enter accepts and Esc keeps your text."
    }
  ],
  "result": "You can move around the screenplay, set each line's element and fill character names from the script.",
  "mistakes": [
    "Fighting the format. Press Enter and OpenFrame moves to the usual next element for you.",
    "Typing a character name that is a little different. Use the suggestion list so the name stays the same everywhere."
  ],
  "troubleshooting": [
    [
      "I do not see the suggestion list",
      "Suggestions show on a Character line while you type. Keep typing the first letters of the name."
    ],
    [
      "I want to add a note that does not print",
      "Use \"General note (not printed)\" in the element type menu."
    ]
  ],
  "related": [
    "screenplay-notes",
    "characters",
    "drafts-revisions",
    "import-script"
  ],
  "next": "screenplay-notes"
};
