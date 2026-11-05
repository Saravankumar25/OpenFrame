// Simulated Mac dialogs and notices for capabilities the Mac capture never reached (owner decision 2026-11-01: every capability shows a Mac screen).
// Each is a real Mac capture of the Night Bus project: with a sheet over the dimmed window, or with a part of the page redrawn. Words and structure come from the Mac source.
// Fields used by sim-mac.mjs: measure (name to selector, in the base page), shift, paint, sheet (dims the picture as a real sheet does), html.
import { controlsCss, ICON } from './sim-mac-screens.mjs';

const FONT = '-apple-system,BlinkMacSystemFont,"SF Pro Text","Helvetica Neue","Segoe UI",system-ui,sans-serif';

export const freeCss = (t) => `${controlsCss(t)}
.fx-sheet{z-index:6;left:50%;top:50%;transform:translate(-50%,-50%);display:flex;flex-direction:column;gap:16px;padding:24px;border-radius:24px;background:${t.panel};box-shadow:0 16px 48px ${t.shadow}}
.fx-sheet.bg{background:${t.bg}}
.fx-head{display:flex;flex-direction:column;align-items:center;gap:4px;text-align:center}
.fx-head b{font:600 15px/20px ${FONT}}
.fx-head span{font-size:12px;color:${t.muted}}
.fx-btns{display:flex;justify-content:flex-end;gap:10px}
.fx-kv{border:1px solid ${t.line};border-radius:8px;background:${t.surface2};font-size:12px}
.fx-kv div{display:flex;gap:8px;padding:6px 10px;border-top:1px solid ${t.line}}.fx-kv div:first-child{border-top:0}
.fx-kv span{flex:none;width:170px;color:${t.muted}}
.fx-cands{display:flex;flex-direction:column;border-radius:8px;background:${t.btn}}
.fx-cand{display:flex;align-items:center;gap:8px;padding:8px;font-size:12px}.fx-cand i{font-style:normal;color:${t.muted}}
.fx-impact{display:flex;flex-direction:column;gap:3px;font-size:12px}.fx-impact b{font-weight:600}
.fx-brand{display:flex;align-items:center;justify-content:center;gap:13px;font-size:28px;line-height:34px;letter-spacing:-.56px}.fx-brand b{font-weight:700}.fx-brand span{font-weight:300}
.fx-big{text-align:center;font:600 17px/22px ${FONT}}
.fx-center{text-align:center}
.fx-gray{display:flex;gap:8px;padding:10px;border-radius:8px;background:${t.btn};color:${t.muted};font-size:12px}.fx-gray svg{flex:none;margin-top:1px}
.fx-drawer{z-index:5;display:flex;flex-direction:column;overflow:hidden;background:${t.panel}}
.fx-dh{display:flex;align-items:flex-start;padding:12px 16px;border-bottom:1px solid ${t.line}}.fx-dh>div{flex:1}.fx-dh small{display:block;font-size:10px;line-height:13px;color:${t.muted}}.fx-dh b{font-weight:700}
.fx-dbody{display:flex;flex-direction:column;gap:14px;padding:16px}
.fx-row{display:flex;gap:6px}
.fx-file{display:flex;align-items:center;gap:10px;padding:12px;border-radius:8px;background:${t.surface2};font-size:12px}.fx-file svg{flex:none;color:${t.muted}}.fx-file small{display:block;font-size:10px;line-height:13px;color:${t.muted}}
.fx-cap{font-size:10px;line-height:13px;color:${t.muted}}
.fx-miss{display:flex;flex-direction:column;align-items:flex-start;gap:6px;padding:10px;border-radius:8px;background:${t.redSoft};color:${t.redT};font-size:12px}.fx-miss b{font-weight:600}.fx-miss .fx-cap{color:${t.muted}}.fx-miss .m-b{color:${t.ink}}
.fx-lab{display:flex;flex-direction:column;gap:4px}.fx-lab>b{font:600 10px/13px ${FONT};color:${t.ink2}}
.fx-in{height:24px;padding:3px 7px;border:1px solid ${t.line2};border-radius:6px;background:${t.field};color:${t.placeholder}}.fx-in.two{height:40px}
.fx-status{display:flex;justify-content:space-between;font-size:10px;line-height:13px;color:${t.muted}}
.fx-pop{display:flex;align-items:center;justify-content:space-between;width:172px;height:24px;padding:0 8px 0 12px;border-radius:7px;background:${t.btn}}
.fx-page{z-index:5;display:flex;flex-direction:column;align-items:stretch;gap:16px;padding-top:16px;overflow:hidden}
.fx-page h2{margin:0;font:700 22px/28px ${FONT}}
.fx-sub{margin-top:4px;font-size:12px;color:${t.muted}}
.fx-check{display:flex;align-items:center;justify-content:center;gap:7px}.fx-check i{width:14px;height:14px;border-radius:4px;background:${t.btn};border:1px solid ${t.line2}}
.fx-sec{display:flex;align-items:center;gap:8px;padding:8px 10px;border-bottom:1px solid ${t.line};font-size:12px}.fx-sec b{font-weight:600}.fx-sec small{font-size:10px;color:${t.muted}}.fx-sec .m-grow{flex:1}
.fx-el{display:flex;align-items:center;gap:8px;padding:6px 10px;font-size:12px}.fx-el b{font-weight:600}.fx-el em{width:8px;height:8px;border-radius:50%;background:${t.muted}}.fx-el .m-grow{flex:1}.fx-el small{font-size:10px;color:${t.muted}}
.fx-foot{z-index:5;display:flex;align-items:center;gap:7px;padding:10px;border-top:1px solid ${t.line};font-size:12px;background:${t.panel}}
`;

const head = (title, sub) => `<div class="fx-head"><b>${title}</b>${sub ? `<span>${sub}</span>` : ''}</div>`;
const btns = (list) => `<div class="fx-btns">${list.map(([l, kind]) => `<span class="m-b${kind ? ` ${kind}` : ''}">${l}</span>`).join('')}</div>`;
const sheet = (w, inner, bg = '') => () => `<div class="sim-root fx-sheet${bg}" style="width:${w}px">${inner}</div>`;
const candidate = (label, tone, name, evidence) => `<div class="fx-cand"><span class="m-chip ${tone}">${label}</span><span><b>${name}</b> <i>${evidence}</i></span></div>`;
// CallSheetEditorView: the banner stands between the title and the paper, so the paper and the Source panel move down. The move stops above the base's toast and its
// shadow, and paper colour closes the band left under the paper.
const CROP_END = 720;
const callSheetBanner = (toneIcon, tone, inner, dy) => ({
  measure: { editor: '[data-id=callsheet-editor]', toast: '[data-id=toast]', back: '[data-id=callsheet-back]' },
  shift: (g) => { const y = g.m.back[1] + 34, x = g.m.editor[0] - 12; return { rect: [x, y, g.win[0] - x, CROP_END - y], dy }; },
  paint: (g) => { const y = g.m.back[1] + 34, x = g.m.editor[0] - 12, paper = [323, CROP_END + dy, 860, 776 - (CROP_END + dy)]; return [[[x, y, g.win[0] - x, dy], [x + 2, y + dy + 20]], [paper, [330, 400]]]; },
  html: (g) => `<div class="sim-root m-ban ${tone}" style="z-index:5;left:${g.m.editor[0]}px;top:${g.m.back[1] + 40}px;width:${g.m.editor[2]}px">${toneIcon}${inner}</div>`,
});

export const FREE_SIMS = [
  { name: 'ai-sim-vault-file-moved', title: 'Idea Vault, a linked file that moved', base: 'vault-drawer-note',
    measure: { drawer: '[data-id=vault-drawer]', del: '[data-id=drawer-delete]', close: '[data-id=drawer-close]' },
    html: (g) => { const { drawer: d, del, close } = g.m, top = close[1] - 12; return `<div class="sim-root fx-drawer" style="left:${d[0] + 1}px;top:${top}px;width:${d[2] - 1}px;height:${del[1] - 13 - top}px"><div class="fx-dh"><div><small>File · external file</small><b>Route map notes.txt</b></div><span class="m-link">${ICON.close}Close</span></div><div class="fx-dbody"><div class="fx-row"><span class="m-b sm">Pin</span><span class="m-b sm">Rename</span><span class="m-b sm">Move</span><span class="m-b sm">Copy to Global Vault</span></div><div class="fx-file">${ICON.doc}<div>Route map notes.txt<small>This file is not available right now.</small></div></div><div class="fx-cap">Route map notes.txt · 2 KB</div><div class="fx-miss"><b>File unavailable.</b><span>The original file could not be found at /Users/maya/Documents/Night Bus/Route map notes.txt.</span><span>Straft Studio keeps this item and its notes. Nothing has been deleted.</span><div class="fx-row"><span class="m-b sm">Relink…</span><span class="m-b sm">Open Containing Folder</span></div><span class="fx-cap">This item links to a file outside the project. Files stored inside the project show a different marker.</span></div><div class="fx-lab"><b>Title</b><div class="fx-in">Leave blank to keep it untitled</div></div><div class="fx-lab"><b>Caption</b><div class="fx-in two">Optional</div></div><div class="fx-lab"><b>Source link (optional)</b><div class="fx-in">https://…</div></div><div class="fx-status"><span>Saved automatically</span><span>Added today · Modified today</span></div><div class="fx-lab"><b>Research flag</b><div class="fx-pop">No flag${ICON.updown}</div><span class="fx-cap">Verification flags are yours to set. Straft Studio never marks research as true just because you imported it.</span></div></div></div>`; },
    texts: ['File · external file', 'Route map notes.txt', 'This file is not available right now.', 'File unavailable.', 'The original file could not be found at /Users/maya/Documents/Night Bus/Route map notes.txt.', 'Straft Studio keeps this item and its notes. Nothing has been deleted.', 'Relink…', 'Open Containing Folder', 'Title', 'Caption', 'Source link (optional)'] },
  { name: 'ai-sim-import-failed', title: 'Screenplay, an import that failed', base: 'screenplay-navigator-columns-shown', sheet: true,
    html: sheet(560, `${head('Import failed. Your current project was not changed.')}<div class="fx-kv"><div><span>File</span>Lost Property.pages</div><div><span>Stage</span>Checking the file type</div><div><span>Problem</span>This file type can't be imported as a screenplay. Supported types are PDF, Final Draft (.fdx), Fountain, TXT and DOCX.</div><div><span>Your project</span>Unchanged</div><div><span>Source file</span>Untouched</div></div><div style="font-size:12px">You can still import it as a project document, or paste the screenplay text into a new draft.</div>${btns([['Paste Text Instead'], ['Add to Project Files'], ['Close', 'go']])}`),
    texts: ['Import failed. Your current project was not changed.', 'File', 'Lost Property.pages', 'Stage', 'Checking the file type', 'Problem', "This file type can't be imported as a screenplay. Supported types are PDF, Final Draft (.fdx), Fountain, TXT and DOCX.", 'Your project', 'Unchanged', 'Source file', 'Untouched', 'Paste Text Instead', 'Add to Project Files', 'Close'] },
  { name: 'ai-sim-breakdown-needs-review', title: 'Breakdown, a scene changed in the script', base: 'breakdown-scenes', sheet: true,
    paint: () => [[[590, 740, 640, 44], [600, 700]]], // the base's toast
    html: sheet(600, `${head('Scene 1 changed. Review breakdown differences.', 'The screenplay changed after this scene was broken down. Your production data has not been changed.')}<div class="fx-cands">${candidate('Removed candidate', 'red', 'Cardigan', '(Wardrobe): “cardigan over a railway uniform”')}${candidate('Added candidate', 'green', 'Birdcage', '(Prop): “a birdcage with no bird”')}</div><div class="fx-impact"><b>Impact</b><span>• New elements suggested: 1</span><span>• Existing elements requiring review: 1 (Cardigan)</span></div>${btns([['Review Manually'], ['Keep Production Data'], ['Apply Suggested Update', 'def']])}`),
    texts: ['Scene 1 changed. Review breakdown differences.', 'The screenplay changed after this scene was broken down. Your production data has not been changed.', 'Removed candidate', 'Added candidate', 'Impact', 'New elements suggested: 1', 'Existing elements requiring review: 1 (Cardigan)', 'Review Manually', 'Keep Production Data', 'Apply Suggested Update'] },
  { name: 'ai-sim-breakdown-removed-scenes', title: 'Breakdown, removed scenes kept for reference', base: 'breakdown-scenes',
    paint: () => [[[479, 88, 1121, 704], [1400, 500]], [[229, 730, 249, 62], [350, 700]]],
    // ReviewViews.HistoricalView replaces the script and the elements panel; the page column is 760 wide, the scene list stays and names the page at its foot
    html: () => `<div class="sim-root fx-page" style="left:${479 + 180}px;top:88px;width:760px;height:704px"><div><span class="m-b">${ICON.left}Back to Breakdown</span></div><div><h2>Removed scenes</h2><div class="fx-sub">Scene removed from current source. Production data is kept here so nothing is lost; archive what you no longer need.</div></div><div class="fx-check"><i></i>Show archived</div><div><div class="fx-sec"><b>EXT. KESARGANJ JUNCTION - WATER TOWER - DAY</b><small>from Example-Short (imported)</small><span class="m-grow"></span><span class="m-b sm">Open</span></div><div class="fx-el"><em></em><b>Ladder</b><span class="m-grow"></span><small>Props</small><span class="m-b sm">Archive</span></div><div class="fx-el"><em></em><b>Lantern</b><span class="m-grow"></span><small>Props</small><span class="m-b sm">Archive</span></div></div><div><div class="fx-sec"><b>INT. NIGHT TRAIN - GUARD VAN - NIGHT</b><small>from Example-Short (imported)</small><span class="m-grow"></span><span class="m-b sm">Open</span></div><div class="fx-el"><em></em><b>Whistle</b><span class="m-grow"></span><small>Props</small><span class="m-b sm">Archive</span></div></div></div><div class="sim-root fx-foot" style="left:229px;top:754px;width:249px;height:38px;border-bottom-left-radius:0">${ICON.clock}Removed scenes (2)</div>`,
    texts: ['Back to Breakdown', 'Removed scenes', 'Scene removed from current source. Production data is kept here so nothing is lost; archive what you no longer need.', 'Show archived', 'Open', 'Archive', 'Removed scenes (2)'] },
  { name: 'ai-sim-callsheet-schedule-changed', title: 'Call sheet, the schedule changed after final', base: 'callsheet-finalized',
    ...callSheetBanner(ICON.info, 'info', '<span class="m-grow">The schedule changed after this call sheet was finalized (Times). The final version never changes; start a new revision to update it.</span><span class="m-b sm">Start New Revision</span>', 46),
    texts: ['The schedule changed after this call sheet was finalized (Times). The final version never changes; start a new revision to update it.', 'Start New Revision'] },
  { name: 'ai-sim-callsheet-day-deleted', title: 'Call sheet, its shooting day was deleted', base: 'callsheet-editor',
    ...callSheetBanner(ICON.warn, 'warn', '<span class="m-grow">The shooting day of this call sheet was deleted. The call sheet stays readable and unchanged.</span>', 40),
    texts: ['The shooting day of this call sheet was deleted. The call sheet stays readable and unchanged.'] },
  { name: 'ai-sim-trial-reminder', title: 'The 7-day free trial reminder', base: 'project-home', sheet: true,
    // LicenceScreens.ReminderDialog with the mark of BrandMark.swift: the film frame on its 24 by 24 grid
    html: sheet(480, `${head('Trial reminder')}<div class="fx-brand"><svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="url(#fx-logo)" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><defs><linearGradient id="fx-logo" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#ff5d6c"/><stop offset=".5" stop-color="#ff9a3d"/><stop offset="1" stop-color="#ffc23d"/></linearGradient></defs><rect x="2" y="2" width="20" height="20" rx="2.2"/><path d="M7 2v20M17 2v20M2 12h20M2 7h5M2 17h5M22 7h-5M22 17h-5"/></svg><div><b>Straft</b> <span>Studio</span></div></div><div class="fx-big">Your free trial ends in 3 days</div><div class="fx-center">It ends on Sun 8 Nov at 9:42 PM.</div><div class="fx-center">Buy the full version to keep using Straft Studio. Pay once, own forever. No subscription.</div><div class="fx-gray">${ICON.info}<span>Your projects stay on your computer either way. Nothing is deleted when the trial ends.</span></div>${btns([['Continue trial'], [`${ICON.bolt}Buy full version`, 'go']])}`, ' bg'),
    texts: ['Trial reminder', 'Straft Studio', 'Your free trial ends in 3 days', 'It ends on Sun 8 Nov at 9:42 PM.', 'Buy the full version to keep using Straft Studio. Pay once, own forever. No subscription.', 'Your projects stay on your computer either way. Nothing is deleted when the trial ends.', 'Continue trial', 'Buy full version'] },
];
