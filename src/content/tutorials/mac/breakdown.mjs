export default {
  "id": "breakdown",
  "available": true,
  "reason": "",
  "path": "prep",
  "order": 1,
  "feature": "breakdown",
  "title": "Tag a script element in the breakdown",
  "purpose": "Tag a word in a scene as a prop and see it join the production catalog.",
  "difficulty": "Intermediate",
  "minutes": 6,
  "prerequisites": [
    "A project with a screenplay draft.",
    "The first time, you choose the production source for the breakdown."
  ],
  "objectives": [
    "Select words in a scene",
    "Tag them with a category",
    "See the item in the scene breakdown and the catalog"
  ],
  "steps": [
    {
      "screen": "309-breakdown-scenes",
      "action": "doubleclick",
      "target": "[data-id=\"script-text\"]",
      "label": "In Breakdown, scene 1 is open. Select the words to tag in the script (here REGISTER)",
      "text": null,
      "key": null,
      "body": "Open Breakdown in the sidebar. The first time, a sheet called \"Choose the production source\" sets the draft. Scene 1 is open in the Script column. Select the words to tag by double-clicking one, here REGISTER."
    },
    {
      "screen": "322-breakdown-script-selection",
      "action": "click",
      "target": "[data-id=\"tag-selection\"]",
      "label": "Click Tag Selection",
      "text": null,
      "key": null,
      "body": "With the words selected, click Tag Selection in the breakdown column. The Tag Selection as menu above the script offers the same categories."
    },
    {
      "screen": "325-breakdown-tag-sheet",
      "action": "click",
      "target": "[data-id=\"add-to-scene\"]",
      "label": "Check the Category (Props) and the Name, then click Add to Scene 1",
      "text": null,
      "key": null,
      "body": "The tag sheet asks what the scene needs. Check the Category (here Props) and the Name, then click Add to Scene 1. The screenplay text is not changed."
    },
    {
      "screen": "326-breakdown-tagged",
      "action": "view",
      "target": null,
      "label": "Register appears under Props in the Scene 1 breakdown and is added to the production catalog",
      "text": null,
      "key": null,
      "body": "The word now appears under Props in the Scene 1 breakdown. It is also added to the production catalog, so other scenes can reuse it. Mark Complete finishes the scene when you are done."
    }
  ],
  "result": "A tagged prop in Scene 1 and a new item in the production catalog.",
  "mistakes": [
    "Typing a new name when a catalog item already exists. Pick the existing item so scenes share it.",
    "Expecting the script text to change. Tagging never edits the screenplay."
  ],
  "troubleshooting": [
    [
      "Tag Selection is greyed out.",
      "Select some words in the script first."
    ],
    [
      "The breakdown shows Needs Review.",
      "The script changed after tagging. Review the scene and confirm your tags."
    ]
  ],
  "related": [
    "cast-crew",
    "locations",
    "schedule"
  ],
  "next": "cast-crew"
};
