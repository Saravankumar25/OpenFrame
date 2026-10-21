export default {
  "id": "storyboard",
  "available": true,
  "reason": "",
  "path": "visual",
  "order": 3,
  "feature": "shot-lists-storyboards",
  "title": "Add storyboard panels to a scene",
  "purpose": "Add empty panels to a scene and learn what the panel menu can do.",
  "difficulty": "Beginner",
  "minutes": 5,
  "prerequisites": [
    "A project is open with at least one scene."
  ],
  "objectives": [
    "Add panels to a scene",
    "Open the panel menu",
    "Know the ways to fill a panel"
  ],
  "steps": [
    {
      "screen": "534-plan-storyboard-scene-empty",
      "action": "click",
      "target": "[data-id=\"storyboard-add-panel\"]",
      "label": "In Production, Storyboards, with a scene chosen, click Add Panel",
      "text": null,
      "key": null,
      "body": "Open Production, then Storyboards, and choose a scene in the \"Scene or storyboard\" pop-up. The page asks \"What will the audience see?\". Click Add Panel."
    },
    {
      "screen": "536-plan-storyboard-add-menu",
      "action": "click",
      "target": "[data-id=\"menu-item-2\"]",
      "label": "Choose Empty Panel. Repeat to add more panels",
      "text": null,
      "key": null,
      "body": "The Add Panel menu holds Import Image, Draw / Sketch and Empty Panel. Choose Empty Panel. Repeat to add more panels. Draw / Sketch opens a basic sketch sheet for quick drawings, and Import Image is better for polished art."
    },
    {
      "screen": "538-plan-storyboard-panels",
      "action": "rightclick",
      "target": "[data-id=\"panel-01a15bac-3c03-722b-8d36-ba12f0b56f23\"]",
      "label": "Control-click (right-click) a panel",
      "text": null,
      "key": null,
      "body": "Panels read \"Panel 1, empty\" and so on. Drag panels to change the order. Control-click a panel."
    },
    {
      "screen": "539-plan-storyboard-panel-menu",
      "action": "view",
      "target": null,
      "label": "The panel menu offers Open, Move Earlier, Move Later, Move to Storyboard…, Link Shot… and Delete",
      "text": null,
      "key": null,
      "body": "The panel menu offers Open, Move Earlier, Move Later, Move to Storyboard, Link Shot and Delete. Link Shot ties a panel to a shot from the shot list. Use Export to make a PDF panel sheet."
    }
  ],
  "result": "A scene with storyboard panels you can reorder and link to shots.",
  "mistakes": [
    "Expecting polished drawing from the sketch tool. It is for basic sketching; import an image made elsewhere for finished art.",
    "Forgetting to choose a scene first. Panels belong to a scene or storyboard."
  ],
  "troubleshooting": [
    [
      "The page only shows \"Create panels for this scene.\"",
      "That is the empty state. Click Import Image, Draw / Sketch or Empty Panel."
    ],
    [
      "A panel is in the wrong place.",
      "Drag it, or Control-click it and choose Move Earlier or Move Later."
    ]
  ],
  "related": [
    "shot-list",
    "moodboard",
    "schedule"
  ],
  "next": "breakdown"
};
