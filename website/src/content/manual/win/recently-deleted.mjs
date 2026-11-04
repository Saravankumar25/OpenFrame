export default {
  area: `Recently Deleted`,
  slug: `recently-deleted`,
  order: 27,
  part: `Project and data`,
  intro: [
    `Recently Deleted is where deleted things wait before they are gone for good. When you delete a file, a folder, a note, a task, an idea or a template that lives in the project, Straft does not destroy it. It moves it here for 30 days. The page says: Deleted items stay here for 30 days, then they are removed for good. Restore them before then, or delete them sooner.`,
    `For each item the page shows what it is, when and by whom it was deleted, how many days are left and where it was. You can restore an item to its place, or delete it permanently after a second confirmation.`,
    `A switch at the top right chooses what you look at. This project lists what was deleted from the open project. Global Idea Vault lists what was deleted from your vault of ideas outside any project.`,
    `Open the page from the project menu (click the project name at the top and choose Recently Deleted) or from QUICK ACCESS on Project Home. The Global Idea Vault also has its own Recently Deleted button.`,
  ],
  layout: [
    { name: `Header`, text: `The title Recently Deleted and the 30-day line. At the right, the switch with This project and Global Idea Vault.` },
    { name: `Search`, text: `A Search deleted items box under the title, and a count at the right, such as 6 items or 1 item.` },
    { name: `Deleted items table`, text: `Columns ITEM, TYPE, DELETED, REMOVED IN, WAS IN and ACTIONS. DELETED shows the day and who deleted it. REMOVED IN counts the days left, such as 30 days left. WAS IN says where the item came from, for example Notes & Tasks, Files or Idea Vault. Each row has Restore and Delete Permanently.` },
    { name: `Notice`, text: `A yellow bar under the table: Delete Permanently asks for a second, deliberate confirmation. If the original place no longer exists, Restore puts the item in an Unassigned area and tells you.` },
  ],
  workflows: [
    {
      id: `recently-deleted-open`,
      title: `Open Recently Deleted and read the list`,
      purpose: `Find what has been deleted and see how long it stays.`,
      level: `Beginner`,
      prerequisites: [`A project is open.`],
      steps: [
        {
          screen: `trash-empty`,
          do: `Click the project name at the top of the window and choose Recently Deleted.`,
          explain: `If nothing has been deleted, the page says Nothing has been deleted. and When you delete something it stays here for 30 days, and you can restore it until then. The notice at the bottom is always there. This project is selected.`,
        },
        {
          screen: `trash-filled`,
          do: `Look at a project where things were deleted.`,
          explain: `Six items are listed, for example a task, a note, a folder, two files and an idea. TYPE says what each is (Task, Note, Folder, File, Idea Vault item). WAS IN says where it was (Notes & Tasks, Files, Idea Vault). Hover over the DELETED time to see the exact date and time. REMOVED IN shows 30 days left, which counts down to zero.`,
        },
      ],
      result: `You can see every deleted item, who deleted it and when it will be removed.`,
      cautions: [
        `After the days run out the item is removed for good, with no further warning. Restore what you need before then.`,
      ],
      related: [`recently-deleted-restore`, `recently-deleted-search`],
    },
    {
      id: `recently-deleted-undo-first`,
      title: `Undo a delete from the message, or restore it later`,
      purpose: `Know the two ways to bring back something you just deleted.`,
      level: `Beginner`,
      prerequisites: [`You have just deleted a file, folder, note, task or template in the project.`],
      steps: [
        {
          screen: `trash-filled`,
          do: `Right after a delete, click Undo in the message at the bottom of the window.`,
          explain: `Every delete in the project shows a message such as Moved task “Tour task call the depot” to Recently Deleted, with Undo. Undo puts the item straight back where it was. You can also press Ctrl+Z.`,
        },
        {
          screen: `trash-filled`,
          do: `If the message has gone, open Recently Deleted and find the item in the list.`,
          explain: `The item stays in the list for 30 days whether or not you used Undo. The next workflow shows how to restore it.`,
        },
      ],
      result: `You know to use Undo at once, and that the list keeps the item for 30 days.`,
      cautions: [
        `The message goes away on its own or when you dismiss it. The item stays in the list.`,
      ],
      related: [`recently-deleted-restore`],
    },
    {
      id: `recently-deleted-restore`,
      title: `Restore a deleted item`,
      purpose: `Bring an item back to the place it was deleted from.`,
      level: `Beginner`,
      prerequisites: [`An item is in the Recently Deleted list.`],
      steps: [
        {
          screen: `trash-filled`,
          do: `Find the item and click Restore on its row.`,
          explain: `Restore is the white button at the right of the row. It is named after the item for screen readers, for example Restore Tour note on the radio tower.`,
        },
        {
          screen: `trash-filled`,
          do: `Read the message at the bottom.`,
          explain: `A message says Restored followed by the item name and where it went, for example to Notes & Tasks, with Undo. The row leaves the list and the item is back in its page.`,
        },
        {
          screen: `trash-empty`,
          do: `If the original place no longer exists, read the message carefully.`,
          explain: `The notice on this page says: If the original place no longer exists, Restore puts the item in an Unassigned area and tells you. The message then says that the original place no longer exists, so it was put in Unassigned. A screenplay draft that belongs to a deleted screenplay cannot be restored until the screenplay itself is restored.`,
        },
      ],
      result: `The item is back in the project, and the row has left Recently Deleted.`,
      cautions: [
        `If you restored by mistake, click Undo in the message.`,
      ],
      related: [`recently-deleted-delete-permanently`],
    },
    {
      id: `recently-deleted-search`,
      title: `Search the deleted items`,
      purpose: `Find one item in a long list.`,
      level: `Beginner`,
      prerequisites: [`The Recently Deleted list has items.`],
      steps: [
        {
          screen: `trash-search-filtered`,
          do: `Type a word in Search deleted items, for example Tour task.`,
          explain: `The table narrows to the matching items. Only Tour task call the depot is left. The count at the right still shows the whole list.`,
        },
        {
          screen: `trash-search-no-match`,
          do: `If nothing matches, the page says No deleted items match followed by what you typed.`,
          explain: `Here zzzz matches nothing. Clear the box to see all items again.`,
        },
      ],
      result: `The list shows only the matching deleted items.`,
      cautions: [
        `The search box looks at the list for the side you are on. Use the switch to look at the other side.`,
      ],
      related: [`recently-deleted-restore`],
    },
    {
      id: `recently-deleted-delete-permanently`,
      title: `Delete an item permanently`,
      purpose: `Remove an item for good before its 30 days are up.`,
      level: `Intermediate`,
      prerequisites: [`An item is in the Recently Deleted list and you are sure you do not need it.`],
      steps: [
        {
          screen: `trash-filled`,
          do: `Click Delete Permanently on the item's row.`,
          explain: `The red button is at the far right of the row. Nothing is deleted yet. A second dialog asks you to confirm.`,
        },
        {
          screen: `trash-purge-confirm`,
          do: `Read the dialog Delete “Old title ideas” permanently?`,
          explain: `It says the item will be removed from the project for good, that this can't be undone and that it can't be restored afterwards. It adds that anything that was created from it elsewhere in the project is not affected. Keep in Recently Deleted closes the dialog and keeps the item. The x does the same.`,
        },
        {
          screen: `trash-purge-confirm`,
          do: `Click Delete Permanently to confirm.`,
          explain: `The item is removed. A message says the item was deleted permanently.`,
        },
      ],
      result: `The item is gone and cannot be restored.`,
      cautions: [
        `This cannot be undone. If you are unsure, click Keep in Recently Deleted and leave the item for the 30 days.`,
      ],
      related: [`recently-deleted-restore`],
    },
    {
      id: `recently-deleted-global-vault`,
      title: `Work with deleted ideas in the Global Idea Vault`,
      purpose: `Restore or permanently delete ideas that were deleted outside any project.`,
      level: `Intermediate`,
      prerequisites: [`Something was deleted from the Global Idea Vault.`],
      steps: [
        {
          screen: `trash-global-empty`,
          do: `Click Global Idea Vault at the top right of the page.`,
          explain: `The switch has two sides: This project and Global Idea Vault. With nothing deleted from the vault the page says Nothing has been deleted.`,
        },
        {
          screen: `trash-global-filled`,
          do: `Look at the list for the vault.`,
          explain: `A deleted idea shows as an Idea Vault item with the day, 30 days left and Idea Vault under WAS IN. The vault keeps its own list and its own Undo, separate from the project.`,
        },
        {
          screen: `trash-global-purge-confirm`,
          do: `Click Restore to bring the idea back, or Delete Permanently, then confirm.`,
          explain: `The confirmation dialog is the same as in a project: Delete “Tour global idea” permanently? with Keep in Recently Deleted and Delete Permanently.`,
        },
        {
          screen: `trash-purged-toast`,
          do: `After you confirm, read the message.`,
          explain: `The message says “Tour global idea” was deleted permanently, with a x to close it. The list is empty again and says Nothing has been deleted.`,
        },
      ],
      result: `The idea is restored to the Global Idea Vault, or removed for good.`,
      cautions: [
        `The two sides are separate. An idea deleted from the vault does not show under This project.`,
      ],
      related: [`recently-deleted-delete-permanently`, `recently-deleted-restore`],
    },
  ],
  screens: {
    'trash-empty': `Recently Deleted with nothing in it. The page says Nothing has been deleted. and explains the 30 days. This project is selected. Used in recently-deleted-open and recently-deleted-restore.`,
    'trash-filled': `Recently Deleted in the project with six items: a task, a note, a folder, two files and an idea. Each row has TYPE, DELETED, REMOVED IN, WAS IN, Restore and Delete Permanently. Used in recently-deleted-open, recently-deleted-undo-first, recently-deleted-restore and recently-deleted-delete-permanently.`,
    'trash-search-filtered': `Search deleted items with Tour task typed. Only Tour task call the depot is left. Used in recently-deleted-search.`,
    'trash-search-no-match': `Search deleted items with text that matches nothing. The page says No deleted items match followed by the text. Used in recently-deleted-search.`,
    'trash-purge-confirm': `The dialog Delete “Old title ideas” permanently? It says the item can't be undone or restored, and offers Keep in Recently Deleted and Delete Permanently. Used in recently-deleted-delete-permanently.`,
    'trash-global-empty': `Recently Deleted with Global Idea Vault selected and nothing in it. Used in recently-deleted-global-vault.`,
    'trash-global-filled': `Recently Deleted with Global Idea Vault selected, listing one deleted idea, Tour global idea, with Restore and Delete Permanently. Used in recently-deleted-global-vault.`,
    'trash-global-purge-confirm': `The dialog Delete “Tour global idea” permanently? with Keep in Recently Deleted and Delete Permanently. Used in recently-deleted-global-vault.`,
    'trash-purged-toast': `The Global Idea Vault side after a permanent delete. A message says “Tour global idea” was deleted permanently and the list is empty. Used in recently-deleted-global-vault.`,
  },
  shortcuts: [
    [`Ctrl+Z`, `Undo the last change, for example a delete or a restore`],
  ],
  limits: [
    `Items stay in Recently Deleted for 30 days, then they are removed for good.`,
    `Delete Permanently cannot be undone and always asks for a second confirmation.`,
    `There is no Empty Recently Deleted button. Items are removed one at a time or when their 30 days run out.`,
    `This project and Global Idea Vault are two separate lists.`,
    `The project itself is not here. A deleted project goes to the Windows Recycle Bin. See the Application Home chapter.`,
  ],
  troubleshooting: [
    {
      problem: `I cannot find a deleted item.`,
      fix: `Check the switch at the top right. This project and Global Idea Vault are separate. Clear the Search deleted items box. If it was deleted more than 30 days ago, it has been removed for good.`,
    },
    {
      problem: `Restore put the item somewhere I did not expect.`,
      fix: `Its original place no longer existed, so it was put in Unassigned, and the message said so. Move it from there. Click Undo in the message to put it back in the list.`,
    },
    {
      problem: `A screenplay draft will not restore.`,
      fix: `Its screenplay is in Recently Deleted too. Restore the screenplay first, then the draft.`,
    },
    {
      problem: `The page says Recently Deleted couldn't be loaded.`,
      fix: `Click Try Again. Your deleted items are still stored.`,
    },
    {
      problem: `I clicked Delete Permanently and nothing happened.`,
      fix: `A second dialog asks for confirmation. Click Delete Permanently in the dialog to go ahead, or Keep in Recently Deleted to stop.`,
    },
  ],
};
