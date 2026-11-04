// Packs the generated screen pages (.cache/shots, not in Git) into the file the deploy workflow downloads from a GitHub release.
// Run after shots.mjs and sim-mac.mjs: node scripts/pack-screens.mjs <n>, where <n> is the new bundle number.
// It first replaces developer paths in the captured text, then refuses to pack anything but the expected file types, the old
// product name or a developer path. It writes scripts/screens-bundle.json; upload the file to a release with the same tag.
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { hasOldName } from '../src/lib/brand.mjs';
import { DEV_PATH } from './dev-paths.mjs';

const ROOT = path.resolve(import.meta.dirname, '..');
const SHOTS = path.join(ROOT, '.cache', 'shots');
const n = Number(process.argv[2]);
if (!Number.isInteger(n) || n < 1) throw new Error('usage: node scripts/pack-screens.mjs <bundle number>');
const name = `website-screens-${n}`, file = `${name}.tar.gz`;

// The capture ran from a developer's test folders; visitors see ordinary install and document folders instead.
const SCRUB = [
  [/ src="http:\/\/ofasset\.localhost\/[^"]*"/g, ''], // captured audio players pointed at the app's own file server
  [/C:\\Straft\\\.build-cache\\target\\debug\\ai/g, 'C:\\Program Files\\Straft Studio\\ai'],
  [/C:\\Straft\\tests\\e2e\\\.run\\[^\\]+\\zz-capture\\appdata/g, 'C:\\Users\\Maya\\AppData\\Local\\Straft Studio'],
  [/C:\\Straft\\tests\\e2e\\\.run\\[^\\]+\\zz-capture\\capture-[^\\]+/g, 'C:\\Users\\Maya\\Documents'],
];

const problems = []; let count = 0, scrubbed = 0;
const walk = (d) => fs.readdirSync(d, { withFileTypes: true }).flatMap((e) => (e.isDirectory() ? walk(path.join(d, e.name)) : [path.join(d, e.name)]));
for (const f of walk(path.join(SHOTS, 'pages'))) {
  count++;
  const rel = path.relative(SHOTS, f);
  if (!/\.(html|css|png)$/.test(f)) { problems.push(`unexpected file type: ${rel}`); continue; }
  if (f.endsWith('.png')) continue;
  const before = fs.readFileSync(f, 'utf8');
  const after = SCRUB.reduce((s, [re, to]) => s.replace(re, to), before);
  if (after !== before) { fs.writeFileSync(f, after); scrubbed++; }
  if (hasOldName(after)) problems.push(`old product name: ${rel}`);
  if (DEV_PATH.test(after)) problems.push(`developer path: ${rel}`);
}
if (hasOldName(fs.readFileSync(path.join(SHOTS, 'manifest.json'), 'utf8'))) problems.push('old product name: manifest.json');
if (problems.length) throw new Error(`not packed:\n  ${problems.slice(0, 30).join('\n  ')}`);

execFileSync('tar', ['-czf', `../${file}`, 'manifest.json', 'pages'], { cwd: SHOTS, stdio: 'inherit' });
const out = path.join(SHOTS, '..', file);
const sha256 = crypto.createHash('sha256').update(fs.readFileSync(out)).digest('hex');
const size = fs.statSync(out).size;
fs.writeFileSync(path.join(ROOT, 'scripts', 'screens-bundle.json'), JSON.stringify({ tag: name, file, sha256, size }, null, 2) + '\n');
console.log(`${out}: ${count} files, ${scrubbed} scrubbed, ${(size / 1048576).toFixed(0)} MiB, sha256 ${sha256}`);
