// Definitions of the simulated Mac AI screens (see sim-mac.mjs). Words match the Windows AI screens; the project is the Mac capture project (Night Bus, LOST PROPERTY).
export const SIM_AREA = 'Straft AI';

export const TOKENS = {
  light: { fg: '#1d1d1f', muted: '#6e6e73', line: 'rgba(0,0,0,.11)', card: '#f6f6f8', user: '#2b2b2e', userFg: '#fff', ai: '#f1ede6', pill: 'rgba(179,84,30,.12)', ok: 'rgba(52,199,89,.17)', okFg: '#1e7d3a', field: '#fff', dim: 'rgba(0,0,0,.28)', sheet: '#fff', btn: 'rgba(0,0,0,.06)' },
  dark: { fg: '#f5f5f7', muted: '#a1a1a6', line: 'rgba(255,255,255,.16)', card: '#2a2a2c', user: '#48484a', userFg: '#fff', ai: '#2c2a28', pill: 'rgba(255,159,90,.18)', ok: 'rgba(48,209,88,.2)', okFg: '#5fdc7f', field: '#1e1e20', dim: 'rgba(0,0,0,.4)', sheet: '#2c2c2e', btn: 'rgba(255,255,255,.1)' },
};

const FONT = '-apple-system,BlinkMacSystemFont,"SF Pro Text","Helvetica Neue","Segoe UI",system-ui,sans-serif';
export const panelCss = (t, bg, hdr, acc) => `
.sim-panel,.sim-panel *{box-sizing:border-box}
.sim-panel{position:absolute;z-index:5;background:${bg};color:${t.fg};font:13px/1.38 ${FONT};overflow:hidden;padding:0 12px}
.sim-chip{position:absolute;z-index:6;background:${t.ok};color:${t.okFg};font:600 11px/17px ${FONT};padding:0 8px;border-radius:9px;white-space:nowrap}
.sp-top{display:flex;align-items:center;gap:8px;margin-top:12px}
.sp-l{font-size:11px;color:${t.muted}}
.sp-pop{flex:1;display:flex;justify-content:space-between;align-items:center;height:26px;padding:0 9px;border:1px solid ${t.line};border-radius:7px;background:${t.field};font-size:13px}
.sp-pop:after{content:"";width:6px;height:6px;border:solid ${t.muted};border-width:0 1.5px 1.5px 0;transform:rotate(45deg) translateY(-2px);margin-left:8px}
.sp-ico{width:22px;height:22px;display:grid;place-items:center;color:${t.muted}}
.sp-btn{font-size:12px;color:${t.fg};padding:3px 8px;border-radius:6px;background:${t.btn}}
.sp-using{font-size:11px;color:${t.muted};margin:7px 0 12px 2px}
.sp-hint{color:${t.muted};font-size:13px;margin:46px 4px 0;line-height:1.5}
.sp-u{display:flex;justify-content:flex-end;margin:0 0 10px}.sp-u span{background:${t.user};color:${t.userFg};padding:7px 12px;border-radius:14px 14px 4px 14px;max-width:88%}
.sp-a{background:${t.ai};border-radius:14px 14px 14px 4px;padding:10px 12px;margin:0 22px 10px 0}
.sp-a p{margin:0 0 6px}.sp-a p:last-child{margin:0}
.sp-src{font-size:11px;color:${t.fg};margin-top:6px}.sp-src:before{content:"\\25B8";margin-right:5px;font-size:9px}.sp-src.open:before{content:"\\25BE"}
.sp-inf{font-size:11px;color:${t.muted};margin-top:5px}
.sp-based{display:flex;flex-wrap:wrap;gap:5px;margin-top:6px;font-size:11px;color:${t.muted};align-items:center}
.sp-pill{background:${t.pill};color:${acc};padding:2px 9px;border-radius:11px;font-size:12px}
.sp-work{display:flex;align-items:center;gap:8px;color:${t.muted};font-size:12px;margin:2px 4px 8px}
.sp-dots{display:inline-flex;gap:3px}.sp-dots i{width:6px;height:6px;border-radius:50%;background:${acc};opacity:.8}.sp-dots i:nth-child(2){opacity:.5}.sp-dots i:nth-child(3){opacity:.25}
.sp-cancel{margin-left:auto;font-size:12px;color:${acc}}
.sp-card{border:1px solid ${t.line};border-radius:10px;background:${t.field};margin:0 0 10px;overflow:hidden}
.sp-card h4{margin:0;padding:8px 12px;font:600 12px/16px ${FONT};border-bottom:1px solid ${t.line}}
.sp-card .big{padding:7px 12px;border-bottom:1px solid ${t.line}}.sp-card .big b{display:block;font-size:13px}.sp-card .big small{font-size:12px;color:${t.muted}}
.sp-row{display:flex;justify-content:space-between;padding:6px 12px;border-bottom:1px solid ${t.line};font-size:12px}.sp-row b{font-weight:600}
.sp-note{padding:7px 12px;font-size:11px;color:${t.muted};line-height:1.4}
.sp-act{display:flex;align-items:center;gap:12px;margin:2px 0 8px 4px;font-size:13px}
.sp-apply{background:${acc};color:#fff;padding:5px 13px;border-radius:7px;font-weight:600;font-size:13px}
.sp-res{font-size:12px;color:${t.muted};margin:0 4px 8px}
.sp-comp{position:absolute;left:12px;right:12px;bottom:12px;height:58px;border:1px solid ${t.line};border-radius:10px;background:${t.field};padding:8px 10px;font-size:13px;color:${t.muted}}
.sp-comp.typed{color:${t.fg}}
.sp-send{position:absolute;right:8px;bottom:7px;padding:3px 12px;border-radius:7px;font-size:12px;background:${t.btn};color:${t.muted}}.sp-send.on{background:${acc};color:#fff}
.sp-menu{position:absolute;left:55px;top:38px;width:238px;background:${t.sheet};border:1px solid ${t.line};border-radius:9px;padding:5px;box-shadow:0 8px 24px rgba(0,0,0,.22);z-index:3}
.sp-menu div{padding:4px 10px 4px 24px;border-radius:5px;font-size:13px;position:relative}
.sp-menu .on{background:${acc};color:#fff}.sp-menu .on:before{content:"\\2713";position:absolute;left:8px}
`;

export const settingsCss = (t, bg, acc) => `
.sim-set,.sim-set *{box-sizing:border-box}
.sim-set{position:absolute;z-index:5;background:${bg};color:${t.fg};font:13px/1.4 ${FONT};overflow:hidden}
.ss-in{padding:20px 22px 0 22px}
.ss-card{background:${t.card};border-radius:12px;padding:16px 18px;margin-bottom:16px}
.ss-card h3{margin:0 0 6px;font:700 15px/20px ${FONT};display:flex;align-items:center;gap:10px}
.ss-chip{background:${t.ok};color:${t.okFg};font:600 11px/17px ${FONT};padding:0 8px;border-radius:9px}
.ss-p{font-size:13px;margin:0 0 10px}.ss-m{font-size:12px;color:${t.muted};margin:0 0 8px}
.ss-btns{display:flex;gap:10px;margin:12px 0}
.ss-b{display:inline-block;background:${t.btn};padding:4px 12px;border-radius:7px;font-size:13px}
.ss-seg{display:inline-flex;background:${t.btn};border-radius:8px;padding:2px;margin-left:10px;vertical-align:middle}.ss-seg span{padding:2px 14px;font-size:12px;border-radius:6px}.ss-seg .on{background:${t.sheet};box-shadow:0 1px 3px rgba(0,0,0,.25)}
.ss-tbl{width:100%;border-collapse:collapse;margin-top:6px}
.ss-tbl th{text-align:left;font:600 11px/16px ${FONT};color:${t.muted};padding:5px 6px;border-bottom:1px solid ${t.line}}
.ss-tbl td{padding:7px 6px;border-bottom:1px solid ${t.line};vertical-align:top;font-size:12px}
.ss-tbl small{display:block;color:${t.muted};font-size:11px}.ss-tbl code{display:block;font:10px/14px ui-monospace,Menlo,Consolas,monospace;color:${t.muted};word-break:break-all}
.ss-bar{position:absolute;right:4px;top:6px;width:7px;height:150px;border-radius:4px;background:rgba(128,128,128,.55)}
.ss-dim{position:absolute;inset:0;background:${t.dim};z-index:6}
.ss-sheet{position:absolute;z-index:7;left:190px;top:88px;width:380px;background:${t.sheet};border-radius:0 0 14px 14px;box-shadow:0 12px 36px rgba(0,0,0,.3);padding:20px 22px 16px;font-size:13px}
.ss-sheet h3{margin:0 0 6px;font:700 15px/20px ${FONT};text-align:center}.ss-sheet p{margin:0 0 14px;color:${t.muted};text-align:center;font-size:12px}
.ss-sb{display:flex;justify-content:flex-end;gap:10px}.ss-sb .go{background:#d92d20;color:#fff;padding:4px 16px;border-radius:7px;font-weight:600}.ss-sb .no{background:${t.btn};padding:4px 16px;border-radius:7px}
`;

// ---- panel parts ----
const scopeRow = (scope = 'Whole Project') => `<div class="sp-top"><span class="sp-l">Scope</span><span class="sp-pop">${scope}</span><span class="sp-ico"><svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M7 1v8M3.5 6 7 9.5 10.5 6M2 12.5h10"/></svg></span><span class="sp-btn">New</span></div><div class="sp-using">Using: ${scope}</div>`;
const HINT = `<p class="sp-hint">Ask about your project, for example “How many scenes are in the script?”, “Open Scene 12.” or “Create a task to scout the railway station.” Answers come from your project and say what they are based on; changes are only ever prepared for you to review.</p>`;
const comp = (text, on) => `<div class="sp-comp${text ? ' typed' : ''}">${text || 'Ask or command… e.g. “Open Scene 42.”'}<span class="sp-send${on ? ' on' : ''}">Send</span></div>`;
const user = (t) => `<div class="sp-u"><span>${t}</span></div>`;
const ai = (inner, { src = true, open = false, pills = [] } = {}) => `<div class="sp-a">${inner}${src ? `<div class="sp-src${open ? ' open' : ''}">See sources</div>` : ''}${pills.length ? `<div class="sp-inf">Inferred · written by Offline AI</div><div class="sp-based">Based on: ${pills.map((p) => `<span class="sp-pill">${p}</span>`).join('')}</div>` : ''}</div>`;
const proposal = (task, result) => `<div class="sp-card"><h4>Proposed Changes</h4><div class="big"><b>Proposed task</b><small>I prepared a task. Nothing has been added yet.</small></div>
<div class="sp-row"><span>Affected areas</span><b>Notes &amp; Tasks</b></div><div class="sp-row"><span>Task</span><b>${task}</b></div><div class="sp-row"><span>Impact</span><b>1 new object</b></div>
${result ? '' : '<div class="sp-note">1 change, applied together as one step you can undo. If any part fails, nothing is changed.</div>'}</div>
${result ? `<div class="sp-res">${result}</div>` : '<div class="sp-act"><span>Reject</span><span class="sp-apply">Apply Changes</span></div>'}`;
const panel = (inner, typed, on, scope) => () => `${scopeRow(scope)}${inner}${comp(typed, on)}`;

const deletion = `<div class="sp-card"><h4>Proposed Changes</h4><div class="big"><b>Proposed scene deletion</b><small>I prepared this change. Nothing has been changed yet.</small></div>
<div class="sp-row"><span>Affected areas</span><b>Screenplay</b></div><div class="sp-row"><span>Delete</span><b>Scene 1</b></div><div class="sp-row"><span>Its text</span><b>16 lines</b></div><div class="sp-row"><span>Other drafts</span><b>Not changed</b></div>
<div class="sp-note">1 change, applied together as one step you can undo. If any part fails, nothing is changed.</div></div><div class="sp-act"><span>Reject</span><span class="sp-apply">Apply Changes</span></div>`;
const SCOPE_MENU = () => `<div class="sp-menu"><div>Current Screenplay</div><div>Beat Board</div><div>Production Workspace</div><div class="on">Whole Project</div></div>`;

// ---- Offline AI pane ----
const offlineCard = (gfx) => `<div class="ss-card"><h3>Offline AI <span class="ss-chip">AI Ready</span></h3>
<p class="ss-p">Straft AI runs on this computer after a one-time download. Your project data is not sent to Straft or to an AI cloud service.</p>
<div class="ss-btns"><span class="ss-b">Stop Offline AI (free memory)</span><span class="ss-b">Remove Straft AI…</span></div>
<p class="ss-p"><b>Graphics</b><span class="ss-seg"><span class="${gfx ? '' : 'on'}">Automatic</span><span class="${gfx ? 'on' : ''}">Off</span></span></p>
<p class="ss-m">Decided before the first answer. Automatic uses the graphics processor of this Mac only after it passes a short check; the main processor is always the fallback.</p>
<p class="ss-m" style="margin:0">Straft works normally without Straft AI.</p></div>`;
const healthCard = `<div class="ss-card"><h3>AI health</h3><p class="ss-p" style="opacity:.75">No requests yet. The figures appear after you use the AI Assistant.</p>
<p class="ss-m">Only numbers are kept on this computer: no questions, answers or project text. Nothing is sent. Export AI report saves them to a file you choose, to share if you want to.</p><span class="ss-b">Export AI report…</span></div>`;
const memoryCard = `<div class="ss-card"><h3>Assistant memory</h3><p class="ss-p" style="opacity:.75">Nothing is remembered yet. The assistant keeps a fact or preference only when you state it outright, or confirm one of its answers.</p>
<p class="ss-m" style="margin:0">Saved on this Mac, in this project, for you only, and only when you state a fact or confirm an answer. Delete an entry and the assistant forgets it.</p></div>`;
const detailsCard = `<div class="ss-card"><h3>▾ Technical details</h3><table class="ss-tbl"><tr><th>Component</th><th>Licence</th><th>Size</th></tr>
<tr><td><b>AI engine</b>: llama.cpp server (Metal)<small>Version b6259</small><code>a671fe6f24b83a01e46b61b2845edc0a6c05591288d7110abb8e35bfdda2fccd</code></td><td>MIT</td><td>42 MB</td></tr>
<tr><td><b>Language model</b>: Gemma 3 1B Instruct (Q8_0)<small>Version f9c28bc</small><code>b205840c5dcef55078e37d344677869a714ffd42a4ae448c48dcfb52e4bb10d5</code></td><td>Gemma-Terms-of-Use</td><td>1020 MB</td></tr>
<tr><td><b>Search model</b>: BGE small English v1.5 (Q8_0)<small>Version f2068ed</small></td><td>MIT</td><td>35 MB</td></tr>
<tr><td><b>Search model</b>: Multilingual E5 small (Q8_0)<small>Version b6cac96</small></td><td>MIT</td><td>127 MB</td></tr></table>
<p class="ss-m" style="margin:8px 0 0">Package: straft-local-ai-v1 v1.0.0 · files: checked before the first answer<br>This computer: 16 GB memory · Apple silicon · engine and models: 1.3 GB<br>Everything runs on this computer. Prompts, project text and answers are never sent anywhere. The only network use is the model download you start.</p></div>`;
const downloadCard = `<div class="ss-card"><h3>Offline AI <span class="ss-chip" style="background:rgba(255,159,10,.2);color:#b25e00">Downloading</span></h3>
<p class="ss-p">Straft AI runs on this computer after a one-time download. Your project data is not sent to Straft or to an AI cloud service.</p>
<p class="ss-m" style="margin-bottom:4px">Downloading the language model: 436 MB of 1020 MB</p><div style="height:6px;border-radius:3px;background:rgba(128,128,128,.25);overflow:hidden;margin-bottom:12px"><div style="width:43%;height:100%;background:#0a64d8"></div></div>
<div class="ss-btns" style="margin-top:0"><span class="ss-b">Cancel download</span><span class="ss-b">Import model files…</span></div>
<p class="ss-m" style="margin:0">Straft works normally without Straft AI. The files are checked before the first answer.</p></div>`;
const pane = (inner, scroll = 0, bar = 0) => () => `<div class="ss-in" style="transform:translateY(-${scroll}px)">${inner}</div><span class="ss-bar" style="top:${6 + bar}px"></span>`;
const REMOVE = () => `<div class="ss-dim"></div><div class="ss-sheet"><h3>Remove Straft AI?</h3><p>This deletes the downloaded engine and models from this Mac (about 1.3 GB). Your projects are not changed. You can download Offline AI again at any time.</p><div class="ss-sb"><span class="no">Cancel</span><span class="go">Remove</span></div></div>`;

const T = (...a) => a;
export const SIMS = [
  { name: 'ai-sim-beside-vault', title: 'Straft AI open beside Idea Vault', kind: 'panel', base: 'ai-panel-idea-vault', panel: panel(HINT), texts: T('AI Assistant', 'AI Ready', 'Scope', 'Whole Project', 'New', 'Send') },
  { name: 'ai-sim-beside-story', title: 'Straft AI open beside Story', kind: 'panel', base: 'ai-panel-story', panel: panel(HINT), texts: T('AI Assistant', 'AI Ready', 'Scope', 'Whole Project', 'New', 'Send') },
  { name: 'ai-sim-beside-screenplay', title: 'Straft AI open beside Screenplay', kind: 'panel', base: 'ai-panel-screenplay', panel: panel(HINT), texts: T('AI Assistant', 'AI Ready', 'Scope', 'Whole Project', 'New', 'Send') },
  { name: 'ai-sim-beside-breakdown', title: 'Straft AI open beside Breakdown', kind: 'panel', base: 'ai-panel-breakdown', panel: panel(HINT), texts: T('AI Assistant', 'AI Ready', 'Scope', 'Whole Project', 'New', 'Send') },
  { name: 'ai-sim-beside-production', title: 'Straft AI open beside Production', kind: 'panel', base: 'ai-panel-production', panel: panel(HINT), texts: T('AI Assistant', 'AI Ready', 'Scope', 'Whole Project', 'New', 'Send') },
  { name: 'ai-sim-answer-delete-request', title: 'Straft AI asked to delete, waiting for Apply', kind: 'panel', base: 'ai-panel-screenplay', panel: panel(`${user('Delete all the scenes in the screenplay.')}${ai('I prepared this change. Nothing has been changed yet.')}${deletion}`), texts: T('AI Assistant', 'Delete all the scenes in the screenplay.', 'I prepared this change. Nothing has been changed yet.', 'Proposed scene deletion', 'Affected areas', 'Screenplay', 'Its text', 'Other drafts', 'Not changed', 'Reject', 'Apply Changes') },
  { name: 'ai-sim-offline-ai-download', title: 'Settings, Offline AI, one-time download', kind: 'settings', base: 'settings-offline-ai', panel: pane(`${downloadCard}${healthCard}`), texts: T('Offline AI', 'Downloading', 'Downloading the language model: 436 MB of 1020 MB', 'Cancel download', 'Import model files…') },
  { name: 'ai-sim-offline-ai-health', title: 'Settings, Offline AI, AI health', kind: 'settings', base: 'settings-offline-ai', panel: pane(`${offlineCard(false)}${healthCard}${memoryCard}`, 262, 90), texts: T('AI health', 'No requests yet. The figures appear after you use the AI Assistant.', 'Export AI report…', 'Assistant memory') },
  { name: 'ai-sim-ready', title: 'Straft AI, ready to ask', kind: 'panel', base: 'ai-panel', panel: panel(HINT), texts: T('AI Assistant', 'AI Ready', 'Scope', 'Whole Project', 'New', 'Ask about your project', 'Send') },
  { name: 'ai-sim-question-typed', title: 'Straft AI, a question typed', kind: 'panel', base: 'ai-panel', panel: panel(HINT, 'How many scenes are in the script?', true), texts: T('AI Assistant', 'AI Ready', 'How many scenes are in the script?', 'Send') },
  { name: 'ai-sim-working', title: 'Straft AI working on an answer', kind: 'panel', base: 'ai-panel', panel: panel(`${user('List the characters.')}<div class="sp-work"><span class="sp-dots"><i></i><i></i><i></i></span>Writing the answer<span class="sp-cancel">Cancel</span></div>`), texts: T('AI Assistant', 'List the characters.', 'Writing the answer', 'Cancel') },
  { name: 'ai-sim-answer-characters', title: 'Straft AI answers with its sources', kind: 'panel', base: 'ai-panel', panel: panel(`${user('List the characters.')}${ai('Devi is the Lead character. Kiran is a Supporting character.', { pills: ['Character: DEVI', 'Character: KIRAN'] })}`), texts: T('AI Assistant', 'List the characters.', 'Devi is the Lead character. Kiran is a Supporting character.', 'See sources', 'Inferred · written by Offline AI', 'Based on:', 'Character: DEVI', 'Character: KIRAN') },
  { name: 'ai-sim-answer-scene-count', title: 'Straft AI counts the scenes', kind: 'panel', base: 'ai-panel-screenplay', panel: panel(`${user('How many scenes are in the script?')}${ai('LOST PROPERTY has 10 scenes.', { pills: ['Screenplay: LOST PROPERTY'] })}`, '', false, 'Current Screenplay'), texts: T('AI Assistant', 'How many scenes are in the script?', 'LOST PROPERTY has 10 scenes.', 'Based on:', 'Screenplay: LOST PROPERTY') },
  { name: 'ai-sim-answer-sources-open', title: 'Straft AI answer with its sources open', kind: 'panel', base: 'ai-panel-screenplay', panel: panel(`${user('Which locations are used in the screenplay?')}${ai('<p>The screenplay uses these locations:</p><p>Kesarganj Junction: the Lost Property Office, Platform 3, the Store Room and the Footbridge. One scene is on the night train, in a second class compartment.</p>', { open: true, pills: ['Scene locations: LOST PROPERTY', 'Location: Kesarganj Junction', 'Location: Night Train'] })}`, '', false, 'Current Screenplay'), texts: T('AI Assistant', 'Which locations are used in the screenplay?', 'See sources', 'Inferred · written by Offline AI', 'Based on:', 'Scene locations: LOST PROPERTY', 'Location: Kesarganj Junction', 'Location: Night Train') },
  { name: 'ai-sim-scope-menu', title: 'Straft AI, choosing what it looks at', kind: 'panel', base: 'ai-panel-screenplay', panel: () => `${scopeRow('Current Screenplay')}${HINT}${comp('', false)}${SCOPE_MENU()}`, texts: T('AI Assistant', 'Scope', 'Current Screenplay', 'Beat Board', 'Production Workspace', 'Whole Project', 'Using: Current Screenplay') },
  { name: 'ai-sim-proposed-changes', title: 'Straft AI proposes a change', kind: 'panel', base: 'ai-panel-screenplay', panel: panel(`${user('Create a task to scout the footbridge.')}${ai('I prepared a task. Nothing has been added yet.')}${proposal('Scout the footbridge')}`), texts: T('AI Assistant', 'Create a task to scout the footbridge.', 'I prepared a task. Nothing has been added yet.', 'Proposed Changes', 'Proposed task', 'Affected areas', 'Notes & Tasks', 'Task', 'Scout the footbridge', 'Impact', '1 new object', 'Reject', 'Apply Changes') },
  { name: 'ai-sim-proposal-applied', title: 'Straft AI change applied', kind: 'panel', base: 'ai-panel-screenplay', panel: panel(`${user('Create a task to ask the station for a footbridge permit.')}${ai('I prepared a task. Nothing has been added yet.')}${proposal('Ask the station for a footbridge permit', 'Applied 1 change. Use Undo to reverse it.')}`), texts: T('AI Assistant', 'Proposed Changes', 'Notes & Tasks', 'Ask the station for a footbridge permit', 'Applied 1 change. Use Undo to reverse it.') },
  { name: 'ai-sim-proposal-rejected', title: 'Straft AI change rejected', kind: 'panel', base: 'ai-panel-screenplay', panel: panel(`${user('Create a task to scout the footbridge.')}${ai('I prepared a task. Nothing has been added yet.')}${proposal('Scout the footbridge', 'Rejected. Nothing was changed.')}`), texts: T('AI Assistant', 'Proposed Changes', 'Notes & Tasks', 'Scout the footbridge', 'Rejected. Nothing was changed.') },
  { name: 'ai-sim-offline-ai-ready', title: 'Settings, Offline AI, running', kind: 'settings', base: 'settings-offline-ai', panel: pane(`${offlineCard(false)}${healthCard}${memoryCard}`), texts: T('Offline AI', 'AI Ready', 'Stop Offline AI (free memory)', 'Remove Straft AI…', 'Graphics', 'Automatic', 'Off', 'AI health', 'Assistant memory') },
  { name: 'ai-sim-offline-ai-technical-details', title: 'Settings, Offline AI, technical details', kind: 'settings', base: 'settings-offline-ai', panel: pane(`${offlineCard(false)}${detailsCard}`, 176, 60), texts: T('Offline AI', 'Technical details', 'Component', 'Licence', 'Size', 'AI engine: llama.cpp server (Metal)', 'Language model: Gemma 3 1B Instruct (Q8_0)', 'Search model: BGE small English v1.5 (Q8_0)', 'Search model: Multilingual E5 small (Q8_0)') },
  { name: 'ai-sim-offline-ai-graphics-off', title: 'Settings, Offline AI, graphics set to Off', kind: 'settings', base: 'settings-offline-ai', panel: pane(`${offlineCard(true)}${healthCard}`), texts: T('Offline AI', 'AI Ready', 'Graphics', 'Automatic', 'Off', 'Decided before the first answer.') },
  { name: 'ai-sim-offline-ai-remove-confirm', title: 'Settings, Offline AI, remove confirmation', kind: 'settings', base: 'settings-offline-ai', panel: pane(`${offlineCard(false)}${healthCard}`), over: REMOVE, texts: T('Remove Straft AI?', 'Cancel', 'Remove') },
];
