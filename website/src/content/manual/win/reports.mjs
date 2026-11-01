export default {
  area: `Reports`,
  slug: `reports`,
  order: 21,
  part: `Shoot`,
  intro: [
    `Reports answer the question: what concise production information do I need to see or send? There are six reports, each a table built from your breakdown and schedule: a Scene Report, a Location Report, a Cast Scene Report, a Prop Report, a Schedule Report and a Breakdown Completeness report.`,
    `Reports are read-only. They are worked out from your project each time you open them, so they always show the current state, and you cannot edit a report. To change a number in a report, change it where it comes from, in the Breakdown, Locations, Cast & Crew or Schedule.`,
    `You can narrow any report with a filter box, export the report as a PDF, a CSV file or an Excel file, or save a snapshot. A saved snapshot freezes the report as it is now. It is listed under SAVED SNAPSHOTS and does not change when your production data changes, so you can keep a record of how things stood on a given day.`,
    `Reports are built from the screenplay version chosen as the Production Source in the Breakdown. Until a source is chosen, the reports are empty and tell you to choose one.`,
  ],
  layout: [
    { name: `Heading`, text: `Reports, with the line What concise production information do I need to see or send?.` },
    { name: `Save snapshot and Export`, text: `At the top right. Save snapshot keeps a frozen copy of the report you are looking at. Export opens the Export dialog for the report you are looking at. When a saved snapshot is open, Save snapshot is not shown and Export exports that snapshot.` },
    { name: `Report list`, text: `The list on the left: Scene Report, Location Report, Cast Scene Report, Prop Report, Schedule Report and Breakdown Completeness. The report you are looking at is highlighted.` },
    { name: `SAVED SNAPSHOTS`, text: `A heading under the report list, shown once you have saved a snapshot. Each snapshot shows its name, made of the report name and the draft name, with the date and time, and a bin button to delete it.` },
    { name: `Filter box`, text: `Above the table, a box with a search icon that says Filter rows (scene, location, name…). A Clear button appears beside it once you have typed something.` },
    { name: `Report table`, text: `The report's title, a line that reads Source: followed by the screenplay draft and Generated with the date and time, and then the table. Each report has its own columns.` },
    { name: `Saved snapshot bar`, text: `A blue bar at the top of the right side while a snapshot is open. It names the snapshot, says it does not change when production data changes, and has Back to live reports.` },
  ],
  workflows: [
    {
      id: `reports-scene-report`,
      title: `Read the Scene Report`,
      purpose: `See every scene with its heading, length, location, cast and shooting day.`,
      level: `Beginner`,
      prerequisites: [`A Production Source is chosen in the Breakdown.`],
      steps: [
        {
          screen: `prod-reports-scene`,
          do: `Open Production and click Reports in the row of tabs. Scene Report is the first report and opens by default.`,
          explain: `The list on the left has the six reports, with Scene Report highlighted. Above the table are the title, Scene Report, and a line such as Source: THE TOWER AT PEDDA REVU: Second Draft · Generated 31 Oct, 12:33 am. The source is the screenplay draft the report was built from. Generated is the time you opened it.`,
        },
        {
          screen: `prod-reports-scene`,
          do: `Read the columns: SC, HEADING, I/E · D/N, PAGES, LOCATION, CAST and SCHEDULED.`,
          explain: `SC is the scene number. HEADING is the scene heading. I/E · D/N shows interior or exterior and day or night, for example EXT · D. When the time of day is not set in the heading, the column says Not set, as it does for the scenes that end in CONTINUOUS or INTERCUT. PAGES is the length in eighths of a page, for example 6/8, and a whole page shows as 1. LOCATION is the location taken from the heading. CAST lists the characters confirmed for the scene, and stays empty where none are confirmed. SCHEDULED shows the shooting day, or Unscheduled when the scene is not on any day yet.`,
        },
      ],
      result: `You can see at a glance which scenes are scheduled, where, and with whom.`,
      cautions: [
        `The CAST column is empty for scenes where no cast is confirmed in the Breakdown. The scene is not missing cast, it has none confirmed yet.`,
      ],
      related: [`reports-location-report`, `reports-filter`],
    },
    {
      id: `reports-location-report`,
      title: `Read the Location Report`,
      purpose: `See each location with the scenes that use it and the days it is needed.`,
      level: `Beginner`,
      prerequisites: [`A Production Source is chosen in the Breakdown.`],
      steps: [
        {
          screen: `prod-reports-location`,
          do: `Click Location Report in the list on the left.`,
          explain: `The table lists each location once, in alphabetical order. The columns are LOCATION, STATUS, ADDRESS, SCENES and SHOOTING DAYS.`,
        },
        {
          screen: `prod-reports-location`,
          do: `Read a row, for example Rayapuram Bus Stand.`,
          explain: `STATUS says From heading when the location name was taken from the scene heading. ADDRESS is empty until you enter one. SCENES lists the scene numbers that happen there, here 1 and 9. SHOOTING DAYS lists every day the location is needed, here Shoot Day 1 and Shoot Day 4. A location whose scenes are not scheduled has an empty SHOOTING DAYS cell.`,
        },
      ],
      result: `You know which locations the film needs, how often, and on which days.`,
      cautions: [
        `A blank ADDRESS means the address has not been entered. Add it in Locations. The report shows it the next time you open it.`,
      ],
      related: [`reports-scene-report`, `reports-schedule-report`],
    },
    {
      id: `reports-cast-scene-report`,
      title: `Read the Cast Scene Report`,
      purpose: `See which scenes and shooting days each actor is needed for.`,
      level: `Beginner`,
      prerequisites: [`Cast is confirmed in the Breakdown.`],
      steps: [
        {
          screen: `prod-reports-cast-scene`,
          do: `Click Cast Scene Report.`,
          explain: `The table has one row for each actor and character. The columns are ACTOR, CHARACTER, SCENES, SCENE COUNT and SHOOTING DAYS.`,
        },
        {
          screen: `prod-reports-cast-scene`,
          do: `Read a row, for example Ananya Rao as LAKSHMI.`,
          explain: `SCENES lists the scene numbers, here 9, 10, 11 and 12. SCENE COUNT is how many, here 4. SHOOTING DAYS lists the days the actor is needed, here Shoot Day 4 · Thu 12 Nov 2026. Use it to plan which actors are needed on which days.`,
        },
      ],
      result: `You know each actor's scenes and days.`,
      cautions: [
        `Only confirmed cast is listed. If an actor is missing, confirm the character in the Breakdown.`,
      ],
      related: [`reports-prop-report`, `reports-filter`],
    },
    {
      id: `reports-prop-report`,
      title: `Read the Prop Report`,
      purpose: `See each prop with the scenes that need it.`,
      level: `Beginner`,
      prerequisites: [`Props are confirmed in the Breakdown.`],
      steps: [
        {
          screen: `prod-reports-prop`,
          do: `Click Prop Report.`,
          explain: `The table has the columns PROP, SCENES and SCENE COUNT.`,
        },
        {
          screen: `prod-reports-prop`,
          do: `Read a row, for example Soldering iron.`,
          explain: `SCENES says which scenes use the prop, here 1, and SCENE COUNT says how many, here 1. A prop used in many scenes shows every scene number.`,
        },
      ],
      result: `You have a list of props with the scenes that need them, ready for the art department.`,
      cautions: [
        `Only confirmed props are listed. Props that the Breakdown has only suggested are not.`,
      ],
      related: [`reports-cast-scene-report`, `reports-export`],
    },
    {
      id: `reports-schedule-report`,
      title: `Read the Schedule Report`,
      purpose: `See every shooting day with its scenes, locations and estimated time.`,
      level: `Beginner`,
      prerequisites: [`A shooting schedule exists.`],
      steps: [
        {
          screen: `prod-reports-schedule`,
          do: `Click Schedule Report.`,
          explain: `The columns are DAY, DATE, SCENES, LOCATIONS and EST. TIME. There is one row for each shooting day, in order.`,
        },
        {
          screen: `prod-reports-schedule`,
          do: `Read a row, for example Shoot Day 1.`,
          explain: `It shows Mon 9 Nov 2026, scenes 1, 2, 3, the three locations and 45m. EST. TIME is the total estimated time of the scenes, or Missing when no scene on the day has an estimate, as for Shoot Day 2 and Shoot Day 3.`,
        },
        {
          screen: `prod-reports-schedule`,
          do: `Look at the last rows.`,
          explain: `An off day has its own row, with an empty scenes cell and Missing time. The last row, Unscheduled, counts the scenes that are not on any day, here 24 scenes.`,
        },
      ],
      result: `You can see how the schedule is filled, and which days still lack estimates.`,
      cautions: [
        `Missing in EST. TIME means no estimate has been entered. It is not a calculation error. Add estimates in the Schedule.`,
      ],
      related: [`reports-location-report`, `reports-completeness-report`],
    },
    {
      id: `reports-completeness-report`,
      title: `Read the Breakdown Completeness report`,
      purpose: `See how much of each scene's breakdown is confirmed and how much is still waiting for review.`,
      level: `Intermediate`,
      prerequisites: [`The screenplay has been broken down, with some items suggested or confirmed.`],
      steps: [
        {
          screen: `prod-reports-breakdown-completeness`,
          do: `Click Breakdown Completeness.`,
          explain: `One row for every scene. The columns are SC, HEADING, CONFIRMED, SUGGESTED (UNREVIEWED) and STATUS.`,
        },
        {
          screen: `prod-reports-breakdown-completeness`,
          do: `Read the CONFIRMED and SUGGESTED (UNREVIEWED) numbers.`,
          explain: `CONFIRMED is the number of breakdown items you have accepted for the scene. SUGGESTED (UNREVIEWED) is the number of items the breakdown has suggested that no one has reviewed yet. The other reports use confirmed items only.`,
        },
        {
          screen: `prod-reports-breakdown-completeness`,
          do: `Read the STATUS column.`,
          explain: `In the picture, a scene with nothing confirmed or suggested is Not started. A scene with suggestions and nothing confirmed is Needs review. A scene with some confirmed and some still suggested is In progress. A scene with confirmed items and none left to review is Reviewed.`,
        },
      ],
      result: `You know which scenes still need breakdown work before the other reports are complete.`,
      cautions: [
        `If other reports look thin, check this one first. Unconfirmed items do not show up elsewhere.`,
      ],
      related: [`reports-cast-scene-report`, `reports-prop-report`],
    },
    {
      id: `reports-filter`,
      title: `Filter a report`,
      purpose: `Show only the rows that contain what you type.`,
      level: `Beginner`,
      prerequisites: [`A report is open.`],
      steps: [
        {
          screen: `prod-reports-location`,
          do: `Click the box above the table, which says Filter rows (scene, location, name…), and type a word, for example bus.`,
          explain: `The filter works on the report you are looking at. You can type up to 200 characters.`,
        },
        {
          screen: `prod-reports-filtered`,
          do: `Read the rows that remain.`,
          explain: `Only rows with the text stay. Typing bus in the Location Report leaves Farooq's Bus - Moving, Farooq's Bus - Parked and Rayapuram Bus Stand. A Clear button appears beside the box.`,
        },
        {
          screen: `prod-reports-filter-no-match`,
          do: `If nothing matches, the table says No matching data.`,
          explain: `Check the spelling or click Clear.`,
        },
        {
          screen: `prod-reports-filtered`,
          do: `Click Clear to show all rows again.`,
          explain: `The filter also applies to Save snapshot and Export. Clear it first if you want the whole report.`,
        },
      ],
      result: `The table shows only the rows you asked for.`,
      cautions: [
        `A snapshot or an export made while a filter is on contains only the filtered rows.`,
      ],
      related: [`reports-save-snapshot`, `reports-export`],
    },
    {
      id: `reports-save-snapshot`,
      title: `Save a report snapshot`,
      purpose: `Freeze a report as it is now, so you can look back at it later.`,
      level: `Beginner`,
      prerequisites: [`A report is open. Clear the filter if you want the whole report.`],
      steps: [
        {
          screen: `prod-reports-scene`,
          do: `Choose the report, then click Save snapshot at the top right.`,
          explain: `The button saves the report as it looks now, including any filter you typed.`,
        },
        {
          screen: `prod-reports-saved-snapshot`,
          do: `Look for the message at the bottom: Saved a report snapshot.`,
          explain: `The message has Undo and a close button. The snapshot is listed under SAVED SNAPSHOTS on the left, named for the report and the draft, for example Scene Report: THE TOWER AT PEDDA REVU: Second Draft, with the date and time. It opens at once.`,
        },
        {
          screen: `prod-reports-saved-snapshot`,
          do: `Read the blue bar at the top of the snapshot.`,
          explain: `It says Saved snapshot with the name. It does not change when production data changes. A snapshot made with a filter says so in the bar. Save snapshot is not shown while a snapshot is open. Back to live reports returns to the current data.`,
        },
      ],
      result: `A frozen copy of the report is kept under SAVED SNAPSHOTS.`,
      cautions: [
        `A snapshot does not update. Save a new one to capture later changes.`,
      ],
      related: [`reports-open-snapshot`, `reports-delete-snapshot`],
    },
    {
      id: `reports-open-snapshot`,
      title: `Open a snapshot and return to live reports`,
      purpose: `Look at a saved snapshot, then go back to the current reports.`,
      level: `Beginner`,
      prerequisites: [`At least one snapshot has been saved.`],
      steps: [
        {
          screen: `prod-reports-live-with-snapshots`,
          do: `Look under SAVED SNAPSHOTS on the left.`,
          explain: `Each snapshot shows its name and the date and time it was saved. The live report is showing in this picture.`,
        },
        {
          screen: `prod-reports-saved-snapshot`,
          do: `Click a snapshot.`,
          explain: `The table shows the report as it was saved. The report list has no highlighted entry because you are not looking at a live report.`,
        },
        {
          screen: `prod-reports-live-with-snapshots`,
          do: `Click Back to live reports in the blue bar, or click any report in the list.`,
          explain: `The page returns to the live report. The snapshot stays in the list.`,
        },
      ],
      result: `You can compare a past snapshot with what the project says now.`,
      cautions: [],
      related: [`reports-save-snapshot`],
    },
    {
      id: `reports-delete-snapshot`,
      title: `Delete a saved snapshot`,
      purpose: `Remove a snapshot you no longer need.`,
      level: `Beginner`,
      prerequisites: [`At least one snapshot has been saved.`],
      steps: [
        {
          screen: `prod-reports-live-with-snapshots`,
          do: `Click the bin button beside the snapshot. Its tooltip names it, for example Delete Scene Report: THE TOWER AT PEDDA REVU: Second Draft.`,
          explain: `The snapshot is removed, and a message at the bottom says Deleted a saved report. If the snapshot was open, the page returns to the live reports.`,
        },
        {
          screen: `prod-reports-saved-snapshot`,
          do: `Click Undo in the message if you deleted the wrong one.`,
          explain: `The picture shows the message that follows saving, which has the same Undo and close buttons. Use Undo before you close the message.`,
        },
      ],
      result: `The snapshot is gone from SAVED SNAPSHOTS.`,
      cautions: [
        `Deleting a snapshot never touches your production data. It removes only the saved copy.`,
      ],
      related: [`reports-save-snapshot`],
    },
    {
      id: `reports-export`,
      title: `Export a report as PDF, CSV or Excel`,
      purpose: `Save a report in a file you can print, open in a spreadsheet or send.`,
      level: `Beginner`,
      prerequisites: [`A report, or a saved snapshot, is open.`],
      steps: [
        {
          screen: `prod-reports-scene`,
          do: `Open the report you want, with the filter you want, then click Export at the top right.`,
          explain: `Export saves the report you are looking at. With a snapshot open, it saves the snapshot.`,
        },
        {
          screen: `prod-reports-export-dialog`,
          do: `In the dialog, called for example Export Scene Report, choose a format.`,
          explain: `PDF printable is selected when the dialog opens. CSV spreadsheet-friendly text gives one plain file. Excel (XLSX) one sheet per table gives a workbook. The dialog reads: Private notes excluded. Export creates a snapshot and does not change your project.`,
        },
        {
          screen: `prod-reports-export-dialog`,
          do: `Click Export… and choose a folder and file name in the Windows window.`,
          explain: `Cancel, or the Close button at the top, leaves without saving.`,
        },
      ],
      result: `A file with the report is saved where you chose.`,
      cautions: [
        `The export holds what the table shows. If a filter is on, only the filtered rows are exported.`,
        `Private notes are not included.`,
      ],
      related: [`reports-filter`, `reports-save-snapshot`],
    },
    {
      id: `reports-no-source`,
      title: `What to do when a report is empty`,
      purpose: `Know why a report shows no table and how to fix it.`,
      level: `Beginner`,
      prerequisites: [`The project has no Production Source chosen in the Breakdown.`],
      steps: [
        {
          screen: `prod-reports-empty`,
          do: `Open Reports in a project with no production source.`,
          explain: `The title shows, for example, Scene Report and a Generated line, but there is no table. The page says Choose a Production Source in Breakdown first.`,
        },
        {
          screen: `prod-reports-empty`,
          do: `Open the Breakdown, choose the screenplay version as the Production Source, then return to Reports.`,
          explain: `The Breakdown chapter explains how. The reports then fill in, and the Source line names the draft.`,
        },
      ],
      result: `The reports show data from the draft you chose.`,
      cautions: [],
      related: [`reports-scene-report`],
    },
  ],
  screens: {
    'prod-reports-scene': `Reports on the Scene Report, which is the first report. The list of six reports is on the left, Save snapshot and Export are at the top right, and the table shows SC, HEADING, I/E · D/N, PAGES, LOCATION, CAST and SCHEDULED for every scene. Used in reports-scene-report, reports-save-snapshot and reports-export.`,
    'prod-reports-location': `The Location Report: LOCATION, STATUS, ADDRESS, SCENES and SHOOTING DAYS for each location. Used in reports-location-report and reports-filter.`,
    'prod-reports-cast-scene': `The Cast Scene Report: ACTOR, CHARACTER, SCENES, SCENE COUNT and SHOOTING DAYS. Used in reports-cast-scene-report.`,
    'prod-reports-prop': `The Prop Report: PROP, SCENES and SCENE COUNT. Used in reports-prop-report.`,
    'prod-reports-schedule': `The Schedule Report: DAY, DATE, SCENES, LOCATIONS and EST. TIME for each shooting day, an off day row and an Unscheduled row. Used in reports-schedule-report.`,
    'prod-reports-breakdown-completeness': `The Breakdown Completeness report: SC, HEADING, CONFIRMED, SUGGESTED (UNREVIEWED) and STATUS for every scene, with the statuses Not started, Needs review, In progress and Reviewed. Used in reports-completeness-report.`,
    'prod-reports-filtered': `The Location Report with bus typed in the filter box. Three rows remain and a Clear button shows beside the box. Used in reports-filter.`,
    'prod-reports-filter-no-match': `A filter that matches nothing. The table is empty except for No matching data. Used in reports-filter.`,
    'prod-reports-saved-snapshot': `A saved snapshot open after Save snapshot. SAVED SNAPSHOTS lists it on the left, the blue bar says it does not change when production data changes and offers Back to live reports, and a message at the bottom says Saved a report snapshot with Undo. Save snapshot is not shown. Used in reports-save-snapshot, reports-open-snapshot and reports-delete-snapshot.`,
    'prod-reports-live-with-snapshots': `The live Scene Report with a snapshot listed under SAVED SNAPSHOTS, with its date and a bin button. Used in reports-open-snapshot and reports-delete-snapshot.`,
    'prod-reports-export-dialog': `The Export Scene Report dialog: PDF printable (selected), CSV spreadsheet-friendly text and Excel (XLSX) one sheet per table, a note that private notes are excluded, and Cancel and Export…. Used in reports-export.`,
    'prod-reports-empty': `Reports when no Production Source is chosen. There is no table, and the page says Choose a Production Source in Breakdown first. Used in reports-no-source.`,
  },
  shortcuts: [],
  limits: [
    `Reports are read-only. You cannot edit a cell in a report.`,
    `Reports are built from the chosen Production Source and show only confirmed breakdown items.`,
    `A saved snapshot is a frozen copy. It never updates.`,
    `Reports can be exported as PDF, CSV and Excel. Private notes are never included.`,
    `There are six reports. You cannot add your own or change their columns.`,
  ],
  troubleshooting: [
    {
      problem: `A report is empty and says Choose a Production Source in Breakdown first.`,
      fix: `Open the Breakdown and choose the screenplay version as the Production Source. Then return to Reports.`,
    },
    {
      problem: `The CAST column, or the Cast Scene Report, is empty.`,
      fix: `Reports show confirmed cast only. Confirm the characters for each scene in the Breakdown. The Breakdown Completeness report shows which scenes still need review.`,
    },
    {
      problem: `The table says No matching data.`,
      fix: `The filter box has text that no row contains. Check the spelling or click Clear.`,
    },
    {
      problem: `My export is missing rows.`,
      fix: `A filter was on when you exported. Click Clear and export again.`,
    },
    {
      problem: `A snapshot shows old numbers.`,
      fix: `Snapshots never change. Click Back to live reports to see the current data, and save a new snapshot if you need one.`,
    },
    {
      problem: `EST. TIME says Missing.`,
      fix: `No scene on that day has a time estimate. Add estimates in the Schedule.`,
    },
  ],
};
