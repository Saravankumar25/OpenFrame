// IndexNow sending rules: host refusal, response codes, retries, Retry-After. No network and no dist/ needed.
import test from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { SITE_URL } from '../site.config.mjs';
import { ENDPOINT, MAX_AUTO, assertOwnHost, payloads, retryAfterMs, submit } from '../scripts/indexnow.mjs';

const key = 'k'.repeat(32);
const body = (urls) => ({ host: new URL(SITE_URL).host, key, keyLocation: `${SITE_URL}/${key}.txt`, urlList: urls });
const ok = body([`${SITE_URL}/a/`]);
const reply = (status, headers = {}) => ({ status, headers: { get: (n) => headers[n.toLowerCase()] ?? null } });
/** A fake fetch that answers from a list and records the calls; a fake sleep that records the waits. */
const fake = (answers) => {
  const calls = [], waits = [];
  return { calls, waits, io: { fetch: async (url, init) => { calls.push({ url, init }); const a = answers.shift(); if (a instanceof Error) throw a; return a; }, sleep: async (ms) => { waits.push(ms); } } };
};
const quiet = async (fn) => { const log = console.log; console.log = () => {}; try { return await fn(); } finally { console.log = log; } };

test('IndexNow refuses any URL or host that is not https on the site host', async () => {
  assert.doesNotThrow(() => assertOwnHost(ok, SITE_URL));
  for (const bad of ['https://elsewhere.example/x/', `http://${new URL(SITE_URL).host}/x/`, `https://www.${new URL(SITE_URL).host}/x/`, 'not a url'])
    assert.throws(() => assertOwnHost(body([`${SITE_URL}/a/`, bad]), SITE_URL), /refusing/, bad);
  assert.throws(() => assertOwnHost({ ...ok, host: 'elsewhere.example' }, SITE_URL), /only .* is allowed/);
  const f = fake([reply(200)]);
  await assert.rejects(submit(body(['https://elsewhere.example/x/']), SITE_URL, f.io), /refusing/);
  assert.equal(f.calls.length, 0, 'nothing is sent when a URL is refused');
});

test('IndexNow sends only to the official endpoint, as JSON, and splits at 10,000 URLs', async () => {
  const f = fake([reply(200)]);
  assert.equal(await submit(ok, SITE_URL, f.io), 200);
  assert.equal(f.calls[0].url, 'https://api.indexnow.org/indexnow');
  assert.equal(ENDPOINT, 'https://api.indexnow.org/indexnow');
  assert.match(f.calls[0].init.headers['content-type'], /application\/json/);
  assert.ok(f.calls[0].init.signal, 'a timeout signal is set');
  const many = [...Array(20001)].map((_, i) => `${SITE_URL}/p${i}/`);
  assert.deepEqual(payloads(many, SITE_URL, key).map((b) => b.urlList.length), [10000, 10000, 1]);
});

test('IndexNow: 200 and 202 succeed, 400, 403 and 422 fail at once with a clear message and no key', async () => {
  assert.equal(await submit(ok, SITE_URL, fake([reply(202)]).io), 202);
  for (const s of [400, 403, 422]) {
    const f = fake([reply(s), reply(200)]);
    await assert.rejects(submit(ok, SITE_URL, f.io), (e) => e.message.includes(String(s)) && !e.message.includes(key));
    assert.equal(f.calls.length, 1, `${s} is not retried`);
  }
  await assert.rejects(submit(ok, SITE_URL, fake([reply(404)]).io), /unexpected 404/);
});

test('IndexNow retries 429, 5xx and network errors with backoff, honours Retry-After, then gives up', async () => {
  const f = fake([reply(429, { 'retry-after': '7' }), reply(503), new Error('reset'), reply(200)]);
  assert.equal(await quiet(() => submit(ok, SITE_URL, f.io)), 200);
  assert.deepEqual(f.waits, [7000, 4000, 8000]);
  const g = fake([reply(500), reply(500), reply(500), reply(500)]);
  await assert.rejects(quiet(() => submit(ok, SITE_URL, g.io)), /still failing after 3 retries \(status 500\)/);
  assert.equal(g.calls.length, 4);
  assert.equal(retryAfterMs('9999'), 60000, 'capped at a minute');
  assert.equal(retryAfterMs(null), null);
  assert.equal(retryAfterMs('soon'), null);
  assert.equal(retryAfterMs(new Date(1000 + 5000).toUTCString(), 1000 - (1000 % 1000)), 5000);
});

test('IndexNow CLI: no mode is refused, and --all is the only way to send every URL', () => {
  const run = (...a) => { try { return { code: 0, out: execFileSync(process.execPath, [path.join(import.meta.dirname, '..', 'scripts', 'indexnow.mjs'), ...a], { encoding: 'utf8', stdio: 'pipe' }) }; } catch (e) { return { code: e.status, out: `${e.stdout}${e.stderr}` }; } };
  const bare = run();
  assert.notEqual(bare.code, 0);
  assert.match(bare.out, /choose exactly one of/);
  const soft = run('--soft');
  assert.equal(soft.code, 0, 'soft mode never fails the deploy');
  assert.match(soft.out, /IndexNow skipped/);
  assert.ok(MAX_AUTO <= 500);
});
