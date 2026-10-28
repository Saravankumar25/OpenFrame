#!/usr/bin/env node
// Builds the MacHTML folder from the two runs of the capture tour (Light and Dark).
//
// Usage: node scripts/mac-html/build-mac-html.mjs --both <light run dir> <dark run dir> <MacHTML dir> [commit]
//
// The capture tour (apps/macos/Sources/OpenFrameE2EDriver/Scenarios/CaptureTour*.swift) writes, for every screen,
// capture/<name>.png (the window at 2x, sheets and menus drawn in) and capture/<name>.json (the accessibility tree
// read at the same moment, frames in points from the window's top left). Capture folders are found at any depth, so a
// run made by run-parallel.sh (shard-k folders) works the same as one made by run.sh.
//
// Output: light/NNN-name.html and dark/NNN-name.html (only screens present in both runs, same number in both),
// light/assets and dark/assets (one picture per page plus a small preview), index.html and README.md.
// The pages hold no script and load nothing from the network. No dependencies.

import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';

const args = process.argv.slice(2).filter((a) => a !== '--both');
const [lightRun, darkRun, root, commit = ''] = args;
if (!lightRun || !darkRun || !root) {
  console.error('usage: build-mac-html.mjs --both <light run dir> <dark run dir> <MacHTML dir> [commit]');
  process.exit(2);
}

const esc = (s) => String(s ?? '').replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
const str = (v) => (v === undefined || v === null ? '' : String(v));
const px = (n) => `${+Number(n).toFixed(2)}px`;

// 1. Read every capture of a run.
function readRun(dir) {
  const found = new Map();
  const skipped = [];
  (function walk(d) {
    let entries = [];
    try { entries = fs.readdirSync(d, { withFileTypes: true }); } catch { return; }
    for (const entry of entries) {
      const p = path.join(d, entry.name);
      if (entry.isDirectory()) walk(p);
      else if (entry.name === 'skipped.tsv') {
        for (const line of fs.readFileSync(p, 'utf8').split('\n')) if (line.trim()) skipped.push(line.replace('\t', ': '));
      } else if (entry.name.endsWith('.json') && path.basename(d) === 'capture') {
        try {
          const meta = JSON.parse(fs.readFileSync(p, 'utf8'));
          const png = p.replace(/\.json$/, '.png');
          if (meta.name && fs.existsSync(png)) {
            meta.pngPath = png;
            meta.elements = sheetsApart((meta.elements || []).map(normalise));
            // The same name written twice keeps the later file.
            const old = found.get(meta.name);
            if (!old || fs.statSync(old.pngPath).mtimeMs <= fs.statSync(png).mtimeMs) found.set(meta.name, meta);
          }
        } catch (e) {
          console.error('skipped an unreadable capture', p, e.message);
        }
      }
    }
  })(dir);
  return { pages: found, skipped };
}

// The main window's tree also lists an open sheet (AXSheet and its children). When the sheet window was read on its
// own those copies are dropped; otherwise they are moved to the sheet window.
function sheetsApart(list) {
  const hasSheetWindow = list.some((e) => e.window === 'sheet');
  const out = [];
  for (let i = 0; i < list.length; i++) {
    const e = list[i];
    if (e.window === 'main' && e.role === 'AXSheet') {
      let j = i + 1;
      while (j < list.length && list[j].window === 'main' && list[j].depth > e.depth) j++;
      if (!hasSheetWindow) for (const x of list.slice(i, j)) out.push({ ...x, window: 'sheet' });
      i = j - 1;
      continue;
    }
    out.push(e);
  }
  return out;
}

function normalise(e) {
  return {
    role: str(e.role), subrole: str(e.subrole), id: str(e.id), label: str(e.label), title: str(e.title), value: str(e.value),
    enabled: e.enabled !== false, selected: e.selected === true, focused: e.focused === true,
    window: str(e.window) || 'main', depth: Number(e.depth) || 0,
    x: Number(e.x) || 0, y: Number(e.y) || 0, w: Math.max(0, Number(e.w) || 0), h: Math.max(0, Number(e.h) || 0),
    options: Array.isArray(e.options) ? e.options.map(str) : null,
  };
}

const light = readRun(lightRun);
const dark = readRun(darkRun);

// 2. The screens present in both runs, in tour order. A screen whose picture or whose texts and controls are the same
// as an earlier screen's (a tab that changed nothing) is dropped and links to it go to the earlier one.
let names = [...light.pages.keys()].filter((n) => dark.pages.has(n));
names.sort((a, b) => (light.pages.get(a).order ?? 0) - (light.pages.get(b).order ?? 0) || a.localeCompare(b));
const alias = new Map();
const seen = new Map();
const SIG_ROLES = new Set(['AXStaticText', 'AXHeading', 'AXButton', 'AXRadioButton', 'AXMenuItem', 'AXTextField', 'AXTextArea', 'AXCheckBox', 'AXPopUpButton']);
names = names.filter((n) => {
  const keys = [];
  for (const m of [light.pages.get(n), dark.pages.get(n)]) {
    keys.push(createHash('md5').update(fs.readFileSync(m.pngPath)).digest('hex'));
  }
  const p = light.pages.get(n);
  const text = p.elements.filter((e) => SIG_ROLES.has(e.role)).map((e) => `${e.role}|${e.label}|${e.title}|${e.value}|${e.selected}`).join('\n');
  keys.push('sig:' + createHash('md5').update(`${p.width}x${p.height}|${p.sheets}|` + text).digest('hex'));
  for (const k of keys) if (seen.has(k)) { alias.set(n, seen.get(k)); return false; }
  for (const k of keys) seen.set(k, n);
  return true;
});
const resolve = (n) => { while (n && alias.has(n)) n = alias.get(n); return n; };
if (names.length === 0) {
  console.error(`no screen is present in both runs (light ${light.pages.size}, dark ${dark.pages.size})`);
  process.exit(1);
}
const onlyOne = [...new Set([...light.pages.keys(), ...dark.pages.keys()])].filter((n) => !(light.pages.has(n) && dark.pages.has(n)));

const meta = names.map((n, i) => {
  const p = light.pages.get(n);
  const num = String(i + 1).padStart(3, '0');
  return { name: n, num, file: `${num}-${n}.html`, title: p.title || n, area: p.area || 'Other', from: resolve(p.from), via: p.via || '' };
});
const metaByName = new Map(meta.map((m) => [m.name, m]));

// Pages reached from a page: target.from is this page, target.via names the control that was used.
const children = new Map();
for (const m of meta) {
  if (!m.from || !metaByName.has(m.from) || m.from === m.name) continue;
  if (!children.has(m.from)) children.set(m.from, []);
  children.get(m.from).push(m);
}
// The sidebar leads to the first page of each workspace.
const areaOfKey = {
  home: 'Project Home', screenplay: 'Screenplay', story: 'Story', ideaVault: 'Idea Vault', breakdown: 'Breakdown',
  schedule: 'Schedule', callSheets: 'Call Sheets', budget: 'Budget', visual: 'Visual Planning', files: 'Files',
  notes: 'Notes and Tasks', activity: 'Activity', recentlyDeleted: 'Recently Deleted',
};
const firstOfArea = new Map();
for (const m of meta) if (!firstOfArea.has(m.area)) firstOfArea.set(m.area, m);

// 3. Regions: the window picture is cut into the main containers. Each region shows its own part of the one picture
// (background-position), so the regions together give back the picture pixel for pixel.
function regionsOf(p) {
  const W = Math.round(p.width), H = Math.round(p.height);
  const main = p.elements.filter((e) => e.window === 'main');
  const area = (e) => e.w * e.h;
  const biggest = (list) => list.reduce((a, b) => (!a || area(b) > area(a) ? b : a), null);
  const clamp = (v, lo, hi) => Math.min(hi, Math.max(lo, v));
  const toolbar = biggest(main.filter((e) => e.role === 'AXToolbar' && e.w > W / 2));
  const tb = toolbar ? clamp(Math.round(toolbar.y + toolbar.h), 0, H) : 0;
  const split = biggest(main.filter((e) => e.role === 'AXSplitGroup' && e.w > W / 2));
  const scroll = biggest(main.filter((e) => e.role === 'AXScrollArea' && e.w > W / 2));
  const body = split || scroll;
  let bottom = body ? clamp(Math.round(body.y + body.h), tb + 1, H) : H;
  if (H - bottom < 8) bottom = H;
  // Columns from the split group's direct children.
  let kids = [];
  if (split) {
    const i = p.elements.indexOf(split);
    for (let j = i + 1; j < p.elements.length && p.elements[j].depth > split.depth; j++) {
      const e = p.elements[j];
      if (e.depth === split.depth + 1 && e.role !== 'AXSplitter' && e.w >= 40 && e.h >= 40) kids.push(e);
    }
    kids.sort((a, b) => a.x - b.x);
  }
  const cols = [];
  if (kids.length < 2) cols.push({ x0: 0, x1: W, full: false });
  else {
    let x0 = 0;
    kids.forEach((k, i) => {
      const x1 = i === kids.length - 1 ? W : clamp(Math.round((k.x + k.w + kids[i + 1].x) / 2), x0 + 1, W);
      cols.push({ x0, x1, full: k.y < tb - 4, w: k.w });
      x0 = x1;
    });
  }
  const colName = (i) => {
    if (cols.length === 1) return 'content';
    if (i === 0 && cols[0].w <= 360) return 'sidebar';
    const contentIndex = cols.length > 1 && cols[0].w <= 360 ? 1 : 0;
    if (i === contentIndex) return 'content';
    if (i === cols.length - 1) return 'inspector';
    return `content-${i}`;
  };
  const regions = [];
  // Toolbar band over the columns that do not reach the top, joined where they touch.
  if (tb > 0) {
    let run = null;
    for (const c of cols) {
      if (!c.full) { if (run && run.x1 === c.x0) run.x1 = c.x1; else regions.push((run = { kind: 'toolbar', x0: c.x0, x1: c.x1 })); }
      else run = null;
    }
  }
  const out = [];
  for (const r of regions) out.push({ kind: 'toolbar', label: 'Toolbar', x: r.x0, y: 0, w: r.x1 - r.x0, h: tb });
  cols.forEach((c, i) => {
    const kind = colName(i);
    const y0 = c.full ? 0 : tb;
    out.push({ kind, label: kind === 'sidebar' ? 'Sidebar' : kind === 'inspector' ? 'Inspector' : 'Content', x: c.x0, y: y0, w: c.x1 - c.x0, h: bottom - y0 });
  });
  if (bottom < H) out.push({ kind: 'statusbar', label: 'Status bar', x: 0, y: bottom, w: W, h: H - bottom });
  // Sheets and menus lie over the base regions; they show the same picture pixels, so the overlap is exact.
  const sheets = p.elements.filter((e) => e.window === 'sheet' && e.role === 'AXSheet');
  if (sheets.length) sheets.forEach((s, i) => out.push({ kind: 'sheet', label: 'Sheet', x: s.x, y: s.y, w: s.w, h: s.h, el: s, n: i }));
  else {
    const se = p.elements.filter((e) => e.window === 'sheet' && e.w > 0);
    if (se.length) out.push({ kind: 'sheet', label: 'Sheet', ...bbox(se, 0, W, H) });
  }
  const me = p.elements.filter((e) => e.window === 'menu' && e.w > 0);
  if (me.length) out.push({ kind: 'menu', label: 'Menu', ...bbox(me, 6, W, H) });
  for (const r of out) { r.x = Math.round(r.x); r.y = Math.round(r.y); r.w = Math.round(r.w); r.h = Math.round(r.h); }
  return out.filter((r) => r.w > 0 && r.h > 0);
}
function bbox(list, pad, W, H) {
  const x0 = Math.max(0, Math.min(...list.map((e) => e.x)) - pad), y0 = Math.max(0, Math.min(...list.map((e) => e.y)) - pad);
  const x1 = Math.min(W, Math.max(...list.map((e) => e.x + e.w)) + pad), y1 = Math.min(H, Math.max(...list.map((e) => e.y + e.h)) + pad);
  return { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
}

// 4. The HTML layer: the accessibility tree as real elements, each at its own frame.
function treeOf(list) {
  const roots = [], stack = [];
  for (const e of list) {
    const node = { e, kids: [] };
    while (stack.length && stack[stack.length - 1].e.depth >= e.depth) stack.pop();
    (stack.length ? stack[stack.length - 1].kids : roots).push(node);
    stack.push(node);
  }
  return roots;
}

const LEAF = new Set(['AXButton', 'AXMenuButton', 'AXPopUpButton', 'AXTextField', 'AXSearchField', 'AXSecureTextField', 'AXTextArea', 'AXComboBox',
  'AXCheckBox', 'AXRadioButton', 'AXLink', 'AXSlider', 'AXIncrementor', 'AXDisclosureTriangle']);
const LISTS = { AXList: 'list', AXOutline: 'tree', AXMenu: 'menu', AXOpaqueProviderGroup: 'list' };
const ITEM_ROLE = { list: 'listitem', tree: 'treeitem', menu: 'none' };
const truthy = (v) => v === '1' || v === 'true' || v === 'on';

function makeRenderer(p, mode) {
  const kids = children.get(p.name) || [];
  const used = new Set();
  let radioGroup = 0;
  function targetOf(e) {
    for (const k of [e.id, e.label, e.title]) {
      if (!k) continue;
      const t = kids.find((m) => m.via === k);
      if (t) { used.add(t.name); return t; }
    }
    const sb = /^sidebar-(.+)$/.exec(e.id);
    if (sb && areaOfKey[sb[1]] && firstOfArea.has(areaOfKey[sb[1]])) {
      const t = firstOfArea.get(areaOfKey[sb[1]]);
      if (t.name !== p.name) return t;
    }
    return null;
  }
  function common(e, extraClass = '') {
    const a = [`class="el ${e.role.replace(/^AX/, '').toLowerCase()}${extraClass ? ' ' + extraClass : ''}"`];
    if (e.id) a.push(`data-id="${esc(e.id)}"`);
    a.push(`data-role="${esc(e.role)}${e.subrole ? '/' + esc(e.subrole) : ''}"`);
    if (e.label) a.push(`aria-label="${esc(e.label)}"`);
    if (e.title && e.title !== e.label) a.push(`title="${esc(e.title)}"`);
    if (e.selected) a.push('aria-selected="true"');
    if (e.focused) a.push('data-focused="true"');
    return a;
  }
  // ox, oy: the origin of the element's HTML parent, in window points.
  function render(node, ox, oy, radioName, inList) {
    const e = node.e;
    const style = `left:${px(e.x - ox)};top:${px(e.y - oy)};width:${px(e.w)};height:${px(e.h)}`;
    const a = common(e);
    const off = !e.enabled;
    const text = e.value || e.label || e.title;
    const name = e.label || e.title || e.value;
    let open, close = '', inner = '', leaf = LEAF.has(e.role);
    let childRadio = radioName;
    const list = LISTS[e.role] && !(e.role === 'AXOpaqueProviderGroup' && !/List|Grid/.test(e.subrole)) ? LISTS[e.role] : null;
    const target = targetOf(e);
    switch (e.role) {
      case 'AXStaticText': {
        const multi = e.h > 26;
        const fs = multi ? 13 : Math.min(Math.max(e.h * 0.76, 9), 26);
        const parentText = node.parentText;
        if (parentText && parentText === text) return '';
        open = `<span ${a.join(' ')} style="${style};font-size:${px(fs)};line-height:${px(multi ? 16 : e.h)}">`;
        inner = esc(text); close = '</span>';
        leaf = true; break;
      }
      case 'AXHeading': {
        const fs = Math.min(Math.max(e.h * 0.72, 11), 30);
        open = `<h2 ${a.join(' ')} style="${style};font-size:${px(fs)};line-height:${px(e.h)}">`;
        inner = esc(text); close = '</h2>'; break;
      }
      case 'AXButton': case 'AXMenuButton': case 'AXDisclosureTriangle': {
        const extra = e.role === 'AXMenuButton' ? ' aria-haspopup="menu"' : e.role === 'AXDisclosureTriangle' ? ` aria-expanded="${truthy(e.value)}"` : '';
        if (target) {
          open = `<a ${a.join(' ')} role="button" href="${esc(target.file)}" style="${style}"${extra}${off ? ' aria-disabled="true"' : ''}>`;
          inner = esc(name); close = '</a>';
        } else {
          open = `<button type="button" ${a.join(' ')} style="${style}"${extra}${off ? ' disabled' : ''}>`;
          inner = esc(name); close = '</button>';
        }
        break;
      }
      case 'AXLink': {
        open = `<a ${a.join(' ')}${target ? ` href="${esc(target.file)}"` : ''} style="${style}"${off ? ' aria-disabled="true"' : ''}>`;
        inner = esc(name); close = '</a>'; break;
      }
      case 'AXPopUpButton': {
        const opts = e.options && e.options.length ? e.options : [e.value || e.title || e.label];
        if (!opts.includes(e.value) && e.value) opts.unshift(e.value);
        open = `<select ${a.join(' ')} style="${style}"${off ? ' disabled' : ''}>`;
        inner = opts.map((o) => `<option${o === e.value ? ' selected' : ''}>${esc(o)}</option>`).join('');
        close = '</select>'; break;
      }
      case 'AXTextField': case 'AXSearchField': case 'AXSecureTextField': case 'AXComboBox': {
        const type = e.role === 'AXSecureTextField' ? 'password' : e.role === 'AXSearchField' || e.subrole === 'AXSearchField' ? 'search' : 'text';
        if (e.h > 40) {
          open = `<textarea ${a.join(' ')} style="${style}"${off ? ' disabled' : ''}>`;
          inner = esc(e.value); close = '</textarea>';
        } else {
          open = `<input type="${type}" ${a.join(' ')} value="${esc(e.value)}" style="${style}"${off ? ' disabled' : ''}>`;
        }
        break;
      }
      case 'AXTextArea':
        open = `<textarea ${a.join(' ')} style="${style}"${off ? ' disabled' : ''}>`;
        inner = esc(e.value); close = '</textarea>'; break;
      case 'AXCheckBox': {
        const sw = e.subrole === 'AXToggle' ? ' role="switch"' : '';
        open = `<input type="checkbox"${sw} ${a.join(' ')} value="${esc(e.value)}" style="${style}"${truthy(e.value) ? ' checked' : ''}${off ? ' disabled' : ''}>`;
        break;
      }
      case 'AXRadioButton': {
        const nm = radioName || `radio-${++radioGroup}`;
        open = `<input type="radio" name="${nm}" ${a.join(' ')} value="${esc(e.value)}" style="${style}"${e.selected || truthy(e.value) ? ' checked' : ''}${off ? ' disabled' : ''}>`;
        break;
      }
      case 'AXRadioGroup':
        childRadio = `radio-${++radioGroup}`;
        open = `<div role="radiogroup" ${a.join(' ')} style="${style}">`; close = '</div>'; break;
      case 'AXTabGroup':
        open = `<div role="tablist" ${a.join(' ')} style="${style}">`; close = '</div>'; break;
      case 'AXMenuItem': {
        const tag = inList || e.window === 'menu' ? 'li' : 'div';
        open = `<${tag} role="menuitem" ${a.join(' ')} style="${style}"${off ? ' aria-disabled="true"' : ''}>`;
        inner = esc(name); close = `</${tag}>`; leaf = true; break;
      }
      case 'AXTable':
        open = `<div role="table" ${a.join(' ')} style="${style}">`; close = '</div>'; break;
      case 'AXRow':
        open = `<div role="row" ${a.join(' ')} style="${style}">`; close = '</div>'; break;
      case 'AXCell':
        open = `<div role="cell" ${a.join(' ')} style="${style}">`; close = '</div>'; break;
      case 'AXToolbar':
        open = `<div role="toolbar" ${a.join(' ')} style="${style}">`; close = '</div>'; break;
      case 'AXSplitter':
        open = `<div role="separator" ${a.join(' ')}${e.value ? ` aria-valuenow="${esc(e.value)}"` : ''} style="${style}">`; close = '</div>'; break;
      case 'AXScrollBar': case 'AXValueIndicator': case 'AXSlider': case 'AXIncrementor': {
        const r = e.role === 'AXScrollBar' ? 'scrollbar' : e.role === 'AXIncrementor' ? 'spinbutton' : 'slider';
        open = `<div role="${r}" ${a.join(' ')} aria-valuenow="${esc(e.value)}" style="${style}">`; close = '</div>'; break;
      }
      case 'AXBusyIndicator':
        open = `<div role="progressbar" ${a.join(' ')} style="${style}">`; close = '</div>'; break;
      case 'AXImage':
        open = `<div role="img" ${a.join(' ')} style="${style}">`; close = '</div>'; break;
      case 'AXSheet':
        open = `<div role="dialog" ${a.join(' ')} style="${style}">`; close = '</div>'; break;
      default: {
        if (list) {
          const tag = list === 'menu' ? 'menu' : 'ul';
          open = `<${tag} role="${list}" ${a.join(' ')} style="${style}">`; close = `</${tag}>`;
        } else {
          const role = e.label && e.role === 'AXGroup' ? ' role="group"' : '';
          open = `<div${role} ${a.join(' ')} style="${style}">`; close = '</div>';
          if (e.value && !node.kids.length) inner = `<span class="v">${esc(e.value)}</span>`;
          // A container with children (the Beat Board says "Vertical layout") keeps its value as a tooltip.
          else if (e.value && e.value !== e.label && !e.title) open = open.replace('<div', `<div title="${esc(e.value)}"`);
        }
      }
    }
    // Children: inside the element when the tag allows it, otherwise next to it.
    let kidsHtml = '';
    let after = '';
    const itemRole = list ? ITEM_ROLE[list] : null;
    for (const k of node.kids) {
      if (e.role === 'AXButton' || e.role === 'AXLink') k.parentText = name;
      if (leaf || e.role === 'AXHeading') {
        after += render(k, ox, oy, childRadio, false);
      } else if (list) {
        const ke = k.e;
        const ks = `left:${px(ke.x - e.x)};top:${px(ke.y - e.y)};width:${px(ke.w)};height:${px(ke.h)}`;
        kidsHtml += `<li class="el item" role="${itemRole}" style="${ks}">${render(k, ke.x, ke.y, childRadio, false)}</li>`;
      } else {
        kidsHtml += render(k, e.x, e.y, childRadio, false);
      }
    }
    // A shortcut or state shown next to a button or menu item title.
    if (leaf && close && e.value && e.value !== name && !['AXTextArea', 'AXTextField', 'AXSearchField', 'AXSecureTextField', 'AXComboBox', 'AXPopUpButton', 'AXStaticText'].includes(e.role)) {
      inner += `<span class="key">${esc(e.value)}</span>`;
    }
    // A control whose own texts follow it as elements shows only those texts in the HTML only view.
    if (leaf && close && node.kids.length) open = open.replace('class="el ', 'class="el haskids ');
    let html;
    if (!close) html = open; // a void element (input)
    else if (leaf || e.role === 'AXHeading') html = open + inner + close;
    else html = open + inner + kidsHtml + close;
    // A control that leads to another captured screen but is not a button gets a link laid over it.
    if (target && !['AXButton', 'AXMenuButton', 'AXDisclosureTriangle', 'AXLink'].includes(e.role)) {
      after += `<a class="el go" href="${esc(target.file)}" aria-label="${esc((name || target.title) + ': ' + target.title)}" style="${style}"></a>`;
    }
    return html + after;
  }
  return { render, used };
}

// Place the trees into regions: a node goes into the region that holds most of it; a node spread over several regions
// is split and its children are placed one by one.
function overlap(e, r) {
  const w = Math.min(e.x + e.w, r.x + r.w) - Math.max(e.x, r.x), h = Math.min(e.y + e.h, r.y + r.h) - Math.max(e.y, r.y);
  return w > 0 && h > 0 ? w * h : 0;
}
function place(nodes, regions, into, loose) {
  for (const node of nodes) {
    const e = node.e;
    const a = Math.max(e.w * e.h, 1);
    let best = null, bestShare = 0;
    for (const r of regions) {
      const share = overlap(e, r) / a;
      if (share > bestShare) { best = r; bestShare = share; }
    }
    if (e.w * e.h === 0) {
      const cx = e.x, cy = e.y;
      best = regions.find((r) => cx >= r.x && cx <= r.x + r.w && cy >= r.y && cy <= r.y + r.h) || regions[0];
      bestShare = 1;
    }
    if (best && (bestShare >= 0.8 || (!node.kids.length && bestShare > 0))) into.get(best).push(node);
    else {
      if (e.id || e.label || e.value) loose.push(node);
      place(node.kids, regions, into, loose);
    }
  }
}

const CSS = `
:root{color-scheme:light;--page:#e4e4e8;--fg:#1d1d1f;--mut:#6b6b73;--card:#f7f7f9;--line:#c8c8ce;--acc:#b85f00}
:root.dark{color-scheme:dark;--page:#232326;--fg:#f2f2f4;--mut:#a0a0a8;--card:#1c1c1f;--line:#3c3c42;--acc:#ff9f0a}
*{box-sizing:border-box}
html{background:var(--page)}
body{margin:0;background:var(--page);color:var(--fg);font:14px/1.4 -apple-system,BlinkMacSystemFont,"SF Pro Text","Helvetica Neue",sans-serif}
a{color:var(--acc)}
.toggle{position:absolute;opacity:0;pointer-events:none}
header.top{display:flex;flex-wrap:wrap;gap:6px 14px;align-items:center;padding:12px 20px;border-bottom:1px solid var(--line);background:var(--card)}
header.top h1{font-size:17px;margin:0}
header.top .meta{color:var(--mut);font-size:13px}
header.top nav{display:flex;gap:8px;flex-wrap:wrap;margin-left:auto}
header.top nav a,header.top label{padding:3px 10px;border:1px solid var(--line);border-radius:6px;text-decoration:none;color:var(--fg);font-size:13px;cursor:pointer;background:var(--page)}
#htmlonly:checked~header label[for=htmlonly]{background:var(--acc);color:#fff;border-color:var(--acc)}
#htmlonly:focus-visible~header label[for=htmlonly]{outline:2px solid var(--acc)}
.leads{padding:8px 20px 0;font-size:13px;color:var(--mut)}
.leads a{margin-right:10px}
/* The band has a fixed height and the window a fixed left edge, so the picture always lands on whole points. */
.band{height:104px;overflow:auto;border-bottom:1px solid var(--line);background:var(--card)}
.band header.top{border-bottom:0}
.wrap{padding:32px 40px 64px;overflow:auto}
.win{position:relative;margin:0;border-radius:10px;background:var(--winbg);box-shadow:0 0 0 .5px rgba(0,0,0,.35),0 10px 30px rgba(0,0,0,.28),0 24px 70px rgba(0,0,0,.22)}
.region{position:absolute;display:block;margin:0;padding:0;border:0;background-repeat:no-repeat;color:inherit;max-width:none;max-height:none;list-style:none}
.el{position:absolute;margin:0;padding:0;border:0;border-radius:0;background:transparent;color:transparent;-webkit-text-fill-color:transparent;font:inherit;font-size:13px;line-height:1.2;text-align:left;
 overflow:hidden;text-decoration:none;appearance:none;-webkit-appearance:none;outline:none;box-shadow:none;list-style:none;white-space:pre;caret-color:transparent;resize:none;min-width:0;min-height:0}
.el::selection{background:rgba(255,149,0,.35)}
.el .v{display:block;padding:0 4px}
.el .key{float:right;padding:0 6px;opacity:.7}
.statictext{cursor:text;user-select:text}
.statictext[style*="line-height:16px"]{white-space:pre-wrap}
textarea.el{white-space:pre-wrap}
button.el,a.el,select.el,input[type=checkbox].el,input[type=radio].el{cursor:pointer}
a.go{z-index:1}
button.el:focus-visible,a.el:focus-visible,select.el:focus-visible,input.el:focus-visible,textarea.el:focus-visible{outline:2px solid var(--acc);outline-offset:-2px}
.loose{pointer-events:none}
/* HTML only: the picture is hidden and the HTML layer draws the screen on the window colour. */
#htmlonly:checked~.wrap .region{background-image:none!important;background-color:var(--winbg);box-shadow:inset 0 0 0 .5px rgba(128,128,128,.35)}
#htmlonly:checked~.wrap .win{background:var(--winbg);overflow:hidden}
#htmlonly:checked~.wrap .region.sheet,#htmlonly:checked~.wrap .region.menu{border-radius:10px;box-shadow:0 0 0 1px rgba(128,128,128,.5),0 8px 30px rgba(0,0,0,.35)}
#htmlonly:checked~.wrap .el{color:var(--winfg);-webkit-text-fill-color:var(--winfg);overflow:visible;text-overflow:ellipsis}
#htmlonly:checked~.wrap button.el,#htmlonly:checked~.wrap a[role=button].el,#htmlonly:checked~.wrap select.el,#htmlonly:checked~.wrap input.el,#htmlonly:checked~.wrap textarea.el,#htmlonly:checked~.wrap .menubutton{
 overflow:hidden;border:1px solid rgba(128,128,128,.45);border-radius:5px;background:rgba(128,128,128,.12);padding:0 5px;display:flex;align-items:center;font-size:12px}
#htmlonly:checked~.wrap textarea.el{display:block;padding:4px 6px}
#htmlonly:checked~.wrap .el.haskids{color:transparent;-webkit-text-fill-color:transparent}
#htmlonly:checked~.wrap input[type=checkbox].el,#htmlonly:checked~.wrap input[type=radio].el{appearance:auto;-webkit-appearance:auto;border:0;background:none;padding:0;margin:0;width:14px!important;height:14px!important}
#htmlonly:checked~.wrap [disabled].el,#htmlonly:checked~.wrap [aria-disabled=true].el{opacity:.45}
#htmlonly:checked~.wrap [aria-selected=true].el{background:rgba(10,132,255,.22)}
#htmlonly:checked~.wrap .el[data-focused]{outline:2px solid rgba(10,132,255,.7);outline-offset:-2px}
#htmlonly:checked~.wrap a.el{color:var(--acc);-webkit-text-fill-color:var(--acc)}
#htmlonly:checked~.wrap a.go{background:rgba(255,149,0,.08)}
#htmlonly:checked~.wrap .scrollbar,#htmlonly:checked~.wrap .valueindicator{display:none}
`;

const INDEX_CSS = `
.sw{display:inline-flex;border:1px solid var(--line);border-radius:8px;overflow:hidden}
.sw label{padding:4px 16px;font-size:13px;cursor:pointer;border:0;border-radius:0}
#m-light:checked~header label[for=m-light],#m-dark:checked~header label[for=m-dark]{background:var(--acc);color:#fff}
#m-light:checked~main .set-dark,#m-dark:checked~main .set-light{display:none}
#m-dark:checked~*{--page:#232326;--fg:#f2f2f4;--mut:#a0a0a8;--card:#1c1c1f;--line:#3c3c42;--acc:#ff9f0a}
main{padding:16px 20px 40px;max-width:1500px;margin:0 auto;background:var(--page)}
main h2{margin:26px 0 10px;font-size:16px;border-bottom:1px solid var(--line);padding-bottom:4px}
.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(230px,1fr));gap:14px}
.card{background:var(--card);border:1px solid var(--line);border-radius:10px;overflow:hidden;text-decoration:none;color:var(--fg);display:block}
.card:hover{border-color:var(--acc)}
.card img{width:100%;display:block;aspect-ratio:2/1;object-fit:cover;object-position:top left}
.card div{padding:7px 10px;font-size:13px}
.card small{color:var(--mut);display:block}
`;

// 5. Write the pages.
fs.rmSync(root, { recursive: true, force: true });
const sizes = new Map();
for (const mode of ['light', 'dark']) {
  const run = mode === 'light' ? light : dark;
  const other = mode === 'light' ? 'dark' : 'light';
  const out = path.join(root, mode);
  fs.mkdirSync(path.join(out, 'assets', 'thumbs'), { recursive: true });
  const checkList = [];
  for (const m of meta) {
    const p = run.pages.get(m.name);
    const W = Math.round(p.width), H = Math.round(p.height);
    const scale = Number(p.scale) || 2;
    if (mode === 'light') sizes.set(`${W} x ${H}`, (sizes.get(`${W} x ${H}`) || 0) + 1);
    const pic = `assets/${m.num}-${m.name}.png`;
    fs.copyFileSync(p.pngPath, path.join(out, pic));
    const regions = regionsOf(p);
    const into = new Map(regions.map((r) => [r, []]));
    const loose = [];
    for (const win of ['main', 'sheet', 'menu']) {
      const list = p.elements.filter((e) => e.window === win);
      if (!list.length) continue;
      const pool = regions.filter((r) => (win === 'main' ? !['sheet', 'menu'].includes(r.kind) : r.kind === win));
      let roots = treeOf(list);
      // A sheet region stands for the AXSheet element itself: its children go straight into the region.
      if (win === 'sheet') roots = roots.flatMap((n) => (n.e.role === 'AXSheet' ? n.kids : [n]));
      if (pool.length) place(roots, pool, into, loose);
    }
    const { render, used } = makeRenderer(p, mode);
    const tagOf = { toolbar: 'header', sidebar: 'nav', content: 'main', inspector: 'aside', statusbar: 'footer', sheet: 'div', menu: 'menu' };
    const regionHtml = regions.map((r) => {
      const tag = r.kind.startsWith('content-') ? 'section' : tagOf[r.kind] || 'section';
      const role = r.kind === 'sheet' ? ' role="dialog" aria-modal="true"' : r.kind === 'menu' ? ' role="menu"' : '';
      const label = r.kind === 'sheet' && r.el && r.el.label ? r.el.label : r.label;
      const st = `left:${r.x}px;top:${r.y}px;width:${r.w}px;height:${r.h}px;background-image:url(${pic});background-size:${W}px ${H}px;background-position:${-r.x}px ${-r.y}px`;
      const body = into.get(r).map((n) => render(n, r.x, r.y, null, r.kind === 'menu')).join('\n');
      return `<${tag} class="region ${r.kind.replace(/-\d+$/, '')}" data-region="${r.kind}"${role} aria-label="${esc(label)}" style="${st}">\n${body}\n</${tag}>`;
    }).join('\n');
    const looseHtml = loose.map((n) => {
      const e = n.e;
      return `<div class="el loose" data-role="${esc(e.role)}"${e.id ? ` data-id="${esc(e.id)}"` : ''}${e.label ? ` aria-label="${esc(e.label)}"` : ''}${e.title && e.title !== e.label ? ` title="${esc(e.title)}"` : ''} style="left:${px(e.x)};top:${px(e.y)};width:${px(e.w)};height:${px(e.h)}">${e.value ? `<span class="v">${esc(e.value)}</span>` : ''}</div>`;
    }).join('\n');
    const kidsHere = children.get(m.name) || [];
    const from = m.from && metaByName.get(m.from);
    const dk = p.dark === true || (p.dark === undefined && mode === 'dark');
    const winbg = /^#[0-9a-f]{3,8}$/i.test(p.bg || '') ? p.bg : dk ? '#1e1e1e' : '#f5f5f5';
    const html = `<!doctype html>
<html lang="en"${mode === 'dark' ? ' class="dark"' : ''}><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>${esc(m.title)} (${mode}) - OpenFrame Studio for Mac</title><style>${CSS}</style></head>
<body>
<input type="checkbox" id="htmlonly" class="toggle">
<div class="band">
<header class="top">
<h1>${esc(m.title)}</h1>
<span class="meta">${esc(m.area)} &middot; page ${m.num} of ${meta.length} &middot; ${mode} &middot; window ${W} &times; ${H} points at ${scale}x</span>
<nav>
<a href="../index.html">All pages</a>
<a href="../${other}/${esc(m.file)}">Same page in ${other}</a>
${from ? `<a href="${esc(from.file)}">Back to ${esc(from.title)}</a>` : ''}
<label for="htmlonly">HTML only</label>
</nav>
</header>
${kidsHere.length ? `<div class="leads">Leads to: ${kidsHere.map((k) => `<a href="${esc(k.file)}">${esc(k.title)}</a>`).join(' ')}</div>` : ''}
</div>
<div class="wrap"><div class="win" role="application" aria-label="${esc(p.windowTitle || 'OpenFrame Studio')} window" style="width:${W}px;height:${H}px;--winbg:${esc(winbg)};--winfg:${dk ? '#f2f2f4' : '#1d1d1f'}">
${looseHtml}
${regionHtml}
</div></div>
</body></html>
`;
    fs.writeFileSync(path.join(out, m.file), html);
    // What the check looks for in the HTML layer: every label and value of the tree.
    const expect = [];
    for (const e of p.elements) for (const s of [e.label, e.value]) if (s && s.trim()) expect.push(s);
    checkList.push({ n: m.num, name: m.name, file: m.file, title: m.title, area: m.area, picture: pic, width: W, height: H, scale, expect: [...new Set(expect)] });
    void used;
  }
  fs.writeFileSync(path.join(out, 'pages.json'), JSON.stringify(checkList));
  // Small previews for the index, made with sips (part of macOS).
  const pics = checkList.map((c) => path.join(out, c.picture));
  for (let i = 0; i < pics.length; i += 60) {
    const r = spawnSync('sips', ['-Z', '520', ...pics.slice(i, i + 60), '--out', path.join(out, 'assets', 'thumbs')], { encoding: 'utf8' });
    if (r.status !== 0) console.error('sips could not make previews:', (r.stderr || '').slice(0, 300));
  }
}

// 6. The index: every page grouped by area, with a Light and Dark switch made in CSS.
const groups = [];
for (const m of meta) { let g = groups.find((x) => x.area === m.area); if (!g) groups.push((g = { area: m.area, pages: [] })); g.pages.push(m); }
const slug = (s) => s.replace(/\W+/g, '-');
const index = `<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>OpenFrame Studio for Mac, every screen</title><style>${CSS}${INDEX_CSS}</style></head>
<body>
<input type="radio" name="mode" id="m-light" class="toggle" checked>
<input type="radio" name="mode" id="m-dark" class="toggle">
<header class="top"><h1>OpenFrame Studio for Mac: every screen</h1>
<span class="meta">${meta.length} screens in ${groups.length} areas, each in Light and in Dark${commit ? ' &middot; commit ' + esc(commit) : ''}</span>
<nav><span class="sw" role="group" aria-label="Appearance"><label for="m-light">Light</label><label for="m-dark">Dark</label></span></nav></header>
<main>
<p class="meta" style="color:var(--mut)">Each page is the picture of the running app, cut into its main parts, with the app's accessibility tree laid over it as real HTML. Every page has an HTML only switch.</p>
${['light', 'dark'].map((mode) => `<div class="set-${mode}">
<nav>${groups.map((g) => `<a href="#${mode}-${slug(g.area)}">${esc(g.area)} (${g.pages.length})</a>`).join(' &middot; ')}</nav>
${groups.map((g) => `<h2 id="${mode}-${slug(g.area)}">${esc(g.area)} <small>(${g.pages.length})</small></h2>
<div class="grid">${g.pages.map((m) => `<a class="card" href="${mode}/${esc(m.file)}"><img loading="lazy" src="${mode}/assets/thumbs/${m.num}-${esc(m.name)}.png" alt=""><div>${m.num} ${esc(m.title)}<small>${esc(m.name)}</small></div></a>`).join('')}</div>`).join('\n')}
</div>`).join('\n')}
</main>
</body></html>
`;
fs.writeFileSync(path.join(root, 'index.html'), index);

// 7. README.
// A step skipped in both modes is listed once.
const skipped = [...new Set([...light.skipped, ...dark.skipped])]
  .map((s) => (light.skipped.includes(s) && dark.skipped.includes(s) ? s : (light.skipped.includes(s) ? 'Light only: ' : 'Dark only: ') + s));
const captured = new Date().toISOString().slice(0, 10);
const sizeList = [...sizes.entries()].sort((a, b) => b[1] - a[1]);
const [mainSize] = sizeList[0];
const readme = `# MacHTML: every screen of OpenFrame Studio for the Mac

${meta.length} screens in ${groups.length} areas, each captured from the real app in Light mode (\`light/\`) and in Dark mode
(\`dark/\`). Open \`index.html\`. Page \`NNN-name.html\` is the same screen in both folders.

## What MacHTML is, and how it differs from WindowsHTML

WindowsHTML saved the Windows app's own DOM and CSS, because that app is a web view. The Mac app is native SwiftUI: there
is no DOM to save. MacHTML is built from two things captured at the same moment for every screen:

- the 2x picture of the window as the window server draws it (title bar, toolbar, sidebar, content, and any sheet or menu
  at its real position), and
- the full accessibility tree of the window: role, identifier, label, title, value, enabled, selected and focused state,
  and the frame of every element in points.

## How the pages are made

- The capture tour (\`apps/macos/Sources/OpenFrameE2EDriver/Scenarios/CaptureTour*.swift\`, scenarios \`zz-capture/*\`) drives
  the real app, seeds a project and visits every screen it can reach, once with Settings, Appearance, Mode set to Light and
  once with Dark. Only screens present in both runs become pages. A screen that looks the same as an earlier one is dropped.
- \`scripts/mac-html/build-mac-html.mjs\` writes one page per screen. The window picture is the background, cut into the
  main containers (toolbar, sidebar, content, inspector, status bar, each sheet and menu), found from the toolbar, the
  split group and its columns, the main scroll area and the sheet windows in the tree. Each part shows its own piece of
  the one picture, so together they give back the picture pixel for pixel. The window has a 10 point corner radius and a
  shadow drawn outside the picture.
- Every accessibility element is a real HTML element at its exact frame (one CSS pixel is one point): text as selectable
  text, buttons as buttons, fields with their value, check boxes and switches, radio buttons, pop-up buttons as select
  boxes with their choices, lists and outlines as lists, menus as menus, each with its label as \`aria-label\` and its
  identifier as \`data-id\`. In the normal view this layer is transparent over the picture.
- The HTML only switch at the top of each page hides the picture and shows the HTML layer alone on the window colour.
- A control that led to another captured screen is a link to that page. Each page also links to the same page in the
  other mode and to the index.
- No page holds a script, and nothing is loaded from the network. The Light and Dark switch on the index and the HTML only
  switch are plain CSS.
- Commit of the app these came from: \`${commit || 'unknown'}\`, captured on ${captured} from the development build.
- Theme: Screening Room, the default Mac theme, in Light and in Dark. The window is the full visible screen, frontmost
  and active for every picture.

## Window size

The window is ${mainSize} points (shown at one CSS pixel per point; the pictures are 2x).${sizeList.length > 1 ? ` Some screens were captured at another size: ${sizeList.slice(1).map(([s, n]) => `${s} (${n} ${n === 1 ? 'screen' : 'screens'})`).join(', ')}.` : ''}

## Regenerate

    apps/macos/E2E/capture.sh "apps/macos/build/OpenFrame Studio.app"

That runs the tour in Light and in Dark (with \`OPENFRAME_E2E_PARALLEL=n\` it runs n runners side by side), builds this
folder and checks every page (\`scripts/mac-html/check-pages.swift\`). To rebuild the pages from an earlier capture
without running the app:

    OPENFRAME_CAPTURE_SKIP_RUN=1 OPENFRAME_CAPTURE_DIR=<capture folder with light/ and dark/> apps/macos/E2E/capture.sh "apps/macos/build/OpenFrame Studio.app"

## Not captured

- The system Open and Save panels (they run in another process and block the driver).
- Real drag and drop.
- Voice recording (needs the microphone and its permission dialog).
- The print panel.
- Forced crashes and disk failures.
- Real AI answers and an Assistant memory with entries: the development build has no AI engine, so the Assistant screens
  show the state without one.
- In Dark mode a sheet's rounded top corners show small white wedges: the capture lays the sheet window's transparent
  corners on white. The app draws them correctly.
${onlyOne.length ? `- Screens captured in one mode only: ${onlyOne.join(', ')}.\n` : ''}${skipped.length ? '- Steps the tour skipped (in both modes unless marked):\n' + skipped.map((s) => '  - ' + s.slice(0, 220)).join('\n') + '\n' : ''}
## Count

${meta.length} screens, ${meta.length * 2} pages (Light and Dark).
`;
fs.writeFileSync(path.join(root, 'README.md'), readme);
console.log(`built ${meta.length} screens in Light and Dark in ${root} (${alias.size} repeated screens dropped, ${onlyOne.length} in one mode only)`);
