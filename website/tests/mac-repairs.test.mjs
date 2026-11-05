// The recorded Mac picture repairs: an unlisted screen is left alone, a listed one comes back as a picture of the same size.
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import sharp from 'sharp';
import { macRepair } from '../scripts/mac-repairs.mjs';

test('macRepair', async (t) => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'mac-repairs-'));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  const png = path.join(dir, 'fixture.png');
  await sharp({ create: { width: 1000, height: 800, channels: 4, background: { r: 245, g: 246, b: 250, alpha: 1 } } }).png().toFile(png);

  assert.equal(await macRepair('not-a-listed-screen', 'light', png), null);
  assert.equal(await macRepair('pages-files-list', 'dark', png), null, 'a repair of one theme leaves the other alone');

  const out = await macRepair('pages-files-list', 'light', png);
  const meta = await sharp(out).metadata();
  assert.deepEqual([meta.format, meta.width, meta.height], ['png', 1000, 800]);
});
