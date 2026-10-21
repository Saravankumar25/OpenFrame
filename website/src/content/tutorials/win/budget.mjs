export default {
  "id": "budget",
  "available": true,
  "reason": "",
  "path": "shoot",
  "order": 4,
  "feature": "reports-budget",
  "title": "Add a line to the budget",
  "purpose": "Add a planned cost to the budget and see the planned total update.",
  "difficulty": "Beginner",
  "minutes": 3,
  "prerequisites": [
    "A project is open.",
    "You are in Production > Budget."
  ],
  "objectives": [
    "Add a budget line",
    "Choose a category and planned amount",
    "Read the new total"
  ],
  "steps": [
    {
      "screen": "623-prod-budget-filled",
      "action": "click",
      "target": "role=button[name=\"Add Line\"]",
      "label": "Click Add Line",
      "text": null,
      "key": null,
      "body": "In Production, open Budget. It asks \"What did we plan to spend, and what have we spent?\" Totals for PLANNED, ACTUAL and DIFFERENCE sit above the lines. Click Add Line."
    },
    {
      "screen": "624-prod-budget-add-line-dialog",
      "action": "type",
      "target": "#of-bl-desc",
      "label": "Type the line item, choose a category and enter the planned amount",
      "text": "Colour grade and sound mix",
      "key": null,
      "body": "In the dialog, type the Line item * (here \"Colour grade and sound mix\"), pick the Category and enter the Planned (INR) * amount. Actual (INR) stays empty until the money is spent."
    },
    {
      "screen": "627-prod-budget-add-line-filled",
      "action": "click",
      "target": "role=button[name=\"Add Line\"]",
      "label": "Click Add Line",
      "text": null,
      "key": null,
      "body": "With the fields filled, click Add Line in the dialog."
    },
    {
      "screen": "628-prod-budget-line-added",
      "action": "view",
      "target": null,
      "label": "The line is added and the planned total updates",
      "text": null,
      "key": null,
      "body": "The line appears under its category and the planned total updates. Export makes a PDF, CSV or XLSX, and Save snapshot keeps a copy."
    }
  ],
  "result": "A new budget line with its category, and updated totals. This is a light budget for planning, not an accounting or payroll tool.",
  "mistakes": [
    "Entering an Actual amount too early. Leave it empty until the money is spent.",
    "Using the budget for formal accounting. The page says formal accounting is outside OpenFrame.",
    "Skipping the category. Lines are grouped by category, so choose one."
  ],
  "troubleshooting": [
    [
      "Add Line is blocked.",
      "Line item and Planned are required. Fill both."
    ],
    [
      "The page says \"Start a budget\".",
      "The project has no lines yet. Click Add Line to start."
    ],
    [
      "I want to change the contingency.",
      "Use Edit plan beside the totals."
    ]
  ],
  "related": [
    "reports",
    "schedule",
    "tasks",
    "call-sheet"
  ],
  "next": "tasks"
};
