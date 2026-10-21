// Home page (/). Status words follow research/facts.json: internal team testing, store release planned, prices are targets.
import { pairShot, tutorialsFor } from './features.mjs';

const HERO = { win: '013-shell-project-home', mac: '035-project-home' };
const STAGES = [
  ['ideate', 'Ideate', 'Collect everything before it has a shape.', 'Notes, images, links, files and voice notes go into the Idea Vault. Shape them into acts, beats and scene cards on the Beat Board.', '/features/idea-vault/', { win: '151-vault-grid-all-types', mac: '259-vault-filled' }],
  ['write', 'Write', 'Industry formatting while you type.', 'Eight screenplay elements, smart suggestions, drafts, comments and locked revisions. Import Final Draft, Fountain, Word or PDF; export the same.', '/features/screenplay/', { win: '244-sp-editor-standard', mac: '068-screenplay-editor-typing' }],
  ['plan', 'Plan', 'Break it down and build the schedule.', 'Tag every scene, keep cast, crew and locations, then drag scenes into shooting days on a stripboard that warns you about conflicts.', '/features/scheduling/', { win: '533-prod-schedule-board', mac: '366-schedule-board' }],
  ['shoot', 'Shoot', 'Call sheets and sides from the schedule.', 'Create a call sheet from a shooting day, add sides of that day\'s scenes, and keep reports and a light budget beside them.', '/features/call-sheets/', { win: '645-cs-editor-draft', mac: '420-callsheet-editor' }],
];
const MIGRATE = { win: '011-sp-import-preview', mac: '078-screenplay-tools-import-screenplay' };

export const SCREENS = {
  win: [HERO.win, ...STAGES.map((s) => s[5].win), MIGRATE.win],
  mac: [HERO.mac, ...STAGES.map((s) => s[5].mac), MIGRATE.mac],
};

export default function (ctx) {
  const { html, ICON } = ctx;
  // Only tutorials that have a built page are linked.
  const startList = (plat) => {
    const base = plat === 'win' ? '/windows/tutorials/' : '/mac/tutorials/';
    return html`<ul class="mk-learn">${tutorialsFor(ctx, plat, ['first-launch', 'create-project']).map((t) => html`
      <li><a href="${base}${t.id}/">${t.title || t.id}${ICON.arrow}</a></li>`)}
      <li><a href="${base}">All ${plat === 'win' ? 'Windows' : 'macOS'} tutorials${ICON.arrow}</a></li></ul>`;
  };
  const DESC = 'One local-first app for writers, screenwriters and production teams: ideas, story, screenplay, breakdown, schedule, call sheets. Windows 11 and Apple silicon.';

  const body = html`
<section class="mk-hero">
  <div class="wrap">
    <h1 class="mk-hero-title"><span>Ideate.</span> <span>Write.</span> <span>Plan.</span> <span>Shoot.</span></h1>
    <p class="mk-hero-lead">OpenFrame Studio is one desktop app for writers, screenwriters and production teams. Your ideas, story, screenplay, breakdown, schedule and call sheets live in one project on your own computer.</p>
    <p class="mk-hero-actions">
      <a class="btn btn-primary btn-lg" href="/features/">See what it does</a>
      <a class="btn btn-ghost btn-lg" href="/download/">Where it stands today</a>
    </p>
    <p class="mk-hero-meta">${ICON.windows} Windows 11 <span aria-hidden="true">·</span> ${ICON.apple} macOS 14 or later on Apple silicon <span aria-hidden="true">·</span> In team testing now</p>
    <div class="mk-hero-shot">${pairShot(ctx, HERO, 'Project home with every workspace in the sidebar', { eager: true })}</div>
  </div>
</section>

<section class="section">
  <div class="wrap">
    <div class="sec-head">
      <h2>Made for the people who make the film.</h2>
      <p class="lead">Small productions spread the work across a notes app, an outliner, screenwriting software, a scheduling tool and a pile of spreadsheets. OpenFrame Studio keeps it in one place, and each part reads from the one before it.</p>
    </div>
    <div class="mk-who">
      <div>
        <h3>Writers</h3>
        <p>Catch ideas the moment they arrive and shape them into a story you can see. A scene card becomes a scene with one click and a preview.</p>
        <a href="/features/idea-vault/">Idea Vault and Beat Board ${ICON.arrow}</a>
      </div>
      <div>
        <h3>Screenwriters</h3>
        <p>Proper screenplay format as you type, named drafts, side-by-side compare, locked shooting drafts and coloured revisions.</p>
        <a href="/features/screenplay/">Screenplay Editor ${ICON.arrow}</a>
      </div>
      <div>
        <h3>Production teams</h3>
        <p>Break down the locked script, build the schedule, and hand out call sheets and sides that follow every change to the plan.</p>
        <a href="/features/breakdown/">Breakdown to call sheet ${ICON.arrow}</a>
      </div>
    </div>
  </div>
</section>

<section class="section tint">
  <div class="wrap">
    <div class="sec-head">
      <h2>One project, from first idea to call sheet.</h2>
      <p class="lead">These are real screens from the app on your platform.</p>
    </div>
    <div class="mk-stages">
      ${STAGES.map(([id], n) => html`<input class="mk-stage-in" type="radio" name="mk-stage" id="mk-st-${id}"${n === 0 ? ctx.raw(' checked') : ''}>`)}
      <div class="mk-stage-tabs">${STAGES.map(([id, label]) => html`<label for="mk-st-${id}">${label}</label>`)}</div>
      <div class="mk-stage-panels">${STAGES.map(([id, label, head, text, href, shots]) => html`
        <div class="mk-stage-panel mk-p-${id}">
          <div class="mk-stage-text">
            <h3><span class="mk-stage-word">${label}.</span> ${head}</h3>
            <p>${text}</p>
            <a href="${href}">Learn how to ${label.toLowerCase()} in OpenFrame ${ICON.arrow}</a>
          </div>
          <div class="mk-stage-shot">${pairShot(ctx, shots, `${label}: ${head}`)}</div>
        </div>`)}
      </div>
    </div>
  </div>
</section>

<section class="section">
  <div class="wrap mk-five">
    <div class="mk-five-text">
      <h2>Five tools become one.</h2>
      <p class="lead">Why keep five apps in step by hand? In OpenFrame Studio the scene you write is the scene you tag, schedule and print on the call sheet.</p>
      <p>The plan is a one-time purchase, not a subscription. Prices are not set yet.</p>
    </div>
    <div class="mk-five-diagram" role="img" aria-label="Five separate tools, a notes app, an outliner, screenwriting software, a scheduler and a call sheet tool, replaced by one OpenFrame Studio project.">
      <ul class="mk-five-before" aria-hidden="true">
        <li>Notes and ideas app</li>
        <li>Story outlining tool</li>
        <li>Screenwriting software</li>
        <li>Breakdown and scheduling</li>
        <li>Call sheet spreadsheets</li>
      </ul>
      <span class="mk-five-arrow" aria-hidden="true">${ICON.arrow}</span>
      <div class="mk-five-after" aria-hidden="true">
        <strong>${ctx.MARK} OpenFrame Studio</strong>
        <span>Idea Vault</span><span>Beat Board</span><span>Screenplay</span><span>Breakdown and Schedule</span><span>Call Sheets</span>
      </div>
    </div>
  </div>
</section>

<section class="section dark mk-manifesto">
  <div class="wrap narrow">
    <h2>The tools to tell your story should belong to you.</h2>
    <p>Too much creative software is rented by the month and kept on someone else's servers. OpenFrame Studio is a desktop app. Your film is a folder on your computer, it opens without the internet, and it does not ask you to sign in.</p>
    <ul class="mk-manifesto-list">
      <li>No mandatory cloud.</li>
      <li>No account.</li>
      <li>Planned as a one-time purchase.</li>
    </ul>
  </div>
</section>

<section class="section">
  <div class="wrap mk-privacy">
    <div>
      <h2>Your privacy stays on your computer.</h2>
      <p class="lead">Your story is the most private thing you will write. OpenFrame Studio keeps it where it belongs.</p>
      <ul class="ticks">
        <li>Each project is a folder on your computer, or on a drive you choose.</li>
        <li>Works fully offline. No account, no sign-in, no telemetry, no update check.</li>
        <li>Nothing leaves unless you send it. Share a script or a call sheet as a file.</li>
        <li>The only download the app makes is the optional assistant, and only when you start it.</li>
      </ul>
      <a href="/local-and-private/">How your data is handled ${ICON.arrow}</a>
    </div>
    <div class="mk-privacy-diagram" role="img" aria-label="Your project lives on your computer. No OpenFrame server is involved.">
      <div class="mk-pd-local" aria-hidden="true">
        ${ICON.laptop}
        <strong>Your computer</strong>
        <span class="mk-pd-folder">${ICON.folder} Night Bus.openframe</span>
        <span>Ideas, story, screenplay, schedule, call sheets</span>
      </div>
      <div class="mk-pd-gap" aria-hidden="true">${ICON.offline}<span>Nothing sent</span></div>
      <div class="mk-pd-remote" aria-hidden="true">
        <strong>Our servers</strong>
        <span>None. There is no OpenFrame cloud.</span>
      </div>
    </div>
  </div>
</section>

<section class="section tint">
  <div class="wrap">
    <div class="sec-head">
      <h2>Our promises to you.</h2>
    </div>
    <div class="mk-promises">
      <div>
        <h3>We never silently change your work</h3>
        <p>OpenFrame suggests; you decide. Breakdown suggestions, imports and the optional assistant all show you the change first, and nothing happens until you apply it.</p>
      </div>
      <div>
        <h3>Nothing is lost by accident</h3>
        <p>Autosave is always on. Undo works across the project. Deleted items wait 30 days in Recently Deleted. <a href="/features/backups-recovery/">Backups and recovery</a></p>
      </div>
      <div>
        <h3>Leave any time, with everything</h3>
        <p>Scripts export to PDF, Final Draft, Fountain, Word and text. Production documents export to PDF, CSV and Excel. Back up or move the whole project as one file.</p>
      </div>
      <div>
        <h3>Works when the internet does not</h3>
        <p>Remote location, power cut, no signal. Writing, planning and exporting never need a connection.</p>
      </div>
    </div>
  </div>
</section>

<section class="section">
  <div class="wrap mk-row">
    <div class="mk-row-text">
      <h2>Coming from other software? Bring your scripts.</h2>
      <p>See what will be imported, and every warning, before anything changes.</p>
      <p class="mk-formats"><span class="pill">.fdx</span> <span class="pill">.fountain</span> <span class="pill">.docx</span> <span class="pill">.pdf</span> <span class="pill">.txt</span></p>
      <p class="mk-small">PDF import is best effort. Older formats such as .doc, .rtf and .celtx are refused with a clear message; save them as .fdx, .fountain or .docx first.</p>
      <a href="/switch/">How switching works ${ICON.arrow}</a>
    </div>
    <div class="mk-row-shot">${pairShot(ctx, MIGRATE, 'Import Screenplay with a preview before anything changes')}</div>
  </div>
</section>

<section class="section tint">
  <div class="wrap mk-glance">
    <div>
      <h2>At a glance</h2>
      <dl class="kv">
        <dt>What it is</dt><dd>A local-first desktop app for writers, screenwriters and production teams: Idea Vault, Beat Board, screenplay editor, breakdown, cast and locations, shot lists, storyboards, schedule, call sheets, sides, reports and budget.</dd>
        <dt>Platforms</dt><dd>Windows 11 (64-bit). macOS 14 or later on Apple silicon Macs.</dd>
        <dt>Status</dt><dd>In team testing as unsigned internal builds. Public release is planned on the Microsoft Store and the Mac App Store.</dd>
        <dt>Price model</dt><dd>Planned as a one-time purchase with a 7-day free trial in the stores. Target prices: 3,000 rupees, 89 US dollars, 89 euros. Not final.</dd>
        <dt>AI</dt><dd>Optional. Runs offline on your computer after a one-time download of about 1.2 GB. It suggests; nothing changes until you apply it.</dd>
        <dt>Your data</dt><dd>Stored on your computer as a project folder. No account. No telemetry.</dd>
        <dt>Languages</dt><dd>App screens in English. Write in any language; screenplay PDF supports Latin, Hindi, Telugu, Tamil, Kannada and Malayalam scripts.</dd>
      </dl>
    </div>
    <div class="mk-start">
      <h2>How to start</h2>
      <p>There is no public download yet. While the team tests the app, you can learn every workflow from the tutorials, made from the real screens.</p>
      ${ctx.osBlocks({ win: startList('win'), mac: startList('mac') })}
      <p><a class="btn btn-dark" href="/download/">Download status</a></p>
    </div>
  </div>
</section>
${ctx.cta()}`;

  const url = (ctx.SITE_URL || 'https://openframe-studio.pages.dev').replace(/\/$/, '') + '/';
  return [{
    path: '/',
    title: 'OpenFrame Studio: ideas to call sheets in one app',
    description: DESC,
    body,
    nav: null,
    os: null,
    alternates: null,
    breadcrumbs: [],
    ogImage: `/screens/win/${HERO.win}-1200.jpg`,
    jsonld: [
      {
        '@context': 'https://schema.org',
        '@type': 'SoftwareApplication',
        name: 'OpenFrame Studio',
        applicationCategory: 'MultimediaApplication',
        operatingSystem: 'Windows 11, macOS 14 or later (Apple silicon)',
        description: DESC,
        url,
      },
      { '@context': 'https://schema.org', '@type': 'WebSite', name: 'OpenFrame Studio', url },
      { '@context': 'https://schema.org', '@type': 'Organization', name: 'OpenFrame Studio' },
    ],
  }];
}
