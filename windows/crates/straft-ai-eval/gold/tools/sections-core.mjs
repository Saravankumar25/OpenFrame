// Case shorthand, ids, the fixed model sample and the output of the generator.
import assert from 'node:assert/strict';
import { LEAVES, leafIds, NEW_DATA } from './sections-data.mjs';

const BASE = 'low-tide-two-screenplays';
const WITH_SECTIONS = 'low-tide-sections';

// type code: [answer_type, length_class] for the types the first harness knows;
// the new types and the long classes go to new_answer_type and new_length_class (see README).
const OLD = {
  lk: ['lookup', 'one_sentence'],
  ct: ['count', 'one_sentence'],
  ls: ['list', 'short'],
  lp: ['list', 'paragraph'],
  rl: ['relation', 'short'],
  ex: ['explain', 'paragraph'],
  cp: ['compare', 'short'],
  hp: ['help', 'short'],
  hx: ['help', 'paragraph'],
  nv: ['navigate', 'short'],
};
const NEW = {
  ds: { type: 'describe', cls: 'long250' },
  br: { type: 'brief', cls: 'long250' },
  se: { type: 'story_explanation', cls: 'long350' },
  rl2: { type: 'relation', cls: 'long250', oldType: true },
};
const WORDS = { long250: [40, 250], long350: [60, 350] };
const QUIET = new Set(['explain', 'relation', 'compare', 'brainstorm', 'describe', 'brief', 'story_explanation']);

const norm = (v) => (v === undefined ? [] : v).map((x) => (Array.isArray(x) ? x : [x]));

/** One case, described shortly. Fields:
 *  leaf (own case of that leaf) or fam (pair, triple, conv, turn, neg); picks (the sections array);
 *  q the request; before earlier requests; sb sections before; t type code; has f no ptr abst d s proj. */
export function C(o) {
  assert(o.q, 'a case needs a request');
  assert(o.t, `${o.q}: a case needs a type code`);
  assert(OLD[o.t] || NEW[o.t], `${o.q}: unknown type code ${o.t}`);
  return o;
}

/** A pick, with the instance choices of section 3.1. */
export const P = (leaf, more = {}) => {
  assert(leafIds.includes(leaf), `unknown leaf ${leaf}`);
  const pick = { leaf };
  if (leaf === 'screenplay/scenes') pick.screenplay = 'LOW TIDE RADIO';
  return { ...pick, ...more };
};

const slug = (leaf) => leaf.replace('/', '-');

function expectOf(o) {
  const e = {};
  const has = norm(o.has);
  if (has.length) e.answer_has = has;
  if (o.f?.length) e.required_facts = o.f;
  if (o.no?.length) e.answer_lacks = o.no;
  if (o.abst) {
    e.abstain = true;
    e.abstain_names = o.abst;
  }
  if (o.ptr) e.pointer = o.ptr;
  const nw = NEW[o.t];
  const old = OLD[o.t];
  const type = nw ? nw.type : old[0];
  if (nw) {
    e.new_answer_type = nw.type;
    e.new_length_class = nw.cls;
    if (nw.oldType) e.answer_type = nw.type;
    const [lo, hi] = WORDS[nw.cls];
    e.min_words = lo;
    e.max_words = hi;
  } else {
    e.answer_type = old[0];
    e.length_class = old[1];
  }
  if (QUIET.has(type)) e.no_navigate = true;
  if (o.before?.length) e.no_reask = true;
  e.no_internal_words = true;
  if (o.d?.length) e.derived = o.d;
  return e;
}

function projectOf(o) {
  const leaves = [...o.picks.map((p) => p.leaf), ...(o.sb ?? []).map((p) => p.leaf), ...(o.ptr ? [o.ptr] : [])];
  return leaves.some((l) => NEW_DATA.includes(l)) ? WITH_SECTIONS : BASE;
}

/** Evenly spaced picks from a list ordered by type, so every kind of question is in the sample. */
function spaced(items, want) {
  if (items.length <= want) return new Set(items);
  const order = [...items].sort((a, b) => a.kindKey.localeCompare(b.kindKey));
  return new Set(Array.from({ length: want }, (_, i) => order[Math.floor((i * order.length) / want)]));
}

const SAMPLE = { leaf: 6, pair: 28, neg: 52, turn: 30 };

export function build(specs) {
  const cases = specs.map((o) => {
    assert(o.picks?.length >= 1 && o.picks.length <= 3, `${o.q}: one to three picks`);
    assert.equal(new Set(o.picks.map((p) => p.leaf)).size, o.picks.length, `${o.q}: a leaf twice`);
    const fam = o.leaf ? 'leaf' : o.fam;
    assert(fam, `${o.q}: no family`);
    return { ...o, fam, key: o.leaf ? slug(o.leaf) : fam, kindKey: `${o.t}|${o.q}`, e: expectOf(o) };
  });

  // The fixed model sample.
  const groups = new Map();
  for (const c of cases) {
    const g = c.fam === 'leaf' ? c.leaf : c.fam;
    (groups.get(g) ?? groups.set(g, []).get(g)).push(c);
  }
  for (const [g, items] of groups) {
    const fam = items[0].fam;
    const take = fam === 'leaf' ? spaced(items, SAMPLE.leaf) : SAMPLE[fam] ? spaced(items, SAMPLE[fam]) : new Set(items);
    for (const c of items) c.sample = take.has(c) || Boolean(c.keep);
  }

  // Ids and lines.
  const n = new Map();
  const lines = [];
  const rows = [];
  for (const c of cases) {
    const mark = c.sample ? 's' : 'x';
    const k = `${mark}-${c.key}`;
    n.set(k, (n.get(k) ?? 0) + 1);
    const id = `sec-${k}-${String(n.get(k)).padStart(3, '0')}`;
    const line = {
      id,
      kind: 'conversation',
      lang: 'en',
      project: projectOf(c),
      sections: c.picks,
    };
    if (c.sb) line.sections_before = c.sb;
    if (c.before?.length) line.before = c.before;
    line.request = c.q;
    line.expect = c.e;
    lines.push(JSON.stringify(line));
    rows.push({ id, c });
  }

  const own = (leaf) => rows.filter((r) => r.c.leaf === leaf);
  const asPick = (leaf, fams) => rows.filter((r) => fams.includes(r.c.fam) && r.c.picks.some((p) => p.leaf === leaf));
  const table = [
    '| Leaf | Own cases | In the sample | Negatives that pick it | In pairs, triples, conversation, turns | Why the count is what it is |',
    '|---|---|---|---|---|---|',
    ...LEAVES.map(([leaf, label]) => {
      const mine = own(leaf);
      const negs = rows.filter((r) => ['neg'].includes(r.c.fam) && r.c.picks.length === 1 && r.c.picks[0].leaf === leaf);
      const mixed = asPick(leaf, ['pair', 'triple', 'conv', 'turn']);
      return `| \`${leaf}\` (${label}) | ${mine.length} | ${mine.filter((r) => r.c.sample).length} | ${negs.length} | ${mixed.length} | ${NOTES[leaf] ?? 'The fixture holds enough data.'} |`;
    }),
  ].join('\n');

  const fam = (f) => rows.filter((r) => r.c.fam === f).length;
  const famRows = ['leaf', 'pair', 'triple', 'conv', 'turn', 'neg'].map((f) => `| ${f === 'leaf' ? 'one leaf each' : f} | ${fam(f)} | ${rows.filter((r) => r.c.fam === f && r.c.sample).length} |`);
  const famTable = ['| Family | Cases | In the sample |', '|---|---|---|', ...famRows, `| all | ${rows.length} | ${rows.filter((r) => r.c.sample).length} |`].join('\n');
  const sample = rows.filter((r) => r.c.sample).length;
  const summary = [
    `cases ${rows.length}, sample ${sample}`,
    `own ${fam('leaf')}, pair ${fam('pair')}, triple ${fam('triple')}, conv ${fam('conv')}, turn ${fam('turn')}, neg ${fam('neg')}`,
    `on ${WITH_SECTIONS}: ${rows.filter((r) => r.c.picks.length && projectOf(r.c) === WITH_SECTIONS).length}`,
  ].join('\n');
  return { lines, table: `${table}\n\n${famTable}`, summary };
}

/** Why a leaf has fewer cases than about thirty (shown in the README table). */
export const NOTES = {
  'idea-vault/this-project': 'The project vault holds four items.',
  'idea-vault/global': 'Four items, added in low-tide-sections (the base fixture has no global vault).',
  'story/story-page': 'None on purpose: the Story page is not offered to the assistant (the operations are hidden), so there is nothing it may answer. Negatives only.',
  'story/beat-board': 'Three acts and nine cards.',
  'story/outline': 'The Outline shows the same acts and cards as the Beat Board, so the facts are the same and the questions are about order and structure.',
  'story/timeline': 'Five Story Days and three time notes, added in low-tide-sections.',
  'story/episodes': 'None: the project is a feature film and the leaf is offered only on episodic projects, so no case can pick it. Needs an episodic fixture if it must be measured.',
  'screenplay/scenes': 'About thirty.',
  'breakdown/breakdown': 'About thirty.',
  'production/overview': 'The overview shows six numbers, so few questions are about it alone.',
  'production/catalog': 'The catalog is the breakdown elements by item; only props and vehicles have named, countable items in FACTS.md.',
  'production/moodboards': 'One board with four notes and no images.',
  'production/storyboards': 'One storyboard with four empty panels.',
  'production/shot-lists': 'Eleven shots.',
  'production/day-out-of-days': 'Derived from the schedule and the breakdown cast; the questions use only work days, start and finish.',
  'production/daily-view': 'Derived from the schedule, the breakdown and the day notes.',
  'production/sides': 'Derived from the scenes of a day; the questions use the scenes and cast of the day and the scene text.',
  'production/reports': 'Six report types, each a projection of data held elsewhere; the questions name the report.',
  'files/files': 'Four files in two folders.',
  'notes-tasks/notes': 'Five project notes and one private note of the local user.',
  'notes-tasks/tasks': 'Eight tasks.',
  'help/product-guide': 'Facts come from knowledge.md, not from the fixture.',
  'help/settings': 'Facts come from the Settings parts of knowledge.md, not from the fixture.',
};
