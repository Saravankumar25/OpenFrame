// Cloudflare setup and live checks for the public domain. Safe to run again: every step looks first and changes only what differs.
//   node scripts/cloudflare-ops.mjs ensure   DNS, Pages domains, www redirect, SSL/TLS, pages.dev redirect
//   node scripts/cloudflare-ops.mjs verify   public checks only (no credentials needed): HTTPS, redirects, canonical, sitemap, robots
// Credentials come from CLOUDFLARE_API_TOKEN and CLOUDFLARE_ACCOUNT_ID (user variables on Windows) and are never printed.
// Token permissions: docs/OPERATIONS.md section 4.11.
import { execFileSync } from 'node:child_process';
import { SITE_URL, PAGES_PROJECT } from '../site.config.mjs';

const APEX = new URL(SITE_URL).hostname, WWW = `www.${APEX}`, PAGES_HOST = `${PAGES_PROJECT}.pages.dev`;
const ev = (k) => process.env[k] || (process.platform === 'win32' ? execFileSync('powershell', ['-NoProfile', '-Command', `[Environment]::GetEnvironmentVariable('${k}','User')`], { encoding: 'utf8' }).trim() : '');
const problems = [], notes = [];
const ok = (m) => console.log(`  ok    ${m}`), warn = (m) => { notes.push(m); console.log(`  WARN  ${m}`); }, bad = (m) => { problems.push(m); console.log(`  FAIL  ${m}`); };

async function ensure() {
  const token = ev('CLOUDFLARE_API_TOKEN'), account = ev('CLOUDFLARE_ACCOUNT_ID');
  if (!token || !account) throw new Error('CLOUDFLARE_API_TOKEN and CLOUDFLARE_ACCOUNT_ID must be set');
  const cf = async (method, path, body) => {
    const r = await fetch('https://api.cloudflare.com/client/v4' + path, { method, headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' }, body: body ? JSON.stringify(body) : undefined });
    const j = await r.json().catch(() => ({}));
    return { status: r.status, ok: !!j.success, result: j.result, errors: (j.errors || []).map((e) => e.message).join('; ') };
  };
  const need = (r, what, perm) => { if (!r.ok) { bad(`${what}: ${r.errors || r.status}${r.status === 403 ? ` (token needs: ${perm})` : ''}`); return false; } return true; };

  const zones = await cf('GET', `/zones?name=${APEX}`);
  if (!need(zones, 'zone lookup', 'Zone > Zone > Read') || !zones.result?.length) return bad(`zone ${APEX} not visible to the token`);
  const zone = zones.result[0].id;

  console.log('DNS');
  const recs = await cf('GET', `/zones/${zone}/dns_records?per_page=100`);
  if (need(recs, 'DNS list', 'Zone > DNS > Edit')) {
    for (const name of [APEX, WWW]) {
      const here = recs.result.filter((r) => r.name === name);
      const want = here.find((r) => r.type === 'CNAME' && r.content === PAGES_HOST && r.proxied);
      if (want) { ok(`${name} CNAME ${PAGES_HOST} (proxied)`); continue; }
      if (here.length) { bad(`${name} has other records (${here.map((r) => r.type).join(', ')}); fix by hand, nothing was changed`); continue; }
      const c = await cf('POST', `/zones/${zone}/dns_records`, { type: 'CNAME', name, content: PAGES_HOST, proxied: true, ttl: 1, comment: 'Pages custom domain' });
      if (need(c, `create ${name}`, 'Zone > DNS > Edit')) ok(`created ${name} CNAME ${PAGES_HOST} (proxied)`);
    }
  }

  console.log('Pages custom domains');
  const base = `/accounts/${account}/pages/projects/${PAGES_PROJECT}/domains`;
  const doms = await cf('GET', base);
  if (need(doms, 'Pages domains', 'Account > Cloudflare Pages > Edit')) {
    for (const name of [APEX, WWW]) {
      let d = doms.result.find((x) => x.name === name);
      if (!d) { const a = await cf('POST', base, { name }); if (!need(a, `add ${name}`, 'Account > Cloudflare Pages > Edit')) continue; d = a.result; }
      d.status === 'active' ? ok(`${name} active`) : warn(`${name} is ${d.status}; rerun in a few minutes`);
    }
  }

  console.log('Redirect www to apex');
  const rule = { ref: 'www_to_apex', description: `${WWW} to ${APEX}, 301, path and query kept`, enabled: true, action: 'redirect', expression: `(http.host eq "${WWW}")`,
    action_parameters: { from_value: { status_code: 301, preserve_query_string: true, target_url: { expression: `concat("https://${APEX}", http.request.uri.path)` } } } };
  const phase = `/zones/${zone}/rulesets/phases/http_request_dynamic_redirect/entrypoint`;
  const cur = await cf('GET', phase);
  const rules = cur.ok ? cur.result.rules || [] : [];
  if (!cur.ok && cur.status !== 404) need(cur, 'redirect rules', 'Zone > Single Redirect > Edit');
  else if (rules.some((r) => r.ref === 'www_to_apex' && r.expression === rule.expression && JSON.stringify(r.action_parameters) === JSON.stringify(rule.action_parameters))) ok('rule www_to_apex present');
  else if (need(await cf('PUT', phase, { rules: [...rules.filter((r) => r.ref !== 'www_to_apex'), rule] }), 'save redirect rule', 'Zone > Single Redirect > Edit')) ok('rule www_to_apex saved');

  console.log('SSL/TLS');
  for (const [id, value] of [['ssl', 'strict'], ['always_use_https', 'on'], ['min_tls_version', '1.2'], ['tls_1_3', 'on']]) {
    const g = await cf('GET', `/zones/${zone}/settings/${id}`);
    if (!need(g, `read ${id}`, 'Zone > Zone Settings > Edit')) continue;
    if (g.result.value === value) { ok(`${id} = ${value}`); continue; }
    if (need(await cf('PATCH', `/zones/${zone}/settings/${id}`, { value }), `set ${id}`, 'Zone > Zone Settings > Edit')) ok(`${id}: ${g.result.value} -> ${value}`);
  }
  const dnssec = await cf('GET', `/zones/${zone}/dnssec`);
  if (dnssec.ok) dnssec.result.status === 'active' ? ok('DNSSEC active') : warn(`DNSSEC is ${dnssec.result.status}: turn it on in the DNS page (Cloudflare Registrar adds the DS record by itself)`);

  console.log(`Redirect ${PAGES_HOST} to ${APEX} (account Bulk Redirect)`);
  const listName = 'pages_dev_to_domain', item = { redirect: { source_url: `${PAGES_HOST}/`, target_url: `https://${APEX}/`, status_code: 301, include_subdomains: false, subpath_matching: true, preserve_query_string: true, preserve_path_suffix: true } };
  const lists = await cf('GET', `/accounts/${account}/rules/lists`);
  if (!need(lists, 'Bulk Redirect lists', 'Account > Account Filter Lists > Edit')) return;
  let list = lists.result.find((l) => l.name === listName);
  if (!list) { const c = await cf('POST', `/accounts/${account}/rules/lists`, { name: listName, kind: 'redirect', description: `${PAGES_HOST} to ${APEX}` }); if (!need(c, 'create list', 'Account > Account Filter Lists > Edit')) return; list = c.result; }
  const items = await cf('GET', `/accounts/${account}/rules/lists/${list.id}/items`);
  if (need(items, 'list items', 'Account > Account Filter Lists > Edit') && !items.result.length) need(await cf('POST', `/accounts/${account}/rules/lists/${list.id}/items`, [item]), 'add list item', 'Account > Account Filter Lists > Edit');
  const bphase = `/accounts/${account}/rulesets/phases/http_request_redirect/entrypoint`;
  const bcur = await cf('GET', bphase);
  const brules = bcur.ok ? bcur.result.rules || [] : [];
  if (!bcur.ok && bcur.status !== 404) return void need(bcur, 'Bulk Redirect rules', 'Account > Mass URL Redirects > Edit');
  if (brules.some((r) => r.ref === 'pages_dev_to_domain')) return ok('Bulk Redirect rule present');
  const brule = { ref: 'pages_dev_to_domain', description: `${PAGES_HOST} to ${APEX}`, enabled: true, action: 'redirect', expression: `http.request.full_uri in $${listName}`, action_parameters: { from_list: { name: listName, key: 'http.request.full_uri' } } };
  if (need(await cf('PUT', bphase, { rules: [...brules, brule] }), 'save Bulk Redirect rule', 'Account > Mass URL Redirects > Edit')) ok('Bulk Redirect rule saved');
}

// Public checks. A fresh deploy or rule can take a minute to show, so each check retries.
async function verify() {
  const get = async (url, opt = {}) => { try { return await fetch(url, { redirect: 'manual', headers: { 'user-agent': 'openframe-verify' }, ...opt }); } catch (e) { return { status: 0, error: e.cause?.code || e.message, headers: new Headers(), text: async () => '' }; } };
  const retry = async (label, fn) => { let last = ''; for (let i = 0; i < 6; i++) { last = await fn(); if (!last) return ok(label); await new Promise((r) => setTimeout(r, 10000)); } bad(`${label}: ${last}`); };
  console.log('Live checks');
  const home = `https://${APEX}/`;
  await retry(`${home} returns 200 with a valid certificate`, async () => { const r = await get(home); return r.status === 200 ? '' : `status ${r.status} ${r.error || ''}`; });
  await retry(`canonical tag is ${home}`, async () => { const t = await (await get(home)).text(); return t.includes(`<link rel="canonical" href="${home}">`) ? '' : 'canonical tag missing or wrong'; });
  await retry('robots.txt allows crawling and names the sitemap', async () => { const t = await (await get(`${home}robots.txt`)).text(); return t.includes(`Sitemap: ${SITE_URL}/sitemap.xml`) && !/^Disallow: \/\s*$/m.test(t) ? '' : 'robots.txt wrong'; });
  await retry('sitemap.xml lists only the domain', async () => { const r = await get(`${home}sitemap.xml`); const t = await r.text(); return r.status === 200 && t.includes(`<loc>${home}`) && !t.includes('pages.dev') ? '' : `status ${r.status} or wrong host`; });
  await retry('http redirects to https', async () => { const r = await get(`http://${APEX}/`); return r.status === 301 && r.headers.get('location') === home ? '' : `status ${r.status} to ${r.headers.get('location')}`; });
  await retry(`${WWW} redirects to the apex with path and query`, async () => { const r = await get(`https://${WWW}/features?x=1`); return r.status === 301 && r.headers.get('location') === `${home}features?x=1` ? '' : `status ${r.status} to ${r.headers.get('location')}`; });
  const r = await get(`https://${PAGES_HOST}/features?x=1`);
  if (r.status === 301 && r.headers.get('location') === `${home}features?x=1`) ok(`${PAGES_HOST} redirects to the apex with path and query`);
  else warn(`${PAGES_HOST} does not redirect yet (status ${r.status}); the Bulk Redirect needs account token permissions`);
}

const cmd = process.argv[2];
if (cmd === 'ensure') await ensure(); else if (cmd === 'verify') await verify(); else { console.error('usage: cloudflare-ops.mjs ensure|verify'); process.exit(2); }
console.log(problems.length ? `\n${problems.length} problem(s)` : `\nall checks passed${notes.length ? ` (${notes.length} warning(s))` : ''}`);
process.exit(problems.length ? 1 : 0);
