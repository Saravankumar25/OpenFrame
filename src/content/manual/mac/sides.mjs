export default {
  area: `Sides`,
  slug: `sides`,
  order: 14,
  part: `Shoot`,
  intro: [
    `Sides are the script pages a team needs for one shooting day. They contain only the scenes of that day, in screenplay format, so cast and crew carry a few pages instead of the whole script.`,
    `Open Sides from Production in the sidebar, using the pop-up at the top of the page: choose Sides. Sides need a shooting schedule with a shooting day, so create those in the Schedule first.`,
    `The page has a Setup panel on the left and a live preview of the pages on the right. You choose the day, the scenes and who the sides are for, add a call-sheet cover page if you want one, then save the sides as a snapshot or export them as a PDF.`,
  ],
  layout: [
    { name: `Setup panel`, text: `Shooting day, the Scenes of that day with a check box each, For (All cast or Selected cast), the options Include call-sheet cover page and Show draft / revision name, and the buttons Preview, Save Sides and Export PDF. A line under them says: Saved sides are snapshots. Later script edits don't change them.` },
    { name: `Sides preview`, text: `The pages as they will be printed, in screenplay format. The top line reads, for example, NIGHT BUS · SIDES · SHOOT DAY 1 followed by the screenplay name.` },
    { name: `Saved sides`, text: `A list under the Setup panel of the sides you have saved. Each entry shows its name, scene count and time, and a Delete button.` },
    { name: `Saved snapshot bar`, text: `Shown above the preview when a saved snapshot is open. It names the snapshot and offers Export PDF and Back to live preview.` },
  ],
  workflows: [
    {
      id: `sides-open`,
      title: `Open Sides for a shooting day`,
      purpose: `Reach the Sides page and see the scenes of a day.`,
      level: `Beginner`,
      prerequisites: [`A shooting schedule with a shooting day that has scenes.`],
      steps: [
        {
          screen: `schedule-sides-empty`,
          do: `Click Production in the sidebar, then choose Sides in the pop-up at the top of the page.`,
          explain: `Without a schedule the page says: Sides need a shooting schedule. It adds: Create a shooting schedule and a shooting day first. Sides contain that day's screenplay scenes. Click Open Schedule to set one up.`,
        },
        {
          screen: `schedule-sides`,
          do: `Read the page once a shooting day exists.`,
          explain: `The page asks: What script pages does this team need for this shooting day? In Setup, the Shooting day pop-up shows the chosen day, and Scenes lists its scenes with a check box each. The preview on the right shows the pages.`,
        },
      ],
      result: `The Sides page shows the scenes of a day and a preview of their pages.`,
      cautions: [],
      related: [`sides-choose-scenes`, `sides-choose-day`],
    },
    {
      id: `sides-choose-day`,
      title: `Choose another shooting day`,
      purpose: `Make sides for a different day of the shoot.`,
      level: `Beginner`,
      prerequisites: [`The Schedule has more than one shooting day.`],
      steps: [
        {
          screen: `schedule-sides-day-menu`,
          do: `Click the Shooting day pop-up in Setup.`,
          explain: `It lists every shooting day with its date, for example Shoot Day 2 · Tue 15 Jun 2027 and Shoot Day 3 · Wed 16 Jun 2027.`,
        },
        {
          screen: `schedule-sides`,
          do: `Choose a day and look at Scenes.`,
          explain: `Scenes now lists the scenes of that day. The preview title changes to the new day.`,
        },
      ],
      result: `Setup and the preview show the day you chose.`,
      cautions: [
        `A day with no scenes has nothing to preview. Place scenes in it in the Schedule first.`,
      ],
      related: [`sides-choose-scenes`],
    },
    {
      id: `sides-choose-scenes`,
      title: `Choose scenes and cast`,
      purpose: `Include only the scenes you want, and make sides for everyone or for chosen cast members.`,
      level: `Beginner`,
      prerequisites: [`The Sides page shows a shooting day.`],
      steps: [
        {
          screen: `schedule-sides`,
          do: `Under Scenes, clear the check box of any scene you do not want in the sides.`,
          explain: `Every scene of the day starts ticked, for example 1: INT. KESARGANJ JUNCTION - LOST PROPERTY OFFICE - NIGHT. Only ticked scenes appear in the preview and in the PDF.`,
        },
        {
          screen: `schedule-sides`,
          do: `Under For, leave All cast chosen to make one set of sides for everyone.`,
          explain: `All cast shows every selected scene.`,
        },
        {
          screen: `schedule-sides-selected-cast`,
          do: `Click Selected cast to make sides for particular people.`,
          explain: `A list of cast members appears, each with the number of scenes they have that day, for example Kiran · 2 scenes and Devi · 1 scene. Until you tick someone the preview says: Choose a shooting day and at least one scene and cast member to preview the sides. Preview and Export PDF stay greyed out until you do.`,
        },
        {
          screen: `schedule-sides`,
          do: `Tick the people you want, or go back to All cast.`,
          explain: `The preview shows the scenes for those cast members.`,
        },
      ],
      result: `The preview shows only the scenes and cast you selected.`,
      cautions: [
        `Check the For setting before you save or export. Selected cast with nobody ticked produces no preview.`,
      ],
      related: [`sides-cover-page`, `sides-save`],
    },
    {
      id: `sides-cover-page`,
      title: `Add the call-sheet cover page and the draft name`,
      purpose: `Put a cover page with the day's locations and cast in front of the sides, and show which draft the pages come from.`,
      level: `Beginner`,
      prerequisites: [`The Sides page shows a preview.`],
      steps: [
        {
          screen: `schedule-sides`,
          do: `Tick Include call-sheet cover page.`,
          explain: `The box is under For. Show draft / revision name is the box under it. In the picture the cover page is off and the draft name is on.`,
        },
        {
          screen: `schedule-sides-cover`,
          do: `Look at the top of the preview.`,
          explain: `A CALL SHEET COVER block now sits before the first scene. It shows the day and date, each location, and each cast member. A value that is not known yet reads Needs input.`,
        },
        {
          screen: `schedule-sides-cover`,
          do: `Clear Show draft / revision name if you do not want the draft name on the pages.`,
          explain: `The export sheet says the draft name appears on every page when this is on.`,
        },
      ],
      result: `The sides start with a cover page that matches the call sheet.`,
      cautions: [
        `Needs input on the cover means the information is missing in the Schedule, Locations or Cast. Fix it there, then preview again.`,
      ],
      related: [`sides-save`, `sides-export`],
    },
    {
      id: `sides-save`,
      title: `Save the sides as a snapshot`,
      purpose: `Keep a fixed copy of the sides that later script edits will not change.`,
      level: `Beginner`,
      prerequisites: [`The preview shows the pages you want.`],
      steps: [
        {
          screen: `schedule-sides-cover`,
          do: `Click Save Sides.`,
          explain: `The note under the buttons says: Saved sides are snapshots. Later script edits don't change them.`,
        },
        {
          screen: `schedule-sides-saved`,
          do: `Read what appears.`,
          explain: `A message says Saved sides, with Undo and Dismiss. A bar above the preview reads Saved snapshot “Sides: Shoot Day 1” with its date and time, with Export PDF and Back to live preview. The sides are added to Saved sides under the Setup panel, with a Delete button.`,
        },
      ],
      result: `A snapshot of the sides is saved with the day name, scene count and time.`,
      cautions: [
        `A snapshot does not follow the screenplay. After a script revision, save new sides.`,
        `Click Undo in the message if you saved by mistake.`,
      ],
      related: [`sides-open-saved`, `sides-export`],
    },
    {
      id: `sides-open-saved`,
      title: `Open or delete saved sides`,
      purpose: `Look at a saved snapshot again, go back to the live pages, or remove a snapshot.`,
      level: `Beginner`,
      prerequisites: [`At least one set of sides is saved.`],
      steps: [
        {
          screen: `schedule-sides-snapshot`,
          do: `Under Saved sides, click the entry, for example Sides: Shoot Day 1.`,
          explain: `Each entry shows its name, how many scenes it holds and when it was saved, for example 2 scenes · 28 Oct at 08:29. The preview area shows a spinner while the saved pages load.`,
        },
        {
          screen: `schedule-sides-saved`,
          do: `Click Back to live preview to return to the current script.`,
          explain: `Export PDF in the same bar exports the snapshot. Back to live preview shows the pages as the screenplay is now.`,
        },
        {
          screen: `schedule-sides-snapshot`,
          do: `To remove a snapshot, click Delete beside its entry.`,
          explain: `The button is named Delete followed by the sides name, for example Delete Sides: Shoot Day 1.`,
        },
      ],
      result: `You can reopen a snapshot, return to the live pages, or remove a snapshot you no longer need.`,
      cautions: [
        `Deleting a snapshot removes that saved copy. The screenplay is not changed.`,
      ],
      related: [`sides-save`, `sides-export`],
    },
    {
      id: `sides-export`,
      title: `Export the sides as a PDF`,
      purpose: `Make a PDF of the sides to send or print.`,
      level: `Beginner`,
      prerequisites: [`The preview shows the pages you want, or a saved snapshot is open.`],
      steps: [
        {
          screen: `schedule-sides`,
          do: `Click Export PDF in Setup. With a snapshot open, click Export PDF in the snapshot bar.`,
          explain: `Export PDF is greyed out while there is nothing to preview.`,
        },
        {
          screen: `schedule-sides-export`,
          do: `Read the Export Sides sheet, then click Export… and choose a place for the file in the Mac save panel.`,
          explain: `The sheet says: Only the selected scenes, in screenplay format, with the draft name on every page. It names the source and shows a suggested file name that includes the project, the day and the date. A note says: Sides are a snapshot of the screenplay source. Editing the PDF never changes the screenplay. Private notes excluded. Cancel closes the sheet.`,
        },
      ],
      result: `You have a PDF of the day's script pages.`,
      cautions: [
        `The PDF is a copy. Editing it never changes the screenplay.`,
      ],
      related: [`sides-save`, `sides-cover-page`],
    },
  ],
  screens: {
    'schedule-sides-empty': `Sides when there is no shooting schedule. It says Sides need a shooting schedule and offers Open Schedule. Used in sides-open.`,
    'schedule-sides': `The Sides page for Shoot Day 1: Setup with the day, two ticked scenes, All cast chosen, the two options, Preview, Save Sides and Export PDF, and the preview of the pages. Used in sides-open, sides-choose-day, sides-choose-scenes, sides-cover-page and sides-export.`,
    'schedule-sides-day-menu': `The Shooting day pop-up open, listing Shoot Day 2 · Tue 15 Jun 2027 and Shoot Day 3 · Wed 16 Jun 2027. Used in sides-choose-day.`,
    'schedule-sides-selected-cast': `Sides with Selected cast chosen. The list shows Kiran · 2 scenes and Devi · 1 scene, nobody is ticked, the preview asks you to choose at least one scene and cast member, and Preview and Export PDF are greyed out. Used in sides-choose-scenes.`,
    'schedule-sides-cover': `Sides with Include call-sheet cover page ticked. The preview starts with a CALL SHEET COVER block showing the day, the locations and the cast, with Needs input where a value is missing. Used in sides-cover-page and sides-save.`,
    'schedule-sides-export': `The Export Sides sheet over the cover-page preview. It describes the PDF, names the source, shows the suggested file name and the snapshot note, with Cancel and Export…. Used in sides-export.`,
    'schedule-sides-saved': `Sides right after Save Sides. A message says Saved sides, a bar above the preview names the saved snapshot and offers Export PDF and Back to live preview, and Saved sides lists the entry with Delete. Used in sides-save and sides-open-saved.`,
    'schedule-sides-snapshot': `The Sides page while a saved snapshot loads. Setup and the Saved sides entry with its Delete button are shown, and the preview area shows a spinner. Used in sides-open-saved.`,
  },
  shortcuts: [],
  limits: [
    `Sides contain only the scenes of one shooting day, in screenplay format. They are not a replacement for the full script.`,
    `A saved snapshot never changes when the screenplay changes. Save new sides after a revision.`,
    `The export is a PDF. Private notes are excluded.`,
  ],
  troubleshooting: [
    {
      problem: `The page says Sides need a shooting schedule.`,
      fix: `Click Open Schedule, create a shooting schedule and a shooting day, and place scenes in it.`,
    },
    {
      problem: `The preview says to choose a shooting day and at least one scene and cast member.`,
      fix: `Tick at least one scene, and under Selected cast tick at least one person, or switch For back to All cast.`,
    },
    {
      problem: `Preview and Export PDF are greyed out.`,
      fix: `There is nothing to show yet. Tick a scene and, with Selected cast, a cast member.`,
    },
    {
      problem: `The cover page shows Needs input.`,
      fix: `That information is missing. Fill it in the Schedule, Locations or cast details, then preview again.`,
    },
    {
      problem: `My saved sides do not show the latest script changes.`,
      fix: `Saved sides are snapshots. Click Back to live preview, then Save Sides again.`,
    },
  ],
};
