import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { SITE_URL } from '../site.config.mjs';
import { DIST, SKIP, htmlFiles, pagePath, read, resolvesInDist } from './_helpers.mjs';

const files = htmlFiles();
const pages = files.map((f) => {
  const html = read(f);
  const tags = (name) => [...html.matchAll(new RegExp(`<${name}\\b[^>]*>`, 'gi'))].map((m) => m[0]);
  const attr = (tag, a) => {
    const m = tag.match(new RegExp(`\\s${a}\\s*=\\s*(?:"([^"]*)"|'([^']*)')`, 'i'));
    return m ? (m[1] ?? m[2]) : null;
  };
  return { file: f, rel: path.relative(DIST, f).split(path.sep).join('/'), path: pagePath(f), html, tags, attr };
});
const is404 = (p) => p.rel === '404.html';
const noindex = (p) => p.tags('meta').some((t) => /name=["']robots["']/i.test(t) && /noindex/i.test(t));
const indexable = pages.filter((p) => !is404(p) && !noindex(p));
const metaContent = (p, key) => {
  const t = p.tags('meta').find((x) => new RegExp(`(name|property)=["']${key}["']`, 'i').test(x));
  return t ? p.attr(t, 'content') : null;
};
const esc = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

test('dist has pages', { skip: SKIP }, () => assert.ok(pages.length > 5, `only ${pages.length} html files`));

test('every page: one h1, lang, title, description, og tags, canonical', { skip: SKIP }, () => {
  const errs = [];
  const titles = new Map();
  const descs = new Map();
  for (const p of pages) {
    const e = (m) => errs.push(`${p.rel}: ${m}`);
    const h1s = (p.html.match(/<h1[\s>]/gi) || []).length;
    if (h1s !== 1) e(`${h1s} h1 elements`);
    if (!/<html\b[^>]*\blang=["']en["']/i.test(p.html)) e('html lang="en" missing');
    const title = p.html.match(/<title>([\s\S]*?)<\/title>/i)?.[1].trim();
    if (!title) e('no <title>');
    else {
      if (title.length > 65) e(`title ${title.length} chars: ${title}`);
      if (titles.has(title)) e(`duplicate title with ${titles.get(title)}`);
      else titles.set(title, p.rel);
    }
    const desc = metaContent(p, 'description');
    if (!desc) e('no meta description');
    else if (descs.has(desc)) e(`duplicate description with ${descs.get(desc)}`);
    else descs.set(desc, p.rel);
    for (const k of ['og:title', 'og:description', 'og:url']) if (!metaContent(p, k)) e(`${k} missing`);
    if (is404(p) || noindex(p)) continue;
    const canon = p.tags('link').find((t) => /rel=["']canonical["']/i.test(t));
    const href = canon && p.attr(canon, 'href');
    if (!href) e('no canonical');
    else {
      if (!/^https?:\/\//.test(href)) e(`canonical not absolute: ${href}`);
      if (href !== SITE_URL + p.path) e(`canonical ${href} != ${SITE_URL + p.path}`);
    }
  }
  assert.deepEqual(errs, []);
});

test('images have alt, width, height; no inline style; no stray inline script', { skip: SKIP }, () => {
  const errs = [];
  for (const p of pages) {
    const e = (m) => errs.push(`${p.rel}: ${m}`);
    for (const t of p.tags('img')) {
      if (p.attr(t, 'alt') === null) e(`img without alt: ${t.slice(0, 90)}`);
      if (!p.attr(t, 'width') || !p.attr(t, 'height')) e(`img without width/height: ${t.slice(0, 90)}`);
    }
    // match inside tags only, so text that mentions style="" is not flagged
    if (/<[a-z][^>]*\sstyle\s*=/i.test(p.html.replace(/<script\b[\s\S]*?<\/script>/gi, ''))) e('inline style attribute');
    for (const m of p.html.matchAll(/<script\b([^>]*)>/gi)) {
      const a = m[1];
      if (/\ssrc\s*=/i.test(a)) continue;
      if (/type=["']application\/(ld\+)?json["']/i.test(a) || /id=["']os-boot["']/i.test(a)) continue;
      e(`inline script not allowed: <script${a}>`);
    }
    if (/<[a-z][^>]*\son[a-z]+\s*=/i.test(p.html.replace(/<script\b[\s\S]*?<\/script>/gi, ''))) e('inline event handler attribute');
  }
  assert.deepEqual(errs, []);
});

test('JSON-LD parses', { skip: SKIP }, () => {
  const errs = [];
  for (const p of pages) {
    for (const m of p.html.matchAll(/<script\b[^>]*type=["']application\/ld\+json["'][^>]*>([\s\S]*?)<\/script>/gi)) {
      try { JSON.parse(m[1]); } catch (x) { errs.push(`${p.rel}: ${x.message}`); }
    }
  }
  assert.deepEqual(errs, []);
});

test('no em or en dashes in page text, no placeholder strings', { skip: SKIP }, () => {
  const errs = [];
  const banned = [/example\.com/i, /lorem/i, /TODO/, /#microsoft-store-link/];
  for (const p of pages) {
    const text = p.html.replace(/<style\b[\s\S]*?<\/style>/gi, '').replace(/<script\b(?![^>]*ld\+json)[\s\S]*?<\/script>/gi, '');
    if (/[–—]|&mdash;|&ndash;|&#8212;|&#8211;|&#x2014;|&#x2013;/i.test(text)) errs.push(`${p.rel}: em or en dash`);
    for (const b of banned) if (b.test(p.html)) errs.push(`${p.rel}: contains ${b}`);
  }
  assert.deepEqual(errs, []);
});

test('every internal href/src resolves to a file in dist', { skip: SKIP }, () => {
  const errs = [];
  for (const p of pages) {
    const html = p.html.replace(/<script\b(?![^>]*\bsrc=)[\s\S]*?<\/script>/gi, '');
    const urls = [];
    for (const m of html.matchAll(/<[a-z][^>]*>/gi)) {
      for (const a of ['href', 'src', 'poster']) {
        const v = p.attr(m[0], a);
        if (v) urls.push(v);
      }
      const ss = p.attr(m[0], 'srcset');
      if (ss) for (const part of ss.split(',')) { const u = part.trim().split(/\s+/)[0]; if (u) urls.push(u); }
    }
    for (const u of urls) {
      if (!u.startsWith('/') || u.startsWith('//')) continue;
      const clean = u.split('#')[0].split('?')[0];
      if (!resolvesInDist(clean)) errs.push(`${p.rel}: broken ${u}`);
    }
  }
  assert.deepEqual(errs, []);
});

test('sitemap lists every indexable page and only existing ones', { skip: SKIP }, () => {
  const f = path.join(DIST, 'sitemap.xml');
  assert.ok(existsSync(f), 'dist/sitemap.xml missing');
  const locs = [...read(f).matchAll(/<loc>\s*([^<\s]+)\s*<\/loc>/g)].map((m) => m[1]);
  assert.equal(new Set(locs).size, locs.length, 'duplicate sitemap entries');
  const set = new Set(locs);
  const errs = [];
  for (const p of indexable) if (!set.has(SITE_URL + p.path)) errs.push(`missing from sitemap: ${p.path}`);
  for (const l of locs) {
    if (!l.startsWith(SITE_URL)) { errs.push(`foreign url in sitemap: ${l}`); continue; }
    if (!resolvesInDist(l.slice(SITE_URL.length) || '/')) errs.push(`sitemap url has no file: ${l}`);
    else if (!indexable.some((p) => SITE_URL + p.path === l)) errs.push(`sitemap lists non-indexable page: ${l}`);
  }
  assert.deepEqual(errs, []);
});

test('robots.txt, _headers, _redirects, 404, llms.txt', { skip: SKIP }, () => {
  assert.match(read(path.join(DIST, 'robots.txt')), new RegExp(`Sitemap:\\s*${esc(SITE_URL)}/sitemap\\.xml`, 'i'));
  assert.match(read(path.join(DIST, '_headers')), /Content-Security-Policy/i);
  for (const f of ['_redirects', '404.html', 'llms.txt']) assert.ok(existsSync(path.join(DIST, f)), `${f} missing`);
});
