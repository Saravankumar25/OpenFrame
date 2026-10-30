export default {
  area: `Notes and tasks`,
  slug: `notes-and-tasks`,
  order: 25,
  part: `Project and data`,
  intro: [
    `Notes & Tasks is the page for small things that do not belong anywhere else: a reminder about the weather, a thought about the music, a call you must make before the shoot. The page says so itself: Small things that do not belong anywhere else.`,
    `It has two lists side by side. Project Notes, on the left, holds short notes you write and pin. Tasks, on the right, holds small to-do items with an optional due date, an optional owner and an optional link to the scene, location, character, shooting day or call sheet they are about.`,
    `Notes and tasks belong to the project and are never printed in formal documents. A note is kept out of screenplay and call-sheet exports. Use the Idea Vault for ideas and research, and use notes for everything else.`,
    `Open the page from the project menu (click the project name at the top) or from QUICK ACCESS on Project Home. New Note and New Task are also in the Create menu at the top, so you can add one from any page.`,
  ],
  layout: [
    { name: `Header`, text: `The title Notes & Tasks and the line Small things that do not belong anywhere else. At the right: New Note and New Task.` },
    { name: `Project Notes`, text: `The left list, with a Search notes box. Each note shows its title, the start of its text and when it was changed. A pinned note has a pin beside its title. A note without a title shows as Untitled note.` },
    { name: `Tasks`, text: `The right list. A count such as 3 open sits beside the heading. An owner list (Anyone, Me and the names of the people in the project) and an Open, Done and All switch filter the list. Each task has a tick box, its title, a chip for the item it is linked to and its due date, such as due 3 Nov.` },
    { name: `Note drawer`, text: `Opens on the right when you click a note. It is headed PROJECT NOTE and has Title, Note, who added it and when, Delete and Pin.` },
    { name: `Task drawer`, text: `Opens on the right for a new or existing task. It has Task *, Due date, Owner, Related to and Notes, with Cancel and Add Task (or Delete, Mark Done, Cancel and Save for an existing task).` },
  ],
  workflows: [
    {
      id: `notes-and-tasks-open-page`,
      title: `Open Notes & Tasks and start from an empty page`,
      purpose: `Reach the page and know what each empty list offers.`,
      level: `Beginner`,
      prerequisites: [`A project is open.`],
      steps: [
        {
          screen: `notes-empty`,
          do: `Click the project name at the top of the window and choose Notes & Tasks. You can also click Notes & Tasks under QUICK ACCESS on Project Home.`,
          explain: `Notes & Tasks is not in the left rail. The project menu and Project Home are the two ways in. The Project window and navigation chapter shows the menu.`,
        },
        {
          screen: `notes-empty`,
          do: `Read the two empty lists.`,
          explain: `Project Notes says No notes yet. and Add a note for something that doesn't naturally belong in Idea Vault or another workspace. Tasks says No tasks yet. and Add a small task only when you need to remember an action. Each has its own New Note or New Task button. The Tasks count reads 0 open. The owner list shows Anyone and Me, and Open is selected.`,
        },
      ],
      result: `You are on Notes & Tasks and can add your first note or task.`,
      cautions: [
        `Do not put ideas here. The Idea Vault is for ideas and research, with tags and collections.`,
      ],
      related: [`notes-and-tasks-write-note`, `notes-and-tasks-add-task`],
    },
    {
      id: `notes-and-tasks-write-note`,
      title: `Write a project note`,
      purpose: `Capture a short note and keep it with the project.`,
      level: `Beginner`,
      prerequisites: [`The Notes & Tasks page is open.`],
      steps: [
        {
          screen: `notes-filled`,
          do: `Click New Note at the top right, or New Note above the list.`,
          explain: `The page shows three notes, Monsoon schedule risk, Permits and Music, each with its first line and Today. You can also choose New Note in the Create menu at the top of the window.`,
        },
        {
          screen: `notes-new-note-editor`,
          do: `A new note opens in the drawer. Click the Title box.`,
          explain: `An Untitled note has already joined the list. The drawer is headed PROJECT NOTE and has Title and Note. Below them: Added by your name, with when it was created and changed, and the line Notes are kept out of formal documents such as screenplay and call-sheet exports.`,
        },
        {
          screen: `notes-new-note-filled`,
          do: `Type a title, then click the Note box and write the note.`,
          explain: `Here the title is Tour note on the radio tower. The note saves as you type, and the list updates with the new title and the start of the text. There is no Save button.`,
        },
        {
          screen: `notes-filled`,
          do: `Click the x at the top of the drawer to close it.`,
          explain: `The note stays in the Project Notes list. Click it any time to read or edit it again.`,
        },
      ],
      result: `A note with your title and text is saved in Project Notes.`,
      cautions: [
        `A note with no title shows in the list as Untitled note. Give it a title so you can find it.`,
        `Notes are never included in screenplay or call-sheet exports.`,
      ],
      related: [`notes-and-tasks-find-note`, `notes-and-tasks-pin-note`, `notes-and-tasks-delete-note`],
    },
    {
      id: `notes-and-tasks-find-note`,
      title: `Find a note`,
      purpose: `Narrow Project Notes to the notes that match a word.`,
      level: `Beginner`,
      prerequisites: [`The project has some notes.`],
      steps: [
        {
          screen: `notes-search-filtered`,
          do: `Type a word in Search notes, for example permit.`,
          explain: `The list narrows as you type. Only Permits remains. The Tasks list on the right is not affected by this box.`,
        },
        {
          screen: `notes-search-no-match`,
          do: `If nothing matches, the Project Notes list is empty for that word.`,
          explain: `Here zzzz matches nothing. Clear the box to see every note again.`,
        },
      ],
      result: `You find the note you wanted.`,
      cautions: [
        `This box searches notes only. To search the whole project, use the search box at the top of the window.`,
      ],
      related: [`notes-and-tasks-write-note`],
    },
    {
      id: `notes-and-tasks-pin-note`,
      title: `Pin a note so it stays on top`,
      purpose: `Keep an important note at the top of the list.`,
      level: `Beginner`,
      prerequisites: [`A note exists.`],
      steps: [
        {
          screen: `notes-note-editor`,
          do: `Click a note, for example Permits, then click Pin at the bottom of the drawer.`,
          explain: `The drawer shows the Title and the Note, the line Added by, Delete and Pin.`,
        },
        {
          screen: `notes-note-editor-pinned`,
          do: `Look at the list and the drawer.`,
          explain: `A pinned note shows a pin beside its title, as Monsoon schedule risk does here, and its button now reads Unpin. Click Unpin to remove the pin.`,
        },
      ],
      result: `The note carries a pin and stays at the top of Project Notes.`,
      cautions: [],
      related: [`notes-and-tasks-write-note`],
    },
    {
      id: `notes-and-tasks-delete-note`,
      title: `Delete a note and get it back`,
      purpose: `Remove a note you do not need, with a way to undo.`,
      level: `Beginner`,
      prerequisites: [`A note exists.`],
      steps: [
        {
          screen: `notes-new-note-filled`,
          do: `Open the note and click Delete at the bottom left of the drawer.`,
          explain: `There is no confirmation dialog, because the note is kept for 30 days.`,
        },
        {
          screen: `notes-note-deleted-toast`,
          do: `Read the message: Moved note “Tour note on the radio tower” to Recently Deleted.`,
          explain: `Click Undo to bring the note back at once, or Dismiss to close the message. The note leaves the list.`,
        },
      ],
      result: `The note is out of the list. Undo or Recently Deleted brings it back for 30 days.`,
      cautions: [
        `After 30 days the note is removed for good. See the Recently Deleted chapter.`,
      ],
      related: [`notes-and-tasks-write-note`],
    },
    {
      id: `notes-and-tasks-add-task`,
      title: `Add a task with a due date`,
      purpose: `Write down something you must do, with a day and an owner.`,
      level: `Beginner`,
      prerequisites: [`The Notes & Tasks page is open.`],
      steps: [
        {
          screen: `notes-filled`,
          do: `Click New Task at the top right, or New Task in the Create menu.`,
          explain: `The Tasks list shows three open tasks, for example Scout the east jetty at dusk, due 3 Nov.`,
        },
        {
          screen: `notes-task-editor-new`,
          do: `In the drawer, click the Task * box and type what must be done.`,
          explain: `The drawer is headed TASK and New Task. It has Task *, Due date, Owner, Related to and Notes. Cancel closes it without adding anything.`,
        },
        {
          screen: `notes-task-editor-title-error`,
          do: `If you click Add Task or leave the box with nothing typed, the message Give the task a short title. shows under it.`,
          explain: `A task needs a title. Type one and the message goes away.`,
        },
        {
          screen: `notes-task-editor-filled`,
          do: `Set a Due date, choose an Owner and add Notes if you need them.`,
          explain: `Due date opens a date picker. Owner lists No owner and the people in the project, with you marked (you). Notes is free text, for example Ask about the spare bus for the night shoot.`,
        },
        {
          screen: `notes-task-added-toast`,
          do: `Click Add Task.`,
          explain: `A message says Added task “Tour task call the depot” with Undo. The task joins the open list with its due date and, if you chose one, the chip for its related item. The count goes up by one.`,
        },
      ],
      result: `A new open task appears in the Tasks list.`,
      cautions: [
        `Click Undo in the message if you added the task by mistake.`,
      ],
      related: [`notes-and-tasks-link-task`, `notes-and-tasks-complete-task`],
    },
    {
      id: `notes-and-tasks-link-task`,
      title: `Link a task to a scene, location or other item`,
      purpose: `Connect a task to the item it is about, so you can jump to that item from the task.`,
      level: `Intermediate`,
      prerequisites: [`The task drawer is open for a new or existing task.`],
      steps: [
        {
          screen: `notes-task-editor-filled`,
          do: `Click the Related to box and type a few letters, for example bus.`,
          explain: `The box is labelled Find a scene, location, character, shooting day or call sheet. A note under it says: Optional. Completing the task never changes the related item.`,
        },
        {
          screen: `notes-task-editor-related-results`,
          do: `Read the Matching items list and click the item you mean.`,
          explain: `Each result shows its kind (Catalog Item, Location, Scene Card, Scene) and its name, and where it lives (Production, Story, or Screenplay with the draft). You see up to eight results. Type more letters to narrow them.`,
        },
        {
          screen: `notes-task-editor-related-chosen`,
          do: `Check the chosen item, then click Add Task.`,
          explain: `The item shows with a Remove button. Click Remove to choose a different one, or to link nothing.`,
        },
        {
          screen: `notes-task-added-toast`,
          do: `Look at the new task in the list.`,
          explain: `The task shows a chip such as Catalog Item: Night bus. Click the chip to open that item in the project.`,
        },
      ],
      result: `The task shows the item it is about, and the chip opens it.`,
      cautions: [
        `Completing the task never changes the linked item, and deleting the task never deletes it.`,
        `If a linked item is later deleted, the chip says (deleted) and no longer opens it.`,
      ],
      related: [`notes-and-tasks-add-task`, `notes-and-tasks-edit-task`],
    },
    {
      id: `notes-and-tasks-edit-task`,
      title: `Change a task`,
      purpose: `Edit the title, date, owner, link or notes of a task you already added.`,
      level: `Beginner`,
      prerequisites: [`At least one task exists.`],
      steps: [
        {
          screen: `notes-task-editor-existing`,
          do: `Click the title of a task in the list.`,
          explain: `The drawer opens with the task's details. It shows Task *, Due date, Owner, Related to and Notes, and at the bottom Delete, Mark Done, Cancel and Save.`,
        },
        {
          screen: `notes-task-editor-existing`,
          do: `Change what you need and click Save.`,
          explain: `Save is greyed out until you change something. A message says Edited task with Undo. Cancel closes the drawer without a change.`,
        },
      ],
      result: `The task shows your changes in the list.`,
      cautions: [
        `The title cannot be empty. The message Give the task a short title. shows if you clear it.`,
      ],
      related: [`notes-and-tasks-complete-task`, `notes-and-tasks-delete-task`],
    },
    {
      id: `notes-and-tasks-complete-task`,
      title: `Mark a task done, and reopen it`,
      purpose: `Tick off finished tasks and bring one back if it is not finished after all.`,
      level: `Beginner`,
      prerequisites: [`At least one open task exists.`],
      steps: [
        {
          screen: `notes-filled`,
          do: `Click the tick box at the left of a task.`,
          explain: `The tick box is named Mark followed by the task title done. You can also open the task and click Mark Done.`,
        },
        {
          screen: `notes-task-completed-toast`,
          do: `Read the message: Completed task “Tour task call the depot”.`,
          explain: `Click Undo to reopen it at once. The task leaves the Open list and the open count goes down by one.`,
        },
        {
          screen: `notes-tasks-done`,
          do: `Click Done above the list.`,
          explain: `Done tasks show Done in place of a date. Their tick boxes are ticked. Click a ticked box, or open the task and click Reopen, to make it an open task again. A message says Reopened task followed by the title.`,
        },
        {
          screen: `notes-tasks-all`,
          do: `Click All.`,
          explain: `All shows open and done tasks together. The Open, Done and All switch only changes what you see.`,
        },
      ],
      result: `Finished tasks are in Done and open ones stay in Open.`,
      cautions: [
        `Marking a task done never changes the scene, location or other item it is linked to.`,
      ],
      related: [`notes-and-tasks-filter-tasks`, `notes-and-tasks-add-task`],
    },
    {
      id: `notes-and-tasks-filter-tasks`,
      title: `Show only your tasks, or one person's tasks`,
      purpose: `Filter the Tasks list by owner and by state.`,
      level: `Beginner`,
      prerequisites: [`The project has tasks.`],
      steps: [
        {
          screen: `notes-filled`,
          do: `Open the owner list at the top of Tasks, which reads Anyone.`,
          explain: `Anyone shows every task. Me shows tasks you own. The list also has the names of the other people in the project.`,
        },
        {
          screen: `notes-tasks-all`,
          do: `Use Open, Done and All to pick the state.`,
          explain: `The owner list and the state switch work together. For example, Me and Open shows what you still have to do. A task with an owner shows the owner's name beside its title. When a list is empty the page says Nothing open. Well done., No completed tasks yet. or No tasks for this owner.`,
        },
      ],
      result: `The Tasks list shows only the tasks you asked for.`,
      cautions: [
        `The owner of a task is chosen in the task drawer.`,
      ],
      related: [`notes-and-tasks-complete-task`, `notes-and-tasks-edit-task`],
    },
    {
      id: `notes-and-tasks-delete-task`,
      title: `Delete a task`,
      purpose: `Remove a task and keep a way to undo it.`,
      level: `Beginner`,
      prerequisites: [`At least one task exists.`],
      steps: [
        {
          screen: `notes-task-editor-existing`,
          do: `Open the task and click Delete at the bottom left of the drawer.`,
          explain: `There is no confirmation dialog, because the task is kept for 30 days.`,
        },
        {
          screen: `notes-task-deleted-toast`,
          do: `Read the message: Moved task “Tour task call the depot” to Recently Deleted.`,
          explain: `Click Undo to bring the task back at once, or Dismiss to close the message.`,
        },
      ],
      result: `The task leaves the list. Undo or Recently Deleted brings it back for 30 days.`,
      cautions: [
        `The item the task was linked to is not changed.`,
      ],
      related: [`notes-and-tasks-edit-task`],
    },
  ],
  screens: {
    'notes-empty': `Notes & Tasks with no notes and no tasks. Project Notes says No notes yet. and Tasks says No tasks yet., each with a button. The Tasks owner list reads Anyone and Open is selected. Used in notes-and-tasks-open-page.`,
    'notes-filled': `Notes & Tasks with three notes (Monsoon schedule risk, Permits, Music) and three open tasks. Used in notes-and-tasks-write-note, notes-and-tasks-add-task, notes-and-tasks-complete-task and notes-and-tasks-filter-tasks.`,
    'notes-search-filtered': `Search notes with permit typed. Only the Permits note is left in Project Notes. Used in notes-and-tasks-find-note.`,
    'notes-search-no-match': `Search notes with text that matches nothing, so the Project Notes list is empty. Used in notes-and-tasks-find-note.`,
    'notes-note-editor': `The note drawer for Permits. It has Title, Note, Added by, the line that notes stay out of formal documents, and Delete and Pin. Used in notes-and-tasks-pin-note.`,
    'notes-note-editor-pinned': `The note drawer for Monsoon schedule risk, which is pinned. The button reads Unpin. Used in notes-and-tasks-pin-note.`,
    'notes-new-note-editor': `The drawer for a brand new note. The Title is empty and an Untitled note is already in the list. Used in notes-and-tasks-write-note.`,
    'notes-new-note-filled': `The note drawer with the title Tour note on the radio tower and its text written. The note is in the list. Used in notes-and-tasks-write-note and notes-and-tasks-delete-note.`,
    'notes-note-deleted-toast': `Notes & Tasks after a note was deleted. A message says Moved note “Tour note on the radio tower” to Recently Deleted, with Undo. Used in notes-and-tasks-delete-note.`,
    'notes-task-editor-new': `The task drawer for New Task: Task *, Due date, Owner, Related to and Notes, with Cancel and Add Task. Used in notes-and-tasks-add-task.`,
    'notes-task-editor-title-error': `The New Task drawer with the title empty. Give the task a short title. shows under the box. Variant, part of notes-and-tasks-add-task.`,
    'notes-task-editor-filled': `The New Task drawer with a title, a due date of 20 November 2026 and a note typed. Used in notes-and-tasks-add-task and notes-and-tasks-link-task.`,
    'notes-task-editor-related-results': `The Matching items list under Related to, with Catalog Item, Location, Scene Card and Scene results. Used in notes-and-tasks-link-task.`,
    'notes-task-editor-related-chosen': `The New Task drawer with the catalog item Night bus chosen under Related to, with a Remove button. Used in notes-and-tasks-link-task.`,
    'notes-task-added-toast': `Notes & Tasks after a task is added. A message says Added task “Tour task call the depot” with Undo, and the task shows a Catalog Item: Night bus chip. Used in notes-and-tasks-add-task and notes-and-tasks-link-task.`,
    'notes-task-editor-existing': `The drawer for an existing task, Scout the east jetty at dusk, with its due date and notes. It has Delete, Mark Done, Cancel and Save. Used in notes-and-tasks-edit-task and notes-and-tasks-delete-task.`,
    'notes-tasks-done': `The Tasks list with Done selected, showing one finished task, Lock the shooting script. Used in notes-and-tasks-complete-task.`,
    'notes-tasks-all': `The Tasks list with All selected, showing open tasks and the finished one. Used in notes-and-tasks-complete-task and notes-and-tasks-filter-tasks.`,
    'notes-task-completed-toast': `Notes & Tasks after a task is ticked. A message says Completed task “Tour task call the depot” with Undo. Used in notes-and-tasks-complete-task.`,
    'notes-task-deleted-toast': `Notes & Tasks after a task was deleted. A message says Moved task “Tour task call the depot” to Recently Deleted, with Undo. Used in notes-and-tasks-delete-task.`,
  },
  shortcuts: [
    [`Ctrl+Z`, `Undo the last change, also offered as Undo in the message`],
    [`Ctrl+K`, `Search the whole project from the top bar`],
  ],
  limits: [
    `Notes are short project notes. They are never included in screenplay or call-sheet exports.`,
    `A task has one owner and one related item at most.`,
    `A task can link to a scene, location, character, shooting day, call sheet and similar project items.`,
    `Completing a task does not change the item it is linked to.`,
    `The Search notes box searches notes only, not tasks.`,
    `Deleted notes and tasks stay in Recently Deleted for 30 days.`,
  ],
  troubleshooting: [
    {
      problem: `I cannot find Notes & Tasks in the left rail.`,
      fix: `It is not in the rail. Click the project name at the top and choose Notes & Tasks, or use QUICK ACCESS on Project Home.`,
    },
    {
      problem: `Add Task does nothing and the box shows Give the task a short title.`,
      fix: `A task needs a title. Type what must be done in Task *.`,
    },
    {
      problem: `A task I finished has vanished.`,
      fix: `The Open filter hides finished tasks. Click Done or All.`,
    },
    {
      problem: `The task I am looking for is not shown.`,
      fix: `Check the owner list. Me shows only your tasks. Choose Anyone to see all of them, and check the Open, Done and All switch.`,
    },
    {
      problem: `I deleted a note or task by mistake.`,
      fix: `Click Undo in the message, or open Recently Deleted from the project menu and click Restore.`,
    },
    {
      problem: `The related item chip says (deleted).`,
      fix: `The item it points to was deleted. Restore it from Recently Deleted, or open the task and click Remove to unlink it.`,
    },
  ],
};
