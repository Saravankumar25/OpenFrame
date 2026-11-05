// Definitions of the simulated Mac AI screens (see sim-mac.mjs). Structure and words come from the Mac source (StraftUI/AI, StraftUI/Settings/AISettingsView.swift,
// the core's ai module); colours are the Mac theme (macDefaultTheme.json over themes.json). The project is the Mac capture project (Night Bus, LOST PROPERTY).
export const SIM_AREA = 'Straft AI';

export const TOKENS = {
  light: { bg: '#f4f6f9', panel: '#ffffff', surface2: '#f9fafb', ink: '#141922', ink2: '#3a4252', muted: '#5f687a', line: '#e0e4ea', line2: '#cdd3dc', accent: '#e08a1e', accentD: '#9c5a0a', accentSoft: '#fcefdc', accentInk: '#1a1206', tint: '#a65b0a', placeholder: '#8b94a6', blueSoft: '#e6eefc', blueT: '#22499a', greenSoft: '#e0f3e8', greenT: '#1c6440', redSoft: '#fbe6e3', redT: '#8f2b22', yellowSoft: '#fcf2d2', yellowT: '#7a570b', btn: 'rgba(0,0,0,.063)', def: '#000000', defFg: '#ffffff', chipPlain: 'rgba(0,0,0,.17)', field: '#ffffff', dimColor: { r: 159, g: 159, b: 159, alpha: 0.68 }, dimCss: 'rgba(159,159,159,.68)', alert: '#f2f2f3', shadow: 'rgba(0,0,0,.3)' },
  dark: { bg: '#12151b', panel: '#191d25', surface2: '#161a21', ink: '#e9ecf2', ink2: '#c0c7d3', muted: '#8b94a6', line: '#262c37', line2: '#353c49', accent: '#f0a040', accentD: '#f5b362', accentSoft: '#3a2a14', accentInk: '#1a1206', tint: '#b8650f', placeholder: '#6b7487', blueSoft: '#1b2943', blueT: '#afc8f7', greenSoft: '#152f20', greenT: '#98dfba', redSoft: '#3b1d1b', redT: '#f6b0a9', yellowSoft: '#33290e', yellowT: '#f2d88f', btn: 'rgba(255,255,255,.05)', def: '#ffffff', defFg: '#141922', chipPlain: 'rgba(255,255,255,.2)', field: 'rgba(255,255,255,.04)', dimColor: { r: 15, g: 15, b: 16, alpha: 0.55 }, dimCss: 'rgba(15,15,16,.55)', alert: '#2a2e36', shadow: 'rgba(0,0,0,.55)' },
};

const FONT = '-apple-system,BlinkMacSystemFont,"SF Pro Text","Helvetica Neue","Segoe UI",system-ui,sans-serif';
export const svg = (inner, s = 14) => `<svg width="${s}" height="${s}" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${inner}</svg>`;
export const ICON = {
  stack: svg('<path d="M8 2l6 3-6 3-6-3z"/><path d="M2 8l6 3 6-3"/><path d="M2 11l6 3 6-3"/>'),
  save: svg('<path d="M8 2v8M5 7l3 3 3-3M3 10v3.5h10V10"/>'),
  bubble: svg('<path d="M2.5 3h11v8H7l-3 2.5V11H2.5z"/><path d="M5 6h6M5 8.2h4"/>', 13),
  copy: svg('<rect x="5.5" y="5.5" width="8" height="8" rx="1.5"/><path d="M10.5 3.5v-1h-8v8h1"/>', 11),
  right: svg('<path d="M6 3.5l4.5 4.5L6 12.5"/>', 10),
  down: svg('<path d="M3.5 6L8 10.5 12.5 6"/>', 10),
  left: svg('<path d="M10 3.5L5.5 8l4.5 4.5"/>', 13),
  remove: svg('<circle cx="8" cy="8" r="6.5" fill="currentColor" stroke="none"/><path d="M5.8 5.8l4.4 4.4M10.2 5.8l-4.4 4.4" stroke="var(--sim-bg)"/>', 11),
  picked: svg('<circle cx="8" cy="8" r="6.8" fill="currentColor" stroke="none"/><path d="M5 8.2l2.1 2.1L11 6" stroke="var(--sim-bg)" stroke-width="1.6"/>', 14),
  empty: svg('<circle cx="8" cy="8" r="6.3"/>', 14),
  info: svg('<circle cx="8" cy="8" r="6.3"/><path d="M8 7.2v3.8M8 4.8v.2"/>', 13),
  warn: svg('<path d="M8 2.2l6.3 11H1.7z"/><path d="M8 6.5v3.2M8 11.6v.1"/>', 13),
  close: svg('<path d="M3.5 3.5l9 9M12.5 3.5l-9 9"/>', 12),
  bolt: svg('<path d="M9 1.5L3.5 9H8l-1 5.5L12.5 7H8z" fill="currentColor"/>', 12),
  doc: svg('<path d="M4 1.8h5.2L12.5 5v9.2H4z"/><path d="M9 1.8V5h3.5"/>', 22),
  clock: svg('<path d="M3 8a5.3 5.3 0 1 0 1.7-3.9L3 5.6"/><path d="M3 2.8v2.8h2.8M8 5.2V8.3l2 1.3"/>', 13),
  updown: svg('<path d="M5 6.2L8 3.5l3 2.7M5 9.8l3 2.7 3-2.7"/>', 11),
  spin: `<svg width="14" height="14" viewBox="0 0 16 16" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" aria-hidden="true">${[...Array(8).keys()].map((i) => `<path d="M8 1.5V4.2" opacity="${(0.25 + i * 0.1).toFixed(2)}" transform="rotate(${i * 45} 8 8)"/>`).join('')}</svg>`,
};

// the controls every simulated screen shares: push button, prominent, default, borderless, chip, banner
export const controlsCss = (t) => `
.sim-root,.sim-root *{box-sizing:border-box}
.sim-root{--sim-bg:${t.panel};position:absolute;color:${t.ink};font:13px/16px ${FONT};white-space:normal;text-align:left}
.m-b{display:inline-flex;align-items:center;justify-content:center;gap:5px;height:24px;padding:0 12px;border-radius:7px;background:${t.btn};color:${t.ink};white-space:nowrap}
.m-b.sm{height:20px;padding:0 9px;border-radius:6px;font-size:12px}
.m-b.go{background:${t.accent};color:${t.accentInk}}
.m-b.def{background:${t.def};color:${t.defFg}}
.m-b.off{opacity:.45}
.m-link{display:inline-flex;align-items:center;gap:4px;color:${t.accentD};white-space:nowrap}
.m-link.off{opacity:.4}
.m-chip{display:inline-flex;align-items:center;gap:3px;height:17px;padding:0 7px;border-radius:9px;font:500 10px/17px ${FONT};white-space:nowrap}
.m-chip.green{background:${t.greenSoft};color:${t.greenT}}
.m-chip.red{background:${t.redSoft};color:${t.redT}}
.m-chip.plain{background:${t.btn};color:${t.ink2}}
.m-ban{display:flex;align-items:flex-start;gap:8px;padding:8px 10px;border-radius:8px;font-size:12px;line-height:16px}
.m-ban svg{flex:none;margin-top:1px}.m-ban .m-grow{flex:1}.m-ban .m-b{flex:none;margin:-1px 0}
.m-ban.info{background:${t.blueSoft};color:${t.blueT}}
.m-ban.warn{background:${t.yellowSoft};color:${t.yellowT}}
.m-ban.err{background:${t.redSoft};color:${t.redT}}
`;

export const panelCss = (t) => `${controlsCss(t)}
.sim-chip{z-index:6;line-height:17px}
.sim-panel{z-index:5;display:flex;flex-direction:column;overflow:hidden}
.sp-ctl{display:flex;flex-direction:column;gap:6px;padding:8px 12px;flex:none}
.sp-top{display:flex;align-items:center;gap:8px}.sp-top .m-grow{flex:1}
.sp-chips{display:flex;flex-direction:column;align-items:flex-start;gap:4px}
.sp-pick{display:inline-flex;align-items:center;gap:4px;max-width:100%;height:18px;padding:0 7px;border-radius:9px;background:${t.accentSoft};color:${t.ink2};font:500 10px/18px ${FONT};white-space:nowrap}
.sp-using{font-size:10px;line-height:13px;color:${t.muted}}
.sp-list{flex:1;min-height:0;display:flex;flex-direction:column;gap:10px;padding:12px;overflow:hidden}
.sp-cap{font-size:10px;line-height:13px;color:${t.muted}}
.sp-try{font:600 10px/13px ${FONT};color:${t.ink2}}
.sp-starters{display:flex;flex-direction:column;align-items:flex-start;gap:6px}
.sp-starters .m-b{height:auto;min-height:24px;padding:4px 10px;font-size:12px;white-space:normal;text-align:left;justify-content:flex-start;gap:6px}
.sp-ex{display:flex;flex-direction:column;gap:8px}
.sp-u{display:flex;justify-content:flex-end}.sp-u span{max-width:100%;padding:7px 10px;border-radius:10px;background:${t.accentSoft};font-size:12px}
.sp-r{display:flex;flex-direction:column;align-items:flex-start;gap:6px;padding:10px;border-radius:10px;background:${t.surface2}}
.sp-t{line-height:18px}.sp-t p{margin:0 0 8px}.sp-t p:last-child{margin:0}
.sp-items{align-self:stretch;display:flex;flex-direction:column;gap:4px;font-size:12px}
.sp-item{display:flex;align-items:baseline;gap:6px}.sp-item span:first-child{flex:1}.sp-item i{font-style:normal;color:${t.muted}}
.sp-see{display:inline-flex;align-items:center;gap:5px;font-size:10px;line-height:13px;color:${t.muted}}
.sp-srcs{display:flex;flex-direction:column;gap:4px;padding-left:16px;font-size:10px;line-height:13px;color:${t.muted}}
.sp-srcs b{font-weight:500;color:${t.blueT}}
.sp-copy{font-size:10px;line-height:13px}
.sp-work{display:flex;align-items:center;gap:8px;font-size:12px;color:${t.muted}}.sp-work .m-grow{flex:1}.sp-work .m-b{color:${t.ink}}
.sp-card{display:flex;flex-direction:column;border:1px solid ${t.line};border-radius:8px;background:${t.bg}}
.sp-card h4{margin:0;padding:6px 10px;font:600 10px/13px ${FONT};color:${t.muted};border-bottom:1px solid ${t.line}}
.sp-head{display:flex;flex-direction:column;gap:2px;padding:10px;font-size:12px}.sp-head b{font-size:13px;font-weight:700}
.sp-line{display:flex;justify-content:space-between;align-items:baseline;gap:8px;padding:4px 10px;font-size:12px;color:${t.ink2}}.sp-line b{font-weight:600;color:${t.ink};text-align:right}
.sp-note{padding:6px 10px;font-size:10px;line-height:13px;color:${t.muted}}
.sp-res{font-size:10px;line-height:13px;color:${t.muted}}
.sp-btns{display:flex;justify-content:flex-end;gap:8px}
.sp-in{flex:none;display:flex;flex-direction:column;gap:6px;padding:12px;border-top:1px solid ${t.line}}
.sp-hint{font-size:10px;line-height:13px;color:${t.muted}}
.sp-row{display:flex;align-items:flex-end;gap:8px}
.sp-field{flex:1;min-height:40px;padding:4px 7px;border:1px solid ${t.line2};border-radius:6px;background:${t.field};color:${t.placeholder}}
.sp-field.typed{color:${t.ink}}
.sp-pop{z-index:8;width:300px;height:420px;display:flex;flex-direction:column;border:1px solid ${t.line};border-radius:12px;background:${t.panel};box-shadow:0 10px 34px ${t.shadow}}
.sp-pop:before{content:"";position:absolute;left:50%;top:-7px;margin-left:-7px;width:13px;height:13px;transform:rotate(45deg);background:${t.panel};border:solid ${t.line};border-width:1px 0 0 1px}
.sp-pop-in{flex:1;min-height:0;overflow:hidden;display:flex;flex-direction:column;gap:10px;padding:12px;border-radius:12px 12px 0 0}
.sp-pop-b{display:flex;flex-direction:column;gap:4px}.sp-pop-b h5{margin:0;font:600 10px/13px ${FONT};color:${t.muted}}
.sp-leaf{display:flex;align-items:center;gap:6px;font-size:12px;color:${t.ink}}.sp-leaf svg{color:${t.muted}}.sp-leaf.on svg{color:${t.accent}}.sp-leaf.no{opacity:.45}
.sp-sub{display:flex;align-items:center;gap:6px;padding-left:22px;font-size:10px;color:${t.ink2}}.sp-sub span:first-child{width:38px}
.sp-pk{flex:1;display:flex;justify-content:space-between;align-items:center;height:18px;padding:0 5px 0 8px;border-radius:5px;background:${t.btn};color:${t.ink}}
.sp-foot{flex:none;display:flex;justify-content:flex-end;padding:8px;border-top:1px solid ${t.line};border-radius:0 0 12px 12px}
`;

export const settingsCss = (t) => `${controlsCss(t)}
.sim-set{z-index:5;overflow:hidden}
.ss-in{position:absolute;left:0;right:0;display:flex;flex-direction:column;gap:16px;padding:20px}
.ss-card{display:flex;flex-direction:column;align-items:flex-start;gap:10px;padding:14px;border-radius:10px;background:${t.surface2}}
.ss-card h3{margin:0;display:flex;align-items:center;gap:8px;font:700 13px/16px ${FONT}}
.ss-cap{font-size:10px;line-height:13px;color:${t.muted}}
.ss-m{color:${t.muted}}
.ss-btns{display:flex;gap:8px}
.ss-gfx{display:flex;align-items:center;gap:8px}.ss-gfx b{font-weight:700}
.ss-seg{display:flex;width:180px;height:22px;padding:1px;border-radius:7px;background:${t.btn}}.ss-seg span{flex:1;display:flex;align-items:center;justify-content:center;border-radius:6px;font-size:12px}.ss-seg .on{background:${t.tint};color:#fff}
.ss-stack{display:flex;flex-direction:column;gap:4px}
.ss-bar{align-self:stretch;height:5px;border-radius:3px;background:${t.btn};overflow:hidden}.ss-bar i{display:block;height:100%;background:${t.tint}}
.ss-th{position:absolute;z-index:6;right:3px;width:7px;border-radius:4px;background:rgba(128,128,128,.55)}
.ss-tech{display:flex;flex-direction:column;gap:8px;font-size:12px}.ss-tech>b{display:flex;align-items:center;gap:6px;font-weight:400;font-size:13px}
.ss-part{display:flex;flex-direction:column;gap:2px}.ss-part b{font-weight:500;font-size:13px}.ss-part span{color:${t.muted}}.ss-part code{font:10px/13px ui-monospace,Menlo,Consolas,monospace;word-break:break-all}
.ss-rule{height:1px;background:${t.line}}
.ss-dim{position:absolute;z-index:6;inset:0;border-radius:24px;background:${t.dimCss}}
.ss-alert{position:absolute;z-index:7;left:50%;top:50%;transform:translate(-50%,-50%);width:260px;display:flex;flex-direction:column;gap:8px;padding:20px 16px 14px;border-radius:24px;background:${t.alert};box-shadow:0 12px 36px ${t.shadow}}
.ss-alert b{padding:0 6px;font-weight:700}.ss-alert p{margin:0 0 6px;padding:0 6px}
.ss-pill{display:flex;align-items:center;justify-content:center;height:28px;border-radius:14px;background:${t.btn}}
.ss-pill.danger{background:rgba(255,59,48,.2);color:#ff3b30}
`;

// ---- panel parts (AIConversationView) ----
const SCREENPLAY_PICK = 'Screenplay (Example-Short (imported), all scenes)';
const EMPTY_TEXT = 'Ask about your project, for example “How many scenes are in the script?”, “Open Scene 12.” or “Create a task to scout the railway station.” Answers come from your project and say what they are based on; changes are only ever prepared for you to review.';
const STARTERS = {
  screenplay: ['How many scenes does the screenplay have?', 'List the scene headings.', 'How long is the screenplay?', 'Are there unresolved comments?'],
  story: ['How many scene cards are on the beat board?', 'Show the characters on the beat board.', 'How many acts are there?', 'How many scenes does the screenplay have?'],
  breakdown: ['How far along is the breakdown?', 'Which draft is the production source?', 'Which locations are used in the script?', 'How many scenes does the screenplay have?'],
  production: ['How many scenes are unscheduled?', 'How far along is the breakdown?', 'Which locations are used in the script?', 'Which draft is the production source?'],
  other: ['How many scenes does the screenplay have?', 'How many scene cards are on the beat board?', 'What open tasks do I have?', 'How far along is the breakdown?'],
};
const controls = (pick, { saved = false, busy = false } = {}) => `<div class="sp-ctl"><div class="sp-top"><span class="m-b">${ICON.stack}Sections</span><span class="m-grow"></span><span class="m-link${saved ? '' : ' off'}">${ICON.save}</span><span class="m-link${busy ? ' off' : ''}">New</span></div>${pick ? `<div class="sp-chips"><span class="sp-pick">${pick}${ICON.remove}</span></div>` : ''}<div class="sp-using">${pick ? `Using: ${pick}` : 'Pick a section'}</div></div>`;
const composer = ({ pick, typed = '', busy = false } = {}) => `<div class="sp-in">${pick ? '' : '<div class="sp-hint">Pick a section</div>'}<div class="sp-row"><div class="sp-field${typed ? ' typed' : ''}">${typed || 'Ask or command… e.g. “Open Scene 42.”'}</div><span class="m-b go${typed && pick && !busy ? '' : ' off'}">Send</span></div></div>`;
const empty = (ws, pick) => `<div class="sp-cap">${EMPTY_TEXT}</div><div class="sp-try">Try asking</div><div class="sp-starters">${STARTERS[ws].map((q) => `<span class="m-b${pick ? '' : ' off'}">${ICON.bubble}${q}</span>`).join('')}</div>`;
const user = (t) => `<div class="sp-u"><span>${t}</span></div>`;
const answer = (text, { items = [], sources, closed = true } = {}) => `<div class="sp-r"><div class="sp-t">${text}</div>${items.length ? `<div class="sp-items">${items.map(([a, b]) => `<div class="sp-item"><span>${a}${b ? ` <i>· ${b}</i>` : ''}</span><span class="m-b sm">Show</span></div>`).join('')}</div>` : ''}${closed ? `<span class="sp-see">${ICON.right}See sources</span>` : `<span class="sp-see">${ICON.down}See sources</span><div class="sp-srcs">${sources}</div>`}<span class="m-link sp-copy">${ICON.copy}Copy</span></div>`;
const card = (title, summary, area, rows, pending) => `<div class="sp-card"><h4>Proposed Changes</h4><div class="sp-head"><b>${title}</b><span>${summary}</span></div><div class="sp-line"><span>Affected areas</span><b>${area}</b></div>${rows.map(([a, b]) => `<div class="sp-line"><span>${a}</span><b>${b}</b></div>`).join('')}${pending ? '<div class="sp-note">1 change, applied together as one step you can undo. If any part fails, nothing is changed.</div>' : ''}</div>`;
const PENDING_BTNS = '<div class="sp-btns"><span class="m-b">Reject</span><span class="m-b go">Apply Changes</span></div>';
const TASK_SUMMARY = 'I prepared a task. Nothing has been added yet.';
const taskExchange = (q, task, after) => `<div class="sp-ex">${user(q)}${answer(TASK_SUMMARY)}${card('Proposed task', TASK_SUMMARY, 'Notes &amp; Tasks', [['Task', task], ['Impact', '1 new object']], after === 'pending')}${after === 'pending' ? PENDING_BTNS : after}</div>`;
const conversation = (pick, inner, opts = {}) => () => `${controls(pick, opts)}<div class="sp-list">${inner}</div>${composer({ pick, typed: opts.typed, busy: opts.busy })}`;
const firstOpen = (ws) => conversation('', empty(ws, ''));
const PICKED_SOURCES = '<span>Inferred · written by Straft AI</span><span>Based on:</span><b>Draft: Example-Short (imported)</b><b>Location: Kesarganj Junction</b><b>Location: Night Train</b>';
const leaf = (label, state = '') => `<div class="sp-leaf ${state}">${state === 'on' ? ICON.picked : ICON.empty}<span>${label}</span></div>`;
const sub = (label, value) => `<div class="sp-sub"><span>${label}</span><span class="sp-pk">${value}${ICON.updown}</span></div>`;
const branch = (label, rows) => `<div class="sp-pop-b"><h5>${label}</h5>${rows}</div>`;
// AISectionPickerButton: a 300 by 420 popover under the button; the branches and leaves are the core's section table (ai/sections.rs)
const SECTION_LIST = (g) => `<div class="sim-root sp-pop" style="left:${g.body[0] + 58 - 150}px;top:${g.body[1] + 8 + 24 + 9}px"><div class="sp-pop-in"><div class="sp-cap">Pick up to 3 sections. Straft AI reads only these.</div>${branch('Idea Vault', leaf('This Project') + leaf('Global'))}${branch('Story', leaf('Story Page') + leaf('Beat Board') + leaf('Outline') + leaf('Characters') + leaf('Timeline') + leaf('Episodes', 'no'))}${branch('Screenplay', leaf('Scenes', 'on') + sub('Draft', 'Current draft') + sub('Scene', 'All scenes'))}${branch('Breakdown', leaf('Breakdown'))}${branch('Production', leaf('Overview') + leaf('Catalog'))}</div><div class="sp-foot"><span class="m-b">Done</span></div></div>`;
const FIRST_OPEN_TEXTS = (ws) => ['Straft AI', 'AI Ready', 'Sections', 'New', 'Pick a section', 'Try asking', ...STARTERS[ws], 'Ask or command… e.g. “Open Scene 42.”', 'Send'];

// ---- Settings, Straft AI (AISettingsView) ----
const PROMISE = 'Straft AI runs on this computer after a one-time download. Your project data is not sent to Straft Studio or to an AI cloud service.';
const NORMAL = '<span class="ss-cap">Straft Studio works normally without Straft AI.</span>';
const statusCard = (off = false) => `<div class="ss-card"><h3>Straft AI <span class="m-chip green">AI Ready</span></h3><div>${PROMISE}</div><div class="ss-btns"><span class="m-b">Stop Straft AI (free memory)</span><span class="m-b">Remove Straft AI…</span></div><div class="ss-stack"><div class="ss-gfx"><b>Graphics card</b><div class="ss-seg"><span${off ? '' : ' class="on"'}>Automatic</span><span${off ? ' class="on"' : ''}>Off</span></div></div><span class="ss-cap">${off ? 'Runs on the processor. The graphics card is switched off in Settings.' : 'Decided before the first answer.'} Automatic uses a graphics card only after it passes a short check; the processor is always the fallback.</span></div>${NORMAL}</div>`;
const healthCard = '<div class="ss-card"><h3>AI health</h3><div class="ss-m">No requests yet. The figures appear after you use Straft AI.</div><span class="ss-cap">Only numbers are kept on this computer: no questions, answers or project text. Nothing is sent. Export AI report saves them to a file you choose, to share if you want to.</span><span class="m-b">Export AI report…</span></div>';
const memoryCard = '<div class="ss-card"><h3>Straft AI memory</h3><div class="ss-m">Nothing is remembered yet. Straft AI keeps a fact or preference only when you state it outright, or confirm one of its answers.</div><span class="ss-cap">Saved on this Mac, in this project, for you only, and only when you state a fact or confirm an answer. Delete an entry and Straft AI forgets it.</span></div>';
const part = (role, name, facts, sha) => `<div class="ss-part"><b>${role}: ${name}</b><span>${facts}</span><code>${sha}</code></div>`;
// names, versions, sizes and hashes are the macOS entries of packaging/ai/ai-assets.lock.json; the computer's memory and processor and the two paths are examples
const techDetails = `<div class="ss-tech"><b>${ICON.down}Technical details</b>${part('AI engine', 'llama.cpp server (processor)', 'Version b11259, 28.6 MB, licence MIT', '6a072f8bf308ea28144476441d25760ee01b84ed102f86be4662d21ace8d72a6')}${part('Language model', 'Gemma 3 1B Instruct (Q8_0)', 'Version f9c28bc, 1.07 GB, licence Gemma-Terms-of-Use', 'b205840c5dcef55078e37d344677869a714ffd42a4ae448c48dcfb52e4bb10d5')}${part('Search model', 'BGE small English v1.5 (Q8_0)', 'Version f2068ed, 36.7 MB, licence MIT', 'f046db1dc724cf4f6f0a0c5917e922823b73eb1d27b8f9a9c2797f7866974804')}${part('Search model', 'Multilingual E5 small (Q8_0)', 'Version b6cac96, 132.4 MB, licence MIT', 'e011debc1208e31bf7b6aebee2d9fc8bd2ca11694a77ed66ac9d0c9d0a877c93')}<span class="ss-cap">Gemma is provided under and subject to the Gemma Terms of Use found at ai.google.dev/gemma/terms.</span><div class="ss-rule"></div><span>Package: straft-local-ai-v1 v1.0.0 · files: checked</span><span>Language model: Ready · search model: Ready (search always on the processor)</span><span>This computer: 17.18 GB memory · Apple M2 (8 threads)</span><span>Engine in: /Applications/Straft Studio.app/Contents/Resources/ai (read-only) · engine and models: 1.27 GB</span><span>Log: /Users/maya/Library/Application Support/Straft Studio/logs/ai-runtime.log</span><span class="ss-cap">Everything runs on this computer. Prompts, project text and answers are never sent anywhere. The only network use is the model download you start.</span></div>`;
const downloadCards = `<div class="ss-card"><h3>Straft AI <span class="m-chip plain">Not downloaded</span></h3><div>${PROMISE}</div>${NORMAL}</div><div class="ss-card" style="padding:12px"><h3>Download Straft AI</h3><div style="font-size:12px">Downloading: 436.2 MB of 1.24 GB</div><div class="ss-bar"><i style="width:35%"></i></div><span class="m-b">Pause</span></div>${healthCard}${memoryCard}`;
// bottom: anchors the end of the page, as when it is scrolled all the way down
const pane = (inner, { bottom = false, thumb = [4, 290] } = {}) => () => `<div class="ss-in" style="${bottom ? 'bottom:0' : 'top:0'}">${inner}</div><span class="ss-th" style="top:${thumb[0]}px;height:${thumb[1]}px"></span>`;
const REMOVE = () => '<div class="ss-dim"></div><div class="sim-root ss-alert"><b>Remove Straft AI?</b><p>This frees the space the Straft AI models use on this computer. Straft AI stays off until you download it again. Your projects are not changed.</p><div class="ss-pill danger">Remove</div><div class="ss-pill">Cancel</div></div>';

const T = (...a) => a;
export const SIMS = [
  { name: 'ai-sim-beside-vault', title: 'Straft AI open beside Idea Vault', kind: 'panel', base: 'ai-panel-idea-vault', panel: firstOpen('other'), texts: FIRST_OPEN_TEXTS('other') },
  { name: 'ai-sim-beside-story', title: 'Straft AI open beside Story', kind: 'panel', base: 'ai-panel-story', panel: firstOpen('story'), texts: FIRST_OPEN_TEXTS('story') },
  { name: 'ai-sim-beside-screenplay', title: 'Straft AI open beside Screenplay', kind: 'panel', base: 'ai-panel-screenplay', panel: firstOpen('screenplay'), texts: FIRST_OPEN_TEXTS('screenplay') },
  { name: 'ai-sim-beside-breakdown', title: 'Straft AI open beside Breakdown', kind: 'panel', base: 'ai-panel-breakdown', panel: firstOpen('breakdown'), texts: FIRST_OPEN_TEXTS('breakdown') },
  { name: 'ai-sim-beside-production', title: 'Straft AI open beside Production', kind: 'panel', base: 'ai-panel-production', panel: firstOpen('production'), texts: FIRST_OPEN_TEXTS('production') },
  { name: 'ai-sim-answer-delete-request', title: 'Straft AI asked to delete, waiting for Apply', kind: 'panel', base: 'ai-panel-screenplay', panel: conversation(SCREENPLAY_PICK, `<div class="sp-ex">${user('Delete Scene 1.')}${answer('I prepared this change. Nothing has been changed yet.')}${card('Proposed scene deletion', 'I prepared this change. Nothing has been changed yet.', 'Screenplay', [['Delete', 'Scene 1'], ['Its text', '16 lines'], ['Other drafts', 'Not changed']], true)}${PENDING_BTNS}</div>`, { saved: true }), texts: T('Straft AI', 'Delete Scene 1.', 'I prepared this change. Nothing has been changed yet.', 'Proposed Changes', 'Proposed scene deletion', 'Affected areas', 'Screenplay', 'Its text', 'Other drafts', 'Not changed', 'Reject', 'Apply Changes') },
  { name: 'ai-sim-offline-ai-download', title: 'Settings, Straft AI, downloading', kind: 'settings', base: 'settings-offline-ai', panel: pane(downloadCards), texts: T('Straft AI', 'Not downloaded', 'Download Straft AI', 'Downloading: 436.2 MB of 1.24 GB', 'Pause', 'AI health') },
  { name: 'ai-sim-ready', title: 'Straft AI, ready to ask', kind: 'panel', base: 'ai-panel', panel: firstOpen('other'), texts: FIRST_OPEN_TEXTS('other') },
  { name: 'ai-sim-question-typed', title: 'Straft AI, a question typed', kind: 'panel', base: 'ai-panel-screenplay', panel: conversation(SCREENPLAY_PICK, empty('screenplay', SCREENPLAY_PICK), { typed: 'How many scenes are in the script?' }), texts: T('Straft AI', 'AI Ready', 'Sections', SCREENPLAY_PICK, `Using: ${SCREENPLAY_PICK}`, 'How many scenes are in the script?', 'Send') },
  { name: 'ai-sim-working', title: 'Straft AI working on an answer', kind: 'panel', base: 'ai-panel', panel: conversation('Characters', `${user('List the characters.')}<div class="sp-work">${ICON.spin}<span class="m-grow">Working on it on this computer…</span><span class="m-b">Cancel</span></div>`, { busy: true }), texts: T('Straft AI', 'Using: Characters', 'List the characters.', 'Working on it on this computer…', 'Cancel') },
  { name: 'ai-sim-answer-characters', title: 'Straft AI answers with a list', kind: 'panel', base: 'ai-panel', panel: conversation('Characters', `<div class="sp-ex">${user('List the characters.')}${answer('The story has 4 characters.', { items: [['Kiran', 'A ticket inspector on the night route'], ['Devi', 'A nurse coming off shift'], ['Meera', 'A student who sleeps on the bus'], ['Tomas', 'The depot mechanic']] })}</div>`, { saved: true }), texts: T('Straft AI', 'Using: Characters', 'List the characters.', 'The story has 4 characters.', 'Kiran', 'Devi', 'Meera', 'Tomas', 'Show', 'See sources', 'Copy') },
  { name: 'ai-sim-answer-scene-count', title: 'Straft AI counts the scenes', kind: 'panel', base: 'ai-panel-screenplay', panel: conversation(SCREENPLAY_PICK, `<div class="sp-ex">${user('How many scenes does the screenplay have?')}${answer('LOST PROPERTY has 10 scenes.')}</div>`, { saved: true }), texts: T('Straft AI', `Using: ${SCREENPLAY_PICK}`, 'How many scenes does the screenplay have?', 'LOST PROPERTY has 10 scenes.', 'See sources', 'Copy') },
  { name: 'ai-sim-answer-sources-open', title: 'Straft AI answer with its sources open', kind: 'panel', base: 'ai-panel-screenplay', panel: conversation(SCREENPLAY_PICK, `<div class="sp-ex">${user('Which locations are used in the script?')}${answer('<p>The screenplay uses these locations:</p><p>Kesarganj Junction: the Lost Property Office, Platform 3, the Store Room and the Footbridge. One scene is on the night train, in a second class compartment.</p>', { closed: false, sources: PICKED_SOURCES })}</div>`, { saved: true }), texts: T('Straft AI', 'Which locations are used in the script?', 'See sources', 'Inferred · written by Straft AI', 'Based on:', 'Draft: Example-Short (imported)', 'Location: Kesarganj Junction', 'Location: Night Train') },
  { name: 'ai-sim-scope-menu', title: 'Straft AI, choosing the sections it reads', kind: 'panel', base: 'ai-panel-screenplay', panel: conversation(SCREENPLAY_PICK, empty('screenplay', SCREENPLAY_PICK)), over: SECTION_LIST, texts: T('Straft AI', 'Sections', 'Pick up to 3 sections. Straft AI reads only these.', 'Idea Vault', 'This Project', 'Global', 'Story', 'Beat Board', 'Characters', 'Screenplay', 'Scenes', 'Draft', 'Current draft', 'Scene', 'All scenes', 'Done', `Using: ${SCREENPLAY_PICK}`) },
  { name: 'ai-sim-proposed-changes', title: 'Straft AI proposes a change', kind: 'panel', base: 'ai-panel-screenplay', panel: conversation('Tasks', taskExchange('Create a task to scout the footbridge.', 'Scout the footbridge', 'pending'), { saved: true }), texts: T('Straft AI', 'Using: Tasks', 'Create a task to scout the footbridge.', 'I prepared a task. Nothing has been added yet.', 'Proposed Changes', 'Proposed task', 'Affected areas', 'Notes & Tasks', 'Task', 'Scout the footbridge', 'Impact', '1 new object', 'Reject', 'Apply Changes') },
  { name: 'ai-sim-proposal-applied', title: 'Straft AI change applied', kind: 'panel', base: 'ai-panel-screenplay', panel: conversation('Tasks', taskExchange('Create a task to ask the station for a footbridge permit.', 'Ask the station for a footbridge permit', '<div class="sp-res">Applied 1 change. Use Undo to reverse it.</div><div class="sp-btns"><span class="m-b">Undo</span></div>'), { saved: true }), texts: T('Straft AI', 'Proposed Changes', 'Notes & Tasks', 'Ask the station for a footbridge permit', 'Applied 1 change. Use Undo to reverse it.', 'Undo') },
  { name: 'ai-sim-proposal-rejected', title: 'Straft AI change rejected', kind: 'panel', base: 'ai-panel-screenplay', panel: conversation('Tasks', taskExchange('Create a task to scout the footbridge.', 'Scout the footbridge', '<div class="sp-res">Rejected. Nothing was changed.</div>'), { saved: true }), texts: T('Straft AI', 'Proposed Changes', 'Notes & Tasks', 'Scout the footbridge', 'Rejected. Nothing was changed.') },
  { name: 'ai-sim-offline-ai-ready', title: 'Settings, Straft AI, running', kind: 'settings', base: 'settings-offline-ai', panel: pane(`${statusCard()}${healthCard}${memoryCard}`), texts: T('Straft AI', 'AI Ready', 'Stop Straft AI (free memory)', 'Remove Straft AI…', 'Graphics card', 'Automatic', 'Off', 'AI health', 'Straft AI memory') },
  { name: 'ai-sim-offline-ai-technical-details', title: 'Settings, Straft AI, technical details', kind: 'settings', base: 'settings-offline-ai', panel: pane(`${memoryCard}${techDetails}`, { bottom: true, thumb: [170, 286] }), texts: T('Technical details', 'AI engine: llama.cpp server (processor)', 'Language model: Gemma 3 1B Instruct (Q8_0)', 'Search model: BGE small English v1.5 (Q8_0)', 'Search model: Multilingual E5 small (Q8_0)', 'Package: straft-local-ai-v1 v1.0.0 · files: checked') },
  { name: 'ai-sim-offline-ai-graphics-off', title: 'Settings, Straft AI, graphics card set to Off', kind: 'settings', base: 'settings-offline-ai', panel: pane(`${statusCard(true)}${healthCard}${memoryCard}`), texts: T('Straft AI', 'AI Ready', 'Graphics card', 'Automatic', 'Off', 'Runs on the processor. The graphics card is switched off in Settings.') },
  { name: 'ai-sim-offline-ai-remove-confirm', title: 'Settings, Straft AI, remove confirmation', kind: 'settings', base: 'settings-offline-ai', panel: pane(`${statusCard()}${healthCard}${memoryCard}`), over: REMOVE, texts: T('Remove Straft AI?', 'This frees the space the Straft AI models use on this computer. Straft AI stays off until you download it again. Your projects are not changed.', 'Remove', 'Cancel') },
];
