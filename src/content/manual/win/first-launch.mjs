export default {
  area: `First launch`,
  slug: `first-launch`,
  order: 2,
  part: `Start`,
  intro: [
    `The first time you open OpenFrame Studio, a Welcome dialog asks one question: what should we call you? That is all. There is no account to create, no sign-in and no email address to give.`,
    `Your name appears on notes, tasks and project activity. It stays on this computer. You can change it at any time from the round avatar at the top right of Application Home.`,
    `When you click Continue, OpenFrame opens Application Home, where you create or open your first project.`,
  ],
  layout: [
    { name: `Welcome dialog`, text: `The title Welcome to OpenFrame Studio, the question What should we call you?, one box labelled Your name *, a short note and the Continue button.` },
    { name: `Your name * box`, text: `A required text box. The star means it cannot be left empty. It is filled in with a suggested name when the dialog opens, and you can replace it.` },
    { name: `Note under the box`, text: `Says your name appears on notes, tasks and project activity, that there is no account, that it stays on this computer, and that you can change it any time from the avatar on the Home screen.` },
    { name: `Continue`, text: `Saves your name and opens Application Home. It is greyed out while the name box is empty.` },
  ],
  workflows: [
    {
      id: `enter-your-name`,
      title: `Tell OpenFrame your name`,
      purpose: `Enter the name OpenFrame shows on your notes, tasks and activity, and reach Application Home.`,
      level: `Beginner`,
      prerequisites: [`You have chosen where your library lives. See the Library setup chapter.`],
      steps: [
        {
          screen: `app-first-launch-welcome`,
          do: `Read the Welcome to OpenFrame Studio dialog. Click in the Your name * box and type the name you want others to see on your notes and tasks.`,
          explain: `The box is required. Use the name your team knows you by. Nothing is sent anywhere: the name is stored on this computer only.`,
        },
        {
          screen: `app-first-launch-name-required`,
          do: `If you clear the box, the message Enter your name. appears and Continue stays greyed out. Type a name to make the message go away.`,
          explain: `This is the empty state of the same dialog. The name is used on notes, tasks and project activity.`,
        },
        {
          screen: `app-first-launch-welcome`,
          do: `Click Continue.`,
          explain: `The dialog closes and Application Home opens.`,
        },
        {
          screen: `app-home-empty`,
          do: `Look at Application Home. It says Create your first project. and offers Create Project, Open Existing Project and Restore Backup or Package….`,
          explain: `Your initials, MI in the picture, show in the avatar at the top right. Click the avatar whenever you want to change your name. Next, create a project: see the Application Home chapter.`,
        },
      ],
      result: `OpenFrame knows your name and you are on Application Home, ready to make your first project.`,
      cautions: [
        `The Your name * box cannot be empty. Without a name, Continue stays greyed out and the message Enter your name. shows.`,
        `OpenFrame has no sign-in. If you were expecting to log in, there is nothing to do: the name is all it needs.`,
      ],
      related: [`change-your-name`, `create-first-project`],
    },
  ],
  screens: {
    'app-first-launch-welcome': `The Welcome to OpenFrame Studio dialog with a name typed in the Your name * box and Continue ready to click. Used in the enter-your-name workflow.`,
    'app-first-launch-name-required': `The same dialog with the name box empty. The message Enter your name. is shown and Continue is greyed out. Variant of the Welcome screen, part of the enter-your-name workflow.`,
  },
  shortcuts: [],
  limits: [
    `There is no account, password or sign-in. OpenFrame asks only for a name.`,
    `The name is stored on this computer and used on notes, tasks and project activity.`,
  ],
  troubleshooting: [
    {
      problem: `Continue is greyed out.`,
      fix: `The name box is empty. Type a name and the message Enter your name. goes away.`,
    },
    {
      problem: `I typed the wrong name.`,
      fix: `Click the avatar at the top right of Application Home and choose a new name in the Your name dialog. See the Application Home chapter.`,
    },
  ],
};
