export default {
  area: `Shot lists`,
  slug: `shot-lists`,
  order: 15,
  part: `Plan`,
  intro: [
    `A shot list plans how you will capture each scene. The page is called Shot Lists and asks: How will I capture this scene? You plan shots one scene at a time. Each shot has a short description, and if you want it, camera details such as shot size, movement, angle and lens, the characters in it, a sound note, a reference picture and a link to a storyboard panel.`,
    `Shot Lists is one of the tabs inside Production. Shots belong to the scenes of your screenplay, and the scene headings come from the script, so you never type them twice. Without a screenplay there are no scenes, and the tab asks you to open the Screenplay first.`,
    `Shots are named by scene number and a letter, for example 1A, 1B and 1C. The letters follow the order of the shots, so you never renumber by hand. Drag a shot and the letters update.`,
    `You can see a scene's shots as a List, with one row per shot, or as a Grid of cards with reference pictures. Export makes a PDF, a CSV file or an Excel file.`,
  ],
  layout: [
    { name: `Page header`, text: `The title Shot Lists and the line How will I capture this scene? On the right: Export and Add Shot. Add Shot is greyed out until a scene is chosen, and its tooltip says Choose a scene first. A small line under the title says which script version the scenes come from.` },
    { name: `Scene picker`, text: `A list labelled Scene. It starts with All scenes, then every scene of the screenplay with its number and heading. A scene that changed after you planned it is marked review. Next to it a tag shows how many shots the chosen scene, or the whole list, has.` },
    { name: `View and actions`, text: `At the right of the same row: the List and Grid buttons, a Delete button that appears when you select more than one shot, a Show removed scenes check box that appears when the script has lost scenes that had shots, and the ... button, called More scene actions, which appears when the chosen scene has shots.` },
    { name: `Shot rows and cards`, text: `In List view each row shows a drag handle, the shot number, the shot size, the description, then the movement, angle and lens as small tags. Small icons show that the shot has a storyboard panel or a reference picture. In Grid view each shot is a card with a picture, its details and its description.` },
    { name: `New shot box`, text: `At the bottom of a scene's list: a box that says Describe the next shot, for example Wide: Arjun enters the station, and an Add Shot button. A line under it reminds you to add only the shots you need.` },
    { name: `Shot drawer`, text: `A panel that slides in from the right when you open a shot. Its heading shows the shot number, size and description.` },
  ],
  workflows: [
    {
      id: `shot-lists-start-from-script`,
      title: `Get scenes into the Shot Lists tab`,
      purpose: `Understand why the tab is empty and how to give it scenes to plan.`,
      level: `Beginner`,
      prerequisites: [`A project is open and you are in Production.`],
      steps: [
        {
          screen: `prod-shots-no-script`,
          do: `Click the Shot Lists tab in Production.`,
          explain: `If the project has no screenplay scenes, the page says No screenplay scenes yet. It explains that shots are planned scene by scene, that you write or import a screenplay first, and that scene headings come from the script. Add Shot is greyed out.`,
        },
        {
          screen: `prod-shots-no-script`,
          do: `Click Open Screenplay, write or import a scene, then return to Shot Lists.`,
          explain: `Open Screenplay takes you to the Screenplay. The Shot Lists tab finds the scenes by itself once the script has them.`,
        },
      ],
      result: `The Shot Lists tab lists your scenes and Add Shot becomes available.`,
      cautions: [
        `You never need to type a scene heading in Shot Lists. It always comes from the screenplay.`,
      ],
      related: [`shot-lists-add-shot`],
    },
    {
      id: `shot-lists-add-shot`,
      title: `Add shots to a scene`,
      purpose: `Write the shots you need for a scene, one line each.`,
      level: `Beginner`,
      prerequisites: [`The screenplay has at least one scene.`],
      steps: [
        {
          screen: `prod-shots-scene-empty`,
          do: `Choose a scene in the Scene list.`,
          explain: `A scene with no shots says No shots in this scene yet and reminds you to add only the shots you need. Camera details are optional. The tag says how many shots the scene has.`,
        },
        {
          screen: `prod-shots-composer-error`,
          do: `Click in the New shot description box and press Enter, or click Add Shot, with nothing typed.`,
          explain: `The box turns red and says Describe the shot to add it. A shot needs a description. Nothing is added until you type one.`,
        },
        {
          screen: `prod-shots-list`,
          do: `Type a description, for example Wide: the bus arrives at the empty stand, and click Add Shot or press Enter.`,
          explain: `The shot is added at the end of the scene's list with the next letter, such as 1A, 1B or 1C. The box clears and stays ready for the next shot. The Add Shot button at the top right does the same job. It moves you to the box, and adds the shot if you have already typed a description.`,
        },
      ],
      result: `The scene lists its shots in order, each with its own letter.`,
      cautions: [
        `A shot needs a description. Empty shots are never added.`,
        `You cannot add a shot while All scenes is chosen. Pick one scene first.`,
      ],
      related: [`shot-lists-edit-shot`, `shot-lists-reorder-delete`],
    },
    {
      id: `shot-lists-edit-shot`,
      title: `Open a shot and set its details`,
      purpose: `Add shot size, movement, angle, lens, camera notes, characters and a sound note to a shot.`,
      level: `Beginner`,
      prerequisites: [`A scene has at least one shot.`],
      steps: [
        {
          screen: `prod-shots-list`,
          do: `Click a shot in the list.`,
          explain: `Each row shows the shot number, the shot size such as Wide or Medium, the description, and a tag for the movement. In the picture the three shots are 1A, 1B and 1C. You can also right-click a shot and choose Open.`,
        },
        {
          screen: `prod-shot-drawer`,
          do: `In the drawer, edit the Description, then click Technical details if the details are hidden.`,
          explain: `Description * is required. If you clear it and click out, the message Description is required. shows and the old text stays. Technical details is a button that shows or hides the optional fields, and it opens by itself when the shot already has details. Changes save when you click out of a box.`,
        },
        {
          screen: `prod-shot-drawer`,
          do: `Fill in Shot size, Movement, Angle and Lens, then Camera notes and Sound note.`,
          explain: `These are free text boxes that suggest common choices as you type. Shot size suggests Wide, Full, Medium, Medium close, Close, Extreme close, Two-shot, OTS, Insert, POV and Establishing. Movement suggests Static, Handheld, Pan, Tilt, Dolly, Tracking, Crane, Steadicam and Zoom. Angle suggests Eye level, Low, High, Overhead, Dutch and Ground level. You can type your own words. Lens and Camera notes show examples, 35mm and follow focus to the folder.`,
        },
        {
          screen: `prod-shot-drawer`,
          do: `In Characters, type a name and press Enter. Click the x on a name to remove it.`,
          explain: `A line under the box lists the characters in this scene, for example FAROOQ and LAKSHMI, and the box suggests them as you type. You can still type any name. Names already added are not suggested again.`,
        },
      ],
      result: `The row shows the size, movement, angle and lens you set, and the shot details are saved.`,
      cautions: [
        `If a message in the drawer says the scene changed since the shot was planned, check the shot against the script.`,
        `Shot size and Movement are typed fields with suggestions, not fixed lists.`,
      ],
      related: [`shot-lists-reference-image`, `shot-lists-storyboard-link`],
    },
    {
      id: `shot-lists-reference-image`,
      title: `Add a reference picture to a shot`,
      purpose: `Attach a picture to a shot as a visual aid, copied into your project.`,
      level: `Beginner`,
      prerequisites: [`A shot is open in its drawer.`],
      steps: [
        {
          screen: `prod-shot-drawer`,
          do: `In the Reference image section, click Add reference image….`,
          explain: `A hint says it is a visual aid for this shot and that it is copied into the project. A Windows window opens so you choose the picture file.`,
        },
        {
          screen: `prod-shot-drawer`,
          do: `Once a picture is there, click Replace… to choose another, or Remove to take it off.`,
          explain: `The picture shows as a small thumbnail. If its file cannot be read, the thumbnail says Image unavailable. A message tells you what changed. The shot shows a small picture icon in List view and the picture itself in Grid view.`,
        },
      ],
      result: `The shot has a reference picture, shown on its Grid card.`,
      cautions: [],
      related: [`shot-lists-switch-view`],
    },
    {
      id: `shot-lists-switch-view`,
      title: `Switch between List, Grid and All scenes`,
      purpose: `See a scene's shots as rows or as cards, or see the shots of every scene together.`,
      level: `Beginner`,
      prerequisites: [`The project has shots.`],
      steps: [
        {
          screen: `prod-shots-list`,
          do: `Look at the shots in List view.`,
          explain: `List is the default. Each row has a drag handle at the left, the number, the shot size, the description and small tags for movement, angle and lens. Use List when you want to reorder shots.`,
        },
        {
          screen: `prod-shots-grid`,
          do: `Click Grid at the top right of the list.`,
          explain: `Each shot becomes a card. It shows Scene and its number, the shot number such as Shot 1A, a picture, a line of details such as Wide · Static, the description and the characters. The picture is the shot's reference picture, or else the first picture of its storyboard panels. A card with neither says No reference image. OpenFrame remembers your choice of List or Grid on this computer.`,
        },
        {
          screen: `prod-shots-all-scenes`,
          do: `Choose All scenes in the Scene list.`,
          explain: `Every scene with shots gets its own heading, with its shots under it. A Scene changed since planning tag appears on a scene that needs review. Click Open scene at the right of a heading to jump to that scene. You cannot add shots in this view, because it has no new shot box.`,
        },
      ],
      result: `You see the shots the way you chose, and the choice is kept the next time you open the tab.`,
      cautions: [
        `Dragging to reorder works in List view only. Grid cards cannot be dragged.`,
      ],
      related: [`shot-lists-reorder-delete`],
    },
    {
      id: `shot-lists-reorder-delete`,
      title: `Reorder, select and delete shots`,
      purpose: `Put shots in the order you will film them and remove the ones you do not need.`,
      level: `Intermediate`,
      prerequisites: [`A scene has two or more shots, shown in List view.`],
      steps: [
        {
          screen: `prod-shots-list`,
          do: `Drag the handle at the left of a shot up or down.`,
          explain: `Each handle is labelled Reorder shot followed by its number. The letters, such as 1A and 1B, are renumbered from the new order, and a message says Reordered shots. You can also focus the handle and use the keyboard.`,
        },
        {
          screen: `prod-shots-list`,
          do: `Hold Ctrl and click shots to pick several, or click one shot and Shift-click another to pick the range between them.`,
          explain: `A Delete button appears in the toolbar and says how many shots are picked, for example Delete 2 shots.`,
        },
        {
          screen: `prod-shot-menu`,
          do: `Right-click a shot and choose Delete, or open the shot and click Delete in the drawer.`,
          explain: `Delete is the red item at the bottom of the menu. With several shots picked it reads Delete followed by the number. A message says Deleted shot with an Undo link. Pressing Delete or Backspace with shots picked does the same.`,
        },
      ],
      result: `The shots are in your order and numbered again from it.`,
      cautions: [
        `Deleting can be undone from the message that appears. Do not renumber shots by hand, the letters follow the order.`,
      ],
      related: [`shot-lists-shot-menu`],
    },
    {
      id: `shot-lists-shot-menu`,
      title: `Duplicate or move a shot`,
      purpose: `Copy a shot inside its scene, or move it to another scene.`,
      level: `Intermediate`,
      prerequisites: [`A shot exists. To move a shot, the script needs at least two scenes.`],
      steps: [
        {
          screen: `prod-shot-menu`,
          do: `Right-click a shot in the list or grid.`,
          explain: `The menu has Open, Duplicate, Move to scene…, then Attach Storyboard… and Create storyboard panel, and finally Delete in red. Move to scene… is greyed out when the script has only one scene.`,
        },
        {
          screen: `prod-shot-menu`,
          do: `Click Duplicate.`,
          explain: `A copy of the shot is added to the scene with the next letter. A message says Duplicated as followed by the new number. The Duplicate button in the drawer does the same.`,
        },
        {
          screen: `prod-shot-move-dialog`,
          do: `Click Move to scene…. In the dialog Move shot followed by its number, choose a Scene and click Move Shot.`,
          explain: `A hint says the shot is added at the end of that scene's shot list. The list shows the other scenes of the script. Cancel closes the dialog. A message says Moved shot to the scene with its new number.`,
        },
      ],
      result: `The shot is copied or moved, and the letters in both scenes are up to date.`,
      cautions: [],
      related: [`shot-lists-storyboard-link`],
    },
    {
      id: `shot-lists-storyboard-link`,
      title: `Connect a shot to storyboard panels`,
      purpose: `Attach an existing panel to a shot, or make a panel from a shot.`,
      level: `Intermediate`,
      prerequisites: [`A shot exists. To attach a panel, a storyboard with panels exists.`],
      steps: [
        {
          screen: `prod-shot-drawer`,
          do: `In the drawer, look at the Storyboard section.`,
          explain: `It lists the panels attached to the shot as buttons such as Storyboard panel 2, each with an x to detach it. Click the button to jump to that panel in Storyboards. Attach… and Create panel are under it.`,
        },
        {
          screen: `prod-shot-attach-dialog`,
          do: `Click Attach… (or Attach Storyboard… in the shot's menu). Choose a Storyboard in the dialog, then click a panel.`,
          explain: `The dialog is called Attach storyboard panel to followed by the shot number. The storyboards of this shot's scene come first, each with its panel count, for example Scene 1: Arrival (3). Each panel shows its number and description. A panel already attached to this shot is greyed out. Cancel closes the dialog. If the project has no storyboards the dialog says to use Create panel. A message says Attached storyboard panel to the shot.`,
        },
        {
          screen: `prod-shot-drawer`,
          do: `Or click Create panel (Create storyboard panel in the shot's menu).`,
          explain: `A new panel is added to the scene's storyboard, linked to this shot. A message says Created storyboard panel from shot. Click the x on a panel button to detach it. The panel itself stays.`,
        },
      ],
      result: `The shot and the panel point at each other. The shot row shows a small clapperboard icon.`,
      cautions: [
        `Detaching a panel only removes the link. The panel stays in its storyboard.`,
      ],
      related: [`shot-lists-edit-shot`],
    },
    {
      id: `shot-lists-copy-planning`,
      title: `Copy a scene's planning to another scene`,
      purpose: `Duplicate the shots and scene storyboards of one scene into another, for example after the scene was duplicated in the script.`,
      level: `Advanced`,
      prerequisites: [`A scene with shots is chosen, and the script has at least one other scene.`],
      steps: [
        {
          screen: `prod-shots-scene-actions-menu`,
          do: `Click the ... button called More scene actions at the right of the toolbar, then click Copy planning to another scene….`,
          explain: `The button only appears when a single scene with shots is chosen.`,
        },
        {
          screen: `prod-shots-copy-planning-dialog`,
          do: `In the dialog, choose the scene in Copy to and click Copy Planning.`,
          explain: `The dialog explains that it copies this scene's shots and scene storyboards as new, independent items, which is useful when a scene was duplicated in the script. Cancel closes it. A message says how many shots and storyboards were copied.`,
        },
      ],
      result: `The other scene has its own copies of the shots and storyboards. Changing one does not change the other.`,
      cautions: [
        `The copies are independent. Later edits to the first scene's shots do not carry over.`,
      ],
      related: [`shot-lists-shot-menu`],
    },
    {
      id: `shot-lists-export`,
      title: `Export a shot list`,
      purpose: `Save the shot list of one scene, some scenes or the whole project as a PDF, CSV or Excel file.`,
      level: `Beginner`,
      prerequisites: [`The project has shots.`],
      steps: [
        {
          screen: `prod-shots-list`,
          do: `Click Export at the top right.`,
          explain: `The export covers the scene you have open, or the scenes you pick in the dialog.`,
        },
        {
          screen: `prod-shots-export-dialog`,
          do: `In the Export Shot List dialog, choose the scope and the format.`,
          explain: `Scope: Current scene (with its number and heading), Selected scenes, which lets you tick scenes, or Whole project. Format: PDF printable, CSV spreadsheet-friendly text, or Excel (XLSX) with one sheet per table. Whole project: also shots of scenes removed from the script includes shots whose scene is no longer in the script. A note says private notes are excluded and that Export creates a snapshot and does not change your project.`,
        },
        {
          screen: `prod-shots-export-dialog`,
          do: `Click Export… and choose where to save the file in the Windows window that opens.`,
          explain: `Cancel closes the dialog with no file made. The PDF carries a small Created using OpenFrame line. CSV files stay clean without it.`,
        },
      ],
      result: `A file with your shots is saved where you chose.`,
      cautions: [
        `The export is a snapshot. Change the shot list afterwards and you need to export again.`,
      ],
      related: [`shot-lists-add-shot`],
    },
    {
      id: `shot-lists-scene-changed`,
      title: `Review a scene that changed after planning`,
      purpose: `Know what the review marks mean and how to clear them.`,
      level: `Intermediate`,
      prerequisites: [`A scene with shots changed in the screenplay after you planned it.`],
      steps: [
        {
          screen: `prod-shots-all-scenes`,
          do: `Look for the word review after a scene in the Scene list, or the tag Scene changed since planning next to a scene heading in All scenes.`,
          explain: `The mark means the scene's script text changed after its shots were planned. Your shots are kept exactly as they were.`,
        },
        {
          screen: `prod-shots-list`,
          do: `Choose that scene and read the banner above the shots.`,
          explain: `The banner says Scene N changed since planning, with the time, and that your shots and storyboards are kept. Check them against the script, then click Mark reviewed. The mark disappears. The picture shows a scene that needs no review, so no banner is visible in it.`,
        },
      ],
      result: `The scene is no longer marked for review.`,
      cautions: [
        `If a scene was removed from the script, its shots are kept for reference and hidden from active planning. Tick Show removed scenes to see them.`,
      ],
      related: [`shot-lists-export`],
    },
  ],
  screens: {
    'prod-shots-no-script': `The Shot Lists tab in a project with no screenplay scenes. It says No screenplay scenes yet, offers Open Screenplay, and shows a greyed-out Add Shot. Used in shot-lists-start-from-script.`,
    'prod-shots-list': `Scene 1 in List view with three shots (1A, 1B, 1C), the 3 shots tag, the List and Grid buttons, More scene actions and the new shot box. Used in shot-lists-add-shot, shot-lists-edit-shot, shot-lists-switch-view, shot-lists-reorder-delete, shot-lists-export and shot-lists-scene-changed.`,
    'prod-shots-grid': `The same scene in Grid view. Each shot is a card with its scene number, shot number, a No reference image area, its details line and its description. Used in shot-lists-switch-view.`,
    'prod-shots-all-scenes': `The Scene list set to All scenes. Shots are grouped under scene headings, each with an Open scene button. Used in shot-lists-switch-view and shot-lists-scene-changed.`,
    'prod-shots-scene-empty': `Scene 3 chosen, which has no shots. The page offers the new shot box. Used in shot-lists-add-shot.`,
    'prod-shots-composer-error': `The new shot box after Add Shot was used with nothing typed. The message Describe the shot to add it. shows in red. Used in shot-lists-add-shot.`,
    'prod-shot-menu': `The right-click menu of a shot: Open, Duplicate, Move to scene…, Attach Storyboard…, Create storyboard panel and Delete. Used in shot-lists-shot-menu and shot-lists-reorder-delete.`,
    'prod-shot-drawer': `The drawer of shot 1A: Description *, the Technical details button, Shot size, Movement, Angle, Lens, Camera notes, Characters, Sound note, Reference image with Add reference image…, and Storyboard with Attach… and Create panel. Delete and Duplicate are at the bottom. Used in shot-lists-edit-shot, shot-lists-reference-image and shot-lists-storyboard-link.`,
    'prod-shot-move-dialog': `The Move shot 1A dialog with a Scene list, Cancel and Move Shot. Used in shot-lists-shot-menu.`,
    'prod-shot-attach-dialog': `The Attach storyboard panel to 1A dialog with a Storyboard list (Scene 1: Arrival (3)), three panels to pick and Cancel. Used in shot-lists-storyboard-link.`,
    'prod-shots-scene-actions-menu': `The More scene actions menu with one entry, Copy planning to another scene…. Used in shot-lists-copy-planning.`,
    'prod-shots-copy-planning-dialog': `The Copy planning to another scene dialog with a Copy to list, Cancel and Copy Planning. Used in shot-lists-copy-planning.`,
    'prod-shots-export-dialog': `The Export Shot List dialog: Current scene, Selected scenes and Whole project; PDF printable, CSV and Excel (XLSX); an option to include shots of scenes removed from the script; and Cancel and Export…. Used in shot-lists-export.`,
  },
  shortcuts: [
    [`Enter`, `Add the shot typed in the new shot box`],
    [`Ctrl+click`, `Pick or unpick a shot in the list`],
    [`Shift+click`, `Pick the range of shots up to the one you click`],
    [`Delete or Backspace`, `Delete the picked shots`],
  ],
  limits: [
    `Shots belong to screenplay scenes. A project with no screenplay cannot have a shot list.`,
    `Only the description is required. Everything else is optional.`,
    `Dragging to reorder works in List view, not Grid view.`,
    `Shot numbers are made from scene number and order. You cannot type them.`,
    `Export covers PDF, CSV and Excel. Other formats are not offered here.`,
  ],
  troubleshooting: [
    {
      problem: `The tab says No screenplay scenes yet.`,
      fix: `Shots are planned from scenes of the screenplay. Click Open Screenplay, write or import a scene, then come back.`,
    },
    {
      problem: `Add Shot is greyed out.`,
      fix: `Choose one scene in the Scene list. Add Shot is not available while All scenes is chosen or when the project has no scenes.`,
    },
    {
      problem: `Describe the shot to add it. appears.`,
      fix: `The new shot box is empty. Type a description and press Enter.`,
    },
    {
      problem: `Move to scene… is greyed out.`,
      fix: `The script has only one scene, so there is nowhere to move the shot.`,
    },
    {
      problem: `I cannot drag shots in Grid view.`,
      fix: `Switch to List. The drag handles are in List view.`,
    },
    {
      problem: `A scene is marked review.`,
      fix: `Its script text changed after planning. Read the shots against the script, then click Mark reviewed.`,
    },
    {
      problem: `The shots of a removed scene are missing.`,
      fix: `Tick Show removed scenes to list them, or choose Whole project with the option for removed scenes when you export.`,
    },
  ],
};
