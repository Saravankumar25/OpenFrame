// Deploy dist/ to Cloudflare Pages (project openframe-studio). Credentials come from the environment
// (CLOUDFLARE_API_TOKEN, CLOUDFLARE_ACCOUNT_ID); on Windows they are read from the user's saved variables.
import { execFileSync } from 'node:child_process';
import { PAGES_PROJECT } from '../site.config.mjs';
const env = { ...process.env };
for (const k of ['CLOUDFLARE_API_TOKEN', 'CLOUDFLARE_ACCOUNT_ID']) {
  if (!env[k] && process.platform === 'win32') {
    env[k] = execFileSync('powershell', ['-NoProfile', '-Command', `[Environment]::GetEnvironmentVariable('${k}','User')`], { encoding: 'utf8' }).trim();
  }
  if (!env[k]) throw new Error(`${k} is not set`);
}
const branch = process.argv[2] || 'main';
execFileSync('npx', ['-y', 'wrangler@4.151.3', 'pages', 'deploy', 'dist', '--project-name', PAGES_PROJECT, '--branch', branch, '--commit-dirty=true'], { stdio: 'inherit', env, shell: true });
