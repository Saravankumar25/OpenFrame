// Deploy dist/ to Cloudflare Pages (project openframe-studio: the Cloudflare project still has that name). Credentials come from the environment
// (CLOUDFLARE_API_TOKEN, CLOUDFLARE_ACCOUNT_ID); on Windows they are read from the user's saved variables.
// The values are passed only to the wrangler child process and are never printed.
import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { PAGES_PROJECT } from '../site.config.mjs';
import { inspectDist } from './dist-guard.mjs';
const env = { ...process.env };
for (const k of ['CLOUDFLARE_API_TOKEN', 'CLOUDFLARE_ACCOUNT_ID']) {
  if (!env[k] && process.platform === 'win32') {
    env[k] = execFileSync('powershell', ['-NoProfile', '-Command', `[Environment]::GetEnvironmentVariable('${k}','User')`], { encoding: 'utf8' }).trim();
  }
  if (!env[k]) throw new Error(`${k} is not set`);
}
const branch = process.argv[2] || 'main';
// The command runs through a shell on Windows, so only plain branch names are accepted.
if (!/^[A-Za-z0-9._-]{1,60}$/.test(branch)) throw new Error('branch name may use letters, digits, dot, dash and underscore only');
const dist = path.resolve(import.meta.dirname, '..', 'dist');
const g = inspectDist(dist, { secrets: [env.CLOUDFLARE_API_TOKEN] });
if (g.problems.length) throw new Error(`dist/ is not safe to deploy:\n  ${g.problems.slice(0, 20).join('\n  ')}`);
if (g.warn) console.warn(`WARNING ${g.warn}`);
execFileSync('npx', ['-y', 'wrangler@4.151.3', 'pages', 'deploy', 'dist', '--project-name', PAGES_PROJECT, '--branch', branch, '--commit-dirty=true'], { stdio: 'inherit', env, shell: true });
// Public checks on the live domain (HTTPS, redirects, canonical, sitemap, robots). A failure here fails the deploy command.
execFileSync(process.execPath, [path.join(import.meta.dirname, 'cloudflare-ops.mjs'), 'verify'], { stdio: 'inherit' });
