export default {
  area: `Activity`,
  slug: `activity`,
  order: 21,
  part: `Project and data`,
  intro: [
    `Activity is the project's logbook. The page is called Activity History and asks What important actions have happened in this project? It is read-only: you look at it, you do not edit it.`,
    `Each line shows when something happened and who did it, for example that you created the project or imported a screenplay. Four filters narrow the list to a kind of action: Drafts, Story, Production and Exchange. All shows everything.`,
    `Open Activity from Activity on Project Home, or from the Go menu.`,
  ],
  layout: [
    { name: `Header`, text: `The title Activity History and the line What important actions have happened in this project? Read-only.` },
    { name: `Filter bar`, text: `Five choices in one row: All, Drafts, Story, Production and Exchange. The chosen one is highlighted.` },
    { name: `Activity list`, text: `One row per action. The left side shows the day and time, such as Today 3:58 AM. The right side says who did what, with the name of the thing in quotation marks.` },
    { name: `Empty state`, text: `A filter with nothing to show says No major activity yet. and Nothing of this kind has happened yet.` },
  ],
  workflows: [
    {
      id: `activity-read`,
      title: `Read the project's activity`,
      purpose: `See the important actions that have happened in the project, in order.`,
      level: `Beginner`,
      prerequisites: [`A project is open.`],
      steps: [
        {
          screen: `activity`,
          do: `On Project Home click Activity, or choose Activity in the Go menu.`,
          explain: `The page is titled Activity History. With All chosen it lists every recorded action. In the picture the list shows that the project was created and that a screenplay called LOST PROPERTY was imported, each with the day and time and the name of the person who did it.`,
        },
        {
          screen: `activity`,
          do: `Read down the list.`,
          explain: `The page is read-only. There is nothing to type and no way to change an entry. A message at the bottom, for example Imported screenplay “LOST PROPERTY” with Undo, appears when you have just made a change.`,
        },
      ],
      result: `You can see what happened in the project and when.`,
      cautions: [
        `Activity lists important actions, as the page says, not every small edit.`,
      ],
      related: [`activity-filter`, `activity-undo`],
    },
    {
      id: `activity-filter`,
      title: `Filter the activity by kind`,
      purpose: `Show only draft, story, production or exchange activity.`,
      level: `Beginner`,
      prerequisites: [`Activity History is open.`],
      steps: [
        {
          screen: `pages-activity-drafts`,
          do: `Click Drafts.`,
          explain: `Only actions about screenplay drafts are listed. The imported screenplay is a draft action, so its line stays. The filter is highlighted.`,
        },
        {
          screen: `pages-activity-story`,
          do: `Click Story.`,
          explain: `Only story actions are listed. With none yet, the page says No major activity yet. Nothing of this kind has happened yet.`,
        },
        {
          screen: `pages-activity-production`,
          do: `Click Production.`,
          explain: `Only production planning actions are listed. In the picture there are none, so the same empty message shows.`,
        },
        {
          screen: `pages-activity-exchange`,
          do: `Click Exchange.`,
          explain: `Only actions about packages you export, import or review are listed, such as sending a review package or importing a response. Click All to see everything again.`,
        },
      ],
      result: `The list shows only the kind of action you chose.`,
      cautions: [
        `If you cannot find an action, click All. A filter hides every other kind.`,
      ],
      related: [`activity-read`],
    },
    {
      id: `activity-undo`,
      title: `Undo a recent project change`,
      purpose: `Take back the last change from the message that appears, or from the Edit menu.`,
      level: `Intermediate`,
      prerequisites: [`You have just made a change in the project.`],
      steps: [
        {
          screen: `activity`,
          do: `Look for the dark message at the bottom of the window and click Undo.`,
          explain: `The message names what happened, such as Imported screenplay “LOST PROPERTY”. Dismiss closes it without undoing.`,
        },
        {
          screen: `activity`,
          do: `Or open the Edit menu and choose Undo Project Change. The shortcut is Command+Option+Z. Redo Project Change is Command+Option+Shift+Z.`,
          explain: `Command+Z on its own undoes typing in a text box. Use the Project Change items to undo an action in the project itself.`,
        },
      ],
      result: `The last project change is taken back.`,
      cautions: [
        `The message disappears after a short time. The Edit menu is always there.`,
      ],
      related: [`activity-read`],
    },
  ],
  screens: {
    'activity': `Activity History with the filter All chosen. Two rows show that the project was created and that the screenplay LOST PROPERTY was imported, each with the day and time and who did it. A message at the bottom offers Undo and Dismiss. Used in activity-read and activity-undo.`,
    'pages-activity-drafts': `Activity History with Drafts chosen. The screenplay import line is listed. Used in activity-filter.`,
    'pages-activity-story': `Activity History with Story chosen. The page says No major activity yet and Nothing of this kind has happened yet. Used in activity-filter.`,
    'pages-activity-production': `Activity History with Production chosen. The same empty message shows. Used in activity-filter.`,
    'pages-activity-exchange': `Activity History with Exchange chosen. The same empty message shows. Used in activity-filter.`,
  },
  shortcuts: [
    [`Command+Option+Z`, `Undo Project Change`],
    [`Command+Option+Shift+Z`, `Redo Project Change`],
  ],
  limits: [
    `Activity is read-only. You cannot edit or delete a line.`,
    `It lists important actions in this project, not every edit.`,
    `Activity is kept in the project on your Mac. It is not sent anywhere.`,
  ],
  troubleshooting: [
    {
      problem: `The page says No major activity yet.`,
      fix: `Either nothing of that kind has happened, or a filter is hiding it. Click All.`,
    },
    {
      problem: `I cannot change or remove a line.`,
      fix: `Activity is a read-only record. To take a change back, use Undo Project Change in the Edit menu.`,
    },
    {
      problem: `The Undo message went away.`,
      fix: `Choose Undo Project Change in the Edit menu, or press Command+Option+Z.`,
    },
  ],
};
