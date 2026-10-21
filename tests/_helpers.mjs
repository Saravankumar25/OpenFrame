import { readdirSync, readFileSync, existsSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
export const DIST = path.join(ROOT, 'dist');
export const HAS_DIST = existsSync(path.join(DIST, 'index.html'));
export const SKIP = HAS_DIST ? false : 'dist/ is missing: run "npm run build" first';

export function walk(dir, out = []) {
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    e.isDirectory() ? walk(p, out) : out.push(p);
  }
  return out;
}

/** dist file -> site path ('/features/x/'), or null for 404.html and non-page html. */
export function pagePath(file) {
  const rel = path.relative(DIST, file).split(path.sep).join('/');
  if (rel === 'index.html') return '/';
  if (rel.endsWith('/index.html')) return '/' + rel.slice(0, -'index.html'.length);
  return '/' + rel;
}

export function htmlFiles() {
  return HAS_DIST ? walk(DIST).filter((f) => f.endsWith('.html')) : [];
}

export const read = (f) => readFileSync(f, 'utf8');

/** Does a URL path (no query/hash) resolve to a file in dist? */
export function resolvesInDist(p) {
  let d;
  try { d = decodeURIComponent(p); } catch { return false; }
  const abs = path.resolve(DIST, '.' + d);
  if (!abs.startsWith(DIST)) return false;
  if (!existsSync(abs)) return false;
  const s = statSync(abs);
  return s.isFile() || (s.isDirectory() && existsSync(path.join(abs, 'index.html')));
}
