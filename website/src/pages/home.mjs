// Home page (/), laid out as the approved website design's home page. Screens are the real captured app screens of
// the selected platform; claims follow research/capability-matrix.md.
import { APP, ORG, WEBSITE } from './info.mjs';

const STAGES = [
  ['Ideate', 'Idea Vault', 'Drop anything in: notes, images, links, voice notes and files.', { win: 'vault-grid-all-items', mac: 'vault-filled' }],
  ['Write', 'Screenplay', 'Standard screenplay formatting while you type.', { win: 'sp-editor-standard', mac: 'screenplay-editor-typing' }],
  ['Plan', 'Shooting schedule', 'Scenes become strips on shooting days, with warnings before problems.', { win: 'prod-schedule-board', mac: 'schedule-board' }],
  ['Shoot', 'Call sheets', 'A call sheet filled from the shooting day, ready to finalise and export.', { win: 'cs-editor-draft', mac: 'callsheet-editor' }],
];

const IC = {
  local: '<rect x="3" y="4" width="18" height="12" rx="2"/><path d="M8 20h8M12 16v4"/>',
  once: '<circle cx="12" cy="12" r="9"/><path d="M8.5 12.5l2.5 2.5 4.5-5"/>',
  five: '<rect x="3" y="3" width="7" height="7" rx="1.5"/><rect x="14" y="3" width="7" height="7" rx="1.5"/><rect x="3" y="14" width="7" height="7" rx="1.5"/><rect x="14" y="14" width="7" height="7" rx="1.5"/>',
  lock: '<rect x="4.5" y="10.5" width="15" height="10" rx="2"/><path d="M8 10.5V7a4 4 0 0 1 8 0v3.5"/>',
  arrow: '<path d="M5 12h14M13 6l6 6-6 6"/>',
  offline: '<path d="M3 3l18 18M8.5 16.5a5 5 0 0 1 7 0M5 13a10 10 0 0 1 5-2.7M14.5 10.4A10 10 0 0 1 19 13M12 20h.01"/>',
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
<section class="hero center"><div class="wrap">
<a class="chip" href="/download/">One app for Windows 11 and Mac &middot; 7-day free trial &rarr;</a>
<h1>A film production software for all your needs.</h1>
<p class="caption">From film lovers, to film lovers across the world.</p>
<div class="hero-cta">${storeRow()}</div>
<ul class="micro"><li>7-day free trial</li><li>No Straft account</li><li>Works offline</li></ul>
</div>
<div class="tour tour-real" data-stages>
<ul class="stages" role="tablist" aria-label="From idea to shoot">${STAGES.map(([n], i) => html`<li role="presentation"><button type="button" role="tab" id="stage-tab-${i}" aria-controls="stage-${i}" aria-selected="${String(i === 0)}" tabindex="${i ? -1 : 0}"><b>${n}</b></button></li>`)}</ul>
${STAGES.map(stage)}
<p class="tour-note">Real screens from Straft Studio. Use the controls at the top of the page to switch between Windows and Mac, and between light and dark app screens.</p>
</div></section>

<div class="trust"><div class="wrap"><ul>
<li>${svg('local')}<div>Completely local<span>Lives on your computer</span></div></li>
<li>${svg('once')}<div>Pay once<span>Own forever</span></div></li>
<li>${svg('lock')}<div>7-day free trial<span>Then one payment</span></div></li>
<li>${svg('offline')}<div>Works offline<span>No sign-in to write</span></div></li>
<li>${svg('arrow')}<div>Easy migration<span>Bring your scripts</span></div></li>
</ul></div></div>

<section class="tint"><div class="wrap">
<div class="sec-head rv"><span class="eyebrow">All your needs in a single place</span>
<h2>All your needs in a single place: Straft.</h2>
<p class="lead">Why use five different apps when you can do all of them in one, at the cost of one?</p></div>
<div class="merge static">
<div class="apps" aria-label="Five separate kinds of app">
${[['bulb', 'Notes and idea app'], ['board', 'Story outlining tool'], ['page', 'Screenwriting software'], ['list', 'Breakdown and scheduling'], ['clip', 'Call sheet tool']].map(([k, t]) => html`<div class="app">${svg(k)}<b>${t}</b><small>own login, own bill</small></div>`)}
</div>
<div class="one"><div class="one-card"><div class="wm"><b>Straft</b></div><p>One app. One install. One payment.</p>
<div class="pills"><span>Idea Vault</span><span>Beat Board</span><span>Screenplay</span><span>Breakdown and Schedule</span><span>Call Sheets</span></div></div></div>
</div>
<div class="merge-stats rv">
<div class="stat"><b><s>5</s><i>&rarr;</i>1</b><span>app to use, install and update</span></div>
<div class="stat"><b><s>5</s><i>&rarr;</i>0</b><span>monthly subscriptions</span></div>
<div class="stat"><b><s>5</s><i>&rarr;</i>1</b><span>place your film lives: a folder on your computer</span></div>
</div></div></section>

<section class="dark"><div class="wrap manifesto">
<div class="rv"><span class="eyebrow">The way it should have been</span>
<h2>Let’s break the chain of corporate rentals.</h2>
<p class="lead mt18">You’ve been renting the tools to tell your own story, every month, for as long as you make films. Straft is software you simply own.</p>
<ul class="nos"><li>${svg('x')}No mandatory cloud.</li><li>${svg('x')}No Straft account.</li><li>${svg('x')}No monthly subscription.</li></ul>
<p class="big-line">Pay once. <span>Own forever.</span></p></div>
<div class="rv d2"><div class="chart">
<svg viewBox="0 0 520 320" role="img" aria-label="Illustration: the cost of rented tools keeps climbing every year, while Straft is one payment.">
<g stroke="#ffffff1f"><line x1="46" y1="270" x2="500" y2="270"/><line x1="46" y1="200" x2="500" y2="200" stroke-dasharray="3 6"/><line x1="46" y1="130" x2="500" y2="130" stroke-dasharray="3 6"/><line x1="46" y1="60" x2="500" y2="60" stroke-dasharray="3 6"/></g>
<path d="M46 250 L110 250 L110 228 L190 228 L190 190 L270 190 L270 150 L350 150 L350 108 L430 108 L430 66 L492 66" stroke="#ff7a52" stroke-width="4" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
<path d="M46 250 L56 250 L56 214 L492 214" stroke="#ffffff" stroke-width="4" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
<circle cx="56" cy="214" r="6" fill="#fff"/>
<text x="496" y="52" text-anchor="end" fill="#ffab8e" font-weight="700" font-size="15">Rented tools: the bill never stops</text>
<text x="496" y="240" text-anchor="end" fill="#fff" font-weight="700" font-size="15">Straft: you paid once</text>
<g text-anchor="middle"><text x="110" y="292">Year 1</text><text x="190" y="292">Year 2</text><text x="270" y="292">Year 3</text><text x="350" y="292">Year 4</text><text x="430" y="292">Year 5</text></g>
<text x="8" y="32" font-size="13">Total you’ve paid</text></svg>
<p class="cap">Illustration of the idea, not real prices.</p></div></div>
</div></section>

<section><div class="wrap split">
<div class="rv"><span class="eyebrow">Completely local</span>
<h2>Your privacy stays on your computer.</h2>
<p class="lead mt18">Your story is the most private thing you’ll ever write. Straft keeps it where it belongs, with you.</p>
<ul class="ticks">
<li><span><b>Your project is a folder on your computer</b>, or on an external drive you choose.</span></li>
<li><span><b>Works fully offline.</b> No sign-in and no internet needed to write, plan or export.</span></li>
<li><span><b>Nothing leaves unless you send it.</b> Share a script or call sheet as a file, the way you always have.</span></li>
<li><span><b>You’re never trapped.</b> Export, back up and move your whole project any time.</span></li>
</ul></div>
<div class="diagram rv d2"><svg viewBox="0 0 520 330" role="img" aria-label="Diagram: your project stays on your computer; the connection to outside servers is crossed out.">
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
<text x="302" y="118" text-anchor="middle" font-weight="800" font-size="14" fill="#d2441c">Nothing leaves</text></svg></div>
</div></section>

<section class="tint"><div class="wrap">
<div class="sec-head rv"><span class="eyebrow">What makes Straft different</span>
<h2>Built different, on purpose.</h2>
<p class="lead">Six things you won’t find together in any other film production software.</p></div>
<div class="grid3">
<div class="card rv"><div class="ic">${svg('local')}</div><h3>Completely local</h3><p>Your scripts, ideas and schedules are files on your computer. Not rented space on someone else’s.</p></div>
<div class="card rv d1"><div class="ic">${svg('once')}</div><h3>Buy once, own forever</h3><p>No subscription. No renewal date. No “your plan has expired” in the middle of a shoot.</p></div>
<div class="card rv d2"><div class="ic">${svg('five')}</div><h3>The power of 5 apps in 1 app</h3><p>Ideas, story, screenplay, breakdown, schedule and call sheets, all talking to each other, so you never retype.</p></div>
<div class="card hl rv"><span class="badge">Risk-free</span><div class="ic">${svg('lock')}</div><h3>Have trust issues? Good.</h3><p><b>Try it free for 7 days.</b> Judge it by using it, not by reading about it.</p><a class="btn btn-primary btn-sm" href="/download/#trial">How the free trial works</a></div>
<div class="card rv d1"><div class="ic">${svg('arrow')}</div><h3>Easy migration from other software</h3><p>Bring your existing scripts: Final Draft (.fdx), Fountain, PDF, DOCX or plain text. Preview before anything is imported.</p></div>
<div class="card rv d2"><div class="ic">${svg('offline')}</div><h3>Works when the internet doesn’t</h3><p>Remote location, power cut, no signal: your film keeps moving forward.</p></div>
</div></div></section>

<section class="bt"><div class="wrap">
<div class="sec-head rv"><span class="eyebrow">Make it yours</span>
<h2>Help when you want it. Looks the way you like it.</h2>
<p class="lead">Two optional extras. Neither gets in your way until you ask for it.</p></div>
<div class="grid2">
<article class="card big rv">
<div data-os-show="win">${mock('win', 'ai-proposed-changes', { link: false })}</div><div data-os-show="mac">${mock('mac', 'ai-panel-screenplay', { link: false })}</div>
<span class="eyebrow">Straft AI</span>
<h3>Straft AI that helps. You stay in charge.</h3>
<p>Ask in plain language and get answers from your own project, with the sources shown. It changes nothing until you press Apply. It runs on your computer after a one-time download that you start, and if you do not want it, leave it off. Straft works completely without it.</p>
<a class="more" href="/features/straft-ai/">Meet Straft AI &rarr;</a></article>
<article class="card big rv d2">
<div data-os-show="win">${mock('win', 'settings-appearance', { link: false })}</div><div data-os-show="mac">${mock('mac', 'settings-appearance', { link: false })}</div>
<span class="eyebrow">Themes</span>
<h3>Do not like the colours? Make them yours.</h3>
<p>Choose a theme in Light, Dark or System. Still not right? Paste a theme file, previewed before it is applied. Your screenplay and documents always keep their professional look.</p>
<a class="more" href="/themes/">Explore themes &rarr;</a></article>
</div></div></section>

<section><div class="wrap center">
<span class="eyebrow rv">Why we built it</span>
<p class="quote rv">From <b>film lovers</b>, to <b>film lovers</b> across the world.</p>
<p class="lead rv lead-mid">Straft is made for people who make films with limited resources: writer-directors, student filmmakers, short-film makers and regional-language filmmakers.</p>
<div class="langs rv" aria-label="Scripts you can write and export in"><span>English</span><span lang="te">తెలుగు</span><span lang="hi">हिन्दी</span><span lang="ta">தமிழ்</span><span lang="ml">മലയാളം</span><span lang="kn">ಕನ್ನಡ</span></div>
<h2 class="rv mt84">Your idea. Your film. <span class="ac">Our promise.</span></h2>
<div class="promise-row">
<div class="card rv"><div class="n">01</div><h3>We never silently change your work</h3><p>Straft suggests. You decide. Nothing rewrites your story, script or schedule on its own.</p></div>
<div class="card rv d1"><div class="n">02</div><h3>Nothing is lost by accident</h3><p>Deleted items wait in Recently Deleted. Undo is offered after changes. Your work saves as you go.</p></div>
<div class="card rv d2"><div class="n">03</div><h3>Leave any time, taking everything</h3><p>Every document exports. Your project is yours to back up, move and keep.</p></div>
</div></div></section>

<section class="tint"><div class="wrap split">
<div class="rv"><span class="eyebrow">Easy migration</span>
<h2>Coming from other software? Bring your scripts.</h2>
<p class="lead mt18">See exactly what will be imported, and every warning, before anything is applied. Nothing is silently flattened or lost.</p>
<div class="fmts"><span>.fdx</span><span>.fountain</span><span>.pdf</span><span>.docx</span><span>.txt</span></div>
<p class="mt26"><a class="btn btn-dark" href="/switch/">See how switching works &rarr;</a></p></div>
<div class="rv d2"><div data-os-show="win">${mock('win', 'sp-import-preview', { caption: 'The import preview on Windows' })}</div><div data-os-show="mac">${mock('mac', 'screenplay-tools-import-screenplay', { caption: 'Import a screenplay on the Mac' })}</div></div>
</div></section>

<section class="final"><div class="wrap">
<span class="eyebrow rv">Ready when you are</span>
<h2 class="rv">Have trust issues? Get a 7-day trial.</h2>
<p class="lead rv mt18">Install it, write a scene, schedule a shoot. If it isn’t for you, you’ve lost nothing.</p>
<div class="trio">
<div class="panel win rv"><span class="tagline">Windows 11</span><h3>Get it on the Microsoft Store</h3><p>7-day free trial, then one payment of ${ctx.price()}. Pay once and it’s yours.</p>${ctx.storeBtn('win', 'btn-lg')}</div>
<div class="panel apple rv d1"><span class="tagline">Mac</span><h3>Get it on the Mac App Store</h3><p>7-day free trial on Apple silicon Macs, then the same one-time price.</p>${ctx.storeBtn('mac', 'btn-lg')}</div>
</div>
<p class="small-print center">The store sets the final checkout amount, including any taxes that apply where you live.</p>
</div></section>`;
  return [{
    path: '/', nav: 'home', title: 'Straft Studio: film production software',
    description: 'One app for screenwriters and film teams: Idea Vault, Beat Board, screenplay editor, breakdown, schedule and call sheets. 7-day free trial, then pay once.',
    body,
    jsonld: [APP(ctx, 'Windows 11, macOS 14 or later'), ORG(ctx), WEBSITE(ctx)],
  }];
}
