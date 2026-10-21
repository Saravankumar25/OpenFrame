// Information pages: platform pages, download, pricing, FAQ, switching, compare, privacy, themes, about, contact, legal.
// Truth source: research/facts.json and research/capability-matrix.md. Do not add claims they do not show.
import {
  PRICES, SHORTCUTS, SHORTCUT_NOTE, IMPORT_OK, IMPORT_REFUSED, EXPORTS, THEMES, FAQ,
} from '../content/copy.mjs';

export const SCREENS = {
  win: ['003-app-home-empty', '797-settings-appearance', '079-apphome-settings-appearance'],
  mac: ['003-home-empty', '022-settings-appearance', '024-settings-appearance-theme-chosen'],
};

const UPDATED = '21 October 2026';

export default function (ctx) {
  const { html, md, kbd, shot, osBlocks, cta } = ctx;

  // Build-time guard: a page that breaks the title or description rules fails the build.
  const page = (p) => {
    if (p.title.length > 60) throw new Error(`title too long (${p.title.length}): ${p.path}`);
    const d = p.description.length;
    if (d < 120 || d > 160) throw new Error(`description length ${d}: ${p.path}`);
    return { os: null, alternates: null, nav: null, ...p };
  };

  // Turn key combinations inside a sentence into keycaps.
  const KEY = /((?:Ctrl|Cmd|Alt|Opt|Shift)(?:\+[A-Za-z0-9[\],.]+)+|F7)/;
  const keys = (s) => s.split(KEY).map((part, i) => (i % 2 ? kbd(part) : part));

  const shortcutTable = (idx) => html`
    <div class="i-scroll"><table class="table i-table">
      <thead><tr><th scope="col">Action</th><th scope="col">Shortcut</th></tr></thead>
      <tbody>${SHORTCUTS.filter((r) => r[idx]).map((r) => html`<tr><th scope="row">${r[0]}</th><td>${keys(r[idx])}</td></tr>`)}</tbody>
    </table></div>
    <p class="i-small">${SHORTCUT_NOTE}</p>`;

  const faqGroup = (group, items) => html`
    <section class="i-faq-group" aria-labelledby="g-${group.toLowerCase().replace(/\W+/g, '-')}">
      <h2 id="g-${group.toLowerCase().replace(/\W+/g, '-')}">${group}</h2>
      ${items.map(([q, a]) => html`<details class="i-q"><summary>${q}</summary><div class="i-a"><p>${md(a)}</p></div></details>`)}
    </section>`;

  const head = (h1, lead) => html`<h1>${h1}</h1><p class="lead">${md(lead)}</p>`;

  const statusNote = html`<p class="note"><strong>Status today.</strong> OpenFrame Studio is in internal team testing. The builds are unsigned and ask a team password. There is no public download, no price on sale and no trial running.</p>`;

  // ---------- Windows ----------
  const windows = page({
    path: '/windows/',
    title: 'OpenFrame Studio for Windows',
    description: 'OpenFrame Studio for Windows 11: system needs, how the internal team build installs, keyboard shortcuts and release status. No public download yet.',
    nav: 'download',
    os: 'win',
    alternates: { win: '/windows/', mac: '/mac/' },
    breadcrumbs: [['Windows', '/windows/']],
    body: html`
<section class="section"><div class="wrap i-hero">
  <div class="i-hero-text">
    ${head('OpenFrame Studio for Windows', 'Write, plan and schedule a film in one project on your own computer. This page says what Windows needs, how the current internal build installs, and which keys you will use.')}
    ${statusNote}
    <p><a class="btn btn-ghost" href="/windows/tutorials/">Windows tutorials</a> <a class="btn btn-ghost" href="/download/">All download options</a></p>
  </div>
  ${shot('win', '003-app-home-empty', { alt: 'Application Home in OpenFrame Studio on Windows, before any project has been created.', caption: 'Application Home on Windows, before your first project.' })}
</div></section>

<section class="section tint"><div class="wrap">
  <div class="sec-head"><h2>What you need</h2></div>
  <dl class="kv i-kv">
    <div><dt>System</dt><dd>Windows 11 on a 64-bit (x64) computer. Windows 10 is not supported.</dd></div>
    <div><dt>Window size</dt><dd>At least 1200 by 720 pixels.</dd></div>
    <div><dt>Memory and disk space</dt><dd>A minimum has not been published yet. The installer is about 27 MB. The optional assistant adds a 1.2 GB download and its chat model needs 4 GB of memory at least (8 GB recommended).</dd></div>
    <div><dt>Account and internet</dt><dd>Neither. The app works offline. The internet is used only for the assistant download you start, and by the installer if Windows needs Microsoft's WebView2 component.</dd></div>
  </dl>
</div></section>

<section class="section"><div class="narrow">
  <h2>Release status and download</h2>
  <div class="prose">
    <p>The Windows app exists today as an internal team build: an unsigned installer (.exe) that asks a team password every time it starts. It has no trial, no price and no purchase step.</p>
    <p>There is no public download yet. The public release is planned through the Microsoft Store, with a 7-day trial and a one-time payment. Both are planned, not live. See <a href="/pricing/">Pricing</a>.</p>
  </div>
  <h2>How installation works</h2>
  <p>These are the steps for the current internal installer. The Microsoft Store version will install in the normal Store way and will not show the warning in step 2.</p>
  <ol class="i-steps">
    <li><strong>Run the installer.</strong> It installs for your user account only.</li>
    <li><strong>Get past SmartScreen.</strong> Because the build is unsigned, Windows may show "Windows protected your PC". Choose <strong>More info</strong>, then <strong>Run anyway</strong>. This is expected for an unsigned build. It is not a fault in the app.</li>
    <li><strong>Start OpenFrame Studio and enter the team password.</strong> The password is at least 8 characters. After wrong tries the app makes you wait, from one minute up to thirty. It is a door lock for the test team, not encryption.</li>
    <li><strong>Say what to call you.</strong> The first launch asks for a display name. It stays on this computer. There is no account.</li>
    <li><strong>Optional: add the assistant.</strong> Open Settings, Offline AI, and choose Download OpenFrame AI. The download is about 1.2 GB, is checked against fixed sizes and checksums, and can resume. You can skip this and use the whole app without it.</li>
  </ol>
</div></section>

<section class="section tint"><div class="wrap">
  <div class="sec-head"><h2>Keyboard shortcuts on Windows</h2><p class="lead">Defaults. You can change any of them in Settings.</p></div>
  ${shortcutTable(1)}
</div></section>

<section class="section"><div class="narrow">
  <h2>Good to know on Windows</h2>
  <ul class="ticks">
    <li>Your project is a folder named <strong>Title.openframe</strong>, by default under Documents, in OpenFrame, Projects. Network (UNC) paths are refused, and a folder inside a cloud-sync service gets a warning.</li>
    <li>Settings for the app live in your Local AppData folder, in OpenFrame.</li>
    <li>The project menu inside the window holds Share and Exchange, and Backup and Portability. The left rail holds Home, Idea Vault, Story, Screenplay, Breakdown, Production, Call Sheets and Files.</li>
    <li>Undo and redo use the keyboard. The old Undo and Redo buttons in the top bar were removed.</li>
    <li>Spell check uses the dictionaries installed in Windows, with a main and a second language.</li>
  </ul>
  <p>New to the app? Start with the <a href="/windows/tutorials/">Windows tutorials</a>.</p>
</div></section>
${cta()}`,
  });

  // ---------- Mac ----------
  const mac = page({
    path: '/mac/',
    title: 'OpenFrame Studio for Mac',
    description: 'OpenFrame Studio for macOS 14 or later on Apple silicon: what is planned, how an unsigned build opens, Mac keyboard shortcuts and how it differs.',
    nav: 'download',
    os: 'mac',
    alternates: { win: '/windows/', mac: '/mac/' },
    breadcrumbs: [['Mac', '/mac/']],
    body: html`
<section class="section"><div class="wrap i-hero">
  <div class="i-hero-text">
    ${head('OpenFrame Studio for Mac', 'The same project and the same engine, in a native Mac app. This page says what the Mac needs, where the build stands, and how it differs from Windows.')}
    <p class="note"><strong>Status today.</strong> The Mac app has been built by the developers but is not packaged yet. The internal disk image (.dmg), unsigned and not notarized, is planned and has not been built. There is no public download. The Mac App Store is planned for the public release.</p>
    <p><a class="btn btn-ghost" href="/mac/tutorials/">Mac tutorials</a> <a class="btn btn-ghost" href="/download/">All download options</a></p>
  </div>
  ${shot('mac', '003-home-empty', { alt: 'Application Home in OpenFrame Studio on a Mac, before any project has been created.', caption: 'Application Home on the Mac, before your first project.' })}
</div></section>

<section class="section tint"><div class="wrap">
  <div class="sec-head"><h2>What you need</h2></div>
  <dl class="kv i-kv">
    <div><dt>System</dt><dd>A Mac with Apple silicon, running macOS 14 or later. Intel Macs are not supported.</dd></div>
    <div><dt>Memory and disk space</dt><dd>A minimum has not been published yet. The optional assistant adds a 1.2 GB download and its chat model needs 4 GB of memory at least (8 GB recommended).</dd></div>
    <div><dt>Account and internet</dt><dd>Neither. The app works offline. The internet is used only for the assistant download you start.</dd></div>
  </dl>
</div></section>

<section class="section"><div class="narrow">
  <h2>When the internal build arrives</h2>
  <p>The first Mac build for the team will be an unsigned, not notarized .dmg that asks the team password. macOS Gatekeeper will warn about it, which is normal for an unsigned build.</p>
  <ol class="i-steps">
    <li><strong>Open the .dmg</strong> and drag OpenFrame Studio into Applications.</li>
    <li><strong>Control-click the app, then choose Open</strong>, and confirm in the dialog. If macOS does not offer Open, go to System Settings, Privacy and Security, and choose Open Anyway next to the message about OpenFrame Studio.</li>
    <li><strong>Enter the team password</strong> (at least 8 characters). It is a door lock for the test team, not encryption.</li>
    <li><strong>Optional: add the assistant</strong> from Settings, Offline AI. It is a download of about 1.2 GB and the app works fully without it.</li>
  </ol>
  <p>Until that build exists, there is nothing to install on a Mac. Public release is planned through the Mac App Store, with a 7-day trial and a one-time payment, both planned and not live. See <a href="/pricing/">Pricing</a>.</p>
</div></section>

<section class="section tint"><div class="wrap">
  <div class="sec-head"><h2>Keyboard shortcuts on the Mac</h2><p class="lead">Defaults. You can change any of them in Settings.</p></div>
  ${shortcutTable(2)}
</div></section>

<section class="section"><div class="narrow">
  <h2>How the Mac app differs</h2>
  <ul class="ticks">
    <li>It uses the normal Mac menu bar: File, Edit, View, Go and Help. Import, export, print and close-project commands live in the File menu.</li>
    <li>Dialogs that Windows shows as windows appear as sheets attached to the app window. Settings opens with Cmd+, and Project Settings with Cmd+Shift+,.</li>
    <li>Project undo and redo use Cmd+Opt+Z and Cmd+Opt+Shift+Z. Plain Cmd+Z stays as text undo.</li>
    <li>There is no Save shortcut. The app saves as you type.</li>
    <li>There is no status bar. The window subtitle names the page. The sidebar holds the same eight places as the Windows left rail.</li>
    <li>Search is called Search and Commands and opens with Cmd+K. Spell check uses the macOS spelling service.</li>
    <li>The assistant runs through a separate helper process that uses the Mac's graphics (Metal).</li>
    <li>Your projects are kept in the Documents folder, in OpenFrame, and app settings in Library, Application Support.</li>
  </ul>
  <p>New to the app? Start with the <a href="/mac/tutorials/">Mac tutorials</a>.</p>
</div></section>
${cta()}`,
  });

  // ---------- Download ----------
  const card = (h, href, lines) => html`
    <article class="card i-os-card">
      <h2>${h}</h2>
      ${lines.map((l) => html`<p>${md(l)}</p>`)}
      <p><a class="btn btn-ghost" href="${href}">Read about ${h.replace('OpenFrame Studio for ', '')}</a></p>
    </article>`;

  const download = page({
    path: '/download/',
    title: 'Download OpenFrame Studio: current status',
    description: 'There is no public download of OpenFrame Studio yet. See where the Windows and Mac builds stand and how the public release is planned.',
    nav: 'download',
    breadcrumbs: [['Download', '/download/']],
    body: html`
<section class="section"><div class="narrow">
  ${head('Download OpenFrame Studio', 'There is nothing to download from this site yet. OpenFrame Studio is in internal team testing, and the public release is planned through the Microsoft Store and the Mac App Store.')}
  ${osBlocks({
    win: html`<p class="note i-you">You are viewing Windows details. More on <a href="/windows/">OpenFrame Studio for Windows</a>.</p>`,
    mac: html`<p class="note i-you">You are viewing macOS details. More on <a href="/mac/">OpenFrame Studio for Mac</a>.</p>`,
  })}
</div>
<div class="wrap i-os-grid">
  ${card('OpenFrame Studio for Windows', '/windows/', [
    '**Needs:** Windows 11, 64-bit.',
    '**Build today:** an unsigned internal .exe of about 27 MB for the invited team. It asks a team password.',
    '**Public release:** planned on the Microsoft Store. No date yet.',
  ])}
  ${card('OpenFrame Studio for Mac', '/mac/', [
    '**Needs:** macOS 14 or later, Apple silicon.',
    '**Build today:** none. An unsigned internal .dmg is planned and not built.',
    '**Public release:** planned on the Mac App Store. No date yet.',
  ])}
</div>
<div class="narrow">
  <h2>What to expect</h2>
  <ul class="ticks">
    <li>There are no download buttons here because there is no public file. Anything else that offers an OpenFrame Studio installer is not from us.</li>
    <li>Internal builds have no trial and no price. A 7-day trial and a one-time payment are planned for the store release. See <a href="/pricing/">Pricing</a>.</li>
    <li>The optional assistant is a separate download of about 1.2 GB that you start inside the app.</li>
    <li>Windows 10 and Intel Macs are not supported.</li>
  </ul>
  <p>Questions? See the <a href="/faq/">questions and answers</a> or <a href="/contact/">Contact</a>.</p>
</div></section>`,
  });

  // ---------- Pricing ----------
  const pricing = page({
    path: '/pricing/',
    title: 'Pricing | OpenFrame Studio',
    description: 'The planned model for OpenFrame Studio is one payment, yours to keep. Target prices and the 7-day trial are planned for the store release, not on sale.',
    nav: 'pricing',
    breadcrumbs: [['Pricing', '/pricing/']],
    body: html`
<section class="section"><div class="narrow">
  ${head('Pricing', 'The plan is simple: pay once and keep it. Nothing is on sale yet. The figures below are targets for the store release.')}
  <p class="note"><strong>Not on sale.</strong> There is no price to pay today. The numbers are targets and may change. The store will show the final price when you buy.</p>
  <h2>The planned model</h2>
  <div class="prose">
    <p>One payment, no subscription, no renewal. You pay once for the app and own it.</p>
    <p>Purchases will be made through the Microsoft Store on Windows and the Mac App Store on the Mac. The app never shows a price. The store's own purchase window does.</p>
  </div>
  <h2>Target prices</h2>
  <div class="i-scroll"><table class="table i-table">
    <thead><tr><th scope="col">Region</th><th scope="col">Target price</th><th scope="col">State</th></tr></thead>
    <tbody>${PRICES.map(([r, p]) => html`<tr><th scope="row">${r}</th><td>${p}</td><td>Target, not on sale</td></tr>`)}</tbody>
  </table></div>
  <h2>The 7-day trial</h2>
  <div class="prose">
    <p>A free 7-day trial is planned for the store release. It does not need a credit card to start. It is <strong>not running yet</strong>, because the app is not in the stores.</p>
    <p>The store version is designed to offer only Buy and Restore once a trial ends without a purchase. Your project files stay on your computer.</p>
    <p>Still open: the final prices, the refund rules, and exactly what the trial includes.</p>
  </div>
  <h2>What one purchase covers</h2>
  <ul class="ticks">
    <li>The whole app. There are no editions and no locked parts: Idea Vault, Beat Board, screenplay editor, breakdown, schedule, call sheets, reports and budget.</li>
    <li>The optional offline assistant. It runs on your own computer, so it has no running cost.</li>
    <li>Import, export, backups and project packages.</li>
    <li>No account, and no data leaving your computer.</li>
  </ul>
  <h2>Internal builds</h2>
  <p>The builds the team tests today have no trial, no price, no purchase step and no lock, other than the team password.</p>
  <h2>Quick answers</h2>
  ${FAQ.find(([g]) => g === 'Pricing and release')[1].map(([q, a]) => html`<details class="i-q"><summary>${q}</summary><div class="i-a"><p>${md(a)}</p></div></details>`)}
  <p>More in the <a href="/faq/">questions and answers</a>.</p>
</div></section>
${cta()}`,
  });

  // ---------- FAQ ----------
  const faq = page({
    path: '/faq/',
    title: 'Questions and answers | OpenFrame Studio',
    description: 'Answers about OpenFrame Studio: platforms, privacy and data, the offline assistant, files and formats, and the planned price, trial and release.',
    nav: 'faq',
    breadcrumbs: [['Questions', '/faq/']],
    body: html`
<section class="section"><div class="narrow">
  ${head('Questions and answers', 'Short, plain answers. If something here is planned and not live, the answer says so.')}
  <nav class="i-jump" aria-label="Question groups">${FAQ.map(([g]) => html`<a href="#g-${g.toLowerCase().replace(/\W+/g, '-')}">${g}</a>`)}</nav>
  ${FAQ.map(([g, items]) => faqGroup(g, items))}
  <p>Not answered here? See <a href="/contact/">Contact</a>.</p>
</div></section>
${cta()}`,
  });

  // ---------- Switch ----------
  const migrate = page({
    path: '/switch/',
    title: 'Switch to OpenFrame Studio | Import and export',
    description: 'Bring a screenplay into OpenFrame Studio from Final Draft, Fountain, Word, PDF or text, see a preview first, and export it back out any time.',
    breadcrumbs: [['Switch', '/switch/']],
    body: html`
<section class="section"><div class="narrow">
  ${head('Switch to OpenFrame Studio', 'Bring your script in, check what will happen, and know you can take it out again in a standard format.')}
  <h2>What you can import</h2>
  <div class="i-scroll"><table class="table i-table">
    <thead><tr><th scope="col">Format</th><th scope="col">Detail</th></tr></thead>
    <tbody>${IMPORT_OK.map(([f, d]) => html`<tr><th scope="row">${f}</th><td>${d}</td></tr>`)}</tbody>
  </table></div>
  <p>PDF import is best effort. A PDF has no real screenplay structure inside it, so the app guesses the elements. Read the preview before you accept.</p>
  <h2>You see a preview first</h2>
  <p>Import Screenplay shows a preview with counts and warnings before anything in your project changes. Four sample scripts are offered if you want to try the steps with nothing of your own.</p>
  <h2>What is refused</h2>
  <p>These formats are not opened. The app says so with a clear message instead of guessing.</p>
  <ul class="i-chips">${IMPORT_REFUSED.map((f) => html`<li>${f}</li>`)}</ul>
  <p>If your script is in one of these, save or export it as Final Draft, Fountain, Word or plain text from the program that made it, then import that file.</p>
  <h2>Other things you can bring</h2>
  <ul class="ticks">
    <li>The Idea Vault takes notes, images, links, any file and video, and voice notes recorded in the app.</li>
    <li>OpenFrame packages that it opens: backups (.ofbackup), full projects (.ofproject), screenplay review packages (.ofscriptreview), Beat Board packages (.ofstory) and replies (.ofresponse).</li>
    <li>Older breakdown, shot list, schedule and call sheet review files still import.</li>
  </ul>
  <h2>What you can export</h2>
  <div class="i-scroll"><table class="table i-table">
    <thead><tr><th scope="col">Document</th><th scope="col">Formats</th></tr></thead>
    <tbody>${EXPORTS.map(([d, f]) => html`<tr><th scope="row">${d}</th><td>${f}</td></tr>`)}</tbody>
  </table></div>
  <p>Screenplay files carry no OpenFrame branding. Production PDFs carry a small "Created using OpenFrame" line. CSV files stay clean.</p>
  <h2>Writing in your own language</h2>
  <p>You can type, save, search and export Hindi, Telugu, Tamil, Kannada and Malayalam text, and the screenplay PDF embeds fonts for them. Other scripts such as Chinese, Japanese, Korean and Arabic can be typed and exported to Final Draft, Fountain and Word, but the screenplay PDF refuses them with a clear message.</p>
  <p>Next: <a href="/features/import-export/">Import and Export</a> or the <a href="/faq/">questions and answers</a>.</p>
</div></section>
${cta()}`,
  });

  // ---------- Compare ----------
  const rows = [
    ['Ideas and research', 'A notes app, a folder of images and bookmarks.', 'The Idea Vault holds notes, images, links, files and voice notes inside the project.'],
    ['Story outline', 'Index cards, a document or a whiteboard.', 'The Beat Board holds acts, sequences, beats and scene cards. Build Screenplay turns cards into scenes after a preview.'],
    ['The screenplay', 'A screenwriting program.', 'A screenplay editor in the same project, with drafts, comparison and shooting-draft locking.'],
    ['Breakdown', 'A spreadsheet or a separate breakdown program.', 'Tag each scene in the script itself, with suggestions you accept or reject and a shared catalog.'],
    ['Schedule', 'A spreadsheet or a board in another tool.', 'A stripboard tied to the breakdown and to cast availability. It warns about problems and never moves scenes on its own.'],
    ['Call sheets', 'A document typed again for each day.', 'Created from the shooting day and refreshed when the schedule changes.'],
    ['Backups', 'Copies you remember to make.', 'Autosave, Recently Deleted, backups and full project packages.'],
  ];
  const compare = page({
    path: '/compare/',
    title: 'One app or many tools | OpenFrame Studio',
    description: 'An honest look at using OpenFrame Studio as one app for ideas, script and production planning versus several separate tools, with its limits stated.',
    breadcrumbs: [['Compare', '/compare/']],
    body: html`
<section class="section"><div class="narrow">
  ${head('One app or several tools', 'Many productions run on a notes app, a screenwriting program, spreadsheets and documents. This page compares that way of working with one app, and says where one app is the wrong choice.')}
  <h2>The same job, two ways</h2>
  <div class="i-scroll"><table class="table i-table i-compare">
    <thead><tr><th scope="col">Job</th><th scope="col">With separate tools</th><th scope="col">In OpenFrame Studio</th></tr></thead>
    <tbody>${rows.map(([j, a, b]) => html`<tr><th scope="row">${j}</th><td>${a}</td><td>${b}</td></tr>`)}</tbody>
  </table></div>
  <h2>What one app gives you</h2>
  <ul class="ticks">
    <li>Change a scene once and the breakdown, the schedule and the call sheet know about it.</li>
    <li>No copying between files, and no version of the truth hiding in an old spreadsheet.</li>
    <li>One place to back up and one project file to send.</li>
    <li>Everything stays on your computer, with no account.</li>
  </ul>
  <h2>Where separate tools may suit you better</h2>
  <ul class="ticks">
    <li>You need several people editing the same document at the same moment. OpenFrame has no live collaboration. People pass files back and forth instead.</li>
    <li>You want your work available on a phone or in a browser. OpenFrame is a desktop app for Windows 11 and for Apple silicon Macs.</li>
    <li>You need accounting or payroll. The budget is a light planning sheet.</li>
    <li>You are happy with a tool you already know well and only need one part of the job.</li>
  </ul>
  <h2>Not released yet</h2>
  <p>OpenFrame Studio is in internal team testing. There is no public download today. You can follow the state on the <a href="/download/">download page</a>.</p>
</div></section>
${cta()}`,
  });

  // ---------- Local and private ----------
  const local = page({
    path: '/local-and-private/',
    title: 'Local and private | OpenFrame Studio',
    description: 'OpenFrame Studio keeps your work on your computer: no account, no telemetry, no update check. The few network uses are listed, and encryption is not offered.',
    breadcrumbs: [['Local and private', '/local-and-private/']],
    body: html`
<section class="section"><div class="narrow">
  ${head('Local and private', 'Your scripts and plans stay on your computer. This page says exactly what the app does, and does not do, with your data and your network.')}
  <h2>Where your work lives</h2>
  <p>Each project is one folder named <strong>Title.openframe</strong>. Inside are the project database (SQLite), your assets, recovery copies and backups. By default it sits in Documents, in OpenFrame. You can copy, move or back up that folder like any other.</p>
  <h2>What the app does not do</h2>
  <ul class="ticks">
    <li>No account and no sign-in.</li>
    <li>No telemetry, no analytics and no crash upload.</li>
    <li>No update check. The app does not call home to ask whether a new version exists.</li>
    <li>No network service that other computers can reach.</li>
  </ul>
  <h2>The only times it uses the network</h2>
  <ol class="i-steps">
    <li><strong>The assistant download you start.</strong> It fetches about 1.2 GB of model files from huggingface.co and its content network, nothing else. Each file is checked against a fixed size and checksum built into the app. You can also import the files by hand.</li>
    <li><strong>A saved link in the Idea Vault.</strong> When you click it, it opens in your normal browser.</li>
    <li><strong>The Windows installer.</strong> If Windows lacks Microsoft's WebView2 component, the installer fetches it.</li>
  </ol>
  <h2>The offline assistant</h2>
  <p>After the download, the assistant runs on your computer and talks only to your own machine, using a key made for each launch. It never changes your project without your Apply, never sends project data anywhere, and does not read other people's private notes or your Story page.</p>
  <h2>Projects are not encrypted</h2>
  <p class="note">OpenFrame does not encrypt your projects. Anyone who can open the project folder or a backup file can read it. If you need that protection, use your operating system's disk encryption. The team password on internal builds is a door lock for testers, not encryption.</p>
  <h2>Backups, packages and diagnostics</h2>
  <ul class="ticks">
    <li>Create a backup (.ofbackup) or export a full project (.ofproject) to move a project to another computer.</li>
    <li>Share a screenplay or Beat Board for review as a package file. Send it however you like. The reply comes back as a file, and you preview its changes before applying them.</li>
    <li>A diagnostics bundle exists, but only you can create it. You can read it before you decide to share it. Local log files are kept for 14 days.</li>
    <li>Network (UNC) paths are refused as project locations, and a folder inside a cloud-sync service gets a warning, because two programs writing one project can damage it.</li>
  </ul>
  <p>The website has its own notice: <a href="/legal/privacy/">website privacy notice</a>.</p>
</div></section>
${cta()}`,
  });

  // ---------- Themes ----------
  const themes = page({
    path: '/themes/',
    title: 'Themes | OpenFrame Studio',
    description: 'OpenFrame Studio has 17 built-in themes in Light, Dark or System, plus your own imported themes. Your screenplay prints in the standard look either way.',
    breadcrumbs: [['Themes', '/themes/']],
    body: html`
<section class="section"><div class="narrow">
  ${head('Themes', 'Choose how the app looks. Your documents do not change.')}
  <h2>17 built-in themes</h2>
  <ul class="i-chips i-chips-wide">${THEMES.map((t) => html`<li>${t}</li>`)}</ul>
  <p>Each theme works in <strong>Light</strong> or <strong>Dark</strong>, or follows your system with <strong>System</strong>. High Contrast is there for people who need stronger separation.</p>
</div>
<div class="wrap i-shots">
  ${osBlocks({
    win: html`${shot('win', '797-settings-appearance', { alt: 'The Appearance page of Settings in OpenFrame Studio on Windows, listing the built-in themes.', caption: 'Settings, Appearance, on Windows.' })}${shot('win', '079-apphome-settings-appearance', { alt: 'The Appearance settings opened from Application Home on Windows.', caption: 'The same page opened from Application Home.' })}`,
    mac: html`${shot('mac', '022-settings-appearance', { alt: 'The Appearance page of Settings in OpenFrame Studio on a Mac, listing the built-in themes.', caption: 'Settings, Appearance, on the Mac.' })}${shot('mac', '024-settings-appearance-theme-chosen', { alt: 'The Appearance page on a Mac after a different theme has been chosen.', caption: 'After choosing another theme.' })}`,
  })}
</div>
<div class="narrow">
  <h2>Your own themes</h2>
  <p>Import a theme file or paste one in. The app checks it for readable contrast, shows what it found, and keeps it under My Themes. You can rename, export or delete your own themes.</p>
  <h2>Documents keep the standard look</h2>
  <p>A theme changes the app, not your pages. Print preview and exports such as PDF use the standard screenplay look in every theme.</p>
  <p>Looking for more? See <a href="/features/">Features</a>.</p>
</div></section>
${cta()}`,
  });

  // ---------- About ----------
  const about = page({
    path: '/about/',
    title: 'About | OpenFrame Studio',
    description: 'OpenFrame Studio is a desktop app for writers, screenwriters and production teams that keeps every project on your own computer. This is why it exists.',
    breadcrumbs: [['About', '/about/']],
    body: html`
<section class="section"><div class="narrow">
  ${head('About OpenFrame Studio', 'A desktop app for writers, screenwriters and production teams. One app for everyone, with no separate editions.')}
  <h2>What it is for</h2>
  <p>A film often starts as scattered notes and ends as a pile of separate files. OpenFrame Studio keeps the whole path in one project folder: first idea, story, screenplay, breakdown, schedule, call sheets and a light budget.</p>
  <h2>What we believe</h2>
  <ul class="ticks">
    <li><strong>Your work is yours.</strong> It lives on your computer. No account, no cloud service, no telemetry.</li>
    <li><strong>You stay in charge.</strong> The optional assistant only suggests. Nothing changes until you choose Apply.</li>
    <li><strong>Say only what is true.</strong> This site claims nothing the app cannot show, and says plainly what is planned.</li>
    <li><strong>No lock-in.</strong> Standard formats in, standard formats out.</li>
  </ul>
  <h2>How it is built</h2>
  <p>One engine, written in Rust over SQLite, holds the rules, storage, search, import and export. Windows uses a Tauri shell and the Mac uses a native SwiftUI shell, both on that same engine. The offline assistant runs on llama.cpp.</p>
  <h2>Where we are</h2>
  <p>The work is in three stages: finish and test the app on Windows and the Mac, build this website, then release through the Microsoft Store and the Mac App Store. We are in the first stage, with internal team testing. See <a href="/download/">Download</a> for the current state.</p>
  <p>The company name, legal details and a public contact address will be published when the official domain is ready. See <a href="/contact/">Contact</a>.</p>
</div></section>
${cta()}`,
  });

  // ---------- Contact ----------
  const contact = page({
    path: '/contact/',
    title: 'Contact | OpenFrame Studio',
    description: 'No public contact address for OpenFrame Studio has been set yet. It will be published here with the official domain. There is no contact form on this site.',
    breadcrumbs: [['Contact', '/contact/']],
    body: html`
<section class="section"><div class="narrow">
  ${head('Contact', 'There is no public contact address yet.')}
  <div class="prose">
    <p>We have not chosen an official domain or a company email address yet. When they exist, the address will be published on this page. Until then there is nothing to write to, and we would rather say so than show an address that does not work.</p>
    <p>This site has no contact form and collects nothing, so a message typed here would go nowhere.</p>
    <p>If you are part of the invited testing team, report problems the way you were told when you were invited.</p>
  </div>
  <p>Many answers are already on the <a href="/faq/">questions and answers</a> page, and the state of the release is on <a href="/download/">Download</a>.</p>
</div></section>`,
  });

  // ---------- Legal ----------
  const legalNote = (t) => html`<p class="note"><strong>${t}</strong> Website notice, not the product licence. Final legal terms will be published before release. Last updated ${UPDATED}.</p>`;

  const privacy = page({
    path: '/legal/privacy/',
    title: 'Website privacy notice | OpenFrame Studio',
    description: 'How this static website treats visitor data: no cookies, no analytics and no forms. Cloudflare serves the pages and may keep standard request logs.',
    breadcrumbs: [['Privacy notice', '/legal/privacy/']],
    body: html`
<section class="section"><div class="narrow">
  <h1>Website privacy notice</h1>
  ${legalNote('Interim notice.')}
  <div class="prose">
    <h2>What this covers</h2>
    <p>This notice is about this website only. It is not about the OpenFrame Studio app, which has its own behaviour described on <a href="/local-and-private/">Local and private</a>. The app has no account, no telemetry and no update check.</p>
    <h2>What the site does not do</h2>
    <ul class="ticks">
      <li>It sets no cookies.</li>
      <li>It runs no analytics and no advertising or tracking scripts.</li>
      <li>It has no forms, no accounts and no email sign-up, so it collects no personal details from you.</li>
      <li>It does not load scripts, fonts or images from other companies.</li>
    </ul>
    <h2>What happens when you visit</h2>
    <p>The pages are served by Cloudflare, which hosts the site. Like any web host, Cloudflare receives your request and may keep standard request logs, such as your IP address, the page asked for, the time and your browser type. These logs are handled by Cloudflare under its own policies. We do not add anything to them and we do not use them to identify you.</p>
    <p>If a page remembers a display choice, such as light or dark, it keeps it only in your own browser and never sends it to us.</p>
    <h2>Links</h2>
    <p>If a link takes you to another site, that site has its own policy.</p>
    <h2>Contact and changes</h2>
    <p>No public contact address exists yet. See <a href="/contact/">Contact</a>. This notice will be updated when the official domain and the company details are set.</p>
  </div>
</div></section>`,
  });

  const terms = page({
    path: '/legal/terms/',
    title: 'Website terms | OpenFrame Studio',
    description: 'Interim terms for using this website. The OpenFrame Studio product terms and licence are not published yet and will appear before the public release.',
    breadcrumbs: [['Terms', '/legal/terms/']],
    body: html`
<section class="section"><div class="narrow">
  <h1>Website terms</h1>
  ${legalNote('Interim terms.')}
  <div class="prose">
    <h2>Product terms are not published yet</h2>
    <p>The licence and terms for the OpenFrame Studio app itself are not written yet. They will be published before any public release. The internal test builds are for invited testers only.</p>
    <h2>Using this website</h2>
    <ul class="ticks">
      <li>You may read this site and link to it.</li>
      <li>The site is information only. It is not an offer to sell anything. Prices shown are targets and nothing is on sale.</li>
      <li>We try to keep the pages accurate and say what is planned and what is live, but the app is still changing. Features, platforms and plans may change.</li>
      <li>The site is provided as it is, without any promise that it is complete or always available.</li>
      <li>Please do not copy the site's text or screens and present them as your own, or use the OpenFrame Studio name in a way that suggests we endorse you.</li>
    </ul>
    <h2>Other parties</h2>
    <p>Other product names mentioned on this site belong to their owners. Links to other sites are for convenience, and we are not responsible for what they contain.</p>
    <h2>Company details and law</h2>
    <p>The legal entity, the governing law and a contact address have not been set. They will be added when the official domain is ready. See also the <a href="/legal/privacy/">website privacy notice</a> and <a href="/contact/">Contact</a>.</p>
  </div>
</div></section>`,
  });

  return [windows, mac, download, pricing, faq, migrate, compare, local, themes, about, contact, privacy, terms];
}
