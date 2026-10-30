export default {
  area: `Library setup`,
  slug: `library-setup`,
  order: 1,
  part: `Start`,
  intro: [
    `OpenFrame keeps your projects and your Global Idea Vault together in one folder, your OpenFrame library. The Set up your OpenFrame library screen is where you choose that folder. You see it when OpenFrame has no library to use yet, for example the first time you open the app, or after a reinstall.`,
    `The screen offers three choices, shown as three segments: Use the default location, Choose a new location and Locate existing OpenFrame data. The first two make a new library. The third reconnects a library you already have.`,
    `You are not locked in. The first line on the screen says you can move the library later in Settings > Storage. Quit OpenFrame at the bottom right closes the app without making anything.`,
  ],
  layout: [
    { name: `Heading and explanation`, text: `The title Set up your OpenFrame library, and a paragraph that says OpenFrame keeps your projects and your Global Idea Vault in one folder.` },
    { name: `Choice bar`, text: `Three segments: Use the default location, Choose a new location and Locate existing OpenFrame data. The selected segment is filled with orange.` },
    { name: `Choice panel`, text: `The box under the bar. It changes with the segment: the default folder and Create Library Here, a Choose Folder… button with Create Library, or a Choose Folder… button for an existing library.` },
    { name: `Quit OpenFrame`, text: `A button at the bottom right of the screen. It closes the app and creates nothing.` },
  ],
  workflows: [
    {
      id: `library-setup-default`,
      title: `Create your library in the default location`,
      purpose: `Make a new OpenFrame library in the folder OpenFrame suggests.`,
      level: `Beginner`,
      prerequisites: [`OpenFrame Studio is open on the Set up your OpenFrame library screen.`],
      steps: [
        {
          screen: `library-setup-default`,
          do: `Read the screen. Leave Use the default location selected.`,
          explain: `The first segment is selected when the screen opens. The panel shows the full path of a folder named OpenFrame in your own home folder. A grey note explains why: the folder is outside Documents, and iCloud Drive does not sync it with Desktop & Documents, so cloud syncing can never lock or replace your files while you work. The note ends with Back up this folder.`,
        },
        {
          screen: `library-setup-default`,
          do: `Click Create Library Here.`,
          explain: `OpenFrame makes the library in that folder. Next you see the Welcome sheet, which asks for your name. See the Home chapter.`,
        },
      ],
      result: `A library exists in the default folder, and OpenFrame moves on to the welcome step.`,
      cautions: [
        `Back up the library folder. OpenFrame stores your work on this Mac only, and the screen reminds you to back it up.`,
        `If you click Quit OpenFrame instead, the app closes and no library is made. Open the app again to return to this screen.`,
      ],
      related: [`library-setup-new-location`, `library-setup-locate`],
    },
    {
      id: `library-setup-new-location`,
      title: `Create your library in a folder you choose`,
      purpose: `Put a new library on another disk or in a folder of your own.`,
      level: `Beginner`,
      prerequisites: [`OpenFrame Studio is open on the Set up your OpenFrame library screen.`, `The folder you want to use already exists, for example on an external drive.`],
      steps: [
        {
          screen: `library-setup-new-location`,
          do: `Click Choose a new location in the choice bar.`,
          explain: `The panel now says: Choose a folder. OpenFrame makes a folder named OpenFrame inside it for your library. There is no Create Library button yet, because no folder is chosen.`,
        },
        {
          screen: `library-setup-new-location`,
          do: `Click Choose Folder… and pick a folder in the macOS window that opens.`,
          explain: `Pick the folder that should contain the library, not the library itself. OpenFrame adds the folder named OpenFrame inside the one you pick.`,
        },
        {
          screen: `library-setup-new-location-chosen`,
          do: `Read the line Your library will be followed by a path. Check that it is the place you want.`,
          explain: `The path ends with OpenFrame. A Create Library button now appears under it. To change your mind, click Choose Folder… again and pick another folder.`,
        },
        {
          screen: `library-setup-new-location-chosen`,
          do: `Click Create Library.`,
          explain: `OpenFrame makes the library there and moves on to the welcome step.`,
        },
      ],
      result: `A library exists in a new OpenFrame folder inside the folder you picked.`,
      cautions: [
        `If you use an external drive, keep it connected whenever you work. A project on a disconnected drive cannot open. See the Recovery chapter.`,
        `You can move the library later in Settings > Storage.`,
      ],
      related: [`library-setup-default`, `library-setup-locate`],
    },
    {
      id: `library-setup-locate`,
      title: `Reconnect a library you already have`,
      purpose: `Point OpenFrame at an existing library after a reinstall or after you moved the library or brought it from another drive.`,
      level: `Intermediate`,
      prerequisites: [`The OpenFrame library folder exists on this Mac or on a connected drive.`],
      steps: [
        {
          screen: `library-setup-locate`,
          do: `Click Locate existing OpenFrame data in the choice bar.`,
          explain: `The panel says: After a reinstall, or when you moved your library or brought it from another drive, choose its folder. OpenFrame shows what it found before connecting anything, and never makes an empty library in its place.`,
        },
        {
          screen: `library-setup-locate`,
          do: `Click Choose Folder… and pick the library folder in the macOS window that opens.`,
          explain: `Choose the library folder you used before. OpenFrame shows you what it found before it connects anything, so you can check it is the right one.`,
        },
      ],
      result: `OpenFrame is connected to your existing library, with your projects and Global Idea Vault in it.`,
      cautions: [
        `This choice never creates a new empty library. If you want a fresh library, use Use the default location or Choose a new location instead.`,
      ],
      related: [`library-setup-default`, `library-setup-new-location`],
    },
  ],
  screens: {
    'library-setup-default': `Set up your OpenFrame library with Use the default location selected. The panel shows the default folder path, a note about iCloud Drive and Desktop & Documents, and the Create Library Here button. Quit OpenFrame is at the bottom right. Used in library-setup-default.`,
    'library-setup-new-location': `The same screen with Choose a new location selected. The panel says to choose a folder and that OpenFrame makes a folder named OpenFrame inside it. Only Choose Folder… is offered. Used in library-setup-new-location.`,
    'library-setup-new-location-chosen': `Choose a new location after a folder is picked. A line says Your library will be and shows the path ending in OpenFrame, and a Create Library button appears. Used in library-setup-new-location.`,
    'library-setup-locate': `The screen with Locate existing OpenFrame data selected. The panel explains that OpenFrame shows what it found before connecting anything and never makes an empty library in its place. Only Choose Folder… is offered. Used in library-setup-locate.`,
  },
  shortcuts: [],
  limits: [
    `The library is one folder on this Mac or on a drive connected to it. OpenFrame does not keep a copy in the cloud.`,
    `Locate existing OpenFrame data connects a library you already have. It does not make a new one.`,
    `The choices on this screen can be changed later in Settings > Storage.`,
  ],
  troubleshooting: [
    {
      problem: `I do not see Create Library after clicking Choose a new location.`,
      fix: `Click Choose Folder… and pick a folder. Create Library appears under the path once a folder is chosen.`,
    },
    {
      problem: `I reinstalled OpenFrame and my projects are missing.`,
      fix: `Click Locate existing OpenFrame data, then Choose Folder…, and pick your old library folder. Do not create a new library.`,
    },
    {
      problem: `I want my library somewhere other than the default folder.`,
      fix: `Click Choose a new location and pick a folder. You can also move the library later in Settings > Storage.`,
    },
  ],
};
