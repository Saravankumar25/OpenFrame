// Manual and catalog content against the screen catalogs: names, coverage of every screen, writing rules.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, existsSync, readdirSync } from 'node:fs';
import path from 'node:path';
import { ROOT, DIST, SKIP } from './_helpers.mjs';
import { checkContent, loadManual } from '../scripts/check-content.mjs';
import { CATALOG } from '../src/content/catalog.mjs';
import { FEATURES } from '../src/content/nav.mjs';

test('manual content passes the content check', async () => {
  const { errors } = await checkContent();
  assert.deepEqual(errors.slice(0, 20), []);
});

test('every screen of both platforms has a manual chapter and a description', async () => {
  for (const plat of ['win', 'mac']) {
    const cat = JSON.parse(readFileSync(path.join(ROOT, 'research', `screens-${plat}.json`), 'utf8'));
    const areas = await loadManual(plat);
    const described = new Set(areas.flatMap((a) => Object.keys(a.screens)));
    assert.deepEqual(cat.filter((s) => !described.has(s.name)).map((s) => s.name), [], plat);
  }
});

test('feature catalog: every group, unique ids, real screens, availability by platform', () => {
  const ids = new Set();
  const names = { win: new Set(), mac: new Set() };
  for (const plat of ['win', 'mac']) for (const s of JSON.parse(readFileSync(path.join(ROOT, 'research', `screens-${plat}.json`), 'utf8'))) names[plat].add(s.name);
  for (const f of FEATURES) assert.ok(CATALOG[f.slug]?.length, `catalog group ${f.slug}`);
  for (const items of Object.values(CATALOG)) for (const it of items) {
    assert.ok(!ids.has(it.id), `duplicate ${it.id}`); ids.add(it.id);
    for (const k of ['name', 'summary', 'does', 'purpose']) assert.ok(it[k], `${it.id}.${k}`);
    for (const plat of ['win', 'mac']) for (const n of it[plat] || []) assert.ok(names[plat].has(n), `${it.id}: ${plat}/${n}`);
    assert.ok((it.win?.length || 0) + (it.mac?.length || 0) > 0, `${it.id} has no screen`);
  }
});

test('the user manuals are internal: no manual page, link or PDF on the website', { skip: SKIP }, () => {
  assert.ok(!existsSync(path.join(DIST, 'manuals')));
  const walk = (d) => readdirSync(d, { withFileTypes: true }).flatMap((e) => (e.isDirectory() ? (e.name === 'mockups' ? [] : walk(path.join(d, e.name))) : [path.join(d, e.name)]));
  for (const f of walk(DIST)) {
    assert.ok(!f.endsWith('.pdf'), f);
    if (f.endsWith('.html')) assert.doesNotMatch(readFileSync(f, 'utf8'), /href="\/manuals\/|user manual/i, f);
  }
});
