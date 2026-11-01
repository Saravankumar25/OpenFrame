// Home page (/). Same fonts, colours and components as the approved website design; the page is arranged as a product
// overview: hero with the app beside the text, one application instead of five, ownership, privacy, import, optional
// tools, principles, then the download. Screens are the real captured app screens of the selected platform; claims
// follow research/capability-matrix.md.
import { APP, ORG, WEBSITE } from './info.mjs';

const STAGES = [
  ['Ideate', 'Idea Vault', 'Collect notes, images, links, voice notes and files in one place.', { win: 'vault-grid-all-items', mac: 'vault-filled' }],
  ['Write', 'Screenplay', 'Standard screenplay formatting as you type.', { win: 'sp-editor-standard', mac: 'screenplay-editor-typing' }],
  ['Plan', 'Shooting schedule', 'Scenes become strips on shooting days, with warnings before problems arise.', { win: 'prod-schedule-board', mac: 'schedule-board' }],
  ['Shoot', 'Call sheets', 'A call sheet filled from the shooting day, ready to finalise and export.', { win: 'cs-editor-draft', mac: 'callsheet-editor' }],
];

const IC = {
  bulb: '<path d="M9 18h6M10 21h4M12 3a6 6 0 0 0-3.5 10.9c.6.5 1 1.2 1 2.1h5c0-.9.4-1.6 1-2.1A6 6 0 0 0 12 3z"/>',
  board: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M9 4v16M15 4v16"/>',
  page: '<path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/><path d="M14 3v5h5M9 13h6M9 17h6"/>',
  list: '<path d="M9 6h12M9 12h12M9 18h12M4 6h.01M4 12h.01M4 18h.01"/>',
  clip: '<rect x="5" y="4" width="14" height="17" rx="2"/><path d="M9 4V3h6v1M9 10h6M9 14h6"/>',
  x: '<path d="M6 6l12 12M18 6L6 18"/>',
};

export default function home(ctx) {
  const { html, raw, mock, storeRow } = ctx;
  const svg = (k) => raw(`<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">${IC[k]}</svg>`);
  const stage = ([name, label, line, scr], i) => html`<div class="stage-panel" id="stage-${i}" role="tabpanel" aria-labelledby="stage-tab-${i}"${i ? raw(' hidden') : ''}>
<div data-os-show="win">${mock('win', scr.win, { caption: `${label}. ${line}`, eager: i === 0 })}</div>
<div data-os-show="mac">${mock('mac', scr.mac, { caption: `${label}. ${line}`, eager: i === 0 })}</div></div>`;
  const body = html`
<section class="hero hero-split"><div class="wrap hero-grid">
<div class="hero-text">
<a class="chip" href="/download/">Windows 11 and Mac &middot; 7-day free trial &rarr;</a>
<h1>Film production software for the whole project.</h1>
<p class="lead">Write the screenplay, break it down, build the shooting schedule and issue call sheets in one application. Your project is stored on your own computer.</p>
<div class="hero-cta">${storeRow()}</div>
<ul class="micro"><li>7-day free trial</li><li>One-time payment</li><li>No OpenFrame account</li><li>Works offline</li></ul>
</div>
<div class="tour tour-real" data-stages>
<ul class="stages" role="tablist" aria-label="From idea to shoot">${STAGES.map(([n], i) => html`<li role="presentation"><button type="button" role="tab" id="stage-tab-${i}" aria-controls="stage-${i}" aria-selected="${String(i === 0)}" tabindex="${i ? -1 : 0}"><b>${n}</b></button></li>`)}</ul>
${STAGES.map(stage)}
<p class="tour-note">Screens from OpenFrame Studio. Use the controls at the top of the page to switch between Windows and Mac, and between light and dark screens.</p>
</div></div></section>

<section class="tint"><div class="wrap">
<div class="sec-head left rv"><span class="eyebrow">One application</span>
<h2>Replace five separate tools with one.</h2>
<p class="lead">Ideas, story structure, the screenplay, breakdown and scheduling, and call sheets usually live in different programs. OpenFrame keeps them in one project, so nothing is retyped or exported between steps.</p></div>
<div class="merge static side">
<div class="apps" aria-label="The five kinds of tool OpenFrame replaces">
${[['bulb', 'Notes and idea app'], ['board', 'Story outlining tool'], ['page', 'Screenwriting software'], ['list', 'Breakdown and scheduling'], ['clip', 'Call sheet tool']].map(([k, t]) => html`<div class="app">${svg(k)}<b>${t}</b></div>`)}
</div>
<div class="one"><div class="one-card"><div class="wm"><i>Open</i><b>Frame</b></div><p>One application, one install, one payment.</p>
<div class="pills"><span>Idea Vault</span><span>Beat Board</span><span>Screenplay</span><span>Breakdown and Schedule</span><span>Call Sheets</span></div></div></div>
</div>
<div class="merge-stats rv">
<div class="stat"><b>1</b><span>application to install and update</span></div>
<div class="stat"><b>0</b><span>monthly subscriptions</span></div>
<div class="stat"><b>1</b><span>folder on your computer holds the whole project</span></div>
</div></div></section>

<section class="dark"><div class="wrap manifesto">
<div class="rv"><span class="eyebrow">Ownership</span>
<h2>Pay once and keep it.</h2>
<p class="lead mt18">Subscription tools charge every month for as long as you keep working. OpenFrame is a one-time purchase, with a 7-day free trial first.</p>
<ul class="nos"><li>${svg('x')}No mandatory cloud.</li><li>${svg('x')}No OpenFrame account.</li><li>${svg('x')}No monthly subscription.</li></ul>
<p class="big-line">One payment. <span>Yours to keep.</span></p></div>
<div class="rv d2"><div class="chart">
<svg viewBox="0 0 520 320" role="img" aria-label="Illustration: the cost of subscription tools keeps climbing every year, while OpenFrame is one payment.">
<g stroke="#ffffff1f"><line x1="46" y1="270" x2="500" y2="270"/><line x1="46" y1="200" x2="500" y2="200" stroke-dasharray="3 6"/><line x1="46" y1="130" x2="500" y2="130" stroke-dasharray="3 6"/><line x1="46" y1="60" x2="500" y2="60" stroke-dasharray="3 6"/></g>
<path d="M46 250 L110 250 L110 228 L190 228 L190 190 L270 190 L270 150 L350 150 L350 108 L430 108 L430 66 L492 66" stroke="#ff7a52" stroke-width="4" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
<path d="M46 250 L56 250 L56 214 L492 214" stroke="#ffffff" stroke-width="4" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
<circle cx="56" cy="214" r="6" fill="#fff"/>
<text x="496" y="52" text-anchor="end" fill="#ffab8e" font-weight="700" font-size="15">Subscription tools: the cost keeps rising</text>
<text x="496" y="240" text-anchor="end" fill="#fff" font-weight="700" font-size="15">OpenFrame: one payment</text>
<g text-anchor="middle"><text x="110" y="292">Year 1</text><text x="190" y="292">Year 2</text><text x="270" y="292">Year 3</text><text x="350" y="292">Year 4</text><text x="430" y="292">Year 5</text></g>
<text x="8" y="32">Total paid</text></svg>
<p class="cap">Illustration of the idea, not real prices.</p></div></div>
</div></section>

<section><div class="wrap split">
<div class="rv"><span class="eyebrow">Privacy</span>
<h2>Your project stays on your computer.</h2>
<p class="lead mt18">Scripts, notes and schedules are private work. OpenFrame stores them as files on your own drive and does not need a server.</p>
<ul class="ticks">
<li><span><b>Local project folder.</b> Your project is a folder on your computer, or on an external drive you choose.</span></li>
<li><span><b>Fully offline.</b> Writing, planning and exporting need no sign-in and no internet connection.</span></li>
<li><span><b>Nothing leaves unless you send it.</b> Share scripts and call sheets as files.</span></li>
<li><span><b>No lock-in.</b> Export, back up and move the whole project at any time.</span></li>
</ul></div>
<div class="diagram rv d2"><svg viewBox="0 0 520 330" role="img" aria-label="Diagram: your project stays on your computer; the connection to outside servers is crossed out.">
<rect x="18" y="70" width="230" height="170" rx="16" class="dg-card" stroke-width="2"/>
<rect x="34" y="86" width="198" height="116" rx="8" fill="#1d1a2e"/>
<g transform="translate(110 100) scale(2.5)" fill="none" stroke="#ff9a3d" stroke-width="1.3" stroke-linecap="round"><rect x="2" y="2" width="20" height="20" rx="2.2"/><path d="M7 2v20M17 2v20M2 12h20M2 7h5M2 17h5M22 7h-5M22 17h-5"/></g>
<text x="133" y="190" text-anchor="middle" fill="#fff" font-weight="700" font-size="14">YOUR FILM</text>
<rect x="70" y="240" width="126" height="12" rx="6" class="dg-stand"/>
<text x="133" y="280" text-anchor="middle" font-weight="700" font-size="16" class="dg-ink">Your computer</text>
<text x="133" y="300" text-anchor="middle" font-size="13" class="dg-mut">The project lives here</text>
<path d="M400 118c-26 0-40 14-40 32-18 3-30 16-30 32 0 18 14 32 34 32h76c20 0 34-12 34-30 0-16-12-28-28-30-2-22-18-36-46-36z" class="dg-cloud" stroke-width="2" stroke-dasharray="5 6" transform="translate(22 0)"/>
<text x="412" y="278" text-anchor="middle" font-weight="700" font-size="16" class="dg-mut">Outside servers</text>
<text x="412" y="298" text-anchor="middle" font-size="13" class="dg-mut">Not required</text>
<path d="M252 156 H350" stroke="#9a94ab" stroke-width="3" fill="none" stroke-dasharray="6 7"/>
<circle cx="302" cy="156" r="19" fill="#d2441c"/><path d="M294 148l16 16M310 148l-16 16" stroke="#fff" stroke-width="3.2" stroke-linecap="round"/>
<text x="302" y="118" text-anchor="middle" font-weight="800" font-size="14" fill="#d2441c">No data leaves</text></svg></div>
</div></section>

<section class="tint"><div class="wrap split flip">
<div class="rv"><span class="eyebrow">Import</span>
<h2>Bring your existing scripts.</h2>
<p class="lead mt18">Import Final Draft, Fountain, PDF, DOCX and plain text files. OpenFrame shows what will be imported, with every warning, before anything is applied.</p>
<div class="fmts"><span>.fdx</span><span>.fountain</span><span>.pdf</span><span>.docx</span><span>.txt</span></div>
<p class="mt26"><a class="btn btn-dark" href="/switch/">How switching works &rarr;</a></p></div>
<div class="rv d2"><div data-os-show="win">${mock('win', 'sp-import-preview', { caption: 'The import preview on Windows' })}</div><div data-os-show="mac">${mock('mac', 'screenplay-tools-import-screenplay', { caption: 'Import a screenplay on the Mac' })}</div></div>
</div></section>

<section><div class="wrap">
<div class="sec-head left rv"><span class="eyebrow">Optional</span>
<h2>AI assistance and themes, when you want them.</h2></div>
<div class="split rv">
<div class="rv d2"><div data-os-show="win">${mock('win', 'ai-proposed-changes', { link: false })}</div><div data-os-show="mac">${mock('mac', 'ai-panel-screenplay', { link: false })}</div></div>
<div><span class="eyebrow">OpenFrame AI</span>
<h3 class="t3">An assistant that works only on your own project.</h3>
<p class="lead mt18">Ask questions in plain language and get answers from your project, with the sources shown. Nothing changes until you choose Apply. The assistant runs on your computer after a one-time download that you start yourself, and OpenFrame works fully without it.</p>
<p class="mt26"><a class="more" href="/features/openframe-ai/">About OpenFrame AI &rarr;</a></p></div>
</div>
<div class="split flip rv mt84">
<div><span class="eyebrow">Themes</span>
<h3 class="t3">Choose how the app looks.</h3>
<p class="lead mt18">Select Light, Dark or System, or load a theme file and preview it before applying it. Screenplays and exported documents keep their standard professional formatting.</p>
<p class="mt26"><a class="more" href="/themes/">Explore themes &rarr;</a></p></div>
<div class="rv d2"><div data-os-show="win">${mock('win', 'settings-appearance', { link: false })}</div><div data-os-show="mac">${mock('mac', 'settings-appearance', { link: false })}</div></div>
</div>
</div></section>

<section class="tint"><div class="wrap">
<div class="sec-head left rv"><span class="eyebrow">Who it is for</span>
<h2>Made for filmmakers working with limited resources.</h2>
<p class="lead">OpenFrame is built for writer-directors, student filmmakers, short-film makers and regional-language filmmakers.</p>
<div class="langs left rv" aria-label="Languages you can write and export scripts in"><span>English</span><span lang="te">తెలుగు</span><span lang="hi">हिन्दी</span><span lang="ta">தமிழ்</span><span lang="ml">മലയാളം</span><span lang="kn">ಕನ್ನಡ</span></div></div>
<div class="promise-row">
<div class="card rv"><div class="n">01</div><h3>Your work is never changed silently</h3><p>OpenFrame suggests and you decide. Nothing rewrites your story, script or schedule on its own.</p></div>
<div class="card rv d1"><div class="n">02</div><h3>Nothing is lost by accident</h3><p>Deleted items stay in Recently Deleted, undo is offered after changes, and your work saves as you go.</p></div>
<div class="card rv d2"><div class="n">03</div><h3>You can leave at any time</h3><p>Every document exports, so you can back up, move and keep your project.</p></div>
</div></div></section>

<section class="final"><div class="wrap">
<span class="eyebrow rv">Get started</span>
<h2 class="rv">Try OpenFrame free for 7 days.</h2>
<p class="lead rv mt18">Install it, write a scene and schedule a shoot. If it does not suit you, you have lost nothing. <a class="more" href="/download/#trial">How the free trial works &rarr;</a></p>
<div class="trio">
<div class="panel win rv"><span class="tagline">Windows 11</span><h3>Download from the Microsoft Store</h3><p>7-day free trial, then a one-time payment of ${ctx.price()}.</p>${ctx.storeBtn('win', 'btn-lg')}</div>
<div class="panel apple rv d1"><span class="tagline">Mac</span><h3>Download from the Mac App Store</h3><p>7-day free trial on Apple silicon Macs, then the same one-time price.</p>${ctx.storeBtn('mac', 'btn-lg')}</div>
</div>
<p class="small-print center">The store sets the final checkout amount, including any taxes that apply where you live.</p>
</div></section>`;
  return [{
    path: '/', nav: 'home', title: 'OpenFrame Studio: film production software',
    description: 'One application for screenwriters and film teams: Idea Vault, Beat Board, screenplay editor, breakdown, schedule and call sheets. 7-day free trial, then pay once.',
    body,
    jsonld: [APP(ctx, 'Windows 11, macOS 14 or later'), ORG(ctx), WEBSITE(ctx)],
  }];
}
