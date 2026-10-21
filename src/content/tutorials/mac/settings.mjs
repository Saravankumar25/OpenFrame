export default {
  "id": "settings",
  "available": true,
  "reason": "",
  "path": "data",
  "order": 5,
  "feature": null,
  "title": "Tour the Settings window",
  "purpose": "Visit each Settings page and learn what lives there.",
  "difficulty": "Beginner",
  "minutes": 5,
  "prerequisites": [
    "OpenFrame Studio is open."
  ],
  "objectives": [
    "Open the Settings window",
    "Visit each page",
    "Tell app Settings from Project Settings"
  ],
  "steps": [
    {
      "screen": "003-home-empty",
      "action": "click",
      "target": "[data-id=\"home-settings\"]",
      "label": "Click Settings (or choose OpenFrame Studio > Settings…, Cmd+,)",
      "text": null,
      "key": null,
      "body": "On Application Home, click Settings. You can also choose OpenFrame Studio > Settings in the menu bar, or press Cmd+,."
    },
    {
      "screen": "020-settings-general",
      "action": "click",
      "target": "a[aria-label=\"Appearance\"]",
      "label": "General holds your display name and the data folder. Click Appearance",
      "text": null,
      "key": null,
      "body": "General holds your display name, which appears on your notes, comments and history, and the data folder, with Show in Finder. Click Appearance."
    },
    {
      "screen": "022-settings-appearance",
      "action": "click",
      "target": "a[aria-label=\"Keyboard Shortcuts\"]",
      "label": "Appearance holds Mode and Theme. Click Keyboard Shortcuts",
      "text": null,
      "key": null,
      "body": "Appearance holds Mode and Theme. Click Keyboard Shortcuts."
    },
    {
      "screen": "027-settings-keyboard-shortcuts",
      "action": "click",
      "target": "a[aria-label=\"Offline AI\"]",
      "label": "Change, remove or add keys for 23 commands. Click Offline AI",
      "text": null,
      "key": null,
      "body": "Keyboard Shortcuts lists 23 commands. Use Filter commands to search, change, remove or add keys on a row, or Restore all defaults. Click Offline AI."
    },
    {
      "screen": "032-settings-offline-ai",
      "action": "click",
      "target": "a[aria-label=\"Diagnostics\"]",
      "label": "Offline AI manages the on-device model. Click Diagnostics",
      "text": null,
      "key": null,
      "body": "Offline AI manages the optional on-device model. Nothing is needed here if you do not use the assistant. Click Diagnostics."
    },
    {
      "screen": "033-settings-diagnostics",
      "action": "view",
      "target": null,
      "label": "Diagnostics helps when something needs support",
      "text": null,
      "key": null,
      "body": "Diagnostics helps when something needs support. Settings for one project are separate: open Project Settings from the toolbar Settings button in a project, or press Cmd+Shift+,."
    }
  ],
  "result": "You know where each app setting lives and how it differs from Project Settings.",
  "mistakes": [
    "Changing a shortcut and forgetting the default. Use Restore all defaults to undo.",
    "Looking for project writing options here. They are in Project Settings."
  ],
  "troubleshooting": [
    [
      "A shortcut does not work after I changed it.",
      "Open Keyboard Shortcuts and check the row for a conflict, or restore defaults."
    ],
    [
      "I cannot see Project Settings.",
      "Open a project first. The toolbar Settings button appears inside a project."
    ]
  ],
  "related": [
    "themes-appearance",
    "backup-restore",
    "manage-projects"
  ],
  "next": "themes-appearance"
};
