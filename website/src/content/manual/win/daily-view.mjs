export default {
  area: `Daily View`,
  slug: `daily-view`,
  order: 18,
  part: `Shoot`,
  intro: [
    `Daily View answers one question: what do I need to know for this shooting day? It gathers the scenes, locations, cast, call sheet, day notes, important breakdown items and scene order for one day onto a single page, so you can read the whole day without opening five other tabs.`,
    `The page is built from your schedule. Nothing on it is typed in here. If a scene, a location or a note looks wrong, you fix it where it lives (the Breakdown, Locations, Cast & Crew or Schedule) and Daily View shows the change the next time you look. Many items are buttons that take you straight to that place.`,
    `You choose the day with the Shooting day box at the top right. A yellow bar under the heading lists anything to check on that day, such as scenes with no location or a call sheet that is out of date. The Export button saves the day as a document you can print or send.`,
    `Daily View is the tab named Daily View inside Production, between Day Out of Days and Sides. It needs a shooting schedule with at least one shooting day. Before that, the page tells you to open the Schedule.`,
  ],
  layout: [
    { name: `Heading`, text: `The name of the day, for example Shoot Day 1, with its date and the line What do I need to know for this day?. When the day is today, the heading starts with Today:. An off day carries a small Off day label beside the name.` },
    { name: `Shooting day and Export`, text: `At the top right. The Shooting day box lists every day in the schedule, including off days, each with its date. Export opens the Export Daily View dialog.` },
    { name: `Things to check bar`, text: `A yellow bar under the heading, shown only when something needs attention. One issue is shown as a sentence. Several are introduced by a count, for example 3 things to check on this day:, followed by each message. Open day at the right of the bar opens that day in the Schedule.` },
    { name: `SCENES`, text: `A numbered button for each scene on the day, and a line such as 3 scenes · 45m estimated · 3 without estimate. Click a number to open that scene in the Breakdown. A scene with no number is labelled Not numbered.` },
    { name: `LOCATION`, text: `Each location the day needs, with its status and address. An address that has not been entered reads Address missing. A location that is linked to your Locations list is a button that opens it. When the day has no location at all, the card shows a red Missing label.` },
    { name: `CAST`, text: `The actors confirmed for these scenes, separated by dots, for example Ananya Rao · Suresh Babu, with a line below for call times (Call times not entered yet until you enter them). The names are a button that opens Cast & Crew. With no confirmed cast, the card says No cast confirmed for these scenes.` },
    { name: `CALL SHEET`, text: `The state of the call sheet for the day: a status label (such as Needs refresh), one sentence explaining it, and Open Call Sheet. When there is no call sheet yet, the card says No call sheet yet for this day. and offers Create Call Sheet. On an off day it says Off days have no call sheet.` },
    { name: `IMPORTANT NOTES`, text: `The notes you wrote for the day, and small labels for the day's markers such as DINNER (45m) or LUNCH at 13:00 (45m). With nothing written, it says No day notes.` },
    { name: `IMPORTANT BREAKDOWN ITEMS`, text: `The confirmed props, wardrobe and other breakdown items for the day's scenes, shown as small labels. With none, it says No confirmed props, wardrobe or other items for these scenes.` },
    { name: `SCENE ORDER`, text: `The scenes in shooting order as a numbered list, each with its heading and, when there is an estimate, its time in brackets. With nothing scheduled, it says Nothing scheduled yet.` },
  ],
  workflows: [
    {
      id: `daily-view-read-day`,
      title: `Read everything about a shooting day`,
      purpose: `Open Daily View and read a day's scenes, locations, cast, call sheet, notes and scene order in one place.`,
      level: `Beginner`,
      prerequisites: [`The project has a shooting schedule with at least one shooting day.`],
      steps: [
        {
          screen: `prod-daily-day-1`,
          do: `Open Production in the left rail and click Daily View in the row of tabs.`,
          explain: `The page opens on a shooting day. The heading shows the day and its date, and under it the question What do I need to know for this day?. The page is read-only: it shows what the schedule says and changes nothing.`,
        },
        {
          screen: `prod-daily-day-1`,
          do: `Read the SCENES card, then the LOCATION and CAST cards across the top.`,
          explain: `SCENES lists the scene numbers with a total and the time estimated. In the picture, Shoot Day 1 has 3 scenes, 45m estimated and 3 without estimate, which means three scenes have no time estimate of their own yet. LOCATION lists three places, each marked Address missing. CAST says No cast confirmed for these scenes. because no cast is confirmed in the Breakdown for these scenes.`,
        },
        {
          screen: `prod-daily-day-1`,
          do: `Read the CALL SHEET and IMPORTANT NOTES cards in the middle row.`,
          explain: `The call sheet card says Needs refresh and Available. The schedule changed after it was created. The notes card shows the day note, Night exteriors, and a marker label, DINNER (45m).`,
        },
        {
          screen: `prod-daily-day-1`,
          do: `Read IMPORTANT BREAKDOWN ITEMS and SCENE ORDER at the bottom.`,
          explain: `The items are labels such as Soldering iron and Night bus. Scene order lists the scenes in the order they will be shot, here 1, 2 and 3 with their headings.`,
        },
        {
          screen: `prod-daily-day-4`,
          do: `Choose Shoot Day 4 in the Shooting day box to compare with a busier day.`,
          explain: `This day has four scenes and 6h 30m estimated. The CAST card lists Ananya Rao · Suresh Babu with the line Call times not entered yet. Scene order shows each scene's time in brackets, for example (1h 30m). IMPORTANT BREAKDOWN ITEMS says No confirmed props, wardrobe or other items for these scenes.`,
        },
      ],
      result: `You know the scenes, places, people, call sheet and notes for the day, and what is still missing.`,
      cautions: [
        `Daily View never edits your project. To change a scene, location, cast member or note, go to the place that owns it.`,
        `A card that says Missing, Address missing or Call times not entered yet is telling you what is not filled in elsewhere. It is not an error on this page.`,
      ],
      related: [`daily-view-switch-day`, `daily-view-check-warnings`, `daily-view-jump-to-source`],
    },
    {
      id: `daily-view-switch-day`,
      title: `Switch to another shooting day`,
      purpose: `Look at a different day, including a day with no call sheet and an off day.`,
      level: `Beginner`,
      prerequisites: [`The schedule has more than one day.`],
      steps: [
        {
          screen: `prod-daily-day-1`,
          do: `Click the Shooting day box at the top right.`,
          explain: `The box lists every day with its date, for example Shoot Day 1 · Mon 9 Nov 2026. Off days are listed too.`,
        },
        {
          screen: `prod-daily-no-call-sheet`,
          do: `Choose Shoot Day 3 · Wed 11 Nov 2026.`,
          explain: `The page changes at once to Shoot Day 3. This day has two scenes, 7 and 8, with no estimate for either, two locations and no cast. Its call sheet card says No call sheet yet for this day. and shows Create Call Sheet.`,
        },
        {
          screen: `prod-daily-off-day`,
          do: `Choose Off Day · Fri 13 Nov 2026.`,
          explain: `An off day shows the heading Off Day with a small Off day label. SCENES says No scenes scheduled. and LOCATION shows a red Missing label because nothing is planned. CALL SHEET says Off days have no call sheet. IMPORTANT NOTES carries the day note, for example Travel and rest day, and SCENE ORDER says Nothing scheduled yet.`,
        },
      ],
      result: `Daily View shows the day you chose. The Export button now exports that day.`,
      cautions: [
        `The day you pick is the day Export saves. Check the heading before you export.`,
      ],
      related: [`daily-view-read-day`, `daily-view-off-day`, `daily-view-export`],
    },
    {
      id: `daily-view-check-warnings`,
      title: `Read the things to check bar and fix what it names`,
      purpose: `Understand the yellow bar at the top of a day and go to the Schedule to deal with it.`,
      level: `Intermediate`,
      prerequisites: [`A day has something to check, so the yellow bar shows under the heading.`],
      steps: [
        {
          screen: `prod-daily-no-call-sheet`,
          do: `Look under the heading of Shoot Day 3.`,
          explain: `With one thing to check, the bar is a single sentence: Scene 7, Scene 8 on Shoot Day 3 are missing location information. The bar has an Open day button at its right.`,
        },
        {
          screen: `prod-daily-day-1`,
          do: `Switch to Shoot Day 1 and read the bar.`,
          explain: `With more than one issue, the bar starts with a count: 2 things to check on this day:. It then lists each issue. Here the scenes are missing location information, and the call sheet was prepared before this day changed, so you are told to refresh it to review the update.`,
        },
        {
          screen: `prod-daily-day-4`,
          do: `Switch to Shoot Day 4 and read the bar.`,
          explain: `It says 3 things to check on this day:. Two are Potential actor conflict messages, for example Ananya Rao (LAKSHMI) is needed at 4 locations on Shoot Day 4, which means the same actor is needed in four different places on one day. The third lists scenes 9 to 12 as missing location information.`,
        },
        {
          screen: `prod-daily-day-4`,
          do: `Click Open day at the right of the bar.`,
          explain: `The Schedule opens on that day, where you can move scenes, add location details or decide what to do about a conflict. When an issue has been settled in the Schedule, it stops appearing in the bar.`,
        },
      ],
      result: `You know what is wrong or unfinished on the day, and you are in the Schedule to fix it.`,
      cautions: [
        `The bar only reports. It does not change the schedule. Fix the cause in the Schedule, then come back to Daily View.`,
        `A day with nothing to check shows no bar at all.`,
      ],
      related: [`daily-view-read-day`, `daily-view-call-sheet`],
    },
    {
      id: `daily-view-call-sheet`,
      title: `Open or create the call sheet for a day`,
      purpose: `Use the CALL SHEET card to open an existing call sheet or start one for the day.`,
      level: `Beginner`,
      prerequisites: [`A shooting day is selected. The day is not an off day.`],
      steps: [
        {
          screen: `prod-daily-day-1`,
          do: `Find the CALL SHEET card. On a day that already has a call sheet, click Open Call Sheet.`,
          explain: `The card shows a status label and one sentence. Needs refresh with Available. The schedule changed after it was created. means the call sheet exists but the day has changed since. Open Call Sheet takes you to it so you can review the update. A call sheet that matches the schedule says it is up to date, and a finalized one says it is Finalized and matches the schedule.`,
        },
        {
          screen: `prod-daily-no-call-sheet`,
          do: `On a day with no call sheet, click Create Call Sheet.`,
          explain: `The card says No call sheet yet for this day. Create Call Sheet starts a call sheet for this day in the Call Sheets area. The Call sheets chapter explains the dialog that follows.`,
        },
        {
          screen: `prod-daily-off-day`,
          do: `On an off day, read the card.`,
          explain: `An off day has no call sheet, so there is no button, only the line Off days have no call sheet.`,
        },
      ],
      result: `You are in the call sheet for the day, or you have started one.`,
      cautions: [
        `Needs refresh does not mean the call sheet is lost. It means the schedule changed after the call sheet was made. Open it and review.`,
      ],
      related: [`daily-view-check-warnings`, `daily-view-off-day`],
    },
    {
      id: `daily-view-jump-to-source`,
      title: `Jump from Daily View to a scene, location or cast`,
      purpose: `Use the buttons on the cards to open the place where an item is edited.`,
      level: `Intermediate`,
      prerequisites: [`A shooting day with scenes is selected.`],
      steps: [
        {
          screen: `prod-daily-day-4`,
          do: `Click a scene number in the SCENES card, for example 10.`,
          explain: `The Breakdown opens on that scene. Hover over a number to see the scene heading.`,
        },
        {
          screen: `prod-daily-day-4`,
          do: `Click a location name in the LOCATION card, for example Rayapuram Bus Stand.`,
          explain: `The Locations tab opens on that location, where you can add the missing address. A location that is not linked to your Locations list is shown as plain bold text and is not a button.`,
        },
        {
          screen: `prod-daily-day-4`,
          do: `Click the cast names in the CAST card.`,
          explain: `The Cast & Crew tab opens, where the cast and crew of the project are kept. Call times, once entered, appear on this card in place of Call times not entered yet.`,
        },
        {
          screen: `prod-daily-day-4`,
          do: `Use the Back arrow at the top of the window to return to Daily View.`,
          explain: `Back and Forward are the arrows beside the search box at the top of the window.`,
        },
      ],
      result: `You can open the source of any item on the day, fix it, and come back.`,
      cautions: [
        `Scene order and the notes are not buttons. Change the order in the Schedule and the notes on the day in the Schedule.`,
      ],
      related: [`daily-view-read-day`, `daily-view-check-warnings`],
    },
    {
      id: `daily-view-off-day`,
      title: `Understand an off day`,
      purpose: `Read what Daily View shows for a day with no shooting.`,
      level: `Beginner`,
      prerequisites: [`The schedule has an off day.`],
      steps: [
        {
          screen: `prod-daily-off-day`,
          do: `Choose the off day in the Shooting day box.`,
          explain: `The heading is Off Day, with a small Off day label, and the date below it. Export is still available.`,
        },
        {
          screen: `prod-daily-off-day`,
          do: `Read the cards.`,
          explain: `Scenes, cast, call sheet, breakdown items and scene order are all empty on purpose. LOCATION shows a red Missing label, which is normal for a day with no shooting. IMPORTANT NOTES can hold a note such as Travel and rest day.`,
        },
      ],
      result: `You can tell an empty off day from a working day that has gaps.`,
      cautions: [
        `The Missing label on an off day is not a problem to fix.`,
      ],
      related: [`daily-view-switch-day`],
    },
    {
      id: `daily-view-export`,
      title: `Export a day as a document`,
      purpose: `Save one day's Daily View as a printable PDF or a plain text file.`,
      level: `Beginner`,
      prerequisites: [`A shooting day is selected in Daily View.`],
      steps: [
        {
          screen: `prod-daily-day-1`,
          do: `Choose the day you want, then click Export at the top right.`,
          explain: `The button always exports the day shown in the heading.`,
        },
        {
          screen: `prod-daily-export-dialog`,
          do: `In the Export Daily View dialog, read the Source line and choose a FORMAT.`,
          explain: `Source names what will be exported, for example Shoot Day 1 - Daily View. FORMAT offers PDF, printable, which is selected when the dialog opens, and Plain text. A line below shows the suggested file name, for example Shoot Day 1 - Daily View.pdf.`,
        },
        {
          screen: `prod-daily-export-dialog`,
          do: `Read the note at the bottom, then click Export….`,
          explain: `The note says Private notes excluded. The document shows what the Daily View shows; the schedule is not changed. Export… opens the Windows window where you choose the folder and file name. Cancel, or the Close button at the top, leaves without saving.`,
        },
      ],
      result: `A file with the day's Daily View is saved where you chose.`,
      cautions: [
        `Private notes are left out of the export.`,
        `The document shows what the page shows. If the page says Address missing, so will the document.`,
      ],
      related: [`daily-view-switch-day`],
    },
    {
      id: `daily-view-no-schedule`,
      title: `What to do when Daily View is empty`,
      purpose: `Know why Daily View asks you to select or create a shooting day, and how to get started.`,
      level: `Beginner`,
      prerequisites: [`The project has no shooting schedule yet.`],
      steps: [
        {
          screen: `prod-daily-empty`,
          do: `Open Daily View in a project with no schedule.`,
          explain: `The page shows a calendar picture and says Select or create a shooting day. Under it: The daily view shows everything you need to know for one shooting day, derived from the schedule.`,
        },
        {
          screen: `prod-daily-empty`,
          do: `Click Open Schedule.`,
          explain: `The Schedule tab opens. Create the shooting schedule and add at least one shooting day there. Then return to Daily View.`,
        },
      ],
      result: `You are in the Schedule, ready to add the day that Daily View will show.`,
      cautions: [],
      related: [`daily-view-read-day`],
    },
  ],
  screens: {
    'prod-daily-day-1': `Daily View for Shoot Day 1 (Mon 9 Nov 2026). The yellow bar lists 2 things to check. SCENES shows 1, 2 and 3 (45m estimated), LOCATION shows three places with Address missing, CAST says No cast confirmed for these scenes., CALL SHEET shows Needs refresh with Open Call Sheet, IMPORTANT NOTES shows Night exteriors and DINNER (45m), then breakdown items and scene order. Used in daily-view-read-day, daily-view-switch-day, daily-view-check-warnings, daily-view-call-sheet and daily-view-export.`,
    'prod-daily-no-call-sheet': `Daily View for Shoot Day 3 (Wed 11 Nov 2026). One issue is shown as a sentence in the bar, scenes 7 and 8 have no estimate, there is no confirmed cast, and the CALL SHEET card says No call sheet yet for this day. with Create Call Sheet. Used in daily-view-switch-day, daily-view-check-warnings and daily-view-call-sheet.`,
    'prod-daily-day-4': `Daily View for Shoot Day 4 (Thu 12 Nov 2026), the busiest day. The bar says 3 things to check, two of them actor conflicts. Four scenes (6h 30m estimated), four locations, the cast Ananya Rao and Suresh Babu, a Create Call Sheet button, two marker labels and a timed scene order. Used in daily-view-read-day, daily-view-check-warnings and daily-view-jump-to-source.`,
    'prod-daily-off-day': `Daily View for Off Day (Fri 13 Nov 2026). The heading carries an Off day label. No scenes, a red Missing label under LOCATION, Off days have no call sheet, the note Travel and rest day and Nothing scheduled yet. Used in daily-view-switch-day, daily-view-call-sheet and daily-view-off-day.`,
    'prod-daily-export-dialog': `The Export Daily View dialog over Shoot Day 1. Source, FORMAT with PDF printable (selected) and Plain text, the suggested file name, the note that private notes are excluded, and Cancel and Export…. Used in daily-view-export.`,
    'prod-daily-empty': `Daily View in a project with no shooting schedule. It says Select or create a shooting day. and offers Open Schedule. Used in daily-view-no-schedule.`,
  },
  shortcuts: [],
  limits: [
    `Daily View is read-only. You cannot edit scenes, locations, cast, notes or the scene order from it.`,
    `It shows one day at a time. Use the Schedule to see all days together.`,
    `Export saves PDF or plain text on Windows. It does not save CSV or Excel.`,
    `Private notes are never included in the export.`,
    `Cast shown comes from confirmed breakdown items. Cast that is only suggested and not confirmed is not shown.`,
  ],
  troubleshooting: [
    {
      problem: `Daily View says Select or create a shooting day.`,
      fix: `The project has no shooting schedule or no shooting day yet. Click Open Schedule and create one.`,
    },
    {
      problem: `The CAST card says No cast confirmed for these scenes.`,
      fix: `Cast comes from the confirmed items in the Breakdown. Confirm the characters for each scene there.`,
    },
    {
      problem: `The call sheet says Needs refresh.`,
      fix: `The schedule changed after the call sheet was made. Click Open Call Sheet and review the update.`,
    },
    {
      problem: `A location shows Address missing.`,
      fix: `Click the location name to open it in Locations, and enter the address there.`,
    },
    {
      problem: `The yellow bar will not go away.`,
      fix: `Click Open day and deal with each issue in the Schedule. The bar lists only issues that are still open.`,
    },
    {
      problem: `I cannot find a day in the Shooting day box.`,
      fix: `The box lists the days in the Schedule. Add the day in the Schedule and it appears here.`,
    },
  ],
};
