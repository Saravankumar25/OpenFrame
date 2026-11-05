// What a captured screen page may not hold before it goes on the website. Used by pack-screens.mjs on every page, stylesheet and
// picture; tests/screen-scan.test.mjs proves each rule catches what it names.
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';

// A OneDrive folder is private; the word alone is product text (Library setup explains that the app avoids it).
export const PRIVATE_PATH = /onedrive[\\/]/i;
// Words that must never appear: the owner's private project and account names. Only their hashes are kept, so this file does not publish them.
// The sample names of the captures (Maya, Night Bus, LOST PROPERTY) are allowed.
const PRIVATE_WORDS = new Set(['2b486eda29b84af5', '63dabef0707436b4', 'f68a41f5061077bc', 'fcfb490e15d2ecce', '53b6e07f5bbc8162']);
const seen = new Map();
const wordHash = (w) => { let h = seen.get(w); if (!h) { h = crypto.createHash('sha256').update(w).digest('hex').slice(0, 16); seen.set(w, h); } return h; };
/** words: the hashes to look for; the default is the private list. */
export const hasPrivate = (text, words = PRIVATE_WORDS) => PRIVATE_PATH.test(text) || (text.match(/[A-Za-z]{5,}/g) || []).some((w) => words.has(wordHash(w.toLowerCase())));
export const wordHashOf = wordHash;
export const SECRET = /AKIA[0-9A-Z]{16}|gh[pousr]_[A-Za-z0-9]{30,}|github_pat_\w{20,}|-----BEGIN [A-Z ]*PRIVATE KEY|\bsk-[A-Za-z0-9_-]{20,}|xox[abprs]-[A-Za-z0-9-]{10,}|CLOUDFLARE_API_TOKEN/;
const REF = /\b(?:src|href|poster|data|action)\s*=\s*"([^"]*)"|\bsrcset\s*=\s*"([^"]*)"|url\(\s*['"]?([^'")]+)['"]?\s*\)|@import\s+"([^"]+)"/g;

/** Problems in one page or stylesheet. file is its path on disk, for resolving what it links to. */
export function pageProblems(text, file) {
  const out = [];
  if (/<script\b/i.test(text)) out.push('script');
  if (/<(?:iframe|object|embed|base|meta\s+http-equiv)\b/i.test(text)) out.push('embedded or redirecting element');
  if (/<[a-z][^>]*\son[a-z]+\s*=/i.test(text)) out.push('inline event handler');
  if (/javascript:/i.test(text)) out.push('javascript link');
  if (hasPrivate(text)) out.push('private name');
  if (SECRET.test(text)) out.push('secret');
  for (const m of text.matchAll(REF)) {
    const ref = m[1] ?? m[2] ?? m[3] ?? m[4];
    if (!ref || ref.startsWith('#') || ref.startsWith('data:')) continue;
    if (/^file:/i.test(ref)) out.push(`file link ${ref}`);
    else if (/^(?:[a-z][a-z0-9+.-]*:|\/\/)/i.test(ref)) out.push(`network reference ${ref}`);
    else if (file && !fs.existsSync(path.resolve(path.dirname(file), decodeURI(ref.split(/[?#]/)[0])))) out.push(`missing file ${ref}`);
  }
  return out;
}

/** Problems in a PNG: a text chunk can carry a path or a name the eye never sees. Only the chunk names and text are read. */
export function pictureProblems(buf) {
  if (buf.subarray(0, 8).toString('latin1') !== '\x89PNG\r\n\x1a\n') return ['not a PNG'];
  const out = [];
  for (let i = 8; i + 8 <= buf.length;) {
    const n = buf.readUInt32BE(i), type = buf.toString('latin1', i + 4, i + 8);
    if (/^(tEXt|iTXt|zTXt|eXIf)$/.test(type)) {
      const t = buf.toString('latin1', i + 8, i + 8 + n);
      if (hasPrivate(t) || SECRET.test(t) || /\/Users\/|[A-Z]:[\\]/.test(t)) out.push(`private text in ${type}`);
    }
    i += 12 + n;
  }
  return out;
}
