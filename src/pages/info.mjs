// Information pages in the approved website design: platform pages, download, pricing, FAQ, switching, compare,
// privacy, themes, about, contact and legal. Truth source: research/facts.json and
// research/capability-matrix.md. Do not add claims they do not show.
import { FOUNDERS } from '../content/founders.mjs';
import { SHORTCUTS, SHORTCUT_NOTE, IMPORT_OK, IMPORT_REFUSED, EXPORTS, THEMES, FAQ } from '../content/copy.mjs';

const UPDATED = '31 October 2026';

// Structured data for search and answer engines: the company with its founders, and the app with its prices.
export const ORG = (ctx) => ({ '@context': 'https://schema.org', '@type': 'Organization', name: 'OpenFrame Studio', url: ctx.SITE_URL + '/',
  logo: ctx.SITE_URL + '/favicon.svg',
  founder: FOUNDERS.map((f) => ({ '@type': 'Person', name: f.name, jobTitle: f.role, description: f.text[0], ...(f.link ? { url: f.link, sameAs: [f.link] } : {}) })),
 });
export const APP = (ctx, os, p = '/') => ({ '@context': 'https://schema.org', '@type': 'SoftwareApplication', name: 'OpenFrame Studio', url: ctx.SITE_URL + p,
  applicationCategory: 'MultimediaApplication', applicationSubCategory: 'Screenwriting and film production planning', operatingSystem: os,
  description: 'A local-first desktop app for writers, screenwriters and film production teams: ideas, story, screenplay, breakdown, schedule, call sheets and reports in one project.',
  publisher: { '@type': 'Organization', name: 'OpenFrame Studio' },
  offers: [{ '@type': 'Offer', price: '2999', priceCurrency: 'INR', eligibleRegion: { '@type': 'Country', name: 'IN' } }, { '@type': 'Offer', price: '89', priceCurrency: 'USD' }] });
const slug = (s) => s.toLowerCase().replace(/\W+/g, '-');

export default function (ctx) {
  const { html, md, mock, osBlocks, storeBtn, price, cta } = ctx;

  // Build-time guard: a page that breaks the title or description rules fails the build.
  const page = (p) => {
    if (p.title.length > 60) throw new Error(`title too long (${p.title.length}): ${p.path}`);
    const d = p.description.length;
    if (d < 110 || d > 165) throw new Error(`description length ${d}: ${p.path}`);
    return { os: null, alternates: null, nav: null, ...p };
  };
  const kbd = (combo) => html`<span class="keys">${combo.split('+').map((k, i) => html`${i ? '+' : ''}<kbd>${k}</kbd>`)}</span>`;
  const KEY = /((?:Ctrl|Cmd|Alt|Opt|Shift)(?:\+[A-Za-z0-9[\],.]+)+|F7)/;
  const keys = (s) => s.split(KEY).map((part, i) => (i % 2 ? kbd(part) : part));
  const phero = (eyebrow, h1, lead, extra = '') => html`<section class="phero"><div class="wrap"><span class="eyebrow">${eyebrow}</span><h1>${h1}</h1><p class="lead">${md(lead)}</p>${extra}</div></section>`;
  const shortcutTable = (idx) => html`<div class="tbl-wrap"><table class="tbl"><thead><tr><th scope="col">Action</th><th scope="col">Shortcut</th></tr></thead>
<tbody>${SHORTCUTS.filter((r) => r[idx]).map((r) => html`<tr><td><b>${r[0]}</b></td><td>${keys(r[idx])}</td></tr>`)}</tbody></table></div>
<p class="small-print">${SHORTCUT_NOTE}</p>`;
  const faqItem = ([q, a]) => html`<details class="faq-item"><summary>${q}</summary><div class="a"><p>${md(a)}</p></div></details>`;

  // ---------- Platform pages ----------
  const platform = (os) => {
    const win = os === 'win';
    const name = win ? 'Windows' : 'Mac';
    return page({
      path: win ? '/windows/' : '/mac/', os, alternates: { win: '/windows/', mac: '/mac/' }, nav: 'download',
      title: `OpenFrame Studio for ${name}`,
      description: win
        ? 'OpenFrame Studio for Windows 11: what it needs, how it installs from the Microsoft Store, the keyboard shortcuts and the step-by-step Windows tutorials.'
        : 'OpenFrame Studio for Mac on Apple silicon: what it needs, the Mac App Store, Mac keyboard shortcuts, how it differs, and the step-by-step Mac tutorials.',
      breadcrumbs: [[name, win ? '/windows/' : '/mac/']],
      jsonld: [APP(ctx, win ? 'Windows 11' : 'macOS 14 or later (Apple silicon)', win ? '/windows/' : '/mac/')],
      body: html`
${phero(win ? 'Windows 11' : 'macOS on Apple silicon', `OpenFrame Studio for ${name}.`, win
        ? 'Write, plan and schedule a film in one project on your own computer, in an app made for Windows 11.'
        : 'The same project and the same engine, in a native Mac app with the Mac menu bar, sheets and Command shortcuts.',
      html`<div class="hero-cta">${storeBtn(os, 'btn-lg')}<a class="btn btn-ghost btn-lg" href="${win ? '/windows/tutorials/' : '/mac/tutorials/'}">${name} tutorials</a></div>
<div class="tour-real">${mock(os, win ? 'app-home-empty' : 'home-empty', { caption: `Application Home on ${name}, before your first project.`, eager: true })}</div>`)}
<section class="tint sec-s"><div class="wrap narrow"><div class="sec-head"><h2>What you need</h2></div>
<div class="tbl-wrap"><table class="tbl"><tbody>
<tr><td><b>System</b></td><td>${win ? 'Windows 11 on a 64-bit (x64) computer. Windows 10 is not supported.' : 'A Mac with Apple silicon, running macOS 14 or later. Intel Macs are not supported.'}</td></tr>
${win ? html`<tr><td><b>Window size</b></td><td>At least 1200 by 720 pixels.</td></tr>` : ''}
<tr><td><b>Account and internet</b></td><td>No OpenFrame account. The app works offline. The internet is used for the store and for OpenFrame AI download that you start.</td></tr>
<tr><td><b>OpenFrame AI</b></td><td>A one-time download of about 1.2 GB. Its chat model needs at least 4 GB of memory, 8 GB recommended.</td></tr>
<tr><td><b>Price</b></td><td>7-day free trial, then one payment of ${price()}.</td></tr>
</tbody></table></div></div></section>
<section class="sec-s" id="install"><div class="wrap"><div class="sec-head"><h2>Get started in four steps</h2></div>
<div class="steps3 four">
<div class="step"><div class="n">1</div><h3>Install</h3><p>${win ? 'Get OpenFrame Studio from the Microsoft Store. The Store installs and updates it.' : 'Get OpenFrame Studio from the Mac App Store. The App Store installs and updates it.'}</p></div>
<div class="step"><div class="n">2</div><h3>Choose where projects live</h3><p>The first start asks where to keep your projects. The suggested place is fine for most people.</p></div>
<div class="step"><div class="n">3</div><h3>Say what to call you</h3><p>Your name appears on notes, tasks and activity. It stays on this computer. There is no account.</p></div>
<div class="step"><div class="n">4</div><h3>Create a project</h3><p>Create Project asks for a title. Everything else is optional and can be changed later.</p></div>
</div>
<p class="center mt26"><a class="more" href="${win ? '/windows/tutorials/' : '/mac/tutorials/'}">Follow the step-by-step ${name} tutorials &rarr;</a></p></div></section>
<section class="tint sec-s"><div class="wrap narrow"><div class="sec-head"><h2>Keyboard shortcuts on ${name}</h2><p class="lead">The defaults. You can change any of them in Settings.</p></div>
${shortcutTable(win ? 1 : 2)}</div></section>
<section class="sec-s"><div class="wrap narrow"><div class="facts"><h3>Good to know on ${name}</h3><ul class="ticks">
${win ? html`<li><span>Your project is a folder named <b>Title.openframe</b>, by default under Documents, in OpenFrame. Network (UNC) paths are refused, and a folder inside a cloud-sync service gets a warning.</span></li>
<li><span>The left rail holds Home, Idea Vault, Story, Screenplay, Breakdown, Production, Call Sheets and Files. The project menu holds sharing, backups and project tools.</span></li>
<li><span>Spell check uses the dictionaries installed in Windows, with a main and a second language.</span></li>`
        : html`<li><span>It uses the Mac menu bar: File, Edit, View, Go, Window and Help. Import, export, print and Close Project live in the File menu.</span></li>
<li><span>Dialogs appear as sheets attached to the window. Settings opens with Command and comma.</span></li>
<li><span>Project undo and redo use Command, Option and Z. Plain Command and Z undoes text.</span></li>
<li><span>Search and Commands opens with Command and K. Spell check uses the macOS spelling service.</span></li>`}
</ul></div>
<p class="center mt26"><a class="btn btn-dark" href="${win ? '/windows/tutorials/' : '/mac/tutorials/'}">Read the ${name} tutorials</a></p></div></section>
${cta()}`,
    });
  };

  // ---------- Download ----------
  const download = page({
    path: '/download/', nav: 'download',
    title: 'Download OpenFrame Studio for Windows and Mac',
    description: 'Get OpenFrame Studio from the Microsoft Store for Windows 11 or the Mac App Store for Apple silicon Macs. 7-day free trial, then one payment.',
    breadcrumbs: [['Download', '/download/']],
    body: html`
${phero('Download', 'Get OpenFrame Studio.', 'From the Microsoft Store on Windows and the Mac App Store on the Mac. Try it free for 7 days.')}
<section class="sec-s pt30"><div class="wrap"><div class="price-grid">
<div class="pcard win rv"><span class="badge">Windows 11</span><h3>OpenFrame Studio for Windows</h3><p class="price-sub mt10">Installed and updated by the Microsoft Store.</p>
<ul class="ticks"><li><span>Windows 11, 64-bit (x64)</span></li><li><span>7-day free trial, then one payment</span></li><li><span>Works offline, no OpenFrame account</span></li></ul>${storeBtn('win', 'btn-lg')}
<p class="small-print"><a href="/windows/">More about Windows</a></p></div>
<div class="pcard mac rv d1"><span class="badge">Mac</span><h3>OpenFrame Studio for Mac</h3><p class="price-sub mt10">Installed and updated by the Mac App Store.</p>
<ul class="ticks"><li><span>macOS 14 or later on Apple silicon</span></li><li><span>7-day free trial, then one payment</span></li><li><span>Works offline, no OpenFrame account</span></li></ul>${storeBtn('mac', 'btn-lg')}
<p class="small-print"><a href="/mac/">More about the Mac</a></p></div>
</div></div></section>
<section class="tint sec-s" id="trial"><div class="wrap"><div class="sec-head"><span class="eyebrow">Have trust issues? Good.</span><h2>Your 7-day free trial, step by step.</h2></div>
<div class="steps3">
<div class="step rv"><div class="n">1</div><h3>Install</h3><p>Get the app from the Microsoft Store or the Mac App Store. The 7-day trial starts with the whole app.</p></div>
<div class="step rv d1"><div class="n">2</div><h3>Make something real</h3><p>Write a scene, build a Beat Board, schedule a day, make a call sheet. A small chip shows the days left.</p></div>
<div class="step rv d2"><div class="n">3</div><h3>Decide</h3><p>Before the trial ends the app reminds you. Buy in the store to keep using the full app. Your project folders stay on your computer either way.</p></div>
</div></div></section>`,
  });

  // ---------- Pricing ----------
  const pricing = page({
    path: '/pricing/', nav: 'pricing',
    title: 'Pricing: pay once, own forever | OpenFrame Studio',
    description: 'OpenFrame Studio is a one-time lifetime purchase: ₹2,999 in India and US$89 everywhere else, after a 7-day free trial. No subscription and no renewal.',
    breadcrumbs: [['Pricing', '/pricing/']],
    body: html`
${phero('Pricing', 'Pay once. Own forever.', 'No subscription, no renewal, no rented software. One payment for the whole app, after a 7-day free trial.')}
<section class="sec-s pt30"><div class="wrap"><div class="price-one">
<div class="pcard win rv"><span class="badge">Lifetime purchase</span>
<div class="region-tg" role="group" aria-label="Price region"><button type="button" data-region-set="in" aria-pressed="false">India</button><button type="button" data-region-set="world" aria-pressed="false">Outside India</button></div>
<h3>OpenFrame Studio</h3>
<div class="price"><b>${price()}</b></div>
<p class="price-sub">One payment for <span data-region-name>your region</span>. Windows and Mac. 7-day free trial first.</p>
<ul class="ticks">
<li><span>Idea Vault, Story page, Beat Board and moodboards</span></li>
<li><span>Screenplay editor with drafts, review and revisions</span></li>
<li><span>Import and export: Final Draft, Fountain, PDF, Word and text</span></li>
<li><span>Breakdown, catalog, cast, crew and locations</span></li>
<li><span>Shot lists, storyboards and the shooting schedule</span></li>
<li><span>Call sheets, sides, Daily View, reports and a light budget</span></li>
<li><span>Backups and project packages</span></li>
<li><span>OpenFrame AI, with no running cost</span></li>
</ul>
<div class="store-col">${storeBtn('win')}${storeBtn('mac')}</div>
<p class="price-note">The Microsoft Store and the Mac App Store set the final checkout amount under their own rules, which can include taxes where you live. The website does not convert currencies.</p></div>
</div></div></section>
<section class="dark"><div class="wrap manifesto">
<div class="rv"><span class="eyebrow">Rent vs own</span><h2>Renting looks cheap in month one.</h2>
<p class="lead mt18">Five tools, five bills, every month, for as long as you make films. Or one payment, and it is yours. Break the chain of corporate rentals.</p>
<p class="big-line">Pay once. <span>Own forever.</span></p></div>
<div class="rv d2"><div class="chart"><svg viewBox="0 0 520 320" role="img" aria-label="Illustration: rented tools cost more every year; a one-time purchase stays flat.">
<g stroke="#ffffff1f"><line x1="46" y1="270" x2="500" y2="270"/><line x1="46" y1="200" x2="500" y2="200" stroke-dasharray="3 6"/><line x1="46" y1="130" x2="500" y2="130" stroke-dasharray="3 6"/></g>
<path d="M46 250 L110 250 L110 228 L190 228 L190 190 L270 190 L270 150 L350 150 L350 108 L430 108 L430 66 L492 66" stroke="#ff7a52" stroke-width="4" fill="none"/>
<path d="M46 250 L56 250 L56 214 L492 214" stroke="#fff" stroke-width="4" fill="none"/><circle cx="56" cy="214" r="6" fill="#fff"/>
<text x="496" y="52" text-anchor="end" fill="#ffab8e" font-weight="700" font-size="15">Rented tools: the bill never stops</text>
<text x="496" y="240" text-anchor="end" fill="#fff" font-weight="700" font-size="15">OpenFrame: you paid once</text>
<g text-anchor="middle"><text x="110" y="292">Year 1</text><text x="190" y="292">Year 2</text><text x="270" y="292">Year 3</text><text x="350" y="292">Year 4</text><text x="430" y="292">Year 5</text></g></svg>
<p class="cap">Illustration of the idea, not real prices.</p></div></div></div></section>
<section class="sec-s"><div class="wrap narrow"><div class="sec-head"><span class="eyebrow">Have trust issues? Good.</span><h2>Try it free for 7 days.</h2></div>
${FAQ.find(([g]) => g === 'Pricing and trial')[1].map(faqItem)}</div></section>
${cta()}`,
  });

  // ---------- FAQ ----------
  const faq = page({
    path: '/faq/', nav: 'faq',
    title: 'Questions and answers | OpenFrame Studio',
    description: 'Answers about OpenFrame Studio: platforms, privacy and data, OpenFrame AI, files and formats, the price and the 7-day free trial.',
    breadcrumbs: [['Questions', '/faq/']],
    jsonld: [{ '@context': 'https://schema.org', '@type': 'FAQPage', mainEntity: FAQ.flatMap(([, items]) => items.map(([q, a]) => ({ '@type': 'Question', name: q, acceptedAnswer: { '@type': 'Answer', text: a.replace(/\[([^\]]+)\]\([^)]+\)/g, '$1') } }))) }],
    body: html`
${phero('FAQ', 'Questions and answers.', 'Short, plain answers about the app, your data, OpenFrame AI, files, the price and the trial.', html`<div class="fchips">${FAQ.map(([g]) => html`<a class="fchip" href="#g-${slug(g)}">${g}</a>`)}</div>`)}
<section class="sec-s pt30"><div class="wrap narrow">
${FAQ.map(([g, items]) => html`<div class="faq-group" id="g-${slug(g)}"><h2>${g}</h2>${items.map(faqItem)}</div>`)}
<p class="small-print">Not answered here? See <a href="/contact/">Contact</a> or the <a href="/tutorials/">tutorials</a>.</p>
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
${phero('Easy migration', 'Coming from other software? Bring your scripts.', 'See exactly what will be imported, and every warning, before anything is applied. Nothing is silently flattened or lost.')}
<section class="sec-s pt30"><div class="wrap"><div class="frow">
<div class="ftxt"><h2>What you can import</h2>
<div class="tbl-wrap mt18"><table class="tbl"><tbody>${IMPORT_OK.map(([f, d]) => html`<tr><td><b>${f}</b></td><td>${d}</td></tr>`)}</tbody></table></div>
<p>PDF import is best effort. A PDF has no real screenplay structure inside it, so the app guesses the elements. Read the preview before you accept.</p></div>
<div>${osBlocks({ win: mock('win', 'sp-import-preview', { caption: 'The import preview on Windows' }), mac: mock('mac', 'screenplay-tools-import-screenplay', { caption: 'Import Screenplay on the Mac' }) })}</div>
</div></div></section>
<section class="tint sec-s"><div class="wrap narrow">
<h2>What is refused</h2><p class="lead mt18">These formats are not opened. The app says so with a clear message instead of guessing.</p>
<div class="fmts">${IMPORT_REFUSED.map((f) => html`<span>${f}</span>`)}</div>
<p class="mt26 muted">If your script is in one of these, save or export it as Final Draft, Fountain, Word or plain text from the program that made it, then import that file.</p>
<h2 class="mt84">What you can export</h2>
<div class="tbl-wrap mt18"><table class="tbl"><tbody>${EXPORTS.map(([d, f]) => html`<tr><td><b>${d}</b></td><td>${f}</td></tr>`)}</tbody></table></div>
<p class="mt26 muted">Screenplay files carry no OpenFrame branding. Production PDFs carry a small "Created using OpenFrame" line. CSV files stay clean.</p>
</div></section>
<section class="sec-s"><div class="wrap narrow"><div class="facts"><h3>Writing in your own language</h3><ul class="ticks">
<li><span>Type, save, search and export Hindi, Telugu, Tamil, Kannada and Malayalam text. The screenplay PDF embeds fonts for them.</span></li>
<li><span>Other scripts such as Chinese, Japanese, Korean and Arabic can be typed and exported to Final Draft, Fountain and Word. The screenplay PDF refuses them with a clear message.</span></li>
</ul></div></div></section>
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
    title: 'One app or five tools | OpenFrame Studio',
    description: 'An honest look at using OpenFrame Studio as one app for ideas, script and production planning versus several separate tools, with its limits stated.',
    breadcrumbs: [['Compare', '/compare/']],
    body: html`
${phero('One app vs five', 'What you stop paying for.', 'Many productions run on a notes app, a screenwriting program, spreadsheets and documents. Here is the same job done both ways, and where one app is the wrong choice.')}
<section class="sec-s pt30"><div class="wrap"><div class="tbl-wrap"><table class="tbl">
<thead><tr><th scope="col">Job</th><th scope="col">With separate tools</th><th scope="col" class="us">In OpenFrame Studio</th></tr></thead>
<tbody>${rows.map(([j, a, b]) => html`<tr><td><b>${j}</b></td><td class="no">${a}</td><td class="us">${b}</td></tr>`)}</tbody></table></div></div></section>
<section class="tint sec-s"><div class="wrap"><div class="grid2">
<div class="card"><h3>What one app gives you</h3><ul class="ticks">
<li><span>Change a scene once and the breakdown, the schedule and the call sheet know about it.</span></li>
<li><span>No copying between files, and no version of the truth hiding in an old spreadsheet.</span></li>
<li><span>One place to back up and one project to send.</span></li>
<li><span>Everything stays on your computer, with no OpenFrame account.</span></li></ul></div>
<div class="card"><h3>Where separate tools may suit you better</h3><ul class="ticks">
<li><span>You need several people editing the same document at the same moment. OpenFrame has no live collaboration; people pass files instead.</span></li>
<li><span>You want your work on a phone or in a browser. OpenFrame is a desktop app for Windows 11 and Apple silicon Macs.</span></li>
<li><span>You need accounting or payroll. The budget is a light planning sheet.</span></li></ul></div>
</div></div></section>
${cta()}`,
  });

  // ---------- Local and private ----------
  const local = page({
    path: '/local-and-private/',
    title: 'Local and private | OpenFrame Studio',
    description: 'OpenFrame Studio keeps your work on your computer: no OpenFrame account, no telemetry, no cloud service. The few network uses are listed here.',
    breadcrumbs: [['Local and private', '/local-and-private/']],
    body: html`
${phero('Completely local', 'Your privacy stays on your computer.', 'Your scripts and plans stay on your computer. This page says exactly what the app does, and does not do, with your data and your network.')}
<section class="sec-s pt30"><div class="wrap split">
<div><h2>Where your work lives</h2><p class="lead mt18">Each project is one folder named <b>Title.openframe</b>, by default in Documents, in OpenFrame. You can copy, move or back up that folder like any other.</p>
<ul class="ticks"><li><span><b>No OpenFrame account</b> and no sign-in.</span></li><li><span><b>No telemetry</b>, no analytics and no crash upload.</span></li><li><span><b>No network service</b> that other computers can reach.</span></li><li><span><b>Not encrypted by the app.</b> Use your operating system's disk encryption if you need that protection.</span></li></ul></div>
<div class="facts"><h3>The only times the app uses the network</h3><ul class="ticks">
<li><span><b>The store</b> installs and updates the app and handles the trial and the purchase.</span></li>
<li><span><b>OpenFrame AI download you start.</b> About 1.2 GB, checked before use. You can also import the files by hand.</span></li>
<li><span><b>A saved link in the Idea Vault.</b> When you click it, it opens in your normal browser.</span></li></ul></div>
</div></section>
<section class="tint sec-s"><div class="wrap narrow"><h2>Backups, packages and diagnostics</h2><ul class="ticks">
<li><span>Create a backup (.ofbackup) or export a full project (.ofproject) to move a project to another computer.</span></li>
<li><span>Share a screenplay or Beat Board for review as a package file. The reply comes back as a file, and you preview its changes before applying them.</span></li>
<li><span>A diagnostics bundle exists, but only you can create it, and you can read it before you decide to share it.</span></li>
<li><span>Network (UNC) paths are refused as project locations, and a folder inside a cloud-sync service gets a warning, because two programs writing one project can damage it.</span></li></ul>
<p class="small-print">The website has its own notice: <a href="/legal/privacy/">website privacy notice</a>.</p></div></section>
${cta()}`,
  });

  // ---------- Themes ----------
  const themes = page({
    path: '/themes/',
    title: 'Themes | OpenFrame Studio',
    description: 'OpenFrame Studio has 17 built-in themes in Light, Dark or System, plus themes you paste in. Your screenplay prints in the standard look either way.',
    breadcrumbs: [['Themes', '/themes/']],
    body: html`
${phero('Themes', 'Do not like the colours? Make them yours.', 'Choose how the app looks in Light, Dark or System. Your documents keep the standard look.')}
<section class="sec-s pt30"><div class="wrap"><div class="frow">
<div class="ftxt"><h2>17 built-in themes</h2><div class="pill-list">${THEMES.map((t) => html`<span>${t}</span>`)}</div>
<p>Each theme works in Light or Dark, or follows your system with System. High Contrast is there for people who need stronger separation.</p>
<h2 class="mt26">Your own themes</h2><p>Paste a theme file. The app checks it, shows what it found, and keeps it under My Themes.</p></div>
<div>${osBlocks({ win: mock('win', 'settings-appearance', { caption: 'Settings, Appearance, on Windows' }), mac: mock('mac', 'settings-appearance', { caption: 'Settings, Appearance, on the Mac' }) })}</div>
</div></div></section>
${cta()}`,
  });

  // ---------- About ----------
  const about = page({
    path: '/about/', nav: 'about',
    title: 'About OpenFrame Studio and its founders',
    description: 'Why OpenFrame Studio exists, who it is for, the promises it keeps about your work, and the two founding engineers who build it.',
    breadcrumbs: [['About', '/about/']],
    jsonld: [ORG(ctx)],
    body: html`
${phero('About OpenFrame', 'From film lovers, to film lovers across the world.', 'OpenFrame Studio is one app for the whole film, made for people who make films with limited resources.')}
<section class="tint sec-s"><div class="wrap"><div class="two">
<div class="rv"><h2>What it is for</h2><p class="lead mt18">A film often starts as scattered notes and ends as a pile of separate files. OpenFrame Studio keeps the whole path in one project: first idea, story, screenplay, breakdown, schedule, call sheets and a light budget.</p></div>
<div class="rv d1"><h2>Who it is for</h2><p class="lead mt18">Writers, screenwriters and production teams, especially people who make films with limited resources. There is one app for everyone and no separate editions.</p></div>
</div></div></section>
<section class="sec-s"><div class="wrap"><div class="sec-head"><span class="eyebrow">The founders</span><h2>Two founding engineers.</h2>
<p class="lead">One turns the film-making journey into a product. The other turns that product into software you can trust with your work.</p></div>
<div class="founders">${FOUNDERS.map((f, i) => html`<article class="founder rv${i ? ' d1' : ''}"><span class="role">${f.role}</span><h3>${f.link ? html`<a class="name-link" href="${f.link}" target="_blank" rel="noopener noreferrer">${f.name}</a>` : f.name}</h3>${f.text.map((p) => html`<p>${p}</p>`)}</article>`)}
</div></div></section>
<section class="tint sec-s"><div class="wrap"><div class="sec-head"><h2>Your idea. Your film. Our promise.</h2></div><div class="grid3">
<div class="card rv"><h3>Your work is yours</h3><p>It lives on your computer. No OpenFrame account, no cloud service, no telemetry.</p></div>
<div class="card rv d1"><h3>You stay in charge</h3><p>OpenFrame AI only suggests. Nothing changes until you choose Apply.</p></div>
<div class="card rv d2"><h3>No lock-in</h3><p>Standard formats in, standard formats out. Back up and move your whole project any time.</p></div>
</div></div></section>
${cta()}`,
  });

  // ---------- Contact ----------
  const contact = page({
    path: '/contact/',
    title: 'Contact | OpenFrame Studio',
    description: 'How to reach the team behind OpenFrame Studio, and where to find answers first: the step-by-step tutorials, the features and the FAQ.',
    breadcrumbs: [['Contact', '/contact/']],
    body: html`
${phero('Contact us', 'Questions? Start here.', 'Most answers are already in the tutorials and the questions and answers.')}
<section class="sec-s pt30"><div class="wrap"><div class="contact">
<div><div class="grid2">
<a class="card linkcard" href="/tutorials/" data-os-href-win="/windows/tutorials/" data-os-href-mac="/mac/tutorials/"><h3>Tutorials</h3><p>Step-by-step tasks on the real screens.</p></a>
<a class="card linkcard" href="/features/"><h3>Features</h3><p>Every capability, for Windows and Mac.</p></a>
<a class="card linkcard" href="/faq/"><h3>Questions and answers</h3><p>Platforms, privacy, files and the trial.</p></a>
<a class="card linkcard" href="/pricing/"><h3>Pricing</h3><p>One payment after a 7-day trial.</p></a>
</div></div>
<aside class="cinfo"><h3>Store purchases</h3><p>Purchases, refunds and receipts are handled by the Microsoft Store and the Mac App Store.</p><h3>This website</h3><p>The site has no contact form and collects nothing.</p></aside>
</div></div></section>`,
  });

  // ---------- Legal ----------
  const legalHead = () => html`<p class="upd">Last updated ${UPDATED}.</p>`;
  const privacy = page({
    path: '/legal/privacy/',
    title: 'Website privacy notice | OpenFrame Studio',
    description: 'How the OpenFrame Studio website treats visitor data: no cookies, no analytics and no forms. Cloudflare serves the pages and may keep standard request logs.',
    breadcrumbs: [['Privacy notice', '/legal/privacy/']],
    body: html`
${phero('Privacy Policy', 'Website privacy notice.', 'This notice covers this website. The app’s own behaviour is described on [Local and private](/local-and-private/).')}
<section class="sec-s pt30"><div class="wrap legal">${legalHead()}
<h2>What the site does not do</h2><ul><li>It sets no cookies.</li><li>It runs no analytics and no advertising or tracking scripts.</li><li>It has no forms, no accounts and no email sign-up, so it collects no personal details from you.</li><li>It does not load scripts, fonts or images from other companies.</li></ul>
<h2>What happens when you visit</h2><p>The pages are served by Cloudflare, which hosts the site. Like any web host, Cloudflare receives your request and may keep standard request logs, such as your IP address, the page asked for, the time and your browser type. These logs are handled by Cloudflare under its own policies.</p>
<h2>Choices kept in your browser</h2><p>The platform, the website theme, the app screen theme and the price region you pick are kept only in your own browser storage and are never sent to us. Clearing your browser data removes them.</p>
<h2>The app</h2><p>OpenFrame Studio has no OpenFrame account and no telemetry. Purchases happen in the Microsoft Store or the Mac App Store, under the store’s own privacy terms.</p>
<h2>Links</h2><p>If a link takes you to another site, that site has its own policy.</p></div></section>`,
  });
  const terms = page({
    path: '/legal/terms/',
    title: 'Website terms of use | OpenFrame Studio',
    description: 'Terms for using the OpenFrame Studio website. Buying the app, its price, refunds and its licence follow the Microsoft Store and Mac App Store terms.',
    breadcrumbs: [['Terms', '/legal/terms/']],
    body: html`
${phero('Terms of Use', 'Website terms of use.', 'These terms cover this website. Buying and using the app follows the terms shown in the store.')}
<section class="sec-s pt30"><div class="wrap legal">${legalHead()}
<h2>Using this website</h2><ul><li>You may read this site and link to it.</li><li>The site describes OpenFrame Studio and its features as accurately as we can. The app changes over time, so screens and features can differ from what you see here.</li><li>The site is provided as it is, without any promise that it is complete or always available.</li><li>Please do not copy the site’s text or screens and present them as your own, or use the OpenFrame Studio name in a way that suggests we endorse you.</li></ul>
<h2>Buying the app</h2><p>OpenFrame Studio is sold through the Microsoft Store and the Mac App Store. The price, taxes, payment, refunds and the licence to use the app are handled by the store under its own terms. Prices on this site are ₹2,999 in India and US$89 elsewhere; the store shows the final amount.</p>
<h2>Other parties</h2><p>Other product names mentioned on this site belong to their owners. Microsoft and Microsoft Store are trademarks of the Microsoft group of companies. Apple, Mac, macOS and Mac App Store are trademarks of Apple Inc.</p></div></section>`,
  });

  return [platform('win'), platform('mac'), download, pricing, faq, migrate, compare, local, themes, about, contact, privacy, terms];
}
