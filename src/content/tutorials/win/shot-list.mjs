export default {
  "id": "shot-list",
  "available": true,
  "reason": "",
  "path": "visual",
  "order": 2,
  "feature": "shot-lists-storyboards",
  "title": "Open a shot and set its details",
  "purpose": "Open a shot in the Shot Lists tab and set its size, movement, angle and lens in the shot details.",
  "difficulty": "Beginner",
  "minutes": 4,
  "prerequisites": [
    "A project is open with a screenplay that has scenes.",
    "Shots are listed for at least one scene."
  ],
  "objectives": [
    "Open the Shot Lists tab",
    "Open a shot from its menu",
    "Set shot size and movement in the drawer"
  ],
  "steps": [
    {
      "screen": "520-prod-storyboard-scene-no-board",
      "action": "click",
      "target": "role=button[name=\"Shot Lists\"]",
      "label": "In Production, open the Shot Lists tab",
      "text": null,
      "key": null,
      "body": "In Production, click the Shot Lists tab. It asks \"How will I capture this scene?\" and lists shots for the scene you pick."
    },
    {
      "screen": "521-prod-shots-list",
      "action": "rightclick",
      "target": "button[aria-label=\"Open shot 1A: Wide: the bus arrives at the empty stand\"]",
      "label": "Right-click shot 1A",
      "text": null,
      "key": null,
      "body": "Right-click shot 1A in the list. A menu opens."
    },
    {
      "screen": "526-prod-shot-menu",
      "action": "click",
      "target": "role=menuitem[name=\"Open\"]",
      "label": "Choose Open",
      "text": null,
      "key": null,
      "body": "The menu offers Open, Duplicate, Move to scene, Attach Storyboard, Create storyboard panel and Delete. Choose Open."
    },
    {
      "screen": "527-prod-shot-drawer",
      "action": "view",
      "target": null,
      "label": "Set the shot size, movement, angle and lens in the shot details",
      "text": null,
      "key": null,
      "body": "The drawer for SHOT 1A opens. \"Description *\" is required. Below it are Shot size, Movement, Angle, Lens, Camera notes, Characters and Sound note. Shot size and Movement are typed fields, not fixed lists."
    }
  ],
  "result": "A shot open in its drawer with the technical details filled in. Shot letters such as 1A, 1B and 1C follow the order of the shots, so you never renumber by hand.",
  "mistakes": [
    "Leaving the description empty when adding a shot. The app says \"Describe the shot to add it.\" and adds nothing.",
    "Adding every possible shot. The hint under the list says to add only the shots you need.",
    "Expecting the app to pick lenses for you. Camera details are optional and you type them."
  ],
  "troubleshooting": [
    [
      "The tab shows \"No screenplay scenes yet\".",
      "The shot list works from screenplay scenes. Click Open Screenplay, write or import a scene, then return."
    ],
    [
      "I cannot find the shot I want.",
      "Use the Scene list at the top to choose one scene instead of All scenes. A List and Grid switch changes how shots are drawn."
    ],
    [
      "I want a copy of a shot.",
      "Right-click the shot and choose Duplicate."
    ]
  ],
  "related": [
    "storyboard",
    "moodboard",
    "write-screenplay",
    "export-script"
  ],
  "next": "storyboard"
};
