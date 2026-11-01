// Checks src/content/manual/<plat>/*.mjs against the screen catalogs (docs/CONTENT-BRIEF.md).
// Run: node scripts/check-content.mjs [win|mac] [area-slug]. Exit code 1 on any error.
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const ROOT = path.resolve(import.meta.dirname, '..');
const BANNED = /\b(delve|seamless(ly)?|leverag(e|es|ing)|robust|elevate|unlock the power|game-changer|cutting-edge|tapestry|supercharge|harness(es|ing)?|streamline|it'?s important to note|it is worth noting|in today's)\b/i;
const DASH = /[–—]/;

export async function loadManual(plat) {
  const dir = path.join(ROOT, 'src', 'content', 'manual', plat);
  if (!fs.existsSync(dir)) return [];
  const out = [];
  for (const f of fs.readdirSync(dir).filter((f) => f.endsWith('.mjs')).sort()) {
    out.push({ file: f, ...(await import(pathToFileURL(path.join(dir, f)).href)).default });
  }
  return out.sort((a, b) => a.order - b.order);
}

function strings(v, at, cb) {
  if (typeof v === 'string') cb(v, at);
  else if (Array.isArray(v)) v.forEach((x, i) => strings(x, `${at}[${i}]`, cb));
  else if (v && typeof v === 'object') for (const [k, x] of Object.entries(v)) strings(x, `${at}.${k}`, cb);
}

export async function checkContent(plats = ['win', 'mac'], onlySlug) {
  const errors = [], notes = [];
  const err = (m) => errors.push(m);
  for (const plat of plats) {
    const cat = JSON.parse(fs.readFileSync(path.join(ROOT, 'research', `screens-${plat}.json`), 'utf8'));
    const byName = new Map(cat.map((s) => [s.name, s]));
    const areas = await loadManual(plat);
    const ids = new Map();
    for (const a of areas) for (const w of a.workflows || []) {
      if (ids.has(w.id)) err(`${plat}: workflow id ${w.id} in ${a.file} and ${ids.get(w.id)}`);
      ids.set(w.id, a.file);
    }
    for (const a of areas) {
      if (onlySlug && a.slug !== onlySlug) continue;
      const at = `${plat}/${a.file}`;
      const own = cat.filter((s) => s.area === a.area).map((s) => s.name);
      if (!own.length) err(`${at}: area "${a.area}" has no screens in screens-${plat}.json`);
      for (const k of ['slug', 'order', 'part', 'intro', 'workflows', 'screens']) if (a[k] == null) err(`${at}: missing ${k}`);
      const described = Object.keys(a.screens || {});
      for (const n of described) if (!byName.has(n)) err(`${at}: screens.${n} is not a ${plat} screen`);
      for (const n of own) if (!described.includes(n)) err(`${at}: screen ${n} has no description`);
      const used = new Set();
      for (const w of a.workflows || []) {
        if (!/^[a-z0-9-]+$/.test(w.id || '')) err(`${at}: bad workflow id ${w.id}`);
        if (!w.title || !w.purpose || !w.result || !w.steps?.length) err(`${at}: workflow ${w.id} needs title, purpose, steps and result`);
        for (const s of w.steps || []) {
          if (!byName.has(s.screen)) err(`${at}: ${w.id} step screen ${s.screen} is not a ${plat} screen`);
          if (!s.do) err(`${at}: ${w.id} step on ${s.screen} has no "do" text`);
          used.add(s.screen);
        }
        for (const r of w.related || []) if (!ids.has(r)) err(`${at}: ${w.id} related ${r} is not a workflow id`);
      }
      for (const n of own.filter((n) => !used.has(n))) {
        if (!/workflow|tutorial|variant|same as|see /i.test(a.screens?.[n] || '')) err(`${at}: screen ${n} is in no workflow and its description does not name the workflow it belongs to`);
      }
      strings(a, at, (s, where) => {
        if (DASH.test(s)) err(`${where}: em or en dash`);
        const m = s.match(BANNED);
        if (m) err(`${where}: banned phrase "${m[0]}"`);
        if (/\/var\/folders|[A-Z]:\\Users|team testing|unsigned|team password/i.test(s)) err(`${where}: internal information`);
      });
    }
    const covered = new Set(areas.map((a) => a.area));
    const all = [...new Set(cat.map((s) => s.area))];
    notes.push(`${plat}: ${areas.length} of ${all.length} areas written; missing: ${all.filter((x) => !covered.has(x)).join(', ') || 'none'}`);
  }
  return { errors, notes };
}

if (process.argv[1] && path.resolve(process.argv[1]) === path.resolve(import.meta.filename)) {
  const [onlyPlat, onlySlug] = process.argv.slice(2);
  const { errors, notes } = await checkContent(onlyPlat ? [onlyPlat] : undefined, onlySlug);
  console.log(notes.join('\n'));
  if (errors.length) { console.error(errors.join('\n')); console.error(`${errors.length} problems`); process.exit(1); }
  console.log('content check passed');
}
