import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { DIST, HAS_DIST, ROOT, SKIP, resolvesInDist } from './_helpers.mjs';

const manifestFile = path.join(ROOT, 'screens', 'manifest.json');
const manifest = existsSync(manifestFile) ? JSON.parse(readFileSync(manifestFile, 'utf8')) : {};
const { TUTORIAL_PATHS } = await import(pathToFileURL(path.join(ROOT, 'src/content/nav.mjs')).href);
const pathKeys = new Set(TUTORIAL_PATHS.map((p) => (Array.isArray(p) ? p[0] : p.key ?? p.id)));
const SLUG = { win: 'windows', mac: 'mac' };

const all = { win: [], mac: [] };
for (const plat of ['win', 'mac']) {
  const dir = path.join(ROOT, 'src/content/tutorials', plat);
  if (!existsSync(dir)) continue;
  for (const f of readdirSync(dir).filter((x) => x.endsWith('.mjs'))) {
    all[plat].push({ file: f, ...(await import(pathToFileURL(path.join(dir, f)).href)).default });
  }
}
const idsOf = (plat) => new Set(all[plat].filter((t) => t.available !== false).map((t) => t.id));

test('tutorial modules exist', () => {
  assert.ok(all.win.length + all.mac.length > 0, 'no tutorial modules found yet');
});

for (const plat of ['win', 'mac']) {
  for (const t of all[plat].filter((x) => x.available !== false)) {
    test(`tutorial ${plat}/${t.id}`, () => {
      const errs = [];
      const e = (m) => errs.push(m);
      if (t.file !== `${t.id}.mjs`) e(`file name ${t.file} != id`);
      for (const k of ['id', 'path', 'feature', 'title', 'purpose', 'difficulty', 'result', 'next']) if (!(k in t)) e(`missing ${k}`);
      if (!t.title || t.title.length > 60) e('title missing or over 60 chars');
      if (!pathKeys.has(t.path)) e(`unknown path ${t.path}`);
      if (typeof t.order !== 'number') e('order not a number');
      if (!['Beginner', 'Intermediate', 'Advanced'].includes(t.difficulty)) e(`bad difficulty ${t.difficulty}`);
      if (!(t.minutes > 0)) e('minutes not positive');
      for (const k of ['prerequisites', 'objectives', 'mistakes', 'troubleshooting', 'related']) if (!Array.isArray(t[k])) e(`${k} not an array`);
      if (!Array.isArray(t.steps) || t.steps.length < 3 || t.steps.length > 9) e(`steps ${t.steps?.length}, need 3 to 9`);
      const screenDir = path.join(ROOT, 'screens', plat);
      const existing = existsSync(screenDir) ? readdirSync(screenDir) : [];
      for (const [i, s] of (t.steps || []).entries()) {
        const w = `step ${i + 1}`;
        for (const k of ['screen', 'action', 'label', 'body']) if (!s[k]) e(`${w}: missing ${k}`);
        if (s.screen?.endsWith('.html')) e(`${w}: screen ends with .html`);
        const m = manifest[`${plat}/${s.screen}`];
        if (!m) { e(`${w}: ${plat}/${s.screen} not in manifest`); continue; }
        const base = path.basename(m.src);
        if (!existing.some((f) => f.startsWith(base + '-'))) e(`${w}: no image files for ${s.screen} in screens/${plat}/`);
        if (s.target != null) {
          const r = m.targets?.[s.target];
          if (!r || (r.rect ?? r) == null) e(`${w}: target ${s.target} has no rect in manifest`);
        }
      }
      for (const r of t.related || []) if (!idsOf(plat).has(r)) e(`related ${r} not available on ${plat}`);
      if (t.next && !idsOf(plat).has(t.next)) e(`next ${t.next} not available on ${plat}`);
      if (HAS_DIST && !resolvesInDist(`/${SLUG[plat]}/tutorials/${t.id}/`)) e(`no dist page /${SLUG[plat]}/tutorials/${t.id}/`);
      assert.deepEqual(errs, []);
    });
  }
}

test('dist/tutorial-screen-map.json lists every tutorial', { skip: SKIP }, () => {
  const f = path.join(DIST, 'tutorial-screen-map.json');
  assert.ok(existsSync(f), 'dist/tutorial-screen-map.json missing');
  const text = readFileSync(f, 'utf8');
  const errs = [];
  for (const plat of ['win', 'mac']) {
    for (const t of all[plat].filter((x) => x.available !== false)) {
      if (!text.includes(`"${t.id}"`)) errs.push(`${plat}/${t.id} not in map`);
    }
  }
  assert.deepEqual(errs, []);
});
