export default {
  area: `Reports`,
  slug: `reports`,
  order: 16,
  part: `Shoot`,
  intro: [
    `Reports answer the question: What concise production information do I need to see or send? There are six, each a table built from your breakdown and schedule: the Scene Report, Location Report, Cast Scene Report, Prop Report, Schedule Report and Breakdown Completeness.`,
    `Reports is a page of the Production workspace. Click Production in the sidebar, then choose Reports in the picker at the top of the page, next to the word Production. The list of six reports sits at the top of the page and the table of the report you pick is below it. In a wider window the list sits at the left and the table beside it.`,
    `Reports are live. They are rebuilt from your project each time you open them, and the line above each table says what the report was built from and when it was generated. To keep a copy that will not change, save a snapshot. To send one outside Straft, export it as a PDF, CSV or spreadsheet.`,
    `Reports only read your project. Filtering, saving a snapshot and exporting never change your breakdown, schedule or anything else.`,
  ],
  layout: [
    { name: `Page header`, text: `The title Reports and the line What concise production information do I need to see or send? At the right are Save snapshot and Export.` },
    { name: `Report list`, text: `Scene Report, Location Report, Cast Scene Report, Prop Report, Schedule Report and Breakdown Completeness. The one you are viewing is highlighted.` },
    { name: `Filter box`, text: `A box labelled Filter rows (scene, location, name...) above the table. A Clear button appears once you type.` },
    { name: `Report table`, text: `The name of the report, a Source line that says what it was built from and when (for example Source: ... · Generated 28 Oct at 08:29), then the table. A wide table scrolls sideways.` },
    { name: `Saved snapshots`, text: `A Saved snapshots list under the report list, shown once you have saved one. Each entry has its name, the time it was saved and a Delete button.` },
    { name: `Snapshot banner`, text: `While you view a snapshot, a blue banner says it does not change when production data changes, and has Back to live reports.` },
  ],
  workflows: [
    {
      id: `reports-open-and-switch`,
      title: `Open Reports and read each report`,
      purpose: `Find the Reports page and know what each of the six reports shows.`,
      level: `Beginner`,
      prerequisites: [`A project with a breakdown and, for the scheduling columns, a schedule.`],
      steps: [
        {
          screen: `schedule-reports-scene`,
          do: `Click Production in the sidebar, then choose Reports in the picker at the top of the page.`,
          explain: `The Scene Report opens first. Its columns are Sc (the scene number), Heading, I/E · D/N (inside or outside, day or night, for example INT · N), Pages (in eighths, for example 6/8), Location, Cast and Scheduled. A scene not yet placed in the schedule reads Unscheduled.`,
        },
        {
          screen: `schedule-reports-location`,
          do: `Click Location Report in the list.`,
          explain: `It has one row per location, with Location, Status, Address, Scenes and Shooting days. Status shows where the location record stands, for example From heading or Idea. Scenes lists the scene numbers shot there, for example 1, 3, 4, 6, and Shooting days lists the days.`,
        },
        {
          screen: `schedule-reports-cast-scene`,
          do: `Click Cast Scene Report.`,
          explain: `One row per character, with Actor, Character, Scenes, Scene count and Shooting days. An actor column that reads Not cast yet means nobody is cast in that role yet.`,
        },
        {
          screen: `schedule-reports-prop`,
          do: `Click Prop Report.`,
          explain: `One row per prop with Prop, Scenes and Scene count, for example Keys or Umbrella. Props come from the breakdown.`,
        },
        {
          screen: `schedule-reports-schedule`,
          do: `Click Schedule Report.`,
          explain: `One row per shooting day with Day, Date, Scenes, Locations and Est. time, then a row called Unscheduled that says how many scenes are not yet on any day, for example 7 scenes. Est. time reads Missing when a day has no time estimate.`,
        },
        {
          screen: `schedule-reports-breakdown-completeness`,
          do: `Click Breakdown Completeness.`,
          explain: `One row per scene with Sc, Heading, Confirmed, Suggested (unreviewed) and Status. Confirmed counts the breakdown elements you confirmed. Suggested (unreviewed) counts elements suggested but not yet reviewed. Status reads Reviewed or Not started. Use this report to see which scenes still need a breakdown pass.`,
        },
      ],
      result: `You can move between the six reports and read what each one shows.`,
      cautions: [
        `A report looks empty when the breakdown or schedule behind it is empty. Tag elements and place scenes first.`,
        `Check the Source line to see when the report was generated.`,
      ],
      related: [`reports-filter-rows`, `reports-export`],
    },
    {
      id: `reports-filter-rows`,
      title: `Filter the rows of a report`,
      purpose: `Narrow a report to the scenes, locations or names that match what you type.`,
      level: `Beginner`,
      prerequisites: [`A report is open.`],
      steps: [
        {
          screen: `schedule-reports-scene`,
          do: `Click the Filter rows box above the table.`,
          explain: `The box says Filter rows (scene, location, name...). It filters the report you are viewing.`,
        },
        {
          screen: `schedule-reports-filtered`,
          do: `Type part of a scene number, location or name, for example 1.`,
          explain: `Only matching rows remain. In the picture the Scene Report shows scenes 1, 2, 3 and 10, because each of those rows contains the typed text somewhere. A Clear button appears beside the box.`,
        },
        {
          screen: `schedule-reports-scene`,
          do: `Click Clear to show every row again.`,
          explain: `The filter belongs to the report on screen. It does not change the report or what a snapshot or export contains.`,
        },
      ],
      result: `The table shows just the rows you are interested in.`,
      cautions: [
        `The filter matches any text in a row, not only the column you have in mind.`,
      ],
      related: [`reports-open-and-switch`],
    },
    {
      id: `reports-save-snapshot`,
      title: `Save a snapshot of a report`,
      purpose: `Keep a copy of a report as it is now, so it does not change when your project does.`,
      level: `Beginner`,
      prerequisites: [`A report is open.`],
      steps: [
        {
          screen: `schedule-reports-scene`,
          do: `Pick the report you want to keep, then click Save snapshot at the top right.`,
          explain: `Live reports change as your breakdown and schedule change. A snapshot is a copy frozen at this moment.`,
        },
        {
          screen: `schedule-reports-snapshot`,
          do: `Look at the page after saving.`,
          explain: `A message says Saved a report snapshot, with Undo. A Saved snapshots list appears under the report list. The new entry is named after the report and its source, for example Scene Report: LOST PROPERTY: Example-Short (imported), with the time it was saved. A blue banner says the snapshot does not change when production data changes. Save snapshot is gone from the top right while you view a snapshot, and Export is still there.`,
        },
        {
          screen: `schedule-reports-snapshot`,
          do: `Click Back to live reports in the banner.`,
          explain: `You return to the live report. The snapshot stays in the Saved snapshots list.`,
        },
      ],
      result: `A frozen copy of the report is in Saved snapshots.`,
      cautions: [
        `Click Undo in the message right after saving if you saved by mistake.`,
        `A snapshot saves one report. Save one for each report you want to keep.`,
      ],
      related: [`reports-manage-snapshots`, `reports-export`],
    },
    {
      id: `reports-manage-snapshots`,
      title: `Open and delete saved snapshots`,
      purpose: `Look at a snapshot again, or remove one you no longer need.`,
      level: `Beginner`,
      prerequisites: [`At least one snapshot is saved.`],
      steps: [
        {
          screen: `schedule-reports-snapshot`,
          do: `Click an entry under Saved snapshots to open it.`,
          explain: `The entry is highlighted and the table shows the report as it was when you saved it. The Source line still shows the time it was generated. Export works on the snapshot too.`,
        },
        {
          screen: `schedule-reports-snapshot`,
          do: `To remove the snapshot, click Delete at the right of its entry.`,
          explain: `Delete removes the saved copy from the list. Your live reports and your project are not affected.`,
        },
        {
          screen: `schedule-reports-snapshot`,
          do: `Click Back to live reports to return to the current data.`,
          explain: `The banner is at the top of the table while a snapshot is open.`,
        },
      ],
      result: `You can look at a saved copy, or the snapshot is gone from the list.`,
      cautions: [
        `Deleting a snapshot removes the saved copy. Export it first if you may want it again.`,
      ],
      related: [`reports-save-snapshot`],
    },
    {
      id: `reports-export`,
      title: `Export a report as a PDF, CSV or spreadsheet`,
      purpose: `Make a file of a report to send or open in another program.`,
      level: `Beginner`,
      prerequisites: [`A report, or a saved snapshot, is open.`],
      steps: [
        {
          screen: `schedule-reports-scene`,
          do: `Pick the report, then click Export at the top right.`,
          explain: `The Export button is next to Save snapshot.`,
        },
        {
          screen: `schedule-reports-export`,
          do: `In the Export sheet, choose a Format.`,
          explain: `The sheet is called Export Scene Report for the Scene Report, and names the report you picked as the Source. Format offers PDF, CSV and Spreadsheet (XLSX). PDF is for reading or sending. CSV and XLSX open in a spreadsheet program.`,
        },
        {
          screen: `schedule-reports-export`,
          do: `Read the suggested file name, then click Export… and choose where to save the file.`,
          explain: `The Suggested file name looks like Night Bus - Scene Report - 2026-10-28.pdf and changes with the format. A note says: Private notes excluded. Export creates a snapshot and does not change your project. Cancel closes the sheet without making a file.`,
        },
      ],
      result: `A file of the report is saved where you chose.`,
      cautions: [
        `The file is a snapshot of the report at the moment you export. It does not update later.`,
        `The export contains the whole report, whatever you typed in the filter box.`,
      ],
      related: [`reports-save-snapshot`, `reports-open-and-switch`],
    },
  ],
  screens: {
    'schedule-reports-scene': `The Reports page with the Scene Report selected. The six reports are listed, then the filter box, the Source line and a table with Sc, Heading, I/E · D/N, Pages, Location, Cast and Scheduled. Save snapshot and Export are at the top right. Used in reports-open-and-switch, reports-filter-rows, reports-save-snapshot and reports-export.`,
    'schedule-reports-location': `The Location Report: Location, Status, Address, Scenes and Shooting days for each location. Used in reports-open-and-switch.`,
    'schedule-reports-cast-scene': `The Cast Scene Report: Actor, Character, Scenes, Scene count and Shooting days. The actors read Not cast yet. Used in reports-open-and-switch.`,
    'schedule-reports-prop': `The Prop Report: Prop, Scenes and Scene count for each prop. Used in reports-open-and-switch.`,
    'schedule-reports-schedule': `The Schedule Report: Day, Date, Scenes, Locations and Est. time for each shooting day, and an Unscheduled row that counts the scenes with no day. Used in reports-open-and-switch.`,
    'schedule-reports-breakdown-completeness': `The Breakdown Completeness report: Sc, Heading, Confirmed, Suggested (unreviewed) and Status for each scene. Used in reports-open-and-switch.`,
    'schedule-reports-filtered': `The Scene Report with 1 typed in the filter box. Only the rows containing it remain and a Clear button shows beside the box. Used in reports-filter-rows.`,
    'schedule-reports-export': `The Export Scene Report sheet. Format offers PDF, CSV and Spreadsheet (XLSX). It shows the suggested file name and a note that private notes are excluded, with Cancel and Export…. Used in reports-export.`,
    'schedule-reports-snapshot': `A saved snapshot open. The Saved snapshots list shows its entry with a Delete button, and a blue banner says it does not change when production data changes, with Back to live reports. A message says Saved a report snapshot. Used in reports-save-snapshot and reports-manage-snapshots.`,
  },
  shortcuts: [],
  limits: [
    `Reports only read your project. They never change the breakdown, schedule or anything else.`,
    `Live reports are rebuilt from your project. Only a saved snapshot or an exported file stays the same.`,
    `The reports are fixed: the six in the list. You cannot add your own.`,
    `Export gives a PDF, a CSV file or a spreadsheet (XLSX). Private notes are never included.`,
  ],
  troubleshooting: [
    {
      problem: `A report looks empty or short.`,
      fix: `Reports come from the breakdown and the schedule. Tag the elements in Breakdown and place scenes on shooting days, then open the report again.`,
    },
    {
      problem: `Est. time reads Missing in the Schedule Report.`,
      fix: `The day has no time estimate yet.`,
    },
    {
      problem: `The numbers changed since I last looked.`,
      fix: `Live reports follow your project. Save a snapshot when you want numbers that stay put.`,
    },
    {
      problem: `Save snapshot is not at the top right.`,
      fix: `You are viewing a snapshot. Click Back to live reports and the button returns.`,
    },
    {
      problem: `The filter hides rows I expected.`,
      fix: `Click Clear beside the filter box to show every row.`,
    },
    {
      problem: `I need the report in a spreadsheet.`,
      fix: `Click Export and choose CSV or Spreadsheet (XLSX) as the Format.`,
    },
  ],
};
