export default {
  "id": "themes-appearance",
  "available": true,
  "reason": "",
  "path": "data",
  "order": 4,
  "feature": null,
  "title": "Change the theme",
  "purpose": "Open Settings, choose a theme and see it apply across the app.",
  "difficulty": "Beginner",
  "minutes": 3,
  "prerequisites": [
    "OpenFrame Studio is open."
  ],
  "objectives": [
    "Open the Settings window",
    "Open the Appearance page",
    "Choose a theme"
  ],
  "steps": [
    {
      "screen": "003-home-empty",
      "action": "click",
      "target": "[data-id=\"home-settings\"]",
      "label": "Click Settings (or choose OpenFrame Studio > Settings…, Cmd+,)",
      "text": null,
      "key": null,
      "body": "On Application Home, click Settings. You can also choose OpenFrame Studio > Settings in the menu bar, or press Cmd+, to open the Settings window."
    },
    {
      "screen": "020-settings-general",
      "action": "click",
      "target": "a[aria-label=\"Appearance\"]",
      "label": "In the Settings window, click Appearance",
      "text": null,
      "key": null,
      "body": "The Settings window has General, Appearance, Keyboard Shortcuts, Offline AI and Diagnostics. Click Appearance."
    },
    {
      "screen": "022-settings-appearance",
      "action": "click",
      "target": "[data-id=\"theme-card-black-and-white\"]",
      "label": "Mode offers System, Light and Dark. Click a theme, here Black and White",
      "text": null,
      "key": null,
      "body": "Mode offers System, Light and Dark. System follows your Mac. Below it, theme cards fill the page. Click a theme, here Black and White. Scroll down for My Themes."
    },
    {
      "screen": "024-settings-appearance-theme-chosen",
      "action": "view",
      "target": null,
      "label": "The theme applies at once across the app",
      "text": null,
      "key": null,
      "body": "The theme applies at once across the app. Choose another card any time to change it back."
    }
  ],
  "result": "The app uses the theme you chose.",
  "mistakes": [
    "Looking for themes inside a project only. Appearance is in the Settings window, and Project Settings has its own Appearance page.",
    "Expecting System mode to hold one look. It follows your Mac."
  ],
  "troubleshooting": [
    [
      "I cannot find Settings.",
      "Click the Settings button on Application Home, or use the OpenFrame Studio menu in the menu bar."
    ],
    [
      "The colors look wrong.",
      "Check Mode. It may be set to Light or Dark rather than System."
    ]
  ],
  "related": [
    "settings",
    "first-launch",
    "manage-projects"
  ],
  "next": "settings"
};
