export default {
  area: `Recovery`,
  slug: `recovery`,
  order: 26,
  part: `Project and data`,
  intro: [
    `Straft saves your work as you type, so a crash or a forced quit should not cost you your latest changes. The Saved indicator at the bottom of the project sidebar shows that your project is written to disk. There is no Save command to run.`,
    `This chapter covers the two situations where Straft stops to talk to you. The first is after the app closed unexpectedly: it tells you that your latest work was kept. The second is when a project cannot be found, for example because it is on an external drive that is not connected.`,
    `In both cases Straft never makes a replacement project for you and never deletes anything on its own. You decide what happens next.`,
  ],
  layout: [
    { name: `Recovery message`, text: `A message over the project that says Straft closed unexpectedly last time. It tells you that your latest work was kept and when it was last saved, and offers Keep the latest work.` },
    { name: `Unavailable card`, text: `A project card on Application Home with an Unavailable label and the line Folder not found. Reconnect the drive or locate the project.` },
    { name: `This project could not be opened`, text: `A sheet that names the project and its last known location, and offers Remove from List, Cancel, Locate Project… and Try Again.` },
  ],
  workflows: [
    {
      id: `recovery-after-unexpected-close`,
      title: `Reopen a project after Straft closed unexpectedly`,
      purpose: `Open your project again after a forced quit or a crash, and keep your latest work.`,
      level: `Beginner`,
      prerequisites: [`Straft closed without you quitting it, for example after a forced quit or a power cut.`],
      steps: [
        {
          screen: `home-after-crash`,
          do: `Open Straft Studio again.`,
          explain: `Application Home opens as usual. Your project is still on its card, with its title, type, status and Last modified. Nothing is removed from the list.`,
        },
        {
          screen: `home-after-crash`,
          do: `Click the card of the project you were working in.`,
          explain: `The project opens. If Straft closed unexpectedly the last time you used it, a message appears over the project.`,
        },
        {
          screen: `recovery-alert`,
          do: `Read the message Straft closed unexpectedly last time.`,
          explain: `It says: Your latest work was kept. Last saved, followed by the date and time of the last save. The project page is behind the message, with Saved in the sidebar. The RECENT list on Project Home shows your last changes, for example Typing in Scene 1.`,
        },
        {
          screen: `recovery-alert`,
          do: `Click Keep the latest work.`,
          explain: `The message closes and the project is ready. This is the only choice, because Straft has already kept your latest work.`,
        },
        {
          screen: `pages-recovery-kept`,
          do: `Look at Project Home.`,
          explain: `The page is back to normal. RECENT shows the changes that were saved before the app closed, for example Typing in Scene 1 and Created the screenplay. Check your pages to see that your work is there.`,
        },
      ],
      result: `The project is open with your latest saved work, and you can carry on.`,
      cautions: [
        `The message tells you the time of the last save. If you typed after that time, check your last few lines.`,
        `Keep your library folder backed up. A backup protects you from more than a crash. See the Packages chapter.`,
      ],
      related: [`recovery-project-unavailable`],
    },
    {
      id: `recovery-project-unavailable`,
      title: `Open a project that cannot be found`,
      purpose: `Deal with a project whose folder is not where Straft expects it.`,
      level: `Intermediate`,
      prerequisites: [`A project card on Application Home is marked Unavailable.`],
      steps: [
        {
          screen: `home-project-unavailable`,
          do: `Find the project card on Application Home.`,
          explain: `The card carries the label Unavailable and says Folder not found. Reconnect the drive or locate the project. The cover is greyed out. The project is not deleted. Its folder is just not where Straft last saw it.`,
        },
        {
          screen: `project-unavailable`,
          do: `Click the card.`,
          explain: `A sheet titled This project could not be opened appears. A red line says The project location is unavailable. It may be on an external drive that is not connected, or the folder was moved. Below it are the project name and its last known location.`,
        },
        {
          screen: `project-unavailable`,
          do: `Read the last line of the sheet.`,
          explain: `It says: Straft has not created a replacement project. Reconnect the drive and try again, or locate another copy of the project.`,
        },
        {
          screen: `project-unavailable`,
          do: `If the project is on an external drive, connect the drive and click Try Again.`,
          explain: `Try Again looks for the project in its last known location. If it is there now, it opens.`,
        },
        {
          screen: `project-unavailable`,
          do: `If the folder was moved, click Locate Project… and pick the project in the macOS window that opens.`,
          explain: `Use this when you know where the project is now, or when you have another copy of it.`,
        },
        {
          screen: `project-unavailable`,
          do: `If you no longer want the card, click Remove from List.`,
          explain: `This only removes the card from your list. Nothing is deleted. Cancel closes the sheet and leaves everything as it was.`,
        },
      ],
      result: `The project opens again, or you have removed its card without deleting any files.`,
      cautions: [
        `Remove from List does not delete the project. If the folder is still in your projects folder, it can come back under Found on this Mac on Application Home. See the Home chapter.`,
        `Do not make a new project to replace a missing one. Reconnect the drive or locate the project first, so you keep your work.`,
      ],
      related: [`recovery-after-unexpected-close`],
    },
  ],
  screens: {
    'home-after-crash': `Application Home after a forced quit. The project card for Night Bus is still in RECENT PROJECTS with its type, status and Last modified. Used in recovery-after-unexpected-close.`,
    'recovery-alert': `Night Bus on Project Home with a message over it: Straft closed unexpectedly last time. Your latest work was kept, with the time of the last save, and a Keep the latest work button. Used in recovery-after-unexpected-close.`,
    'pages-recovery-kept': `Project Home after Keep the latest work. RECENT lists the saved changes, for example Typing in Scene 1 and Created the screenplay. Used in recovery-after-unexpected-close.`,
    'home-project-unavailable': `Application Home with the Night Bus card marked Unavailable and the line Folder not found. Reconnect the drive or locate the project. Used in recovery-project-unavailable.`,
    'project-unavailable': `The sheet This project could not be opened. It explains that the project location is unavailable, shows the project and its last known location, and offers Remove from List, Cancel, Locate Project… and Try Again. Used in recovery-project-unavailable.`,
  },
  shortcuts: [],
  limits: [
    `Straft keeps the work it has saved. It cannot recover words you typed after the last save.`,
    `Straft never makes a replacement for a missing project. You reconnect the drive, locate the project, or remove the card.`,
    `Removing a card from the list never deletes a project.`,
  ],
  troubleshooting: [
    {
      problem: `Straft says it closed unexpectedly. Is my work gone?`,
      fix: `No. The message says your latest work was kept and shows when it was last saved. Click Keep the latest work and check your pages.`,
    },
    {
      problem: `A project card says Unavailable.`,
      fix: `Its folder is not where Straft last saw it. Connect the drive it was on and click Try Again, or click Locate Project… and pick the folder.`,
    },
    {
      problem: `I clicked Remove from List by mistake.`,
      fix: `Nothing was deleted. Open Application Home and look under Found on this Mac, or use Open Project to open the folder again.`,
    },
    {
      problem: `I moved my whole library to another drive.`,
      fix: `On the Set up your Straft library screen, or in Settings > Storage, locate the existing library instead of making a new one.`,
    },
  ],
};
