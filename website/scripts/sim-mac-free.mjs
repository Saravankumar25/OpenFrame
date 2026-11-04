// Simulated Mac dialogs and notices for capabilities the Mac captures never reached (owner decision 2026-11-01: no "not on Mac"
// notes, every capability shows a Mac screen). Each is a real Mac capture with a dimmed window and a dialog, or with a notice
// painted into a flat area. Words follow the Windows screens. Used by sim-mac.mjs.
export const ACCENT = { light: '#a8560f', dark: '#e08a3c' };
const FONT = '-apple-system,BlinkMacSystemFont,"SF Pro Text","Helvetica Neue","Segoe UI",system-ui,sans-serif';

export const freeCss = (t, acc) => `
.fx,.fx *{box-sizing:border-box}
.fx{position:absolute;z-index:5;color:${t.fg};font:13px/1.4 ${FONT}}
.fx-dim{position:absolute;left:6px;top:6px;right:6px;bottom:6px;border-radius:12px;background:${t.dim};z-index:4}
.fx-sheet{position:absolute;z-index:6;top:52px;left:50%;transform:translateX(-50%);background:${t.sheet};border:1px solid ${t.line};border-radius:14px;box-shadow:0 14px 40px rgba(0,0,0,.34);padding:20px 22px 16px;color:${t.fg};font:13px/1.4 ${FONT}}
.fx-sheet h3{margin:0 0 8px;font:700 15px/20px ${FONT};text-align:center}
.fx-sheet p{margin:0 0 10px;font-size:12px;color:${t.muted};text-align:center}
.fx-lab{font-size:11px;color:${t.muted};margin:8px 0 3px}
.fx-in{border:1px solid ${t.line};border-radius:7px;background:${t.field};padding:5px 9px;font-size:13px}
.fx-btns{display:flex;justify-content:flex-end;gap:10px;margin-top:14px}
.fx-b{display:inline-block;white-space:nowrap;background:${t.btn};padding:4px 14px;border-radius:7px;font-size:13px}
.fx-b.go{background:${acc};color:#fff;font-weight:600}
.fx-row{display:flex;align-items:center;gap:8px;padding:6px 4px;border-bottom:1px solid ${t.line};font-size:12px}
.fx-row b{font-weight:600}.fx-row span{color:${t.muted};margin-left:auto}
.fx-tool{display:flex;gap:6px;align-items:center;margin:10px 0 8px;font-size:12px}
.fx-dot{width:16px;height:16px;border-radius:50%;border:2px solid transparent}.fx-dot.on{border-color:${t.fg}}
.fx-canvas{height:190px;border:1px solid ${t.line};border-radius:8px;background:#fff;overflow:hidden}
.fx-radio{display:flex;align-items:center;gap:9px;padding:5px 2px;font-size:13px}.fx-radio i{width:14px;height:14px;border-radius:50%;border:1.5px solid ${t.muted};display:inline-block}.fx-radio i.on{border:4.5px solid ${acc}}
.fx-note{border:1px solid ${acc};background:${t.pill};border-radius:10px;padding:10px 12px;font-size:12px;line-height:1.45}
.fx-note b{display:block;font-size:13px;margin-bottom:3px}.fx-note .fx-b{margin-top:8px}
.fx-warn{border:1px solid rgba(217,45,32,.5);background:rgba(217,45,32,.09);border-radius:10px;padding:10px 12px;font-size:12px;line-height:1.45}
.fx-h{font:700 17px/22px ${FONT};margin:0 0 8px}
`;

const sheet = (w, inner, btns, extra = '') => () => `<div class="fx-dim"></div><div class="fx-sheet" style="width:${w}px;${extra}">${inner}<div class="fx-btns">${btns.map(([l, go]) => `<span class="fx-b${go ? ' go' : ''}">${l}</span>`).join('')}</div></div>`;
const panel = (x, y, w, inner) => () => `<div class="fx" style="left:${x}px;top:${y}px;width:${w}px">${inner}</div>`;
const sketch = `<svg width="100%" height="190" viewBox="0 0 416 190" fill="none" stroke-linecap="round" stroke-linejoin="round"><path d="M40 130 L40 90 Q40 80 52 80 L260 80 Q276 80 282 92 L300 118 L320 122 Q330 124 330 134 L330 142 L40 142 Z" stroke="#222" stroke-width="3"/><circle cx="92" cy="146" r="14" stroke="#222" stroke-width="3"/><circle cx="256" cy="146" r="14" stroke="#222" stroke-width="3"/><path d="M62 90 H112 V112 H62 Z M128 90 H178 V112 H128 Z M194 90 H244 V112 H194 Z" stroke="#1f5fd0" stroke-width="3"/><path d="M330 60 q10 -24 28 -10 q14 -26 30 -6" stroke="#d92d20" stroke-width="3"/></svg>`;

export const FREE_SIMS = [
  { name: 'ai-sim-vault-sketch', title: 'Idea Vault, drawing a new sketch', base: 'vault-filled',
    html: sheet(460, `<h3>New Sketch</h3><div class="fx-lab">Name (optional)</div><div class="fx-in">&nbsp;</div><div class="fx-tool"><span class="fx-dot on" style="background:#111"></span><span class="fx-dot" style="background:#d92d20"></span><span class="fx-dot" style="background:#1f5fd0"></span>&nbsp;<span class="fx-b">Fine</span><span class="fx-b" style="outline:1.5px solid currentColor">Medium</span><span class="fx-b">Bold</span>&nbsp;<span class="fx-b">Eraser</span><span class="fx-b">Clear</span></div><div class="fx-canvas">${sketch}</div>`, [['Cancel'], ['Save Sketch', 1]]),
    texts: ['New Sketch', 'Name (optional)', 'Black ink', 'Red ink', 'Blue ink', 'Fine', 'Medium', 'Bold', 'Eraser', 'Clear', 'Cancel', 'Save Sketch'] },
  { name: 'ai-sim-vault-file-moved', title: 'Idea Vault, a linked file that moved', base: 'vault-drawer-note', paint: [[[892, 40, 380, 558], [1000, 400]]],
    html: panel(900, 46, 366, `<div class="fx-lab" style="margin:0 0 2px">File</div><div class="fx-h">Harbour notes.txt</div><div class="fx-warn"><b style="display:block;font-size:13px;margin-bottom:3px">File unavailable</b>The original file could not be found at ~/Documents/Research/Harbour notes.txt. Straft keeps this item and its notes. Nothing has been deleted.<div style="margin-top:8px;display:flex;gap:8px"><span class="fx-b">Relink…</span><span class="fx-b">Open Location</span></div></div><div class="fx-lab" style="margin-top:12px">Title</div><div class="fx-in">Harbour notes</div><div class="fx-lab">Caption</div><div class="fx-in">&nbsp;</div><div class="fx-lab">Source link</div><div class="fx-in">&nbsp;</div><div class="fx-lab">Research flag</div><div class="fx-in">No flag</div><div class="fx-lab" style="margin-top:14px">Saved automatically</div>`),
    texts: ['Harbour notes.txt', 'File unavailable', 'The original file could not be found', 'Straft keeps this item and its notes. Nothing has been deleted.', 'Relink…', 'Open Location', 'Title', 'Caption', 'Source link', 'Research flag', 'No flag'] },
  { name: 'ai-sim-scene-number', title: 'Screenplay, changing a scene number used in production', base: 'screenplay-navigator-columns-shown',
    html: sheet(420, `<h3>Scene number</h3><div class="fx-lab">Number</div><div class="fx-in">1A</div><div class="fx-warn" style="margin-top:10px">This scene is already used in production planning (Breakdown, 3 shots and the shooting schedule). Its new number will show everywhere; the production items stay linked to the scene.</div>`, [['Cancel'], ['Change the number', 1]]),
    texts: ['Scene number', 'Number', '1A', 'This scene is already used in production planning (Breakdown, 3 shots and the shooting schedule).', 'Its new number will show everywhere; the production items stay linked to the scene.', 'Cancel', 'Change the number'] },
  { name: 'ai-sim-import-failed', title: 'Screenplay, an import that failed', base: 'screenplay-navigator-columns-shown',
    html: sheet(440, `<h3>Import failed. Your current project was not changed.</h3><p>This file could not be read. Older and other formats such as .doc, .rtf and .pages are not supported. Save the screenplay as Final Draft, Fountain or Word first, or paste its text.</p>`, [['Close'], ['Paste Text Instead', 1]]),
    texts: ['Import failed. Your current project was not changed.', 'Older and other formats such as .doc, .rtf and .pages are not supported.', 'Close', 'Paste Text Instead'] },
  { name: 'ai-sim-breakdown-needs-review', title: 'Breakdown, a scene changed in the script', base: 'breakdown-scenes',
    html: sheet(560, `<h3>Scene 4 changed. Review breakdown differences.</h3><p>INT. KESARGANJ JUNCTION - LOST PROPERTY OFFICE - NIGHT changed in the production source. Its breakdown is marked Needs Review until you decide.</p><div class="fx-row"><b>Added to the script</b><span>Umbrella</span></div><div class="fx-row"><b>No longer in the script</b><span>Cardigan</span></div>`, [['Review Manually'], ['Keep Production Data'], ['Apply Suggested Update', 1]], 'top:46px'),
    texts: ['Scene 4 changed. Review breakdown differences.', 'Needs Review', 'Review Manually', 'Keep Production Data', 'Apply Suggested Update'] },
  { name: 'ai-sim-breakdown-removed-scenes', title: 'Breakdown, a removed scene kept for reference', base: 'breakdown-scenes',
    html: sheet(460, `<h3>Removed scenes</h3><p>Scene removed from current source. Its breakdown is kept for reference.</p><div class="fx-row"><b>Scene 7</b>EXT. KESARGANJ JUNCTION - FOOTBRIDGE - NIGHT<span>4 items kept</span></div><div class="fx-row"><b>Scene 9</b>INT. NIGHT TRAIN - SECOND CLASS COMPARTMENT<span>2 items kept</span></div>`, [['Close', 1]]),
    texts: ['Removed scenes', 'Scene removed from current source. Its breakdown is kept for reference.', 'Close'] },
  { name: 'ai-sim-catalog-export', title: 'Breakdown catalog, exporting it', base: 'breakdown-scenes',
    html: sheet(420, `<h3>Export Catalog</h3><div class="fx-lab">Format</div><div class="fx-radio"><i class="on"></i>PDF</div><div class="fx-radio"><i></i>CSV file</div><div class="fx-radio"><i></i>Excel workbook</div><p style="margin-top:8px;text-align:left">Private notes excluded. Export creates a snapshot and does not change your project.</p>`, [['Cancel'], ['Export…', 1]]),
    texts: ['Export Catalog', 'PDF', 'CSV file', 'Excel workbook', 'Private notes excluded. Export creates a snapshot and does not change your project.', 'Cancel', 'Export…'] },
  { name: 'ai-sim-callsheet-schedule-changed', title: 'Call sheet, the schedule changed after final', base: 'callsheet-finalized', paint: [[[1010, 398, 252, 130], [1030, 560]]],
    html: panel(1012, 408, 250, `<div class="fx-note"><b>The schedule changed after this call sheet was finalized (Times).</b>The final version never changes; start a new revision to update it.<br><span class="fx-b go">Start New Revision</span></div>`),
    texts: ['The schedule changed after this call sheet was finalized (Times).', 'The final version never changes; start a new revision to update it.', 'Start New Revision'] },
  { name: 'ai-sim-callsheet-day-deleted', title: 'Call sheet, its shooting day was deleted', base: 'callsheet-editor', paint: [[[1010, 462, 252, 110], [1030, 590]]],
    html: panel(1012, 470, 250, `<div class="fx-note"><b>The shooting day of this call sheet was deleted.</b>The call sheet stays readable and unchanged.</div>`),
    texts: ['The shooting day of this call sheet was deleted.', 'The call sheet stays readable and unchanged.'] },
  { name: 'ai-sim-trial-reminder', title: 'The 7-day free trial reminder', base: 'project-home',
    html: sheet(420, `<h3>Trial reminder</h3><p style="font-size:15px;color:inherit;font-weight:600;margin-bottom:6px">Your free trial ends in 3 days</p><p>Your projects stay on your computer either way. Nothing is deleted when the trial ends.</p>`, [['Continue trial'], ['Buy full version', 1]]),
    texts: ['Trial reminder', 'Your free trial ends in 3 days', 'Your projects stay on your computer either way. Nothing is deleted when the trial ends.', 'Continue trial', 'Buy full version'] },
];
