// Writes gold/sections.lowtide.en.jsonl and the count table in gold/README.md.
//
//   node crates/straft-ai-eval/gold/tools/make-sections.mjs
//
// Every expected value comes from the fixture documents (FACTS.md of low-tide-two-screenplays and
// low-tide-sections, knowledge.md for the help leaves) or is computed from the tables in them, and
// the script asserts the totals that FACTS.md states. Plain words, no em dashes.
// Spec: docs/specs/ai-section-scoped-assistant.md sections 3.1, 3.6, 3.7, 6 and 12.

import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';

import { LEAVES, scenes, teaser, chars, rels, cast, crew, locs, bd, days, sheets, budget, shots, helpers } from './sections-data.mjs';
import { positives } from './sections-leaves.mjs';
import { pairs, triples, conversational, turns, negatives } from './sections-families.mjs';
import { C, build } from './sections-core.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const gold = path.join(here, '..');

const all = [
  ...positives(),
  ...pairs(),
  ...triples(),
  ...conversational(),
  ...turns(),
  ...negatives(),
];
const out = build(all);
fs.writeFileSync(path.join(gold, 'sections.lowtide.en.jsonl'), out.lines.join('\n') + '\n');

// The count table of the README, between the two markers.
const readme = path.join(gold, 'README.md');
let text = fs.readFileSync(readme, 'utf8');
const a = '<!-- sections-table:start -->';
const b = '<!-- sections-table:end -->';
const i = text.indexOf(a);
const j = text.indexOf(b);
assert(i > 0 && j > i, 'README markers are missing');
text = text.slice(0, i + a.length) + '\n' + out.table + '\n' + text.slice(j);
fs.writeFileSync(readme, text);
console.log(out.summary);
