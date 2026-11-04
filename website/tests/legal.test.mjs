// Legal and contact pages: structure, footer links, writing rules, and hidden placeholder markers.
import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { DIST, ROOT, SKIP, read, resolvesInDist } from './_helpers.mjs';

const LEGAL = ['/legal/', '/legal/privacy/', '/legal/terms/', '/legal/refunds/', '/legal/cookies/', '/legal/notices/'];
const FOOTER = ['/legal/privacy/', '/legal/terms/', '/legal/refunds/', '/legal/cookies/', '/legal/notices/'];
const KEYS = ['licence-log-retention', 'grievance-officer', 'grievance-days', 'device-limit', 'lifetime-meaning', 'ai-model-terms', 'discontinuation', 'liability', 'governing-law'];
const page = (p) => read(path.join(DIST, p, 'index.html'));
const main = (h) => h.slice(h.indexOf('<main'), h.indexOf('</main>'));
const visible = (h) => h.replace(/<script[\s\S]*?<\/script>/g, '').replace(/<style[\s\S]*?<\/style>/g, '');
const text = (h) => visible(h).replace(/<[^>]+>/g, ' ');
const noLicence = (h) => h.replace(/<pre class="lic-text">[\s\S]*?<\/pre>/g, '');

test('legal pages exist, with one h1 and the update date', { skip: SKIP }, () => {
  for (const p of [...LEGAL, '/contact/']) {
    assert.ok(existsSync(path.join(DIST, p, 'index.html')), p);
    const h = page(p);
    assert.equal((h.match(/<h1[\s>]/g) || []).length, 1, `${p} needs exactly one h1`);
    if (p !== '/contact/') assert.match(text(h), /Last updated 3 November 2026/, p);
  }
  assert.match(page('/legal/cookies/'), /<h1>Cookies and browser storage\.<\/h1>/);
});

test('cookie claims are not absolute and name the host security cookies', { skip: SKIP }, () => {
  const ck = page('/legal/cookies/');
  assert.ok(ck.includes('<code>__cf_bm</code>') && ck.includes('<code>cf_clearance</code>'));
  for (const p of LEGAL) assert.ok(!/(This website|The site|It) sets no cookies[.,]/.test(text(page(p))), `${p} makes an absolute no-cookies claim`);
});

test('footer links to the five legal pages on every page, and they resolve', { skip: SKIP }, () => {
  for (const p of ['/', '/legal/', '/contact/', '/pricing/']) {
    const h = page(p);
    const foot = h.slice(h.indexOf('<footer'));
    for (const l of FOOTER) assert.ok(foot.includes(`href="${l}"`), `${p} footer lacks ${l}`);
    assert.ok(text(foot).replace(/\s+/g, " ").includes("Published by Vighnajith Advisories Private Limited · 12/1, Ground Floor, 22nd Main Road, Bengaluru, Karnataka 560078, India · support@openframestudio.app"), `${p} footer lacks the publisher line`);
  }
  for (const l of FOOTER) assert.ok(resolvesInDist(l), l);
});

test('hub links to every legal page', { skip: SKIP }, () => {
  const h = main(page('/legal/'));
  for (const l of FOOTER) assert.ok(h.includes(`href="${l}"`), l);
});

test('no dashes, no technology names, no banned wording', { skip: SKIP }, () => {
  const stack = /\b(Rust|SQLite|Tauri|React|TypeScript|SwiftUI|UniFFI|llama\.cpp|Gemma|Vulkan|Hugging Face|SHA-256|WebView2|StoreKit|Ed25519)\b/;
  const banned = /\b(seamless|elevate|unlock|robust|empower|game-changer|learn|course|web series|AI Assistant)\b/i;
  for (const p of [...LEGAL, '/contact/']) {
    const t = text(main(page(p)));
    assert.ok(!/[–—]/.test(t), `${p} has a dash`);
    assert.ok(!stack.test(t), `${p} names a technology: ${t.match(stack)?.[0]}`);
    assert.ok(!banned.test(t), `${p} has banned wording: ${t.match(banned)?.[0]}`);
  }
});

test('default build shows visitors no placeholder text, and keeps every hidden marker', { skip: SKIP }, () => {
  const all = [...LEGAL, '/contact/'].map((p) => noLicence(main(page(p)))).join('\n');
  assert.ok(!/class="ph"/.test(all), 'no visible placeholder styling');
  assert.ok(!/\[[A-Za-z][^\]]*\]/.test(text(all)), 'no bracketed text is visible');
  assert.ok(!/\b(TODO|TBD|lorem|coming soon|example\.com|your@)/i.test(text(all)), 'no placeholder wording');
  const found = [...all.matchAll(/<span data-placeholder="([a-z-]+)" hidden><\/span>/g)].map((m) => m[1]);
  assert.equal((all.match(/data-placeholder="/g) || []).length, found.length, 'every marker is the empty hidden span');
  assert.deepEqual([...new Set(found)].sort(), [...KEYS].sort());
});

test('every hidden item is listed in the report as an owner or lawyer action', () => {
  const report = read(path.join(ROOT, '.cache/agents/legal-report.md'));
  for (const k of KEYS) assert.ok(report.includes('`' + k + '`'), `${k} missing from legal-report.md`);
});

test('OF_SHOW_PLACEHOLDERS=1 renders the bracketed text for review builds', async () => {
  const { html, md } = await import('../src/lib/html.mjs');
  const run = async (v) => {
    if (v) process.env.OF_SHOW_PLACEHOLDERS = v; else delete process.env.OF_SHOW_PLACEHOLDERS;
    const mod = await import('../src/pages/legal.mjs');
    return mod.default({ html, md }).map((p) => String(p.body)).join('\n');
  };
  try {
    assert.ok(!/class="ph"/.test(await run('')));
    assert.match(await run('1'), /class="ph" data-placeholder="governing-law">\[/);
  } finally { delete process.env.OF_SHOW_PLACEHOLDERS; }
});

test('the contact page shows the support email and links the verified store pages', { skip: SKIP }, () => {
  const h = main(page('/contact/'));
  assert.ok(!h.includes('data-placeholder'));
  assert.ok(h.includes('href="mailto:support@openframestudio.app"'));
  assert.ok(h.includes('support.microsoft.com/en-us/accounts-billing/subscriptions/get-a-refund-for-apps-and-games-purchased-from-microsoft-store'));
  assert.ok(h.includes('support.apple.com/en-us/118223'));
  assert.ok(!/\+?\d[\d ()-]{8,}\d/.test(text(h)), 'no phone number may appear');
  for (const m of text(h).match(/[\w.+-]+@[\w-]+\.[\w.]+/g) || []) assert.ok(h.includes(`mailto:${m}`), `${m} must be a mailto link`);
  for (const p of ['/legal/privacy/', '/legal/terms/']) { const m = text(main(page(p))); assert.match(m, /12[/]1, Ground Floor, 22nd Main Road, Bengaluru, Karnataka 560078, India/); assert.ok(page(p).includes('href="mailto:support@openframestudio.app"')); }
});

test('privacy, cookies and refunds state the verified facts', { skip: SKIP }, () => {
  const pr = text(page('/legal/privacy/'));
  assert.match(pr, /Vighnajith Advisories Private Limited/);
  assert.match(pr, /Digital Personal Data Protection Act, 2023/);
  assert.match(pr, /data fiduciary/);
  const ck = page('/legal/cookies/');
  for (const k of ['of_os', 'of_theme', 'of_mock', 'of_region', 'of_recent_search', 'of_seen']) assert.ok(ck.includes(`<code>${k}</code>`), k);
  const rf = page('/legal/refunds/');
  assert.match(rf, /support\.microsoft\.com\/en-us\/accounts-billing\/subscriptions\/get-a-refund-for-apps-and-games-purchased-from-microsoft-store/);
  assert.match(rf, /support\.apple\.com\/en-us\/118223/);
  assert.match(page('/legal/terms/'), /apple\.com\/legal\/internet-services\/itunes\/dev\/stdeula\//);
});

test('notices publish the four font licences', { skip: SKIP }, () => {
  const h = page('/legal/notices/');
  for (const f of ['OFL-Inter.txt', 'OFL-Manrope.txt', 'OFL-Newsreader.txt', 'OFL-Sora.txt']) {
    assert.ok(h.includes(`/assets/fonts/${f}`), f);
    assert.ok(resolvesInDist(`/assets/fonts/${f}`), f);
  }
  assert.equal((h.match(/SIL OPEN FONT LICENSE Version 1\.1/g) || []).length, 4);
});

test('the old privacy, terms and contact pages are gone from info.mjs', () => {
  const info = read(path.join(ROOT, 'src/pages/info.mjs'));
  assert.ok(!info.includes("path: '/legal/"));
  assert.ok(!info.includes("path: '/contact/'"));
});
