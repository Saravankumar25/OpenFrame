export default {
  area: `Sides`,
  slug: `sides`,
  order: 19,
  part: `Shoot`,
  intro: [
    `Sides are the script pages an actor or a department needs for one shooting day: only that day's scenes, in shooting order, set out like screenplay pages. Instead of sending the whole script, you send the few pages that matter for the day.`,
    `The Sides tab has two halves. On the left is a Setup panel: pick the shooting day, tick the scenes, choose whether the sides are for all cast or only some actors, and choose two options for the top of the document. On the right is a live preview of the pages, which changes as you change the setup. The pages come from the screenplay version set as the production source in the Breakdown. Editing here never changes your screenplay.`,
    `When the pages look right, you can export them as a PDF, or save them. Saving takes a snapshot of the sides as they are now. A saved set appears under SAVED SIDES at the bottom of the Setup panel and does not change when the script changes later. This is the version you can trust to match what you sent.`,
    `Sides need a shooting schedule with at least one shooting day. Without one, the tab tells you to open the Schedule first.`,
  ],
  layout: [
    { name: `Heading`, text: `Sides, with the line What script pages does this team need for this shooting day?.` },
    { name: `Setup panel`, text: `The panel on the left. It holds the Shooting day box, the Scenes checklist, the For switch (All cast and Selected cast), the options Include call-sheet cover page and Show draft / revision name, the buttons Preview, Save Sides and Export PDF, and the note Saved sides are snapshots. Later script edits don't change them.` },
    { name: `Shooting day`, text: `A box listing the shooting days of the schedule, each with its date. Off days are not in this list. Changing the day resets the scene ticks and the cast choice.` },
    { name: `Scenes`, text: `One tick box for each scene on the day, written as the scene number, a colon and the scene heading, for example 9: EXT. RAYAPURAM BUS STAND - DAWN. All scenes are ticked when you choose a day.` },
    { name: `For`, text: `A two-way switch: All cast or Selected cast. With Selected cast, a list of the day's actors appears, each with the number of scenes they are in, for example Ananya Rao (LAKSHMI) · 4 scenes.` },
    { name: `Options`, text: `Include call-sheet cover page, which is off by default, and Show draft / revision name, which is on by default.` },
    { name: `Preview, Save Sides and Export PDF`, text: `Three buttons under the options. They are greyed out until the day, scenes and cast choice are enough to make sides.` },
    { name: `SAVED SIDES`, text: `A list at the bottom of the Setup panel, shown once you have saved sides. Each row shows the name, the number of scenes and the date and time, and has a bin button to delete it.` },
    { name: `Preview area`, text: `The right half. It shows the sides as a page: a title line such as NIGHT BUS · SIDES · SHOOT DAY 1 · Second Draft, then each scene. When you open a saved set, a blue bar at the top says Saved snapshot and offers Export PDF and Back to live preview.` },
  ],
  workflows: [
    {
      id: `sides-preview-day`,
      title: `Preview the sides for a shooting day`,
      purpose: `Choose a day and see its script pages at once.`,
      level: `Beginner`,
      prerequisites: [`A shooting schedule with at least one shooting day that has scenes. The screenplay has a production source set in the Breakdown.`],
      steps: [
        {
          screen: `prod-sides-setup`,
          do: `Open Production and click Sides in the row of tabs.`,
          explain: `The Setup panel opens at the left with the first shooting day chosen, here Shoot Day 1 · Mon 9 Nov 2026. All of that day's scenes are ticked and All cast is selected, so the preview on the right is already showing pages.`,
        },
        {
          screen: `prod-sides-setup`,
          do: `Read the title line at the top of the preview.`,
          explain: `It reads NIGHT BUS · SIDES · SHOOT DAY 1 · Second Draft: the project, the word SIDES, the day and the name of the screenplay draft the pages come from. Below it are the scenes of the day with their numbers, headings, action, character names and dialogue.`,
        },
        {
          screen: `prod-sides-day-4`,
          do: `Open the Shooting day box and choose another day, for example Shoot Day 4 · Thu 12 Nov 2026.`,
          explain: `The Scenes list changes to that day's scenes, 9 to 12, all ticked, and the preview follows. Choosing a different day clears any scene ticks or cast choices you had made.`,
        },
        {
          screen: `prod-sides-day-4`,
          do: `Click Preview if you want the pages refreshed.`,
          explain: `The preview updates by itself when you change a setting. Preview is there to show the live pages again, for example after you have been looking at a saved set.`,
        },
      ],
      result: `The right half shows the script pages for the day you chose.`,
      cautions: [
        `The pages are copied from the screenplay as it is now. If you change the script later, the live preview changes, but a saved set does not.`,
      ],
      related: [`sides-choose-scenes`, `sides-save`],
    },
    {
      id: `sides-choose-scenes`,
      title: `Leave scenes out of the sides`,
      purpose: `Untick scenes you do not want in the document, and know what happens on a day with no scenes.`,
      level: `Beginner`,
      prerequisites: [`The Sides tab is open on a day with scenes.`],
      steps: [
        {
          screen: `prod-sides-day-4`,
          do: `Look at the Scenes list.`,
          explain: `Each scene has a tick box and is written as number, colon, heading. Every scene on the day starts ticked.`,
        },
        {
          screen: `prod-sides-scene-unchecked`,
          do: `Click the tick box of a scene to clear it, for example 9: EXT. RAYAPURAM BUS STAND - DAWN.`,
          explain: `The scene stays in the list but is left out of the preview and of any export or saved set. Tick it again to put it back.`,
        },
        {
          screen: `prod-sides-no-scenes`,
          do: `Choose a shooting day that has nothing scheduled, for example Shoot Day 5 · Sun 15 Nov 2026.`,
          explain: `The Scenes list says No scenes scheduled on this day. The preview area says Choose a shooting day and at least one scene to preview the sides. Preview, Save Sides and Export PDF stay greyed out. Add scenes to the day in the Schedule, then come back.`,
        },
      ],
      result: `The sides include only the scenes that are ticked.`,
      cautions: [
        `If you clear every tick, the preview empties and Preview, Save Sides and Export PDF are greyed out. Tick at least one scene.`,
      ],
      related: [`sides-preview-day`, `sides-selected-cast`],
    },
    {
      id: `sides-selected-cast`,
      title: `Make sides for selected actors only`,
      purpose: `Keep only the scenes that include the actors you choose.`,
      level: `Intermediate`,
      prerequisites: [`The day's scenes have confirmed cast in the Breakdown.`],
      steps: [
        {
          screen: `prod-sides-day-4`,
          do: `Under For, click Selected cast.`,
          explain: `All cast is the default: every ticked scene is in the sides. Selected cast keeps only the scenes in which the actors you pick appear.`,
        },
        {
          screen: `prod-sides-selected-cast-none`,
          do: `Read the list of actors that appears under the switch.`,
          explain: `Each actor is listed with the character and the number of scenes, for example Ananya Rao (LAKSHMI) · 4 scenes and Suresh Babu (VENKAT RAO) · 4 scenes. Until you tick someone, the preview says Choose a shooting day and at least one scene and cast member to preview the sides, and Preview, Save Sides and Export PDF are greyed out.`,
        },
        {
          screen: `prod-sides-selected-cast`,
          do: `Tick the actor you want, for example Ananya Rao (LAKSHMI) · 4 scenes.`,
          explain: `A note appears above For: 4 of 4 scenes have the selected cast. Only those are in the sides. The preview shows those scenes.`,
        },
        {
          screen: `prod-sides-selected-cast`,
          do: `Tick more actors to include their scenes too.`,
          explain: `A scene counts when any ticked actor is in it. A scene with none of them is shown in the list greyed out with the words Left out: no selected cast in this scene, and it is not in the sides. Your own tick on that scene is kept, so switching back to All cast brings it back unchanged.`,
        },
        {
          screen: `prod-sides-setup`,
          do: `Click All cast to go back to the whole day.`,
          explain: `Every ticked scene is included again.`,
        },
      ],
      result: `The sides contain only the scenes that include the actors you ticked.`,
      cautions: [
        `If no scene on the day includes the ticked actors, the preview says there is nothing to show and tells you to choose other cast or switch to All cast.`,
        `Cast comes from each scene's confirmed breakdown. If an actor is missing from the list, confirm the character in the Breakdown. With no confirmed cast, the panel says No confirmed cast on this day.`,
      ],
      related: [`sides-choose-scenes`, `sides-cover-page`],
    },
    {
      id: `sides-cover-page`,
      title: `Add a call sheet cover page and show the draft name`,
      purpose: `Put the day's locations, cast and notes at the top of the sides, and control the draft name line.`,
      level: `Beginner`,
      prerequisites: [`The Sides tab is open on a day with scenes.`],
      steps: [
        {
          screen: `prod-sides-day-4`,
          do: `Look under the horizontal line in the Setup panel.`,
          explain: `Two tick boxes: Include call-sheet cover page, which is off, and Show draft / revision name, which is on.`,
        },
        {
          screen: `prod-sides-cover-page`,
          do: `Tick Include call-sheet cover page.`,
          explain: `A block called CALL SHEET COVER appears at the top of the preview. It shows the day and date, the locations (Rayapuram Bus Stand, Farooq's Bus - Moving and so on), a small table of actor, character and call time, and the day's notes. This is a summary taken from the day's schedule, not a separate call sheet.`,
        },
        {
          screen: `prod-sides-cover-page`,
          do: `Clear Show draft / revision name if you do not want the draft name printed.`,
          explain: `With the box ticked, a line at the end of the pages names the source draft. With it cleared the line is left out.`,
        },
      ],
      result: `The sides carry the cover block and draft line you chose.`,
      cautions: [
        `The cover page repeats what the schedule says today. If the schedule is not finished, the cover will show gaps.`,
      ],
      related: [`sides-save`, `sides-export-pdf`],
    },
    {
      id: `sides-save`,
      title: `Save the sides as a snapshot`,
      purpose: `Keep a fixed copy of the sides exactly as you will send them.`,
      level: `Beginner`,
      prerequisites: [`The preview shows the pages you want.`],
      steps: [
        {
          screen: `prod-sides-setup`,
          do: `Check the day, scenes, cast choice and options, then click Save Sides.`,
          explain: `The note beside the buttons says Saved sides are snapshots. Later script edits don't change them.`,
        },
        {
          screen: `prod-sides-saved`,
          do: `Look for the message at the bottom: Saved sides.`,
          explain: `The message has an Undo button and a close button. Undo reverses the save. The set appears under SAVED SIDES as Sides: Shoot Day 1 with 3 scenes and the date and time, and it opens in the preview.`,
        },
        {
          screen: `prod-sides-saved`,
          do: `Read the blue bar at the top of the preview.`,
          explain: `It says Saved snapshot "Sides: Shoot Day 1" with the date and time. It has Export PDF, which exports this saved set, and Back to live preview, which returns to the live pages.`,
        },
      ],
      result: `A saved set of sides is listed under SAVED SIDES and will not change.`,
      cautions: [
        `A saved set is a copy. If the script changes after you save, the saved set keeps the old pages. Save a new one if you need the new pages.`,
      ],
      related: [`sides-open-saved`, `sides-delete-saved`],
    },
    {
      id: `sides-open-saved`,
      title: `Open a saved set and go back to the live preview`,
      purpose: `Look at a saved set of sides, then return to working on the live pages.`,
      level: `Beginner`,
      prerequisites: [`At least one set of sides has been saved.`],
      steps: [
        {
          screen: `prod-sides-live-with-saved`,
          do: `Look at the bottom of the Setup panel.`,
          explain: `Under SAVED SIDES each saved set shows its name, for example Sides: Shoot Day 1, with the number of scenes and the date and time it was saved. In this picture the live preview is showing.`,
        },
        {
          screen: `prod-sides-saved`,
          do: `Click the saved set.`,
          explain: `The preview shows the saved pages with the blue Saved snapshot bar above them. The setup choices on the left are the live ones, not the choices of the saved set.`,
        },
        {
          screen: `prod-sides-live-with-saved`,
          do: `Click Back to live preview in the blue bar, or click Preview.`,
          explain: `The page returns to the live sides for the setup on the left. The saved set stays in the list.`,
        },
      ],
      result: `You can move between a saved set and the live preview without losing either.`,
      cautions: [],
      related: [`sides-save`, `sides-export-pdf`],
    },
    {
      id: `sides-delete-saved`,
      title: `Delete a saved set of sides`,
      purpose: `Remove a saved set you no longer need, and undo it if you change your mind.`,
      level: `Beginner`,
      prerequisites: [`At least one set of sides has been saved.`],
      steps: [
        {
          screen: `prod-sides-live-with-saved`,
          do: `In SAVED SIDES, click the bin (delete) button beside the set. Its tooltip names the set, for example Delete Sides: Shoot Day 1.`,
          explain: `The saved set is removed from the list, and a message at the bottom says Deleted saved sides. If the set was open, the page returns to the live preview.`,
        },
        {
          screen: `prod-sides-saved`,
          do: `Click Undo in the message if you deleted the wrong set.`,
          explain: `The picture shows the same message for saving, with Undo and a close button. Do this before you close the message.`,
        },
      ],
      result: `The saved set is gone from SAVED SIDES.`,
      cautions: [
        `Deleting a saved set does not touch the screenplay or the schedule. It only removes the saved copy.`,
      ],
      related: [`sides-save`],
    },
    {
      id: `sides-export-pdf`,
      title: `Export the sides as a PDF`,
      purpose: `Save the sides as a PDF to print or send.`,
      level: `Beginner`,
      prerequisites: [`The preview shows the pages you want, or a saved set is open.`],
      steps: [
        {
          screen: `prod-sides-live-with-saved`,
          do: `Click Export PDF under the options. To export a saved set, open it and click Export PDF in the blue bar.`,
          explain: `Under the options the button exports the live setup. In the blue bar it exports that saved set exactly as it was saved.`,
        },
        {
          screen: `prod-sides-export-dialog`,
          do: `In the Export Sides dialog, leave PDF printable snapshot selected.`,
          explain: `PDF is the only format for sides. The dialog says: Sides are a snapshot of the screenplay source. Editing the PDF never changes the screenplay. Private notes excluded.`,
        },
        {
          screen: `prod-sides-export-dialog`,
          do: `Click Export… and choose a folder and file name in the Windows window.`,
          explain: `Cancel, or the Close button at the top, leaves without saving.`,
        },
      ],
      result: `A PDF of the sides is saved where you chose.`,
      cautions: [
        `The PDF is a copy. Editing it does not change your screenplay.`,
        `Private notes are not included.`,
      ],
      related: [`sides-save`, `sides-cover-page`],
    },
    {
      id: `sides-no-schedule`,
      title: `What to do when Sides asks for a schedule`,
      purpose: `Know why Sides is empty and how to get started.`,
      level: `Beginner`,
      prerequisites: [`The project has no shooting schedule yet.`],
      steps: [
        {
          screen: `prod-sides-empty`,
          do: `Open Sides in a project with no schedule.`,
          explain: `The page says Sides need a shooting schedule. Under it: Create a shooting schedule and a shooting day first. Sides contain that day's screenplay scenes.`,
        },
        {
          screen: `prod-sides-empty`,
          do: `Click Open Schedule.`,
          explain: `The Schedule tab opens. Create the schedule and add shooting days with scenes. Then return to Sides.`,
        },
      ],
      result: `You are in the Schedule, ready to add the day the sides will be made from.`,
      cautions: [
        `If there is a schedule but no shooting days yet, the Setup panel says Create a shooting day first.`,
      ],
      related: [`sides-preview-day`],
    },
  ],
  screens: {
    'prod-sides-setup': `The Sides tab on Shoot Day 1. The Setup panel shows the day, three ticked scenes, All cast selected, the two options and the Preview, Save Sides and Export PDF buttons. The preview shows the title line and the first scenes. Used in sides-preview-day, sides-selected-cast and sides-save.`,
    'prod-sides-day-4': `The Sides tab on Shoot Day 4 with scenes 9 to 12 ticked and All cast selected. Used in sides-preview-day, sides-choose-scenes, sides-selected-cast and sides-cover-page.`,
    'prod-sides-selected-cast-none': `Selected cast chosen on Shoot Day 4 with no actor ticked. The actors Ananya Rao (LAKSHMI) and Suresh Babu (VENKAT RAO) are listed with 4 scenes each, the preview says to choose at least one scene and cast member, and the buttons are greyed out. Used in sides-selected-cast.`,
    'prod-sides-selected-cast': `Ananya Rao ticked under Selected cast. The note 4 of 4 scenes have the selected cast. Only those are in the sides. shows and the preview lists scenes 9 to 12. Used in sides-selected-cast.`,
    'prod-sides-cover-page': `Include call-sheet cover page ticked. The preview starts with CALL SHEET COVER: the day, the locations, the cast with characters and the day's notes, followed by the scenes. Used in sides-cover-page.`,
    'prod-sides-scene-unchecked': `Scene 9 unticked on Shoot Day 4. The scene stays in the list but is left out of the preview. Used in sides-choose-scenes.`,
    'prod-sides-no-scenes': `A shooting day with nothing scheduled (Shoot Day 5). The Scenes list says No scenes scheduled on this day., the preview asks you to choose a day and a scene, and the buttons are greyed out. Used in sides-choose-scenes.`,
    'prod-sides-saved': `A saved set open after Save Sides. SAVED SIDES lists Sides: Shoot Day 1 (3 scenes) and the blue bar says Saved snapshot with Export PDF and Back to live preview. A message at the bottom says Saved sides with Undo and a close button. Used in sides-save, sides-open-saved and sides-delete-saved.`,
    'prod-sides-live-with-saved': `The live preview with a saved set in SAVED SIDES, which shows the set's name, a scene count and the date and a bin button to delete it. Used in sides-open-saved, sides-delete-saved and sides-export-pdf.`,
    'prod-sides-export-dialog': `The Export Sides dialog. PDF printable snapshot is the only format. It says Sides are a snapshot of the screenplay source and that private notes are excluded. Cancel and Export…. Used in sides-export-pdf.`,
    'prod-sides-empty': `Sides in a project with no shooting schedule. It says Sides need a shooting schedule. and offers Open Schedule. Used in sides-no-schedule.`,
  },
  shortcuts: [],
  limits: [
    `Sides are made from one shooting day at a time.`,
    `PDF is the only export format for sides.`,
    `Off days are not in the Shooting day list, because they have no scenes.`,
    `A saved set is a fixed copy. It is not updated when the screenplay or schedule changes. Save a new set to capture changes.`,
    `Selecting cast uses confirmed breakdown cast. Cast that is only suggested is not used.`,
  ],
  troubleshooting: [
    {
      problem: `Preview, Save Sides and Export PDF are greyed out.`,
      fix: `The sides need a day with at least one ticked scene. With Selected cast chosen, they also need at least one actor ticked. Tick a scene or an actor.`,
    },
    {
      problem: `The preview says No scene on this day has the selected cast.`,
      fix: `Choose other cast, or switch to All cast. Cast comes from each scene's confirmed breakdown.`,
    },
    {
      problem: `A scene is greyed out and says Left out: no selected cast in this scene.`,
      fix: `You are using Selected cast and none of the ticked actors is in that scene. Tick another actor or switch to All cast.`,
    },
    {
      problem: `The day I want is not in the Shooting day box.`,
      fix: `Off days are not listed. Check that the day is a shooting day in the Schedule.`,
    },
    {
      problem: `My saved sides still show old script pages.`,
      fix: `A saved set never changes. Open the live preview and click Save Sides to make a new set from the current script.`,
    },
    {
      problem: `The pages are not from the draft I expected.`,
      fix: `Sides use the screenplay version chosen as the production source in the Breakdown. Change the source there.`,
    },
  ],
};
