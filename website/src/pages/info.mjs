// Information pages in the approved website design: platform pages, download, pricing, FAQ, switching, compare,
// privacy, themes and about. Legal and contact pages live in legal.mjs. Truth source: research/facts.json and
// research/capability-matrix.md. Do not add claims they do not show.
import { FOUNDERS } from '../content/founders.mjs';
import { themesBody } from '../lib/themes-page.mjs';
import { SHORTCUTS, SHORTCUT_NOTE, IMPORT_OK, IMPORT_REFUSED, EXPORTS, THEMES, FAQ } from '../content/copy.mjs';

// Structured data for search and answer engines lives in src/lib/seo.mjs (organization, app with prices, website).
import { ORG, APP, WEBSITE } from '../lib/seo.mjs';
export { ORG, APP, WEBSITE };
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
  const shortcutTable = (idx) => html`<div class="tbl-wrap fit"><table class="tbl"><thead><tr><th scope="col">Action</th><th scope="col">Shortcut</th></tr></thead>
<tbody>${SHORTCUTS.filter((r) => r[idx]).map((r) => html`<tr><td><b>${r[0]}</b></td><td>${keys(r[idx])}</td></tr>`)}</tbody></table></div>
<p class="small-print">${SHORTCUT_NOTE}</p>`;
  // Each answer has a stable id from its question (q-<words>, at most 60 characters), so a link can open it exactly.
  const faqId = (q) => 'q-' + slug(q).replace(/^-+|-+$/g, '').slice(0, 60).replace(/-+$/, '');
  const faqItem = ([q, a]) => html`<details class="faq-item" id="${faqId(q)}"><summary>${q}</summary><div class="a"><p>${md(a)}</p></div></details>`;

  // ---------- Platform pages ----------
  const platform = (os) => {
    const win = os === 'win';
    const name = win ? 'Windows' : 'Mac';
    return page({
      path: win ? '/windows/' : '/mac/', os, alternates: { win: '/windows/', mac: '/mac/' }, nav: 'download',
      title: `Straft Studio for ${name}`,
      description: win
        ? 'Straft Studio for Windows 11: what it needs, how it installs from the Microsoft Store, the keyboard shortcuts and the step-by-step Windows tutorials.'
        : 'Straft Studio for Mac on Apple silicon: what it needs, the Mac App Store, Mac keyboard shortcuts, how it differs, and the step-by-step Mac tutorials.',
      breadcrumbs: [[name, win ? '/windows/' : '/mac/']],
      jsonld: [APP(ctx, win ? 'Windows 11' : 'macOS 14 or later (Apple silicon)', win ? '/windows/' : '/mac/')],
      body: html`
${phero(win ? 'Windows 11' : 'macOS on Apple silicon', `Straft Studio for ${name}.`, win
        ? 'Write, plan and schedule a film in one project on your own computer, in an app made for Windows 11.'
        : 'The same project and the same engine, in a native Mac app with the Mac menu bar, sheets and Command shortcuts.',
      html`<div class="hero-cta">${storeBtn(os, 'btn-lg')}<a class="btn btn-ghost btn-lg" href="${win ? '/windows/tutorials/' : '/mac/tutorials/'}">${name} tutorials</a></div>
<div class="tour-real">${mock(os, win ? 'app-home-empty' : 'home-empty', { caption: `Application Home on ${name}, before your first project.`, eager: true })}</div>`)}
<section class="tint sec-s"><div class="wrap narrow"><div class="sec-head"><h2>What you need</h2></div>
<div class="tbl-wrap fit"><table class="tbl"><tbody>
<tr><td><b>System</b></td><td>${win ? 'Windows 11 on a 64-bit computer. Windows 10 is not supported.' : 'A Mac with Apple silicon, running macOS 14 or later. Intel Macs are not supported.'}</td></tr>
${win ? html`<tr><td><b>Window size</b></td><td>At least 1200 by 720 pixels.</td></tr>` : ''}
<tr><td><b>Account and internet</b></td><td>No Straft account. The app works offline. The internet is used for the store and for Straft AI download that you start.</td></tr>
<tr><td><b>Straft AI</b></td><td>A one-time download of about 1.2 GB. Its chat model needs at least 4 GB of memory, 8 GB recommended.</td></tr>
<tr><td><b>Price</b></td><td>7-day free trial, then one payment of ${price()}.</td></tr>
</tbody></table></div></div></section>
<section class="sec-s" id="install"><div class="wrap"><div class="sec-head"><h2>Get started in four steps</h2></div>
<div class="steps3 four">
<div class="step"><div class="n">1</div><h3>Install</h3><p>${win ? 'Get Straft Studio from the Microsoft Store. The Store installs and updates it.' : 'Get Straft Studio from the Mac App Store. The App Store installs and updates it.'}</p></div>
<div class="step"><div class="n">2</div><h3>Choose where projects live</h3><p>The first start asks where to keep your projects. The suggested place is fine for most people.</p></div>
<div class="step"><div class="n">3</div><h3>Say what to call you</h3><p>Your name appears on notes, tasks and activity. It stays on this computer. There is no account.</p></div>
<div class="step"><div class="n">4</div><h3>Create a project</h3><p>Create Project asks for a title. Everything else is optional and can be changed later.</p></div>
</div></div></section>
<section class="tint sec-s"><div class="wrap narrow"><div class="sec-head"><h2>Keyboard shortcuts on ${name}</h2><p class="lead">The defaults. You can change any of them in Settings.</p></div>
${shortcutTable(win ? 1 : 2)}</div></section>
<section class="sec-s"><div class="wrap narrow"><div class="facts"><h3>Good to know on ${name}</h3><ul class="ticks">
${win ? html`<li><span>Your project is a folder named <b>Title.straft</b>, by default under Documents, in Straft. Network folders are refused, and a folder inside a cloud-sync service gets a warning.</span></li>
<li><span>The left rail holds Home, Idea Vault, Story, Screenplay, Breakdown, Production, Call Sheets and Files. The project menu holds sharing, backups and project tools.</span></li>
<li><span>Spell check uses the dictionaries installed in Windows, with a main and a second language.</span></li>`
        : html`<li><span>It uses the Mac menu bar: File, Edit, View, Go, Window and Help. Import, export, print and Close Project live in the File menu.</span></li>
<li><span>Dialogs appear as sheets attached to the window. Settings opens with Command and comma.</span></li>
<li><span>Project undo and redo use Command, Option and Z. Plain Command and Z undoes text.</span></li>
<li><span>Search and Commands opens with Command and K. Spell check uses the macOS spelling service.</span></li>`}
</ul></div></div></section>
${cta()}`,
    });
  };

  // ---------- Download ----------
  const download = page({
    path: '/download/', nav: 'download',
    title: 'Download Straft Studio for Windows and Mac',
    description: 'Get Straft Studio from the Microsoft Store for Windows 11 or the Mac App Store for Apple silicon Macs. 7-day free trial, then one payment.',
    breadcrumbs: [['Download', '/download/']],
    body: html`
${phero('Download', 'Get Straft Studio.', 'From the Microsoft Store on Windows and the Mac App Store on the Mac. Try it free for 7 days.')}
<section class="sec-s pt30"><div class="wrap"><div class="price-grid">
<div class="pcard win rv"><span class="badge">Windows 11</span><h2>Straft Studio for Windows</h2><p class="price-sub mt10">Windows 11, 64-bit. Installed and updated by the Microsoft Store.</p>${storeBtn('win', 'btn-lg')}
<p class="small-print"><a href="/windows/">More about Windows</a></p></div>
<div class="pcard mac rv d1"><span class="badge">Mac</span><h2>Straft Studio for Mac</h2><p class="price-sub mt10">macOS 14 or later on Apple silicon. Installed and updated by the Mac App Store.</p>${storeBtn('mac', 'btn-lg')}
<p class="small-print"><a href="/mac/">More about the Mac</a></p></div>
</div></div></section>
<section class="tint sec-s" id="trial"><div class="wrap"><div class="sec-head"><h2>Your 7-day free trial, step by step.</h2></div>
<div class="steps3">
<div class="step rv"><div class="n">1</div><h3>Install</h3><p>Get the app from the Microsoft Store or the Mac App Store. The 7-day trial starts with the whole app.</p></div>
<div class="step rv d1"><div class="n">2</div><h3>Make something real</h3><p>Write a scene, build a Beat Board, schedule a day, make a call sheet. A small chip shows the days left.</p></div>
<div class="step rv d2"><div class="n">3</div><h3>Decide</h3><p>Before the trial ends the app reminds you. Buy in the store to keep using the full app. Your project folders stay on your computer either way.</p></div>
</div></div></section>`,
  });

  // ---------- Pricing ----------
  const pricing = page({
    path: '/pricing/', nav: 'pricing',
    title: 'Pricing: pay once, own forever | Straft Studio',
    description: 'Straft Studio is a one-time lifetime purchase. Launch price: ₹3,999 in India, €89 in Europe and US$89 elsewhere, after a 7-day free trial.',
    breadcrumbs: [['Pricing', '/pricing/']],
    body: html`
${phero('Pricing', 'Pay once. Own forever.', 'A lifetime purchase with no subscription. One payment for the whole app, after a 7-day free trial.')}
<section class="sec-s pt30"><div class="wrap"><div class="price-one">
<div class="pcard win rv">
<div class="region-tg" role="group" aria-label="Price region"><button type="button" data-region-set="in" aria-pressed="false">India</button><button type="button" data-region-set="eu" aria-pressed="false">Europe</button><button type="button" data-region-set="world" aria-pressed="false">USA and others</button></div>
<h2>Straft Studio</h2>
<div class="price">${price()}</div>
<div class="store-col">${storeBtn('win')}${storeBtn('mac')}</div></div>
<p class="price-note center">Bought separately for Windows and for Mac, each in its own store. The store sets the final checkout amount under its own rules, which can include taxes where you live. The website does not convert currencies.</p>
</div></div></section>
<section class="sec-s"><div class="wrap narrow"><div class="sec-head"><h2>Questions about the price and the trial</h2></div>
${FAQ.find(([g]) => g === 'Pricing and trial')[1].map(faqItem)}</div></section>
${cta()}`,
  });

  // ---------- FAQ ----------
  const faq = page({
    path: '/faq/', nav: 'faq',
    title: 'Questions and answers | Straft Studio',
    description: 'Answers about Straft Studio: platforms, privacy and data, Straft AI, files and formats, the price and the 7-day free trial.',
    breadcrumbs: [['Questions', '/faq/']],
    jsonld: [{ '@context': 'https://schema.org', '@type': 'FAQPage', mainEntity: FAQ.flatMap(([, items]) => items.map(([q, a]) => ({ '@type': 'Question', name: q, acceptedAnswer: { '@type': 'Answer', text: a.replace(/\[([^\]]+)\]\([^)]+\)/g, '$1') } }))) }],
    body: html`
${phero('FAQ', 'Questions and answers.', 'Short, plain answers about the app, your data, Straft AI, files, the price and the trial.', html`<div class="fchips">${FAQ.map(([g]) => html`<a class="fchip" href="#g-${slug(g)}">${g}</a>`)}</div>`)}
<section class="sec-s pt30"><div class="wrap narrow">
${FAQ.map(([g, items]) => html`<div class="faq-group" id="g-${slug(g)}"><h2>${g}</h2>${items.map(faqItem)}</div>`)}
<p class="small-print">Not answered here? See <a href="/contact/">Contact</a> or the <a href="/tutorials/">tutorials</a>.</p>
</div></section>
${cta()}`,
  });

  // ---------- Switch ----------
  const migrate = page({
    path: '/switch/',
    title: 'Switch to Straft Studio | Import and export',
    description: 'Bring a screenplay into Straft Studio from Final Draft, Fountain, Word, PDF or text, see a preview first, and export it back out any time.',
    breadcrumbs: [['Switch', '/switch/']],
    body: html`
${phero('Easy migration', 'Coming from other software? Bring your scripts.', 'See exactly what will be imported, and every warning, before anything is applied. Nothing is silently flattened or lost.')}
<section class="sec-s pt30"><div class="wrap"><div class="frow">
<div class="ftxt"><h2>What you can import</h2>
<div class="tbl-wrap fit mt18"><table class="tbl"><tbody>${IMPORT_OK.map(([f, d]) => html`<tr><td><b>${f}</b></td><td>${d}</td></tr>`)}</tbody></table></div>
<p>PDF import is best effort. A PDF has no real screenplay structure inside it, so the app guesses the elements. Read the preview before you accept.</p></div>
<div>${osBlocks({ win: mock('win', 'sp-import-preview', { caption: 'The import preview on Windows' }), mac: mock('mac', 'screenplay-tools-import-screenplay', { caption: 'Import Screenplay on the Mac' }) })}</div>
</div></div></section>
<section class="tint sec-s"><div class="wrap narrow">
<h2>What is refused</h2><p class="lead mt18">These formats are not opened. The app says so with a clear message instead of guessing.</p>
<div class="fmts">${IMPORT_REFUSED.map((f) => html`<span>${f}</span>`)}</div>
<p class="mt26 muted">If your script is in one of these, save or export it as Final Draft, Fountain, Word or plain text from the program that made it, then import that file.</p>
<h2 class="mt84">What you can export</h2>
<div class="tbl-wrap fit mt18"><table class="tbl"><tbody>${EXPORTS.map(([d, f]) => html`<tr><td><b>${d}</b></td><td>${f}</td></tr>`)}</tbody></table></div>
<p class="mt26 muted">Screenplay files carry no Straft branding. Production PDFs carry a small "Created using Straft" line. CSV files stay clean.</p>
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
    title: 'One app or five tools | Straft Studio',
    description: 'An honest look at using Straft Studio as one app for ideas, script and production planning versus several separate tools, with its limits stated.',
    breadcrumbs: [['Compare', '/compare/']],
    body: html`
${phero('One app vs five', 'What you stop paying for.', 'Many productions run on a notes app, a screenwriting program, spreadsheets and documents. Here is the same job done both ways, and where one app is the wrong choice.')}
<section class="sec-s pt30"><div class="wrap"><div class="tbl-wrap" tabindex="0" role="region" aria-label="Separate tools compared with Straft Studio"><table class="tbl">
<thead><tr><th scope="col">Job</th><th scope="col">With separate tools</th><th scope="col" class="us">In Straft Studio</th></tr></thead>
<tbody>${rows.map(([j, a, b]) => html`<tr><td><b>${j}</b></td><td class="no">${a}</td><td class="us">${b}</td></tr>`)}</tbody></table></div></div></section>
<section class="tint sec-s"><div class="wrap"><div class="grid2">
<div class="card"><h2 class="t3">What one app gives you</h2><ul class="ticks">
<li><span>Change a scene once and the breakdown, the schedule and the call sheet know about it.</span></li>
<li><span>No copying between files, and no version of the truth hiding in an old spreadsheet.</span></li>
<li><span>One place to back up and one project to send.</span></li>
<li><span>Everything stays on your computer, with no Straft account.</span></li></ul></div>
<div class="card"><h2 class="t3">Where separate tools may suit you better</h2><ul class="ticks">
<li><span>You need several people editing the same document at the same moment. Straft has no live collaboration; people pass files instead.</span></li>
<li><span>You want your work on a phone or in a browser. Straft is a desktop app for Windows 11 and Apple silicon Macs.</span></li>
<li><span>You need accounting or payroll. The budget is a light planning sheet.</span></li></ul></div>
</div></div></section>
${cta()}`,
  });

  // ---------- Local and private ----------
  const local = page({
    path: '/local-and-private/',
    title: 'Local and private | Straft Studio',
    description: 'Straft Studio keeps your work on your computer: no Straft account, no telemetry, no cloud service. The few network uses are listed here.',
    breadcrumbs: [['Local and private', '/local-and-private/']],
    body: html`
${phero('Completely local', 'Your privacy stays on your computer.', 'Your scripts and plans stay on your computer. This page says exactly what the app does, and does not do, with your data and your network.')}
<section class="sec-s pt30"><div class="wrap split">
<div><h2>Where your work lives</h2><p class="lead mt18">Each project is one folder named <b>Title.straft</b>, by default in Documents, in Straft. You can copy, move or back up that folder like any other.</p>
<ul class="ticks"><li><span><b>No Straft account</b> and no sign-in.</span></li><li><span><b>No telemetry</b>, no analytics and no crash upload.</span></li><li><span><b>No network service</b> that other computers can reach.</span></li><li><span><b>Not encrypted by the app.</b> Use your operating system's disk encryption if you need that protection.</span></li></ul></div>
<div class="facts"><h3>The only times the app uses the network</h3><ul class="ticks">
<li><span><b>The store</b> installs and updates the app and handles the trial and the purchase.</span></li>
<li><span><b>Straft AI download you start.</b> About 1.2 GB, checked before use. You can also import the files by hand.</span></li>
<li><span><b>A saved link in the Idea Vault.</b> When you click it, it opens in your normal browser.</span></li></ul></div>
</div></section>
<section class="tint sec-s"><div class="wrap narrow"><h2>Backups, packages and diagnostics</h2><ul class="ticks">
<li><span>Create a backup (.ofbackup) or export a full project (.ofproject) to move a project to another computer.</span></li>
<li><span>Share a screenplay or Beat Board for review as a package file. The reply comes back as a file, and you preview its changes before applying them.</span></li>
<li><span>A diagnostics bundle exists, but only you can create it, and you can read it before you decide to share it.</span></li>
<li><span>Network folders are refused as project locations, and a folder inside a cloud-sync service gets a warning, because two programs writing one project can damage it.</span></li></ul>
<p class="small-print">The website has its own notice: <a href="/legal/privacy/">website privacy notice</a>.</p></div></section>
${cta()}`,
  });

  // ---------- Themes ----------
  const themes = page({
    path: '/themes/',
    title: 'Themes | Straft Studio',
    description: 'Make your own Straft Studio theme as a small .json file, import it on Windows or the Mac, see the check, and share it. Starter themes to download.',
    breadcrumbs: [['Themes', '/themes/']],
    body: themesBody({ ...ctx, THEMES }),
  });

  // ---------- About ----------
  const about = page({
    path: '/about/', nav: 'about',
    title: 'About Straft Studio and its founders',
    description: 'Why Straft Studio exists, who it is for, and the co-founding engineers who lead its technical and product development with their teams.',
    breadcrumbs: [['About', '/about/']],
    jsonld: [ORG(ctx)],
    body: html`
${phero('About Straft', 'From film lovers, to film lovers across the world.', 'Straft Studio is one app for the whole film.')}
<section class="tint sec-s"><div class="wrap"><div class="two">
<div class="rv"><h2>What it is for</h2><p class="lead mt18">A film often starts as scattered notes and ends as a pile of separate files. Straft Studio keeps the whole path in one project: first idea, story, screenplay, breakdown, schedule, call sheets and a light budget.</p></div>
<div class="rv d1"><h2>Who it is for</h2><p class="lead mt18">Writers, screenwriters and production teams, especially people who make films with limited resources. There is one app for everyone and no separate editions.</p></div>
</div></div></section>
<section class="sec-s"><div class="wrap"><div class="sec-head"><span class="eyebrow">The founders</span><h2>Co-founding engineers and their teams.</h2>
<p class="lead">Technical and product development, each led by a co-founding engineer with a team.</p></div>
<div class="founders">${FOUNDERS.map((f, i) => html`<article class="founder rv${i ? ' d1' : ''}"><span class="role">${f.role}</span><h3>${f.link ? html`<a class="name-link" href="${f.link}" target="_blank" rel="noopener noreferrer">${f.name}</a>` : f.name} and team</h3>${f.text.map((p) => html`<p>${p}</p>`)}</article>`)}
</div></div></section>
${cta()}`,
  });

  return [platform('win'), platform('mac'), download, pricing, faq, migrate, compare, local, themes, about];
}
