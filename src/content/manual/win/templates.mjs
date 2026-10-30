export default {
  area: `Templates`,
  slug: `templates`,
  order: 28,
  part: `Project and data`,
  intro: [
    `A template is a saved starting point for a new document, so you do not begin from a blank page. The Templates page asks: Would a saved starting point help me begin faster? Templates are optional. Every document can also start blank.`,
    `OpenFrame comes with eight built-in templates: Feature Film Screenplay, Short Film Screenplay, Episodic / Series, Simple Beat Board, Indie Call Sheet, Shot List, Moodboard and Lightweight Budget. You can also make your own, and keep them for this project only or for every project.`,
    `The page has three groups. BUILT-IN holds the templates that come with OpenFrame. YOURS holds your own templates, kept on this computer and available in every project. THIS PROJECT holds copies kept inside this project. Changing a template never changes anything already made from it, and a document made from a template is an independent copy.`,
    `Templates is a tab of Project settings. Open it from the project menu (click the project name at the top and choose Templates), or open Project settings and click the Templates tab. The other tabs sit beside it: Project settings, Writing, Keyboard shortcuts, Offline AI, Advanced, Appearance and Storage.`,
  ],
  layout: [
    { name: `Tabs`, text: `Across the top of the page: Project settings, Writing, Keyboard shortcuts, Templates, Offline AI, Advanced, Appearance and Storage. Templates is selected.` },
    { name: `Header`, text: `The title Templates, the line Would a saved starting point help me begin faster? and, at the right, Create Template.` },
    { name: `BUILT-IN`, text: `Eight cards, with the line Starting points that come with OpenFrame. Use one to copy it into this project. Each card shows the name, the kind of template and a short description, for example Screenplay starter · Feature film · three acts.` },
    { name: `YOURS`, text: `Your own templates, with the line Kept on this computer and available in every project. Empty, it says: Save a project template to My Templates, or create one, to reuse it in every project.` },
    { name: `THIS PROJECT`, text: `Templates kept inside this project, with the line Copies kept inside this project. Changing a template never changes anything already made from it. Empty, it says: No templates in this project yet. Use a built-in template or one of yours, or create one.` },
    { name: `Template card`, text: `Click a card to preview it. The ... button at its top right opens the actions menu for that template.` },
  ],
  workflows: [
    {
      id: `templates-open-and-browse`,
      title: `Open Templates and see what is there`,
      purpose: `Find the page and understand the three groups.`,
      level: `Beginner`,
      prerequisites: [`A project is open.`],
      steps: [
        {
          screen: `templates-page`,
          do: `Click the project name at the top of the window and choose Templates.`,
          explain: `The page opens on the Templates tab of Project settings. BUILT-IN lists the eight templates that come with OpenFrame. YOURS and THIS PROJECT are empty and say how to fill them. At the bottom: Templates are optional. Every document can also start blank.`,
        },
        {
          screen: `templates-page`,
          do: `Read the kind and description under each name.`,
          explain: `Feature Film Screenplay: Screenplay starter, three acts. Short Film Screenplay: one location. Episodic / Series: cold open and four acts. Simple Beat Board: three acts with key beats. Indie Call Sheet: six default sections plus weather, crew and walkie channels. Shot List: standard coverage for one scene. Moodboard: look and feel board. Lightweight Budget: nine categories with Planned and Actual.`,
        },
      ],
      result: `You know the templates you can start from.`,
      cautions: [
        `You do not need a template. You can always start a screenplay, call sheet or budget blank.`,
      ],
      related: [`templates-preview`, `templates-create`],
    },
    {
      id: `templates-preview`,
      title: `Preview a template before you use it`,
      purpose: `See what a template contains.`,
      level: `Beginner`,
      prerequisites: [`The Templates page is open.`],
      steps: [
        {
          screen: `templates-actions-menu-builtin`,
          do: `Click the ... button on a template card.`,
          explain: `For a built-in template the menu has Preview, then an action that depends on the kind (for a screenplay, Start the screenplay), Use in This Project and Copy to My Templates.`,
        },
        {
          screen: `templates-preview-dialog`,
          do: `Choose Preview, or click the card itself.`,
          explain: `The dialog is headed with the template name and its kind, here Feature Film Screenplay and Screenplay starter · Built-in. It lists what the template contains. Here, SCENES: EXT. OPENING LOCATION - DAY, INT. MIDPOINT LOCATION - NIGHT and EXT. FINAL LOCATION - DAY. The buttons at the bottom are Start the screenplay and Use in This Project. Close with the x.`,
        },
      ],
      result: `You have seen what the template will give you.`,
      cautions: [
        `Previewing changes nothing. Close the dialog to go back.`,
      ],
      related: [`templates-use-in-project`, `templates-start-document`],
    },
    {
      id: `templates-use-in-project`,
      title: `Copy a template into this project`,
      purpose: `Keep a copy of a built-in or personal template inside this project.`,
      level: `Beginner`,
      prerequisites: [`A built-in template or one of yours.`],
      steps: [
        {
          screen: `templates-actions-menu-builtin`,
          do: `Click the ... button on the template and choose Use in This Project.`,
          explain: `You can also click Use in This Project in the preview dialog.`,
        },
        {
          screen: `templates-page`,
          do: `Look under THIS PROJECT.`,
          explain: `A copy of the template is listed there. A message says Added template followed by its name to this project, with Undo. The copy is yours to edit without touching the original.`,
        },
      ],
      result: `A copy of the template is kept inside this project.`,
      cautions: [
        `Click Undo in the message if you copied the wrong template.`,
      ],
      related: [`templates-edit`, `templates-keep-for-every-project`],
    },
    {
      id: `templates-start-document`,
      title: `Start a document from a template`,
      purpose: `Make a screenplay, call sheet or other document that begins with the template's content.`,
      level: `Intermediate`,
      prerequisites: [`The kind of document you want can be created in this project. A call sheet needs a shooting day with no call sheet yet.`],
      steps: [
        {
          screen: `templates-actions-menu-own`,
          do: `Click the ... button on the template and choose the create action at the top of the menu.`,
          explain: `The action is named after the template. For a call sheet template it is Create a call sheet. Other kinds say Start the screenplay, Add shots to a scene, Create a moodboard, Add to the Beat Board or Add budget lines. Templates for reports, Breakdown documents and project starters have no create action. The same action is a button in the preview dialog.`,
        },
        {
          screen: `templates-apply-dialog`,
          do: `In the dialog, choose what the document is for and click Create.`,
          explain: `For a call sheet the dialog is headed Create a call sheet and says From the template name. The new document is an independent copy. Shooting day * lists the days that have no call sheet yet, for example Shoot Day 3 · Wed 11 Nov 2026. A shot list template asks for a Scene instead. The dialog adds: Undo removes everything this creates in one step. If any part can't be made, nothing is created. Cancel closes it.`,
        },
      ],
      result: `The new document opens, made from the template. A message with Undo says what was created.`,
      cautions: [
        `If every shooting day already has a call sheet, or the schedule has no shooting day, the dialog says so and Create stays off. Add a shooting day first.`,
        `A shot list template needs a scene. If there is no screenplay yet, the dialog says to write or import one first.`,
        `Click Undo in the message to remove everything the template created in one step.`,
      ],
      related: [`templates-preview`, `templates-create`],
    },
    {
      id: `templates-create`,
      title: `Create your own template`,
      purpose: `Save a starting point of your own for a kind of document.`,
      level: `Intermediate`,
      prerequisites: [`The Templates page is open.`],
      steps: [
        {
          screen: `templates-page`,
          do: `Click Create Template at the top right.`,
          explain: `The page lists the built-in templates and whatever you have made already.`,
        },
        {
          screen: `templates-create-dialog`,
          do: `Read the Create Template dialog.`,
          explain: `It says: A saved starting point for new documents. Name * is required. Type * is a list: Screenplay starter, Call sheet, Shot list, Moodboard, Report, Beat Board starter, Breakdown document, Simple budget and Project starter. Keep in has This project and Every project. Description is optional. The last box lists what the template adds. Its label changes with the type, and for a call sheet it reads Added sections (Weather, Crew, Extras…).`,
        },
        {
          screen: `templates-create-name-error`,
          do: `If you click Create with the name empty, the message Give the template a name. shows under Name *.`,
          explain: `Type a name to continue. Cancel closes the dialog and creates nothing.`,
        },
        {
          screen: `templates-create-filled`,
          do: `Fill in Name *, pick a Type *, choose Every project or This project under Keep in, add a Description and list the added sections, one per line.`,
          explain: `The picture shows Tour global call sheet, Call sheet, Every project, the description One page for night shoots and the sections Weather, Crew and Extras. The text under the box says: One per line. These become the added sections (weather, crew, extras…) of new documents made from this template.`,
        },
        {
          screen: `templates-created-toast`,
          do: `Click Create.`,
          explain: `A message says Created “Tour global call sheet” in your templates. The template joins YOURS if you chose Every project, or THIS PROJECT if you chose This project.`,
        },
      ],
      result: `Your template is listed and ready to use.`,
      cautions: [
        `The Type cannot be changed after the template is made. Make a new template if you chose the wrong type.`,
        `The name is limited to 120 characters and the description to 200.`,
      ],
      related: [`templates-edit`, `templates-start-document`],
    },
    {
      id: `templates-edit`,
      title: `Edit a template of yours`,
      purpose: `Change the name, description or contents of a template you made or copied.`,
      level: `Intermediate`,
      prerequisites: [`A template exists under YOURS or THIS PROJECT. Built-in templates cannot be edited.`],
      steps: [
        {
          screen: `templates-actions-menu-own`,
          do: `Click the ... button on the template and choose Edit.`,
          explain: `The menu of your own template has Preview, a create action (here Create a call sheet), Save to My Templates, Edit and Delete…. Delete… is red below a divider.`,
        },
        {
          screen: `templates-edit-dialog`,
          do: `Change what you need and click Save.`,
          explain: `The dialog is headed Edit Template and says: Changes apply to future use only. Name *, Description and the list of added sections can be changed. Type * and Keep in are shown but cannot be changed. Cancel closes the dialog without a change. A message says Saved template followed by its name.`,
        },
      ],
      result: `The template shows your changes. Documents made from it earlier are not changed.`,
      cautions: [
        `Edits apply to documents you make from now on. Earlier ones stay as they were.`,
      ],
      related: [`templates-create`],
    },
    {
      id: `templates-keep-for-every-project`,
      title: `Keep a template for every project`,
      purpose: `Turn a built-in template or a project template into one of yours, available in all projects.`,
      level: `Beginner`,
      prerequisites: [`A template card is on the page.`],
      steps: [
        {
          screen: `templates-actions-menu-builtin`,
          do: `On a built-in template, choose Copy to My Templates.`,
          explain: `This saves your own copy to YOURS, kept on this computer.`,
        },
        {
          screen: `templates-actions-menu-own`,
          do: `On a template in THIS PROJECT, choose Save to My Templates.`,
          explain: `The template is copied to YOURS. A message says the template is now in your templates for every project. The copy in this project stays where it is.`,
        },
      ],
      result: `The template is in YOURS and you can use it in any project.`,
      cautions: [
        `A template in YOURS is kept on this computer, not inside the project. A copy under THIS PROJECT is kept inside it.`,
      ],
      related: [`templates-use-in-project`],
    },
    {
      id: `templates-delete`,
      title: `Delete a template`,
      purpose: `Remove a template of yours without affecting documents already made from it.`,
      level: `Beginner`,
      prerequisites: [`A template exists under YOURS or THIS PROJECT.`],
      steps: [
        {
          screen: `templates-actions-menu-own`,
          do: `Click the ... button on the template and choose Delete….`,
          explain: `Built-in templates cannot be deleted.`,
        },
        {
          screen: `templates-delete-confirm`,
          do: `For a template in YOURS, read the dialog Delete template “Tour global call sheet”?`,
          explain: `It says: This template is removed from this computer. Projects and documents that already used it are not affected. Cancel keeps the template. Delete Template removes it.`,
        },
        {
          screen: `templates-global-deleted-toast`,
          do: `Click Delete Template.`,
          explain: `A message says Deleted template “Tour global call sheet”. Projects that already used it are not affected. A template in YOURS is removed for good.`,
        },
        {
          screen: `templates-project-deleted-toast`,
          do: `A template in THIS PROJECT is deleted at once, with no dialog.`,
          explain: `A message says Moved template “Tour project call sheet” to Recently Deleted, with Undo. It stays there for 30 days and can be restored.`,
        },
      ],
      result: `The template is removed. Documents already made from it are unchanged.`,
      cautions: [
        `Deleting a template from YOURS cannot be undone. Deleting one from THIS PROJECT can: use Undo or Restore in Recently Deleted.`,
      ],
      related: [`templates-edit`],
    },
  ],
  screens: {
    'templates-page': `The Templates page, a tab of Project settings. BUILT-IN lists eight templates, and YOURS and THIS PROJECT show how to fill them. Create Template is at the top right. Used in templates-open-and-browse, templates-use-in-project and templates-create.`,
    'templates-actions-menu-builtin': `The ... menu of a built-in template: Preview, Start the screenplay, Use in This Project and Copy to My Templates. Used in templates-preview, templates-use-in-project and templates-keep-for-every-project.`,
    'templates-preview-dialog': `The preview of Feature Film Screenplay: Screenplay starter, Built-in, with its three scene headings and the buttons Start the screenplay and Use in This Project. Used in templates-preview.`,
    'templates-create-dialog': `The Create Template dialog: Name *, Type *, Keep in (This project, Every project), Description and the list of added sections, with Cancel and Create. Used in templates-create.`,
    'templates-create-name-error': `The Create Template dialog with the name empty. Give the template a name. shows under the box. Variant, part of templates-create.`,
    'templates-create-filled': `The Create Template dialog filled in: Tour global call sheet, Call sheet, Every project, a description and the sections Weather, Crew and Extras. Used in templates-create.`,
    'templates-created-toast': `The Templates page after a template is created. A message says Created “Tour global call sheet” in your templates. Used in templates-create.`,
    'templates-actions-menu-own': `The ... menu of a template of your own: Preview, Create a call sheet, Save to My Templates, Edit and Delete…. Used in templates-start-document, templates-edit, templates-keep-for-every-project and templates-delete.`,
    'templates-edit-dialog': `The Edit Template dialog. It says Changes apply to future use only. Name, Description and the sections can be changed, with Cancel and Save. Used in templates-edit.`,
    'templates-apply-dialog': `The Create a call sheet dialog, made from a template. It asks for the Shooting day * and says Undo removes everything this creates in one step. Used in templates-start-document.`,
    'templates-delete-confirm': `The dialog Delete template “Tour global call sheet”? It says projects and documents that already used it are not affected, with Cancel and Delete Template. Used in templates-delete.`,
    'templates-global-deleted-toast': `The Templates page after a template of yours is deleted. A message says it was deleted and projects that used it are not affected. Used in templates-delete.`,
    'templates-project-deleted-toast': `The Templates page after a template in this project is deleted. A message says it was moved to Recently Deleted, with Undo. Used in templates-delete.`,
  },
  shortcuts: [
    [`Ctrl+Z`, `Undo the last change, also offered as Undo in the message`],
  ],
  limits: [
    `Built-in templates cannot be edited or deleted. Copy one first.`,
    `The Type of a template cannot be changed after it is created.`,
    `Templates for reports, Breakdown documents and project starters can be saved and copied but have no create action on this page.`,
    `A document made from a template is an independent copy. Later edits to the template do not change it.`,
    `Templates in YOURS are kept on this computer and are available in every project.`,
  ],
  troubleshooting: [
    {
      problem: `I cannot find Templates.`,
      fix: `It is a tab of Project settings, and it is also in the project menu. Click the project name at the top and choose Templates.`,
    },
    {
      problem: `Create stays off in the Create a call sheet dialog.`,
      fix: `Every shooting day already has a call sheet, or the schedule has no shooting day yet. Add a shooting day in the schedule first.`,
    },
    {
      problem: `The template dialog says Give the template a name.`,
      fix: `Name * is required. Type a name and click Create again.`,
    },
    {
      problem: `I cannot edit or delete a built-in template.`,
      fix: `Built-in templates are fixed. Choose Copy to My Templates or Use in This Project, then edit the copy.`,
    },
    {
      problem: `I deleted a project template by mistake.`,
      fix: `Click Undo in the message, or open Recently Deleted and click Restore. A template deleted from YOURS cannot be restored.`,
    },
    {
      problem: `I edited a template but my old call sheet did not change.`,
      fix: `That is expected. Changes apply to future use only.`,
    },
  ],
};
