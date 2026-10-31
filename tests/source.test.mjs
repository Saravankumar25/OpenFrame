// Source hygiene: no stray control characters (a scripted edit once wrote backspace bytes into regexes).
import test from 'node:test';
import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { ROOT } from './_helpers.mjs';

const walk = (d) => readdirSync(d, { withFileTypes: true }).flatMap((e) => (e.isDirectory() ? walk(path.join(d, e.name)) : [path.join(d, e.name)]));
test('source files hold no control characters', () => {
  const bad = [];
  for (const dir of ['src', 'scripts', 'tests']) for (const f of walk(path.join(ROOT, dir)).filter((f) => /\.(m?js|css)$/.test(f))) {
    if (/[\x00-\x08\x0b\x0c\x0e-\x1f]/.test(readFileSync(f, 'utf8'))) bad.push(path.relative(ROOT, f));
  }
  assert.deepEqual(bad, []);
});
