export default {
  "id": "first-launch",
  "available": true,
  "reason": "",
  "path": "getting-started",
  "order": 1,
  "feature": null,
  "title": "Start OpenFrame Studio for the first time",
  "purpose": "Open OpenFrame Studio, tell it your name and reach Application Home.",
  "difficulty": "Beginner",
  "minutes": 2,
  "prerequisites": [
    "OpenFrame Studio is installed on Windows 11 and ready to open."
  ],
  "objectives": [
    "Enter the name OpenFrame shows on notes, tasks and activity",
    "Reach Application Home",
    "Know where to change your name later"
  ],
  "steps": [
    {
      "screen": "001-app-first-launch-welcome",
      "action": "key",
      "target": "#dn-name",
      "label": "OpenFrame asks what to call you. Clear the suggested name",
      "text": null,
      "key": "Backspace",
      "body": "The first time you open OpenFrame Studio, the \"Welcome to OpenFrame Studio\" dialog asks \"What should we call you?\". The \"Your name *\" box already holds a suggested name, so clear it first."
    },
    {
      "screen": "002-app-first-launch-name-required",
      "action": "type",
      "target": "#dn-name",
      "label": "Type your name, then click Continue",
      "text": "Maya Iyer",
      "key": null,
      "body": "Type the name you want to see on notes, tasks and project activity, then click Continue. If the box is empty you see \"Enter your name.\" and Continue stays disabled."
    },
    {
      "screen": "003-app-home-empty",
      "action": "view",
      "target": null,
      "label": "Application Home opens, ready for your first project",
      "text": null,
      "key": null,
      "body": "Application Home opens, with the buttons Create Project, Open Existing Project and \"Restore Backup or Package…\". Your name shows as the avatar at the top right, and you can change it there at any time."
    }
  ],
  "result": "OpenFrame knows your name and you are on Application Home, ready to make your first project. There is no account to create.",
  "mistakes": [
    "Leaving the name box empty. The field is required, so Continue stays disabled until you type a name.",
    "Looking for a sign-in. OpenFrame has no account. The name stays on this computer."
  ],
  "troubleshooting": [
    [
      "Continue is greyed out",
      "The name box is empty. Type a name and the message \"Enter your name.\" goes away."
    ],
    [
      "I typed the wrong name",
      "Click the avatar on Application Home and change it. The name is only used on notes, tasks and project activity."
    ]
  ],
  "related": [
    "create-project",
    "project-tour",
    "settings"
  ],
  "next": "create-project"
};
