export default {
  "id": "shot-list",
  "available": true,
  "reason": "",
  "path": "visual",
  "order": 2,
  "feature": "shot-lists-storyboards",
  "title": "Plan shots in a shot list",
  "purpose": "Add shots to a scene and open one to see its details.",
  "difficulty": "Beginner",
  "minutes": 6,
  "prerequisites": [
    "A project is open with a screenplay draft."
  ],
  "objectives": [
    "Choose a scene",
    "Add shots with a short description",
    "Open a shot in the inspector"
  ],
  "steps": [
    {
      "screen": "545-plan-shots-all-empty",
      "action": "click",
      "target": "[data-id=\"shots-scene-picker\"]",
      "label": "In Production, Shot Lists, open the Scene pop-up",
      "text": null,
      "key": null,
      "body": "Open Production, then Shot Lists, from the section pop-up. The page asks \"How will I capture this scene?\". Open the Scene pop-up, which starts on All scenes."
    },
    {
      "screen": "546-plan-shots-scene-menu",
      "action": "click",
      "target": "[data-id=\"menu-item-3\"]",
      "label": "Choose Scene 1",
      "text": null,
      "key": null,
      "body": "The menu lists your scenes in order. Choose Scene 1."
    },
    {
      "screen": "547-plan-shots-scene-empty",
      "action": "type",
      "target": "[data-id=\"shots-composer\"]",
      "label": "Type a short shot description, then click Add Shot. Repeat for each shot",
      "text": "Wide: the bus pulls into the empty stop",
      "key": null,
      "body": "Type a short description in the composer, then click Add Shot. An empty description shows \"Describe the shot to add it.\" Repeat for each shot. Starting the text with the shot size, such as \"Wide:\", keeps the list easy to scan."
    },
    {
      "screen": "549-plan-shots-list",
      "action": "rightclick",
      "target": "[data-id=\"shot-01a15bae-3342-71ba-8202-b920a6d6617d\"]",
      "label": "Shots get letters 1A, 1B, 1C. Control-click (right-click) a shot",
      "text": null,
      "key": null,
      "body": "Shots get letters: 1A, 1B, 1C. Control-click a shot to open its menu."
    },
    {
      "screen": "553-plan-shot-row-menu",
      "action": "click",
      "target": "[data-id=\"menu-item-0\"]",
      "label": "Choose Open",
      "text": null,
      "key": null,
      "body": "The menu offers Open, Duplicate, Move to Scene, Move Up, Move Down, Attach Storyboard, Create Storyboard Panel and Delete. Choose Open."
    },
    {
      "screen": "556-plan-shot-open",
      "action": "view",
      "target": null,
      "label": "The shot opens in the inspector with its description, optional technical details, a reference image and storyboard links",
      "text": null,
      "key": null,
      "body": "The inspector shows the shot description. Technical details are optional and stay hidden until you need them. A reference image and storyboard links sit below. Use the List and Grid switch to change how the list looks, and Export for a file."
    }
  ],
  "result": "A scene with lettered shots, each one openable in the inspector.",
  "mistakes": [
    "Adding every camera detail up front. Add only the shots you need; camera details are optional.",
    "Picking All scenes and then looking for the composer. Choose one scene first."
  ],
  "troubleshooting": [
    [
      "Add Shot does nothing.",
      "The description is empty. Type a short description first."
    ],
    [
      "I put a shot in the wrong scene.",
      "Control-click the shot and choose Move to Scene."
    ]
  ],
  "related": [
    "storyboard",
    "schedule",
    "write-screenplay"
  ],
  "next": "storyboard"
};
