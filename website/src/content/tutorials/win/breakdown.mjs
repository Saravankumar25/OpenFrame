export default {
  "id": "breakdown",
  "available": true,
  "reason": "",
  "path": "prep",
  "order": 1,
  "feature": "breakdown",
  "title": "Tag a prop in a scene breakdown",
  "purpose": "Select words in a scene, tag them as a prop and link the tag to the catalog.",
  "difficulty": "Intermediate",
  "minutes": 5,
  "prerequisites": [
    "A project is open with a production script.",
    "The Breakdown page shows the script as its Production Source."
  ],
  "objectives": [
    "Pick a scene to break down",
    "Tag script words with a category",
    "Reuse an existing catalog item"
  ],
  "steps": [
    {
      "screen": "355-bd-scene-quick-reuse",
      "action": "click",
      "target": "role=option[name*=\"CORRIDOR\"]",
      "label": "In the scene list, click Scene 5",
      "text": null,
      "key": null,
      "body": "Open Breakdown in the left rail. In the scene list on the left, click Scene 5. The panel on the right reads \"Scene 5 breakdown\"."
    },
    {
      "screen": "356-bd-scene-nothing-yet",
      "action": "drag",
      "target": "text=A long corridor with green walls",
      "label": "Select the words to tag in the script",
      "text": null,
      "key": null,
      "body": "A new scene says \"Nothing broken down yet.\" Drag across the words in the script that you want to tag, here \"A long corridor with green walls\"."
    },
    {
      "screen": "357-bd-tag-menu",
      "action": "click",
      "target": "role=menuitem[name=\"Props\"]",
      "label": "Choose a category, here Props",
      "text": null,
      "key": null,
      "body": "A menu titled TAG with your selection opens. It lists Cast, Extras / Background, Location / Set, Props, Wardrobe, Vehicles, Hair / Makeup, Special Effects, VFX, Sound and Animals. Choose Props."
    },
    {
      "screen": "358-bd-tag-element-dialog",
      "action": "type",
      "target": "#add-name",
      "label": "Type the item name; an existing catalog item is offered",
      "text": "Soldering iron",
      "key": null,
      "body": "The dialog asks \"What does Scene 5 need?\" Type the item name in Name *. The app offers \"Use existing\" when the catalog already has a match. Category * and Notes (optional) are in the same dialog."
    },
    {
      "screen": "359-bd-add-element-existing-match",
      "action": "click",
      "target": "role=button[name=\"Add to Scene 5\"]",
      "label": "Click Add to Scene 5",
      "text": null,
      "key": null,
      "body": "Click Add to Scene 5. The screenplay text is not changed. The tag only marks the selected words in this scene."
    },
    {
      "screen": "360-bd-element-tagged",
      "action": "view",
      "target": null,
      "label": "The prop is tagged in Scene 5 and linked to the catalog",
      "text": null,
      "key": null,
      "body": "The prop now shows in the Scene 5 breakdown and in Production > Catalog. Use Mark Complete when the scene is done."
    }
  ],
  "result": "A prop tagged in the scene and linked to a catalog item, so every scene that needs it is visible from the Catalog.",
  "mistakes": [
    "Expecting the script to change when you tag. Tags never edit the screenplay.",
    "Creating a second catalog item for the same prop. Pick the Use existing offer instead.",
    "Forgetting Mark Complete. Scenes stay open in the list until you mark them."
  ],
  "troubleshooting": [
    [
      "The tag menu does not appear.",
      "Select the words again by dragging across them in the script, then wait for the menu."
    ],
    [
      "I changed the script and tags look stale.",
      "The scene shows Needs Review after script changes. Open it and confirm the tags."
    ],
    [
      "I tagged the wrong category.",
      "Use the Actions menu on the tagged item to edit or remove it."
    ]
  ],
  "related": [
    "cast-crew",
    "locations",
    "schedule",
    "import-script"
  ],
  "next": "cast-crew"
};
