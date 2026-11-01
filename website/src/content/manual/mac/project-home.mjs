export default {
  area: `Project Home`,
  slug: `project-home`,
  order: 5,
  part: `Start`,
  intro: [
    `Project Home is the first page of an open project. It shows the project title, its type and when you last opened it, a set of quick links to the main pages, your project files, and a list of recent changes. Everything else in the project is one click away from here.`,
    `A brand new project shows a shorter page with three starting points: Open Idea Vault, Build Story and Write/Import Screenplay. Once the project has content, the page fills with quick links, files and recent activity.`,
    `The window around Project Home is the same on every page of the project: a sidebar on the left, a toolbar along the top and a small status area at the bottom of the sidebar. This chapter explains that frame as well as the page.`,
  ],
  layout: [
    { name: `Sidebar`, text: `The project name, then Home, Idea Vault, Story, Screenplay, Breakdown, Production, Call Sheets and Files. At the bottom, Project stored locally on this Mac and a Saved indicator.` },
    { name: `Toolbar`, text: `Hide Sidebar, All Projects, Back and Forward, the Project menu, Search, the New menu, Assistant and Settings.` },
    { name: `Title area`, text: `The project title, then the type and when you last opened it, for example Feature Film · Last opened 5 seconds ago. At the right: the Status menu and Project settings.` },
    { name: `QUICK ACCESS`, text: `Buttons for Idea Vault, Beat Board, Screenplay, Breakdown, Production and Call Sheets, and links for Notes & Tasks, Activity, Recently Deleted and Templates.` },
    { name: `Project Files`, text: `A card at the right with Add File, the files you have added, and Open Files. With no files it says Keep project documents and attachments here.` },
    { name: `RECENT`, text: `A list of the latest changes in the project, each with who did it and how long ago.` },
  ],
  workflows: [
    {
      id: `project-home-tour`,
      title: `Find your way around Project Home`,
      purpose: `Know what each part of the page and the window around it does.`,
      level: `Beginner`,
      prerequisites: [`A project is open.`],
      steps: [
        {
          screen: `project-home`,
          do: `Look at the left side and the top of the window.`,
          explain: `The sidebar lists the main pages. The page you are on is highlighted, here Home. The toolbar has Hide Sidebar, All Projects, Back and Forward, Project, Search, New, Assistant and Settings. At the bottom of the sidebar, Project stored locally on this Mac tells you where your work is, and Saved shows that your latest changes are written to disk.`,
        },
        {
          screen: `project-home`,
          do: `Look at the middle of the page.`,
          explain: `Under the title, the line says the project type and when you last opened it. QUICK ACCESS has buttons for the main pages and links for Notes & Tasks, Activity, Recently Deleted and Templates. Click any of them to open that page.`,
        },
        {
          screen: `project-home`,
          do: `Look at the right side and the bottom of the page.`,
          explain: `Project Files lists your files, such as Example-Short.fountain, with Add File and Open Files. RECENT lists the latest changes, for example Imported screenplay “LOST PROPERTY”, with the name of the person and the time.`,
        },
        {
          screen: `project-home-empty`,
          do: `For a brand new project, read the page.`,
          explain: `It says Start with your ideas, build your story, or open a screenplay. Pick whichever fits where you are. You can move between them at any time. Three cards follow: Open Idea Vault to collect thoughts, images, links and notes, Build Story to arrange acts, beats and scene cards on the Beat Board, and Write/Import Screenplay to start writing or import a screenplay you already have. The line under the title says Created followed by how long ago.`,
        },
      ],
      result: `You can say what each part of Project Home is for.`,
      cautions: [
        `Saved in the sidebar means the project is written to disk. There is no Save command to run.`,
      ],
      related: [`project-home-set-status`, `project-home-create-from-new`],
    },
    {
      id: `project-home-set-status`,
      title: `Set the status of your project`,
      purpose: `Mark where the project is, from Idea to Shooting.`,
      level: `Beginner`,
      prerequisites: [`A project is open on Project Home.`],
      steps: [
        {
          screen: `project-home`,
          do: `Click Status at the top right of the page. It shows the current status, for example Status: Idea.`,
          explain: `The status is also shown on the project card on Application Home.`,
        },
        {
          screen: `project-home-status-menu`,
          do: `Choose a status from the menu.`,
          explain: `The choices are Idea, Development, Writing, Rewrite, Shooting Draft, Pre-Production, Shoot Preparation and Shooting. The same menu is under Status in the Project menu.`,
        },
      ],
      result: `The page shows the new status, and the project card on Application Home shows it too.`,
      cautions: [],
      related: [`project-home-project-menu`],
    },
    {
      id: `project-home-create-from-new`,
      title: `Create something from the New menu`,
      purpose: `Start a new idea, scene card, character, location or other item from anywhere in the project.`,
      level: `Beginner`,
      prerequisites: [`A project is open.`],
      steps: [
        {
          screen: `project-new-menu`,
          do: `Click New in the toolbar.`,
          explain: `The menu is headed Create. It lists New Idea, New Act, New Sequence, New Beat, New Scene Card, New Character, New Location, New Shot List and New Moodboard. After them come Import Screenplay…, New Note and New Task. New Project… is at the bottom.`,
        },
        {
          screen: `project-new-menu`,
          do: `Choose the item you want to make.`,
          explain: `The item is created in the part of the project it belongs to. New Project… starts a different project, with the New Project sheet. See the Home chapter.`,
        },
      ],
      result: `The item you chose is created.`,
      cautions: [
        `New Project… makes a separate project. It does not add anything to the project you have open.`,
      ],
      related: [`project-home-tour`],
    },
    {
      id: `project-home-project-menu`,
      title: `Use the Project menu`,
      purpose: `Reach settings, rename, duplicate, archive and the project pages from one menu.`,
      level: `Beginner`,
      prerequisites: [`A project is open.`],
      steps: [
        {
          screen: `project-menu`,
          do: `Click Project in the toolbar.`,
          explain: `The menu starts with Status, which opens a list of statuses. Then Project Settings… and Settings…. Next are Rename…, Duplicate Project and Show in Finder. Then Notes and Tasks, Activity and Recently Deleted. Archive Project comes next, and Search and Commands… and All Projects are at the bottom. Dividers separate the groups.`,
        },
        {
          screen: `project-menu`,
          do: `Choose the command you need.`,
          explain: `Project Settings… opens the settings of this project. Settings… opens the settings of the app. Show in Finder shows the project in a Finder window. Archive Project hides the project from your main list without deleting it. All Projects returns to Application Home.`,
        },
      ],
      result: `You can reach project-level commands without leaving the page.`,
      cautions: [
        `Archive Project hides the project. It does not delete anything. Restore it from Open Archived on Application Home.`,
      ],
      related: [`project-home-rename`, `project-home-back-forward`],
    },
    {
      id: `project-home-rename`,
      title: `Rename the open project`,
      purpose: `Change the title of the project you are working in.`,
      level: `Beginner`,
      prerequisites: [`A project is open.`],
      steps: [
        {
          screen: `project-menu`,
          do: `Click Project in the toolbar and choose Rename….`,
          explain: `Rename… is the first item in the group below Settings….`,
        },
        {
          screen: `project-rename-sheet`,
          do: `The Rename Project sheet opens. Edit the title.`,
          explain: `The sheet says: Renaming changes the name shown everywhere. The project folder and its contents stay where they are.`,
        },
        {
          screen: `project-rename-empty`,
          do: `If you empty the box, read the message under it.`,
          explain: `The sheet says A title is required. Type a title to continue.`,
        },
        {
          screen: `project-rename-sheet`,
          do: `Click Rename. Cancel closes the sheet and keeps the old title.`,
          explain: `The new title shows in the sidebar, on the page and on the project card.`,
        },
      ],
      result: `The project has its new title everywhere.`,
      cautions: [
        `Renaming changes only the name OpenFrame shows. The project folder keeps its place on disk.`,
      ],
      related: [`project-home-project-menu`],
    },
    {
      id: `project-home-back-forward`,
      title: `Go back, go forward and return to all projects`,
      purpose: `Move between the pages you have visited, or leave the project.`,
      level: `Beginner`,
      prerequisites: [`A project is open and you have visited more than one page.`],
      steps: [
        {
          screen: `project-home`,
          do: `Click Back in the toolbar, or press Command-[.`,
          explain: `Back returns to the page you were on before. Forward is greyed out until you have gone back.`,
        },
        {
          screen: `project-home-after-back`,
          do: `Look at the toolbar after going back.`,
          explain: `Forward is now available. Click it, or press Command-], to return to the page you left.`,
        },
        {
          screen: `project-home-after-back`,
          do: `Click All Projects in the toolbar to leave the project.`,
          explain: `You return to Application Home. The Project menu in the toolbar is labelled Project: Night Bus. Switch or manage projects.`,
        },
      ],
      result: `You can move through your pages and return to Application Home.`,
      cautions: [],
      related: [`project-home-hide-sidebar`],
    },
    {
      id: `project-home-hide-sidebar`,
      title: `Hide and show the sidebar`,
      purpose: `Give the page more room, and still reach every page.`,
      level: `Beginner`,
      prerequisites: [`A project is open.`],
      steps: [
        {
          screen: `project-home`,
          do: `Click Hide Sidebar at the left of the toolbar, or press Control-Command-S.`,
          explain: `The sidebar closes and the page grows wider.`,
        },
        {
          screen: `project-home-sidebar-hidden`,
          do: `Look at the page with the sidebar hidden.`,
          explain: `The button now reads Show Sidebar. QUICK ACCESS gains the pages the sidebar held: Idea Vault, Beat Board, Screenplay, Breakdown, Production and Call Sheets. Click Show Sidebar to bring the sidebar back.`,
        },
      ],
      result: `The sidebar is hidden or shown as you chose, and every page is still reachable.`,
      cautions: [],
      related: [`project-home-tour`],
    },
    {
      id: `project-home-undo-after-import`,
      title: `Undo an import from the message at the bottom`,
      purpose: `Read the message after a screenplay import and take the import back if you want.`,
      level: `Intermediate`,
      prerequisites: [`You have just imported a screenplay into the project.`],
      steps: [
        {
          screen: `home-import-toast`,
          do: `After the import, look at the bottom of the window.`,
          explain: `A message says Imported screenplay “LOST PROPERTY”, with Undo and Dismiss. Behind it the Screenplay page shows the imported scenes. The sidebar and toolbar stay as they were.`,
        },
        {
          screen: `home-import-toast`,
          do: `Click Undo to take the import back, or Dismiss to close the message.`,
          explain: `Undo takes the import back. Dismiss closes the message and keeps the import. The import also appears in RECENT on Project Home.`,
        },
      ],
      result: `The screenplay stays, or the import is undone.`,
      cautions: [
        `The message is only a short reminder. To take an import back later, use Edit > Undo Project Change (Option-Command-Z).`,
      ],
      related: [`project-home-tour`],
    },
    {
      id: `project-home-trial-reminder`,
      title: `Read the free trial reminder`,
      purpose: `Know how the 7-day free trial ends.`,
      level: `Beginner`,
      prerequisites: [`The free trial is running.`],
      steps: [
        {
          screen: `ai-sim-trial-reminder`,
          do: `Read the reminder. Click Continue trial or Buy full version.`,
          explain: `Every new install starts a 7-day free trial with the whole app. The reminder says how many days are left. Your projects stay on your computer either way, and nothing is deleted when the trial ends.`,
        },
      ],
      result: `You decided whether to buy now or keep trying.`,
      cautions: [],
      related: [],
    },
  ],
  screens: {
    'ai-sim-trial-reminder': `The Trial reminder dialog: Your free trial ends in 3 days, with Continue trial and Buy full version. Used in project-home-trial-reminder.`,
    'project-home': `Project Home of Night Bus. The sidebar lists Home to Files, the toolbar has Hide Sidebar, All Projects, Back, Forward, Project, Search, New, Assistant and Settings, and the page shows Status: Idea, Project settings, QUICK ACCESS, Project Files and RECENT. Used in project-home-tour, project-home-set-status and project-home-back-forward.`,
    'project-home-empty': `Project Home of a new, empty project. It says Start with your ideas, build your story, or open a screenplay and offers three cards: Open Idea Vault, Build Story and Write/Import Screenplay. Used in project-home-tour.`,
    'project-menu': `The Project menu from the toolbar: Status, Project Settings…, Settings…, Rename…, Duplicate Project, Show in Finder, Notes and Tasks, Activity, Recently Deleted, Archive Project, Search and Commands… and All Projects. Used in project-home-project-menu and project-home-rename.`,
    'project-new-menu': `The New menu in the toolbar, headed Create: New Idea, New Act, New Sequence, New Beat, New Scene Card, New Character, New Location, New Shot List, New Moodboard, Import Screenplay…, New Note, New Task and New Project…. Used in project-home-create-from-new.`,
    'project-home-status-menu': `The Status menu open on Project Home, with Idea, Development, Writing, Rewrite, Shooting Draft, Pre-Production, Shoot Preparation and Shooting. Used in project-home-set-status.`,
    'project-rename-sheet': `The Rename Project sheet. A note says renaming changes the name shown everywhere and the folder stays where it is. Cancel and Rename are at the bottom. Used in project-home-rename.`,
    'project-rename-empty': `The Rename Project sheet with the title cleared. It says A title is required. Variant, part of project-home-rename.`,
    'project-home-sidebar-hidden': `Project Home with the sidebar hidden. The button reads Show Sidebar, and QUICK ACCESS lists Idea Vault, Beat Board, Screenplay, Breakdown, Production and Call Sheets. Used in project-home-hide-sidebar.`,
    'project-home-after-back': `Project Home after Back. Forward is now available in the toolbar. Used in project-home-back-forward.`,
    'home-import-toast': `The Screenplay page after an import, with the message Imported screenplay “LOST PROPERTY” and Undo and Dismiss at the bottom. Used in project-home-undo-after-import.`,
  },
  shortcuts: [
    [`Command-[`, `Back`],
    [`Command-]`, `Forward`],
    [`Control-Command-S`, `Hide or show the sidebar`],
    [`Command-K`, `Search and Commands`],
    [`Shift-Command-,`, `Project Settings`],
    [`Shift-Command-W`, `Close Project`],
    [`Control-Command-1`, `Go to Home`],
  ],
  limits: [
    `Project Home shows the project you have open. To see all your projects, use All Projects in the toolbar.`,
    `Project Files holds documents you add. It does not sync them anywhere.`,
    `The project is stored on this Mac. The sidebar says Project stored locally on this Mac.`,
  ],
  troubleshooting: [
    {
      problem: `The sidebar is gone.`,
      fix: `Click Show Sidebar in the toolbar, or press Control-Command-S.`,
    },
    {
      problem: `I cannot find the other projects.`,
      fix: `Click All Projects in the toolbar to return to Application Home.`,
    },
    {
      problem: `Rename will not go through.`,
      fix: `The title box is empty and the sheet shows A title is required. Type a title.`,
    },
    {
      problem: `The status I want is missing.`,
      fix: `The Status menu offers Idea, Development, Writing, Rewrite, Shooting Draft, Pre-Production, Shoot Preparation and Shooting. Pick the closest one.`,
    },
  ],
};
