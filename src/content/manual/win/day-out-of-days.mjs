export default {
  area: `Day Out of Days`,
  slug: `day-out-of-days`,
  order: 17,
  part: `Plan`,
  intro: [
    `Day Out of Days is a table that shows, for each cast member, which shooting days they work. Each row is an actor and each column is a shooting day, with a short code in every cell: start work, work, work finish, hold and so on. It is the page that tells you when an actor first arrives, when they are done and how many days you are paying for.`,
    `You do not fill the table in. It is worked out from the cast you confirmed in Breakdown and the scenes you put on each day in the Schedule tab. When you move a scene to another day, the table follows. If the schedule does not show something you know, for example that an actor flies in the night before, you can correct a single cell by hand. A correction is marked with an asterisk, and it never changes the screenplay, the breakdown or the schedule.`,
    `You choose which cast and which range of days to show, and between two styles of code. The table can be exported as a PDF, a spreadsheet or a CSV file. Day Out of Days is the Day Out of Days tab of Production, next to Schedule.`,
  ],
  layout: [
    { name: `Cast to show`, text: `A drop-down that starts as All cast. Choose one actor to show only that row.` },
    { name: `First day and Last day`, text: `Two drop-downs, From the first day and To the last day. Choose a shooting day in each to show only that range of columns.` },
    { name: `Code style`, text: `A drop-down with two choices: Codes: SW, W, WF, H and Codes: W and -.` },
    { name: `Corrected chip and Export`, text: `On the right. When you have changed cells by hand, a yellow chip reads, for example, 1 corrected by hand. Export saves the table you see as a file.` },
    { name: `Table`, text: `The Cast column lists each actor as a number, the actor and the character, for example 1. Ananya Rao (LAKSHMI). Then one column per shooting day, headed with the day and its date, for example Day 1 and Mon 9 Nov. An off day is a shaded column headed Off. The last five columns are Work, Hold, Travel, Start and Finish.` },
    { name: `Legend`, text: `A line under the table that spells out every code, for example SW start work · W work · WF work finish, and says: * corrected by hand. Built from the cast confirmed in Breakdown and the scenes on each day; corrections never change the schedule.` },
  ],
  workflows: [
    {
      id: `day-out-of-days-read`,
      title: `Read the Day Out of Days table`,
      purpose: `Open the report and understand every code and total in it.`,
      level: `Beginner`,
      prerequisites: [`A shooting schedule with shooting days exists.`, `Scenes with confirmed cast are scheduled on those days.`],
      steps: [
        {
          screen: `prod-dood-table`,
          do: `In Production, click the Day Out of Days tab.`,
          explain: `The table opens with all cast and all days. The cast are the actors confirmed in Breakdown for the scenes you scheduled. The table lists the cast who have scenes on shooting days.`,
        },
        {
          screen: `prod-dood-table`,
          do: `Look along a row, for example 1. Ananya Rao (LAKSHMI).`,
          explain: `Each day column holds a code or stays empty. Here the actor works only on Day 4, so the cell reads SWF: start, work and finish on the same day. The other days are empty because the actor is not working before the first or after the last work day.`,
        },
        {
          screen: `prod-dood-table`,
          do: `Look at the five columns on the right.`,
          explain: `Work counts the days the actor works. Hold counts the days they are held, paid but not working. Travel counts travel days. Start and Finish name the first and last work day, here Shoot Day 4 for both.`,
        },
        {
          screen: `prod-dood-table`,
          do: `Read the legend under the table.`,
          explain: `The codes are SW start work, W work, WF work finish, SWF start, work and finish on the same day, H hold (paid, not working), T travel, R rehearsal, F fitting, D drop and a dash for not working. SW is the first work day, WF is the last, and W is every work day between them. H is a day between the first and last work day when the actor is not scheduled.`,
        },
      ],
      result: `You can say for each actor when they start, when they finish and how many days they work.`,
      cautions: [
        `An off day is shaded and its cells stay empty.`,
        `An asterisk after a code means a person corrected that cell. See Correct a cell by hand.`,
      ],
      related: [`day-out-of-days-filter`, `day-out-of-days-correct`],
    },
    {
      id: `day-out-of-days-filter`,
      title: `Show one actor or a range of days`,
      purpose: `Narrow the table to one cast member, to some of the days, or both.`,
      level: `Beginner`,
      prerequisites: [`The Day Out of Days tab is open with a table showing.`],
      steps: [
        {
          screen: `prod-dood-one-cast`,
          do: `Open the first drop-down, All cast, and choose an actor, for example Ananya Rao (LAKSHMI).`,
          explain: `Only that actor's row stays. The list holds every actor in the table, each with the character. Choose All cast to show everybody again.`,
        },
        {
          screen: `prod-dood-day-range`,
          do: `Set From the first day to a day, for example Shoot Day 2 · Tue 10 Nov 2026, and To the last day to another, for example Shoot Day 3 · Wed 11 Nov 2026.`,
          explain: `Only the columns from the first to the last chosen day are shown. Here the table shows Day 2 and Day 3. The lists offer every shooting day and every off day.`,
        },
        {
          screen: `prod-dood-table`,
          do: `Set both drop-downs back to From the first day and To the last day.`,
          explain: `The whole schedule shows again. Cast and range only choose what you see. The codes and the cast numbers always come from the whole schedule, so a code does not change when you narrow the range.`,
        },
      ],
      result: `The table shows just the people and days you asked for, and Export saves exactly that view.`,
      cautions: [
        `If you pick a last day that comes before the first day, the table swaps them and shows the range between the two.`,
      ],
      related: [`day-out-of-days-export`],
    },
    {
      id: `day-out-of-days-code-style`,
      title: `Switch between industry codes and simple codes`,
      purpose: `Choose between the full set of codes and a plain W or dash.`,
      level: `Beginner`,
      prerequisites: [`The Day Out of Days tab is open with a table showing.`],
      steps: [
        {
          screen: `prod-dood-table`,
          do: `Open the fourth drop-down, Codes: SW, W, WF, H.`,
          explain: `Two styles are offered. Codes: SW, W, WF, H is the industry style, where the first and last work day get SW and WF, work days between them W, and gaps between them H. Codes: W and - is the plain style.`,
        },
        {
          screen: `prod-dood-simple-codes`,
          do: `Choose Codes: W and -.`,
          explain: `Every work day reads W, and every other shooting day reads a dash. Off days stay empty. The message reads Changed the Day Out of Days codes with Undo. The legend under the table shortens to match.`,
        },
      ],
      result: `The table uses the code style you chose. The style is kept with the schedule, so the export and the next visit use it as well.`,
      cautions: [
        `The style applies to the whole table, not to one row. Corrections you entered by hand stay as you set them.`,
      ],
      related: [`day-out-of-days-read`, `day-out-of-days-export`],
    },
    {
      id: `day-out-of-days-correct`,
      title: `Correct a cell by hand`,
      purpose: `Replace the code the schedule gives for one actor on one day, for example to record travel, a hold, a fitting or a rehearsal.`,
      level: `Intermediate`,
      prerequisites: [`The Day Out of Days tab is open with a table showing.`],
      steps: [
        {
          screen: `prod-dood-table`,
          do: `Click the cell of an actor and a day, for example the SWF cell of Ananya Rao (LAKSHMI) on Day 4.`,
          explain: `A menu opens. Its heading names the actor and the day, here ANANYA RAO (LAKSHMI), SHOOT DAY 4 · THU 12 NOV 2026. You can click an empty cell too, for example a day before the actor starts.`,
        },
        {
          screen: `prod-dood-cell-menu`,
          do: `Read the menu.`,
          explain: `Each code is listed with its meaning: SW Start work, W Work, WF Work finish, SWF Start, work and finish on the same day, H Hold (paid, not working), T Travel, R Rehearsal, F Fitting, D Drop and - Not working. The code the cell has now is greyed out. Below a line are Use the schedule value (SWF) and Add a note, both greyed out until the cell has been corrected.`,
        },
        {
          screen: `prod-dood-corrected`,
          do: `Choose T Travel.`,
          explain: `The cell now reads T with an asterisk and is underlined. A message reads Set Ananya Rao (LAKSHMI) on Shoot Day 4 · Thu 12 Nov 2026 to T with Undo. A yellow chip above the table reads 1 corrected by hand. The totals follow the corrected cell: here Work is 0 and Travel is 1, and Start and Finish are empty because the actor no longer works on any day.`,
        },
      ],
      result: `The cell shows your code with an asterisk, and the row's totals include it. The screenplay, breakdown and schedule are not changed.`,
      cautions: [
        `A corrected cell's label for screen readers says what the schedule would give, for example corrected by hand (schedule says SWF).`,
        `Use Undo in the message at once if you picked the wrong code. To return a corrected cell to the schedule, see Return a cell to the schedule value.`,
        `Corrections are the only thing stored on this page. If a correction is for an actor or day that is no longer in the schedule it is kept but not shown, and a yellow bar says how many.`,
      ],
      related: [`day-out-of-days-note`, `day-out-of-days-clear`],
    },
    {
      id: `day-out-of-days-note`,
      title: `Add a note to a corrected cell`,
      purpose: `Record why a cell was corrected, for example that an actor flies in the night before.`,
      level: `Intermediate`,
      prerequisites: [`The cell has already been corrected by hand, so it shows an asterisk.`],
      steps: [
        {
          screen: `prod-dood-corrected`,
          do: `Click the corrected cell, T*, and choose Add a note.`,
          explain: `Add a note is available only once the cell is corrected. If the cell already has a note, the same menu item reads Edit the note.`,
        },
        {
          screen: `prod-dood-note-dialog`,
          do: `In the Correction note dialog, type the note in the Note box and click Save.`,
          explain: `The dialog names the cell, for example Ananya Rao (LAKSHMI), Shoot Day 4 · Thu 12 Nov 2026: T. The note is optional and holds up to 500 characters. Cancel closes the dialog without saving. The message reads Changed the note for Ananya Rao (LAKSHMI) on Shoot Day 4 · Thu 12 Nov 2026 with Undo.`,
        },
        {
          screen: `prod-dood-corrected`,
          do: `Hold the pointer over the cell.`,
          explain: `The note shows as the cell's tooltip. A note is also read out with the cell by a screen reader and found by search.`,
        },
      ],
      result: `The corrected cell carries your note.`,
      cautions: [
        `Leave the Note box empty and click Save to remove an existing note. The correction itself stays.`,
        `If you change the code of a corrected cell, its note is kept.`,
      ],
      related: [`day-out-of-days-correct`, `day-out-of-days-clear`],
    },
    {
      id: `day-out-of-days-clear`,
      title: `Return a cell to the schedule value`,
      purpose: `Remove a correction so the cell follows the schedule again.`,
      level: `Beginner`,
      prerequisites: [`A cell has been corrected by hand.`],
      steps: [
        {
          screen: `prod-dood-corrected`,
          do: `Click the corrected cell and choose Use the schedule value.`,
          explain: `The menu item names the value the schedule gives, for example Use the schedule value (SWF), or (blank) when the schedule gives no code. It is greyed out for a cell that has not been corrected.`,
        },
        {
          screen: `prod-dood-table`,
          do: `Look at the table.`,
          explain: `The cell shows the schedule's code again with no asterisk. A message reads Cleared the correction for the actor on the day, with Undo. The chip counting corrections disappears when none are left. Clearing a correction also removes its note.`,
        },
      ],
      result: `The cell is back to the schedule value, with no correction and no note.`,
      cautions: [
        `If you only want to change the code, choose another code in the menu instead. That keeps the note.`,
      ],
      related: [`day-out-of-days-correct`, `day-out-of-days-note`],
    },
    {
      id: `day-out-of-days-export`,
      title: `Export the table as PDF, spreadsheet or CSV`,
      purpose: `Save the table you are looking at as a file to print or send.`,
      level: `Beginner`,
      prerequisites: [`The Day Out of Days tab is open with a table showing.`],
      steps: [
        {
          screen: `prod-dood-table`,
          do: `Set the cast, the range of days and the code style you want in the file, then click Export at the top right of the table.`,
          explain: `The file contains the cast and days shown, in the code style shown.`,
        },
        {
          screen: `prod-dood-export-dialog`,
          do: `In the Export Day Out of Days dialog, choose a Format: PDF (printable), Spreadsheet (XLSX) or CSV.`,
          explain: `The dialog names the Source, Day Out of Days · the cast and days shown, and shows the suggested file name, for example Night Bus - Day Out of Days followed by the date and the extension. A note says: Corrections are marked with *. Export is a snapshot; the schedule is not changed.`,
        },
        {
          screen: `prod-dood-export-dialog`,
          do: `Click Export…, then choose where to save the file in the Windows window that opens.`,
          explain: `When the file is saved, an Export complete box lists the document, source, format and where it was saved, with Reveal in File Manager and Close. Cancel closes the dialog without exporting.`,
        },
      ],
      result: `You have a file of the table as you saw it, and the schedule is unchanged.`,
      cautions: [
        `An export is a snapshot. Export again after you change the schedule or a correction.`,
      ],
      related: [`day-out-of-days-filter`, `day-out-of-days-code-style`],
    },
    {
      id: `day-out-of-days-empty`,
      title: `Fill the Day Out of Days when it is empty`,
      purpose: `Find out why the page shows no table and what to do in the Schedule and Breakdown.`,
      level: `Beginner`,
      prerequisites: [`The Day Out of Days tab is open.`],
      steps: [
        {
          screen: `prod-dood-empty`,
          do: `If the page says Day Out of Days comes from your shooting schedule., click Open Schedule.`,
          explain: `The page appears when the project has no shooting schedule or no shooting days. It says: Create shooting days and schedule scenes with confirmed cast. The report then shows who works on which day.`,
        },
        {
          screen: `prod-dood-table`,
          do: `Create shooting days and move scenes onto them in the Schedule tab, and confirm cast for those scenes in Breakdown. Then return to Day Out of Days.`,
          explain: `The table fills in as soon as scheduled scenes have cast. If days exist but no cast is assigned to the scheduled scenes, the page says: No cast is assigned to scheduled scenes yet. Confirm cast in Breakdown and schedule the scenes.`,
        },
      ],
      result: `The Day Out of Days shows a row for every cast member with scheduled scenes.`,
      cautions: [
        `Only cast confirmed in Breakdown count. Check Breakdown if a row you expect is missing.`,
      ],
      related: [`day-out-of-days-read`],
    },
  ],
  screens: {
    'prod-dood-table': `The Day Out of Days table for Night Bus: the four drop-downs (All cast, From the first day, To the last day, Codes: SW, W, WF, H), Export, and a table with Ananya Rao (LAKSHMI) and Suresh Babu (VENKAT RAO). Both work only on Day 4, where each shows SWF, and Fri 13 Nov is a shaded Off column. The totals read 1 work day and Start and Finish Shoot Day 4. The legend sits under the table. Used in day-out-of-days-read, day-out-of-days-filter, day-out-of-days-code-style, day-out-of-days-correct, day-out-of-days-clear, day-out-of-days-export and day-out-of-days-empty.`,
    'prod-dood-cell-menu': `The menu of the Day 4 cell of Ananya Rao (LAKSHMI): every code with its meaning (SWF greyed out as the current one), then Use the schedule value (SWF) and Add a note, both greyed out because the cell is not corrected. Used in day-out-of-days-correct.`,
    'prod-dood-corrected': `The table after the cell was set to T. It shows T with an asterisk, the chip 1 corrected by hand, the message Set Ananya Rao (LAKSHMI) on Shoot Day 4 · Thu 12 Nov 2026 to T with Undo, and the totals Work 0 and Travel 1. Used in day-out-of-days-correct, day-out-of-days-note and day-out-of-days-clear.`,
    'prod-dood-note-dialog': `The Correction note dialog for the corrected cell, with a Note box holding Flies in the night before, Cancel and Save. Used in day-out-of-days-note.`,
    'prod-dood-simple-codes': `The table with Codes: W and - chosen. Days when the actors do not work show a dash, Day 4 shows W and the off day stays empty. Used in day-out-of-days-code-style.`,
    'prod-dood-one-cast': `The table with Ananya Rao (LAKSHMI) chosen under Cast to show, so only her row is shown. Used in day-out-of-days-filter.`,
    'prod-dood-day-range': `The table with First day set to Shoot Day 2 and Last day set to Shoot Day 3: only the Day 2 and Day 3 columns of both actors are shown. Used in day-out-of-days-filter.`,
    'prod-dood-export-dialog': `The Export Day Out of Days dialog: the Source line, the formats PDF (printable), Spreadsheet (XLSX) and CSV, the suggested file name, the note that corrections are marked with *, Cancel and Export…. Used in day-out-of-days-export.`,
    'prod-dood-empty': `The Day Out of Days tab of a project with no shooting schedule: the message Day Out of Days comes from your shooting schedule. and an Open Schedule button. Used in day-out-of-days-empty.`,
  },
  shortcuts: [],
  limits: [
    `The table is worked out from Breakdown cast and the Schedule. You cannot add a cast member or a day here. Do that in Breakdown and Schedule.`,
    `You can correct one cell at a time. A correction never changes the screenplay, the breakdown or the schedule.`,
    `Only the code style is kept with the schedule. Cast and day range are view choices for the page and for an export.`,
    `Export is a snapshot to PDF, XLSX or CSV. It does not change the schedule.`,
  ],
  troubleshooting: [
    {
      problem: `An actor is missing from the table.`,
      fix: `The actor has no scene on a shooting day, or the cast was not confirmed in Breakdown. Confirm the cast for the scenes, schedule them on a day, and check that Cast to show is All cast.`,
    },
    {
      problem: `A code is not what I expected.`,
      fix: `Codes come from the schedule. SW is the first work day, WF the last, and H a day between them with no scene for the actor. Move scenes in the Schedule tab, or correct the cell by hand.`,
    },
    {
      problem: `Use the schedule value or Add a note is greyed out.`,
      fix: `Both work only on a cell that has been corrected. Choose a code for the cell first.`,
    },
    {
      problem: `A yellow bar says corrections are kept for a cast member or day that is no longer in the schedule.`,
      fix: `A correction you made refers to an actor or day that was removed from the schedule. It is kept and not shown.`,
    },
    {
      problem: `The table changed after I moved a scene.`,
      fix: `That is expected. The table follows the schedule. Cells you corrected by hand keep your code and show an asterisk.`,
    },
    {
      problem: `The page says No cast is assigned to scheduled scenes yet.`,
      fix: `Shooting days exist but no scheduled scene has confirmed cast. Confirm cast in Breakdown and make sure the scenes are on shooting days.`,
    },
  ],
};
