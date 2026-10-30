export default {
  area: `Activity`,
  slug: `activity`,
  order: 26,
  part: `Project and data`,
  intro: [
    `Activity is a read-only history of the important things that have happened in your project. Each line says when it happened, who did it and what they did, for example Maya Iyer created a call sheet for Shoot Day 2. The page is titled Activity History and asks: What important actions have happened in this project?`,
    `Only meaningful actions are listed: drafts created or imported, ideas added, files and folders added or deleted, notes and tasks, schedule and call sheet changes, budget lines. Ordinary typing is never logged. The newest action is at the top.`,
    `You cannot change or delete anything here. Where the thing an action touched still exists, the line has an Open button that takes you to it.`,
    `Open the page from the project menu (click the project name at the top and choose Activity), or with the Activity button in the project status panel at the bottom of the window.`,
  ],
  layout: [
    { name: `Header`, text: `The title Activity History and the line What important actions have happened in this project? Read-only. At the right, the filter switch: All, Drafts, Story, Production and Exchange.` },
    { name: `Activity list`, text: `One line per action. At the left, the day and time, such as Today 12:31 am. Then the name of the person in bold and what they did. Lines about something that still exists have an Open button at the right.` },
    { name: `Show older activity`, text: `A button at the bottom of a long list. It loads older lines.` },
  ],
  workflows: [
    {
      id: `activity-read-history`,
      title: `Read what has happened in your project`,
      purpose: `See the recent important actions, newest first, and who did them.`,
      level: `Beginner`,
      prerequisites: [`A project is open.`],
      steps: [
        {
          screen: `activity-new-project`,
          do: `Click the project name at the top of the window and choose Activity.`,
          explain: `In a brand new project the list holds one line, Maya Iyer created project “Empty Short”, with the day and time. The status bar at the bottom reads Activity.`,
        },
        {
          screen: `activity-all`,
          do: `Look at the list in a project that has been worked on.`,
          explain: `Each line begins with the time and the name of the person, then says what they did: for example deleted task, completed task, added note, created folder, renamed file, added budget line, created a call sheet for Shoot Day 2 or added Weather to a call sheet. All is selected, so you see everything. The newest lines are at the top.`,
        },
      ],
      result: `You can see who did what and when.`,
      cautions: [
        `The list is read-only. To undo a change, use Undo in the message that appeared, or the Edit section of the project menu. See the Project window and navigation chapter.`,
      ],
      related: [`activity-filter`, `activity-open-item`],
    },
    {
      id: `activity-filter`,
      title: `Filter the history by kind of work`,
      purpose: `Show only drafts, story, production or exchange activity.`,
      level: `Beginner`,
      prerequisites: [`The Activity page is open.`],
      steps: [
        {
          screen: `activity-filter-drafts`,
          do: `Click Drafts at the top right.`,
          explain: `Only screenplay actions remain: imported screenplay, created draft, edited the screenplay.`,
        },
        {
          screen: `activity-filter-story`,
          do: `Click Story.`,
          explain: `Story shows Beat Board and Idea Vault actions: ideas added, pinned, flagged or deleted. For example added note “Opening image”, pinned “Opening image” and deleted “Old title ideas”.`,
        },
        {
          screen: `activity-filter-production`,
          do: `Click Production.`,
          explain: `Production shows Breakdown, locations, cast and crew, moodboards, storyboards, shot lists, the schedule, call sheets and the budget. For example added budget line, created Shoot Day 1 and added “Night bus” to Scene 1.`,
        },
        {
          screen: `activity-filter-exchange`,
          do: `Click Exchange.`,
          explain: `Exchange shows packages and imports and exports. When nothing of this kind has happened the page says No major activity yet. and Nothing of this kind has happened yet.`,
        },
        {
          screen: `activity-all`,
          do: `Click All to see every line again.`,
          explain: `Files, folders, notes and tasks appear only under All, because they do not belong to one of the four groups.`,
        },
      ],
      result: `The list shows only the kind of work you picked.`,
      cautions: [
        `If a filter looks empty, the page says so. Try All.`,
      ],
      related: [`activity-read-history`],
    },
    {
      id: `activity-open-item`,
      title: `Jump from a line to the item it is about`,
      purpose: `Go straight to the call sheet, moodboard, Breakdown scene or other item an action touched.`,
      level: `Beginner`,
      prerequisites: [`The Activity page shows a line with an Open button.`],
      steps: [
        {
          screen: `activity-filter-production`,
          do: `Find a line that has an Open button at the right, for example Maya Iyer created a call sheet for Shoot Day 2.`,
          explain: `Open appears only while the item still exists. Lines with no Open button are about something that has no page of its own, or has been deleted.`,
        },
        {
          screen: `activity-filter-production`,
          do: `Click Open.`,
          explain: `The project opens the page the action touched, here the call sheet. Use Back at the top of the window to return to Activity.`,
        },
      ],
      result: `You are on the item the action was about.`,
      cautions: [
        `If an item was deleted, its line has no Open button. Look for it in Recently Deleted.`,
      ],
      related: [`activity-read-history`],
    },
    {
      id: `activity-older`,
      title: `See older activity`,
      purpose: `Load lines further back than the first page of the list.`,
      level: `Intermediate`,
      prerequisites: [`The project has a long history.`],
      steps: [
        {
          screen: `activity-all`,
          do: `Scroll to the bottom of a long list and click Show older activity.`,
          explain: `The page first shows the latest 200 lines. The button adds 200 more each time you click it, up to 1,000 lines in all. When the button no longer appears, or is greyed out, there is nothing more to load.`,
        },
      ],
      result: `Older lines are added to the bottom of the list.`,
      cautions: [
        `Activity shows the latest 1,000 lines at most. The filter works on the lines that are loaded.`,
      ],
      related: [`activity-filter`],
    },
  ],
  screens: {
    'activity-new-project': `Activity History in a brand new project. The only line is Maya Iyer created project “Empty Short”. Used in activity-read-history.`,
    'activity-all': `Activity History with All selected: lines for tasks, notes, files, folders, the budget, call sheets and more, newest first, some with an Open button. Used in activity-read-history, activity-filter and activity-older.`,
    'activity-filter-drafts': `The Drafts filter. Only screenplay actions are shown: imported, created draft, edited. Used in activity-filter.`,
    'activity-filter-story': `The Story filter. It shows Beat Board and Idea Vault actions such as added note, pinned and deleted. Used in activity-filter.`,
    'activity-filter-production': `The Production filter. It shows Breakdown, locations, cast and crew, moodboards, storyboards, the schedule, call sheets and budget actions, with Open buttons on many lines. Used in activity-filter and activity-open-item.`,
    'activity-filter-exchange': `The Exchange filter with nothing to show. The page says No major activity yet. and Nothing of this kind has happened yet. Used in activity-filter.`,
  },
  shortcuts: [],
  limits: [
    `Activity is read-only. You cannot edit, delete or export it.`,
    `Only important actions are listed. Ordinary typing is never logged.`,
    `The page loads the latest 200 lines and can load up to 1,000.`,
    `Open appears only while the item an action touched still exists.`,
  ],
  troubleshooting: [
    {
      problem: `I cannot find a file or note action under Drafts, Story, Production or Exchange.`,
      fix: `Files, folders, notes and tasks are listed under All only.`,
    },
    {
      problem: `A line has no Open button.`,
      fix: `The item it is about was deleted or has no page of its own. For a deleted item, open Recently Deleted and click Restore.`,
    },
    {
      problem: `The page says No major activity yet.`,
      fix: `Nothing of that kind has happened in this project. Click All to see everything, or do some work and come back.`,
    },
    {
      problem: `I want to undo something I see in the list.`,
      fix: `Activity cannot undo. Use Undo in the project menu right after the change, or restore the item from Recently Deleted.`,
    },
  ],
};
