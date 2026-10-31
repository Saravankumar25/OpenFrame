// The Themes page: every starter file is published and passes the app's file limits, every download link resolves,
// the example in the page is a valid theme, and the page has the Windows and the Mac steps.
import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { DIST, ROOT, SKIP, read, resolvesInDist } from './_helpers.mjs';

const STARTERS_DIR = path.join(ROOT, 'src', 'static', 'themes');
const IDS = readdirSync(STARTERS_DIR).filter((f) => f.endsWith('.json')).map((f) => f.slice(0, -5));
const BUILT_IN = ['OpenFrame Default', 'Black and White', 'Studio Amber', 'Slate Noir', 'Midnight Reel', 'Teal & Orange',
  'Courier & Cream', 'Cutting Room', 'Velvet Curtain', 'Sunset Boulevard', 'Arctic Frame', 'Neon Reel',
  'Location Scout', 'Desert Dune', 'Colorist Neutral', 'Silver Screen', 'High Contrast'];
const SEEDS = { neutralHue: [0, 360], neutralTint: [0, 0.04], accentHue: [0, 360], accentChroma: [0, 0.25], accentLightnessNudge: [-12, 16] };
const TOP = ['formatVersion', 'id', 'name', 'description', 'author', 'seeds', 'overrides'];

// The same limits the app's theme checker applies (name, id, text lengths, seed ranges, unknown names, size).
function problems(text) {
  const out = [];
  if (Buffer.byteLength(text) > 10 * 1024) out.push('over 10 KB');
  const t = JSON.parse(text);
  for (const k of Object.keys(t)) if (!TOP.includes(k)) out.push(`unknown ${k}`);
  if (t.formatVersion !== 1) out.push('formatVersion');
  if (!/^[a-z0-9-]{3,40}$/.test(t.id ?? '')) out.push('id');
  const key = (s) => s.split(/\s+/).filter(Boolean).join(' ').toLowerCase();
  if (typeof t.name !== 'string' || !t.name.trim() || [...t.name].length > 60 || /[<>]/.test(t.name) || !/[\p{L}\p{N}]/u.test(t.name)) out.push('name');
  if (BUILT_IN.some((b) => key(b) === key(t.name ?? ''))) out.push('name is a built-in name');
  if (t.description !== undefined && (typeof t.description !== 'string' || [...t.description].length > 200)) out.push('description');
  if (t.author !== undefined && (typeof t.author !== 'string' || [...t.author].length > 80)) out.push('author');
  for (const k of Object.keys(t.seeds ?? {})) if (!(k in SEEDS)) out.push(`unknown seed ${k}`);
  for (const [k, [lo, hi]] of Object.entries(SEEDS)) {
    const v = t.seeds?.[k];
    if (typeof v !== 'number' || v < lo || v > hi) out.push(`seed ${k}`);
  }
  return out;
}

test('there are starter files', () => assert.ok(IDS.length >= 6, IDS.join(',')));

test('every starter file parses, passes the app limits and its id matches its file name', () => {
  const names = new Set();
  for (const id of IDS) {
    const text = readFileSync(path.join(STARTERS_DIR, `${id}.json`), 'utf8');
    assert.deepEqual(problems(text), [], id);
    const t = JSON.parse(text);
    assert.equal(t.id, id);
    assert.ok(!names.has(t.name), `duplicate name ${t.name}`);
    names.add(t.name);
  }
});

test('every starter file is published at /themes/<id>.json and identical to the source', { skip: SKIP }, () => {
  for (const id of IDS) {
    const f = path.join(DIST, 'themes', `${id}.json`);
    assert.ok(existsSync(f), `${id}.json is not published`);
    assert.equal(read(f), readFileSync(path.join(STARTERS_DIR, `${id}.json`), 'utf8'));
  }
});

test('the page offers every starter with a swatch and a working download link', { skip: SKIP }, () => {
  const page = read(path.join(DIST, 'themes', 'index.html'));
  for (const id of IDS) {
    const m = page.match(new RegExp(`<a class="btn btn-ghost" href="(/themes/${id}\\.json)" download="${id}\\.json"`));
    assert.ok(m, `no download link for ${id}`);
    assert.ok(resolvesInDist(m[1]), `${m[1]} does not resolve`);
  }
  const links = [...page.matchAll(/href="(\/themes\/[^"]+\.json)"/g)].map((m) => m[1]);
  assert.equal(links.length, IDS.length);
  for (const l of links) assert.ok(resolvesInDist(l), l);
  assert.equal((page.match(/<svg viewBox="0 0 240 84"/g) || []).length, IDS.length);
  assert.doesNotMatch(page, /\sstyle=/, 'inline style attributes are not allowed');
});

test('the example file on the page is a valid theme', { skip: SKIP }, () => {
  const page = read(path.join(DIST, 'themes', 'index.html'));
  const blocks = [...page.matchAll(/<div class="codebox"><pre><code>([\s\S]*?)<\/code><\/pre><\/div>/g)].map((m) => m[1]
    .replace(/&quot;/g, '"').replace(/&#39;/g, "'").replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&amp;/g, '&'));
  assert.ok(blocks.length >= 2);
  assert.deepEqual(problems(blocks[0]), []);
});

test('the page has the Windows and the Mac steps, the field table and the error list', { skip: SKIP }, () => {
  const page = read(path.join(DIST, 'themes', 'index.html'));
  const win = page.match(/<div data-os-show="win">(<ol class="tsteps">[\s\S]*?)<\/div><div data-os-show="mac">/);
  const mac = page.match(/<div data-os-show="mac">(<ol class="tsteps">[\s\S]*?)<\/div>/);
  assert.ok(win && mac, 'platform blocks');
  for (const [name, block] of [['Windows', win[1]], ['Mac', mac[1]]]) {
    for (const label of ['Import Theme File…', 'Paste Theme…', 'Check Theme', 'Save and Apply', 'Save to My Themes', 'My Themes', 'Appearance'])
      assert.ok(block.includes(label), `${name} steps miss "${label}"`);
  }
  assert.match(win[1], /gear/);
  assert.match(mac[1], /Command and comma/);
  assert.ok(page.includes('Export Selected Theme…'), 'Mac export');
  for (const field of ['formatVersion', 'neutralHue', 'neutralTint', 'accentHue', 'accentChroma', 'accentLightnessNudge', 'overrides'])
    assert.ok(page.includes(`<code>${field}</code>`), field);
  assert.ok(page.includes('is too hard to read'), 'readability message');
  assert.ok(page.includes('is the name of a built-in theme'), 'name message');
  assert.doesNotMatch(page.replace(/<[^>]+>/g, ' '), /[–—]/);
});
