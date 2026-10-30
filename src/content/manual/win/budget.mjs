export default {
  area: `Budget`,
  slug: `budget`,
  order: 22,
  part: `Shoot`,
  intro: [
    `The Budget tab is a light planning budget: what did we plan to spend, and what have we spent? You list rough costs as lines, each with a Planned amount and, once the money is spent, an Actual amount. OpenFrame adds them up by category and shows the difference between plan and actual.`,
    `A banner at the top says it plainly: Use a simple estimate for planning. Formal accounting is outside OpenFrame. The budget is for you and your team to keep a sense of cost. It is not an accounting, payroll or invoicing tool. It is also optional. You can ignore it entirely.`,
    `Budget values never change by themselves. If the script or the schedule changes, you review and update the lines yourself. Three tiles at the top summarise the budget: Planned, Actual and Difference.`,
    `You can save a snapshot, a frozen copy of the budget with a name you choose, such as Pre-production estimate, and look at it later. You can also export the budget as a PDF, CSV or Excel file.`,
  ],
  layout: [
    { name: `Heading`, text: `Budget, with the line What did we plan to spend, and what have we spent?.` },
    { name: `Export, Save snapshot and Add Line`, text: `At the top right, once a budget exists. Export saves a document, Save snapshot freezes a named copy, and Add Line (the orange button) adds a cost line.` },
    { name: `Banner`, text: `A blue bar: Use a simple estimate for planning. Formal accounting is outside OpenFrame.` },
    { name: `PLANNED tile`, text: `The total of all Planned amounts, a line showing your overall plan and contingency, and Edit plan. With no overall plan it says Sum of the Planned amounts. The contingency shows as a percentage and an amount, for example 0% contingency ₹0.` },
    { name: `ACTUAL tile`, text: `The total of the Actual amounts you have entered, or None yet, and a count such as 2 of 6 lines have an Actual amount.` },
    { name: `DIFFERENCE tile`, text: `Planned minus Actual over the lines that have an Actual amount, written in words such as ₹23,000 under plan or ₹7,000 over plan, or Not spent yet.` },
    { name: `Budget lines table`, text: `The columns are CATEGORY, LINE ITEM, PLANNED, ACTUAL and DIFFERENCE, and two buttons on each line: Edit and Delete. Lines are grouped by category, each group ends with a category total, and a final row, All lines, gives the grand totals. Only categories that have lines are shown.` },
    { name: `Budget note`, text: `At the bottom of the table: Budget values never change automatically. If the script or schedule changes, review the budget yourself.` },
    { name: `SAVED SNAPSHOTS`, text: `A card below the table, shown once you have saved a snapshot. Each snapshot shows its name, the date and time, and the Planned and Actual totals, with a View button and a bin button.` },
  ],
  workflows: [
    {
      id: `budget-start`,
      title: `Start a budget`,
      purpose: `Create the project's budget in the currency you will spend in.`,
      level: `Beginner`,
      prerequisites: [`A project is open. It has no budget yet.`],
      steps: [
        {
          screen: `prod-budget-start`,
          do: `Open Production and click Budget in the row of tabs.`,
          explain: `With no budget, the page shows the banner and a card called START A BUDGET. A note at the bottom of the card says Optional. You can ignore the budget entirely.`,
        },
        {
          screen: `prod-budget-start`,
          do: `Choose a currency in the Currency box.`,
          explain: `The list has USD, EUR, GBP, INR, CAD, AUD, NZD, JPY, CHF, SEK, NOK, DKK, ZAR, BRL, MXN, SGD and AED. The box starts at INR or GBP when Windows is set for India or the United Kingdom, and at USD otherwise. You can change the currency later in Edit plan.`,
        },
        {
          screen: `prod-budget-start`,
          do: `Click Start budget.`,
          explain: `An empty budget is created. The tiles appear with no amounts, and the table says No lines yet. Add a line for each rough cost you expect. Export, Save snapshot and Add Line now show at the top right.`,
        },
      ],
      result: `The project has a budget, ready for lines.`,
      cautions: [
        `The budget is optional. If you never click Start budget, nothing else in OpenFrame is affected.`,
      ],
      related: [`budget-add-line`, `budget-edit-plan`],
    },
    {
      id: `budget-read`,
      title: `Read the budget`,
      purpose: `Understand the three tiles and the lines table.`,
      level: `Beginner`,
      prerequisites: [`The budget has at least one line.`],
      steps: [
        {
          screen: `prod-budget-filled`,
          do: `Read the three tiles across the top.`,
          explain: `PLANNED shows ₹12,59,000, the sum of every Planned amount, with Sum of the Planned amounts and 0% contingency ₹0. ACTUAL shows ₹4,92,000, with 2 of 6 lines have an Actual amount, so only two lines have been spent so far. DIFFERENCE shows ₹23,000 under plan, which is Planned minus Actual over those two lines only, not over the whole budget. Amounts are shown in your budget currency.`,
        },
        {
          screen: `prod-budget-filled`,
          do: `Read the table row by row.`,
          explain: `Each line shows its category, the line item, Planned and Actual. A line with no Actual says Not entered, and its DIFFERENCE says Not spent yet. A line that cost less than planned says, for example, ₹30,000 under plan. One that cost more says, for example, ₹7,000 over plan. The words, not only the colour, tell you which way it went.`,
        },
        {
          screen: `prod-budget-filled`,
          do: `Look at the rows under each category and at the bottom.`,
          explain: `Each category ends with its own total, for example Cast total. The last row, All lines, gives ₹12,59,000 planned, ₹4,92,000 actual and ₹23,000 under plan. The categories are Cast, Crew, Locations, Equipment, Art/Props, Travel/Transport, Food, Post/Other and Contingency. A category with no lines is not shown.`,
        },
      ],
      result: `You know how much is planned, how much is spent and where the budget stands.`,
      cautions: [
        `The Difference only covers lines with an Actual amount. A budget that looks under plan can still run over when the other lines are spent.`,
      ],
      related: [`budget-add-line`, `budget-edit-line`],
    },
    {
      id: `budget-add-line`,
      title: `Add a budget line`,
      purpose: `Add one cost with a category, a description and a planned amount.`,
      level: `Beginner`,
      prerequisites: [`The project has a budget.`],
      steps: [
        {
          screen: `prod-budget-filled`,
          do: `Click Add Line at the top right.`,
          explain: `The Add Line dialog opens.`,
        },
        {
          screen: `prod-budget-add-line-dialog`,
          do: `Read the fields in the Add Line dialog.`,
          explain: `Category is a list: Cast, Crew, Locations, Equipment, Art/Props, Travel/Transport, Food, Post/Other and Contingency. Line item * is the description, and it is required. Planned (INR) * is the amount you expect to spend, and it is required. Actual (INR) is for what you really spent. Under it a hint says Leave empty until the money is spent. Notes is a free text box. The currency in brackets is the budget's currency.`,
        },
        {
          screen: `prod-budget-add-line-error-description`,
          do: `If you click Add Line with Line item * empty, the dialog says A line needs a description.`,
          explain: `Type a description. Nothing is added until the line has one.`,
        },
        {
          screen: `prod-budget-add-line-error-planned`,
          do: `If the Planned amount is not a number, the dialog says Enter the planned amount as a non-negative number, for example 12,000 or 1500.50.`,
          explain: `The picture has the words lots in Planned (INR) *. Type digits only, with an optional comma and decimal point. Negative amounts are not allowed.`,
        },
        {
          screen: `prod-budget-add-line-filled`,
          do: `Choose Post/Other, type Colour grade and sound mix, type 180000 in Planned (INR) *, add a note such as Two weeks at an outside studio, and click Add Line.`,
          explain: `Leave Actual (INR) empty because the money is not spent. You can press Enter in the Planned or Actual box instead of clicking Add Line. Cancel, or the Close button at the top, leaves without adding.`,
        },
        {
          screen: `prod-budget-line-added`,
          do: `Look for the message at the bottom: Added “Colour grade and sound mix”.`,
          explain: `The line is in the table under its category, and the totals have gone up. The message has Undo and Dismiss buttons. Undo takes the line back out.`,
        },
      ],
      result: `The new line is in the table and in the totals.`,
      cautions: [
        `The line needs a description and a planned amount. Actual can wait.`,
        `A line's notes show under its description in the table.`,
      ],
      related: [`budget-edit-line`, `budget-delete-line`],
    },
    {
      id: `budget-edit-line`,
      title: `Edit a line and enter what was spent`,
      purpose: `Change a line, or fill in its Actual amount once the money is spent.`,
      level: `Beginner`,
      prerequisites: [`The budget has at least one line.`],
      steps: [
        {
          screen: `prod-budget-filled`,
          do: `Click the Edit button at the right of the line. Its tooltip names the line, for example Edit Lead and supporting cast fees.`,
          explain: `Each line has an Edit and a Delete button at the end of its row.`,
        },
        {
          screen: `prod-budget-edit-line-dialog`,
          do: `Change what you need in the Edit line dialog and click Save.`,
          explain: `The fields are the same as when adding. They open filled in: here Cast, Lead and supporting cast fees, Planned 450000 and Actual 420000. Enter the Actual amount as the money is spent. The same errors as in Add Line apply, and an Actual amount that is not a number shows Enter the actual amount as a non-negative number, or leave it empty. Cancel closes without a change.`,
        },
      ],
      result: `The line shows its new values, and the Difference tile, category total and All lines row are updated. A message at the bottom says Edited with the line name, with Undo.`,
      cautions: [
        `Clear Actual to take an amount back out. The line then says Not entered again.`,
      ],
      related: [`budget-add-line`, `budget-read`],
    },
    {
      id: `budget-delete-line`,
      title: `Delete a budget line`,
      purpose: `Remove a line you no longer need, and undo it if you change your mind.`,
      level: `Beginner`,
      prerequisites: [`The budget has at least one line.`],
      steps: [
        {
          screen: `prod-budget-filled`,
          do: `Click the Delete button at the right of the line. Its tooltip names the line, for example Delete Unit meals for 18 days.`,
          explain: `The line is removed at once. There is no confirmation box. Instead a message at the bottom says Deleted with the line name, and offers Undo.`,
        },
        {
          screen: `prod-budget-line-added`,
          do: `Click Undo in the message if you deleted the wrong line.`,
          explain: `The picture shows the same message after adding a line, with Undo and Dismiss. Use Undo before you close the message.`,
        },
      ],
      result: `The line is gone, and the totals are lower.`,
      cautions: [
        `There is no confirmation dialog. Check the line name in the tooltip before you click.`,
      ],
      related: [`budget-edit-line`],
    },
    {
      id: `budget-edit-plan`,
      title: `Set the overall plan, contingency and currency`,
      purpose: `Enter an overall planned total and a contingency, change the currency, or add a note.`,
      level: `Intermediate`,
      prerequisites: [`The project has a budget.`],
      steps: [
        {
          screen: `prod-budget-filled`,
          do: `Click Edit plan in the PLANNED tile.`,
          explain: `The Budget plan dialog opens.`,
        },
        {
          screen: `prod-budget-edit-plan-dialog`,
          do: `Read the fields.`,
          explain: `Currency is the list of currencies. Planned total is your overall estimate for the film, and it is optional, as the hint Your overall estimate. Optional. says. Contingency has a switch, Percent or Amount, and a box for the value. Percent shows a percent sign beside the box. Notes is free text and shows under the table.`,
        },
        {
          screen: `prod-budget-edit-plan-error`,
          do: `If you type more than 100 in the Percent box, the dialog says Contingency must be between 0% and 100%.`,
          explain: `The picture has 150 typed with Percent chosen. Type a value from 0 to 100, or click Amount to enter a sum of money instead. Other messages are Enter the planned total as a non-negative amount. and Enter the contingency as a non-negative amount.`,
        },
        {
          screen: `prod-budget-edit-plan-dialog`,
          do: `Fill in what you need and click Save.`,
          explain: `The PLANNED tile now shows your overall plan and the contingency, for example 10% contingency with its amount. A message says Changed the budget plan, with Undo. Cancel closes without a change.`,
        },
      ],
      result: `The budget shows your overall plan, contingency, currency and note.`,
      cautions: [
        `Changing the currency changes how amounts are written. It does not convert the amounts you have entered.`,
        `The contingency is shown in the PLANNED tile beside your overall plan. Check there how it relates to the lines' total.`,
      ],
      related: [`budget-read`, `budget-start`],
    },
    {
      id: `budget-save-snapshot`,
      title: `Save a named budget snapshot`,
      purpose: `Keep a frozen copy of the budget, for example before a big change.`,
      level: `Beginner`,
      prerequisites: [`The project has a budget with lines.`],
      steps: [
        {
          screen: `prod-budget-filled`,
          do: `Click Save snapshot at the top right.`,
          explain: `A snapshot is a frozen copy of the budget as it stands now. The live budget keeps going and can change.`,
        },
        {
          screen: `prod-budget-snapshot-name-dialog`,
          do: `In the Save budget snapshot dialog, type a name in Snapshot name *.`,
          explain: `The name is required. A hint says A frozen copy you can look back at later. The example is Pre-production estimate. Save snapshot stays greyed out until you type a name. Cancel closes without saving.`,
        },
        {
          screen: `prod-budget-snapshots-saved`,
          do: `Click Save snapshot.`,
          explain: `A card called SAVED SNAPSHOTS appears under the table. It lists Pre-production estimate with its date and time, Planned ₹12,59,000 · Actual ₹4,92,000, a View button and a bin button. A message at the bottom says Saved budget snapshot with the name, with Undo.`,
        },
      ],
      result: `The snapshot is listed under SAVED SNAPSHOTS.`,
      cautions: [
        `A snapshot never updates. Save another with a new name to capture later changes.`,
      ],
      related: [`budget-view-snapshot`, `budget-delete-snapshot`],
    },
    {
      id: `budget-view-snapshot`,
      title: `View a saved snapshot`,
      purpose: `Open a snapshot to read, or export, the budget as it was.`,
      level: `Beginner`,
      prerequisites: [`At least one snapshot has been saved.`],
      steps: [
        {
          screen: `prod-budget-snapshots-saved`,
          do: `In SAVED SNAPSHOTS, click View beside the snapshot.`,
          explain: `Each snapshot row shows the Planned and Actual totals so you can compare before you open it.`,
        },
        {
          screen: `prod-budget-snapshot-view`,
          do: `Read the dialog.`,
          explain: `The title is the snapshot name, for example Pre-production estimate, and a line says Saved with the date and time and read-only. Under it are the Planned and Actual totals and the full table, with no Edit or Delete buttons. Nothing in the dialog can be changed.`,
        },
        {
          screen: `prod-budget-snapshot-view`,
          do: `Click Export to save this snapshot as a document, or Close to leave.`,
          explain: `Export here saves the snapshot, not the live budget. The Export Budget dialog is the same as the live one.`,
        },
      ],
      result: `You have looked at, or exported, the budget as it was when you saved it.`,
      cautions: [],
      related: [`budget-save-snapshot`, `budget-export`],
    },
    {
      id: `budget-delete-snapshot`,
      title: `Delete a saved snapshot`,
      purpose: `Remove a snapshot you no longer need.`,
      level: `Beginner`,
      prerequisites: [`At least one snapshot has been saved.`],
      steps: [
        {
          screen: `prod-budget-snapshots-saved`,
          do: `Click the bin button beside the snapshot. Its tooltip names it, for example Delete snapshot Pre-production estimate.`,
          explain: `The snapshot is removed from the list at once, with no confirmation box. A message at the bottom says Deleted budget snapshot with its name, and offers Undo.`,
        },
        {
          screen: `prod-budget-snapshots-saved`,
          do: `Click Undo in the message if you deleted the wrong one.`,
          explain: `Use Undo before you close the message. Deleting a snapshot never changes the live budget.`,
        },
      ],
      result: `The snapshot is gone from SAVED SNAPSHOTS.`,
      cautions: [
        `There is no confirmation dialog. Check the name in the tooltip before you click.`,
      ],
      related: [`budget-save-snapshot`],
    },
    {
      id: `budget-export`,
      title: `Export the budget as PDF, CSV or Excel`,
      purpose: `Save a simple budget summary you can print, send or open in a spreadsheet.`,
      level: `Beginner`,
      prerequisites: [`The project has a budget.`],
      steps: [
        {
          screen: `prod-budget-filled`,
          do: `Click Export at the top right.`,
          explain: `The Export Budget dialog opens.`,
        },
        {
          screen: `prod-budget-export-dialog`,
          do: `Read the dialog and choose a FORMAT.`,
          explain: `It says A simple budget summary: planned total, contingency, category totals and line items. FORMAT offers PDF printable, which is selected when the dialog opens, CSV spreadsheet-friendly text and Excel (XLSX) one sheet per table. A line shows the suggested file name, for example Night Bus - Budget Summary - 2026-10-31.pdf.`,
        },
        {
          screen: `prod-budget-export-dialog`,
          do: `Under OPTIONS, tick Include line notes if you want each line's notes in the document.`,
          explain: `It is off by default, so notes are left out unless you ask. A note says Private notes excluded. Export creates a snapshot and does not change your project.`,
        },
        {
          screen: `prod-budget-export-dialog`,
          do: `Click Export… and choose a folder and file name in the Windows window.`,
          explain: `Cancel, or the Close button at the top, leaves without saving.`,
        },
      ],
      result: `A file with the budget summary is saved where you chose.`,
      cautions: [
        `The export is a copy. It does not change your project.`,
        `Line notes are included only when you tick Include line notes.`,
      ],
      related: [`budget-view-snapshot`, `budget-read`],
    },
  ],
  screens: {
    'prod-budget-start': `The Budget tab in a project with no budget. The banner and a START A BUDGET card with a Currency box, Start budget and the note Optional. You can ignore the budget entirely. Used in budget-start.`,
    'prod-budget-filled': `The Budget tab with six lines in six categories. PLANNED shows ₹12,59,000, ACTUAL ₹4,92,000 and DIFFERENCE ₹23,000 under plan. The table shows each line with Edit and Delete, category totals and the All lines row. Used in budget-read, budget-add-line, budget-edit-line, budget-delete-line, budget-edit-plan, budget-save-snapshot and budget-export.`,
    'prod-budget-add-line-dialog': `The Add Line dialog with everything empty. Category (Cast), Line item *, Planned (INR) *, Actual (INR) with its hint, Notes, Cancel and Add Line. Used in budget-add-line.`,
    'prod-budget-add-line-error-description': `The Add Line dialog after Add Line with no description. It says A line needs a description. Variant, part of budget-add-line.`,
    'prod-budget-add-line-error-planned': `The Add Line dialog with the word lots in the Planned box. It says Enter the planned amount as a non-negative number, for example 12,000 or 1500.50. Variant, part of budget-add-line.`,
    'prod-budget-add-line-filled': `The Add Line dialog filled in: Post/Other, Colour grade and sound mix, 180000 planned, and a note. Used in budget-add-line.`,
    'prod-budget-line-added': `The Budget tab right after adding a line. Colour grade and sound mix is in the table, and a message at the bottom says Added with the line name, with Undo and Dismiss. Used in budget-add-line and budget-delete-line.`,
    'prod-budget-edit-line-dialog': `The Edit line dialog open on Lead and supporting cast fees, with Planned 450000 and Actual 420000. Cancel and Save. Used in budget-edit-line.`,
    'prod-budget-edit-plan-dialog': `The Budget plan dialog: Currency, Planned total, Contingency with Percent and Amount and a value box, Notes, Cancel and Save. Used in budget-edit-plan.`,
    'prod-budget-edit-plan-error': `The Budget plan dialog with 150 typed as a Percent contingency. It says Contingency must be between 0% and 100%. Used in budget-edit-plan.`,
    'prod-budget-snapshot-name-dialog': `The Save budget snapshot dialog with the name Pre-production estimate typed in Snapshot name *. Cancel and Save snapshot. Used in budget-save-snapshot.`,
    'prod-budget-snapshots-saved': `The Budget tab with a SAVED SNAPSHOTS card under the table. It lists Pre-production estimate with the date and Planned and Actual totals, and has View and a bin button. Used in budget-save-snapshot, budget-view-snapshot and budget-delete-snapshot.`,
    'prod-budget-snapshot-view': `The snapshot open in a dialog titled Pre-production estimate. It is read-only, shows the Planned and Actual totals and the full table without Edit or Delete, and has Export and Close. Used in budget-view-snapshot.`,
    'prod-budget-export-dialog': `The Export Budget dialog: a description, FORMAT with PDF printable (selected), CSV and Excel, the suggested file name, OPTIONS with Include line notes, the private notes note, and Cancel and Export…. Used in budget-export.`,
  },
  shortcuts: [
    [`Enter`, `Add or save the line from the Planned or Actual box in the line dialog`],
  ],
  limits: [
    `The budget is a simple planning estimate. It is not an accounting, payroll or invoicing tool.`,
    `Budget values never change automatically when the script or schedule changes.`,
    `Each project has one live budget. You can keep as many named snapshots of it as you like.`,
    `A snapshot is read-only. You cannot edit it.`,
    `Changing the currency does not convert amounts.`,
    `Amounts must be zero or more. Negative amounts are not accepted.`,
  ],
  troubleshooting: [
    {
      problem: `I do not see Add Line or Export.`,
      fix: `They show once the project has a budget. Click Start budget first.`,
    },
    {
      problem: `The dialog says A line needs a description.`,
      fix: `Type something in Line item *. A line cannot be added without one.`,
    },
    {
      problem: `The dialog says Enter the planned amount as a non-negative number.`,
      fix: `Type digits only, for example 12,000 or 1500.50. Letters and negative signs are not accepted.`,
    },
    {
      problem: `Contingency must be between 0% and 100%.`,
      fix: `Percent only takes a value from 0 to 100. Click Amount to enter a sum of money instead.`,
    },
    {
      problem: `DIFFERENCE says Not spent yet.`,
      fix: `No line has an Actual amount. Edit a line and enter what was spent.`,
    },
    {
      problem: `I deleted a line or snapshot by mistake.`,
      fix: `Click Undo in the message at the bottom of the window straight away. Once the message has gone, enter the line again.`,
    },
    {
      problem: `The page shows a Review budget bar with Mark reviewed.`,
      fix: `It reminds you to check the budget against the current script and schedule. Review the lines, then click Mark reviewed.`,
    },
  ],
};
