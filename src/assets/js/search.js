/* OpenFrame site search: a command palette in the header, the /search/ page and the filter box on the tutorial lists.
   One same-origin file (/search-index.json) is fetched the first time search opens. No libraries, nothing is sent anywhere.
   Part 1 is the matching engine (also loaded by tests/search.test.mjs). Part 2 is the interface. */
var Core = (function () {
  'use strict';
  var STOP = {};
  'a an the to of in on at for with and or how do does did i my me can you your is are it this that what where when why which be by from as into about'.split(' ').forEach(function (w) { STOP[w] = 1; });
  // Product synonyms. "a = b = c" are the same thing; "a > b | c" means a search for a also finds b and c, not the other way round.
  var SYNONYMS = [
    'delete = remove = trash = erase = discard = recently deleted', 'export = save as = pdf = output', 'import = bring in = load', 'open > import',
    'script = screenplay = screen play', 'ai = openframe ai = assistant', 'call sheet = callsheet = call sheets',
    'schedule = stripboard = shooting day = shooting schedule = strips', 'shortcut = shortcuts = keyboard = hotkey = key binding',
    'theme = appearance = dark mode = light mode', 'backup = back up = restore = recover = recovery', 'backup > snapshot',
    'share > package | send', 'send > package', 'package = pack = zip', 'color = colour', 'organize = organise', 'catalog = catalogue',
    'cast = actor = talent', 'crew = staff', 'location = place = venue', 'budget = costs', 'price = pricing = cost = purchase = buy = payment',
    'storyboard = story board', 'shot list = shotlist', 'day out of days = dood', 'undo = revert', 'duplicate = clone', 'find = search = look up',
    'reorder = rearrange = arrange = sort', 'lock = freeze = finalize = finalise', 'revision = version', 'setup = set up = install',
    'rename = retitle', 'note = notes', 'idea vault = vault', 'beat board = beats'
  ];
  var BOOST = [10, 6, 5, 3, 1.5]; // title, area, keywords, summary, steps
  var TYPES = [['tutorial', 'Tutorials'], ['feature', 'Features'], ['answer', 'Answers']];

  /* ---------- text ---------- */
  function fold(s) { return String(s == null ? '' : s).normalize('NFD').replace(/[̀-ͯ]/g, '').toLowerCase(); }
  function words(s) { return fold(s).replace(/(\d),(?=\d{3}\b)/g, '$1').replace(/['’]/g, '').match(/[a-z0-9]+/g) || []; }
  function esc(s) { return String(s).replace(/[&<>"']/g, function (c) { return { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]; }); }
  function stem(w) {
    var n = w.length;
    if (n > 4 && /ies$/.test(w)) return w.slice(0, -3) + 'y';
    if (n > 5 && /ing$/.test(w)) return w.slice(0, -3);
    if (n > 4 && /ed$/.test(w)) return w.slice(0, -2);
    if (n > 3 && /s$/.test(w) && !/(ss|us|is)$/.test(w)) return w.slice(0, -1);
    return w;
  }
  // Edit distance with swaps counting as one edit; gives up (returns k + 1) once it must exceed k.
  function dist(a, b, k) {
    var la = a.length, lb = b.length, i, j;
    if (Math.abs(la - lb) > k) return k + 1;
    var p2 = null, p = [], c;
    for (j = 0; j <= lb; j++) p[j] = j;
    for (i = 1; i <= la; i++) {
      c = [i];
      var low = i;
      for (j = 1; j <= lb; j++) {
        var v = Math.min(p[j] + 1, c[j - 1] + 1, p[j - 1] + (a.charCodeAt(i - 1) === b.charCodeAt(j - 1) ? 0 : 1));
        if (i > 1 && j > 1 && a.charCodeAt(i - 1) === b.charCodeAt(j - 2) && a.charCodeAt(i - 2) === b.charCodeAt(j - 1)) v = Math.min(v, p2[j - 2] + 1);
        c[j] = v;
        if (v < low) low = v;
      }
      if (low > k) return k + 1;
      p2 = p; p = c;
    }
    return p[lb];
  }

  /* ---------- the query ---------- */
  // "exact phrase", -word, -"phrase" and plain words. An unclosed quote counts as closed (the person is still typing).
  function parse(raw) {
    var s = String(raw || ''), P = { terms: [], phrases: [], neg: [], negPhrases: [] };
    if ((s.match(/"/g) || []).length % 2) s += '"';
    var re = /(-?)"([^"]*)"|(-?)([^\s"]+)/g, m;
    while ((m = re.exec(s))) {
      var neg = m[1] === '-' || m[3] === '-', w = words(m[2] != null ? m[2] : m[4]);
      if (!w.length) continue;
      if (m[2] != null) (neg ? P.negPhrases : P.phrases).push(w);
      else if (neg) P.neg.push(w.join(''));
      else P.terms.push.apply(P.terms, w);
    }
    return P;
  }
  // One query word as a function: token -> how well it matches (1 exact, .85 starts with the word's stem, .6 / .45 a typo away).
  function matcher(q, pmin, typos) {
    var st = stem(q), k = typos ? (q.length >= 8 ? 2 : q.length >= 4 ? 1 : 0) : 0;
    return function (t, fuzzy) {
      if (t === q) return 1;
      if (st.length >= pmin && t.indexOf(st) === 0) return t === st ? 0.95 : 0.85;
      if (fuzzy && k && t.charAt(0) === q.charAt(0)) { var d = dist(q, t, k); if (d <= k) return d === 1 ? 0.6 : 0.45; }
      return 0;
    };
  }
  function expand(vocab, m, fuzzy) {
    var out = new Map();
    vocab.forEach(function (_, t) { var w = m(t, false); if (w) out.set(t, w); });
    if (!out.size && fuzzy) vocab.forEach(function (_, t) { var w = m(t, true); if (w) out.set(t, w); });
    return out;
  }
  var SYN = {};
  SYNONYMS.forEach(function (line) {
    var one = line.indexOf('>') > -1, from = [], to = [];
    if (one) { from = [line.split('>')[0]]; to = line.split('>')[1].split('|'); } else { from = to = line.split('='); }
    from.forEach(function (a) {
      var ka = words(a).map(stem).join(' ');
      to.forEach(function (b) {
        var wb = words(b);
        if (wb.join(' ') !== words(a).join(' ')) (SYN[ka] = SYN[ka] || []).push({ words: wb, w: one ? 0.5 : 0.8 });
      });
    });
  });
  // The query as units. A unit is a word or a known phrase and carries options: the words as typed, or a synonym.
  // An option is a list of token maps that must all match. Words found nowhere in the index are set aside.
  function buildUnits(vocab, P, keepStops) {
    var T = P.terms, units = [], dropped = [], failed = false, i = 0, n, len, key, alts, last;
    while (i < T.length) {
      alts = null; len = 1;
      for (n = Math.min(4, T.length - i); n >= 1; n--) {
        key = T.slice(i, i + n).map(stem).join(' ');
        if (SYN[key]) { alts = SYN[key]; len = n; break; }
      }
      if (!alts && !keepStops && STOP[T[i]] && T.length > 1) { i++; continue; }
      last = i + len >= T.length && !P.phrases.length;
      var span = T.slice(i, i + len), opts = [{ w: 1, re: alts && len > 1 ? new RegExp('(?:^| )' + span.map(stem).join('[a-z0-9]* ') + '[a-z0-9]*(?: |$)') : null, maps: T.slice(i, i + len).map(function (q, j) { return expand(vocab, matcher(q, last && j === len - 1 ? 2 : 3, true), true); }) }];
      (alts || []).forEach(function (a) { opts.push({ w: a.w, maps: a.words.map(function (q) { return expand(vocab, matcher(q, 3, false), false); }) }); });
      opts = opts.filter(function (o) { return o.maps.every(function (m) { return m.size; }); });
      if (opts.length) units.push({ opts: opts }); else dropped.push(T.slice(i, i + len).join(' '));
      i += len;
    }
    if (!units.length && T.length && !keepStops) return buildUnits(vocab, P, true);
    P.phrases.forEach(function (ph) {
      var maps = ph.map(function (q) { var m = new Map(); if (vocab.has(q)) m.set(q, 1); return m; });
      if (maps.every(function (m) { return m.size; })) units.push({ opts: [{ w: 1, maps: maps }] }); else failed = true;
    });
    return { units: units, dropped: dropped, failed: failed };
  }

  /* ---------- the index ---------- */
  function prepare(data) {
    var E = data.entries, post = new Map(), vocab = new Map();
    E.forEach(function (e, i) {
      var f = e._f = { title: words(e.title).join(' '), area: words(e.area).join(' '), kw: words(e.keywords).join(' '), sum: words(e.summary).join(' '), steps: e.steps.map(function (s) { return words(s).join(' '); }) };
      [f.title, f.area, f.kw, f.sum, f.steps.join(' ')].forEach(function (text, b) {
        text.split(' ').forEach(function (t) {
          if (!t) return;
          var m = post.get(t); if (!m) post.set(t, m = new Map());
          m.set(i, (m.get(i) || 0) | (1 << b));
        });
      });
    });
    post.forEach(function (m, t) { vocab.set(t, m.size); });
    return { E: E, post: post, vocab: vocab, parts: data.parts || [] };
  }
  function pass(e, f) {
    if ((f.os === 'win' || f.os === 'mac') && e.os !== f.os && e.os !== 'both') return false;
    return (!f.type || e.type === f.type) && (!f.part || e.part === f.part) && (!f.area || e.area === f.area);
  }
  function scoreMap(ix, m) {
    var r = new Map();
    m.forEach(function (w, t) {
      ix.post.get(t).forEach(function (bits, i) {
        var s = 0;
        for (var b = 0; b < 5; b++) if (bits & (1 << b)) s = Math.max(s, w * BOOST[b]);
        if (s > (r.get(i) || 0)) r.set(i, s);
      });
    });
    return r;
  }
  function both(a, b) { var r = new Map(); a.forEach(function (s, i) { if (b.has(i)) r.set(i, s + b.get(i)); }); return r; }
  function fields(F) { return [F.title, F.area, F.kw, F.sum].concat(F.steps); }
  function has(text, ph) { return (' ' + text + ' ').indexOf(' ' + ph + ' ') > -1; }

  // f: { os: 'win' | 'mac' | 'all', pref: the platform to favour, type, part, area }. Returns { hits: [{ i, s }], hl: Set, ignored: [words] }.
  function search(ix, raw, f) {
    f = f || {};
    var P = parse(raw), U = buildUnits(ix.vocab, P), hl = new Set(), hits = [], excl = new Set(), cand = null;
    P.neg.forEach(function (w) { var m = matcher(w, 3, false); ix.vocab.forEach(function (_, t) { if (m(t)) ix.post.get(t).forEach(function (_b, i) { excl.add(i); }); }); });
    if (U.failed || (!U.units.length && (P.terms.length || P.phrases.length))) return { hits: hits, hl: hl, ignored: U.dropped };
    if (!U.units.length) { // no words: browse by filter
      if (f.type || f.part || f.area) ix.E.forEach(function (e, i) { if (!excl.has(i) && pass(e, f)) hits.push({ i: i, s: 0 }); });
      return { hits: hits, hl: hl, ignored: [] };
    }
    U.units.forEach(function (u) {
      var um = new Map();
      u.opts.forEach(function (o) {
        var om = null;
        o.maps.forEach(function (m) { m.forEach(function (_, t) { if (!STOP[t]) hl.add(t); }); var sm = scoreMap(ix, m); om = om ? both(om, sm) : sm; });
        om.forEach(function (s, i) {
          if (o.re && !fields(ix.E[i]._f).some(function (t) { return o.re.test(t); })) return;
          s = s / o.maps.length * o.w; if (!(um.get(i) >= s)) um.set(i, s); });
      });
      cand = cand ? both(cand, um) : um;
    });
    var plain = P.terms.filter(function (t) { return !STOP[t]; }).join(' ');
    cand.forEach(function (s, i) {
      var e = ix.E[i], F = e._f;
      if (excl.has(i) || !pass(e, f)) return;
      var all = fields(F), k;
      for (k = 0; k < P.negPhrases.length; k++) if (all.some(function (t) { return has(t, P.negPhrases[k].join(' ')); })) return;
      for (k = 0; k < P.phrases.length; k++) {
        var ph = P.phrases[k].join(' ');
        if (has(F.title, ph)) s += 30; else if (has(F.kw, ph) || has(F.area, ph)) s += 20; else if (has(F.sum, ph)) s += 12; else if (F.steps.some(function (t) { return has(t, ph); })) s += 8; else return;
      }
      if (plain) {
        if (F.title === plain) s += 40; else if (has(F.title, plain)) s += 18; else if (F.title.indexOf(plain) === 0) s += 6;
      }
      if (f.pref && e.os === f.pref) s *= 1.25;
      hits.push({ i: i, s: s });
    });
    hits.sort(function (a, b) { return b.s - a.s || ix.E[a.i].title.length - ix.E[b.i].title.length || a.i - b.i; });
    return { hits: hits, hl: hl, ignored: U.dropped };
  }
  // "Did you mean": replace each word the index does not know by the closest indexed word, and keep the suggestion only if it finds something.
  function suggest(ix, raw, f) {
    var P = parse(raw), changed = false, out = P.terms.map(function (t) {
      var m = matcher(t, 2, false), known = false, best = null, bd = 9, k = Math.min(3, Math.max(1, Math.floor(t.length / 3)));
      if (STOP[t] || ix.vocab.has(t)) return t;
      ix.vocab.forEach(function (df, v) {
        if (m(v)) known = true;
        if (known) return;
        var d = dist(t, v, k);
        if (d <= k && (d < bd || (d === bd && df > ix.vocab.get(best)))) { bd = d; best = v; }
      });
      if (known || !best) return t;
      changed = true; return best;
    });
    if (!changed) return '';
    var s = out.join(' ');
    return search(ix, s, f).hits.length ? s : '';
  }
  function groups(ix, hits, caps) {
    var by = {}, out = [];
    hits.forEach(function (h) { (by[ix.E[h.i].type] = by[ix.E[h.i].type] || []).push(h); });
    TYPES.forEach(function (t) { if (by[t[0]]) out.push({ type: t[0], label: t[1], hits: by[t[0]], shown: by[t[0]].slice(0, caps[t[0]] || 99), top: by[t[0]][0].s }); });
    return out.sort(function (a, b) { return b.top - a.top; });
  }

  /* ---------- text shown to the reader ---------- */
  function mark(text, hl) {
    var out = '', last = 0, re = /[\p{L}\p{N}]+(?:['’][\p{L}\p{N}]+)*/gu, m, ws, j;
    while ((m = re.exec(text))) {
      ws = words(m[0]);
      for (j = 0; j < ws.length; j++) if (hl.has(ws[j])) { out += esc(text.slice(last, m.index)) + '<mark>' + esc(m[0]) + '</mark>'; last = m.index + m[0].length; break; }
    }
    return out + esc(text.slice(last));
  }
  function hitCount(text, hl) { var seen = {}, n = 0; words(text).forEach(function (w) { if (hl.has(w) && !seen[w]) { seen[w] = 1; n++; } }); return n; }
  // The best step (or the summary when it matches at least as well), cut around the first matching word.
  function snippet(e, hl, len) {
    len = len || 150;
    var best = e.summary, bn = hitCount(e.summary, hl);
    e.steps.forEach(function (s) { var n = hitCount(s, hl); if (n > bn) { bn = n; best = s; } });
    var text = best, at = 0, re = /[\p{L}\p{N}]+/gu, m;
    if (bn) while ((m = re.exec(text))) { if (words(m[0]).some(function (w) { return hl.has(w); })) { at = m.index; break; } }
    var start = at > 40 ? text.indexOf(' ', at - 40) + 1 || at : 0;
    var cut = text.slice(start, start + len), tail = start + len < text.length;
    if (tail) cut = cut.replace(/\s+\S*$/, '');
    return (start ? '... ' : '') + mark(cut, hl) + (tail ? ' ...' : '');
  }
  // The tutorial list filter: does this card text match the typed words (prefix, typo and synonym aware)?
  function matchText(raw, text) {
    var P = parse(raw), vocab = new Map(), n = words(text);
    if (!P.terms.length && !P.phrases.length && !P.neg.length) return true;
    n.forEach(function (w) { vocab.set(w, 1); });
    var U = buildUnits(vocab, P), joined = n.join(' ');
    if (U.failed || U.dropped.length) return false;
    for (var i = 0; i < P.neg.length; i++) if (n.some(matcher(P.neg[i], 3, false))) return false;
    return P.phrases.every(function (ph) { return has(joined, ph.join(' ')); });
  }

  return { fold: fold, words: words, parse: parse, prepare: prepare, search: search, suggest: suggest, groups: groups, mark: mark, snippet: snippet, matchText: matchText, esc: esc, TYPES: TYPES };
})();

/* ---------- the interface ---------- */
function ui(Core) {
  'use strict';
  var doc = document.documentElement, KEY = 'of_recent_search', ix = null, loading = null, esc = Core.esc;
  var SUGGEST = ['call sheet', 'export pdf', 'import a script', 'shooting schedule', 'backup', 'OpenFrame AI', 'keyboard shortcuts'];
  var ICON_S = '<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="7"/><path d="M20 20l-3.5-3.5"/></svg>';
  var ICON_C = '<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/></svg>';
  var os = function () { return doc.getAttribute('data-os') === 'mac' ? 'mac' : 'win'; };

  function recents() { try { var r = JSON.parse(localStorage.getItem(KEY) || '[]'); return Array.isArray(r) ? r.filter(function (x) { return typeof x === 'string'; }).slice(0, 6) : []; } catch (e) { return []; } }
  function remember(q) {
    q = (q || '').trim(); if (!q) return;
    try { localStorage.setItem(KEY, JSON.stringify([q].concat(recents().filter(function (x) { return x.toLowerCase() !== q.toLowerCase(); })).slice(0, 6))); } catch (e) {}
  }
  function forget() { try { localStorage.removeItem(KEY); } catch (e) {} }
  function load() {
    if (ix) return Promise.resolve(ix);
    if (!loading) loading = fetch('/search-index.json', { credentials: 'same-origin' }).then(function (r) { if (!r.ok) throw new Error('index'); return r.json(); })
      .then(function (d) { ix = Core.prepare(d); return ix; }).catch(function () { loading = null; return null; });
    return loading;
  }
  function debounce(fn, ms) { var t; return function () { var a = arguments, self = this; clearTimeout(t); t = setTimeout(function () { fn.apply(self, a); }, ms); }; }
  function announce(el) { el.textContent = ''; setTimeout(function () { el.textContent = 'Results updated'; }, 40); }
  function item(e, hl, o) {
    var badge = o.badge && e.os !== 'both' ? '<span class="srch-b">' + (e.os === 'win' ? 'Windows' : 'Mac') + '</span>' : '';
    return '<a class="srch-it"' + (o.id ? ' role="option" tabindex="-1" aria-selected="false" id="' + o.id + '"' : '') + ' href="' + esc(e.url) + '"><span class="srch-t">' + Core.mark(e.title, hl) + '</span><span class="srch-m">' + badge + esc(e.area) + '</span><span class="srch-s">' + Core.snippet(e, hl) + '</span></a>';
  }

  /* ----- the palette ----- */
  var dlg, input, list, msg, live, opener, timer, active = -1, opts = [];
  function build() {
    if (dlg) return;
    dlg = document.createElement('dialog'); dlg.className = 'srch-dlg'; dlg.setAttribute('aria-label', 'Search');
    dlg.innerHTML = '<div class="srch-box"><div class="srch-in">' + ICON_S + '<input id="srch-q" class="srch-q" type="text" role="combobox" aria-expanded="false" aria-controls="srch-list" aria-autocomplete="list" aria-haspopup="listbox" aria-label="Search tutorials, features and answers" placeholder="Search tutorials, features and answers" autocomplete="off" autocapitalize="off" spellcheck="false" enterkeyhint="search"><button type="button" class="srch-x" aria-label="Close search">Esc</button></div>'
      + '<div class="srch-body" tabindex="0"><div class="srch-list" id="srch-list" role="listbox" aria-label="Results"></div><div class="srch-msg" id="srch-msg"></div></div>'
      + '<div class="srch-foot"><span class="srch-hint" aria-hidden="true"><kbd>&uarr;</kbd><kbd>&darr;</kbd> to move <kbd>Enter</kbd> to open</span><a class="srch-adv" href="/search/">Advanced search</a></div><div class="sr" role="status" aria-live="polite" id="srch-live"></div></div>';
    document.body.appendChild(dlg);
    input = dlg.querySelector('#srch-q'); list = dlg.querySelector('#srch-list'); msg = dlg.querySelector('#srch-msg'); live = dlg.querySelector('#srch-live');
    input.addEventListener('input', function () { clearTimeout(timer); timer = setTimeout(function () { update(); }, 90); });
    input.addEventListener('keydown', keys);
    dlg.querySelector('.srch-x').addEventListener('click', function () { dlg.close(); });
    dlg.addEventListener('click', function (e) { if (e.target === dlg) dlg.close(); });
    dlg.addEventListener('close', function () { doc.classList.remove('srch-open'); if (opener && opener.focus) opener.focus(); });
    list.addEventListener('mousemove', function (e) { var o = e.target.closest('[role=option]'); if (o) setActive(opts.indexOf(o), false); });
    list.addEventListener('click', function (e) {
      var o = e.target.closest('[role=option]'); if (!o) return;
      if (o.hasAttribute('data-q')) { e.preventDefault(); input.value = o.getAttribute('data-q'); input.focus(); update(); } else remember(input.value);
    });
    msg.addEventListener('click', function (e) {
      if (e.target.closest('[data-clear]')) { forget(); home(); input.focus(); return; }
      var b = e.target.closest('[data-q]'); if (b) { input.value = b.getAttribute('data-q'); input.focus(); update(); } });
  }
  function open(q) {
    if (document.getElementById('sp-q')) { document.getElementById('sp-q').focus(); return; }
    var tg = document.getElementById('menutg');
    if (tg && tg.getAttribute('aria-expanded') === 'true') tg.click();
    build();
    if (!dlg.open) { opener = document.activeElement; dlg.showModal(); doc.classList.add('srch-open'); }
    if (typeof q === 'string') input.value = q;
    input.focus(); input.select();
    update();
    load().then(function () { if (dlg.open) update(); });
  }
  function setActive(i, scroll) {
    if (opts[active]) { opts[active].setAttribute('aria-selected', 'false'); opts[active].classList.remove('on'); }
    active = i;
    var o = opts[i];
    if (o) { o.setAttribute('aria-selected', 'true'); o.classList.add('on'); input.setAttribute('aria-activedescendant', o.id); if (scroll) o.scrollIntoView({ block: 'nearest' }); } else input.removeAttribute('aria-activedescendant');
  }
  function show(html, first, msgHtml, quiet) {
    list.innerHTML = html; msg.innerHTML = msgHtml || ''; list.setAttribute('data-for', input.value.trim());
    opts = [].slice.call(list.querySelectorAll('[role=option]'));
    input.setAttribute('aria-expanded', String(opts.length > 0));
    active = -1; setActive(first && opts.length ? 0 : -1, false);
    dlg.querySelector('.srch-adv').setAttribute('href', '/search/' + (input.value.trim() ? '?q=' + encodeURIComponent(input.value.trim()) : ''));
    if (!quiet) announce(live);
  }
  function home() {
    var r = recents(), n = 0, h = '';
    if (r.length) h += '<div role="group" aria-label="Recent searches"><div class="srch-gh" aria-hidden="true">Recent</div>' + r.map(function (q) { return '<a class="srch-it srch-q2" role="option" tabindex="-1" aria-selected="false" id="srch-o' + (n++) + '" href="/search/?q=' + encodeURIComponent(q) + '" data-q="' + esc(q) + '">' + ICON_C + '<span class="srch-t">' + esc(q) + '</span></a>'; }).join('') + '</div>';
    h += '<div role="group" aria-label="Try searching for"><div class="srch-gh" aria-hidden="true">Try</div>' + SUGGEST.slice(0, 7 - r.length).map(function (q) { return '<a class="srch-it srch-q2" role="option" tabindex="-1" aria-selected="false" id="srch-o' + (n++) + '" href="/search/?q=' + encodeURIComponent(q) + '" data-q="' + esc(q) + '">' + ICON_S + '<span class="srch-t">' + esc(q) + '</span></a>'; }).join('') + '</div>';
    show(h, false, r.length ? '<button type="button" class="srch-clr" data-clear>Clear recent searches</button>' : '', true);
  }
  function update(quiet) {
    var q = input.value.trim();
    if (!q) return home();
    if (!ix) { show('', false, loading ? '<p class="srch-note">Loading search...</p>' : '<p class="srch-note">Search is not available right now. <a href="/search/?q=' + encodeURIComponent(q) + '">Try the search page</a>.</p>', true); if (!loading) load().then(function () { if (dlg.open) update(); }); return; }
    var f = { os: os(), pref: os() }, r = Core.search(ix, q, f);
    if (!r.hits.length) {
      var s = Core.suggest(ix, q, f), h = '<p class="srch-note">Nothing found for &ldquo;' + esc(q) + '&rdquo;.</p>';
      if (s) h += '<p class="srch-note">Did you mean <button type="button" class="srch-dym" data-q="' + esc(s) + '">' + esc(s) + '</button>?</p>';
      h += '<p class="srch-note"><a href="/search/?q=' + encodeURIComponent(q) + '&amp;os=all">Search both platforms</a></p>';
      return show('', false, h, quiet);
    }
    var n = 0, caps = { tutorial: 6, feature: 3, answer: 3 };
    var html = Core.groups(ix, r.hits, caps).map(function (g) {
      return '<div role="group" aria-label="' + g.label + '"><div class="srch-gh" aria-hidden="true">' + g.label + '</div>' + g.shown.map(function (h) { return item(ix.E[h.i], r.hl, { id: 'srch-o' + (n++) }); }).join('') + '</div>';
    }).join('');
    show(html, true, r.ignored.length ? '<p class="srch-note">No match for ' + esc(r.ignored.join(', ')) + '.</p>' : '', quiet);
  }
  function keys(e) {
    if (e.isComposing) return;
    var k = e.key;
    if (/^(Arrow|Home|End|Enter)/.test(k) && list.getAttribute('data-for') !== input.value.trim()) { clearTimeout(timer); update(true); } // typed a moment ago: results first
    var n = opts.length;
    if (k === 'ArrowDown' || k === 'ArrowUp') {
      e.preventDefault(); if (!n) return;
      setActive(k === 'ArrowDown' ? (active + 1) % n : (active - 1 + n) % n, true);
    } else if ((k === 'Home' || k === 'End') && n && !e.shiftKey && !e.ctrlKey && !e.metaKey) {
      e.preventDefault(); setActive(k === 'Home' ? 0 : n - 1, true);
    } else if (k === 'Enter') {
      e.preventDefault();
      var q = input.value.trim();
      if (e.shiftKey && q) { remember(q); location.href = '/search/?q=' + encodeURIComponent(q); } else if (opts[active]) opts[active].click(); else if (q) { remember(q); location.href = '/search/?q=' + encodeURIComponent(q); }
    }
  }

  /* ----- shortcuts and the header button ----- */
  function editable(t) { return t && (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName)); }
  document.addEventListener('keydown', function (e) {
    if (e.defaultPrevented || e.isComposing) return;
    var k = e.key;
    if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && (k === 'k' || k === 'K')) { e.preventDefault(); if (dlg && dlg.open) input.focus(); else open(); }
    else if (k === '/' && !e.ctrlKey && !e.metaKey && !e.altKey && !editable(e.target) && !(dlg && dlg.open)) { e.preventDefault(); open(); }
  });
  document.addEventListener('click', function (e) {
    var b = e.target.closest('[data-search-open]');
    if (!b || e.button || e.ctrlKey || e.metaKey || e.shiftKey) return;
    e.preventDefault(); open();
  });

  /* ----- the search page ----- */
  function page(root) {
    var $ = function (id) { return document.getElementById(id); };
    var q = $('sp-q'), out = $('sp-out'), live2 = $('sp-live'), part = $('sp-part'), area = $('sp-area'), reset = $('sp-reset'), more = {};
    var S = { q: '', os: 'cur', type: '', part: '', area: '' };
    function read() {
      var p = new URLSearchParams(location.search);
      S.q = p.get('q') || ''; S.os = /^(win|mac|all)$/.test(p.get('os')) ? p.get('os') : 'cur';
      S.type = /^(tutorial|feature|answer)$/.test(p.get('type')) ? p.get('type') : ''; S.part = p.get('part') || ''; S.area = p.get('area') || '';
    }
    function url() {
      var p = new URLSearchParams();
      if (S.q) p.set('q', S.q); if (S.os !== 'cur') p.set('os', S.os); if (S.area) p.set('area', S.area); if (S.type) p.set('type', S.type); if (S.part) p.set('part', S.part);
      var s = p.toString(); return '/search/' + (s ? '?' + s : '');
    }
    function save(push) {
      var u = url(); if (u === location.pathname + location.search) return;
      try { history[push ? 'pushState' : 'replaceState'](null, '', u); } catch (e) {}
      title();
    }
    function title() { document.title = (S.q ? S.q + ' | ' : '') + 'Search | OpenFrame Studio'; }
    function filt() { return { os: S.os === 'cur' ? os() : S.os, pref: os(), type: S.type, part: S.part, area: S.area }; }
    function controls() {
      root.querySelectorAll('[data-f]').forEach(function (b) { b.setAttribute('aria-pressed', String((b.getAttribute('data-f') === 'os' ? S.os : S.type) === b.getAttribute('data-v'))); });
      if (!ix) return;
      var f = filt(), parts = [], areas = [], seen = {}, seenA = {};
      ix.E.forEach(function (e) {
        if ((f.os === 'win' || f.os === 'mac') && e.os !== f.os && e.os !== 'both') return;
        if (S.type && e.type !== S.type) return;
        if (e.part && !seen[e.part]) { seen[e.part] = 1; parts.push(e.part); }
        if ((!S.part || e.part === S.part) && !seenA[e.area]) { seenA[e.area] = 1; areas.push(e.area); }
      });
      var label = {}; ix.parts.forEach(function (p) { label[p[0]] = p[1]; });
      if (S.part && !seen[S.part]) S.part = ''; if (S.area && !seenA[S.area]) S.area = '';
      part.innerHTML = '<option value="">All parts</option>' + parts.map(function (p) { return '<option value="' + esc(p) + '">' + esc(label[p] || p) + '</option>'; }).join('');
      area.innerHTML = '<option value="">All areas</option>' + areas.map(function (a) { return '<option value="' + esc(a) + '">' + esc(a) + '</option>'; }).join('');
      part.value = S.part; area.value = S.area;
      reset.hidden = !(S.os !== 'cur' || S.type || S.part || S.area);
    }
    function render(quiet) {
      var query = S.q.trim();
      if (!ix) { out.innerHTML = '<p class="sp-note">' + (loading ? 'Loading search...' : 'Search is not available right now. Please reload the page.') + '</p>'; return; }
      controls();
      var f = filt(), browse = !query && (S.type || S.part || S.area);
      if (!query && !browse) {
        var r0 = recents(), h = '<div class="sp-home">' + (r0.length ? '<h2>Recent searches</h2><p class="sp-chips">' + r0.map(function (x) { return '<a class="fchip" href="/search/?q=' + encodeURIComponent(x) + '">' + esc(x) + '</a>'; }).join('') + ' <button type="button" class="sp-link" data-clear>Clear</button></p>' : '') + '<h2>Try searching for</h2><p class="sp-chips">' + SUGGEST.map(function (x) { return '<a class="fchip" href="/search/?q=' + encodeURIComponent(x) + '">' + esc(x) + '</a>'; }).join('') + '</p></div>';
        out.innerHTML = h; if (!quiet) announce(live2); return;
      }
      var r = Core.search(ix, query, f);
      if (!r.hits.length) {
        var s = query ? Core.suggest(ix, query, f) : '', h2 = '<div class="sp-none"><h2>Nothing found' + (query ? ' for &ldquo;' + esc(query) + '&rdquo;' : '') + '.</h2>';
        if (s) h2 += '<p>Did you mean <button type="button" class="sp-link" data-q="' + esc(s) + '">' + esc(s) + '</button>?</p>';
        if (S.os !== 'all' || S.type || S.part || S.area) h2 += '<p><button type="button" class="sp-link" data-all>Search both platforms with no filters</button></p>';
        else h2 += '<p>Check the spelling, or try fewer words.</p>';
        out.innerHTML = h2 + '</div>'; if (!quiet) announce(live2); return;
      }
      out.innerHTML = (r.ignored.length ? '<p class="sp-note">No match for ' + esc(r.ignored.join(', ')) + '.</p>' : '') + Core.groups(ix, r.hits, {}).map(function (g) {
        var cap = more[g.type] || 20;
        return '<section class="sp-g" aria-labelledby="spg-' + g.type + '"><h2 id="spg-' + g.type + '">' + g.label + '</h2><div class="sp-list">' + g.hits.slice(0, cap).map(function (h) { return item(ix.E[h.i], r.hl, { badge: S.os === 'all' }); }).join('') + '</div>'
          + (g.hits.length > cap ? '<button type="button" class="btn btn-ghost sp-more" data-more="' + g.type + '">Show more ' + g.label.toLowerCase() + '</button>' : '') + '</section>';
      }).join('');
      if (!quiet) announce(live2);
    }
    function go(push, quiet) { more = {}; save(push); render(quiet); }
    var typed = debounce(function () { S.q = q.value; go(false); }, 120);
    q.addEventListener('input', typed);
    q.addEventListener('keydown', function (e) {
      if (e.key === 'Enter') { e.preventDefault(); S.q = q.value; remember(S.q); go(true); }
      else if (e.key === 'ArrowDown') { var a = out.querySelector('.srch-it'); if (a) { e.preventDefault(); a.focus(); } }
    });
    out.addEventListener('keydown', function (e) {
      if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return;
      var all = [].slice.call(out.querySelectorAll('.srch-it')), i = all.indexOf(document.activeElement);
      if (i < 0) return; e.preventDefault();
      if (e.key === 'ArrowUp' && i === 0) q.focus(); else (all[i + (e.key === 'ArrowDown' ? 1 : -1)] || all[i]).focus();
    });
    out.addEventListener('click', function (e) {
      var t = e.target.closest('a.srch-it'); if (t) { remember(S.q); return; }
      var b = e.target.closest('button'); if (!b) return;
      if (b.hasAttribute('data-more')) { var t2 = b.getAttribute('data-more'), c = more[t2] || 20; more[t2] = c + 20; render(true); var n = out.querySelectorAll('#spg-' + t2 + ' ~ .sp-list .srch-it')[c]; if (n) n.focus(); }
      else if (b.hasAttribute('data-q')) { S.q = b.getAttribute('data-q'); q.value = S.q; go(true); }
      else if (b.hasAttribute('data-all')) { S.os = 'all'; S.type = S.part = S.area = ''; go(true); }
      else if (b.hasAttribute('data-clear')) { forget(); render(true); }
    });
    root.addEventListener('click', function (e) {
      var b = e.target.closest('[data-f]'); if (!b) return;
      S[b.getAttribute('data-f')] = b.getAttribute('data-v'); S.area = ''; go(true);
    });
    part.addEventListener('change', function () { S.part = part.value; S.area = ''; go(true); });
    area.addEventListener('change', function () { S.area = area.value; go(true); });
    reset.addEventListener('click', function () { S.os = 'cur'; S.type = S.part = S.area = ''; go(true); });
    addEventListener('popstate', function () { read(); q.value = S.q; title(); render(true); });
    new MutationObserver(function () { if (S.os === 'cur') render(true); }).observe(doc, { attributes: true, attributeFilter: ['data-os'] });
    read(); q.value = S.q; title(); render(true); q.focus();
    load().then(function () { render(); });
  }

  /* ----- the filter box on the tutorial lists ----- */
  function filterBox(box) {
    var input = box.querySelector('input'), cards = [].slice.call(document.querySelectorAll('.tut')), none = document.querySelector('.tf-none');
    var info = cards.map(function (c) { return c.textContent; });
    function apply() {
      var v = input.value.trim(), shown = 0;
      cards.forEach(function (c, i) { var on = !v || Core.matchText(v, info[i]); c.classList.toggle('tf-off', !on); if (on) shown++; });
      document.querySelectorAll('.tgrid').forEach(function (g) {
        var vis = !!g.querySelector('.tut:not(.tf-off)'); g.classList.toggle('tf-off', !vis);
        var h = g.previousElementSibling; if (h && h.tagName === 'H3') h.classList.toggle('tf-off', !vis);
      });
      document.querySelectorAll('.tut-part').forEach(function (p) { p.classList.toggle('tf-off', !p.querySelector('.tut:not(.tf-off)')); });
      document.querySelectorAll('.fchip[href^="#"]').forEach(function (c) { var t = document.querySelector(c.getAttribute('href')); c.classList.toggle('tf-off', !!t && t.classList.contains('tf-off')); });
      document.querySelectorAll('[data-tf-adv]').forEach(function (a) { a.setAttribute('href', '/search/' + (v ? '?q=' + encodeURIComponent(v) : '')); });
      if (none) none.hidden = !!shown || !v;
    }
    input.addEventListener('input', debounce(apply, 60));
    input.addEventListener('keydown', function (e) { if (e.key === 'Escape' && input.value) { e.preventDefault(); input.value = ''; apply(); } });
  }

  var sp = document.getElementById('sp'); if (sp) page(sp);
  var tf = document.querySelector('[data-tfilter]'); if (tf) filterBox(tf);
}

if (typeof module === 'object' && module && module.exports) module.exports = Core;
else if (typeof document !== 'undefined') ui(Core);
