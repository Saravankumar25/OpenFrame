export default {
  area: `Project Settings`,
  slug: `project-settings`,
  order: 23,
  part: `Project and data`,
  intro: [
    `Project Settings is where one project keeps its identity and its working preferences: title, type, status, logline and notes, the spelling language and dictionary, the colours and icons of the breakdown categories, and where the project is stored. It opens as a sheet over the project, so you never leave your place.`,
    `Open it with the Project settings button in the toolbar of the project, or press Shift+Command+,. Click Done at the top right to close it. A row of eight tabs runs along the top: Project settings, Writing, Keyboard shortcuts, Templates, Offline AI, Advanced, Appearance and Storage.`,
    `Some tabs are about this project only: Project settings, Writing, Advanced and the project part of Storage. Others, Keyboard shortcuts, Offline AI and Appearance, show the same settings as the Settings window and apply to every project on this Mac. The line at the bottom of the sheet says so: Offline AI, Appearance and Keyboard shortcuts apply to every project on this Mac. The App Settings… button next to it opens the Settings window.`,
    `Everything is stored on this Mac, in the project folder. The project works fully offline.`,
  ],
  layout: [
    { name: `Tab row`, text: `Project settings, Writing, Keyboard shortcuts, Templates, Offline AI, Advanced, Appearance and Storage. The tab you are on is highlighted. Done is at the right end of the row.` },
    { name: `Project settings tab`, text: `The heading Project settings with the line Project identity fields., and Revert and Save at the right. A form with Title, Type, Language, Genre, Creator, Status, Logline and Project notes, and a card on the right, Stored on this computer.` },
    { name: `Stored on this computer card`, text: `The folder of the project, Open project location, Move or back up…, the dates Created and Last changed, and the note that the project works fully offline and everything stays in this folder.` },
    { name: `Writing tab`, text: `SPELLING with the check box Check spelling as I write, Main language and Second language. PROJECT DICTIONARY with an Add a word box and Add Word.` },
    { name: `Advanced tab`, text: `Categories: the colour and icon of each breakdown category, with Restore Defaults.` },
    { name: `Footer`, text: `The line about settings that apply to every project, and App Settings….` },
  ],
  workflows: [
    {
      id: `project-settings-open`,
      title: `Open Project Settings and move between its tabs`,
      purpose: `Open the sheet for the open project and know what each tab is for.`,
      level: `Beginner`,
      prerequisites: [`A project is open.`],
      steps: [
        {
          screen: `pages-settings-general`,
          do: `Click the Project settings button in the toolbar, or press Shift+Command+,.`,
          explain: `The sheet opens over the project on the Project settings tab. The toolbar button shows the name Project settings. Done closes the sheet.`,
        },
        {
          screen: `pages-settings-writing`,
          do: `Click Writing to set spelling.`,
          explain: `Spelling language and the project dictionary live here.`,
        },
        {
          screen: `pages-settings-shortcuts`,
          do: `Click Keyboard shortcuts, then Offline AI and Appearance when you need them.`,
          explain: `These three show the same pages as the Settings window. A change here applies to every project on this Mac.`,
        },
        {
          screen: `pages-settings-advanced`,
          do: `Click Advanced for breakdown category colours and icons, and Storage for where the project is kept.`,
          explain: `Templates, the fourth tab, is covered in the Templates chapter. Click Done when you finish.`,
        },
      ],
      result: `You can reach every project setting from one sheet.`,
      cautions: [
        `A change on Project settings is kept only after you click Save. Click Revert to undo unsaved edits.`,
      ],
      related: [`project-settings-edit-details`],
    },
    {
      id: `project-settings-edit-details`,
      title: `Edit the title, type, status and other details`,
      purpose: `Change the identity fields of the project and save them.`,
      level: `Beginner`,
      prerequisites: [`Project Settings is open on the Project settings tab.`],
      steps: [
        {
          screen: `pages-settings-general`,
          do: `Look at the form.`,
          explain: `Title (required) and Type (required) must always have a value. Language, Genre and Creator show Add later until you fill them. Status is a menu. Logline asks for one or two sentences about the film, and is optional. Everything here is optional except title and type. Language, genre and creator are reused on exported documents. Revert is greyed out and Save is pale while nothing has changed.`,
        },
        {
          screen: `pages-settings-general-edited`,
          do: `Click a box and type, for example a logline.`,
          explain: `The picture shows a logline typed in. As soon as something changes, Revert and Save become active.`,
        },
        {
          screen: `pages-settings-type-menu`,
          do: `To change the type, click the Type menu and choose Feature Film or Short Film.`,
          explain: `The menu lists the two types. The tick or highlighted entry is the one in use.`,
        },
        {
          screen: `pages-settings-status-menu`,
          do: `To change the status, click the Status menu and choose one.`,
          explain: `The choices are Idea, Development, Writing, Rewrite, Shooting Draft, Pre-Production, Shoot Preparation and Shooting. The current one has a tick. The status also shows in the project header as Status: followed by the name.`,
        },
        {
          screen: `pages-settings-saved`,
          do: `Click Save.`,
          explain: `A message at the top says Updated project settings with Undo and Dismiss. Click Undo to take the change back at once, or Dismiss to close the message. Revert greys out again.`,
        },
      ],
      result: `The project shows the new details and the message Updated project settings.`,
      cautions: [
        `Click Save before you click Done or switch tabs. Revert returns the form to the last saved values.`,
      ],
      related: [`project-settings-title-required`],
    },
    {
      id: `project-settings-title-required`,
      title: `Fix an empty title`,
      purpose: `Recover when the title is empty and the project cannot be saved without one.`,
      level: `Beginner`,
      prerequisites: [`Project Settings is open on the Project settings tab.`],
      steps: [
        {
          screen: `pages-settings-title-error`,
          do: `If you clear the Title box, the box shows its Title prompt and the field stays selected.`,
          explain: `Title (required) is the only field that must always have text. A project has to have a title, so type one.`,
        },
        {
          screen: `pages-settings-general`,
          do: `Type a title, or click Revert to bring back the saved one, then click Save.`,
          explain: `Revert puts back the title the project had when you last saved.`,
        },
      ],
      result: `The project has a title again and the changes are saved.`,
      cautions: [
        `Do not click Done with an empty title. Type one, or click Revert first.`,
      ],
      related: [`project-settings-edit-details`],
    },
    {
      id: `project-settings-location`,
      title: `Find the project folder and see its dates`,
      purpose: `Show where the project lives on this Mac and when it was made and last changed.`,
      level: `Beginner`,
      prerequisites: [`Project Settings is open on the Project settings tab.`],
      steps: [
        {
          screen: `pages-settings-general`,
          do: `Look at the Stored on this computer card.`,
          explain: `It shows the path of the project folder, then two lines: Created and Last changed with the date and time. The card ends: The project works fully offline. Everything stays in this folder.`,
        },
        {
          screen: `pages-settings-general`,
          do: `Click Open project location.`,
          explain: `The project folder opens in Finder.`,
        },
        {
          screen: `pages-settings-general`,
          do: `Click Move or back up… to move or back up the project.`,
          explain: `Moving a project is taught in the Settings chapter, under Move a project to another folder. The Storage tab of this sheet holds the same controls.`,
        },
      ],
      result: `You can open the folder of the project in Finder.`,
      cautions: [
        `Do not rename or delete files inside the project folder by hand. Use the app to move or back up a project.`,
      ],
      related: [`project-settings-storage`],
    },
    {
      id: `project-settings-spelling`,
      title: `Turn spelling on and choose the languages`,
      purpose: `Set whether the project is spell checked and in which languages.`,
      level: `Beginner`,
      prerequisites: [`Project Settings is open.`],
      steps: [
        {
          screen: `pages-settings-writing`,
          do: `Click Writing. Under SPELLING, tick or untick Check spelling as I write.`,
          explain: `When ticked, words are checked on this Mac by the system spelling service. Character names and words you add to the dictionary are never marked as mistakes.`,
        },
        {
          screen: `pages-settings-language-menu`,
          do: `Click the Main language menu and choose a language.`,
          explain: `The list includes Arabic, Bulgarian, Czech, Danish, Dutch, several kinds of English, Finnish, French, German, Greek, Hebrew, Hindi, Hungarian, Icelandic, Indonesian, Irish, Italian, Korean, Lithuanian, Norwegian, Polish, Portuguese, Punjabi, Romanian, Russian, Slovenian, Spanish, Swedish, Telugu, Turkish, Ukrainian and Vietnamese.`,
        },
        {
          screen: `pages-settings-writing`,
          do: `If the script mixes two languages, choose a Second language the same way.`,
          explain: `It starts as None. A second language stops words in that language being flagged as mistakes.`,
        },
      ],
      result: `Spelling is checked in the languages you chose.`,
      cautions: [
        `Spelling uses the dictionaries installed on your Mac. A language the Mac does not have cannot be checked.`,
      ],
      related: [`project-settings-dictionary`],
    },
    {
      id: `project-settings-dictionary`,
      title: `Add a word to the project dictionary`,
      purpose: `Stop a name or a made up word being marked as a mistake in this project.`,
      level: `Beginner`,
      prerequisites: [`Project Settings is open on Writing.`],
      steps: [
        {
          screen: `pages-settings-writing`,
          do: `Under PROJECT DICTIONARY, click the Add a word box and type the word.`,
          explain: `Until you add one, the list says No words added yet. Add Word is greyed out until you type something.`,
        },
        {
          screen: `pages-settings-writing-word`,
          do: `Click Add Word.`,
          explain: `The word appears in the list with a Remove button next to it. The picture shows Kadalkkara added. The word is never marked as a mistake in this project.`,
        },
        {
          screen: `pages-settings-writing-word`,
          do: `To take a word out, click Remove beside it.`,
          explain: `The word goes back to being checked like any other.`,
        },
      ],
      result: `The word is accepted by the spell checker in this project.`,
      cautions: [
        `The dictionary belongs to this project. Another project starts with its own empty list.`,
      ],
      related: [`project-settings-spelling`],
    },
    {
      id: `project-settings-categories`,
      title: `Change the colour and icon of a breakdown category`,
      purpose: `Give each breakdown category the colour and icon that suit your production.`,
      level: `Intermediate`,
      prerequisites: [`Project Settings is open.`],
      steps: [
        {
          screen: `pages-settings-advanced`,
          do: `Click Advanced.`,
          explain: `The page is called Categories. It says: Breakdown category colours and icons. Every category also keeps its name and description, so colour is never the only signal. Each row shows a category, such as Cast, Extras / Background, Location / Set, Props and Wardrobe, with its description.`,
        },
        {
          screen: `pages-settings-advanced`,
          do: `On a row, click one of the twelve coloured circles.`,
          explain: `The colours are Red, Orange, Amber, Green, Teal, Cyan, Blue, Indigo, Violet, Pink, Brown and Slate. The chosen colour has a dark ring and Selected is read out for it.`,
        },
        {
          screen: `pages-settings-advanced`,
          do: `Click one of the icons under the colours.`,
          explain: `Sixteen icons are offered: a person, two people, a map pin, a box, a shirt, a car, scissors, a flame, sparkles, a speaker, a paw, a star, a camera, a palette, a wrench and a tag. The icon in use has a tinted background.`,
        },
        {
          screen: `pages-settings-advanced`,
          do: `To go back to the original look, click Restore Defaults.`,
          explain: `Restore Defaults is greyed out while every category uses its default colour and icon.`,
        },
      ],
      result: `Breakdown shows each category in the colour and icon you chose.`,
      cautions: [
        `Names and descriptions never change, so a category is still recognisable without its colour.`,
      ],
      related: [`project-settings-open`],
    },
    {
      id: `project-settings-appearance`,
      title: `See Appearance, Keyboard shortcuts and Offline AI from a project`,
      purpose: `Reach the app wide settings without leaving the project.`,
      level: `Beginner`,
      prerequisites: [`Project Settings is open.`],
      steps: [
        {
          screen: `pages-settings-appearance`,
          do: `Click Appearance.`,
          explain: `Mode (System, Light, Dark) and the Theme cards, the same as in the Settings window. Choosing here changes the look of every project on this Mac.`,
        },
        {
          screen: `pages-settings-shortcuts`,
          do: `Click Keyboard shortcuts.`,
          explain: `The same list of 23 commands as in the Settings window, with a Filter commands box and Restore all defaults. Your changes apply in every project. Using it is taught in the Settings chapter.`,
        },
        {
          screen: `pages-settings-offline-ai`,
          do: `Click Offline AI.`,
          explain: `The state of the offline AI, AI health and Assistant memory. Here Assistant memory shows what OpenFrame AI remembers in this project. At the bottom, Technical details can be opened for more.`,
        },
      ],
      result: `You changed or checked an app setting without leaving the project.`,
      cautions: [
        `These three tabs are not project only. A change here changes the whole app. The footer says so.`,
      ],
      related: [`project-settings-app-settings`],
    },
    {
      id: `project-settings-app-settings`,
      title: `Jump to the Settings window`,
      purpose: `Open the app Settings window from Project Settings.`,
      level: `Beginner`,
      prerequisites: [`Project Settings is open.`],
      steps: [
        {
          screen: `pages-settings-general`,
          do: `Click App Settings… at the bottom right of the sheet.`,
          explain: `The Settings window opens with General, Appearance, Keyboard Shortcuts, Storage, Offline AI and Diagnostics. It is the place for your display name and for the diagnostics list.`,
        },
      ],
      result: `The Settings window is open.`,
      cautions: [],
      related: [`project-settings-open`],
    },
    {
      id: `project-settings-storage`,
      title: `Check where your library is kept from a project`,
      purpose: `Open the Storage tab to see the library, the folder for new projects and the Global Idea Vault.`,
      level: `Intermediate`,
      prerequisites: [`Project Settings is open.`],
      steps: [
        {
          screen: `project-settings-storage`,
          do: `Click Storage.`,
          explain: `Cards for Your OpenFrame library, Folder for new projects, Global Idea Vault and This project, each with its own buttons. The Settings chapter teaches each of them: see See where your library and projects are kept, Look after the Global Idea Vault and Move a project to another folder.`,
        },
      ],
      result: `You know where to find the storage controls.`,
      cautions: [],
      related: [`project-settings-location`],
    },
  ],
  screens: {
    'pages-settings-general': `Project Settings on the Project settings tab with the saved values: title, type, Language, Genre, Creator, Status, Logline and Project notes, and the Stored on this computer card. Revert is greyed out and Save is pale because nothing has changed. Used in Edit the title, type, status and other details.`,
    'pages-settings-general-edited': `The same tab after a logline has been typed. Revert and Save are active. Used in Edit the title, type, status and other details.`,
    'pages-settings-title-error': `The Title box has been cleared and is selected, showing its Title prompt. Used in Fix an empty title.`,
    'pages-settings-type-menu': `The Type menu open, showing Short Film as the other choice. Used in Edit the title, type, status and other details.`,
    'pages-settings-status-menu': `The Status menu open with the eight statuses and a tick on the current one. Used in Edit the title, type, status and other details.`,
    'pages-settings-saved': `The tab after Save, with the message Updated project settings, Undo and Dismiss. Used in Edit the title, type, status and other details.`,
    'pages-settings-writing': `The Writing tab: Check spelling as I write, Main language, Second language and an empty PROJECT DICTIONARY. Used in Turn spelling on and choose the languages.`,
    'pages-settings-shortcuts': `The Keyboard shortcuts tab inside Project Settings. It is the same list as in the Settings window. Used in See Appearance, Keyboard shortcuts and Offline AI from a project.`,
    'pages-settings-offline-ai': `The Offline AI tab inside Project Settings with the state card, AI health, Assistant memory and Technical details. Used in See Appearance, Keyboard shortcuts and Offline AI from a project.`,
    'pages-settings-advanced': `The Advanced tab, Categories: each breakdown category with twelve colours and sixteen icons, and Restore Defaults. Used in Change the colour and icon of a breakdown category.`,
    'pages-settings-appearance': `The Appearance tab inside Project Settings with Mode and the Theme cards. Used in See Appearance, Keyboard shortcuts and Offline AI from a project.`,
    'pages-settings-writing-word': `The Writing tab with a word in the project dictionary and its Remove button. Used in Add a word to the project dictionary.`,
    'pages-settings-language-menu': `The Main language menu open with the list of languages. Used in Turn spelling on and choose the languages.`,
  },
  shortcuts: [
    [`Shift+Command+,`, `Open Project Settings (default, changeable)`],
  ],
  limits: [
    `Title and type are required. Every other field on Project settings is optional.`,
    `The project dictionary belongs to one project.`,
    `Keyboard shortcuts, Offline AI and Appearance in this sheet are app wide, not project only.`,
    `Spelling uses the languages installed on your Mac.`,
  ],
  troubleshooting: [
    { problem: `My edits are gone.`, fix: `On Project settings, edits are kept only after Save. Click Undo in the message that follows a save to take back a saved change, or Revert before saving.` },
    { problem: `Save does nothing because the title is empty.`, fix: `Type a title in Title (required), or click Revert, then Save.` },
    { problem: `A character name is marked as a spelling mistake.`, fix: `Add it on Writing under PROJECT DICTIONARY, with Add Word. Character names you have created are never marked.` },
    { problem: `I changed the theme here and every project changed.`, fix: `Appearance in Project Settings is the app setting. It applies to every project on this Mac.` },
  ],
};
