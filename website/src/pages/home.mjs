// Home page (/). Layout language: warm cream page, one oversized rounded gradient hero with a tabbed product card, a
// four-card showcase, a scroll-driven product story, alternating feature sections, a promise carousel, a tile grid of
// file formats, a large closing call to action and a rounded dark footer. Screens are the real captured app screens of
// the selected platform; claims follow research/capability-matrix.md. No testimonials, metrics or integrations are
// invented: where a layout normally holds those, it holds OpenFrame's own promises and real file formats.
import { APP, ORG, WEBSITE } from './info.mjs';

const IC = {
  local: '<rect x="3" y="4" width="18" height="12" rx="2"/><path d="M8 20h8M12 16v4"/>',
  once: '<circle cx="12" cy="12" r="9"/><path d="M8.5 12.5l2.5 2.5 4.5-5"/>',
  five: '<rect x="3" y="3" width="7" height="7" rx="1.5"/><rect x="14" y="3" width="7" height="7" rx="1.5"/><rect x="3" y="14" width="7" height="7" rx="1.5"/><rect x="14" y="14" width="7" height="7" rx="1.5"/>',
  lock: '<rect x="4.5" y="10.5" width="15" height="10" rx="2"/><path d="M8 10.5V7a4 4 0 0 1 8 0v3.5"/>',
  arrow: '<path d="M5 12h14M13 6l6 6-6 6"/>',
  up: '<path d="M7 17L17 7M9 7h8v8"/>',
  left: '<path d="M15 5l-7 7 7 7"/>',
  right: '<path d="M9 5l7 7-7 7"/>',
  offline: '<path d="M3 3l18 18M8.5 16.5a5 5 0 0 1 7 0M5 13a10 10 0 0 1 5-2.7M14.5 10.4A10 10 0 0 1 19 13M12 20h.01"/>',
  bulb: '<path d="M9 18h6M10 21h4M12 3a6 6 0 0 0-3.5 10.9c.6.5 1 1.2 1 2.1h5c0-.9.4-1.6 1-2.1A6 6 0 0 0 12 3z"/>',
  board: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M9 4v16M15 4v16"/>',
  page: '<path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/><path d="M14 3v5h5M9 13h6M9 17h6"/>',
  list: '<path d="M9 6h12M9 12h12M9 18h12M4 6h.01M4 12h.01M4 18h.01"/>',
  clip: '<rect x="5" y="4" width="14" height="17" rx="2"/><path d="M9 4V3h6v1M9 10h6M9 14h6"/>',
  x: '<path d="M6 6l12 12M18 6L6 18"/>',
  mega: '<path d="M3 11v2a1 1 0 0 0 1 1h2l5 4V6L6 10H4a1 1 0 0 0-1 1zM15 9a4 4 0 0 1 0 6"/>',
  pause: '<path d="M8 5v14M16 5v14"/>',
  play: '<path d="M8 5l11 7-11 7z"/>',
  tag: '<path d="M3 12V4a1 1 0 0 1 1-1h8l9 9-9 9z"/><circle cx="8" cy="8" r="1.4"/>',
  grid: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 10h18M3 15h18M9 4v16M15 4v16"/>',
  sun: '<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/>',
  coin: '<circle cx="12" cy="12" r="9"/><path d="M14.5 9.2c-.5-.8-1.5-1.2-2.5-1.2-1.4 0-2.5.8-2.5 2s1 1.6 2.5 2 2.5.8 2.5 2-1.1 2-2.5 2c-1 0-2-.4-2.5-1.2M12 6.5V8M12 16v1.5"/>',
  doc: '<path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/><path d="M14 3v5h5"/>',
  chat: '<path d="M4 5h16v11H9l-5 4z"/><path d="M8 9h8M8 12h5"/>',
  cal: '<rect x="3" y="5" width="18" height="16" rx="2"/><path d="M3 10h18M8 3v4M16 3v4"/>',
  user: '<circle cx="12" cy="8" r="4"/><path d="M4 21a8 8 0 0 1 16 0"/>',
  undo: '<path d="M9 14L4 9l5-5"/><path d="M4 9h10a6 6 0 0 1 0 12h-3"/>',
};

// Hero tabs: [chip icon, tab label, product label, headline, line, link, win screen, mac screen]
const TABS = [
  ['bulb', 'Idea Vault', 'Idea Vault', 'Catch every idea before it slips away.', 'Drop anything in: notes, images, links, voice notes and files.', '/features/idea-vault/', 'vault-grid-all-items', 'vault-filled'],
  ['page', 'Screenplay', 'Screenplay Editor', 'Write it properly, from the first line.', 'Standard screenplay formatting while you type.', '/features/screenplay/', 'sp-editor-standard', 'screenplay-editor-typing'],
  ['board', 'Shooting schedule', 'Shooting Schedule', 'Turn scenes into a shooting plan.', 'Scenes become strips on shooting days, with warnings before problems.', '/features/scheduling/', 'prod-schedule-board', 'schedule-board'],
  ['clip', 'Call sheets', 'Call Sheets', 'From shooting day to call sheet.', 'A call sheet filled from the shooting day, ready to finalise and export.', '/features/call-sheets/', 'cs-editor-draft', 'callsheet-editor'],
];

// Four-card showcase: [name, tone, line, bullets, win screen, mac screen, link]
const CARDS = [
  ['Ideate', 'a', 'Drop anything in: notes, images, links, voice notes and files.',
    [['bulb', 'Quick Capture keeps a new idea one shortcut away'], ['board', 'Sort ideas into acts, sequences and beats on the Beat Board'], ['grid', 'Collect visual references on moodboards']],
    'story-board-filled', 'story-board-filled', '/features/story-board/'],
  ['Write', 'b', 'Standard screenplay formatting while you type.',
    [['page', 'Import Final Draft, Fountain, Word, PDF or plain text'], ['doc', 'Compare drafts and lock the shooting script'], ['chat', 'Write in English, Telugu, Hindi, Tamil, Kannada or Malayalam']],
    'sp-import-preview', 'screenplay-tools-import-screenplay', '/features/screenplay/'],
  ['Plan', 'c', 'Scenes become strips on shooting days, with warnings before problems.',
    [['tag', 'Tag every prop, cast member and location in the breakdown'], ['grid', 'Plan every shot with shot lists and storyboards'], ['list', 'Schedule scenes on a stripboard']],
    'bd-main', 'breakdown-scenes', '/features/breakdown/'],
  ['Shoot', 'd', 'A call sheet filled from the shooting day, ready to finalise and export.',
    [['clip', 'Hand out sides: just the day’s pages'], ['cal', 'See the whole day in Daily View'], ['coin', 'Keep six reports and a light budget']],
    'prod-sides-day-4', 'schedule-sides', '/features/call-sheets/'],
];

// Scroll-driven product story: [icon, title, line, link, win screen, mac screen]
const STORY = [
  ['tag', 'Script breakdown', 'Tag cast, props, wardrobe and more in each scene. Suggestions help, and you accept or reject every one.', '/features/breakdown/', 'bd-main', 'breakdown-scenes'],
  ['grid', 'Day Out of Days', 'Built from the cast and the schedule, with a date range and room for manual corrections. Export as PDF, CSV or Excel.', '/features/scheduling/', 'prod-dood-table', 'schedule-dood'],
  ['cal', 'Daily View', 'Everything about one shooting day on a single page, ready to export.', '/features/sides-daily-view/', 'prod-daily-day-1', 'schedule-daily'],
  ['list', 'Reports', 'Six production reports drawn from the project, always current, with PDF, CSV and Excel export.', '/features/reports-budget/', 'prod-reports-scene', 'schedule-reports-scene'],
  ['coin', 'Budget', 'A light budget that compares what you planned with what you spent, with saved snapshots.', '/features/reports-budget/', 'prod-budget-filled', 'budget-table'],
];

const PROMISES = [
  ['01', 'We never silently change your work.', 'OpenFrame suggests. You decide. Nothing rewrites your story, script or schedule on its own.', 'chat'],
  ['02', 'Nothing is lost by accident.', 'Deleted items wait in Recently Deleted. Undo is offered after changes. Your work saves as you go.', 'undo'],
  ['03', 'Leave any time, taking everything.', 'Every document exports. Your project is yours to back up, move and keep.', 'arrow'],
];

const FILES = [['.fdx', 'Final Draft'], ['.fountain', 'Fountain'], ['.docx', 'Word'], ['.pdf', 'PDF'], ['.txt', 'Plain text'], ['.md', 'Markdown'], ['.csv', 'CSV'], ['.xlsx', 'Excel'], ['.ofproject', 'Project package'], ['.ofbackup', 'Backup package']];

export default function home(ctx) {
  const { html, raw, mock, storeBtn } = ctx;
  const svg = (k, cls = '') => raw(`<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"${cls ? ` class="${cls}"` : ''} fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">${IC[k]}</svg>`);
  // One real app screen, the Windows capture or the Mac capture depending on the platform control.
  const both = (win, mac, opts = {}) => html`<div data-os-show="win">${mock('win', win, { link: false, ...opts })}</div><div data-os-show="mac">${mock('mac', mac, { link: false, ...opts })}</div>`;

  const top = html`<div class="annc" id="annc" role="region" aria-label="Announcement"><div class="annc-in">${svg('mega')}<p>OpenFrame Studio is a one-time purchase, with a 7-day free trial on Windows 11 and Mac.</p><a class="annc-link" href="/pricing/">See pricing ${svg('arrow')}</a></div><button type="button" class="annc-x" data-annc-close aria-label="Dismiss announcement">${svg('x')}</button></div>`;

  const chip = ([ic, label], i) => html`<li role="presentation"><button type="button" class="hx-chip" role="tab" id="hx-tab-${i}" aria-controls="hx-pane-${i}" aria-selected="${String(i === 0)}" tabindex="${i ? -1 : 0}" aria-label="${label}"><i class="ring" aria-hidden="true"></i>${svg(ic)}</button></li>`;
  const pane = ([, , product, head, line, href, win, mac], i) => html`<div class="hx-pane" id="hx-pane-${i}" role="tabpanel" aria-labelledby="hx-tab-${i}"${i ? raw(' hidden') : ''}>
<div class="hx-copy"><span class="hx-tag">${svg(TABS[i][0])}${product}</span>
<h2 class="hx-h">${head}</h2><p>${line}</p><a class="ulink" href="${href}">Learn more ${svg('arrow')}</a></div>
<div class="hx-well">${both(win, mac, { eager: i === 0 })}</div></div>`;

  const card = ([name, tone, line, bullets, win, mac, href], i) => html`<article class="cc cc-${tone}${i === 0 ? ' is-on' : ''}" data-cc>
<h3><button type="button" class="cc-t" aria-expanded="${String(i === 0)}" aria-controls="cc-b-${i}">${name}</button></h3>
<div class="cc-b" id="cc-b-${i}"><p class="cc-line">${line}</p>
<ul class="cc-list">${bullets.map(([ic, t]) => html`<li>${svg(ic)}<span>${t}</span></li>`)}</ul>
<a class="ulink cc-more" href="${href}">Explore ${name.toLowerCase()} ${svg('arrow')}</a></div>
<div class="cc-art" aria-hidden="true"><div class="cc-shot">${both(win, mac)}</div></div></article>`;

  const storyItem = ([ic, t, line, href], i) => html`<li class="ss-item${i === 0 ? ' is-on' : ''}" data-ss-item="${i}">
<button type="button" class="ss-btn" aria-expanded="${String(i === 0)}" aria-controls="ss-d-${i}">${svg(ic)}<b>${t}</b></button>
<div class="ss-d" id="ss-d-${i}"><div class="ss-in"><p>${line}</p><a class="ss-go" href="${href}" aria-label="Open ${t}">${svg('up')}</a></div></div></li>`;
  const storyShot = ([, t, , , win, mac], i) => html`<div class="ss-shot${i === 0 ? ' is-on' : ''}" data-ss-shot="${i}"${i ? raw(' aria-hidden="true"') : ''}>${both(win, mac)}</div>`;

  const promise = ([n, t, line, ic], i) => html`<article class="pc" data-pc="${i}" aria-roledescription="slide" aria-label="${i + 1} of ${PROMISES.length}">
<div class="pc-copy"><span class="pc-n">Our promise ${n}</span><h3 class="pc-q">${t}</h3><p>${line}</p><span class="pc-chip">OpenFrame principle</span></div>
<div class="pc-art" aria-hidden="true"><b>${n}</b>${svg(ic)}</div>
<button type="button" class="pc-side" data-pc-go="${i}" tabindex="-1" aria-label="Show promise ${n}"><b>${n}</b></button></article>`;

  const body = html`
<section class="hx" aria-labelledby="hx-title"><div class="hx-panel" data-hx>
<div class="hx-blobs" aria-hidden="true"><i class="b1"></i><i class="b2"></i><i class="b3"></i></div>
<div class="hx-top">
<h1 id="hx-title"><span class="ln"><span>A film production</span></span> <span class="ln"><span>software for all</span></span> <span class="ln"><span>your needs.</span></span></h1>
<p class="hx-sub">From idea to final call sheet, in one app that lives on your computer. From film lovers, to film lovers across the world.</p>
<div class="hx-cta"><div class="store-row">${storeBtn('win', 'btn-lg')}${storeBtn('mac', 'btn-lg')}</div></div>
<p class="hx-micro"><a href="/download/">See how the trial works</a> &middot; 7-day free trial &middot; No OpenFrame account &middot; Works offline</p>
</div>
<div class="hx-tabs" data-hx-tabs>
<div class="hx-nav"><button type="button" class="hx-arr" data-hx-prev aria-label="Previous product">${svg('left')}</button>
<ul class="hx-chips" role="tablist" aria-label="What is in OpenFrame">${TABS.map(chip)}</ul>
<button type="button" class="hx-arr" data-hx-next aria-label="Next product">${svg('right')}</button>
<button type="button" class="hx-pp" data-hx-pp aria-pressed="false" aria-label="Pause the automatic tour">${svg('pause')}</button></div>
<div class="hx-card">${TABS.map(pane)}</div>
</div></div></section>

<section class="sc show"><div class="wrap-w">
<div class="sc-head rv"><span class="kick">All your needs in a single place</span>
<h2 class="disp">One app for the whole film.</h2>
<p class="sc-lead">Why use five different apps when you can do all of them in one, at the cost of one? Ideas, story, screenplay, breakdown, schedule and call sheets all talk to each other, so you never retype.</p>
<a class="btn-navy" href="/download/#trial">Start your free trial</a></div>
<div class="cc-row rv" data-ccs>${CARDS.map(card)}</div>
</div></section>

<section class="ss" data-ss aria-labelledby="ss-h"><div class="ss-track"><div class="ss-stick"><div class="wrap-w ss-grid">
<div class="ss-left"><span class="kick kick-i">${svg('cal')}Planning</span>
<h2 class="serif" id="ss-h">A better way to plan your shoot.</h2>
<ul class="ss-list">${STORY.map(storyItem)}</ul></div>
<div class="ss-right"><div class="gframe gf-1"><div class="ss-stage">${STORY.map(storyShot)}</div></div></div>
</div></div></div></section>

<section class="alt"><div class="wrap-w alt-grid">
<div class="alt-art rv"><div class="gframe gf-2"><div class="local-art">
<svg viewBox="0 0 520 330" role="img" aria-label="Diagram: your project stays on your computer; the connection to outside servers is crossed out.">
<rect x="18" y="70" width="230" height="170" rx="16" class="dg-card" stroke-width="2"/>
<rect x="34" y="86" width="198" height="116" rx="8" fill="#1d1a2e"/>
<g transform="translate(110 100) scale(2.5)" fill="none" stroke="#ff9a3d" stroke-width="1.3" stroke-linecap="round"><rect x="2" y="2" width="20" height="20" rx="2.2"/><path d="M7 2v20M17 2v20M2 12h20M2 7h5M2 17h5M22 7h-5M22 17h-5"/></g>
<text x="133" y="190" text-anchor="middle" fill="#fff" font-weight="700" font-size="14">YOUR FILM</text>
<rect x="70" y="240" width="126" height="12" rx="6" class="dg-stand"/>
<text x="133" y="280" text-anchor="middle" font-weight="700" font-size="16" class="dg-ink">Your computer</text>
<text x="133" y="300" text-anchor="middle" font-size="13" class="dg-mut">Everything lives here</text>
<path d="M400 118c-26 0-40 14-40 32-18 3-30 16-30 32 0 18 14 32 34 32h76c20 0 34-12 34-30 0-16-12-28-28-30-2-22-18-36-46-36z" class="dg-cloud" stroke-width="2" stroke-dasharray="5 6" transform="translate(22 0)"/>
<text x="412" y="278" text-anchor="middle" font-weight="700" font-size="16" class="dg-mut">Their servers</text>
<text x="412" y="298" text-anchor="middle" font-size="13" class="dg-mut">Not required</text>
<path d="M252 156 H350" stroke="#9a94ab" stroke-width="3" fill="none" stroke-dasharray="6 7"/>
<circle cx="302" cy="156" r="19" fill="#d2441c"/><path d="M294 148l16 16M310 148l-16 16" stroke="#fff" stroke-width="3.2" stroke-linecap="round"/>
<text x="302" y="118" text-anchor="middle" font-weight="800" font-size="14" fill="#d2441c">Nothing leaves</text></svg></div></div></div>
<div class="alt-copy rv d1"><span class="kick kick-i">${svg('lock')}Completely local</span>
<h2 class="serif">Your privacy stays on your computer.</h2>
<ul class="rows">
<li>${svg('local')}<div><b>Your project is a folder on your computer</b><p>Or on an external drive you choose.</p></div></li>
<li>${svg('offline')}<div><b>Works fully offline</b><p>No sign-in and no internet needed to write, plan or export.</p></div></li>
<li>${svg('lock')}<div><b>Nothing leaves unless you send it</b><p>Share a script or call sheet as a file, the way you always have.</p></div></li>
<li>${svg('arrow')}<div><b>You are never trapped</b><p>Export, back up and move your whole project any time.</p></div></li>
</ul><a class="ulink" href="/local-and-private/">Why local matters ${svg('arrow')}</a></div>
</div></section>

<section class="alt"><div class="wrap-w alt-grid">
<div class="alt-copy rv"><span class="kick kick-i">${svg('chat')}OpenFrame AI</span>
<h2 class="serif">Help when you want it. You stay in charge.</h2>
<ul class="rows">
<li>${svg('chat')}<div><b>Answers from your own project</b><p>Ask in plain language and see the sources it used.</p></div></li>
<li>${svg('once')}<div><b>Nothing changes until you press Apply</b><p>Proposed changes wait in a card you can read, apply or reject, then undo.</p></div></li>
<li>${svg('offline')}<div><b>Runs on your computer</b><p>After a one-time download that you start. If you do not want it, leave it off: OpenFrame works completely without it.</p></div></li>
</ul><a class="ulink" href="/features/openframe-ai/">Meet OpenFrame AI ${svg('arrow')}</a></div>
<div class="alt-art rv d1"><div class="gframe gf-3">${both('ai-proposed-changes', 'ai-panel-screenplay')}</div></div>
</div></section>

<section class="alt"><div class="wrap-w alt-grid">
<div class="alt-art rv"><div class="gframe gf-4">${both('sp-import-preview', 'screenplay-tools-import-screenplay')}</div></div>
<div class="alt-copy rv d1"><span class="kick kick-i">${svg('arrow')}Easy migration</span>
<h2 class="serif">Coming from other software? Bring your scripts.</h2>
<ul class="rows">
<li>${svg('list')}<div><b>See before anything is applied</b><p>Every item and every warning is listed in a preview. Nothing is silently flattened or lost.</p></div></li>
<li>${svg('doc')}<div><b>The formats you already have</b><p>Final Draft (.fdx), Fountain, Word, plain text and pasted text, plus PDF on a best-effort basis.</p></div></li>
</ul><a class="ulink" href="/switch/">See how switching works ${svg('arrow')}</a></div>
</div></section>

<section class="alt"><div class="wrap-w alt-grid">
<div class="alt-copy rv"><span class="kick kick-i">${svg('sun')}Themes</span>
<h2 class="serif">Do not like the colours? Make them yours.</h2>
<ul class="rows">
<li>${svg('sun')}<div><b>Light, Dark or System</b><p>Choose a theme in settings, with built-in starters to begin from.</p></div></li>
<li>${svg('doc')}<div><b>Paste a theme file</b><p>It is previewed before it is applied. Your screenplay and documents always keep their professional look.</p></div></li>
</ul><a class="ulink" href="/themes/">Explore themes ${svg('arrow')}</a></div>
<div class="alt-art rv d1"><div class="gframe gf-5">${both('settings-appearance', 'settings-appearance')}</div></div>
</div></section>

<section class="prm" aria-labelledby="prm-h"><div class="wrap-w">
<div class="sc-head rv"><span class="kick">Why we built it</span>
<h2 class="disp" id="prm-h">From film lovers, to film lovers.</h2>
<p class="sc-lead">OpenFrame is made for people who make films with limited resources: writer-directors, student filmmakers, short-film makers and regional-language filmmakers.</p></div>
<div class="pc-wrap rv" data-pcs role="group" aria-roledescription="carousel" aria-label="Our promises">
<div class="pc-stage">${PROMISES.map(promise)}</div>
<div class="pc-ctl"><div class="pc-dots" role="group" aria-label="Choose a promise">${PROMISES.map((p, i) => html`<button type="button" class="pc-dot${i === 0 ? ' is-on' : ''}" data-pc-dot="${i}" aria-label="Promise ${p[0]}"${i === 0 ? raw(' aria-current="true"') : ''}><i></i></button>`)}</div>
<button type="button" class="pc-pp" data-pc-pp aria-pressed="false" aria-label="Pause the automatic slides">${svg('pause')}</button></div></div>
<div class="langs rv" aria-label="Scripts you can write and export in"><span>English</span><span lang="te">తెలుగు</span><span lang="hi">हिन्दी</span><span lang="ta">தமிழ்</span><span lang="ml">മലയാളം</span><span lang="kn">ಕನ್ನಡ</span></div>
</div></section>

<section class="fmt"><div class="wrap-w">
<div class="sc-head rv"><span class="kick">Works with the files you already have</span>
<h2 class="disp">Bring your files. Take them with you.</h2>
<p class="sc-lead">Import scripts from other tools, export every document, and keep whole projects as plain packages you can back up and move.</p></div>
<ul class="tiles rv">${FILES.map(([e, n]) => html`<li class="tile"><b>${e}</b><span>${n}</span></li>`)}</ul>
<div class="duo rv">
<a class="duo-card" href="/switch/"><span class="duo-top">${svg('arrow')}<i>${svg('up')}</i></span><h3>Import</h3><p>Final Draft (.fdx), Fountain, Word (.docx), plain text and pasted text, plus PDF on a best-effort basis. Preview first.</p></a>
<a class="duo-card" href="/features/import-export/"><span class="duo-top">${svg('doc')}<i>${svg('up')}</i></span><h3>Export</h3><p>Screenplays as PDF, Final Draft, Fountain, Word and plain text. Production documents as PDF, CSV or XLSX.</p></a>
</div></div></section>

<section class="fin" aria-labelledby="fin-h"><div class="fin-panel">
<div class="hx-blobs" aria-hidden="true"><i class="b1"></i><i class="b2"></i><i class="b3"></i></div>
<div class="fin-in rv"><span class="kick kick-w">Ready when you are</span>
<h2 id="fin-h">Have trust issues? Get a 7-day trial.</h2>
<p class="fin-lead">Install it, write a scene, schedule a shoot. If it isn’t for you, you’ve lost nothing. Keep it for ${ctx.price()}, paid once.</p>
<div class="fin-grid">
<div class="fin-card"><span class="fin-tag">Windows 11</span><h3>Get it on the Microsoft Store</h3><p>7-day free trial, then one payment. Pay once and it’s yours.</p>${storeBtn('win', 'btn-lg')}</div>
<div class="fin-card"><span class="fin-tag">Mac</span><h3>Get it on the Mac App Store</h3><p>7-day free trial on Apple silicon Macs, then the same one-time price.</p>${storeBtn('mac', 'btn-lg')}</div>
</div>
<p class="fin-small">The store sets the final checkout amount, including any taxes that apply where you live. Windows and Mac are bought separately.</p>
</div></div></section>`;
  return [{
    path: '/', nav: 'home', top, title: 'OpenFrame Studio: film production software',
    description: 'One app for screenwriters and film teams: Idea Vault, Beat Board, screenplay editor, breakdown, schedule and call sheets. 7-day free trial, then pay once.',
    body,
    jsonld: [APP(ctx, 'Windows 11, macOS 14 or later'), ORG(ctx), WEBSITE(ctx)],
  }];
}
