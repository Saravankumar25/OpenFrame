export default {
  area: `Search`,
  slug: `search`,
  order: 4,
  part: `Start`,
  intro: [
    `Search and Commands is one window that does two jobs. Type a word and it finds scenes, ideas, characters, locations and files in your project. Or type the name of a command, such as Go to Story, and run it without using the menus.`,
    `Open it with Command-K, from Search and Commands… in the Edit menu, or, inside a project, with Search in the toolbar. Press Esc to close it without choosing anything.`,
    `On Application Home the window lists a few app commands, such as New Project and Settings. Inside a project it lists many more, including a Go to command for every page.`,
  ],
  layout: [
    { name: `Search box`, text: `At the top of the window. Its hint reads Search scenes, ideas, characters, locations, files, or type a command. The word esc at the right reminds you that Esc closes the window.` },
    { name: `Command list`, text: `Shown when the box is empty. Each row has an icon, the command name, and its keys at the right when it has them.` },
    { name: `Results list`, text: `Shown when you type. Each result has a type label such as Scene, a title, a short piece of text around your word with the match in bold, and a note at the right that says where it was found.` },
    { name: `No matches message`, text: `Shown when nothing matches. It says No matches. and explains what Search looks at.` },
  ],
  workflows: [
    {
      id: `search-open-and-run-commands`,
      title: `Open Search and Commands and run a command`,
      purpose: `Find a command by name or by looking at the list, and run it.`,
      level: `Beginner`,
      prerequisites: [`Straft Studio is open on Application Home or in a project.`],
      steps: [
        {
          screen: `search-palette-home`,
          do: `On Application Home, press Command-K.`,
          explain: `The window opens over the page with the box empty. It lists five commands: New Project (Command-N), Open Project… (Command-O), Settings (Command-comma), Keyboard Shortcuts and About Straft Studio. Choose one to run it.`,
        },
        {
          screen: `search-palette-home-commands`,
          do: `Type a word, for example settings.`,
          explain: `The list narrows to the commands that match. Here only Command: Settings remains. Choose it to run it.`,
        },
        {
          screen: `search-palette-project`,
          do: `Inside a project, click Search in the toolbar or press Command-K.`,
          explain: `The list now holds project commands: New Project, Close Project (Shift-Command-W), and Go to Home, Idea Vault, Story, Screenplay, Breakdown, Production, Call Sheets and Files, with the keys Control-Command-1 to 8. Go to Notes and Tasks, Go to Activity, Go to Recently Deleted, Go to Production · Overview and Go to Production · Catalog follow. Scroll for more.`,
        },
        {
          screen: `search-palette-project`,
          do: `Click a command, for example Go to Story.`,
          explain: `The window closes and the page opens. Press Esc instead to close the window and stay where you are.`,
        },
      ],
      result: `The command you chose runs, or the window closes and nothing changes.`,
      cautions: [
        `Commands in this window are the same ones as in the menus. The keys at the right of a row are the same keys as the menu shows.`,
      ],
      related: [`search-find-in-project`],
    },
    {
      id: `search-find-in-project`,
      title: `Search inside a project`,
      purpose: `Find a scene, idea, character, location or file by a word, and jump to it.`,
      level: `Beginner`,
      prerequisites: [`A project is open. Searching for scenes works best once the project has a screenplay.`],
      steps: [
        {
          screen: `search-palette-project`,
          do: `Click Search in the toolbar, or press Command-K.`,
          explain: `The box is ready to type. The hint tells you what can be found: scenes, ideas, characters, locations and files.`,
        },
        {
          screen: `search-palette-results`,
          do: `Type a word from your script, for example a character name.`,
          explain: `Matching items are listed. In the picture, kiran lists scenes. Each shows the label Scene, the scene number and heading, a short piece of text with the word in bold, and where it was found, for example Screenplay followed by the screenplay and draft names. The first result is highlighted.`,
        },
        {
          screen: `search-palette-results`,
          do: `Click a result.`,
          explain: `The window closes and the project opens at that item. Press Esc to close without choosing.`,
        },
      ],
      result: `You land on the scene or item you were looking for.`,
      cautions: [
        `The window looks at titles, text, notes, file names and tags. A word that is not in any of these will not be found.`,
      ],
      related: [`search-nothing-found`],
    },
    {
      id: `search-nothing-found`,
      title: `What to do when nothing is found`,
      purpose: `Read the No matches message and try a different search.`,
      level: `Beginner`,
      prerequisites: [`Search and Commands is open.`],
      steps: [
        {
          screen: `search-palette-empty`,
          do: `Type a word that is not in your project. The picture uses zzzz nothing.`,
          explain: `The window says No matches. Search looks at titles, text, notes, file names and tags.`,
        },
        {
          screen: `search-palette-empty`,
          do: `Change the word, or delete it to see the commands again.`,
          explain: `Try a shorter word, or a different spelling. An empty box shows the command list.`,
        },
      ],
      result: `You find the item with a better word, or you close the window with Esc.`,
      cautions: [
        `On Application Home, this window finds commands. To find a project by name, use the Search box on the Home page. See the Home chapter.`,
      ],
      related: [`search-find-in-project`],
    },
  ],
  screens: {
    'search-palette-home': `Search and Commands over Application Home with an empty box. It lists New Project (Command-N), Open Project… (Command-O), Settings (Command-comma), Keyboard Shortcuts and About Straft Studio. Used in search-open-and-run-commands.`,
    'search-palette-home-commands': `The window with settings typed. Only the Settings command is listed. Used in search-open-and-run-commands.`,
    'search-palette-empty': `The window with a word that matches nothing. It says No matches. Search looks at titles, text, notes, file names and tags. Used in search-nothing-found.`,
    'search-palette-project': `Search and Commands inside a project with an empty box. It lists project commands such as New Project, Close Project, the Go to commands with their keys, and Go to Production · Overview and Catalog. Used in search-open-and-run-commands and search-find-in-project.`,
    'search-palette-results': `Search results inside a project for the word kiran. Each result is labelled Scene, shows the scene heading and a piece of text with the match in bold, and says where it was found. The first result is highlighted. Used in search-find-in-project.`,
  },
  shortcuts: [
    [`Command-K`, `Open Search and Commands`],
    [`Esc`, `Close Search and Commands without choosing`],
    [`Command-N`, `New Project, as shown in the command list`],
    [`Command-O`, `Open Project, as shown in the command list`],
    [`Command-,`, `Settings, as shown in the command list`],
    [`Shift-Command-W`, `Close Project, as shown in the command list`],
    [`Control-Command-1 to 8`, `Go to Home, Idea Vault, Story, Screenplay, Breakdown, Production, Call Sheets and Files`],
  ],
  limits: [
    `Search and Commands looks at titles, text, notes, file names and tags in the open project.`,
    `On Application Home it lists commands only. It does not find projects by name. The Search box on the Home page does that.`,
  ],
  troubleshooting: [
    {
      problem: `Search shows no scenes.`,
      fix: `The project may not have a screenplay yet. Write or import one, then search again.`,
    },
    {
      problem: `I only see commands, not my words.`,
      fix: `The box is empty or holds a command name. Type a word from your script, notes or file names.`,
    },
    {
      problem: `The window will not go away.`,
      fix: `Press Esc to close it.`,
    },
  ],
};
