// Fixture data for the generator, read from FACTS.md (never typed twice), plus small helpers.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const facts = fs.readFileSync(
  path.join(here, '..', 'fixtures', 'low-tide-two-screenplays', 'FACTS.md'),
  'utf8',
).split(/\r?\n/);

/** Rows of the table whose header line starts with `header`. */
function table(header, exact = false) {
  const start = facts.findIndex((l) => (exact ? l.trim() === header : l.startsWith(header)));
  assert(start >= 0, `no table ${header}`);
  const rows = [];
  for (let k = start + 2; k < facts.length && facts[k].startsWith('|'); k++) {
    rows.push(facts[k].slice(1, -1).split('|').map((c) => c.trim()));
  }
  return rows;
}
const list = (s, sep = '; ') => (s === 'none' || s === '' ? [] : s.split(sep));

// Leaf ids exactly as section 3.1 becomes in code, with the label the pointer sentence uses.
export const LEAVES = [
  ['idea-vault/this-project', 'Idea Vault > This Project'],
  ['idea-vault/global', 'Idea Vault > Global'],
  ['story/story-page', 'Story > Story Page'],
  ['story/beat-board', 'Story > Beat Board'],
  ['story/outline', 'Story > Outline'],
  ['story/characters', 'Story > Characters'],
  ['story/timeline', 'Story > Timeline'],
  ['story/episodes', 'Story > Episodes'],
  ['screenplay/scenes', 'Screenplay > Scenes'],
  ['breakdown/breakdown', 'Breakdown'],
  ['production/overview', 'Production > Overview'],
  ['production/catalog', 'Production > Catalog'],
  ['production/locations', 'Production > Locations'],
  ['production/cast-crew', 'Production > Cast & Crew'],
  ['production/moodboards', 'Production > Moodboards'],
  ['production/storyboards', 'Production > Storyboards'],
  ['production/shot-lists', 'Production > Shot Lists'],
  ['production/schedule', 'Production > Schedule'],
  ['production/day-out-of-days', 'Production > Day Out of Days'],
  ['production/daily-view', 'Production > Daily View'],
  ['production/sides', 'Production > Sides'],
  ['production/reports', 'Production > Reports'],
  ['production/budget', 'Production > Budget'],
  ['call-sheets/call-sheets', 'Call Sheets'],
  ['files/files', 'Files'],
  ['notes-tasks/notes', 'Notes & Tasks > Notes'],
  ['notes-tasks/tasks', 'Notes & Tasks > Tasks'],
  ['help/product-guide', 'App Help & Settings > Product Guide'],
  ['help/settings', 'App Help & Settings > Settings'],
];
export const leafIds = LEAVES.map((l) => l[0]);
/** Leaves whose data only the `low-tide-sections` fixture holds. */
export const NEW_DATA = ['story/timeline', 'idea-vault/global'];

export const scenes = table('| # | Heading | Summary | Characters | Location').map((r) => ({
  n: +r[0], heading: r[1], summary: r[2], chars: r[3].split(', '), loc: r[4], day: +r[5],
}));
export const teaser = table('| # | Heading | Summary | Characters |', true).map((r) => ({
  n: +r[0], heading: r[1], summary: r[2], chars: r[3].split(', '),
}));
export const chars = table('| Character | Role label').map((r) => ({
  name: r[0], role: r[1], cues: r[2], other: r[3], actor: r[4],
}));
export const rels = table('| From | Relationship').map((r) => ({ from: r[0], rel: r[1], to: r[2], note: r[3] }));
export const cast = table('| Actor | Character').map((r) => ({
  actor: r[0], character: r[1], contact: r[2], avail: r[3], note: r[4],
  phone: /Phone ([^;]+);/.exec(r[2])[1], email: /email (\S+)/.exec(r[2])[1],
}));
export const crew = table('| Name | Role').map((r) => ({
  name: r[0], role: r[1], dept: r[2], contact: r[3], avail: r[4],
  phone: /Phone ([^;]+);/.exec(r[3])[1], email: /email (\S+)/.exec(r[3])[1],
}));
export const locs = table('| Location | Status').map((r) => ({
  name: r[0], status: r[1], address: r[2], contact: r[3], notes: r[4], avail: r[5],
}));
export const bd = table('| # | Props | Wardrobe').map((r) => ({
  n: +r[0], props: list(r[1]), wardrobe: list(r[2]), vehicles: list(r[3]), hair: list(r[4]),
  sfx: list(r[5]), vfx: list(r[6]), sound: list(r[7]), animals: list(r[8]), extras: list(r[9]),
}));
export const days = table('| Day | Date | Feature scenes').map((r) => ({
  n: +r[0], date: r[1], scenes: r[2].split(', ').map(Number), locs: r[3].split('; '),
}));
export const sheets = table('| Shoot day | Date | Status').map((r) => ({
  day: +r[0].replace('Shoot Day ', ''), date: r[1], status: r[2],
}));
export const budget = table('| Category | Line | Planned').map((r) => ({
  cat: r[0], line: r[1], planned: +r[2], actual: r[3] === 'not recorded' ? null : +r[3],
}));
export const shots = table('| Scene | Description | Size').map((r) => ({
  scene: +r[0], desc: r[1], size: r[2], move: r[3], angle: r[4], lens: r[5],
}));

// ---- totals that FACTS.md states; the script stops when the document and the tables disagree
assert.equal(scenes.length, 45);
assert.equal(teaser.length, 3);
assert.equal(chars.length, 13);
assert.equal(rels.length, 12);
assert.equal(cast.length, 12);
assert.equal(crew.length, 14);
assert.equal(locs.length, 16);
assert.equal(bd.length, 45);
assert.equal(days.length, 14);
assert.equal(sheets.length, 6);
assert.equal(budget.length, 11);
assert.equal(shots.length, 11);
assert.equal(bd.reduce((n, s) => n + s.props.length, 0), 97);
assert.equal(new Set(bd.flatMap((s) => s.props)).size, 73);
assert.equal(bd.reduce((n, s) => n + s.vehicles.length, 0), 7);
assert.equal(bd.reduce((n, s) => n + s.animals.length, 0), 4);
assert.equal(bd.reduce((n, s) => n + s.hair.length, 0), 12);
assert.equal(bd.reduce((n, s) => n + s.sfx.length, 0), 3);
assert.equal(bd.reduce((n, s) => n + s.vfx.length, 0), 3);
assert.equal(bd.reduce((n, s) => n + s.extras.length, 0), 11);
assert.equal(bd.reduce((n, s) => n + s.sound.length, 0), 27);
assert.equal(bd.reduce((n, s) => n + s.wardrobe.length, 0), 102);
assert.equal(scenes.reduce((n, s) => n + s.chars.length, 0), 120);
assert.deepEqual(bd.filter((s) => s.wardrobe.length === 0).map((s) => s.n), [12, 13, 14, 25, 29, 39]);
for (const d of days) for (const n of d.scenes) assert.equal(scenes[n - 1].day, d.n);
assert.equal(budget.reduce((n, l) => n + l.planned, 0), 16460000);
assert.equal(budget.reduce((n, l) => n + (l.actual ?? 0), 0), 13887000);
const anjali = scenes.filter((s) => s.chars.includes('Anjali')).map((s) => s.n);
assert.equal(anjali.length, 28);
const count = (st) => locs.filter((l) => l.status === st).length;
assert.deepEqual([count('Confirmed'), count('Shortlisted'), count('Idea'), count('Rejected')], [10, 3, 2, 1]);

// ---- helpers
const WORDS = ['zero', 'one', 'two', 'three', 'four', 'five', 'six', 'seven', 'eight', 'nine', 'ten', 'eleven',
  'twelve', 'thirteen', 'fourteen', 'fifteen', 'sixteen', 'seventeen', 'eighteen', 'nineteen', 'twenty',
  'twenty-one', 'twenty-two', 'twenty-three', 'twenty-four', 'twenty-five', 'twenty-six', 'twenty-seven', 'twenty-eight'];
/** A count as the answer may write it. */
export const num = (n) => (WORDS[n] ? [String(n), WORDS[n]] : [String(n)]);
/** Each number must appear (one group per number). */
export const each = (ns) => ns.map((n) => [String(n)]);
const indian = (n) => {
  const s = String(n);
  if (s.length <= 3) return s;
  const tail = s.slice(-3);
  const head = s.slice(0, -3).replace(/\B(?=(\d{2})+(?!\d))/g, ',');
  return `${head},${tail}`;
};
/** An amount in rupees as the answer may write it. */
export const money = (n) => [String(n), indian(n), n.toLocaleString('en-US')];
const dayDate = (d) => days[d - 1].date;
/** A shoot day as the answer may write it. */
export const dayAlt = (d) => [`Day ${d}`, dayDate(d), `${d} March`, `March ${d}`];
export const scene = (n) => scenes[n - 1];
export const bdOf = (n) => bd[n - 1];
export const charScenes = (c) => scenes.filter((s) => s.chars.includes(c)).map((s) => s.n);
export const charDays = (c) => [...new Set(charScenes(c).map((n) => scene(n).day))].sort((a, b) => a - b);
export const castOf = (character) => cast.find((c) => c.character === character);
export const crewOf = (name) => crew.find((c) => c.name === name);
export const locOf = (name) => locs.find((l) => l.name === name);
export const charOf = (name) => chars.find((c) => c.name === name);
export const sceneHas = (field, item) => bd.filter((s) => s[field].some((x) => x === item || x.startsWith(item))).map((s) => s.n);
export const helpers = { num, each, money, dayAlt, scene, bdOf, charScenes, charDays, castOf, crewOf, locOf, charOf, sceneHas };
