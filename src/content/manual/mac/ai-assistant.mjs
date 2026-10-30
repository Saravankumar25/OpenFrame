export default {
  area: `AI Assistant`,
  slug: `ai-assistant`,
  order: 18,
  part: `Assistant`,
  intro: [
    `The AI Assistant is an optional panel that opens on the right side of the project window. It runs on your Mac, with an offline AI that is downloaded once. Your project data is not sent to OpenFrame or to an AI cloud service, and OpenFrame works normally without it.`,
    `When Offline AI is installed, you ask the assistant about your project in plain words. It can also propose changes to the project. You read a proposed change and apply it or reject it, and nothing in your project changes until you decide.`,
    `The screens in this chapter show the panel in the state you see when Offline AI is missing from the installation: the panel opens, says AI is not available right now, and points you to Offline AI settings. This is also the quickest way to learn where everything lives: the toolbar button, the panel header, the settings menu, and the Offline AI page in Settings with its AI health and Assistant memory sections.`,
    `The panel is the same in every workspace. It stays open at the right while you move between Home, Idea Vault, Story, Screenplay, Breakdown, Production, Schedule, Call Sheets and Files.`,
  ],
  layout: [
    { name: `Assistant button`, text: `In the toolbar at the top of the window, between New and Settings. It opens the AI Assistant panel.` },
    { name: `Panel header`, text: `The sparkle icon and the name AI Assistant, a small status label (Off when the assistant is not running), a Settings menu and a Close button.` },
    { name: `Settings menu`, text: `The Settings button in the panel header opens a small menu headed Offline AI with one item, Offline AI settings….` },
    { name: `Panel body`, text: `The conversation area. When Offline AI is missing it shows the heading AI is not available right now, an explanation, and the line Everything else in OpenFrame works normally.` },
    { name: `Resize handle`, text: `The edge between your workspace and the panel. Drag it to make the panel wider or narrower.` },
    { name: `Offline AI page in Settings`, text: `Reached from Offline AI settings…. It has the Offline AI status and message, AI health with Export AI report…, and Assistant memory.` },
  ],
  workflows: [
    {
      id: `ai-assistant-open-close`,
      title: `Open and close the AI Assistant`,
      purpose: `Show the assistant panel beside your work and hide it again.`,
      level: `Beginner`,
      prerequisites: [`A project is open.`],
      steps: [
        {
          screen: `ai-closed-home`,
          do: `Look at the toolbar. Find the Assistant button between New and Settings.`,
          explain: `The picture shows Project Home with the assistant closed. Your workspace uses the whole width of the window.`,
        },
        {
          screen: `project-home-ai-panel`,
          do: `Click Assistant. You can also press Command+J.`,
          explain: `The AI Assistant panel opens at the right and the workspace narrows to make room. Project Home is shown here, but the panel opens beside any workspace.`,
        },
        {
          screen: `ai-panel`,
          do: `Look at the panel header. It has the name AI Assistant, the status Off, a Settings menu and Close.`,
          explain: `Off means the assistant is not running. The body of the panel tells you why, see Read the message when AI is not available.`,
        },
        {
          screen: `ai-closed-home`,
          do: `Click Close at the top right of the panel.`,
          explain: `The panel closes and the workspace takes the full width again. Closing the panel never changes your project.`,
        },
      ],
      result: `The panel opens and closes on demand, and your workspace is unchanged either way.`,
      cautions: [
        `The panel is a side panel, not a separate window. Close it when you want the full width of the workspace back.`,
      ],
      related: [`ai-assistant-read-unavailable`, `ai-assistant-beside-workspaces`],
    },
    {
      id: `ai-assistant-read-unavailable`,
      title: `Read the message when AI is not available`,
      purpose: `Understand what the panel says when Offline AI is missing, and what to do about it.`,
      level: `Beginner`,
      prerequisites: [`The AI Assistant panel is open.`],
      steps: [
        {
          screen: `ai-panel`,
          do: `Read the heading AI is not available right now in the middle of the panel.`,
          explain: `The text under it says: Offline AI is missing from this installation, so it can't start. Reinstall OpenFrame Studio to repair it. Your projects were not changed.`,
        },
        {
          screen: `ai-panel`,
          do: `Read the smaller line at the bottom: Everything else in OpenFrame works normally.`,
          explain: `Only the assistant is affected. Writing, planning, exporting and every other workspace keep working. Reinstalling OpenFrame Studio repairs the missing piece.`,
        },
        {
          screen: `ai-panel`,
          do: `To see more detail, click Settings in the panel header and choose Offline AI settings….`,
          explain: `This opens the Offline AI page. See Open Offline AI settings from the assistant.`,
        },
      ],
      result: `You know the assistant is off because Offline AI is missing, that your project is safe, and that a reinstall repairs it.`,
      cautions: [
        `The message says your projects were not changed. The assistant failing to start never touches your work.`,
      ],
      related: [`ai-assistant-offline-settings`],
    },
    {
      id: `ai-assistant-offline-settings`,
      title: `Open Offline AI settings from the assistant`,
      purpose: `Jump from the panel to the Offline AI page of Settings.`,
      level: `Beginner`,
      prerequisites: [`The AI Assistant panel is open.`],
      steps: [
        {
          screen: `ai-panel`,
          do: `In the panel header, click Settings.`,
          explain: `Settings is the button with the small chevron, to the left of Close.`,
        },
        {
          screen: `ai-settings-menu`,
          do: `Read the menu. Its heading is Offline AI. Click Offline AI settings….`,
          explain: `The heading is a label, not a choice. Offline AI settings… is the one item you can click.`,
        },
        {
          screen: `ai-offline-ai-settings`,
          do: `The Settings window opens on the Offline AI page. Read the Offline AI box at the top.`,
          explain: `The window has six tabs: General, Appearance, Keyboard Shortcuts, Storage, Offline AI and Diagnostics. The Offline AI box shows the status (here, Needs a reinstall) and says OpenFrame AI runs on this computer after a one-time download and that your project data is not sent to OpenFrame or to an AI cloud service. A red message repeats that Offline AI is missing and tells you to reinstall. Below it: OpenFrame works normally without OpenFrame AI.`,
        },
      ],
      result: `The Settings window is open on the Offline AI page.`,
      cautions: [
        `You can open the same page yourself with Command+, to open Settings and then the Offline AI tab.`,
      ],
      related: [`ai-assistant-ai-health`, `ai-assistant-memory`],
    },
    {
      id: `ai-assistant-ai-health`,
      title: `Read AI health and export an AI report`,
      purpose: `See what the assistant keeps about its own performance and save it to a file you can share.`,
      level: `Intermediate`,
      prerequisites: [`The Settings window is open on the Offline AI page.`],
      steps: [
        {
          screen: `ai-offline-ai-settings`,
          do: `Scroll to the AI health section.`,
          explain: `Before you have used the assistant it says: No requests yet. The figures appear after you use the AI Assistant.`,
        },
        {
          screen: `ai-offline-ai-settings`,
          do: `Read the note under it.`,
          explain: `Only numbers are kept on this computer. No questions, answers or project text are kept, and nothing is sent anywhere.`,
        },
        {
          screen: `ai-offline-ai-settings`,
          do: `Click Export AI report… and choose where to save the file.`,
          explain: `Export AI report… saves the numbers to a file you choose. You decide whether to share it. OpenFrame never sends it for you.`,
        },
      ],
      result: `You have read what AI health records and know how to save it to a file.`,
      cautions: [
        `The report holds numbers only. It does not contain your questions, the answers or any text from your project.`,
      ],
      related: [`ai-assistant-memory`],
    },
    {
      id: `ai-assistant-memory`,
      title: `Check what the assistant remembers`,
      purpose: `Read the Assistant memory section and know how to make the assistant forget something.`,
      level: `Intermediate`,
      prerequisites: [`The Settings window is open on the Offline AI page.`],
      steps: [
        {
          screen: `ai-offline-ai-settings`,
          do: `Scroll down to Assistant memory.`,
          explain: `With nothing remembered it says: Nothing is remembered yet. The number 0 shows how many entries there are.`,
        },
        {
          screen: `ai-offline-ai-settings`,
          do: `Read what the assistant keeps.`,
          explain: `The assistant keeps a fact or preference only when you state it outright, or confirm one of its answers. Entries are saved on this Mac, in this project, for you only.`,
        },
        {
          screen: `ai-offline-ai-settings`,
          do: `To remove something, delete its entry in this list.`,
          explain: `Delete an entry and the assistant forgets it. The list is empty in the picture, so there is nothing to delete yet.`,
        },
      ],
      result: `You know what the assistant can remember, where it is stored, and how to clear an entry.`,
      cautions: [
        `Memory is per project and per person. It is not shared with other projects or other people.`,
      ],
      related: [`ai-assistant-ai-health`],
    },
    {
      id: `ai-assistant-beside-workspaces`,
      title: `Keep the assistant open beside each workspace`,
      purpose: `See that the panel works the same way next to every workspace, and close it when you want more room.`,
      level: `Beginner`,
      prerequisites: [`A project is open and the AI Assistant panel is open.`],
      steps: [
        {
          screen: `ai-panel-idea-vault`,
          do: `Click Idea Vault in the sidebar with the panel open.`,
          explain: `The Idea Vault keeps its own list on the left and its content in the middle. The panel stays at the right with the same header and message.`,
        },
        {
          screen: `ai-panel-story`,
          do: `Click Story.`,
          explain: `The Story page with its views (Beat Board, Outline, Characters, Timeline) sits beside the panel. The text area shrinks to fit.`,
        },
        {
          screen: `ai-panel-screenplay`,
          do: `Click Screenplay.`,
          explain: `The screenplay page, scene list and page bar stay usable beside the panel.`,
        },
        {
          screen: `ai-closed-screenplay`,
          do: `To get the full width back, click Close in the panel header.`,
          explain: `The screenplay uses the whole window again.`,
        },
        {
          screen: `ai-panel-breakdown`,
          do: `Click Breakdown.`,
          explain: `Breakdown shows Choose a screenplay source to begin the breakdown. with Select Source and Import Screenplay, and the panel stays at the right.`,
        },
        {
          screen: `ai-closed-breakdown`,
          do: `Close the panel to see Breakdown at full width.`,
          explain: `The same Breakdown page, now without the panel.`,
        },
        {
          screen: `ai-panel-production`,
          do: `Click Production.`,
          explain: `The Production Overview with its progress tiles sits beside the panel.`,
        },
        {
          screen: `ai-panel-schedule`,
          do: `Open Schedule from Production.`,
          explain: `The Schedule page, with Open Breakdown, sits beside the panel.`,
        },
        {
          screen: `ai-panel-call-sheets`,
          do: `Click Call Sheets.`,
          explain: `The Create Call Sheet page sits beside the panel.`,
        },
        {
          screen: `ai-panel-files`,
          do: `Click Files.`,
          explain: `Project Files with its folder list and file rows sits beside the panel.`,
        },
      ],
      result: `You can keep the assistant open while you move through the project, or close it for more room.`,
      cautions: [
        `The panel remembers nothing about which workspace you were in. It shows the same message in every workspace while Offline AI is missing.`,
      ],
      related: [`ai-assistant-open-close`, `ai-assistant-small-window`],
    },
    {
      id: `ai-assistant-small-window`,
      title: `Use the assistant in a small window`,
      purpose: `Make room for the panel by hiding the sidebar, and know what happens in a narrow window.`,
      level: `Intermediate`,
      prerequisites: [`The AI Assistant panel is open.`],
      steps: [
        {
          screen: `ai-panel-sidebar-hidden`,
          do: `Click Hide Sidebar at the left of the toolbar, or press Control+Command+S.`,
          explain: `The sidebar disappears and the toolbar button changes to Show Sidebar. The workspace and the panel share the whole window.`,
        },
        {
          screen: `ai-panel-narrow-window`,
          do: `If your window is narrow, the sidebar folds away by itself.`,
          explain: `The toolbar then shows Show Sidebar. Toolbar items that no longer fit move into a small double arrow menu at the right edge, labelled more toolbar items. The panel header keeps Settings and Close.`,
        },
        {
          screen: `ai-panel-sidebar-hidden`,
          do: `Click Show Sidebar to bring the sidebar back.`,
          explain: `Make the window wider if you want the sidebar and the panel open together.`,
        },
      ],
      result: `The workspace, and the assistant panel, fit your window size.`,
      cautions: [
        `If a toolbar button seems to be missing in a narrow window, look under the double arrow at the right edge.`,
      ],
      related: [`ai-assistant-beside-workspaces`],
    },
    {
      id: `ai-assistant-find-via-search`,
      title: `Why Search does not find the assistant`,
      purpose: `Know that Search looks inside your project, and use the toolbar button to reach the assistant.`,
      level: `Beginner`,
      prerequisites: [`A project is open.`],
      steps: [
        {
          screen: `ai-search-palette`,
          do: `Click Search in the toolbar, or press Command+K, and type assistant.`,
          explain: `The Search and Commands window shows No matches. and explains that Search looks at titles, text, notes, file names and tags. The assistant is not one of those, so it never appears as a result.`,
        },
        {
          screen: `ai-search-palette`,
          do: `Press Esc to close Search, then click Assistant in the toolbar.`,
          explain: `The toolbar button is the way to open the panel.`,
        },
      ],
      result: `You open the assistant from the toolbar, not from Search.`,
      cautions: [],
      related: [`ai-assistant-open-close`],
    },
  ],
  screens: {
    'ai-closed-home': `Project Home with the AI Assistant closed. The toolbar has the Assistant button and Home uses the whole window. Used in ai-assistant-open-close.`,
    'project-home-ai-panel': `Project Home with the AI Assistant panel open at the right. The panel header shows AI Assistant, the status Off, Settings and Close. Used in ai-assistant-open-close.`,
    'ai-panel': `The AI Assistant panel on Project Home with its message: AI is not available right now. Offline AI is missing from this installation, so it can't start. Reinstall OpenFrame Studio to repair it. Your projects were not changed. Used in ai-assistant-open-close, ai-assistant-read-unavailable and ai-assistant-offline-settings.`,
    'ai-settings-menu': `The Settings menu of the panel, headed Offline AI, with the single item Offline AI settings…. Used in ai-assistant-offline-settings.`,
    'ai-offline-ai-settings': `The Settings window on the Offline AI tab. It shows the status Needs a reinstall, the explanation, the AI health section with Export AI report… and the Assistant memory section with its count. The tabs are General, Appearance, Keyboard Shortcuts, Storage, Offline AI and Diagnostics. Used in ai-assistant-offline-settings, ai-assistant-ai-health and ai-assistant-memory.`,
    'ai-panel-idea-vault': `The Idea Vault with the AI Assistant panel open at the right. The vault shows Project Idea Vault, the Which Idea Vault switch and an empty list. Used in ai-assistant-beside-workspaces.`,
    'ai-panel-story': `The Story page with the panel open at the right. The Story views are Beat Board, Outline, Characters and Timeline, with the page and its formatting bar. Used in ai-assistant-beside-workspaces.`,
    'ai-panel-screenplay': `The Screenplay with the panel open at the right. The screenplay page bar, the scene list and the page are narrower to make room. Used in ai-assistant-beside-workspaces.`,
    'ai-panel-breakdown': `Breakdown with the panel open at the right. The page asks you to choose a screenplay source and offers Select Source and Import Screenplay. Used in ai-assistant-beside-workspaces.`,
    'ai-panel-production': `The Production Overview with the panel open at the right, showing Start Production Setup and the progress tiles. Used in ai-assistant-beside-workspaces.`,
    'ai-panel-schedule': `The Schedule page with the panel open at the right. The page asks you to choose a Production Source in Breakdown first and offers Open Breakdown. Used in ai-assistant-beside-workspaces.`,
    'ai-panel-call-sheets': `The Call Sheets page with the panel open at the right, with Create Call Sheet. Used in ai-assistant-beside-workspaces.`,
    'ai-panel-files': `Project Files with the panel open at the right. The file list shows Example-Short.fountain stored in the project. Used in ai-assistant-beside-workspaces.`,
    'ai-closed-screenplay': `The Screenplay with the panel closed after clicking Close. The page and scene list use the full width. Used in ai-assistant-beside-workspaces.`,
    'ai-closed-breakdown': `Breakdown with the panel closed. The same page as the one beside the panel, now at full width. Used in ai-assistant-beside-workspaces.`,
    'ai-panel-sidebar-hidden': `Breakdown with the sidebar hidden and the panel open. The toolbar button now reads Show Sidebar. Used in ai-assistant-small-window.`,
    'ai-panel-narrow-window': `A narrow window with the sidebar folded away and the panel open. Extra toolbar items are under a double arrow at the right edge. Used in ai-assistant-small-window.`,
    'ai-search-palette': `The Search and Commands window after typing assistant. It says No matches. Search looks at titles, text, notes, file names and tags. Used in ai-assistant-find-via-search.`,
  },
  shortcuts: [
    [`Command+J`, `Open or close the AI Assistant`],
    [`Command+K`, `Search and Commands`],
    [`Command+,`, `Open Settings`],
    [`Control+Command+S`, `Show or hide the sidebar`],
    [`Esc`, `Close Search and Commands`],
  ],
  limits: [
    `The assistant is optional. It needs Offline AI to be installed on this Mac, and OpenFrame works fully without it.`,
    `Your project data stays on your Mac. It is not sent to OpenFrame or to an AI cloud service.`,
    `A change the assistant proposes is never applied on its own. You read it and apply or reject it.`,
    `Assistant memory is kept per project and per person, and only when you state a fact or confirm an answer.`,
    `AI health keeps numbers only. It never keeps questions, answers or project text.`,
  ],
  troubleshooting: [
    {
      problem: `The panel says AI is not available right now.`,
      fix: `Offline AI is missing from the installation. Reinstall OpenFrame Studio to repair it. Your projects are not changed, and everything else works normally.`,
    },
    {
      problem: `The status in the panel header says Off.`,
      fix: `The assistant is not running. Click Settings, then Offline AI settings…, to read the reason on the Offline AI page.`,
    },
    {
      problem: `I cannot find the Assistant button.`,
      fix: `It is in the toolbar between New and Settings. In a narrow window, look under the double arrow at the right edge of the toolbar.`,
    },
    {
      problem: `Search does not find the assistant.`,
      fix: `Search looks at titles, text, notes, file names and tags in your project. Open the assistant with the Assistant button in the toolbar.`,
    },
    {
      problem: `The workspace feels cramped with the panel open.`,
      fix: `Drag the edge of the panel to make it narrower, hide the sidebar, or click Close.`,
    },
  ],
};
