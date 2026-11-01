// Home page (/). Laid out like film paperwork: slugline labels, scene strips from a shooting-schedule board, script-page type.
// Screens are the real captured app screens of the selected platform; claims follow research/capability-matrix.md.
// All styling lives in src/assets/css/home.css under .hm.
import { APP, ORG, WEBSITE } from './info.mjs';

const STAGES = [
  ['Ideate', 'Idea Vault', 'Drop anything in: notes, images, links, voice notes and files.', { win: 'vault-grid-all-items', mac: 'vault-filled' }],
  ['Write', 'Screenplay', 'Standard screenplay formatting while you type.', { win: 'sp-editor-standard', mac: 'screenplay-editor-typing' }],
  ['Plan', 'Shooting schedule', 'Scenes become strips on shooting days, with warnings before problems.', { win: 'prod-schedule-board', mac: 'schedule-board' }],
  ['Shoot', 'Call sheets', 'A call sheet filled from the shooting day, ready to finalise and export.', { win: 'cs-editor-draft', mac: 'callsheet-editor' }],
];
const STRIP = ['y', 'b', 'g', 'p', 'o'];

export default function home(ctx) {
  const { html, raw, mock, storeRow } = ctx;
  const cross = raw('<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path d="M6 6l12 12M18 6L6 18"/></svg>');
  const stage = ([name, label, line, scr], i) => html`<div class="stage-panel" id="stage-${i}" role="tabpanel" aria-labelledby="stage-tab-${i}"${i ? raw(' hidden') : ''}>
<div data-os-show="win">${mock('win', scr.win, { caption: `${label}. ${line}`, eager: i === 0 })}</div>
<div data-os-show="mac">${mock('mac', scr.mac, { caption: `${label}. ${line}`, eager: i === 0 })}</div></div>`;
  const body = html`
<div class="hm">

<section class="hm-s hm-hero"><div class="hm-w">
<div class="hm-hero-top">
<div>
<a class="hm-slug" href="/download/">One app for Windows 11 and Mac &middot; 7-day free trial &rarr;</a>
<h1>A film production software for all your needs.</h1>
</div>
<div class="hm-hero-side">
<p class="hm-caption">From film lovers, to film lovers across the world.</p>
<div class="hm-cta">${storeRow()}</div>
<ul class="hm-micro"><li>7-day free trial</li><li>No OpenFrame account</li><li>Works offline</li></ul>
</div>
</div>
<div class="tour hm-tour" data-stages>
<ul class="stages" role="tablist" aria-label="From idea to shoot">${STAGES.map(([n], i) => html`<li role="presentation" class="${STRIP[i]}"><button type="button" role="tab" id="stage-tab-${i}" aria-controls="stage-${i}" aria-selected="${String(i === 0)}" tabindex="${i ? -1 : 0}"><i aria-hidden="true">0${i + 1}</i><b>${n}</b></button></li>`)}</ul>
<div class="hm-screen">${STAGES.map(stage)}</div>
<p class="tour-note">Real screens from OpenFrame Studio. Use the controls at the top of the page to switch between Windows and Mac, and between light and dark app screens.</p>
</div>
</div></section>

<section class="hm-s hm-board-s"><div class="hm-w"><ul class="hm-board">
<li class="y"><b>Completely local</b><span>Lives on your computer</span></li>
<li class="b"><b>Pay once</b><span>Own forever</span></li>
<li class="g"><b>7-day free trial</b><span>Then one payment</span></li>
<li class="p"><b>Works offline</b><span>No sign-in to write</span></li>
<li class="o"><b>Easy migration</b><span>Bring your scripts</span></li>
</ul></div></section>

<section class="hm-s"><div class="hm-w">
<div class="hm-head"><span class="hm-eb">All your needs in a single place</span>
<h2>All your needs in a single place: OpenFrame.</h2>
<p class="hm-lead">Why use five different apps when you can do all of them in one, at the cost of one?</p></div>
<div class="hm-merge">
<ol class="hm-five" aria-label="Five separate kinds of app">
${['Notes and idea app', 'Story outlining tool', 'Screenwriting software', 'Breakdown and scheduling', 'Call sheet tool'].map((t, i) => html`<li class="${STRIP[i]}"><i aria-hidden="true">${i + 1}</i><b>${t}</b></li>`)}
</ol>
<div class="hm-one"><div class="wm"><i>Open</i><b>Frame</b></div><p>One app. One install. One payment.</p>
<div class="hm-pills"><span>Idea Vault</span><span>Beat Board</span><span>Screenplay</span><span>Breakdown and Schedule</span><span>Call Sheets</span></div></div>
</div>
<div class="hm-stats">
<div><b><s>5</s><i>&rarr;</i>1</b><span>app to use, install and update</span></div>
<div><b><s>5</s><i>&rarr;</i>0</b><span>monthly subscriptions</span></div>
<div><b><s>5</s><i>&rarr;</i>1</b><span>place your film lives: a folder on your computer</span></div>
</div></div></section>

<section class="hm-s hm-yellow"><div class="hm-w hm-two">
<div><span class="hm-eb">The way it should have been</span>
<h2>Let’s break the chain of corporate rentals.</h2>
<p class="hm-lead">You’ve been renting the tools to tell your own story, every month, for as long as you make films. OpenFrame is software you simply own.</p>
<ul class="hm-nos"><li>${cross}No mandatory cloud.</li><li>${cross}No OpenFrame account.</li><li>${cross}No monthly subscription.</li></ul>
<p class="hm-big">Pay once. Own forever.</p></div>
<div class="hm-chart">
<svg viewBox="0 0 520 320" role="img" aria-label="Illustration: the cost of rented tools keeps climbing every year, while OpenFrame is one payment.">
<g class="grid"><line x1="46" y1="270" x2="500" y2="270"/><line x1="46" y1="200" x2="500" y2="200" stroke-dasharray="3 6"/><line x1="46" y1="130" x2="500" y2="130" stroke-dasharray="3 6"/><line x1="46" y1="60" x2="500" y2="60" stroke-dasharray="3 6"/></g>
<path class="rent" d="M46 250 L110 250 L110 228 L190 228 L190 190 L270 190 L270 150 L350 150 L350 108 L430 108 L430 66 L492 66"/>
<path class="own" d="M46 250 L56 250 L56 214 L492 214"/>
<circle class="dot" cx="56" cy="214" r="6"/>
<text class="t-rent" x="496" y="52" text-anchor="end">Rented tools: the bill never stops</text>
<text class="t-own" x="496" y="240" text-anchor="end">OpenFrame: you paid once</text>
<g text-anchor="middle"><text x="110" y="292">Year 1</text><text x="190" y="292">Year 2</text><text x="270" y="292">Year 3</text><text x="350" y="292">Year 4</text><text x="430" y="292">Year 5</text></g>
<text x="8" y="32">Total you’ve paid</text></svg>
<p class="cap">Illustration of the idea, not real prices.</p></div>
</div></section>

<section class="hm-s"><div class="hm-w hm-two">
<div><span class="hm-eb">Completely local</span>
<h2>Your privacy stays on your computer.</h2>
<p class="hm-lead">Your story is the most private thing you’ll ever write. OpenFrame keeps it where it belongs, with you.</p>
<ol class="hm-ticks">
<li><span><b>Your project is a folder on your computer</b>, or on an external drive you choose.</span></li>
<li><span><b>Works fully offline.</b> No sign-in and no internet needed to write, plan or export.</span></li>
<li><span><b>Nothing leaves unless you send it.</b> Share a script or call sheet as a file, the way you always have.</span></li>
<li><span><b>You’re never trapped.</b> Export, back up and move your whole project any time.</span></li>
</ol></div>
<div class="diagram hm-diagram"><svg viewBox="0 0 520 330" role="img" aria-label="Diagram: your project stays on your computer; the connection to outside servers is crossed out.">
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

<section class="hm-s hm-rule"><div class="hm-w">
<div class="hm-head"><span class="hm-eb">What makes OpenFrame different</span>
<h2>Built different, on purpose.</h2>
<p class="hm-lead">Six things you won’t find together in any other film production software.</p></div>
<ol class="hm-six">
<li><i aria-hidden="true">01</i><h3>Completely local</h3><p>Your scripts, ideas and schedules are files on your computer. Not rented space on someone else’s.</p></li>
<li><i aria-hidden="true">02</i><h3>Buy once, own forever</h3><p>No subscription. No renewal date. No “your plan has expired” in the middle of a shoot.</p></li>
<li><i aria-hidden="true">03</i><h3>The power of 5 apps in 1 app</h3><p>Ideas, story, screenplay, breakdown, schedule and call sheets, all talking to each other, so you never retype.</p></li>
<li class="trial"><i aria-hidden="true">04</i><span class="hm-badge">Risk-free</span><h3>Have trust issues? Good.</h3><p><b>Try it free for 7 days.</b> Judge it by using it, not by reading about it.</p><a class="btn btn-primary btn-sm" href="/download/#trial">How the free trial works</a></li>
<li><i aria-hidden="true">05</i><h3>Easy migration from other software</h3><p>Bring your existing scripts: Final Draft (.fdx), Fountain, PDF, DOCX or plain text. Preview before anything is imported.</p></li>
<li><i aria-hidden="true">06</i><h3>Works when the internet doesn’t</h3><p>Remote location, power cut, no signal: your film keeps moving forward.</p></li>
</ol></div></section>

<section class="hm-s"><div class="hm-w">
<div class="hm-head"><span class="hm-eb">Make it yours</span>
<h2>Help when you want it. Looks the way you like it.</h2>
<p class="hm-lead">Two optional extras. Neither gets in your way until you ask for it.</p></div>
<article class="hm-feat">
<div class="hm-feat-t"><span class="hm-eb">OpenFrame AI</span>
<h3>OpenFrame AI that helps. You stay in charge.</h3>
<p>Ask in plain language and get answers from your own project, with the sources shown. It changes nothing until you press Apply. It runs on your computer after a one-time download that you start, and if you do not want it, leave it off. OpenFrame works completely without it.</p>
<a class="more" href="/features/openframe-ai/">Meet OpenFrame AI &rarr;</a></div>
<div class="hm-feat-s"><div data-os-show="win">${mock('win', 'ai-proposed-changes', { link: false })}</div><div data-os-show="mac">${mock('mac', 'ai-panel-screenplay', { link: false })}</div></div>
</article>
<article class="hm-feat flip">
<div class="hm-feat-t"><span class="hm-eb">Themes</span>
<h3>Do not like the colours? Make them yours.</h3>
<p>Choose a theme in Light, Dark or System. Still not right? Paste a theme file, previewed before it is applied. Your screenplay and documents always keep their professional look.</p>
<a class="more" href="/themes/">Explore themes &rarr;</a></div>
<div class="hm-feat-s"><div data-os-show="win">${mock('win', 'settings-appearance', { link: false })}</div><div data-os-show="mac">${mock('mac', 'settings-appearance', { link: false })}</div></div>
</article>
</div></section>

<section class="hm-s hm-rule"><div class="hm-w">
<span class="hm-eb">Why we built it</span>
<p class="hm-quote">From <b>film lovers</b>, to <b>film lovers</b> across the world.</p>
<p class="hm-lead hm-mid">OpenFrame is made for people who make films with limited resources: writer-directors, student filmmakers, short-film makers and regional-language filmmakers.</p>
<div class="hm-langs" aria-label="Scripts you can write and export in"><span>English</span><span lang="te">తెలుగు</span><span lang="hi">हिन्दी</span><span lang="ta">தமிழ்</span><span lang="ml">മലയാളം</span><span lang="kn">ಕನ್ನಡ</span></div>
<h2 class="hm-promise-h">Your idea. Your film. <span>Our promise.</span></h2>
<ol class="hm-promise">
<li><i aria-hidden="true">01</i><h3>We never silently change your work</h3><p>OpenFrame suggests. You decide. Nothing rewrites your story, script or schedule on its own.</p></li>
<li><i aria-hidden="true">02</i><h3>Nothing is lost by accident</h3><p>Deleted items wait in Recently Deleted. Undo is offered after changes. Your work saves as you go.</p></li>
<li><i aria-hidden="true">03</i><h3>Leave any time, taking everything</h3><p>Every document exports. Your project is yours to back up, move and keep.</p></li>
</ol></div></section>

<section class="hm-s"><div class="hm-w hm-two hm-mig">
<div><span class="hm-eb">Easy migration</span>
<h2>Coming from other software? Bring your scripts.</h2>
<p class="hm-lead">See exactly what will be imported, and every warning, before anything is applied. Nothing is silently flattened or lost.</p>
<div class="hm-fmts"><span class="y">.fdx</span><span class="b">.fountain</span><span class="p">.pdf</span><span class="g">.docx</span><span class="o">.txt</span></div>
<p class="hm-more"><a class="btn btn-dark" href="/switch/">See how switching works &rarr;</a></p></div>
<div><div data-os-show="win">${mock('win', 'sp-import-preview', { caption: 'The import preview on Windows' })}</div><div data-os-show="mac">${mock('mac', 'screenplay-tools-import-screenplay', { caption: 'Import a screenplay on the Mac' })}</div></div>
</div></section>

<section class="hm-s hm-final"><div class="hm-w">
<span class="hm-eb">Ready when you are</span>
<h2>Have trust issues? Get a 7-day trial.</h2>
<p class="hm-lead">Install it, write a scene, schedule a shoot. If it isn’t for you, you’ve lost nothing.</p>
<div class="hm-stores">
<div class="hm-store"><span class="hm-tag y">Windows 11</span><h3>Get it on the Microsoft Store</h3><p>7-day free trial, then one payment of ${ctx.price()}. Pay once and it’s yours.</p>${ctx.storeBtn('win', 'btn-lg')}</div>
<div class="hm-store"><span class="hm-tag b">Mac</span><h3>Get it on the Mac App Store</h3><p>7-day free trial on Apple silicon Macs, then the same one-time price.</p>${ctx.storeBtn('mac', 'btn-lg')}</div>
</div>
<p class="hm-fine">The store sets the final checkout amount, including any taxes that apply where you live.</p>
</div></section>

</div>`;
  return [{
    path: '/', nav: 'home', title: 'OpenFrame Studio: film production software',
    description: 'One app for screenwriters and film teams: Idea Vault, Beat Board, screenplay editor, breakdown, schedule and call sheets. 7-day free trial, then pay once.',
    body,
    jsonld: [APP(ctx, 'Windows 11, macOS 14 or later'), ORG(ctx), WEBSITE(ctx)],
  }];
}
