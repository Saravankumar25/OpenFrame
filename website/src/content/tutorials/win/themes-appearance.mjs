export default {
  "id": "themes-appearance",
  "available": true,
  "reason": "",
  "path": "data",
  "order": 4,
  "feature": null,
  "title": "Choose Light, Dark and a theme",
  "purpose": "Set the mode and theme for OpenFrame on this computer.",
  "difficulty": "Beginner",
  "minutes": 2,
  "prerequisites": [
    "OpenFrame Studio is open."
  ],
  "objectives": [
    "Open Appearance",
    "Choose Light, Dark or System",
    "Pick one of the 17 themes"
  ],
  "steps": [
    {
      "screen": "013-shell-project-home",
      "action": "click",
      "target": "button[aria-label=\"Project: Night Bus. Switch or manage projects\"]",
      "label": "Open the project menu",
      "text": null,
      "key": null,
      "body": "Click the project name at the top left to open the project menu."
    },
    {
      "screen": "014-shell-project-menu",
      "action": "click",
      "target": "role=menuitem[name=\"Appearance\"]",
      "label": "Choose Appearance",
      "text": null,
      "key": null,
      "body": "Choose Appearance from the menu."
    },
    {
      "screen": "797-settings-appearance",
      "action": "view",
      "target": null,
      "label": "Under Mode choose Light, Dark or System, then pick a theme",
      "text": null,
      "key": null,
      "body": "Under MODE choose Light, Dark or System. Under THEME pick a card such as OpenFrame Default, Studio Amber or High Contrast. There are 17 themes and Light and Dark work with every one."
    }
  ],
  "result": "The app uses the mode and theme you picked. Your projects are not changed.",
  "mistakes": [
    "Expecting themes to change printed pages. Screenplay pages, PDFs and printed call sheets stay black on white.",
    "Importing a theme file that is hard to read. Paste Theme checks readability in Light and Dark before Save to My Themes.",
    "Looking for Appearance only inside a project. Without a project, use the Settings button on the projects screen."
  ],
  "troubleshooting": [
    [
      "The theme did not change the exports.",
      "That is by design. Themes change colours on screen only."
    ],
    [
      "I want my own theme.",
      "Under My Themes use Import Theme File or Paste Theme."
    ],
    [
      "Text is hard to read.",
      "Try the High Contrast theme."
    ]
  ],
  "related": [
    "settings",
    "first-launch",
    "project-tour",
    "pages-templates"
  ],
  "next": "settings"
};
