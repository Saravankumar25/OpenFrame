export default {
  "id": "pages-templates",
  "available": true,
  "reason": "",
  "path": "writing",
  "order": 9,
  "feature": null,
  "title": "Preview a built-in template",
  "purpose": "Find the built-in templates in Project Settings and preview one before you use it.",
  "difficulty": "Beginner",
  "minutes": 3,
  "prerequisites": [
    "A project is open. Project Settings opens from Settings in the toolbar, or with Shift+Cmd+,."
  ],
  "objectives": [
    "Open the Templates page",
    "Open the Actions menu of a template",
    "Preview a template"
  ],
  "steps": [
    {
      "screen": "619-pages-settings-general",
      "action": "click",
      "target": "[data-id=\"project-settings-tab-templates\"]",
      "label": "In Project Settings, click Templates",
      "text": null,
      "key": null,
      "body": "In Project Settings, click Templates. The pages are Project settings, Writing, Keyboard shortcuts, Templates, Offline AI, Advanced and Appearance."
    },
    {
      "screen": "632-pages-templates",
      "action": "click",
      "target": "[data-id=\"template-actions-builtin-builtin:feature_screenplay\"]",
      "label": "Open Actions for Feature Film Screenplay",
      "text": null,
      "key": null,
      "body": "Templates asks \"Would a saved starting point help me begin faster?\" Built-in starters include Feature Film Screenplay, Short Film Screenplay, Simple Beat Board, Indie Call Sheet, Shot List, Moodboard and Lightweight Budget. Open Actions on Feature Film Screenplay."
    },
    {
      "screen": "634-pages-templates-actions-menu",
      "action": "click",
      "target": "[data-id=\"menu-item-0\"]",
      "label": "Choose Preview",
      "text": null,
      "key": null,
      "body": "The Actions menu has Preview, Start the screenplay, Use in This Project and Copy to My Templates. Choose Preview."
    },
    {
      "screen": "633-pages-templates-preview",
      "action": "view",
      "target": null,
      "label": "Preview the template, then use it in this project",
      "text": null,
      "key": null,
      "body": "The preview lists the scenes in the template. Close it, or choose Start the screenplay or Use in This Project when you are ready."
    }
  ],
  "result": "You have seen what a built-in template contains before using it.",
  "mistakes": [
    "Applying a template without previewing. Preview first, then use it.",
    "Looking for templates elsewhere. On the Mac they live in Project Settings.",
    "Expecting to edit a built-in starter. Choose Copy to My Templates for a version of your own."
  ],
  "troubleshooting": [
    [
      "I cannot open Project Settings.",
      "Click Settings in the toolbar, or press Shift+Cmd+,."
    ],
    [
      "I want my own template.",
      "Click Create Template on the Templates page."
    ],
    [
      "The sheet will not close.",
      "Click Done at the bottom of Project Settings."
    ]
  ],
  "related": [
    "write-screenplay",
    "settings",
    "shot-list"
  ],
  "next": "moodboard"
};
