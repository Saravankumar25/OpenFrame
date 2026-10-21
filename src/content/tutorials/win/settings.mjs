export default {
  "id": "settings",
  "available": true,
  "reason": "",
  "path": "data",
  "order": 5,
  "feature": null,
  "title": "Tour the project settings",
  "purpose": "Visit each settings tab so you know where title, writing, shortcuts, offline AI and appearance live.",
  "difficulty": "Beginner",
  "minutes": 4,
  "prerequisites": [
    "A project is open."
  ],
  "objectives": [
    "Open project settings",
    "Find spelling, shortcuts and Offline AI",
    "Reach Advanced and Appearance"
  ],
  "steps": [
    {
      "screen": "013-shell-project-home",
      "action": "click",
      "target": "button[aria-label=\"Settings\"]",
      "label": "Click Settings (the gear at the top right)",
      "text": null,
      "key": null,
      "body": "Click Settings, the gear at the top right. The tabs are Project settings, Writing, Keyboard shortcuts, Templates, Offline AI, Advanced and Appearance."
    },
    {
      "screen": "762-settings-general",
      "action": "click",
      "target": "role=tab[name=\"Writing\"]",
      "label": "Project settings hold the title, type and status; open Writing",
      "text": null,
      "key": null,
      "body": "Project settings hold the Title, Type, Language, Genre, Status and Logline. Open the Writing tab."
    },
    {
      "screen": "768-settings-writing",
      "action": "click",
      "target": "role=tab[name=\"Keyboard shortcuts\"]",
      "label": "Spelling and your dictionary are here; open Keyboard shortcuts",
      "text": null,
      "key": null,
      "body": "Writing has the spelling check, proofing languages and a Personal dictionary with New word and Add. Open Keyboard shortcuts."
    },
    {
      "screen": "775-settings-shortcuts",
      "action": "click",
      "target": "role=tab[name=\"Offline AI\"]",
      "label": "Change any shortcut; open Offline AI",
      "text": null,
      "key": null,
      "body": "Keyboard shortcuts lets you filter commands, add a shortcut and Restore all defaults. Open Offline AI."
    },
    {
      "screen": "789-settings-offline-ai-ready",
      "action": "click",
      "target": "role=tab[name=\"Advanced\"]",
      "label": "Check the offline AI; open Advanced",
      "text": null,
      "key": null,
      "body": "Offline AI shows the AI health, a Graphics card setting and Remove OpenFrame AI. The AI is optional. Open Advanced."
    },
    {
      "screen": "794-settings-advanced",
      "action": "click",
      "target": "role=tab[name=\"Appearance\"]",
      "label": "Breakdown category colours are here; open Appearance",
      "text": null,
      "key": null,
      "body": "Advanced holds the Breakdown category colours and icons, with Restore Defaults. Open Appearance."
    },
    {
      "screen": "797-settings-appearance",
      "action": "view",
      "target": null,
      "label": "Choose the mode and theme",
      "text": null,
      "key": null,
      "body": "Appearance is where you choose the mode and theme."
    }
  ],
  "result": "You know where each setting lives. Diagnostics is user started and nothing is sent automatically.",
  "mistakes": [
    "Looking for project settings on the projects screen. There, the gear opens only app-wide settings.",
    "Rebinding Ctrl+A, C, V or X. Those are reserved.",
    "Expecting the AI to be installed. It is optional and downloaded from Settings."
  ],
  "troubleshooting": [
    [
      "A shortcut conflicts with another.",
      "The page warns about the conflict. Choose another key or use Restore all defaults."
    ],
    [
      "I do not see Offline AI ready.",
      "Download the model once in Offline AI. After that it works offline."
    ],
    [
      "Where is Export Diagnostics?",
      "Under SUPPORT on the Project settings tab. Nothing is sent automatically."
    ]
  ],
  "related": [
    "themes-appearance",
    "ai-assistant-setup",
    "manage-projects",
    "backup-restore"
  ],
  "next": "manage-projects"
};
