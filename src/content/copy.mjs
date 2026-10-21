// Shared plain-English copy for the information pages. Source of truth: research/facts.json and
// research/capability-matrix.md (audited 2026-10-21). Nothing here may claim more than those files show.

export const PRICES = [
  ['India', '3,000 rupees'],
  ['United States and the rest of the world', '89 US dollars'],
  ['Europe', '89 euros'],
];

// [action, Windows, Mac]. null means the platform has no shortcut for it.
export const SHORTCUTS = [
  ['Search the project', 'Ctrl+K', 'Cmd+K'],
  ['Open or close the assistant', 'Ctrl+J', 'Cmd+J'],
  ['Save a checkpoint now', 'Ctrl+S', null],
  ['Undo a project change', 'Ctrl+Z', 'Cmd+Opt+Z (Cmd+Z undoes text)'],
  ['Redo a project change', 'Ctrl+Y or Ctrl+Shift+Z', 'Cmd+Opt+Shift+Z'],
  ['New project (on Home)', 'Ctrl+N', 'Cmd+N'],
  ['Open a project (on Home)', 'Ctrl+O', 'Cmd+O'],
  ['Search projects (on Home)', 'Ctrl+F or Ctrl+K', 'Cmd+F or Cmd+K'],
  ['New scene card (in a project)', 'Ctrl+N', 'Cmd+N'],
  ['Quick Capture, a new idea', 'Ctrl+Shift+N', 'Cmd+Shift+N'],
  ['Find in the script or in ideas', 'Ctrl+F', 'Cmd+F'],
  ['Next or previous scene', 'Alt+Down or Alt+Up', 'Opt+Down or Opt+Up'],
  ['Next spelling mistake', 'F7', 'F7'],
  ['Insert a page break', 'Ctrl+Enter', 'Cmd+Return'],
  ['Scene Heading, Action, Character, Parenthetical, Dialogue, Transition, Shot', 'Ctrl+1 to Ctrl+7', 'Cmd+1 to Cmd+7'],
  ['Custom elements', 'Ctrl+Alt+1 to 9', 'Cmd+Opt+1 to 9'],
  ['Back or forward between pages', 'Alt+Left or Alt+Right', 'Cmd+[ or Cmd+]'],
  ['Settings', 'The gear on Home', 'Cmd+,'],
  ['Project settings', 'The project menu', 'Cmd+Shift+,'],
  ['Close the project', 'The project menu', 'Cmd+Shift+W'],
  ['Import a screenplay', 'The import dialog', 'Cmd+Shift+I'],
  ['Export a screenplay', 'The export dialog', 'Cmd+Shift+E'],
  ['Export a PDF directly', null, 'Cmd+Opt+Shift+E'],
  ['Print', 'Print Preview in Export', 'Cmd+P'],
  ['Show or hide the sidebar', 'The collapse button', 'Ctrl+Cmd+S'],
  ['Go to workspace 1 to 8', null, 'Ctrl+Cmd+1 to 8'],
];

export const SHORTCUT_NOTE =
  'Every shortcut can be changed in Settings, Keyboard Shortcuts, and the app warns you about conflicts. Copy, cut, paste and select all (with Ctrl or Cmd plus A, C, V, X) are reserved and cannot be changed.';

export const IMPORT_OK = [
  ['Final Draft', '.fdx'],
  ['Fountain', '.fountain, .spmd'],
  ['Word', '.docx'],
  ['PDF', 'best effort; check the preview'],
  ['Plain text and Markdown', '.txt, .md'],
  ['Pasted text', 'paste straight into the import dialog'],
];

export const IMPORT_REFUSED = ['.doc', '.rtf', '.celtx', '.wps', '.pages', '.odt', '.sexp (Movie Magic)', '.fadein', '.highland'];

export const EXPORTS = [
  ['Screenplay', 'PDF, Final Draft (.fdx), Fountain, Word (.docx), plain text (.txt), print'],
  ['Story page', 'TXT, PDF, DOCX'],
  ['Beat Board', 'PDF, TXT, CSV'],
  ['Breakdown, shot list, schedule, Day Out of Days, reports, budget', 'PDF, CSV, XLSX'],
  ['Moodboard, storyboard, call sheet, sides', 'PDF'],
  ['Daily View', 'PDF (Windows also writes TXT)'],
  ['A whole project or a backup', '.ofproject and .ofbackup packages'],
];

export const THEMES = [
  'OpenFrame Default', 'Black and White', 'Studio Amber', 'Slate Noir', 'Midnight Reel', 'Teal & Orange',
  'Courier & Cream', 'Cutting Room', 'Velvet Curtain', 'Sunset Boulevard', 'Arctic Frame', 'Neon Reel',
  'Location Scout', 'Desert Dune', 'Colorist Neutral', 'Silver Screen', 'High Contrast',
];

// FAQ: [group, [[question, answer in md], ...]]
export const FAQ = [
  ['Product', [
    ['What is OpenFrame Studio?',
      'A desktop app for writers, screenwriters and production teams. One project holds your ideas, story, screenplay and production plan, from breakdown and schedule to call sheets. Everything is stored on your own computer.'],
    ['Who is it for?',
      'Writers, screenwriters and production teams. There is one app for everyone and no separate editions. You can use only the parts you need, for example the Idea Vault and the screenplay editor.'],
    ['Can I try it today?',
      'Not publicly. Today the app exists as internal test builds for the team. See [Download](/download/) for the current state.'],
    ['Can several people edit one project at the same time?',
      'No. There is no live collaboration and no cloud sharing. People take turns by sending files: review packages, Beat Board packages and full project packages. The sender sees the replies in the Review Queue and previews every change before applying it.'],
  ]],
  ['Platforms', [
    ['Which systems does it run on?',
      'Windows 11 on 64-bit (x64) computers, and macOS 14 or later on Apple silicon Macs. Windows 10 and Intel Macs are not supported.'],
    ['Is there a Mac version?',
      'It is built and under test by the developers, but no installer exists yet. A disk image (.dmg) for the team is planned, and the Mac App Store is planned for the public release. See [OpenFrame Studio for Mac](/mac/).'],
    ['Can I move a project between Windows and Mac?',
      'Both apps use the same project format and the same core engine, so a project folder or a full project package is meant to open on either. This has not yet been proved on a clean machine, so treat it as planned until the release.'],
    ['What are the system requirements?',
      'Windows 11 x64 or macOS 14 on Apple silicon. A minimum for memory and disk space has not been published yet. For reference, the Windows installer is about 27 MB, the optional assistant downloads about 1.2 GB, and its chat model needs at least 4 GB of memory (8 GB recommended).'],
  ]],
  ['Privacy and data', [
    ['Where are my files stored?',
      'In a project folder on your computer, by default in Documents, under OpenFrame. Nothing is stored on an OpenFrame server, because there is none.'],
    ['Do I need an account or the internet?',
      'No account and no internet connection. The app works fully offline. The one thing that needs the internet is the optional assistant download, which you start yourself.'],
    ['Does the app send usage data or check for updates?',
      'No. There is no telemetry, no analytics, no crash upload and no update check. A diagnostics bundle exists, but only you can create it, and you can open and read it before sending it to anyone.'],
    ['Are my projects encrypted?',
      'No. OpenFrame does not encrypt projects. Anyone who can open your project folder can read it. Use your operating system disk encryption if you need that.'],
    ['What happens if the computer crashes or I delete something by mistake?',
      'The app saves as you type and offers to recover after a crash. Deleted items stay in Recently Deleted for 30 days, and project-wide undo and redo are available.'],
  ]],
  ['The assistant', [
    ['Is there AI, and is it required?',
      'There is an optional offline assistant. Nothing in the app needs it, and the app works fully before you download it and after you remove it.'],
    ['Does the assistant send my project anywhere?',
      'No. After a one-time download of about 1.2 GB, it runs on your computer. It talks only to your own machine, and no cloud AI service is used.'],
    ['Will it change my screenplay by itself?',
      'Never. It answers questions about the open project and shows sources, or it proposes changes in a Proposed Changes card. Nothing changes until you read the proposal and choose Apply, and you can undo it afterwards.'],
    ['How good is it?',
      'On the developers\' test set in English it chose the right action 96.7 percent of the time and reached the right end state 88.8 percent of the time. That was measured on one development laptop only, not on a wide range of computers. Other languages work but are clearly weaker.'],
  ]],
  ['Files and formats', [
    ['Which scripts can I import?',
      'Final Draft (.fdx), Fountain, Word (.docx), plain text and pasted text, plus PDF on a best-effort basis. A preview shows what will be imported before anything changes. See [Switch to OpenFrame Studio](/switch/).'],
    ['What can I export?',
      'Screenplays as PDF, Final Draft, Fountain, Word and plain text. Production documents as PDF, CSV or XLSX. Screenplay files carry no OpenFrame branding.'],
    ['Can I write in Hindi, Telugu, Tamil, Kannada or Malayalam?',
      'Yes. You can type, save, search and export those scripts, and the screenplay PDF embeds fonts for them. The app screens themselves are in English. Scripts such as Chinese, Japanese, Korean and Arabic can be typed and exported to other formats, but the screenplay PDF export refuses them with a clear message.'],
  ]],
  ['Pricing and release', [
    ['How much will it cost?',
      'The target is a single payment of 3,000 rupees in India, 89 US dollars elsewhere and 89 euros in Europe. These are targets, not final prices, and nothing is on sale today. See [Pricing](/pricing/).'],
    ['Will there be a free trial?',
      'A 7-day trial is planned for the store release. It is not running today.'],
    ['Will there be a subscription?',
      'No. The plan is to pay once and keep it. The store shows the final price when you buy.'],
    ['Where will I be able to get it?',
      'The public release is planned through the Microsoft Store for Windows and the Mac App Store for Mac. There is no date yet.'],
  ]],
];
