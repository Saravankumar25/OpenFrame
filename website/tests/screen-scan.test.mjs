// The checks every screen page passes before it is packed for the website.
import test from 'node:test';
import assert from 'node:assert/strict';
import { pageProblems, pictureProblems, hasPrivate, wordHashOf } from '../scripts/screen-scan.mjs';

const hits = (t) => pageProblems(t).join('|');
test('a clean page has no problems', () => assert.equal(hits('<link href="a.css"><div class="x">Night Bus, Maya Rao, https://example.com in text</div>'), ''));
for (const [name, text, want] of [
  ['script', '<script>1</script>', /script/], ['frame', '<iframe src="a.html">', /embedded/], ['handler', '<div onclick="a()">', /handler/],
  ['javascript link', '<a href="javascript:x()">', /javascript/], ['OneDrive folder', 'C:\Users\Maya\OneDrive\a.txt', /private/],
  ['token', 'ghp_' + 'a'.repeat(36), /secret/], ['network image', '<img src="https://a.example/b.png">', /network/], ['network css', 'a{background:url(//a.example/b.png)}', /network/],
  ['app server', '<audio src="http://x.localhost/a">', /network/], ['file link', '<a href="file:///C:/a">', /file link/],
]) test(`finds: ${name}`, () => assert.match(hits(text), want));
test('a private word is found by its hash, in any case, and only as a whole word', () => {
  const words = new Set([wordHashOf('zzprivate')]);
  assert.equal(hasPrivate('Project ZZPrivate here', words), true);
  assert.equal(hasPrivate('zzprivates and Night Bus', words), false);
});
test('a link to a file that is not there is named', () => assert.match(pageProblems('<link href="nope.css">', import.meta.filename).join(), /missing file nope\.css/));
test('a PNG text chunk with a path is found; a plain one is not', () => {
  const chunk = (type, data) => { const h = Buffer.alloc(8); h.writeUInt32BE(data.length, 0); h.write(type, 4, 'latin1'); return Buffer.concat([h, Buffer.from(data, 'latin1'), Buffer.alloc(4)]); };
  const sig = Buffer.from('\x89PNG\r\n\x1a\n', 'latin1');
  assert.deepEqual(pictureProblems(Buffer.concat([sig, chunk('IDAT', 'x'), chunk('IEND', '')])), []);
  assert.match(pictureProblems(Buffer.concat([sig, chunk('tEXt', 'Comment\0/Users/someone'), chunk('IEND', '')])).join(), /private text/);
  assert.deepEqual(pictureProblems(Buffer.from('nope')), ['not a PNG']);
});
