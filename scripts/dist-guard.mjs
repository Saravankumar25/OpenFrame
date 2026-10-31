// Checks a build folder before it goes to Cloudflare Pages: nothing private, no secrets, inside the host's limits.
// Used by scripts/deploy.mjs (blocks the deploy) and tests/seo.test.mjs. Limits: https://developers.cloudflare.com/pages/platform/limits/
import fs from 'node:fs';
import path from 'node:path';

export const LIMITS = { files: 20000, fileBytes: 25 * 1024 * 1024, warnFiles: 15000 };
const PRIVATE_TOP = new Set(['research', 'manuals', '.cache', '.git', '.github', '.wrangler', 'node_modules', 'src', 'scripts', 'tests', 'docs', 'functions', '.build-removed', '.tmp']);
const PRIVATE_FILE = /(^|\/)(\.env[^/]*|_worker\.js|wrangler\.toml|package(-lock)?\.json|[^/]*\.(map|pem|key|p12|pfx|log|bak|ps1|pdf|docx))$/i;
const TEXT = /\.(html|css|js|mjs|json|txt|xml|svg|webmanifest|md)$|_headers$|_redirects$/i;
export const SECRET_PATTERNS = [
  /-----BEGIN [A-Z ]*PRIVATE KEY-----/, /\bAKIA[0-9A-Z]{16}\b/, /\bgh[pousr]_[A-Za-z0-9]{36,}\b/, /\bsk-[A-Za-z0-9]{32,}\b/, /\bxox[baprs]-[A-Za-z0-9-]{10,}/,
  /CLOUDFLARE_API_TOKEN|CLOUDFLARE_ACCOUNT_ID/, /\bBearer\s+[A-Za-z0-9._~+/-]{24,}/, /\beyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\./,
];

export function inspectDist(dir, { secrets = [] } = {}) {
  const problems = []; let files = 0;
  const walk = (d) => {
    for (const e of fs.readdirSync(d, { withFileTypes: true })) {
      const p = path.join(d, e.name), rel = path.relative(dir, p).split(path.sep).join('/');
      if (d === dir && PRIVATE_TOP.has(e.name)) problems.push(`private folder in the build: ${rel}`);
      if (e.isDirectory()) { walk(p); continue; }
      files++;
      // .well-known/security.txt is intended; everything else that looks private is not
      if (PRIVATE_FILE.test(rel)) problems.push(`private file type in the build: ${rel}`);
      const size = fs.statSync(p).size;
      if (size > LIMITS.fileBytes) problems.push(`over the 25 MiB file limit: ${rel} (${size} bytes)`);
      if (TEXT.test(rel) && size < 5e6) {
        const t = fs.readFileSync(p, 'utf8');
        for (const re of SECRET_PATTERNS) if (re.test(t)) problems.push(`secret-like text (${re}) in ${rel}`);
        for (const s of secrets) if (s && s.length >= 8 && t.includes(s)) problems.push(`a saved secret appears in ${rel}`);
      }
    }
  };
  walk(dir);
  if (files > LIMITS.files) problems.push(`over the ${LIMITS.files} file limit: ${files} files`);
  return { files, problems, warn: files > LIMITS.warnFiles ? `${files} files is close to the ${LIMITS.files} file limit` : '' };
}
