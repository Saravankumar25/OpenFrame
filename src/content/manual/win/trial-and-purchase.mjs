export default {
  area: `Trial and purchase`,
  slug: `trial-and-purchase`,
  order: 31,
  part: `Project and data`,
  intro: [
    `OpenFrame Studio starts with a 7-day free trial. The Microsoft Store runs the trial and takes the purchase, so you pay and manage it the way you do for any other Microsoft Store app. This chapter shows what you see in OpenFrame at each stage: the trial counter, the reminder, the purchase, the file safety steps and the screens that appear if the purchase cannot be checked.`,
    `While the trial runs, a small chip in the top bar counts the days left, and a Buy full version button sits beside it. A few days before the trial ends, a reminder opens. You can keep working on the trial or buy the full version.`,
    `Your work is never held back by any of this. Your projects stay on your computer either way, and nothing is deleted when the trial ends. If the trial ends or a purchase stops being active, OpenFrame shows a screen with Buy full version and Restore purchase. Buying or restoring opens your projects again exactly as you left them.`,
    `Straight after you buy, OpenFrame shows a note about your data, a short four-step File safety tutorial and a welcome. They are there because your projects live only on your computer and nobody else holds a copy.`,
  ],
  layout: [
    { name: `Trial chip`, text: `In the top bar, beside the search box. It reads Trial: 7 days left and counts down, for example Trial: 3 days left. Its tooltip reads Free trial, 7 days left.` },
    { name: `Buy full version`, text: `An orange button next to the trial chip. Click it to start the purchase in the Microsoft Store. If OpenFrame is waiting to check a purchase, the button reads Check purchase instead.` },
    { name: `Trial reminder`, text: `A dialog titled Trial reminder with the line Your free trial ends in 3 days, a note that nothing is deleted, and the buttons Continue trial and Buy full version.` },
    { name: `Your data, your responsibility`, text: `A dialog shown once, right after you buy. Six numbered points, your root folder with a Show in File Explorer button, a tick box and OK, I understand.` },
    { name: `File safety tutorial`, text: `Four steps: Where it lives, Back up, New laptop and Check. Each step has Back and Next, and the last has Finish.` },
    { name: `Welcome to the OpenFrame Studio family`, text: `A thank you with OpenFrame's community links and an OK button.` },
    { name: `Check needed and locked screens`, text: `Checking your purchase, Your free trial has ended and Your purchase is no longer active. Each says your projects are safe and offers the way forward.` },
  ],
  workflows: [
    {
      id: `read-trial-status`,
      title: `See how many trial days are left`,
      purpose: `Read the trial counter and the reminder, and decide whether to keep trying or buy.`,
      level: `Beginner`,
      prerequisites: [`A project is open and your free trial is running.`],
      steps: [
        {
          screen: `licence-trial-chip`,
          do: `Look at the top bar, to the right of the search box.`,
          explain: `The chip reads Trial: 7 days left at the start. Hover over it for the tooltip, Free trial, 7 days left. Next to it is the orange Buy full version button.`,
        },
        {
          screen: `licence-trial-ending-chip`,
          do: `Look again as the trial runs down.`,
          explain: `The chip counts down, for example Trial: 3 days left. The rest of the window is unchanged. You can keep working normally.`,
        },
        {
          screen: `licence-trial-reminder`,
          do: `When the reminder opens, read it, then click Continue trial or Buy full version.`,
          explain: `It says Your free trial ends in 3 days and Buy the full version to keep using OpenFrame Studio. A note adds: Your projects stay on your computer either way. Nothing is deleted when the trial ends. Continue trial closes the reminder and you carry on. Close, the x, does the same.`,
        },
      ],
      result: `You know how many days are left and what happens when the trial ends.`,
      cautions: [
        `Nothing is deleted when the trial ends. Your projects stay on your computer.`,
      ],
      related: [`buy-full-version`, `trial-ended-locked`],
    },
    {
      id: `buy-full-version`,
      title: `Buy the full version`,
      purpose: `Start the purchase from OpenFrame and complete it in the Microsoft Store.`,
      level: `Beginner`,
      prerequisites: [`You are connected to the internet.`],
      steps: [
        {
          screen: `licence-trial-chip`,
          do: `Click Buy full version in the top bar. You can also click Buy full version in the Trial reminder, or on the screen that appears when the trial has ended.`,
          explain: `All of them start the same purchase. The Microsoft Store takes your payment.`,
        },
        {
          screen: `licence-buy-unavailable`,
          do: `If a message appears saying the Microsoft Store connection is not available, click Dismiss and try again later.`,
          explain: `The message says the Microsoft Store connection is not available, so the purchase cannot start. Nothing in your projects changes. Dismiss, or the x, closes the message.`,
        },
        {
          screen: `licence-data-note`,
          do: `After you have bought the full version, OpenFrame shows Your data, your responsibility. Go on to the next workflow.`,
          explain: `It is shown once, right after the purchase.`,
        },
      ],
      result: `The purchase is complete, and OpenFrame shows the data note.`,
      cautions: [
        `If the Microsoft Store cannot be reached, Buy full version does nothing but show the message. Check your internet connection and try again.`,
      ],
      related: [`read-data-note`, `check-your-purchase`],
    },
    {
      id: `read-data-note`,
      title: `Read the data note after you buy`,
      purpose: `Read and confirm the note about backing up your projects, which is shown once after the purchase.`,
      level: `Beginner`,
      prerequisites: [`You have just bought the full version.`],
      steps: [
        {
          screen: `licence-data-note`,
          do: `Read the dialog Your data, your responsibility. Scroll through the six numbered points.`,
          explain: `It says: Shown once, right after you buy the full version. A yellow notice says Important. Please read before you continue. The points are: 1 Your projects live only on this computer, and OpenFrame has no cloud, so your work cannot be recovered if the computer is lost, stolen, damaged or reset. 2 Back up the whole root folder to Google Drive or another cloud drive and, if you can, an external disk. 3 Moving to a new laptop is your job too: copy the same root folder to the new computer, install OpenFrame, then use Open Project. 4 Keep at least two copies, one on this computer and one somewhere else. 5 Never delete or rename the root folder by hand. Use OpenFrame to rename, archive or delete projects. 6 Backups inside OpenFrame are separate copies. Create Backup saves a copy, but keep it off this computer.`,
        },
        {
          screen: `licence-data-note`,
          do: `Click Show in File Explorer to see your root folder.`,
          explain: `YOUR ROOT FOLDER shows the folder path for your library. Show in File Explorer opens it in Windows so you know where it is.`,
        },
        {
          screen: `licence-data-note-read`,
          do: `Scroll to the end of the list. The tick box now becomes available.`,
          explain: `Until you reach the end, the tick box and OK, I understand are greyed out, and the note under them says: Scroll to the end of the list to continue.`,
        },
        {
          screen: `licence-data-note-confirmed`,
          do: `Tick I have read this and I understand that backing up my data is my responsibility, then click OK, I understand.`,
          explain: `OK, I understand turns on only after the box is ticked.`,
        },
      ],
      result: `You have confirmed the data note and OpenFrame moves on to the File safety tutorial.`,
      cautions: [
        `OpenFrame keeps no copy of your work. Back up your root folder and keep a second copy somewhere else.`,
      ],
      related: [`file-safety-tutorial`],
    },
    {
      id: `file-safety-tutorial`,
      title: `Complete the File safety tutorial`,
      purpose: `Learn where your data lives, how to back it up and how to move it to a new laptop, then confirm each point.`,
      level: `Beginner`,
      prerequisites: [`You have confirmed the data note.`],
      steps: [
        {
          screen: `licence-tutorial-step-1`,
          do: `Read step 1, Where your data lives, and click Show in File Explorer if you want to see the folder. Then click Next.`,
          explain: `It shows YOUR ROOT FOLDER and what is INSIDE IT: Projects, one folder for each project with its data, drafts and attachments; Backups, copies you make with Create Backup; and Global Idea Vault, ideas you keep across projects. It ends: Everything you create is inside this one folder. Remember this location. The bottom reads Step 1 of 4 · mandatory.`,
        },
        {
          screen: `licence-tutorial-step-2`,
          do: `Read step 2, Back up the root folder to Google Drive, and click Next.`,
          explain: `Install Google Drive for desktop, or your preferred cloud or external disk tool. Choose the root folder as a folder to sync, or copy it into your Drive folder. Check in the Drive website that your projects appear. Repeat after every working session, or turn on automatic sync. A note says: Close OpenFrame (or Create Backup) before copying so files are complete. An external drive works as well as Google Drive. Back returns to step 1.`,
        },
        {
          screen: `licence-tutorial-step-3`,
          do: `Read step 3, Moving to a new laptop, and click Next.`,
          explain: `Copy the whole root folder to a USB disk or Google Drive. Install OpenFrame Studio on the new laptop from the Microsoft Store, signed in with the same Microsoft account. Copy the folder to the new laptop. In OpenFrame, choose Open Project and pick the project inside the folder. A note adds that linked external files, for example a reference photo on another drive, must be copied too, and that OpenFrame lists missing ones after opening.`,
        },
        {
          screen: `licence-tutorial-step-4`,
          do: `On step 4, Final check, tick the three boxes.`,
          explain: `The three statements are: I know where my root folder is. I will back it up to Google Drive or an external disk regularly. I know I must copy the folder myself when I change laptops. Finish is greyed out until all three are ticked.`,
        },
        {
          screen: `licence-tutorial-step-4-ticked`,
          do: `With all three boxes ticked, click Finish.`,
          explain: `The progress at the top shows each step done: 1 Where it lives, 2 Back up, 3 New laptop and 4 Check.`,
        },
        {
          screen: `licence-welcome-family`,
          do: `Read the welcome, then click OK.`,
          explain: `The title is Welcome to the OpenFrame Studio family. It says: Thank you for backing independent filmmaking. Be a part of our community. It lists Instagram, YouTube, X, Facebook, LinkedIn, Reddit, Discord and Threads, all as @openframestudio. OK, or the x, closes it.`,
        },
      ],
      result: `You have finished the tutorial and you are back in your project, with the full version.`,
      cautions: [
        `The four steps are required. You cannot skip them. Go through them in order. Back returns to the step before.`,
        `Back up your root folder regularly. A copy on the same computer does not protect you.`,
      ],
      related: [`read-data-note`],
    },
    {
      id: `check-your-purchase`,
      title: `Check your purchase when OpenFrame cannot reach the Microsoft Store`,
      purpose: `Run the purchase check again when OpenFrame could not confirm your purchase.`,
      level: `Intermediate`,
      prerequisites: [`You have bought the full version, or you are not sure whether OpenFrame knows about your purchase.`],
      steps: [
        {
          screen: `licence-check-needed`,
          do: `Read the dialog Checking your purchase. Connect to the internet, then click Try again.`,
          explain: `It says: We could not reach the Microsoft Store. Connect to the internet to check your purchase. If you bought before, your purchase will be restored as soon as the check succeeds. A note adds: Your projects stay open and can be exported. Nothing is deleted. Not now closes the dialog and Buy full version starts a purchase.`,
        },
        {
          screen: `licence-check-purchase-button`,
          do: `If you choose Not now, look at the top bar. A Check purchase button now shows in place of Buy full version.`,
          explain: `Click Check purchase to run the check again whenever you are online.`,
        },
      ],
      result: `Once OpenFrame reaches the Microsoft Store, your purchase is confirmed and restored.`,
      cautions: [
        `Do not buy again if you have already bought. Connect to the internet and click Try again. Your purchase is restored as soon as the check succeeds.`,
      ],
      related: [`buy-full-version`, `trial-ended-locked`],
    },
    {
      id: `trial-ended-locked`,
      title: `Buy or restore when the trial has ended or a purchase is no longer active`,
      purpose: `Open your projects again after the trial ends, or after a refunded or revoked purchase.`,
      level: `Beginner`,
      prerequisites: [`The trial has ended, or the Microsoft Store has reported that your purchase is no longer active.`],
      steps: [
        {
          screen: `licence-locked-trial-ended-microsoft-store`,
          do: `Read Your free trial has ended. Click Buy full version to buy, or Restore purchase if you already bought.`,
          explain: `It says: Buy the full version to open your projects again. Your projects are safe on this computer. Nothing is deleted. Buying or restoring opens them again exactly as you left them.`,
        },
        {
          screen: `licence-locked-buy-unavailable`,
          do: `If the same screen adds that the Microsoft Store connection is not available, try again later.`,
          explain: `The extra message means the purchase cannot start for now. Your projects stay safe on this computer.`,
        },
        {
          screen: `licence-locked-revoked`,
          do: `If you see Your purchase is no longer active, read why and choose Buy full version or Restore purchase.`,
          explain: `It says: The Microsoft Store reports that this purchase was refunded or revoked. Buy the full version to open your projects again. The same promise applies: your projects are safe and nothing is deleted.`,
        },
      ],
      result: `After you buy or restore, your projects open again exactly as you left them.`,
      cautions: [
        `Nothing is deleted at any point. Your projects stay on your computer.`,
        `Click Restore purchase, not Buy full version, if you have already bought. It brings back the purchase you already have.`,
      ],
      related: [`buy-full-version`, `check-your-purchase`],
    },
  ],
  screens: {
    'licence-trial-chip': `The top bar of a project during the free trial. It shows the chip Trial: 7 days left and the orange Buy full version button beside the search box. Used in read-trial-status and buy-full-version.`,
    'licence-buy-unavailable': `The project window after Buy full version is clicked when the Microsoft Store connection is not available. A message with a close button says so. Used in buy-full-version.`,
    'licence-trial-reminder': `The Trial reminder dialog. It says Your free trial ends in 3 days, that your projects stay on your computer and nothing is deleted, and has Continue trial and Buy full version. Used in read-trial-status.`,
    'licence-trial-ending-chip': `The top bar near the end of the trial, with the chip reading Trial: 3 days left. Used in read-trial-status.`,
    'licence-data-note': `The Your data, your responsibility dialog with its six points, YOUR ROOT FOLDER and Show in File Explorer. The tick box and OK, I understand are greyed out, and the note reads Scroll to the end of the list to continue. Used in buy-full-version and read-data-note.`,
    'licence-data-note-read': `The data note scrolled to the end. The tick box is available but not ticked, and OK, I understand is still greyed out. Used in read-data-note.`,
    'licence-data-note-confirmed': `The data note with the tick box ticked, so OK, I understand is ready. Used in read-data-note.`,
    'licence-tutorial-step-1': `File safety tutorial, step 1 of 4, Where your data lives. Shows the root folder, Show in File Explorer and what is inside it. Next is the only way on. Used in file-safety-tutorial.`,
    'licence-tutorial-step-2': `File safety tutorial, step 2 of 4, Back up the root folder to Google Drive, with the four actions and a note about closing OpenFrame or using Create Backup first. Back and Next. Used in file-safety-tutorial.`,
    'licence-tutorial-step-3': `File safety tutorial, step 3 of 4, Moving to a new laptop, with the four actions and a note about linked external files. Back and Next. Used in file-safety-tutorial.`,
    'licence-tutorial-step-4': `File safety tutorial, step 4 of 4, Final check, with three unticked statements. Finish is greyed out. Used in file-safety-tutorial.`,
    'licence-tutorial-step-4-ticked': `The Final check with all three statements ticked, so Finish is ready. Used in file-safety-tutorial.`,
    'licence-welcome-family': `The Welcome to the OpenFrame Studio family dialog, with a thank you and OpenFrame's community links. Close and OK. Used in file-safety-tutorial.`,
    'licence-check-needed': `The Checking your purchase dialog. It says the Microsoft Store could not be reached and offers Not now, Buy full version and Try again. Used in check-your-purchase.`,
    'licence-check-purchase-button': `The top bar with a Check purchase button in place of Buy full version. Used in check-your-purchase.`,
    'licence-locked-trial-ended-microsoft-store': `The Your free trial has ended screen, with Buy full version and Restore purchase. It says your projects are safe and nothing is deleted. Used in trial-ended-locked.`,
    'licence-locked-buy-unavailable': `The same screen with an added message that the Microsoft Store connection is not available. Variant, part of trial-ended-locked.`,
    'licence-locked-revoked': `The Your purchase is no longer active screen. It says the Microsoft Store reports the purchase was refunded or revoked, with Buy full version and Restore purchase. Used in trial-ended-locked.`,
  },
  shortcuts: [],
  limits: [
    `The Microsoft Store runs the 7-day trial and takes the purchase.`,
    `The data note and the File safety tutorial are shown once after a purchase. The four tutorial steps are required.`,
    `OpenFrame keeps no cloud copy of your projects. Backing them up is your job.`,
  ],
  troubleshooting: [
    {
      problem: `I clicked Buy full version and a message says the Microsoft Store connection is not available.`,
      fix: `The purchase did not start. Click Dismiss and try again later. Your projects are unchanged.`,
    },
    {
      problem: `OpenFrame says it could not reach the Microsoft Store, but I already bought.`,
      fix: `Connect to the internet and click Try again, or click Check purchase in the top bar. Your purchase is restored as soon as the check succeeds. Your projects stay open meanwhile.`,
    },
    {
      problem: `The trial has ended. Are my projects gone?`,
      fix: `No. Your projects are safe on this computer and nothing is deleted. Buy the full version, or click Restore purchase if you already bought, and they open again exactly as you left them.`,
    },
    {
      problem: `OK, I understand is greyed out on the data note.`,
      fix: `Scroll to the end of the list, then tick the box. The button turns on after both.`,
    },
    {
      problem: `Finish is greyed out in the File safety tutorial.`,
      fix: `Tick all three statements on step 4, Final check.`,
    },
    {
      problem: `I see Your purchase is no longer active.`,
      fix: `The Microsoft Store reports that the purchase was refunded or revoked. Click Buy full version to buy again, or Restore purchase if you think this is a mistake. Your projects are safe either way.`,
    },
  ],
};
