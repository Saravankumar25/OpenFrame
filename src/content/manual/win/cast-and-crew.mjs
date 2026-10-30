export default {
  area: `Cast and crew`,
  slug: `cast-and-crew`,
  order: 12,
  part: `Plan`,
  intro: [
    `Cast & Crew is the small directory of the people on your film. It answers: who is involved, and what role do they have? It has no payroll and no contracts. For each person you keep a contact, availability notes, dates and notes. For each actor you also keep the character they play.`,
    `The page has two lists, chosen with the Cast and Crew buttons under the title. Cast links real people to characters. The characters come from your Production Source, so every character in the script appears in the Cast list on its own, marked Needs casting until you assign an actor. Crew lists the people working on the film, with a role and an optional department.`,
    `A character is part of the story. A cast member is the real person who plays it. One character has one primary actor, and you can add alternates or doubles for the same character. A person is added in one dialog titled Add Person, which links them to a character.`,
    `Contact details are private. They are visible to Owners and Editors only, and they are never shared with the assistant or put in exports. Dates are advisory: the schedule warns about them and never moves a scene by itself.`,
  ],
  layout: [
    { name: `Header`, text: `The title Cast & Crew, the line Who is involved and what role do they have? A small-team directory with no payroll and no contracts., and the orange Add Person button at the top right.` },
    { name: `Toolbar`, text: `The Cast and Crew buttons, a Search box, and a Show archived check box at the far right. The selected list button is dark.` },
    { name: `Cast table`, text: `Columns PERSON, CHARACTER, CONTACT, AVAILABILITY NOTE and SCENES. Each cast member shows a round photo, the name, the character and the number of scenes. An alternate actor carries an Alternate label. Characters with no actor are listed under the actors as Unassigned, with the character name, None for contact, a Needs casting label and an Assign button.` },
    { name: `Crew table`, text: `Columns PERSON, ROLE, DEPARTMENT and CONTACT. A line under the table lists the departments (Direction, Camera, Sound, Art, Costume, Makeup, Production, Editing) and says This is a production directory, not HR software.` },
    { name: `Cast member drawer`, text: `A panel on the right headed CAST MEMBER and the name. It has a photo with Add Photo, the Character with Primary or Alternate and Assign Character…, Person name, Contact, Availability notes, Availability dates, Notes and Scenes. The footer has Delete and Archive.` },
    { name: `Crew member drawer`, text: `A panel headed CREW MEMBER and the name. It has Person name, Role, Department, Contact, Availability notes, Availability dates and Notes. The footer has Delete and Archive.` },
  ],
  workflows: [
    {
      id: `cast-and-crew-read-cast`,
      title: `Read the cast list and see who still needs casting`,
      purpose: `Find the characters that have no actor and see how many scenes each person is in.`,
      level: `Beginner`,
      prerequisites: [`A project is open with a Production Source.`],
      steps: [
        {
          screen: `prod-cast`,
          do: `Click Production in the left rail, then Cast & Crew in the tab strip. Cast is selected when the page opens.`,
          explain: `The table lists actors first, for example Ananya Rao as LAKSHMI, with the number of scenes at the right. Contact shows what you typed, for example an agent's email. Availability note shows the note and the number of date ranges you saved.`,
        },
        {
          screen: `prod-cast`,
          do: `Look at the rows named Unassigned.`,
          explain: `Each is a character from the script with no actor yet, such as NEWSREADER or PADMA. The row shows a yellow Needs casting label and an Assign button. SCENES counts the scenes in the Production Source that have that character.`,
        },
        {
          screen: `prod-cast-empty`,
          do: `In a project with no Production Source, or with no characters, the page is empty.`,
          explain: `It says No cast yet. and Characters from the Production Source appear here automatically. Add the actor who plays each one, with a contact and availability notes. An Add Person button is under the message.`,
        },
      ],
      result: `You can see which characters are cast and which are not.`,
      cautions: [
        `The characters come from the Production Source. If a character you expect is missing, check that the Production Source is the right draft.`,
      ],
      related: [`cast-and-crew-assign-actor`, `cast-and-crew-search`],
    },
    {
      id: `cast-and-crew-assign-actor`,
      title: `Assign an actor to a character`,
      purpose: `Link a real person to a character who still needs casting.`,
      level: `Beginner`,
      prerequisites: [`The Cast list shows at least one character with Needs casting, and your role can edit.`],
      steps: [
        {
          screen: `prod-cast`,
          do: `Click Assign in the row of the character you want to cast. To start without choosing a character, click Add Person at the top right instead.`,
          explain: `Assign opens the dialog with that character already chosen. Add Person opens it with nothing chosen.`,
        },
        {
          screen: `prod-cast-assign-dialog`,
          do: `In the Add Person dialog, type the actor's name in Person *.`,
          explain: `The dialog says: Link the real person to the character they play. The box shows the hint Actor's name. Assign stays greyed out until a person and a character are both chosen. Cancel closes the dialog.`,
        },
        {
          screen: `prod-cast-assign-dialog`,
          do: `Under Choose a character, click the character. Check the Casting buttons, Primary actor and Alternate / double.`,
          explain: `The list shows each character with the number of scenes and either played by the actor or no actor. In the picture NEWSREADER is chosen: 1 scene, no actor. Primary actor is selected. One character has one primary actor by default.`,
        },
        {
          screen: `prod-cast-assign-dialog`,
          do: `Click Assign.`,
          explain: `The dialog closes and a message says the person plays the character, with an Undo link. The row in the Cast list now shows the actor, the character and the scenes.`,
        },
      ],
      result: `The actor is linked to the character. The Unassigned row for that character is gone, and the Needs casting count on the Overview goes down.`,
      cautions: [
        `Contact details you type later are private to Owners and Editors and never put in exports.`,
        `Click Primary actor or Alternate / double on purpose. See Add an alternate or a double.`,
      ],
      related: [`cast-and-crew-add-alternate`, `cast-and-crew-new-character`],
    },
    {
      id: `cast-and-crew-add-alternate`,
      title: `Add an alternate or a double`,
      purpose: `Cast a second person for a character who already has an actor.`,
      level: `Intermediate`,
      prerequisites: [`The character you want already has an actor.`],
      steps: [
        {
          screen: `prod-cast-assign-alternate-banner`,
          do: `Click Add Person. Type the new person's name and click a character that already has an actor, such as LAKSHMI.`,
          explain: `The list shows the actor who plays each character. The picture has Lata Nair typed and LAKSHMI chosen. When a character already has an actor, the dialog starts on Alternate / double.`,
        },
        {
          screen: `prod-cast-assign-alternate-banner`,
          do: `Choose Alternate / double to add a second person, or Primary actor to replace the current actor as primary.`,
          explain: `If you choose Primary actor for a character who already has one, a blue note says the current actor becomes an alternate. In the picture it says Ananya Rao becomes an alternate for LAKSHMI. Nobody is removed. The note is information, not a warning.`,
        },
        {
          screen: `prod-cast-assign-alternate-banner`,
          do: `Click Assign.`,
          explain: `A message says the person plays the character. In the Cast list an alternate shows an Alternate label beside the name.`,
        },
      ],
      result: `The character has a primary actor and one or more alternates.`,
      cautions: [
        `Check which button is selected before you click Assign. Primary actor makes the old primary an alternate.`,
        `You can change the arrangement later with Assign Character… on a person.`,
      ],
      related: [`cast-and-crew-assign-actor`, `cast-and-crew-change-character`],
    },
    {
      id: `cast-and-crew-new-character`,
      title: `Cast someone for a character who is not in the script yet`,
      purpose: `Create a new character while you add a person.`,
      level: `Intermediate`,
      prerequisites: [`Your role can edit.`],
      steps: [
        {
          screen: `prod-cast-assign-dialog`,
          do: `Click Add Person and type the person's name.`,
          explain: `Use this for a part you plan to add, such as a station master the script does not mention yet.`,
        },
        {
          screen: `prod-cast-assign-dialog`,
          do: `In Create a new character, type the character name, for example THE STATION MASTER.`,
          explain: `The hint says: For a character that isn't in the script yet. Typing here overrides any character chosen in the list above, and the choice list stops showing a selection. If there are no characters at all, the list says No characters found in the Production Source yet. Create one below.`,
        },
        {
          screen: `prod-cast-assign-dialog`,
          do: `Choose Primary actor and click Assign.`,
          explain: `The new character is created and linked to the person. It is a production record, so it does not appear in your screenplay by itself.`,
        },
      ],
      result: `The person and a new character appear in the Cast list.`,
      cautions: [
        `Use Choose a character first. Create a new character only for a name that is not in the script.`,
      ],
      related: [`cast-and-crew-assign-actor`],
    },
    {
      id: `cast-and-crew-change-character`,
      title: `Change the character a person plays`,
      purpose: `Move a person to a different character, or add a second character to them.`,
      level: `Beginner`,
      prerequisites: [`The Cast list has at least one person.`],
      steps: [
        {
          screen: `prod-cast-row-menu`,
          do: `Right-click a person's row and choose Assign Character…. You can also open the person and click Assign Character… in the drawer.`,
          explain: `The row menu has Open, Assign Character…, Archive and a red Delete…. The menu appears only on people, not on Unassigned character rows, which have their own Assign button.`,
        },
        {
          screen: `prod-cast-assign-existing-dialog`,
          do: `In the Assign a character to dialog, choose a character and Primary actor or Alternate / double, then click Assign.`,
          explain: `The title names the person, here Ananya Rao. The Person box is filled in and cannot be edited. The list shows the character they play now, selected. The rest of the dialog works like Add Person.`,
        },
      ],
      result: `The person is linked to the character you chose, as primary or alternate.`,
      cautions: [
        `If you set the person as Primary for a character who already has a primary actor, that actor becomes an alternate.`,
      ],
      related: [`cast-and-crew-add-alternate`, `cast-and-crew-edit-cast-member`],
    },
    {
      id: `cast-and-crew-edit-cast-member`,
      title: `Open a cast member and edit their details`,
      purpose: `Keep a person's photo, contact, availability notes and notes up to date, and see which scenes they are in.`,
      level: `Beginner`,
      prerequisites: [`The Cast list has at least one person.`],
      steps: [
        {
          screen: `prod-cast-drawer`,
          do: `Click a person's row. Their drawer opens on the right.`,
          explain: `It is headed CAST MEMBER and the name. The row you opened is highlighted in the table.`,
        },
        {
          screen: `prod-cast-drawer`,
          do: `Click Add Photo (or Change Photo) and choose a picture. Use the small x beside the photo to remove it.`,
          explain: `The photo shows as a round picture in the drawer and in the table. Until you add one a plain portrait symbol is used.`,
        },
        {
          screen: `prod-cast-drawer`,
          do: `Edit Person name *, Contact, Availability notes and Notes.`,
          explain: `Fields save when you click out of them or press Enter. Contact is private: visible to Owners and Editors only, and never shared with the assistant or put in exports. Availability notes is for anything the dates cannot say, for example Free Jun 9 to 20, weekends only. Notes is free text for you.`,
        },
        {
          screen: `prod-cast-drawer`,
          do: `Read Character, and scroll down to Scenes.`,
          explain: `Character shows the name with a Primary or Alternate label. Scenes lists every scene in the Production Source that has the character, as small labels. Click a label to open that scene in the Breakdown.`,
        },
      ],
      result: `The person's details are saved and you can jump to their scenes.`,
      cautions: [
        `Person name cannot be left empty. The old name comes back if you clear it.`,
        `If your role cannot see contacts, the Contact line says Contact details are hidden for your role, and the table shows Hidden.`,
      ],
      related: [`cast-and-crew-availability`, `cast-and-crew-remove-person`],
    },
    {
      id: `cast-and-crew-add-crew`,
      title: `Add a crew member`,
      purpose: `Put someone on the crew list with their role and department.`,
      level: `Beginner`,
      prerequisites: [`Your role can edit.`],
      steps: [
        {
          screen: `prod-crew-empty`,
          do: `Click Crew in the toolbar. On an empty list, click Add Person.`,
          explain: `An empty crew list says No crew yet. and Add the people working on the film with their role and department. This is a production directory, not HR software. Add Person at the top right does the same when you already have crew.`,
        },
        {
          screen: `prod-crew-add-dialog`,
          do: `In the Add Person dialog, type a Person name * and a Role *. Choose a Department, and fill in Contact and Notes if you want.`,
          explain: `The dialog says: A crew member with their role. No payroll, no contracts. The Role box shows the example Director of Photography. Department starts on No department and offers Direction, Camera, Sound, Art, Costume, Makeup, Production and Editing. Add to Crew stays greyed out until both the name and the role have text. Cancel closes the dialog.`,
        },
        {
          screen: `prod-crew`,
          do: `Click Add to Crew.`,
          explain: `The dialog closes, a message says the person was added to the crew with an Undo link, and the person opens in their drawer. The table has PERSON, ROLE, DEPARTMENT and CONTACT, for example Divya Menon, Director of Photography, Camera.`,
        },
      ],
      result: `The person is on the crew list with their role and department.`,
      cautions: [
        `Both the name and the role are required.`,
      ],
      related: [`cast-and-crew-edit-crew-member`],
    },
    {
      id: `cast-and-crew-edit-crew-member`,
      title: `Open a crew member and edit their details`,
      purpose: `Change a role or department, and keep contact, availability and notes current.`,
      level: `Beginner`,
      prerequisites: [`The Crew list has at least one person.`],
      steps: [
        {
          screen: `prod-crew`,
          do: `Click a row in the Crew list. You can also right-click it for Open, Archive and Delete….`,
          explain: `The crew row menu has Open, Archive and a red Delete…. There is no Assign Character… for crew.`,
        },
        {
          screen: `prod-crew-drawer`,
          do: `Edit Person name *, Role *, Department, Contact, Availability notes and Notes.`,
          explain: `The drawer is headed CREW MEMBER and the name. Department is a menu with No department and the eight departments. Availability notes shows a hint such as weekends only. Fields save when you click out of them or press Enter. Person name and Role cannot be left empty: the old text comes back.`,
        },
      ],
      result: `The crew member's role, department and details are updated.`,
      cautions: [
        `Contact is private to Owners and Editors and is never put in exports.`,
      ],
      related: [`cast-and-crew-availability`, `cast-and-crew-add-crew`],
    },
    {
      id: `cast-and-crew-availability`,
      title: `Record when a person is available`,
      purpose: `Add dated windows for an actor or crew member so the schedule can warn about clashes.`,
      level: `Intermediate`,
      prerequisites: [`A cast member or crew member is open in their drawer.`],
      steps: [
        {
          screen: `prod-cast-drawer`,
          do: `Under Availability, choose Not available or Available in the first menu.`,
          explain: `The line above the form reads No dates entered. until you add some. A note below says: Dates are advisory: the schedule warns about them and never moves a scene by itself.`,
        },
        {
          screen: `prod-cast-drawer`,
          do: `Choose a First day and a Last day, and type an optional Note. Then click Add Dates.`,
          explain: `Last day can be left empty for a single day. Add Dates is greyed out until a First day is chosen. The range then appears above the form with a red Not available or green Available label, the dates and the note. A message says Updated availability with the person's name, and has an Undo link.`,
        },
        {
          screen: `prod-crew-drawer`,
          do: `Crew members use the same form. To remove a range, click the x beside it.`,
          explain: `Use Availability notes for anything the dates cannot say, such as weekends only.`,
        },
      ],
      result: `The person lists their dates, and the schedule warns when a scene is placed on a day that clashes.`,
      cautions: [
        `Dates never move a scene. They only produce warnings in the schedule.`,
      ],
      related: [`cast-and-crew-edit-cast-member`, `cast-and-crew-edit-crew-member`],
    },
    {
      id: `cast-and-crew-search`,
      title: `Search and show archived people`,
      purpose: `Find a person by name, character, role or note, and include people you archived.`,
      level: `Beginner`,
      prerequisites: [`The list you are on has people.`],
      steps: [
        {
          screen: `prod-cast`,
          do: `On Cast, type in the Search box.`,
          explain: `The Cast list narrows to people whose name, character or availability note match. Unassigned characters also narrow by name.`,
        },
        {
          screen: `prod-crew`,
          do: `Click Crew and type in the Search box.`,
          explain: `The Crew list narrows to people whose name, role, department or availability note match. The same box is used for both lists, and clicking Cast or Crew switches which list it filters.`,
        },
        {
          screen: `prod-crew`,
          do: `Tick Show archived at the top right.`,
          explain: `Archived people are hidden by default. When shown they are paler and carry an Archived label.`,
        },
      ],
      result: `The list shows only the people you asked for.`,
      cautions: [
        `If someone seems missing, clear the search box and check Show archived.`,
      ],
      related: [`cast-and-crew-remove-person`],
    },
    {
      id: `cast-and-crew-remove-person`,
      title: `Archive or remove a person`,
      purpose: `Take someone out of the active lists, either keeping them or removing them for 30 days.`,
      level: `Intermediate`,
      prerequisites: [`The list has at least one person.`],
      steps: [
        {
          screen: `prod-cast-row-menu`,
          do: `To keep the person, right-click their row and choose Archive, or click Archive in their drawer.`,
          explain: `There is no confirmation question. A message says Archived with the name and has an Undo link. To bring them back, tick Show archived, open the person and click Restore, or choose Restore from Archive in the row menu.`,
        },
        {
          screen: `prod-cast-row-menu`,
          do: `To remove the person, choose the red Delete…. In a drawer, click Delete.`,
          explain: `Nothing is removed until you confirm in the next dialog.`,
        },
        {
          screen: `prod-cast-remove-confirm`,
          do: `Read the Remove Ananya Rao from the cast? dialog and click Remove.`,
          explain: `It says the person moves to Recently Deleted for 30 days, that their character becomes Needs casting again, and that call sheets already issued keep their copy. Used in lists the scenes, and a line says who the person plays. It ends: Archiving is the better choice if they may return. Cancel keeps the person.`,
        },
        {
          screen: `prod-crew-remove-confirm`,
          do: `For a crew member, the dialog is shorter.`,
          explain: `It says the person moves to Recently Deleted for 30 days and that call sheets already issued keep their copy, then repeats that archiving is the better choice if they may return. Cancel and Remove are at the bottom.`,
        },
      ],
      result: `The person is archived or removed. A message says so, with an Undo link, and a removed person can be restored from Recently Deleted for 30 days.`,
      cautions: [
        `Removing a cast member makes their character Needs casting again.`,
        `Call sheets already issued keep their copy of the person.`,
        `Archive is the better choice for someone who may return.`,
      ],
      related: [`cast-and-crew-search`, `cast-and-crew-assign-actor`],
    },
  ],
  screens: {
    'prod-cast-empty': `The Cast list of a project with no characters or people. It says No cast yet. and Characters from the Production Source appear here automatically, with an Add Person button. Cast is selected in the toolbar. Used in cast-and-crew-read-cast.`,
    'prod-crew-empty': `The Crew list of a project with no crew. It says No crew yet. and Add the people working on the film with their role and department. This is a production directory, not HR software. Crew is selected, and an Add Person button is under the message. Used in cast-and-crew-add-crew.`,
    'prod-cast': `The Cast list of Night Bus: Ananya Rao (LAKSHMI, 22 scenes), Imran Khan (FAROOQ, 9) and Suresh Babu (VENKAT RAO, 19), then seven Unassigned characters (NEWSREADER, SURESH, PADMA, CONSTABLE RAJU, BITTU, CHILD, DR. KAVITHA NAIR), each with a Needs casting label and an Assign button. Used in cast-and-crew-read-cast, cast-and-crew-assign-actor and cast-and-crew-search.`,
    'prod-cast-assign-dialog': `The Add Person dialog opened for NEWSREADER. It has Person * (Actor's name), Choose a character with six characters visible and NEWSREADER selected, Create a new character, the Casting buttons Primary actor and Alternate / double, and Cancel and Assign. Assign is greyed out until a person is named. Used in cast-and-crew-assign-actor and cast-and-crew-new-character.`,
    'prod-cast-assign-alternate-banner': `The Add Person dialog with Lata Nair typed and LAKSHMI selected. A blue note says Ananya Rao becomes an alternate for LAKSHMI. Assign is ready. Used in cast-and-crew-add-alternate.`,
    'prod-cast-row-menu': `The menu of a cast row, opened by right-click: Open, Assign Character…, Archive and a red Delete…. Used in cast-and-crew-change-character and cast-and-crew-remove-person.`,
    'prod-cast-assign-existing-dialog': `The Assign a character to Ananya Rao dialog. The Person box holds Ananya Rao, LAKSHMI is selected in the character list and Primary actor is selected. Cancel and Assign are at the bottom. Used in cast-and-crew-change-character.`,
    'prod-cast-drawer': `The CAST MEMBER drawer for Ananya Rao: photo with Add Photo, Character LAKSHMI with a Primary label and Assign Character…, Person name *, Contact, Availability notes, the Availability date form, Notes and the Scenes labels. Delete and Archive are in the footer. Used in cast-and-crew-edit-cast-member and cast-and-crew-availability.`,
    'prod-cast-remove-confirm': `The Remove Ananya Rao from the cast? dialog. It says the person moves to Recently Deleted for 30 days, LAKSHMI becomes Needs casting again and issued call sheets keep their copy. It lists her scenes and Plays LAKSHMI, and ends with Archiving is the better choice if they may return. Cancel and a red Remove are at the bottom. Used in cast-and-crew-remove-person.`,
    'prod-crew': `The Crew list with four people: Karthik S (Production Designer, Art), Divya Menon (Director of Photography, Camera), Ravi Teja (Line Producer, Production) and Neha Joshi (Sound Recordist, Sound). A line under the table lists the departments. Used in cast-and-crew-edit-crew-member, cast-and-crew-add-crew and cast-and-crew-search.`,
    'prod-crew-drawer': `The CREW MEMBER drawer for Divya Menon: Person name *, Role * (Director of Photography), Department (Camera), Contact, Availability notes, the Availability date form and Notes. Delete and Archive are in the footer. Used in cast-and-crew-edit-crew-member and cast-and-crew-availability.`,
    'prod-crew-remove-confirm': `The Remove Divya Menon from the crew? dialog. It says the person moves to Recently Deleted for 30 days and issued call sheets keep their copy, and that archiving is the better choice if they may return. Cancel and a red Remove are at the bottom. Used in cast-and-crew-remove-person.`,
    'prod-crew-add-dialog': `The Add Person dialog for crew: Person name *, Role * (with the example Director of Photography), Department (No department), Contact and Notes. Add to Crew is greyed out until the name and role are filled in. Used in cast-and-crew-add-crew.`,
  },
  shortcuts: [
    [`Ctrl+K`, `Search this project (shown in the search box at the top of the window)`],
  ],
  limits: [
    `Cast & Crew is a directory. It has no payroll, contracts or invoices.`,
    `Characters come from the Production Source. A new character created in the Add Person dialog is a production record and does not appear in the screenplay.`,
    `One character has one primary actor. Others are alternates or doubles.`,
    `Contact is private to Owners and Editors and is never shared with the assistant or put in exports.`,
    `Availability dates are advisory. The schedule warns about them and never moves a scene.`,
    `Crew departments are fixed to Direction, Camera, Sound, Art, Costume, Makeup, Production and Editing, or none.`,
  ],
  troubleshooting: [
    {
      problem: `Assign is greyed out in the dialog.`,
      fix: `You need both a person's name in Person * and a character, either chosen from the list or typed in Create a new character.`,
    },
    {
      problem: `Add to Crew is greyed out.`,
      fix: `Both Person name * and Role * are required. Fill in both.`,
    },
    {
      problem: `I expect a character in the Cast list but it is missing.`,
      fix: `Characters come from the Production Source. Check the source on the Overview tab, or use Create a new character for a part not in the script.`,
    },
    {
      problem: `A person disappeared from the list.`,
      fix: `Clear the Search box and tick Show archived. If you removed them, restore them from Recently Deleted within 30 days.`,
    },
    {
      problem: `I made the wrong person the primary actor.`,
      fix: `Open the person who should be primary and use Assign Character… with Primary actor. The other person becomes an alternate.`,
    },
    {
      problem: `Add Dates is greyed out.`,
      fix: `Choose a First day. Last day is optional.`,
    },
    {
      problem: `The Contact line says Contact details are hidden for your role.`,
      fix: `Only Owners and Editors can see contacts. Ask an Owner or Editor.`,
    },
    {
      problem: `Add Person, Assign, Archive or Delete are missing.`,
      fix: `They show only when your role can edit the project.`,
    },
  ],
};
