// Shared plain-English copy for the information pages. Source of truth: research/facts.json and
// research/capability-matrix.md (audited 2026-10-21). Nothing here may claim more than those files show.

// Confirmed prices (owner, 2026-10-31): one-time purchase. India sees rupees, every other region US dollars.
export const PRICES = { in: '₹2,999', us: 'US$89' };

// [action, Windows, Mac]. null means the platform has no shortcut for it.
export const SHORTCUTS = [
  ['Search the project', 'Ctrl+K', 'Cmd+K'],
  ['Open or close OpenFrame AI', 'Ctrl+J', 'Cmd+J'],
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
    ['Can several people edit one project at the same time?',
      'No. There is no live collaboration and no cloud sharing. People take turns by sending files: review packages, Beat Board packages and full project packages. The sender sees the replies in the Review Queue and previews every change before applying it.'],
  ]],
  ['Platforms', [
    ['Which systems does it run on?',
      'Windows 11 on 64-bit (x64) computers, and macOS 14 or later on Apple silicon Macs. Windows 10 and Intel Macs are not supported.'],
    ['Where do I get it?',
      'From the Microsoft Store on Windows and the Mac App Store on the Mac. See [Download](/download/).'],
    ['Is the Mac app different from the Windows app?',
      'It does the same work on the same kind of project, in a native Mac app: the Mac menu bar, sheets instead of dialog windows, and Command shortcuts. The [Mac tutorials](/mac/tutorials/) use the real Mac screens.'],
    ['What does it need?',
      'Windows 11 x64 or macOS 14 on Apple silicon. OpenFrame AI downloads about 1.2 GB, and its chat model needs at least 4 GB of memory (8 GB recommended).'],
  ]],
  ['Privacy and data', [
    ['Where are my files stored?',
      'In a project folder on your computer, by default in Documents, under OpenFrame. Nothing is stored on an OpenFrame server, because there is none.'],
    ['Do I need an account or the internet?',
      'No OpenFrame account and no internet connection to work. The app works fully offline. OpenFrame AI download, which you start yourself, needs the internet. The store handles the trial and the purchase.'],
    ['Does the app send usage data?',
      'No. There is no telemetry, no analytics and no crash upload. A diagnostics bundle exists, but only you can create it, and you can open and read it before sending it to anyone.'],
    ['Are my projects encrypted?',
      'No. OpenFrame does not encrypt projects. Anyone who can open your project folder can read it. Use your operating system disk encryption if you need that.'],
    ['What happens if the computer crashes or I delete something by mistake?',
      'The app saves as you work and offers to recover after a crash. Deleted items wait in Recently Deleted, and project-wide undo and redo are available.'],
  ]],
  ['OpenFrame AI', [
    ['Is there AI, and is it required?',
      'There is OpenFrame AI. Nothing in the app needs it, and the app works fully before you download it and after you remove it.'],
    ['Does OpenFrame AI send my project anywhere?',
      'No. After a one-time download of about 1.2 GB, it runs on your computer. It talks only to your own machine, and no cloud AI service is used.'],
    ['Will it change my screenplay by itself?',
      'Never. It answers questions about the open project and shows sources, or it proposes changes in a Proposed Changes card. Nothing changes until you read the proposal and choose Apply, and you can undo it afterwards.'],
  ]],
  ['Files and formats', [
    ['Which scripts can I import?',
      'Final Draft (.fdx), Fountain, Word (.docx), plain text and pasted text, plus PDF on a best-effort basis. A preview shows what will be imported before anything changes. See [Switch to OpenFrame Studio](/switch/).'],
    ['What can I export?',
      'Screenplays as PDF, Final Draft, Fountain, Word and plain text. Production documents as PDF, CSV or XLSX. Screenplay files carry no OpenFrame branding.'],
    ['Can I write in Hindi, Telugu, Tamil, Kannada or Malayalam?',
      'Yes. You can type, save, search and export those scripts, and the screenplay PDF embeds fonts for them. The app screens themselves are in English. Scripts such as Chinese, Japanese, Korean and Arabic can be typed and exported to other formats, but the screenplay PDF export refuses them with a clear message.'],
  ]],
  ['Pricing and trial', [
    ['How much does it cost?',
      'One payment of ₹2,999 in India, or US$89 everywhere else. It is a lifetime purchase, not a subscription. The store shows the final checkout amount, which can include taxes that apply where you live. See [Pricing](/pricing/).'],
    ['Is there a free trial?',
      'Yes. Every new install starts a 7-day free trial with the whole app. After the trial, buy OpenFrame Studio in the store to keep using its full functionality.'],
    ['Is there a subscription?',
      'No. Pay once and keep it.'],
    ['What happens to my projects if I do not buy?',
      'Your project folders stay on your computer. They are ordinary files, and nothing is deleted when a trial ends.'],
  ]],
];
