// Brand guard: the product is Straft Studio. No text file of the website may carry the old product name, except the
// external identifiers that keep it on purpose (domain, email, Cloudflare Pages project, API token, GitHub repository)
// and the SIL OFL phrase in the font licences. The old name is built from parts so this file does not match itself.
import test from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync, statSync } from 'node:fs';
import path from 'node:path';
import { ROOT, walk } from './_helpers.mjs';

const O = 'open', F = 'frame';
const OLD = new RegExp(`${O}[ _-]?${F}`, 'gi');
const SKIP_DIR = /^(node_modules|dist|\.cache|\.git|\.tmp|\.wrangler|test-results|manuals|\.build-removed)\/|^research\/[^/]*-src\//;

// [files the exception applies to, exact text that may stay]. Longer texts first: they are removed before shorter ones.
const ALLOWED = [
  [/./, `${O}${F}studio.app`], // the domain, and the support address on it
  [/./, `${O}${F}-studio.pages.dev`], // the Pages host
  [/^(site\.config\.mjs|README\.md|docs\/.*|scripts\/deploy\.mjs)$/, `${O}${F}-studio`], // the Cloudflare Pages project id
  [/^docs\/OPERATIONS\.md$/, `${O}${F}-website`], // the name of the Cloudflare API token
  [/^docs\/SECURITY\.md$/, 'Saravankumar25/' + 'Open' + 'Frame'], // the GitHub repository
  [/(^|\/)OFL-[^/]+\.txt$/, `${O} ${F}work`], // SIL Open Font License wording
];

function files() {
  let list;
  try {
    list = execFileSync('git', ['ls-files', '-z', '--cached', '--others', '--exclude-standard'], { cwd: ROOT, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).split('\0').filter(Boolean);
  } catch {
    list = walk(ROOT).map((f) => path.relative(ROOT, f).split(path.sep).join('/'));
  }
  return list.filter((f) => !SKIP_DIR.test(f) && existsSync(path.join(ROOT, f)) && statSync(path.join(ROOT, f)).isFile());
}

test('no text file of the website carries the old product name', () => {
  const bad = [];
  for (const f of files()) {
    const buf = readFileSync(path.join(ROOT, f));
    if (buf.includes(0)) continue; // binary
    let s = buf.toString('utf8');
    for (const [where, text] of ALLOWED) if (where.test(f)) s = s.split(text).join('');
    for (const m of s.matchAll(OLD)) bad.push(`${f}: ${s.slice(Math.max(0, m.index - 30), m.index + 40).replace(/\s+/g, ' ')}`);
  }
  assert.deepEqual(bad.slice(0, 30), []);
});
