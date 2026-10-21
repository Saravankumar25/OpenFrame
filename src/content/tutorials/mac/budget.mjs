export default {
  "id": "budget",
  "available": true,
  "reason": "",
  "path": "shoot",
  "order": 4,
  "feature": "reports-budget",
  "title": "Start a simple budget",
  "purpose": "Start a budget, add a line with planned and actual amounts and read the totals.",
  "difficulty": "Beginner",
  "minutes": 6,
  "prerequisites": [
    "A project is open."
  ],
  "objectives": [
    "Start a budget in a currency",
    "Add a budget line",
    "Read Planned, Actual and Difference"
  ],
  "steps": [
    {
      "screen": "441-budget-start",
      "action": "click",
      "target": "[data-id=\"budget-empty\"] >> text=Start Budget",
      "label": "In Production, Budget, pick a currency and click Start Budget",
      "text": null,
      "key": null,
      "body": "Open Production, then Budget. The page says to use a simple estimate for planning, and that formal accounting is outside OpenFrame. Pick a Currency and click Start Budget. The budget is optional."
    },
    {
      "screen": "443-budget-empty-table",
      "action": "click",
      "target": "[data-id=\"budget-add-line\"]",
      "label": "Click Add Line",
      "text": null,
      "key": null,
      "body": "The table is empty. Click Add Line."
    },
    {
      "screen": "444-budget-add-line-sheet",
      "action": "type",
      "target": "[data-id=\"of-bl-desc\"]",
      "label": "Choose a category, type the line item, the planned amount and, once spent, the actual amount",
      "text": "Camera team",
      "key": null,
      "body": "In the Add Line sheet, choose a Category, type the Line item, and enter the planned amount. Enter the actual amount once the money is spent, and leave it empty until then."
    },
    {
      "screen": "448-budget-add-line-filled",
      "action": "click",
      "target": "[data-id=\"budget-line-sheet-primary\"]",
      "label": "Click Add Line",
      "text": null,
      "key": null,
      "body": "Check the amounts, add Notes if you like, then click Add Line."
    },
    {
      "screen": "449-budget-table",
      "action": "view",
      "target": null,
      "label": "Lines group by category with totals, and Planned, Actual and Difference sit at the top",
      "text": null,
      "key": null,
      "body": "Lines group by category with totals. PLANNED, ACTUAL and DIFFERENCE sit at the top, and an overspend shows as over plan. Budget values never change automatically. Save Snapshot keeps a copy and Export makes a file."
    }
  ],
  "result": "A budget with at least one line and clear Planned, Actual and Difference totals.",
  "mistakes": [
    "Typing an actual amount before the money is spent. Leave it empty until then.",
    "Using the budget as accounting or payroll. It is a light planning tool."
  ],
  "troubleshooting": [
    [
      "I picked the wrong currency.",
      "Choose the Currency before you click Start Budget. The Add Line sheet shows the currency beside each amount, so check it there."
    ],
    [
      "Totals do not match my sums.",
      "Open the lines and check each planned and actual amount."
    ]
  ],
  "related": [
    "reports",
    "call-sheet",
    "tasks"
  ],
  "next": "tasks"
};
