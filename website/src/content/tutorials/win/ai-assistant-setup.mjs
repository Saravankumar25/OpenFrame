export default {
  "id": "ai-assistant-setup",
  "available": true,
  "reason": "",
  "path": "ai",
  "order": 1,
  "feature": "ai-assistant",
  "title": "Set up the offline AI assistant",
  "purpose": "Find the Offline AI settings, see the ready state and know what runs on your computer.",
  "difficulty": "Beginner",
  "minutes": 4,
  "prerequisites": [
    "A project is open.",
    "Enough free disk space for the one-time download of about 1.2 GB if you choose to use the assistant."
  ],
  "objectives": [
    "Understand that the assistant is optional",
    "Open Offline AI settings",
    "Read the status and technical details"
  ],
  "steps": [
    {
      "screen": "013-shell-project-home",
      "action": "click",
      "target": "button[aria-label=\"Assistant\"]",
      "label": "Click the Assistant button in the top bar",
      "text": null,
      "key": null,
      "body": "The assistant is optional, and OpenFrame works normally without it. It downloads about 1.2 GB once, when you start the download, and then runs offline on this computer. Click the Assistant button in the top bar."
    },
    {
      "screen": "374-ai-panel-ready",
      "action": "click",
      "target": "button[aria-label=\"Offline AI settings\"]",
      "label": "Open the ... menu of the AI Assistant",
      "text": null,
      "key": null,
      "body": "The AI Assistant panel shows its status. Here it reads \"AI Ready\". Click the ... menu of the panel."
    },
    {
      "screen": "375-ai-settings-menu",
      "action": "click",
      "target": "role=menuitem[name=\"Offline AI settings…\"]",
      "label": "Choose Offline AI settings…",
      "text": null,
      "key": null,
      "body": "The OFFLINE AI menu has \"Offline AI settings…\" and \"Stop Offline AI (free memory)\". Choose \"Offline AI settings…\"."
    },
    {
      "screen": "378-ai-offline-settings",
      "action": "click",
      "target": "main >> summary",
      "label": "AI Ready. Technical details lists the engine and models",
      "text": null,
      "key": null,
      "body": "The Offline AI tab says \"AI runs on this computer.\" It shows the status \"AI Ready\" and buttons to stop or remove OpenFrame AI. Click Technical details to see more."
    },
    {
      "screen": "379-ai-offline-technical-details",
      "action": "view",
      "target": null,
      "label": "Everything runs on this computer",
      "text": null,
      "key": null,
      "body": "The details list the engine and the models. They say everything runs on this computer, and the only network use is the model download you start."
    }
  ],
  "result": "You know where the assistant lives, that it is optional and that it runs on your computer after the download.",
  "mistakes": [
    "Expecting the assistant to work before it is set up. It needs the one-time download first.",
    "Thinking it sends your project out. Prompts, project text and answers are never sent anywhere."
  ],
  "troubleshooting": [
    [
      "The panel does not say AI Ready",
      "Open Offline AI settings and check the status. The download is yours to start, so the assistant stays off until you do."
    ],
    [
      "I want my memory back",
      "Choose \"Stop Offline AI (free memory)\". Use \"Remove OpenFrame AI…\" to remove it."
    ]
  ],
  "related": [
    "ai-assistant-ask",
    "settings",
    "backup-restore"
  ],
  "next": "ai-assistant-ask"
};
