export default {
  area: `Day Out of Days`,
  slug: `day-out-of-days`,
  order: 12,
  part: `Plan`,
  intro: [
    `The Day Out of Days report shows, for each cast member, which shooting days they work. It is a grid with one row per actor and one column per shooting day, so you can see at a glance who is needed when, and for how many days.`,
    `You do not fill it in. It is built from the cast confirmed in the Breakdown and the scenes you placed in each shooting day in the Schedule. Change the Schedule and the report follows.`,
    `Open it from Production in the sidebar, using the pop-up at the top of the page: choose Day Out of Days. If no schedule exists yet, the page tells you to create one first and offers Open Schedule.`,
  ],
  layout: [
    { name: `Filter bar`, text: `Four pop-up menus and Export. Cast to show (All cast), First day (From the first day), Last day (To the last day) and Code style narrow the grid. Export saves it as a file.` },
    { name: `Grid`, text: `The Cast column lists each actor with a number, for example 1. Devi. Then one column per shooting day, headed with the day and date, for example Day 1 Mon 14 Jun. A code in a cell says what the actor does that day, and an empty cell means no work.` },
    { name: `Totals`, text: `After the days come Work, Hold and Travel, the number of days of each kind, then Start and Finish, the first and last day the actor works.` },
    { name: `Legend`, text: `A line under the grid explains every code and notes that the grid is built from the cast confirmed in Breakdown and the scenes on each day.` },
  ],
  workflows: [
    {
      id: `day-out-of-days-open`,
      title: `Open the Day Out of Days report`,
      purpose: `Reach the report and understand what it needs before it shows anything.`,
      level: `Beginner`,
      prerequisites: [`The project has a screenplay.`],
      steps: [
        {
          screen: `schedule-dood-empty`,
          do: `Click Production in the sidebar, then choose Day Out of Days in the pop-up at the top of the page.`,
          explain: `Without a schedule the page says: Day Out of Days comes from your shooting schedule. It adds: Create shooting days and schedule scenes with confirmed cast. The report then shows who works on which day. Click Open Schedule to go to the Schedule and set it up.`,
        },
        {
          screen: `schedule-dood`,
          do: `Come back to Day Out of Days once the Schedule has days with scenes.`,
          explain: `The grid shows each cast member against each shooting day. A cast member only appears when they are confirmed in the Breakdown and in a scene that is in a shooting day.`,
        },
      ],
      result: `The report shows your cast against your shooting days.`,
      cautions: [
        `If the grid is empty, check two things: scenes must be in shooting days, and the cast must be confirmed in the Breakdown.`,
      ],
      related: [`day-out-of-days-read`, `day-out-of-days-filter`],
    },
    {
      id: `day-out-of-days-read`,
      title: `Read the grid`,
      purpose: `Understand the codes, the totals and the first and last day for each actor.`,
      level: `Beginner`,
      prerequisites: [`The Schedule has days with scenes and confirmed cast.`],
      steps: [
        {
          screen: `schedule-dood`,
          do: `Find an actor in the Cast column and follow their row across the days.`,
          explain: `Each day is a column, for example Day 1 Mon 14 Jun, Day 2 Tue 15 Jun and Day 3 Wed 16 Jun. In the picture Devi and Kiran both show SWF on Day 1, and the cells for Day 2 and Day 3 are empty.`,
        },
        {
          screen: `schedule-dood`,
          do: `Read the codes with the legend under the grid.`,
          explain: `SW start work, W work, WF work finish, SWF start, work and finish on the same day, H hold (paid, not working), T travel, R rehearsal, F fitting, D drop, and a dash for not working. An asterisk marks a cell corrected by hand. The legend ends: Built from the cast confirmed in Breakdown and the scenes on each day; corrections never change the schedule.`,
        },
        {
          screen: `schedule-dood`,
          do: `Read Work, Hold, Travel, Start and Finish at the right of each row.`,
          explain: `Work, Hold and Travel count the days of each kind. Start and Finish name the first and last shooting day the actor works, for example Shoot Day 1.`,
        },
      ],
      result: `You can say for each actor which days they work and how many days that is.`,
      cautions: [
        `The grid shows what the Schedule says. To change a day, change the Schedule.`,
      ],
      related: [`day-out-of-days-code-style`, `day-out-of-days-filter`],
    },
    {
      id: `day-out-of-days-filter`,
      title: `Show one actor or a range of days`,
      purpose: `Narrow the grid to some cast members or to a stretch of the shoot.`,
      level: `Beginner`,
      prerequisites: [`The report shows a grid.`],
      steps: [
        {
          screen: `schedule-dood`,
          do: `Click the first pop-up, All cast, and choose the cast member you want.`,
          explain: `The pop-up is called Cast to show. It starts on All cast.`,
        },
        {
          screen: `schedule-dood`,
          do: `Click From the first day and choose the first shooting day to show. Click To the last day and choose the last.`,
          explain: `The pop-ups are called First day and Last day. They list your shooting days. Use them to look at one week of a long shoot.`,
        },
      ],
      result: `The grid shows only the cast and days you chose.`,
      cautions: [
        `Export saves what is shown. If you want the full report, set All cast, From the first day and To the last day first.`,
      ],
      related: [`day-out-of-days-export`],
    },
    {
      id: `day-out-of-days-code-style`,
      title: `Choose the code style`,
      purpose: `Switch between the full set of codes and a plain work and not-working view.`,
      level: `Beginner`,
      prerequisites: [`The report shows a grid.`],
      steps: [
        {
          screen: `schedule-dood`,
          do: `Click the Code style pop-up, which reads Codes: SW, W, WF, H.`,
          explain: `The pop-up is the fourth control in the filter bar.`,
        },
        {
          screen: `schedule-dood-code-style-menu`,
          do: `Choose Codes: W and - for a simple view, or keep Codes: SW, W, WF, H.`,
          explain: `The menu has two choices and a check mark shows the current one. The full style marks start, work and finish. The simple style shows W for a working day and a dash for a day off.`,
        },
      ],
      result: `The grid uses the code style you chose.`,
      cautions: [],
      related: [`day-out-of-days-read`],
    },
    {
      id: `day-out-of-days-export`,
      title: `Export the report`,
      purpose: `Save the report as a PDF, a spreadsheet or a CSV file.`,
      level: `Beginner`,
      prerequisites: [`The report shows a grid.`],
      steps: [
        {
          screen: `schedule-dood`,
          do: `Click Export at the right of the filter bar.`,
          explain: `Export saves the cast and days currently shown.`,
        },
        {
          screen: `schedule-dood-export`,
          do: `In the Export Day Out of Days sheet, choose PDF, Spreadsheet (XLSX) or CSV under Format, then click Export….`,
          explain: `The sheet names the source: Day Out of Days · the cast and days shown. The suggested file name is shown, for example Night Bus - Day Out of Days.pdf. It says: Corrections are marked with *. Export is a snapshot; the schedule is not changed. Choose a place for the file in the Mac save panel. Cancel closes the sheet.`,
        },
      ],
      result: `You have a file of the report.`,
      cautions: [
        `The file is a snapshot. Export again after you change the Schedule.`,
      ],
      related: [`day-out-of-days-filter`],
    },
  ],
  screens: {
    'schedule-dood-empty': `Day Out of Days when no schedule exists yet. It explains that the report comes from the shooting schedule and offers Open Schedule. Used in day-out-of-days-open.`,
    'schedule-dood': `The Day Out of Days report with the filter bar (All cast, From the first day, To the last day, Codes: SW, W, WF, H and Export) and a grid of two actors over three days, with Work, Hold, Travel, Start and Finish and the code legend. Used in day-out-of-days-open, day-out-of-days-read, day-out-of-days-filter, day-out-of-days-code-style and day-out-of-days-export.`,
    'schedule-dood-code-style-menu': `The Code style menu open with two choices: Codes: SW, W, WF, H, which is checked, and Codes: W and -. Used in day-out-of-days-code-style.`,
    'schedule-dood-export': `The Export Day Out of Days sheet with the formats PDF, Spreadsheet (XLSX) and CSV, the suggested file name, a note about corrections marked with an asterisk, Cancel and Export…. Used in day-out-of-days-export.`,
  },
  shortcuts: [],
  limits: [
    `The report is built from the Breakdown and the Schedule. It has no scheduling controls of its own.`,
    `Only cast confirmed in the Breakdown appear, and only on days that have their scenes.`,
    `Corrections made by hand are marked with an asterisk and never change the Schedule.`,
    `Exports are snapshots and do not update when the Schedule changes.`,
  ],
  troubleshooting: [
    {
      problem: `The page says Day Out of Days comes from your shooting schedule.`,
      fix: `No schedule exists yet. Click Open Schedule, create a shooting schedule and days, and place scenes in them.`,
    },
    {
      problem: `An actor is missing from the grid.`,
      fix: `Confirm the actor in the Breakdown for their scenes, and check that those scenes are in shooting days in the Schedule.`,
    },
    {
      problem: `A day column shows no codes at all.`,
      fix: `No scene with confirmed cast is in that day. Place scenes in the day in the Schedule.`,
    },
    {
      problem: `I want a simpler grid.`,
      fix: `Open the Code style pop-up and choose Codes: W and -.`,
    },
  ],
};
