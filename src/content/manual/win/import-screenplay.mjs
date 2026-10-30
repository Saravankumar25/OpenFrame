export default {
  area: `Import a screenplay`,
  slug: `import-screenplay`,
  order: 4,
  part: `Start`,
  intro: [
    `If you already have a script, you do not have to retype it. Import Screenplay reads a script from a file, from text you paste, or from one of the sample scripts, and shows you how it understood it before anything is created.`,
    `The dialog has three steps, shown as chips at the top: 1 Select source, 2 Preview interpretation and 3 Import. You pick a source, check the preview, then import. Nothing in your project changes until you click Import.`,
    `Importing never replaces your current screenplay. If a screenplay already exists, the script arrives as a new screenplay or a new draft. When it is done, an Import complete summary tells you exactly what was added and says nothing else in your project was changed.`,
  ],
  layout: [
    { name: `Step chips`, text: `1 Select source, 2 Preview interpretation and 3 Import. The current step is dark.` },
    { name: `Choose a file`, text: `A dashed box on the left. It lists the accepted types: PDF, Final Draft (.fdx), Fountain, TXT and DOCX, and has a Browse… button.` },
    { name: `...OR PASTE SCREENPLAY TEXT`, text: `A large box on the right where you can paste a script. It shows a short example of how a screenplay looks.` },
    { name: `OR OPEN A SAMPLE SCRIPT`, text: `Four buttons for fictional scripts written and owned by OpenFrame, for learning the app: Example Feature, Example Short, Example Scene and FDX Interchange Test.` },
    { name: `Preview`, text: `The preview shows the source file and format, the number of Scenes, Characters and Pages, small chips such as High confidence and the title found, a notice about warnings, and a table of the scene headings it detected.` },
    { name: `Import options`, text: `IMPORT AS with New Screenplay, a Draft name box, and a tick box called Keep the original file in Project Files. Back, Cancel and Import are at the bottom.` },
  ],
  workflows: [
    {
      id: `import-screenplay-file`,
      title: `Import a screenplay from a file`,
      purpose: `Bring a script from a PDF, Final Draft, Fountain, TXT or DOCX file into your project as a new screenplay.`,
      level: `Beginner`,
      prerequisites: [
        `A project is open.`,
        `Your script is in one of these types: PDF, Final Draft (.fdx), Fountain, TXT or DOCX.`,
      ],
      steps: [
        {
          screen: `shell-project-home`,
          do: `On your project's Home page, look at the top bar. Click the + button, which is called Create.`,
          explain: `The Create menu is the same on every page of the project. On a project with no screenplay yet, the Home page also has a Write/Import Screenplay card.`,
        },
        {
          screen: `shell-create-menu`,
          do: `Click Import Screenplay….`,
          explain: `Import Screenplay… sits between New Moodboard and New Note in the CREATE menu.`,
        },
        {
          screen: `sp-import-source`,
          do: `The Import Screenplay dialog opens on step 1, Select source. Click Browse… in the Choose a file box and pick your script in the Windows window that opens.`,
          explain: `The dialog asks: What existing script do you want to bring into OpenFrame? The note at the bottom says: Importing never replaces your current screenplay. If one exists, the script arrives as a new screenplay or a new draft. Cancel closes the dialog and changes nothing. Preview is the next step.`,
        },
        {
          screen: `sp-import-preview`,
          do: `Click Preview. Read the preview before you go on.`,
          explain: `The Source tile names the file and its format, for example Example-Feature.fountain, Fountain. Scenes, Characters and Pages are counts of what was found, and the page count is approximate. Chips such as High confidence, the title and the author, and a count of dialogue blocks tell you how well the script was understood. The table lists each DETECTED SCENE HEADING with a STATUS, shown as OK when it was read cleanly.`,
        },
        {
          screen: `sp-import-preview`,
          do: `If a blue notice says there are warnings, click Show warnings to read them.`,
          explain: `The notice says, for example: 2 warnings. The screenplay structure was recognised. Some details were adjusted on the way in. Warnings are things OpenFrame changed to fit, not failures. Read them so you know what to check later.`,
        },
        {
          screen: `sp-import-preview`,
          do: `Next to IMPORT AS the choice is New Screenplay. Type a name in Draft name, or leave it empty. Leave Keep the original file in Project Files ticked if you want the source kept.`,
          explain: `The grey text in Draft name shows the name OpenFrame will use, for example Example-Feature (imported). Under the box it says: Leave empty to name it after the file. Keeping the original puts a copy of the file in your project's files. Back returns to step 1.`,
        },
        {
          screen: `sp-import-preview`,
          do: `Click Import.`,
          explain: `Import is the orange button. Cancel closes the dialog without importing anything.`,
        },
        {
          screen: `sp-import-complete`,
          do: `Read the Import complete summary, then click Open Screenplay.`,
          explain: `It says, for example: Imported as a new screenplay, “THE TOWER AT PEDDA REVU” (Draft 1: Example-Feature (imported)). Nothing else in your project was changed. The table lists the Source file, Imported scenes, Detected characters, Pages / length, Warnings needing attention and Original file. Review Warnings lets you go through any warnings. Open Screenplay takes you to the new script.`,
        },
      ],
      result: `Your script is in the project as a new screenplay, and nothing else in the project was touched.`,
      cautions: [
        `Importing never replaces an existing screenplay. It adds a new one or a new draft, so nothing you wrote is lost.`,
        `Read the warnings before you import. OpenFrame adjusts some details to fit and tells you which.`,
        `PDF files are read as well as possible. Check the preview of a PDF closely, because layout from a PDF can be misread.`,
      ],
      related: [`import-sample-script`, `import-pasted-text`],
    },
    {
      id: `import-sample-script`,
      title: `Try the import with a sample script`,
      purpose: `Practise the import with a fictional script that comes with OpenFrame, without needing a file of your own.`,
      level: `Beginner`,
      prerequisites: [`A project is open and the Import Screenplay dialog is open on step 1.`],
      steps: [
        {
          screen: `sp-import-source`,
          do: `Under OR OPEN A SAMPLE SCRIPT, look at the four buttons.`,
          explain: `The line above them says: Fictional scripts written and owned by OpenFrame, for learning the app. The buttons are Example Feature, Example Short, Example Scene and FDX Interchange Test.`,
        },
        {
          screen: `sp-import-sample-chosen`,
          do: `Click a sample, for example Example Feature. It turns orange to show it is chosen.`,
          explain: `Only one sample is chosen at a time. Click another to switch. Preview is the next step.`,
        },
        {
          screen: `sp-import-preview`,
          do: `Click Preview, check the counts and scenes, then click Import.`,
          explain: `The preview works the same as for your own file. For Example Feature it shows 36 scenes and 10 characters.`,
        },
        {
          screen: `sp-import-complete`,
          do: `Click Open Screenplay to read the imported sample.`,
          explain: `The sample arrives as a new screenplay in your project.`,
        },
      ],
      result: `The sample script is in your project as a new screenplay.`,
      cautions: [
        `The sample is a real screenplay in your project once imported. Use a practice project if you do not want it among your work.`,
      ],
      related: [`import-screenplay-file`],
    },
    {
      id: `import-pasted-text`,
      title: `Import a script by pasting its text`,
      purpose: `Paste a screenplay from another program and import it, and know what to do when OpenFrame cannot read it.`,
      level: `Beginner`,
      prerequisites: [`A project is open and the Import Screenplay dialog is open on step 1.`],
      steps: [
        {
          screen: `sp-import-source`,
          do: `Click in the …OR PASTE SCREENPLAY TEXT box on the right of the dialog.`,
          explain: `The box is empty and shows a faint example: a scene heading, an action line, a character name and dialogue. It is a guide for how the text should look.`,
        },
        {
          screen: `sp-import-pasted-text`,
          do: `Paste your script text with Ctrl+V, then click Preview.`,
          explain: `The picture shows plain text that is not a screenplay, a shopping list. Paste text laid out like the faint example in the box.`,
        },
        {
          screen: `sp-import-failed`,
          do: `If OpenFrame cannot read the text as a screenplay, it shows Import failed. Your current project was not changed. Click Paste Text Instead to try again, or Close.`,
          explain: `The panel shows the Source (Pasted text), the Stage (Interpreting the screenplay), the Problem (The pasted text could not be interpreted as a screenplay) and Your project (Unchanged). It adds: You can paste the screenplay text instead, or choose another file.`,
        },
      ],
      result: `A readable script goes on to the preview. A script that cannot be read leaves your project unchanged.`,
      cautions: [
        `If the import fails, nothing has been added or changed. Fix the text, or try Browse… with the original file.`,
      ],
      related: [`import-screenplay-file`],
    },
  ],
  screens: {
    'sp-import-source': `The Import Screenplay dialog on step 1, Select source. It has the Choose a file box with Browse…, the …OR PASTE SCREENPLAY TEXT box, the four sample buttons, a note that importing never replaces your current screenplay, and Cancel and Preview. Used in import-screenplay-file and import-pasted-text.`,
    'sp-import-sample-chosen': `The same dialog with Example Feature chosen from the sample scripts, shown in orange. Used in import-sample-script.`,
    'sp-import-preview': `Step 2, Import Screenplay · Preview. It shows the source, counts of scenes, characters and pages, confidence chips, a warnings notice with Show warnings, the table of detected scene headings, IMPORT AS, the Draft name box, the Keep the original file in Project Files tick box and Back, Cancel and Import. Used in import-screenplay-file and import-sample-script.`,
    'sp-import-complete': `Import complete. A summary of the new screenplay with the source file, imported scenes, detected characters, pages, warnings needing attention and where the original file is kept. Review Warnings and Open Screenplay are at the bottom. Used in import-screenplay-file and import-sample-script.`,
    'sp-import-pasted-text': `The Import Screenplay dialog with plain text pasted into the paste box. Used in import-pasted-text.`,
    'sp-import-failed': `Import failed. Your current project was not changed. It lists the source, the stage, the problem and that the project is unchanged, with Paste Text Instead and Close. Used in import-pasted-text.`,
  },
  shortcuts: [],
  limits: [
    `The dialog accepts PDF, Final Draft (.fdx), Fountain, TXT and DOCX files, and pasted text.`,
    `Older or other formats are not accepted. These include .doc, .rtf, .celtx, .wps, .pages, .odt, .sexp, .fadein and .highland. OpenFrame tells you so instead of guessing. Save the script as Fountain, Final Draft, DOCX or plain text from the program that made it, then import that.`,
    `PDF import is best effort. Layout in a PDF can be misread, so check the preview carefully.`,
    `Import never replaces an existing screenplay.`,
  ],
  troubleshooting: [
    {
      problem: `I see Import failed.`,
      fix: `Your current project was not changed. Check that the text is a screenplay and try again. Click Paste Text Instead, or choose the original file with Browse….`,
    },
    {
      problem: `The preview shows warnings.`,
      fix: `Click Show warnings. They list details OpenFrame adjusted to fit. The import still works, and Review Warnings on the final screen lets you go through them again.`,
    },
    {
      problem: `I want to keep the original file.`,
      fix: `Leave Keep the original file in Project Files ticked in the preview. The file is kept in the project's files.`,
    },
    {
      problem: `The scene count looks wrong.`,
      fix: `Click Back, check how the scene headings look in the source. They should be laid out like the example in the paste box. Then preview again.`,
    },
  ],
};
