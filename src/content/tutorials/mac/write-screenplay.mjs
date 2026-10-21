export default {
  "id": "write-screenplay",
  "available": true,
  "reason": "",
  "path": "writing",
  "order": 4,
  "feature": "screenplay",
  "title": "Write your first scene",
  "purpose": "Type a scene heading and action in the screenplay editor and see it in the scene list.",
  "difficulty": "Beginner",
  "minutes": 5,
  "prerequisites": [
    "A project is open and you are in Screenplay in the sidebar with an empty screenplay."
  ],
  "objectives": [
    "Find the element types",
    "Start a scene heading with suggestions",
    "See the scene appear in the scene list"
  ],
  "steps": [
    {
      "screen": "105-screenplay-empty-editor",
      "action": "click",
      "target": "[data-id=\"element-picker\"]",
      "label": "Click Element to see the element types",
      "text": null,
      "key": null,
      "body": "The bottom bar of the screenplay page has an Element menu. Click Element to see the element types."
    },
    {
      "screen": "106-screenplay-empty-element-picker",
      "action": "key",
      "target": null,
      "label": "Press Esc to close the menu",
      "text": null,
      "key": "Esc",
      "body": "The menu lists Scene Heading, Action, Character, Parenthetical, Dialogue, Transition, Shot, General note (not printed) and Insert Page Break. Press Esc to close it."
    },
    {
      "screen": "105-screenplay-empty-editor",
      "action": "type",
      "target": "[data-id=\"screenplay-editor\"]",
      "label": "Start a scene heading",
      "text": "IN",
      "key": null,
      "body": "Click in the page and start a scene heading. The editor suggests INT., EXT., INT./EXT. and I/E. as you type."
    },
    {
      "screen": "114-screenplay-empty-suggestions",
      "action": "type",
      "target": "[data-id=\"screenplay-editor\"]",
      "label": "Finish the heading, press Return and write the action",
      "text": "INT. BUS DEPOT - NIGHT\nMeera waits under the only working lamp.",
      "key": null,
      "body": "Finish the heading, press Return and write the action line. Return at the end of a line starts the element that normally follows."
    },
    {
      "screen": "115-screenplay-empty-first-scene",
      "action": "view",
      "target": null,
      "label": "Your first scene is on the page and in the scene list",
      "text": null,
      "key": null,
      "body": "Your first scene is on the page and in the scene list, headed \"Scenes (1)\"."
    }
  ],
  "result": "Your screenplay has its first scene, shown on the page and in the scene list.",
  "mistakes": [
    "Formatting by hand. Pick the element or use Tab and Return; the editor handles the layout.",
    "Forgetting the shortcuts. Cmd+1 is Scene Heading, Cmd+2 Action, Cmd+3 Character, Cmd+4 Parenthetical, Cmd+5 Dialogue, Cmd+6 Transition and Cmd+7 Shot.",
    "Using the wrong element for a line. Press Tab to move to the next element type and Shift+Tab to go back."
  ],
  "troubleshooting": [
    [
      "The scene list is empty.",
      "The line must be a Scene Heading element. Choose it from the Element menu or press Cmd+1."
    ],
    [
      "Return gave me the wrong element.",
      "Press Tab to cycle to the one you want, or choose it from the Element menu."
    ],
    [
      "I want a page break.",
      "Choose Insert Page Break in the Element menu, or press Cmd+Return."
    ]
  ],
  "related": [
    "characters",
    "screenplay-notes",
    "drafts-revisions"
  ],
  "next": "screenplay-notes"
};
