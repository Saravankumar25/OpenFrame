export default {
  area: `Backups, packages and exchange`,
  slug: `backups-and-packages`,
  order: 29,
  part: `Project and data`,
  intro: [
    `Straft keeps everything on your computer, so this chapter is about the files you make to protect your work and to share it. There are three kinds of file, and they are not interchangeable. A Backup Package (.ofbackup) is a recoverable copy of one project that you keep for yourself. A Full Project Package (.ofproject) moves your whole working project to another PC. Exchange packages are for working with other people: a script review package (.ofscriptreview) or a Beat Board package (.ofstory) goes out, and a response package (.ofresponse) comes back with comments.`,
    `There is no live sharing and no account. You send a package by any normal way you like, such as a USB drive, email or a shared folder, and the other person opens it in their own Straft. Nothing in a package changes your project until you choose to import it, and every import is checked first.`,
    `Every command in this chapter is in the project menu. Click the project name at the top left of the window. The menu has a SHARE / EXCHANGE section (Export Review Package (script)…, Beat Board Package…, Import Exchange Package…, Open Review Package…, Review Queue and Import & Export History) and a BACKUP & PORTABILITY section (Create Backup…, Export Full Project… and Open Project Package…). These items are there while a project is open. From Application Home, Restore Backup or Package… opens the same package window.`,
    `Private notes are handled with care. A backup or a Full Project Package is for you, so it includes your private notes. A review package or exchange package never includes private notes, and the dialogs say so.`,
  ],
  layout: [
    { name: `Project menu`, text: `Opens from the project name at the top left. It lists your recent projects, Undo and Redo, the project pages, then the SHARE / EXCHANGE and BACKUP & PORTABILITY sections, then All projects…, Archived… and Close project.` },
    { name: `Create Backup dialog`, text: `Backup name, an optional Label, Save to with Browse…, the portable option and a WHAT THIS BACKUP CONTAINS table.` },
    { name: `Export Full Project dialog`, text: `An INCLUDE list of what moves with the project, a note about private notes, the linked files option and Export Project Package….` },
    { name: `Script Review Package and Beat Board Package dialogs`, text: `Source draft or Act, a Scope switch, Include comments, Include attachments, Include private notes (always off), and a THIS PACKAGE WILL CONTAIN summary.` },
    { name: `Import Exchange Package dialog`, text: `Starts with Choose Package…. After you pick a file it shows what the package is, how it compares with your project and what each change would do.` },
    { name: `Review Queue`, text: `Imported comments that could not be placed on a scene with certainty, waiting for your decision.` },
    { name: `Import & Export History`, text: `Four tabs: Imports, Review records, My reviews and Package log.` },
    { name: `Open Project Package dialog`, text: `Choose Package… to pick a .ofproject or .ofbackup file. It then shows the package and how to open it.` },
  ],
  workflows: [
    {
      id: `backups-and-packages-find-commands`,
      title: `Find the backup and package commands`,
      purpose: `Open the project menu and know which command does what.`,
      level: `Beginner`,
      prerequisites: [`A project is open.`],
      steps: [
        {
          screen: `pkg-project-menu`,
          do: `Click the project name at the top left of the window.`,
          explain: `The menu opens under it. At the top are RECENT PROJECTS (the open project is ticked and marked Open), then EDIT with Undo and Redo and their keys, then Project settings, Notes & Tasks, Activity, Recently Deleted, Templates and Appearance.`,
        },
        {
          screen: `pkg-project-menu`,
          do: `Find SHARE / EXCHANGE.`,
          explain: `Export Review Package (script)… and Beat Board Package… send your work out for review. Import Exchange Package… brings a package in. Open Review Package… opens a package someone sent you for review. Review Queue shows imported comments waiting for a decision. Import & Export History lists everything that went in or out.`,
        },
        {
          screen: `pkg-project-menu`,
          do: `Find BACKUP & PORTABILITY.`,
          explain: `Create Backup… saves a recoverable copy. Export Full Project… makes a package to move the project to another PC. Open Project Package… opens a project package or restores a backup. At the bottom are All projects…, Archived… and Close project.`,
        },
      ],
      result: `You know where each backup and package command lives.`,
      cautions: [
        `The package commands are hidden when no project is open. Open a project first, or use Restore Backup or Package… on Application Home.`,
      ],
      related: [`backups-and-packages-create-backup`, `backups-and-packages-export-script-review`],
    },
    {
      id: `backups-and-packages-create-backup`,
      title: `Create a backup of your project`,
      purpose: `Save a recoverable copy of the project to a folder you choose, ideally on another disk.`,
      level: `Beginner`,
      prerequisites: [`A project is open.`],
      steps: [
        {
          screen: `pkg-project-menu`,
          do: `Open the project menu and click Create Backup… under BACKUP & PORTABILITY.`,
          explain: `The Create Backup dialog opens over your project.`,
        },
        {
          screen: `pkg-backup-dialog`,
          do: `Read Backup name. Type a Label (optional) if you want one, for example before-rewrite.`,
          explain: `The dialog says: A recoverable copy of supported project state. The Backup name cannot be typed in. It is made from the project name, the date and time, and your label, and it ends in .ofbackup. A label is up to 80 characters, and characters Windows does not allow in file names are replaced with an underscore.`,
        },
        {
          screen: `pkg-backup-dialog`,
          do: `Click Browse… next to Save to, pick a folder in the Windows window and save.`,
          explain: `The note says: Choose another disk if you can. Project backups are separate from your active project. A backup on the same disk does not help if the disk fails. Create Backup stays greyed out until you have chosen a place.`,
        },
        {
          screen: `pkg-backup-dialog`,
          do: `Tick Copy linked external files into the backup (portable backup) if your project links to files outside its own folder and you want them in the backup too.`,
          explain: `Without it, linked files stay where they are and are not copied. Turn it on when the backup must still work if you move to another computer.`,
        },
        {
          screen: `pkg-backup-dialog`,
          do: `Read WHAT THIS BACKUP CONTAINS, then click Create Backup.`,
          explain: `It lists Idea Vault, Beat Board, screenplay drafts, breakdown, catalog, production planning, comments and your private notes. The table shows Files stored in the project with their size and count and the tag Included, Linked external files tagged External, not copied (or Included (copied) when the portable option is on), and Missing linked files tagged Missing. Files named as missing are listed and never counted as saved.`,
        },
      ],
      result: `A backup file ending in .ofbackup is saved in the folder you chose.`,
      cautions: [
        `A progress bar with a Cancel button shows while the backup is made. Your project stays open and saved. If you cancel, no backup file is left behind.`,
        `When it finishes, the dialog says Backup created., shows where the file is and how big it is, and offers Show in folder. If it fails, the dialog says why and that your project was not changed.`,
        `Linked files that are missing on this computer are listed. Fix them before relying on the backup.`,
      ],
      related: [`backups-and-packages-open-project-package`, `backups-and-packages-move-to-another-pc`],
    },
    {
      id: `backups-and-packages-export-full-project`,
      title: `Export a Full Project Package`,
      purpose: `Pack your whole working project into one file that opens on another PC.`,
      level: `Intermediate`,
      prerequisites: [`A project is open.`],
      steps: [
        {
          screen: `pkg-project-menu`,
          do: `Open the project menu and click Export Full Project….`,
          explain: `The command is under BACKUP & PORTABILITY.`,
        },
        {
          screen: `pkg-export-project-dialog`,
          do: `Read the INCLUDE list.`,
          explain: `The dialog says: A Full Project Package moves your whole working project to another PC. It is not a review package and not a PDF collection. The list shows what goes in, with counts: Idea Vault (project), Beat Board, Characters, Timeline, Screenplay drafts, reviews and comments, Breakdown, Catalog, Locations, Cast & Crew, Moodboards, Storyboards, Shot Lists, Schedule, Call Sheets, Sides, Notes, Tasks, Activity, and Project Files. All of them are ticked and cannot be turned off, because the package is the whole project.`,
        },
        {
          screen: `pkg-export-project-dialog`,
          do: `Read the purple note, then decide about Copy linked external files into the package.`,
          explain: `The note says: Private project transfer. Private notes are included because this is for you, not for outside reviewers. The checkbox shows how many linked files there are, here 0. Tick it to copy linked files into the package. If some linked files are missing on this computer, a line says so and they are listed as missing. The project still opens.`,
        },
        {
          screen: `pkg-export-project-dialog`,
          do: `Click Export Project Package…, pick a folder in the Windows window and save.`,
          explain: `The suggested name is the project title with .ofproject. A progress bar with Cancel shows while it runs. Cancel closes the work and leaves no package file behind. When it ends, the dialog says Project package exported. with the place of the file, and offers Show in folder. Open it on the other PC with Open Project Package….`,
        },
      ],
      result: `A .ofproject file holds your whole project.`,
      cautions: [
        `This is a private copy of your work, with your private notes in it. Send it only to someone you trust, or to yourself.`,
        `Do not use a Full Project Package to ask someone for comments. Use a script review package, which leaves private notes out.`,
      ],
      related: [`backups-and-packages-move-to-another-pc`, `backups-and-packages-open-project-package`],
    },
    {
      id: `backups-and-packages-open-project-package`,
      title: `Open a project package or restore a backup`,
      purpose: `Open a .ofproject file or restore a .ofbackup file as a project, without losing the project you already have.`,
      level: `Intermediate`,
      prerequisites: [`You have a .ofproject or .ofbackup file on this computer, a disk or another PC.`],
      steps: [
        {
          screen: `pkg-project-menu`,
          do: `Open the project menu and click Open Project Package…. On Application Home, click Restore Backup or Package… instead.`,
          explain: `Both open the same window. The Application Home route is covered in the Application Home chapter.`,
        },
        {
          screen: `pkg-open-project-package-dialog`,
          do: `Click Choose Package… and pick the file in the Windows window.`,
          explain: `The window says: Open a Full Project Package or restore a Backup Package from another disk or PC. It accepts .ofproject (Full Project Package) and .ofbackup (Backup Package). The package is validated before anything is created. Cancel closes the window. While the file is checked the button reads Checking….`,
        },
        {
          screen: `pkg-open-project-package-dialog`,
          do: `Read what the dialog shows about the package.`,
          explain: `After the file is checked, four boxes show the Package type, the Project name, when it was Created (with your label, if the backup has one) and its Size with a Valid mark. A line under Import report says Validated, and tells you if any linked external files are missing on this PC. The project still opens even then. The title changes to Restore Backup for a backup file.`,
        },
        {
          screen: `pkg-open-project-package-dialog`,
          do: `If a project with the same identity is already on this computer, choose how to open it.`,
          explain: `The window is then titled This project already exists on this computer. Open as Copy is the default. It creates the project with (copy) after its name and a new identity, and your existing project is not touched. Replace Existing After Backup is marked High impact. It saves a backup of the existing project first and then replaces it, closing it if it is open. When there is no match, a note says the project will open as the same project.`,
        },
        {
          screen: `pkg-open-project-package-dialog`,
          do: `Click the main button: Open Project, Restore or Open as Copy. For Replace Existing After Backup, confirm.`,
          explain: `The button changes with your choice. Replace Existing After Backup opens a confirmation titled Replace the existing project?, which says the project will be backed up to the Backups folder and then replaced, and that work saved after the package was made will only be in that backup. Confirm with Back Up and Replace, or cancel. Opening another project closes the one you have open, and Straft first asks about any unsaved work.`,
        },
      ],
      result: `The project opens. The window says Project opened. or Opened as a copy., with a Validated line.`,
      cautions: [
        `Importing never silently replaces the current project. The default is a copy.`,
        `If the package holds private notes, a note tells you that they become yours. You can see and edit them, and other members of the project cannot.`,
        `If the project came from an older version of Straft, the window says it was upgraded and a safety copy was kept.`,
        `A file that is not a valid package is refused before anything is created. Choose a different file.`,
      ],
      related: [`backups-and-packages-create-backup`, `backups-and-packages-export-full-project`],
    },
    {
      id: `backups-and-packages-move-to-another-pc`,
      title: `Move a project to another PC`,
      purpose: `Carry a project from one computer to another with a Full Project Package.`,
      level: `Intermediate`,
      prerequisites: [`Straft Studio is installed on both PCs.`, `A way to carry the file, such as a USB drive.`],
      steps: [
        {
          screen: `pkg-export-project-dialog`,
          do: `On the first PC, open the project, then choose Export Full Project… from the project menu.`,
          explain: `Keep the whole project ticked. Tick Copy linked external files into the package if the project uses files outside its folder.`,
        },
        {
          screen: `pkg-export-project-dialog`,
          do: `Click Export Project Package…, save the .ofproject file on a drive and wait for Project package exported.`,
          explain: `Use Show in folder to find the file. Copy it to the other PC.`,
        },
        {
          screen: `pkg-open-project-package-dialog`,
          do: `On the second PC, click Restore Backup or Package… on Application Home, or Open Project Package… in the project menu, and choose the file.`,
          explain: `The package is checked first. Because the project is new on this PC, a note says it opens as the same project and keeps its identity.`,
        },
        {
          screen: `pkg-open-project-package-dialog`,
          do: `Click Open Project.`,
          explain: `The project opens on the new PC with its ideas, story, screenplay drafts, production plan and files.`,
        },
      ],
      result: `Your project is on the other PC, complete.`,
      cautions: [
        `Linked files that were not copied are missing on the new PC. They are listed, and the project still opens.`,
        `To keep working on both PCs, remember that two copies drift apart. Work in one place at a time, or use review packages to bring comments back.`,
      ],
      related: [`backups-and-packages-export-full-project`, `backups-and-packages-open-project-package`],
    },
    {
      id: `backups-and-packages-export-script-review`,
      title: `Send a script for review`,
      purpose: `Export a draft as a read-only review package that someone can comment on.`,
      level: `Intermediate`,
      prerequisites: [`The project has at least one screenplay draft.`],
      steps: [
        {
          screen: `pkg-project-menu`,
          do: `Open the project menu and click Export Review Package (script)… under SHARE / EXCHANGE.`,
          explain: `The Script Review Package dialog opens.`,
        },
        {
          screen: `pkg-export-script-dialog`,
          do: `Choose the Source draft.`,
          explain: `The dialog asks: How do I send this draft to someone for review and safely bring their notes back? The list shows each draft with its scene count, and marks the current one, for example Second Draft (current) · 36 scenes.`,
        },
        {
          screen: `pkg-export-script-dialog`,
          do: `Under Scope, choose Full script.`,
          explain: `Full script sends every scene. Include comments is ticked and you can untick it. Include attachments is available when the draft has attachments, and its count shows in brackets. Include private notes is greyed out and says not allowed in review packages.`,
        },
        {
          screen: `pkg-export-script-comments`,
          do: `Or choose Script + comments.`,
          explain: `The whole script goes out with the comments in it. Include comments is ticked and cannot be turned off.`,
        },
        {
          screen: `pkg-export-script-dialog`,
          do: `Read THIS PACKAGE WILL CONTAIN, then click Export Package….`,
          explain: `It lists the Project, the Target draft, the Scope, who exported it and the date, then the counts, for example 36 scenes and 4 comments, and the line Private notes: excluded. A note explains: The reviewer opens a read-only copy of this draft in Straft and adds comments. They send back a Response package; nothing in your project changes until you preview it and choose what to import. A Windows window asks where to save the .ofscriptreview file.`,
        },
      ],
      result: `The dialog says Package exported. and shows where the file is. Send it by any normal way.`,
      cautions: [
        `Nothing in the package changes your project, and later changes to your project do not change the package.`,
        `Private notes are never included in a review package.`,
        `Send only the draft you mean to share. Pick the draft in Source draft first.`,
      ],
      related: [`backups-and-packages-export-script-scenes`, `backups-and-packages-open-review-package`, `backups-and-packages-import-response`],
    },
    {
      id: `backups-and-packages-export-script-scenes`,
      title: `Send only some scenes for review`,
      purpose: `Export a review package that holds just the scenes you pick.`,
      level: `Intermediate`,
      prerequisites: [`The Script Review Package dialog is open.`],
      steps: [
        {
          screen: `pkg-export-script-no-scene`,
          do: `Under Scope, click Specific scenes only.`,
          explain: `A list of the draft's scenes appears, each with a check box and its number and heading. Until you tick one, the dialog says Choose at least one scene. and the Export Package… button does not work.`,
        },
        {
          screen: `pkg-export-script-scene-chosen`,
          do: `Tick the scenes you want to send.`,
          explain: `The list scrolls inside the dialog. In the picture Scene 1 is ticked, and the summary reads Scope: Scene 1 with 1 scenes and 1 comments. The counts follow what you tick.`,
        },
        {
          screen: `pkg-export-script-scene-chosen`,
          do: `Choose Include comments if you want the comments on those scenes to go along, then click Export Package….`,
          explain: `Include attachments and Include private notes behave as in a full script. The private notes stay greyed out. Save the file in the Windows window.`,
        },
      ],
      result: `A review package with only the chosen scenes is saved.`,
      cautions: [
        `If you see Choose at least one scene., tick a scene or choose Full script.`,
      ],
      related: [`backups-and-packages-export-script-review`],
    },
    {
      id: `backups-and-packages-export-beat-board`,
      title: `Share the Beat Board`,
      purpose: `Export the whole Beat Board, or one act of it, as a package.`,
      level: `Intermediate`,
      prerequisites: [`The project has a Beat Board with at least one act.`],
      steps: [
        {
          screen: `pkg-project-menu`,
          do: `Open the project menu and click Beat Board Package… under SHARE / EXCHANGE.`,
          explain: `The Beat Board Package dialog opens.`,
        },
        {
          screen: `pkg-export-beat-board-dialog`,
          do: `Under Scope, choose Whole Beat Board.`,
          explain: `The dialog asks: Which part of the Beat Board do I share, and what comes with it? Include comments can be switched on or off. Include attachments is available when there are attachments. Include private notes is greyed out and not allowed. THIS PACKAGE WILL CONTAIN lists the project, the target, the scope, the date and the counts of acts, sequences, beats and scene cards.`,
        },
        {
          screen: `pkg-export-beat-board-one-act`,
          do: `Or choose One Act and pick the act in the Act list.`,
          explain: `The list shows the acts, for example Act 1: The Return, Act 2: The Tower and Act 3: The Signal. The counts change to the one act you picked.`,
        },
        {
          screen: `pkg-export-beat-board-one-act`,
          do: `Click Export Package… and save the file in the Windows window.`,
          explain: `The note under the summary says: The receiver can import the outline as a new copy, or, for the same project, propose changes you review before anything is applied. The file ends in .ofstory.`,
        },
      ],
      result: `A Beat Board package is saved, and the dialog shows where.`,
      cautions: [
        `Private notes are left out of the package.`,
      ],
      related: [`backups-and-packages-import-exchange`],
    },
    {
      id: `backups-and-packages-open-review-package`,
      title: `Open a review package and send back comments`,
      purpose: `Read a script someone sent you, add comments and send them back as a response package.`,
      level: `Intermediate`,
      prerequisites: [`You received a script review package (.ofscriptreview).`],
      steps: [
        {
          screen: `pkg-project-menu`,
          do: `Open the project menu and click Open Review Package….`,
          explain: `You can do this from any project of your own. The package is read-only and kept apart from your projects. Pick the file in the Windows window. The package is checked, and a message appears if it is not valid.`,
        },
        {
          screen: `pkg-history-my-reviews`,
          do: `Find the review in Import & Export History, on the My reviews tab.`,
          explain: `Each open review is listed with the project and draft name, who exported it and how many of your comments it has. Open takes you back into the review later. The viewer is a full window and is not pictured here, so the next lines describe it.`,
        },
        {
          screen: `pkg-history-my-reviews`,
          do: `In the review window, pick a scene on the left, then click Add a comment on the right.`,
          explain: `A bar at the top says Review package. Comments are separate from the original project. The scenes are listed on the left with a count of comments on each. The scene text is in the middle and cannot be edited. To attach a comment to some words, select them in the scene and click Quote selection. Type the note in the Comment box (Ctrl+Enter adds it) and click Comment. Delete this comment removes one of yours.`,
        },
        {
          screen: `pkg-history-my-reviews`,
          do: `Click Export Response Package at the top and save the .ofresponse file.`,
          explain: `The button stays off until you add a comment. The review window then says Response package exported. Send it back to the author. Close review leaves the review window.`,
        },
      ],
      result: `You have a response file with your comments, ready to send back.`,
      cautions: [
        `Nothing you do in a review changes your own projects.`,
        `A review you opened stays in My reviews, so you can add comments later and export a new response.`,
      ],
      related: [`backups-and-packages-import-response`, `backups-and-packages-export-script-review`],
    },
    {
      id: `backups-and-packages-import-response`,
      title: `Bring back comments from a response package`,
      purpose: `Import a reviewer's comments safely and see exactly what changes before anything is applied.`,
      level: `Advanced`,
      prerequisites: [`You sent a review package and received a response (.ofresponse), or you received a Beat Board package.`, `The project the package came from is open.`],
      steps: [
        {
          screen: `pkg-project-menu`,
          do: `Open the project menu and click Import Exchange Package….`,
          explain: `The Import Exchange Package dialog opens.`,
        },
        {
          screen: `pkg-import-exchange-choose`,
          do: `Click Choose Package… and pick the file in the Windows window.`,
          explain: `The dialog asks: What exactly will this imported package change? It takes review, response, Beat Board and the older Breakdown, Shot List, Schedule and Call Sheet packages. The package is validated first; nothing changes until you choose what to import. While it checks, the button reads Validating….`,
        },
        {
          screen: `pkg-import-exchange-choose`,
          do: `Read the summary boxes and the banner.`,
          explain: `After the check, boxes show the Package type, the Source project, the Source version compared with yours, and Compatibility (Valid, Stale or Rejected). A banner tells you how many new comments were received. A rejected package says why, and your project was not changed.`,
        },
        {
          screen: `pkg-import-exchange-choose`,
          do: `Read the table headed Changes by object, and tick the changes you want.`,
          explain: `Each row has a count, a state and a tick box. The states are Safe mapping, Review required, Conflict, Ambiguous, Unmapped and None. Click the arrow on a row to see each change and, for an ambiguous one, the possible places. A conflict is never applied unless you tick it. Import never changes your screenplay text. Tick Create Backup Before Import if you want a safety backup first.`,
        },
        {
          screen: `pkg-import-exchange-choose`,
          do: `Choose how to bring it in with one of the buttons at the bottom.`,
          explain: `Import Selected applies only the ticked changes. Accept All Safe applies every change marked safe. Comments Only attaches only the comments. Copy as New imports a Beat Board as a new copy. Keep as Review Record keeps the package for reference without changing your work. View Package opens it read-only. Cancel closes the window and changes nothing.`,
        },
        {
          screen: `pkg-history-imports`,
          do: `Afterwards, read the message at the top of the dialog and open Import & Export History.`,
          explain: `The message says Applied, Partially Applied or Pending Review, never a plain Import complete for a part. If something needs a decision, an Open Review Queue button appears. In the history, the Imports tab lists the import. Undo Import (in the dialog) or Undo (in the history) removes what that import added, as long as it was not changed afterwards.`,
        },
      ],
      result: `The comments you chose are in the project, and the rest is kept for you to decide.`,
      cautions: [
        `If a package is Stale, because your project changed since it was made, a window titled This package was created from an older project state. offers four choices. They are View the review without importing, Import comments only (the default), Create a separate review record and Compare versions. Staleness does not mean rejection.`,
                `If an import fails part way, everything is rolled back and your project is left as it was.`,
      ],
      related: [`backups-and-packages-review-queue`, `backups-and-packages-history`],
    },
    {
      id: `backups-and-packages-import-exchange`,
      title: `Import a Beat Board package someone sent you`,
      purpose: `Bring an outline from a Beat Board package into your project as a copy, or review proposed changes to the same project.`,
      level: `Advanced`,
      prerequisites: [`You received a Beat Board package (.ofstory).`],
      steps: [
        {
          screen: `pkg-import-exchange-choose`,
          do: `Open the project menu, click Import Exchange Package… and click Choose Package….`,
          explain: `Pick the .ofstory file. The package is validated before anything changes.`,
        },
        {
          screen: `pkg-import-exchange-choose`,
          do: `Read the preview.`,
          explain: `The summary names the package type, the source project and the versions. The Changes by object table lists what the package holds, such as acts, sequences, beats and scene cards, each with a state.`,
        },
        {
          screen: `pkg-import-exchange-choose`,
          do: `Click Copy as New to import the outline as a new copy, or tick changes and click Import Selected for the same project.`,
          explain: `Copy as New leaves your own Beat Board alone. Import Selected proposes changes to the same project, and you review them before they apply.`,
        },
      ],
      result: `The outline is in your project as a copy, or the ticked changes are applied.`,
      cautions: [
        `The package never overwrites your Beat Board without you choosing it.`,
      ],
      related: [`backups-and-packages-import-response`, `backups-and-packages-export-beat-board`],
    },
    {
      id: `backups-and-packages-review-queue`,
      title: `Place imported comments that did not match`,
      purpose: `Decide where each waiting comment belongs, so no comment is lost.`,
      level: `Advanced`,
      prerequisites: [`An import left comments that could not be placed with certainty.`],
      steps: [
        {
          screen: `pkg-project-menu`,
          do: `Open the project menu and click Review Queue.`,
          explain: `You can also click Open Review Queue in the import window when an import leaves comments waiting.`,
        },
        {
          screen: `pkg-review-queue-empty`,
          do: `Read the queue.`,
          explain: `The dialog says: Imported comments that need a decision. No comment is ever silently discarded. When it is empty it says Nothing waiting and When an imported comment can't be matched safely, it waits here for you. Close closes the dialog.`,
        },
        {
          screen: `pkg-review-queue-empty`,
          do: `When comments are waiting, look at the groups Ambiguous mapping and Unmatched Review Notes.`,
          explain: `Each card shows the comment, who wrote it, the words it was on, where it came from and the reason it is waiting. Ambiguous mapping means more than one scene fits. Unmatched Review Notes means none did.`,
        },
        {
          screen: `pkg-review-queue-empty`,
          do: `Click an Attach to button for a suggested scene, or Choose… or Attach manually… to pick a scene in a list, then click Attach.`,
          explain: `A message says Attached to and the scene. The comment moves to the Attached group, and the card is dimmed. Cancel closes the list without attaching.`,
        },
      ],
      result: `Every waiting comment is attached to a scene.`,
      cautions: [
        `Ambiguous and unmatched notes wait here until you decide. Nothing is discarded.`,
      ],
      related: [`backups-and-packages-import-response`, `backups-and-packages-history`],
    },
    {
      id: `backups-and-packages-history`,
      title: `See what you exported and imported`,
      purpose: `Read the record of every package, undo an import and open kept reviews.`,
      level: `Intermediate`,
      prerequisites: [`A project is open.`],
      steps: [
        {
          screen: `pkg-history-imports`,
          do: `Open the project menu and click Import & Export History.`,
          explain: `The dialog says: Every package this project exported or imported, and what happened. The Imports tab is shown first. It lists each import with the package name, its type and where it came from, the result and when, with Undo or Review where they apply. Until you import something it says No imports yet. A note reads: Undo removes everything one import added, as long as it wasn't changed afterwards.`,
        },
        {
          screen: `pkg-history-review-records`,
          do: `Click the Review records tab.`,
          explain: `It holds packages you chose to keep for reference, such as a stale package kept as a record. When none are kept it says No review records. Each record has View, which opens it read-only, and Delete. Delete asks Delete this review record? and moves it to Recently Deleted, where it can be restored.`,
        },
        {
          screen: `pkg-history-my-reviews`,
          do: `Click the My reviews tab.`,
          explain: `It lists review packages you opened with Open Review Package…, with the number of your comments and when you exported a response. When there are none it says No open reviews. Open takes you back into a review.`,
        },
        {
          screen: `pkg-history-package-log`,
          do: `Click the Package log tab.`,
          explain: `It lists every package in or out, tagged Export or Import, with a summary, the time and who did it. When empty it says No packages yet. Close closes the dialog.`,
        },
      ],
      result: `You can see what moved in and out of the project and undo an import.`,
      cautions: [
        `Undo works on a whole import. It does not work once something the import added has been changed.`,
        `A package that was only previewed shows Previewed, nothing applied, with a Review button to finish it.`,
      ],
      related: [`backups-and-packages-import-response`, `backups-and-packages-review-queue`],
    },
  ],
  screens: {
    'pkg-project-menu': `The project menu, opened from the project name at the top left. It shows RECENT PROJECTS, EDIT (Undo and Redo), the project pages, SHARE / EXCHANGE (Export Review Package (script)…, Beat Board Package…, Import Exchange Package…, Open Review Package…, Review Queue, Import & Export History), BACKUP & PORTABILITY (Create Backup…, Export Full Project…, Open Project Package…), then All projects…, Archived… and Close project. Used in backups-and-packages-find-commands and as the first step of most workflows here.`,
    'pkg-backup-dialog': `The Create Backup dialog. It has the read-only Backup name, an optional Label, Save to with Browse…, the Copy linked external files into the backup (portable backup) option, the WHAT THIS BACKUP CONTAINS table, Cancel and a greyed out Create Backup. Used in backups-and-packages-create-backup.`,
    'pkg-export-project-dialog': `The Export Full Project dialog. It explains that a Full Project Package moves the whole project to another PC, lists what is included (all ticked), shows the Private project transfer note and the linked files option, and offers Cancel and Export Project Package…. Used in backups-and-packages-export-full-project and backups-and-packages-move-to-another-pc.`,
    'pkg-export-script-dialog': `The Script Review Package dialog on Full script: Source draft, the Scope switch, Include comments (ticked), Include attachments, Include private notes (greyed out), and the THIS PACKAGE WILL CONTAIN summary with 36 scenes and 4 comments. Used in backups-and-packages-export-script-review.`,
    'pkg-export-script-comments': `The same dialog with Script + comments chosen. Include comments is ticked and fixed. Used in backups-and-packages-export-script-review.`,
    'pkg-export-script-no-scene': `The same dialog with Specific scenes only chosen and no scene ticked. The list of scenes shows and the dialog says Choose at least one scene. Used in backups-and-packages-export-script-scenes.`,
    'pkg-export-script-scene-chosen': `The same dialog with Scene 1 ticked. The summary reads Scope: Scene 1 with 1 scenes and 1 comments. Used in backups-and-packages-export-script-scenes.`,
    'pkg-export-beat-board-dialog': `The Beat Board Package dialog on Whole Beat Board, with the counts 3 acts, 2 sequences, 3 beats and 7 scene cards. Used in backups-and-packages-export-beat-board.`,
    'pkg-export-beat-board-one-act': `The same dialog on One Act with the Act list showing Act 1: The Return and the counts for that act. Used in backups-and-packages-export-beat-board.`,
    'pkg-import-exchange-choose': `The Import Exchange Package dialog before a file is chosen. It asks What exactly will this imported package change?, says the package is validated first, and offers Choose Package…. Used in backups-and-packages-import-response and backups-and-packages-import-exchange. The preview that follows is described in those workflows.`,
    'pkg-review-queue-empty': `The Review Queue with nothing waiting. It says Nothing waiting and explains that unmatched imported comments wait here. Used in backups-and-packages-review-queue.`,
    'pkg-history-imports': `Import & Export History on the Imports tab with No imports yet and the note about Undo. Used in backups-and-packages-history and backups-and-packages-import-response.`,
    'pkg-history-review-records': `Import & Export History on the Review records tab with No review records. Used in backups-and-packages-history.`,
    'pkg-history-my-reviews': `Import & Export History on the My reviews tab with No open reviews. Used in backups-and-packages-history and backups-and-packages-open-review-package.`,
    'pkg-history-package-log': `Import & Export History on the Package log tab with No packages yet. Used in backups-and-packages-history.`,
    'pkg-open-project-package-dialog': `The Open Project Package dialog before a file is chosen. It accepts .ofproject and .ofbackup, says the package is validated before anything is created, and offers Choose Package… and Cancel. Used in backups-and-packages-open-project-package and backups-and-packages-move-to-another-pc.`,
  },
  shortcuts: [
    [`Ctrl+Z`, `Undo (shown in the project menu under EDIT)`],
    [`Ctrl+Y`, `Redo (shown in the project menu under EDIT)`],
    [`Ctrl+Enter`, `Add a comment in a review package`],
  ],
  limits: [
    `Sharing is by files only. There is no live editing, no accounts and no cloud sync.`,
    `You can export two kinds of exchange package: a script review package and a Beat Board package. Older Breakdown, Shot List, Schedule and Call Sheet packages can still be imported, but new ones cannot be exported.`,
    `A review package never includes private notes. Backups and Full Project Packages do, because they are for you.`,
    `A backup or package copies the files stored in the project. Linked files outside the project are copied only when you tick the option, and missing ones are listed.`,
    `Import never changes your screenplay text. It attaches comments and applies the changes you tick.`,
    `Opening a package never silently replaces a project. Replacing needs your choice and a backup first.`,
  ],
  troubleshooting: [
    {
      problem: `I cannot find Create Backup or Export Full Project.`,
      fix: `Open a project, then click its name at the top left. The commands are under BACKUP & PORTABILITY.`,
    },
    {
      problem: `Create Backup is greyed out.`,
      fix: `Click Browse… next to Save to and choose where to save the backup. The button turns on once a place is chosen.`,
    },
    {
      problem: `The backup was cancelled.`,
      fix: `Nothing is saved when you cancel. The dialog says so. Start again from Create Backup….`,
    },
    {
      problem: `The package will not open.`,
      fix: `Packages are checked before anything is created. Make sure the file ends in .ofproject or .ofbackup, and that it was not damaged in transit. Copy it again and choose it again.`,
    },
    {
      problem: `The window says This project already exists on this computer.`,
      fix: `The package holds a project you already have. Choose Open as Copy to keep both, or Replace Existing After Backup to replace it. Replacing saves a backup first.`,
    },
    {
      problem: `Some linked files are missing after opening a package.`,
      fix: `They were linked from another place and not copied. The window lists them. Copy them to this PC and relink, or export again with Copy linked external files ticked.`,
    },
    {
      problem: `Export Package… does not work in Script Review Package.`,
      fix: `If the scope is Specific scenes only, tick at least one scene. The dialog says Choose at least one scene.`,
    },
    {
      problem: `Include private notes is greyed out in a review package.`,
      fix: `That is intended. Private notes are never in review packages. Use a Full Project Package for your own transfers.`,
    },
    {
      problem: `Include attachments is greyed out.`,
      fix: `The draft has no attachments. The number in brackets is 0.`,
    },
    {
      problem: `An imported comment is not on any scene.`,
      fix: `Open Review Queue. Comments that could not be placed with certainty wait there for you to attach them.`,
    },
    {
      problem: `The import says the package was created from an older project state.`,
      fix: `The package is Stale. Choose Import comments only, keep it as a review record, view it or compare versions. Staleness does not mean the package is bad.`,
    },
    {
      problem: `I imported by mistake.`,
      fix: `Open Import & Export History, and click Undo on the import. Undo removes what that import added, as long as it was not changed afterwards.`,
    },
  ],
};
