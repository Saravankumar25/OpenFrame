export default {
  area: `Budget`,
  slug: `budget`,
  order: 17,
  part: `Shoot`,
  intro: [
    `Budget is a light planning tool. It answers one question: What did we plan to spend, and what have we spent? You list rough costs in groups such as Cast, Crew and Locations, give each a Planned amount, and add an Actual amount once the money is spent. Straft adds it up and tells you whether you are over or under plan.`,
    `The page says it plainly: Use a simple estimate for planning. Formal accounting is outside Straft. It is not an accounting, payroll or invoicing tool, and the budget is optional. You can ignore it entirely.`,
    `Budget is a page of the Production workspace. Click Production in the sidebar, then choose Budget in the picker at the top of the page, next to the word Production. The first time, you pick a currency and start the budget. After that the page shows three totals at the top, Planned, Actual and Difference, and a table of your lines below.`,
    `Budget values never change automatically. If the script or schedule changes, you review the budget yourself. You can freeze a copy of the budget as a snapshot, and export it as a PDF, CSV or Excel file.`,
  ],
  layout: [
    { name: `Page header`, text: `The title Budget and the line What did we plan to spend, and what have we spent? Before you start a budget there is nothing else. Afterwards, Export, Save Snapshot and the orange Add Line are at the top right.` },
    { name: `Planning note`, text: `A blue bar: Use a simple estimate for planning. Formal accounting is outside Straft.` },
    { name: `Start a budget`, text: `A card with a Currency menu (INR when it opens), the Start Budget button and the line Optional. You can ignore the budget entirely.` },
    { name: `Planned card`, text: `PLANNED, the total of the Planned amounts, and a line such as Sum of the Planned amounts · 0% contingency ₹0. Once you set a plan it reads, for example, Overall plan ₹1,00,000 · 10% contingency ₹10,000. An Edit Plan button sits under it.` },
    { name: `Actual card`, text: `ACTUAL, the total of the amounts spent, and how many lines have one, for example 2 of 4 lines have an Actual amount. It reads None yet when no line has an Actual amount.` },
    { name: `Difference card`, text: `DIFFERENCE, for example ₹1,500 under plan or ₹2,500 over plan, with the line Planned minus Actual, over the lines with an Actual amount. It reads Not spent yet until a line has an Actual amount.` },
    { name: `Budget lines table`, text: `Columns Category, Line item, Planned, Actual and Difference. Lines are grouped by category, and each group ends with a total row such as Cast total. A final All lines row totals everything. Each line has a pencil button (Edit) and a bin button (Delete) at its right.` },
    { name: `Note under the table`, text: `Budget values never change automatically. If the script or schedule changes, review the budget yourself.` },
  ],
  workflows: [
    {
      id: `budget-start`,
      title: `Start a budget in your currency`,
      purpose: `Turn the budget on for the project and pick the currency it uses.`,
      level: `Beginner`,
      prerequisites: [`A project is open.`],
      steps: [
        {
          screen: `budget-start`,
          do: `Click Production in the sidebar, then choose Budget in the picker at the top of the page.`,
          explain: `The page says Use a simple estimate for planning. Formal accounting is outside Straft. A card called Start a budget offers a Currency menu, set to INR, and the Start Budget button. A line under it says Optional. You can ignore the budget entirely.`,
        },
        {
          screen: `budget-currency-menu`,
          do: `Click the Currency menu and choose your currency.`,
          explain: `The menu lists INR, USD, EUR, GBP, CAD, AUD, NZD, JPY, CHF, SEK, NOK, DKK, ZAR, BRL, MXN, SGD and AED. All amounts on the page use this currency. You can change it later in Edit Plan.`,
        },
        {
          screen: `budget-empty-table`,
          do: `Click Start Budget.`,
          explain: `The budget opens empty. PLANNED reads ₹0 (in the currency you chose), ACTUAL reads None yet with 0 of 0 lines have an Actual amount, and DIFFERENCE reads Not spent yet. The table says: No lines yet. Add a line for each rough cost you expect. Export, Save Snapshot and Add Line are now at the top right.`,
        },
      ],
      result: `An empty budget is ready for your first line.`,
      cautions: [
        `The budget is optional. If you do not need it, leave the page alone.`,
      ],
      related: [`budget-add-first-line`, `budget-edit-plan`],
    },
    {
      id: `budget-add-first-line`,
      title: `Add your first budget line`,
      purpose: `Add one cost to the budget with a category, a planned amount and, if you have it, an actual amount.`,
      level: `Beginner`,
      prerequisites: [`A budget has been started.`],
      steps: [
        {
          screen: `budget-empty-table`,
          do: `Click Add Line at the top right.`,
          explain: `Add a line for each rough cost you expect.`,
        },
        {
          screen: `budget-add-line-sheet`,
          do: `In the Add Line sheet, look at the fields.`,
          explain: `Category (Cast when it opens), Line item, Planned (INR), Actual (INR) and Notes. The currency in brackets is the one you chose. Under Actual a line says Leave empty until the money is spent. Cancel closes the sheet. Add Line saves the line.`,
        },
        {
          screen: `budget-category-menu`,
          do: `Click the Category menu and choose one.`,
          explain: `The categories are Cast, Crew, Locations, Equipment, Art/Props, Travel/Transport, Food, Post/Other and Contingency. Lines with the same category are grouped together in the table.`,
        },
        {
          screen: `budget-add-line-error`,
          do: `Type a name in Line item, for example Lead actors, and the planned amount in Planned as a plain number.`,
          explain: `If the planned amount is not a number, for example forty thousand, a red message appears: Enter the planned amount as a non-negative number, for example 12,000 or 1500.50. Retype the amount with digits only. Amounts cannot be negative.`,
        },
        {
          screen: `budget-one-line`,
          do: `Fill in Actual if the money is already spent, add Notes if you like, then click Add Line.`,
          explain: `The sheet closes and a message says Added “Lead actors”. The line appears in the table with a Cast total under it and an All lines total. In the picture the plan was ₹40,000, the actual ₹42,500, and the line reads ₹2,500 over plan in red.`,
        },
      ],
      result: `The first line is in the budget and the three totals at the top show it.`,
      cautions: [
        `Leave Actual empty until the money is spent. A line with no Actual shows Not entered in the Actual column and Not spent yet as its difference.`,
        `Type amounts as digits, with or without commas and a decimal point.`,
      ],
      related: [`budget-add-more-lines`, `budget-read-totals`],
    },
    {
      id: `budget-add-more-lines`,
      title: `Add more lines in other categories`,
      purpose: `Build the budget out, one line per cost, across the categories you need.`,
      level: `Beginner`,
      prerequisites: [`The budget has at least one line.`],
      steps: [
        {
          screen: `budget-add-line-filled`,
          do: `Click Add Line, choose a Category, and fill in the line.`,
          explain: `The picture shows a second line being added: category Crew, Line item Camera team, Planned 25000 and Actual 21000. The first line, Lead actors, is already in the table behind the sheet.`,
        },
        {
          screen: `budget-table`,
          do: `Click Add Line in the sheet. Repeat for each cost you expect.`,
          explain: `Lines are grouped by category, each group with its own total row, for example Cast total, Crew total, Locations total and Equipment total. A message names the line you just added, for example Added “Lighting package”.`,
        },
      ],
      result: `The budget has a line for each cost, grouped by category with totals.`,
      cautions: [
        `The Contingency category is for a reserve you want as a line. A contingency percentage for the whole budget is set in Edit Plan.`,
      ],
      related: [`budget-edit-line`, `budget-edit-plan`],
    },
    {
      id: `budget-read-totals`,
      title: `Read Planned, Actual and Difference`,
      purpose: `Understand the three totals and how over and under plan are worked out.`,
      level: `Beginner`,
      prerequisites: [`The budget has some lines, ideally with Actual amounts on some.`],
      steps: [
        {
          screen: `budget-one-line`,
          do: `Look at the cards at the top of the page.`,
          explain: `PLANNED is the sum of the Planned amounts of all lines. ACTUAL is the sum of the Actual amounts, with a count such as 1 of 1 line has an Actual amount. DIFFERENCE is Planned minus Actual, worked out only over the lines that have an Actual amount, so a line that is not spent yet does not count as a saving.`,
        },
        {
          screen: `budget-table`,
          do: `Look at the table.`,
          explain: `Each line shows its own Planned, Actual and Difference. A line over plan reads, for example, ₹2,500 over plan in red. A line under plan reads, for example, ₹4,000 under plan. A line with no actual reads Not entered and Not spent yet. In the picture the plan is ₹85,000 across four lines, the actual is ₹63,500 on two of them, and the difference is ₹1,500 under plan.`,
        },
      ],
      result: `You can tell at a glance what you planned, what you spent and whether you are over or under.`,
      cautions: [
        `Difference only compares lines that have an Actual amount. Read the count under Actual to see how many lines that is.`,
        `Totals do not move when the script or schedule changes. Review the lines yourself.`,
      ],
      related: [`budget-edit-line`],
    },
    {
      id: `budget-edit-line`,
      title: `Edit or delete a budget line`,
      purpose: `Correct a line, enter an Actual amount once money is spent, or remove a line.`,
      level: `Beginner`,
      prerequisites: [`The budget has a line.`],
      steps: [
        {
          screen: `budget-table`,
          do: `Click the pencil button at the right of the line, for example Edit Lead actors.`,
          explain: `Each line has a pencil (Edit) and a bin (Delete) at its right.`,
        },
        {
          screen: `budget-edit-line-sheet`,
          do: `In the Edit line sheet, change the Category, Line item, Planned, Actual or Notes, then click Save.`,
          explain: `The sheet shows the line's current values, for example Planned 40000 and Actual 42500. Fill in Actual when the money is spent. Cancel closes the sheet without a change. The totals update when you save.`,
        },
        {
          screen: `budget-table`,
          do: `To remove a line, click the bin button at the right of the line.`,
          explain: `The line and its amounts leave the budget. Check the totals at the top afterwards.`,
        },
      ],
      result: `The line is corrected, or it is gone from the budget, and the totals are up to date.`,
      cautions: [
        `Take a snapshot before a big clean-up if you may want the old numbers.`,
      ],
      related: [`budget-snapshots`, `budget-read-totals`],
    },
    {
      id: `budget-edit-plan`,
      title: `Set the overall plan, contingency and currency`,
      purpose: `Record your overall estimate, a contingency and notes, or change the currency.`,
      level: `Intermediate`,
      prerequisites: [`A budget has been started.`],
      steps: [
        {
          screen: `budget-edit-plan`,
          do: `Click Edit Plan under PLANNED.`,
          explain: `The Budget plan sheet has Currency, Planned total, Contingency and Notes, with Cancel and Save. Planned total says Your overall estimate. Optional. It is a figure of your own and is separate from the total of the lines.`,
        },
        {
          screen: `budget-edit-plan`,
          do: `Enter the Planned total, choose Percent or Amount under Contingency, and type the value.`,
          explain: `Percent takes a percentage, shown with a % sign, and Amount takes a sum of money. Add Notes if you like.`,
        },
        {
          screen: `budget-plan-currency-menu`,
          do: `To change the currency, click the Currency menu and choose another.`,
          explain: `It lists the same seventeen currencies as when you started the budget, from INR to AED.`,
        },
        {
          screen: `budget-with-plan`,
          do: `Click Save.`,
          explain: `A message says Changed the budget plan. The PLANNED card keeps the total of your lines and now shows your plan underneath, for example Overall plan ₹1,00,000 · 10% contingency ₹10,000.`,
        },
      ],
      result: `The Planned card shows your overall plan and contingency, and the budget uses the currency you chose.`,
      cautions: [
        `After changing the currency, review your amounts. Check each line.`,
        `The overall plan and contingency are for your reference. The PLANNED total at the top is still the sum of the lines.`,
      ],
      related: [`budget-start`, `budget-read-totals`],
    },
    {
      id: `budget-snapshots`,
      title: `Save and look at a budget snapshot`,
      purpose: `Keep a frozen copy of the budget to look back at later.`,
      level: `Beginner`,
      prerequisites: [`A budget with some lines.`],
      steps: [
        {
          screen: `budget-snapshot-name-sheet`,
          do: `Click Save Snapshot at the top right.`,
          explain: `The Save budget snapshot sheet asks for a Snapshot name and says A frozen copy you can look back at later. The box suggests a name such as Pre-production estimate. Save Snapshot stays pale until you type a name. Cancel closes the sheet.`,
        },
        {
          screen: `budget-snapshots`,
          do: `Type a name and click Save Snapshot.`,
          explain: `A message says Saved budget snapshot “Pre-production estimate”, with Undo. The live budget carries on as before and you can keep editing it.`,
        },
        {
          screen: `budget-snapshot-view`,
          do: `Open the saved snapshot to look at it.`,
          explain: `It opens in its own window, titled with its name, for example Pre-production estimate. It says Saved 28 Oct, 06:47 · read-only, and gives a summary line such as Planned ₹85,000 · Actual ₹63,500 · Overall plan ₹1,00,000. The table lists the lines with their totals and an All lines row. Export… makes a file of the snapshot and Close shuts the window.`,
        },
      ],
      result: `A named, read-only copy of the budget is saved. Later changes to the live budget do not alter it.`,
      cautions: [
        `A snapshot is read-only. To change numbers, edit the live budget.`,
        `Give snapshots clear names, for example the stage of the project, so you can tell them apart.`,
      ],
      related: [`budget-export`, `budget-edit-line`],
    },
    {
      id: `budget-export`,
      title: `Export the budget as a PDF, CSV or Excel file`,
      purpose: `Make a file of the budget to share or open in a spreadsheet.`,
      level: `Beginner`,
      prerequisites: [`The budget has some lines.`],
      steps: [
        {
          screen: `budget-export`,
          do: `Click Export at the top right.`,
          explain: `The Export Budget sheet opens. Format offers PDF, CSV and Excel (XLSX). Include line notes is an empty check box. Tick it to add each line's Notes to the file.`,
        },
        {
          screen: `budget-export`,
          do: `Choose a Format, tick Include line notes if you want them, and click Export….`,
          explain: `Choose where to save the file in the window that opens. Cancel closes the sheet without making a file. A saved snapshot has its own Export… button that works the same way.`,
        },
      ],
      result: `A file of the budget is saved where you chose.`,
      cautions: [
        `Line notes are left out unless you tick Include line notes.`,
        `The file is a copy. It does not update when you change the budget.`,
      ],
      related: [`budget-snapshots`],
    },
  ],
  screens: {
    'budget-start': `The Budget page before a budget exists. A blue bar says Use a simple estimate for planning, and a Start a budget card offers the Currency menu (INR), Start Budget and the line Optional. You can ignore the budget entirely. Used in budget-start.`,
    'budget-currency-menu': `The Currency menu open on the Start a budget card, listing INR, USD, EUR, GBP, CAD, AUD, NZD, JPY, CHF, SEK, NOK, DKK, ZAR, BRL, MXN, SGD and AED. Used in budget-start.`,
    'budget-empty-table': `A budget with no lines. PLANNED ₹0, ACTUAL None yet and DIFFERENCE Not spent yet, an Edit Plan button, and a table that says No lines yet. Export, Save Snapshot and Add Line are at the top right. Used in budget-start and budget-add-first-line.`,
    'budget-add-line-sheet': `The Add Line sheet with Category set to Cast and empty Line item, Planned (INR), Actual (INR) and Notes boxes, with Cancel and Add Line. Used in budget-add-first-line.`,
    'budget-category-menu': `The Category menu open: Cast, Crew, Locations, Equipment, Art/Props, Travel/Transport, Food, Post/Other and Contingency. Used in budget-add-first-line.`,
    'budget-add-line-error': `The Add Line sheet with forty thousand typed in Planned. A red message says Enter the planned amount as a non-negative number, for example 12,000 or 1500.50. Variant, part of budget-add-first-line.`,
    'budget-one-line': `The budget with one line, Lead actors under Cast, planned ₹40,000, actual ₹42,500 and ₹2,500 over plan, with Cast total and All lines rows. Used in budget-add-first-line and budget-read-totals.`,
    'budget-add-line-filled': `The Add Line sheet filled in for a second line: Crew, Camera team, Planned 25000 and Actual 21000. Used in budget-add-more-lines.`,
    'budget-table': `The budget with lines in four categories (Cast, Crew, Locations, Equipment) and their totals. PLANNED ₹85,000, ACTUAL ₹63,500 on 2 of 4 lines, DIFFERENCE ₹1,500 under plan. Each line has a pencil and a bin button. Used in budget-add-more-lines, budget-read-totals and budget-edit-line.`,
    'budget-edit-line-sheet': `The Edit line sheet for Lead actors with its current values and Cancel and Save buttons. Used in budget-edit-line.`,
    'budget-edit-plan': `The Budget plan sheet: Currency, Planned total (Your overall estimate. Optional.), Contingency with Percent and Amount and a value, Notes, Cancel and Save. Used in budget-edit-plan.`,
    'budget-plan-currency-menu': `The Currency menu open in the Budget plan sheet, listing the seventeen currencies. Used in budget-edit-plan.`,
    'budget-with-plan': `The budget after saving a plan. The Planned card reads Overall plan ₹1,00,000 · 10% contingency ₹10,000 under ₹85,000, and a message says Changed the budget plan. Used in budget-edit-plan.`,
    'budget-snapshot-name-sheet': `The Save budget snapshot sheet with the Snapshot name box and the line A frozen copy you can look back at later, with Cancel and Save Snapshot. Used in budget-snapshots.`,
    'budget-snapshots': `The budget after saving a snapshot. A message says Saved budget snapshot “Pre-production estimate” and offers Undo. Used in budget-snapshots.`,
    'budget-snapshot-view': `A saved snapshot open in its own window: its name, Saved 28 Oct, 06:47 · read-only, a summary of Planned, Actual and Overall plan, the lines with an All lines row, and Export… and Close. Used in budget-snapshots.`,
    'budget-export': `The Export Budget sheet. Format offers PDF, CSV and Excel (XLSX). Include line notes is a check box. Cancel and Export… finish it. Used in budget-export.`,
  },
  shortcuts: [],
  limits: [
    `Budget is a light planning tool. It is not accounting, payroll or invoicing.`,
    `Budget values never change automatically when the script or schedule changes. You update them yourself.`,
    `Amounts must be non-negative numbers.`,
    `A saved snapshot is read-only.`,
  ],
  troubleshooting: [
    {
      problem: `I do not see my lines, only Start a budget.`,
      fix: `The budget has not been started. Choose a currency and click Start Budget.`,
    },
    {
      problem: `Add Line shows Enter the planned amount as a non-negative number.`,
      fix: `Planned holds something that is not a number. Use digits, with or without commas or a decimal point, for example 12,000 or 1500.50.`,
    },
    {
      problem: `Difference reads Not spent yet.`,
      fix: `No line has an Actual amount. Edit a line and fill in Actual once the money is spent.`,
    },
    {
      problem: `The Difference total does not match Planned minus Actual.`,
      fix: `Difference only compares lines that have an Actual amount. Lines not spent yet are left out. The count under ACTUAL tells you how many lines are included.`,
    },
    {
      problem: `The budget did not change when I moved scenes in the schedule.`,
      fix: `Budget values never change automatically. Edit the lines yourself.`,
    },
    {
      problem: `Save Snapshot is pale in the snapshot sheet.`,
      fix: `The Snapshot name box is empty. Type a name.`,
    },
    {
      problem: `The line notes are missing from my export.`,
      fix: `Include line notes was not ticked. Export again with it ticked.`,
    },
  ],
};
