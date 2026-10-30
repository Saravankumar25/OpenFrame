export default {
  area: `Recently Deleted`,
  slug: `recently-deleted`,
  order: 22,
  part: `Project and data`,
  intro: [
    `Recently Deleted is the safety net of the project. When you delete something, it does not vanish at once. It waits here for 30 days. Restore it before then and it goes back where it came from, or delete it for good sooner.`,
    `The page has two lists, chosen with a switch at the top: This project and Global Idea Vault. The first holds items deleted from this project, such as notes, tasks, files, folders and ideas. The second holds items deleted from the Global Idea Vault that all your projects share.`,
    `Open Recently Deleted from Recently Deleted on Project Home, or from the Go menu. The Idea Vault has its own Recently Deleted button too.`,
  ],
  layout: [
    { name: `Header`, text: `The title Recently Deleted and the line: Deleted items stay here for 30 days, then they are removed for good. Restore them before then, or delete them sooner.` },
    { name: `Which list`, text: `A switch with two choices: This project and Global Idea Vault.` },
    { name: `Search deleted items`, text: `A box to find an item by what you type, with a count of the items in the list.` },
    { name: `Deleted list`, text: `A table with the columns Item, Type, Deleted, Removed in and Was in. Each row has Restore and Delete Permanently.` },
    { name: `Footer note`, text: `Delete Permanently asks for a second, deliberate confirmation. If the original place no longer exists, Restore puts the item in an Unassigned area and tells you.` },
  ],
  workflows: [
    {
      id: `recently-deleted-open`,
      title: `Open Recently Deleted and choose a list`,
      purpose: `Find deleted items for this project or for the Global Idea Vault.`,
      level: `Beginner`,
      prerequisites: [`A project is open.`],
      steps: [
        {
          screen: `recently-deleted-empty`,
          do: `On Project Home click Recently Deleted, or choose Recently Deleted in the Go menu.`,
          explain: `With nothing deleted, the page says Nothing has been deleted. and When you delete something it stays here for 30 days, and you can restore it until then. This project is chosen.`,
        },
        {
          screen: `pages-deleted-global-empty`,
          do: `Click Global Idea Vault in the switch.`,
          explain: `This list holds items deleted from the Global Idea Vault. It also says Nothing has been deleted. when it is empty. Click This project to go back.`,
        },
      ],
      result: `You are looking at the list you want.`,
      cautions: [
        `Items deleted from the Global Idea Vault are not in the This project list. Check both lists if you cannot find something.`,
      ],
      related: [`recently-deleted-read-list`, `recently-deleted-restore`],
    },
    {
      id: `recently-deleted-read-list`,
      title: `Read the list of deleted items`,
      purpose: `Understand each column and how long an item has left.`,
      level: `Beginner`,
      prerequisites: [`Something has been deleted from the project.`],
      steps: [
        {
          screen: `pages-deleted-items`,
          do: `Read the table.`,
          explain: `Item is the name. Type says what it was, for example Task or Note. Deleted says when and by whom, such as Today by your name. Removed in counts down the days left, for example 30 days left. Was in names the place it came from, for example Notes & Tasks.`,
        },
        {
          screen: `pages-deleted-items`,
          do: `Look at the message at the bottom.`,
          explain: `Right after a deletion a message says, for example, Moved task “Call the second bus company” to Recently Deleted. Click Undo in it to put the item straight back.`,
        },
      ],
      result: `You know what each deleted item is, where it came from and how many days it has left.`,
      cautions: [
        `When the days left reach zero the item is removed for good and cannot be restored.`,
      ],
      related: [`recently-deleted-restore`, `recently-deleted-search`],
    },
    {
      id: `recently-deleted-search`,
      title: `Search deleted items`,
      purpose: `Find one deleted item among many.`,
      level: `Beginner`,
      prerequisites: [`There are deleted items.`],
      steps: [
        {
          screen: `pages-deleted-search`,
          do: `Click Search deleted items and type, for example, catering.`,
          explain: `The table keeps only the matching rows. Here it shows Old catering list, a Note that was in Notes & Tasks, with 30 days left.`,
        },
        {
          screen: `pages-deleted-search-none`,
          do: `If nothing matches, read the message.`,
          explain: `It says No deleted items match followed by what you typed, for example “lighting”. Clear the box to see everything again.`,
        },
      ],
      result: `You find the item, or you know it is not in this list.`,
      cautions: [
        `The search covers the list you chose. Switch to Global Idea Vault to search that one.`,
      ],
      related: [`recently-deleted-restore`],
    },
    {
      id: `recently-deleted-restore`,
      title: `Restore a deleted item`,
      purpose: `Put a deleted item back where it came from.`,
      level: `Beginner`,
      prerequisites: [`The item is in Recently Deleted and has days left.`],
      steps: [
        {
          screen: `pages-deleted-items`,
          do: `Click Restore at the end of the item's row.`,
          explain: `Restore is the button next to Delete Permanently.`,
        },
        {
          screen: `pages-deleted-restored`,
          do: `Read the message.`,
          explain: `It says, for example, Restored “Old catering list” to Notes & Tasks. The row leaves the list and the count goes down. If the original place no longer exists, the item goes to an Unassigned area and the message tells you.`,
        },
      ],
      result: `The item is back in the place it came from.`,
      cautions: [
        `If the original folder or list was deleted too, the item is put in an Unassigned area instead. The message says so.`,
      ],
      related: [`recently-deleted-purge`],
    },
    {
      id: `recently-deleted-purge`,
      title: `Delete an item permanently`,
      purpose: `Remove an item for good before its 30 days are up.`,
      level: `Intermediate`,
      prerequisites: [`The item is in Recently Deleted and you are sure you will not need it.`],
      steps: [
        {
          screen: `pages-deleted-items`,
          do: `Click Delete Permanently on the item's row.`,
          explain: `Nothing is removed yet. OpenFrame asks a second time.`,
        },
        {
          screen: `pages-deleted-purge-confirm`,
          do: `Read the question Delete “Unused title idea” permanently? and choose.`,
          explain: `It says the note will be removed from the project for good, that this can't be undone and that it can't be restored afterwards. Anything that was created from it elsewhere in the project is not affected. Delete Permanently removes it. Keep in Recently Deleted leaves it in the list.`,
        },
      ],
      result: `The item is gone for good, or it stays in Recently Deleted if you chose to keep it.`,
      cautions: [
        `This cannot be undone. When in doubt, choose Keep in Recently Deleted. The item leaves by itself after 30 days.`,
      ],
      related: [`recently-deleted-restore`],
    },
  ],
  screens: {
    'recently-deleted-empty': `Recently Deleted with This project chosen and nothing deleted. The page says Nothing has been deleted. and explains the 30 days and the second confirmation. Used in recently-deleted-open.`,
    'pages-deleted-global-empty': `Recently Deleted with Global Idea Vault chosen. Nothing has been deleted from it. Used in recently-deleted-open.`,
    'pages-deleted-items': `Recently Deleted with a list of deleted items. A row shows the Task Call the second bus company, deleted today, with 30 days left, from Notes & Tasks, and the buttons Restore and Delete Permanently. A message offers Undo. Used in recently-deleted-read-list, recently-deleted-restore and recently-deleted-purge.`,
    'pages-deleted-search': `Search deleted items with catering typed. The row Old catering list, a Note from Notes & Tasks, remains. Used in recently-deleted-search.`,
    'pages-deleted-search-none': `Search deleted items with lighting typed. The page says No deleted items match “lighting”. Used in recently-deleted-search.`,
    'pages-deleted-purge-confirm': `The confirmation Delete “Unused title idea” permanently? It says the note will be removed for good and cannot be restored. Buttons are Delete Permanently and Keep in Recently Deleted. Used in recently-deleted-purge.`,
    'pages-deleted-restored': `Recently Deleted after a restore. A message says Restored “Old catering list” to Notes & Tasks, and the item has left the list. Used in recently-deleted-restore.`,
  },
  shortcuts: [
    [`Command+Option+Z`, `Undo Project Change`],
  ],
  limits: [
    `Deleted items are kept for 30 days. After that they are removed for good.`,
    `This project and Global Idea Vault are separate lists.`,
    `Delete Permanently cannot be undone.`,
    `Restore puts an item back in the place it came from. If that place is gone, it goes to an Unassigned area.`,
  ],
  troubleshooting: [
    {
      problem: `I cannot find something I deleted.`,
      fix: `Check the switch. Items from the Global Idea Vault are in the Global Idea Vault list. Also try the search box, and remember items older than 30 days are gone.`,
    },
    {
      problem: `The item was restored but I cannot see it where it was.`,
      fix: `Its original place was deleted. It was put in an Unassigned area, and the message after Restore said so.`,
    },
    {
      problem: `I clicked Delete Permanently by mistake.`,
      fix: `Choose Keep in Recently Deleted in the confirmation. Nothing is removed until you confirm.`,
    },
  ],
};
