export default {
  "id": "first-launch",
  "available": true,
  "reason": "",
  "path": "getting-started",
  "order": 1,
  "feature": null,
  "title": "Start OpenFrame Studio for the first time",
  "purpose": "Enter your name on the welcome sheet and reach Application Home, ready for your first project.",
  "difficulty": "Beginner",
  "minutes": 2,
  "prerequisites": [
    "OpenFrame Studio is installed on a Mac with Apple silicon running macOS 14 or later.",
    "You have opened the app and passed the team password screen of the internal build."
  ],
  "objectives": [
    "Enter the name that appears on your notes and tasks",
    "Reach Application Home",
    "Know that no account is needed"
  ],
  "steps": [
    {
      "screen": "001-home-welcome",
      "action": "type",
      "target": "a[aria-label=\"Your name: Welcome, a name typed\"]",
      "label": "Type the name you want on your notes and tasks",
      "text": "E2E Tester",
      "key": null,
      "body": "The \"Welcome to OpenFrame Studio\" sheet asks \"What should we call you?\". Type your own name in the \"Your name (required)\" field. It appears on notes, tasks and project activity."
    },
    {
      "screen": "002-home-welcome-filled",
      "action": "click",
      "target": "[data-id=\"welcome-continue\"]",
      "label": "Click Continue",
      "text": null,
      "key": null,
      "body": "Click Continue. If the field is empty, the sheet shows \"Enter your name.\" and waits."
    },
    {
      "screen": "003-home-empty",
      "action": "view",
      "target": null,
      "label": "Application Home opens, ready for your first project",
      "text": null,
      "key": null,
      "body": "Application Home opens and says \"Create your first project.\" There is no account. Your work is stored on this Mac, and the status line reads \"Offline · Your projects stay on this computer\"."
    }
  ],
  "result": "Your name is saved on this Mac and Application Home is open with no projects yet.",
  "mistakes": [
    "Leaving the name field empty. The sheet shows \"Enter your name.\" and will not continue until you type one.",
    "Looking for a sign-up or login. OpenFrame Studio has no account; the name only labels your notes, tasks and activity.",
    "Worrying that the name is permanent. You can change it any time in Settings."
  ],
  "troubleshooting": [
    [
      "The Continue button does nothing.",
      "Check that the Your name field has text in it. A name is required."
    ],
    [
      "I see a password screen before the welcome sheet.",
      "Internal builds ask for the team password first. Enter the one your team gave you; the public release will not have this step."
    ],
    [
      "I want a different name later.",
      "Open Settings from the toolbar on Application Home and change it there."
    ]
  ],
  "related": [
    "create-project",
    "project-tour"
  ],
  "next": "create-project"
};
