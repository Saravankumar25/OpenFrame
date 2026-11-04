export default {
  area: `Library setup`,
  slug: `library-setup`,
  order: 1,
  part: `Start`,
  intro: [
    `Before Straft Studio shows you anything else, it asks where to keep your work. Everything you make lives in one folder called your Straft library. The library holds your projects and your Global Idea Vault, the place for ideas that do not belong to one film yet.`,
    `The Set up your Straft library screen has three choices along the top: Use the default location, Choose a new location and Locate existing Straft data. Click one and the panel under it changes. You pick once. You can move the library later in Settings, on the Storage tab.`,
    `Most people click Create Library Here on the first choice and carry on. The other two choices are for people who want the library on another drive, and for people who already have a Straft library from an earlier install or another computer.`,
  ],
  layout: [
    { name: `Heading and explanation`, text: `The title Set up your Straft library, and a short paragraph saying that Straft keeps your projects and your Global Idea Vault in one folder, and that you can move it later in Settings > Storage.` },
    { name: `Choice bar`, text: `Three buttons side by side: Use the default location, Choose a new location and Locate existing Straft data. The selected one is dark. Only one is selected at a time.` },
    { name: `Panel under the choice bar`, text: `Changes with the choice. For the default location it shows the folder path, a note about the folder and the Create Library Here button. For the other two it shows a short explanation and a Choose Folder… button.` },
  ],
  workflows: [
    {
      id: `create-library-default`,
      title: `Create your library in the default location`,
      purpose: `Create your Straft library in the folder Straft suggests, so you can start working.`,
      level: `Beginner`,
      prerequisites: [`Straft Studio is installed and you are opening it for the first time.`],
      steps: [
        {
          screen: `app-library-setup-default`,
          do: `Open Straft Studio. The Set up your Straft library screen appears with Use the default location already selected.`,
          explain: `The panel under the choice bar shows the folder Straft will use. The note under the path explains that the folder is inside your own Windows user folder, outside Documents, and that Windows does not sync it to OneDrive, so cloud syncing cannot lock or replace your files while you work. It also reminds you to back up this folder.`,
        },
        {
          screen: `app-library-setup-default`,
          do: `Click Create Library Here.`,
          explain: `Straft creates the library in the folder shown and moves on. If you want a different place, read the next workflow before you click.`,
        },
        {
          screen: `app-first-launch-welcome`,
          do: `Continue with the welcome screen that appears next.`,
          explain: `The welcome screen asks for your name. See the First launch chapter.`,
        },
      ],
      result: `Your library exists in the default folder and Straft asks for your name.`,
      cautions: [
        `The library holds your work. Back up this folder from time to time, as the note on the screen says.`,
      ],
      related: [`create-library-new-location`, `reconnect-existing-library`],
    },
    {
      id: `create-library-new-location`,
      title: `Create your library in a folder you choose`,
      purpose: `Put your Straft library inside a folder of your own, for example on a larger drive.`,
      level: `Beginner`,
      prerequisites: [`The Set up your Straft library screen is open.`],
      steps: [
        {
          screen: `app-library-setup-default`,
          do: `Click Choose a new location in the choice bar.`,
          explain: `The choice bar changes the panel below it. Use the default location, the first choice, is what you see when the screen opens.`,
        },
        {
          screen: `app-library-setup-new-location`,
          do: `Click Choose Folder… and pick a folder in the Windows folder window that opens.`,
          explain: `The panel says: Choose a folder. Straft makes a folder named Straft inside it for your library. So pick the place that should contain the library, not a folder you have already filled with other files. Straft adds its own Straft folder inside the one you pick.`,
        },
      ],
      result: `Straft creates a folder named Straft inside the folder you picked and uses it as your library.`,
      cautions: [
        `If you change your mind later, use Move Library… in Settings on the Storage tab. See the Application Home chapter.`,
        `Do not pick a folder that you sync to a cloud drive while you work. The default folder avoids that on purpose.`,
      ],
      related: [`create-library-default`],
    },
    {
      id: `reconnect-existing-library`,
      title: `Reconnect an existing Straft library`,
      purpose: `Point Straft at a library you already have, after a reinstall or when you brought it from another drive.`,
      level: `Intermediate`,
      prerequisites: [
        `You have a Straft library folder from before. For example you reinstalled Windows, moved the folder, or copied it from another computer.`,
      ],
      steps: [
        {
          screen: `app-library-setup-default`,
          do: `Click Locate existing Straft data in the choice bar.`,
          explain: `Use this choice instead of Create Library Here when your work already exists. Creating a new library would leave your old projects out of sight.`,
        },
        {
          screen: `app-library-setup-locate`,
          do: `Click Choose Folder… and pick the folder that holds your library.`,
          explain: `The panel explains that Straft shows you what it found before connecting anything, and that it never makes an empty library in its place. Pick the library folder itself, the one that holds your projects, your Global Idea Vault and a file called library.json.`,
        },
      ],
      result: `Straft connects to your existing library and your projects appear on Application Home.`,
      cautions: [
        `If you pick the wrong folder, Straft does not connect it without showing you what it found first. Check that what it lists is your work.`,
      ],
      related: [`create-library-default`],
    },
  ],
  screens: {
    'app-library-setup-default': `The Set up your Straft library screen with Use the default location selected. It shows the suggested folder, a note that the folder sits outside Documents and is not synced to OneDrive, and the Create Library Here button. Used in the create-library-default workflow, and as the starting point for the other two.`,
    'app-library-setup-new-location': `The same screen with Choose a new location selected. The panel explains that Straft makes a folder named Straft inside the folder you choose, and offers Choose Folder…. Used in the create-library-new-location workflow.`,
    'app-library-setup-locate': `The same screen with Locate existing Straft data selected. The panel explains that Straft shows what it found before connecting anything and never makes an empty library in its place, and offers Choose Folder…. Used in the reconnect-existing-library workflow.`,
  },
  shortcuts: [],
  limits: [
    `You choose the library location once, here. After that, use Settings > Storage to move it or to point Straft at a different library.`,
    `Straft does not sync your library to any cloud service. Backing it up is up to you.`,
  ],
  troubleshooting: [
    {
      problem: `I clicked Create Library Here but I wanted a different drive.`,
      fix: `Open Settings on Application Home, choose the Storage tab and click Move Library…. Moving the library is covered in the Application Home chapter.`,
    },
    {
      problem: `My old projects are not showing after a reinstall.`,
      fix: `Your library folder is still on the disk. If Straft created a new library by mistake, open Settings, choose Storage and click Locate Existing Library… to point it at the old folder.`,
    },
    {
      problem: `I do not know which folder holds my library.`,
      fix: `On Application Home, open Settings, choose the Storage tab and read YOUR STRAFT LIBRARY. It shows the folder path, and Open Folder shows it in Windows.`,
    },
  ],
};
