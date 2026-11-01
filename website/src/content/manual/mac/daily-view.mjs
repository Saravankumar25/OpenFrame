export default {
  area: `Daily View`,
  slug: `daily-view`,
  order: 13,
  part: `Shoot`,
  intro: [
    `The Daily View answers one question for one shooting day: What do I need to know for this day? It gathers the scenes, the locations, the cast, the call sheet status, the day notes and the important breakdown items on a single page.`,
    `It is derived from the Schedule. You do not type anything into it. If something is wrong or missing, you fix it in the Schedule, the Breakdown or the Locations, and the Daily View shows the change.`,
    `Open it from Production in the sidebar, using the pop-up at the top of the page: choose Daily View. If there is no shooting day yet, the page asks you to create one in the Schedule.`,
  ],
  layout: [
    { name: `Title and day picker`, text: `The day name and date on the left, for example Shoot Day 1 and Mon 14 Jun 2027. On the right a pop-up that lists the shooting days, and Export.` },
    { name: `Check banner`, text: `A yellow bar that appears when something on the day needs a look, for example 2 things to check on this day. Its Open day button opens the day in the Schedule.` },
    { name: `Scenes`, text: `The scene numbers on this day as small tags, and a line such as 2 scenes · 2 without estimate.` },
    { name: `Location`, text: `Each location used that day, with its type and whether the address is known, for example Idea · Address missing.` },
    { name: `Cast`, text: `The cast members needed that day, and a note about call times.` },
    { name: `Call sheet`, text: `Whether a call sheet exists for this day, with a Create Call Sheet button when it does not.` },
    { name: `Important notes`, text: `The notes written for the day, or No day notes.` },
    { name: `Important breakdown items`, text: `Tags for the props, wardrobe and other items confirmed in the Breakdown for the day's scenes.` },
    { name: `Scene order`, text: `The numbered list of scenes in shooting order, with their headings.` },
  ],
  workflows: [
    {
      id: `daily-view-open`,
      title: `Open the Daily View`,
      purpose: `Reach the Daily View and see what it needs before it shows anything.`,
      level: `Beginner`,
      prerequisites: [`The project has a screenplay.`],
      steps: [
        {
          screen: `schedule-daily-empty`,
          do: `Click Production in the sidebar, then choose Daily View in the pop-up at the top of the page.`,
          explain: `With no shooting day the page says: Select or create a shooting day. It explains: The daily view shows everything you need to know for one shooting day, derived from the schedule. Click Open Schedule to create days there.`,
        },
        {
          screen: `schedule-daily`,
          do: `Come back once the Schedule has a shooting day.`,
          explain: `The page opens on a shooting day, in the picture Shoot Day 1, with its date and the question: What do I need to know for this day?`,
        },
      ],
      result: `The Daily View shows a shooting day.`,
      cautions: [],
      related: [`daily-view-read`, `daily-view-switch-day`],
    },
    {
      id: `daily-view-read`,
      title: `Read one shooting day`,
      purpose: `Find the scenes, locations, cast, notes and items for a day, and spot what is missing.`,
      level: `Beginner`,
      prerequisites: [`A shooting day with scenes exists.`],
      steps: [
        {
          screen: `schedule-daily`,
          do: `Read the yellow banner under the title.`,
          explain: `It lists the things to check, for example Potential actor conflict: Kiran is needed at two locations, and scenes missing location information. Click Open day to go to the day's page in the Schedule and deal with them.`,
        },
        {
          screen: `schedule-daily`,
          do: `Read the Scenes, Location and Cast cards.`,
          explain: `Scenes shows the scene numbers and a count, with how many scenes have no time estimate. Location lists each place and says Address missing when no address is known. Cast lists the actors and says Call times not entered yet until call times exist.`,
        },
        {
          screen: `schedule-daily`,
          do: `Read the Call sheet and Important notes cards.`,
          explain: `Call sheet says No call sheet yet for this day. when none exists. Important notes shows the notes written on the day, or No day notes.`,
        },
        {
          screen: `schedule-daily`,
          do: `Read Important breakdown items and Scene order.`,
          explain: `Important breakdown items shows tags such as Bird, Umbrella and Keys, the items confirmed in the Breakdown for these scenes. Scene order lists the scenes in order, for example 1. 1: INT. KESARGANJ JUNCTION - LOST PROPERTY OFFICE - NIGHT.`,
        },
      ],
      result: `You know what the day needs and what is still missing.`,
      cautions: [
        `The Daily View only displays. To fix a missing address, estimate or call time, use the Schedule, Locations or Breakdown.`,
      ],
      related: [`daily-view-create-call-sheet`, `daily-view-switch-day`],
    },
    {
      id: `daily-view-switch-day`,
      title: `Switch to another shooting day`,
      purpose: `Look at a different day without leaving the Daily View.`,
      level: `Beginner`,
      prerequisites: [`The Schedule has more than one shooting day.`],
      steps: [
        {
          screen: `schedule-daily-day-menu`,
          do: `Click the day pop-up at the top right, which reads Shoot Day 1 · Mon 14 Jun 2027.`,
          explain: `The pop-up is called Shooting day. It lists every shooting day with its date, for example Shoot Day 2 · Tue 15 Jun 2027 and Shoot Day 3 · Wed 16 Jun 2027.`,
        },
        {
          screen: `schedule-daily-day-2`,
          do: `Choose Shoot Day 2.`,
          explain: `The page now describes Tue 15 Jun 2027. When no cast is confirmed it says No cast confirmed for these scenes. When no items are confirmed it says No confirmed props, wardrobe or other items for these scenes. A day with a missing location shows Address missing.`,
        },
        {
          screen: `schedule-daily-day-3`,
          do: `Choose Shoot Day 3, which has no scenes yet.`,
          explain: `The banner says No scenes scheduled. Location shows Missing and Scene order says Nothing scheduled yet. Place scenes in the day in the Schedule and the page fills in.`,
        },
      ],
      result: `You can move between days and see each one's needs.`,
      cautions: [],
      related: [`daily-view-read`],
    },
    {
      id: `daily-view-create-call-sheet`,
      title: `Start a call sheet from the Daily View`,
      purpose: `Make a call sheet for the day you are reading.`,
      level: `Beginner`,
      prerequisites: [`The day has no call sheet yet.`],
      steps: [
        {
          screen: `schedule-daily`,
          do: `In the Call sheet card, click Create Call Sheet.`,
          explain: `The card says No call sheet yet for this day. The call sheet is made from this shooting day. The Call Sheets chapter explains how to complete and finalise it.`,
        },
      ],
      result: `A call sheet exists for the day.`,
      cautions: [
        `Fill in missing call times and addresses first. The Daily View shows what is missing.`,
      ],
      related: [`daily-view-read`],
    },
    {
      id: `daily-view-export`,
      title: `Export the Daily View`,
      purpose: `Save one day's Daily View as a PDF or a plain text file.`,
      level: `Beginner`,
      prerequisites: [`A shooting day is showing.`],
      steps: [
        {
          screen: `schedule-daily`,
          do: `Click Export at the top right.`,
          explain: `Export saves the day you are looking at.`,
        },
        {
          screen: `schedule-daily-export`,
          do: `In the Export Daily View sheet choose PDF or Plain text under Format, then click Export….`,
          explain: `The sheet names the source, for example Shoot Day 1 - Daily View, and shows the suggested file name, for example Shoot Day 1 - Daily View.pdf. It says: Private notes excluded. The document shows what the Daily View shows; the schedule is not changed. Choose a place for the file in the Mac save panel. Cancel closes the sheet.`,
        },
      ],
      result: `You have a file of the day to share.`,
      cautions: [
        `Private notes are never included in the export.`,
      ],
      related: [`daily-view-read`],
    },
  ],
  screens: {
    'schedule-daily-empty': `Daily View when there is no shooting day yet. It says Select or create a shooting day, explains the page and offers Open Schedule. Used in daily-view-open.`,
    'schedule-daily': `The Daily View for Shoot Day 1: title, day pop-up and Export, the check banner, and the cards Scenes, Location, Cast, Call sheet, Important notes, Important breakdown items and Scene order. Used in daily-view-open, daily-view-read, daily-view-create-call-sheet and daily-view-export.`,
    'schedule-daily-day-menu': `The shooting day pop-up open, listing Shoot Day 2 · Tue 15 Jun 2027 and Shoot Day 3 · Wed 16 Jun 2027. Used in daily-view-switch-day.`,
    'schedule-daily-export': `The Export Daily View sheet with the formats PDF and Plain text, the suggested file name, a note that private notes are excluded, Cancel and Export…. Used in daily-view-export.`,
    'schedule-daily-day-2': `The Daily View for Shoot Day 2, with one scene, a missing location, and messages that no cast and no items are confirmed. Used in daily-view-switch-day.`,
    'schedule-daily-day-3': `The Daily View for Shoot Day 3, which has no scenes. It says No scenes scheduled and Nothing scheduled yet. Used in daily-view-switch-day.`,
  },
  shortcuts: [],
  limits: [
    `The Daily View is read only. It has no fields to type into, apart from the day pop-up and its buttons.`,
    `It shows only what the Schedule, Breakdown and Locations already hold. Missing values are shown as missing, never guessed.`,
    `Private notes are excluded from the export.`,
  ],
  troubleshooting: [
    {
      problem: `The page says Select or create a shooting day.`,
      fix: `No shooting day exists. Click Open Schedule and create a shooting schedule with a day.`,
    },
    {
      problem: `The page says No scenes scheduled.`,
      fix: `The chosen day is empty. Pick another day in the pop-up, or place scenes in the day in the Schedule.`,
    },
    {
      problem: `Location shows Address missing.`,
      fix: `The scene's location has no address. Confirm one in Breakdown or Locations.`,
    },
    {
      problem: `Cast says Call times not entered yet.`,
      fix: `Open the day in the Schedule and set the Crew call, or complete the call sheet.`,
    },
    {
      problem: `No confirmed props, wardrobe or other items are shown.`,
      fix: `Confirm the items for those scenes in the Breakdown. The Daily View only shows confirmed items.`,
    },
  ],
};
