#!/usr/bin/env python3
"""Generator for the Beat Board design references (Outline + Card Board + stress tests).

Run:  python build.py            (builds every HTML + PDF into exports/story-beat-board/)
      python build.py outline    (only the Outline family)  | board

The HTML is produced from ONE small document model so that the PDF and the HTML are the same
content. Pagination-dependent "continued" strips are inserted by a probe/render loop (see
`paginate`): the probe HTML carries invisible sentinels so the page start of every block can be
read from the PDF; the final HTML/PDF carries no sentinels.
"""
import copy
import os
import html as H
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(ROOT / "tools"))
import pymupdf  # noqa: E402
import render_pdf  # noqa: E402
from data import *  # noqa: E402,F401,F403

OUT = ROOT / "exports" / "story-beat-board"
# Card Board v2: PORTRAIT. Cards per row (rule in the spec, section E). BB_COLS is only used for the 3-vs-4 column evidence runs.
COLS = int(os.environ.get("BB_COLS", "3"))
GAP = {3: 8, 4: 6}[COLS]
EXPORT_DATE = "22 Oct 2026"          # as in the supplied PDFs ("Exported 22 Oct 2026")
SCRATCH = pathlib.Path(r"C:/Users/avuku/AppData/Local/Temp/claude/d--web-development-projects-OpenFrame-export-changes/7f45d7c4-514c-4b3e-a651-c0d0a68695ea/scratchpad/bb")


def esc(s):
    return H.escape(s or "", quote=False)


def plural(n, one, many):
    return f"{n} {one if n == 1 else many}"


# ------------------------------------------------------------------------------------------
# counting
# ------------------------------------------------------------------------------------------

def count(nodes, c=None):
    c = c if c is not None else dict(acts=0, seqs=0, cards=0, beats=0)
    for n in nodes:
        k = {"act": "acts", "seq": "seqs", "card": "cards", "beat": "beats"}[n.kind]
        c[k] += 1
        count(n.children, c)
    return c


def hint_for(n):
    c = count(n.children)
    parts = []
    if c["seqs"]:
        parts.append(plural(c["seqs"], "sequence", "sequences"))
    if c["cards"]:
        parts.append(plural(c["cards"], "scene card", "scene cards"))
    if c["beats"]:
        parts.append(plural(c["beats"], "beat", "beats"))
    return " · ".join(parts)


# ------------------------------------------------------------------------------------------
# renderer of the content blocks
# ------------------------------------------------------------------------------------------

STATUS_CHIP = {"Open": "warn", "In Discussion": "info", "Resolved": "ok"}


class R:
    def __init__(self, layout, notes=True, comments=True, parking=True, probe=False, cont=None, strips=True, pins=()):
        self.layout = layout
        self.on = {"Notes": notes, "Comments": comments, "Parking Lot": parking}
        self.probe = probe
        self.cont = cont or {}
        self.pins = set(pins)
        self.strips = strips
        self.atoms = []          # chain per atom id
        self._prefix = ""

    # -- helpers
    def t(self, label):
        return f' data-toggle="{label}"' + ("" if self.on[label] else " hidden")

    def atom(self, chain):
        self.atoms.append(list(chain))
        return len(self.atoms) - 1

    def sx(self, aid):
        return f'<i class="sx" aria-hidden="true">@@{aid}@@</i>' if self.probe else ""

    def has_strip(self, aid):
        return bool(self.cont.get(aid)) and self.strips

    def pb(self, aid):
        """forced page break (pagination pin) on an atom that starts a page but has no continued strip"""
        return " pb" if (aid in self.pins and not self.has_strip(aid)) else ""

    def strip(self, aid, tag="div"):
        if not self.has_strip(aid):
            return ""
        chain = self.cont[aid]
        label = " › ".join(esc(c) for c in chain)
        pb = " pb" if aid in self.pins else ""
        return f'<{tag} class="cont{pb}" data-cont aria-hidden="true">{label} <i>· continued</i></{tag}>'

    # -- notes / comments
    def extras(self, n):
        h = ""
        if n.note:
            h += (f'<p class="xnote"{self.t("Notes")}><b class="lbl">Note</b>'
                  f'<span class="txt">{esc(n.note)}</span></p>')
        for c in n.comments:
            h += self.comment(c)
        return h

    def comment(self, c):
        chip = STATUS_CHIP.get(c.status, "")
        h = (f'<div class="xcomment"{self.t("Comments")}><div class="xc-head"><b class="lbl">Comment</b>'
             f'<span class="chip {chip}">{esc(c.status)}</span><b class="who">{esc(c.author)}</b></div>'
             f'<p class="xc-body">{esc(c.body)}</p>')
        for who, body in c.replies:
            h += (f'<div class="xreply"><div class="xc-head"><b class="lbl">Reply</b><b class="who">{esc(who)}</b></div>'
                  f'<p class="xc-body">{esc(body)}</p></div>')
        return h + "</div>"

    # -- outline items
    def item(self, n, chain):
        aid = self.atom(chain)
        strip = self.strip(aid, "li")
        if n.kind == "card":
            tile = f'<span class="t-card">{n.number}</span>' if n.number else '<span class="t-ghost">Card</span>'
            body = ""
            if n.title:
                body += f'<p class="ihead">{esc(n.title)}</p>'
            if n.text:
                body += f'<p class="idesc">{esc(n.text)}</p>'
            if not n.title and not n.text:
                body += '<p class="idesc muted">No heading or description</p>'
            cls = "item scene"
        else:
            tile = '<span class="t-beat">Beat</span>'
            body = f'<p class="ibeat">{esc(n.title)}</p>'
            cls = "item beat"
        body += self.extras(n)
        return f'{strip}<li class="{cls}{self.pb(aid)}">{self.sx(aid)}<div class="tc">{tile}</div><div class="ib">{body}</div></li>'

    # -- board cards
    def bcard(self, n):
        if n.kind == "card":
            head = (f'<span class="t-card sm">{n.number}</span>' if n.number else "") + "<span>Scene Card</span>"
            body = ""
            if n.title:
                body += f'<p class="ihead">{esc(n.title)}</p>'
            if n.text:
                body += f'<p class="idesc">{esc(n.text)}</p>'
            if not n.title and not n.text:
                body += '<p class="idesc muted">No heading or description</p>'
            cls = "bcard"
        else:
            head = "<span>Beat</span>"
            body = f'<p class="ibeat">{esc(n.title)}</p>'
            cls = "bcard beat"
        body += self.extras(n)
        return f'<article class="{cls}"><header class="bh">{head}</header><div class="bb">{body}</div></article>'

    def rows(self, nodes, chain):
        out = []
        for i in range(0, len(nodes), COLS):
            aid = self.atom(chain)
            out.append(self.strip(aid) + f'<div class="crow{self.pb(aid)}">{self.sx(aid)}'
                       + "".join(self.bcard(n) for n in nodes[i:i + COLS]) + "</div>")
        return out

    # -- containers. A "lead" is a heading plus the first block that follows it; it never splits across pages.
    #    Board layout: wrapped in div.lead (break-inside: avoid). Outline: siblings chained with break-after/before: avoid.
    def wrap(self, html):
        return f'<div class="lead">{html}</div>'

    def leaf_blocks(self, nodes, chain, cls):
        if self.layout == "outline":
            its = [self.item(n, chain) for n in nodes]
            blocks = [f'<ol class="items {cls} first">{its[0]}</ol>']
            if len(its) > 1:
                blocks.append(f'<ol class="items {cls}">' + "".join(its[1:]) + "</ol>")
            return blocks
        return self.rows(nodes, chain)

    def lead_rest(self, head_html, blocks):
        """-> (lead_html, rest_html): heading + first block (board: wrapped) and the remaining blocks"""
        if blocks:
            return self.wrap(head_html + blocks[0]), "".join(blocks[1:])
        return head_html, ""

    def sequence(self, s, chain):
        aid = self.atom(chain)
        sc = chain + [f"Sequence: {s.title}"]
        hint = hint_for(s)
        head = self.strip(aid)
        head += (f'<header class="seq-head{self.pb(aid)}">{self.sx(aid)}<div class="seq-row"><h3><span class="k">Sequence</span>'
                 f'<span class="t">{esc(s.title)}</span></h3>' + (f'<span class="hint">{hint}</span>' if hint else "") + "</div>"
                 f"{self.extras(s)}</header>")
        if s.children:
            blocks = self.leaf_blocks(s.children, sc, "seq-items")
        else:
            blocks = ['<p class="empty-state seq-empty">No beats or scene cards yet.</p>']
        return self.lead_rest(head, blocks)

    def flow(self, nodes, chain, label_loose):
        """Children of an act / Unassigned area -> list of (lead, rest): sequences, and runs of loose beats/cards (data order)."""
        has_seq = any(n.kind == "seq" for n in nodes)
        pairs, run = [], []

        def flush():
            nonlocal run
            if run:
                head = ""
                if has_seq and label_loose:
                    lid = self.atom(chain)
                    head = self.strip(lid) + f'<p class="loose-label{self.pb(lid)}">{self.sx(lid)}Not in a sequence</p>'
                pairs.append(self.lead_rest(head, self.leaf_blocks(run, chain, "loose-items")))
                run = []

        for n in nodes:
            if n.kind == "seq":
                flush()
                pairs.append(self.sequence(n, chain))
            else:
                run.append(n)
        flush()
        return pairs

    def container(self, head_html, pairs, empty_html=""):
        """head + first (lead, rest) pair kept together; the rest of the pairs flow normally"""
        head_html = self._prefix + head_html
        self._prefix = ""
        if pairs:
            lead = head_html + pairs[0][0]
            return self.wrap(lead) + pairs[0][1] + "".join(l + r for l, r in pairs[1:])
        return self.wrap(head_html + empty_html)

    def act(self, a, chain):
        aid = self.atom(chain)
        h = self.strip(aid)
        hint = hint_for(a)
        head = (f'<h2 class="act-bar{self.pb(aid)}">{self.sx(aid)}<span class="t">{esc(a.title)}</span>'
                + (f'<span class="hint">{hint}</span>' if hint else "") + "</h2>")
        ex = self.extras(a)
        if ex:
            head += f'<div class="act-extra">{ex}</div>'
        pairs = self.flow(a.children, chain + [a.title], True)
        return h + '<section class="act">' + self.container(head, pairs, '<p class="empty-state">No sequences, beats or scene cards yet.</p>') + "</section>"

    def area(self, kind, label, sub, nodes, chain):
        aid = self.atom(chain)
        tog = self.t("Parking Lot") if kind == "parking" else ""
        h = self.strip(aid)
        head = (f'<h2 class="area-bar {kind}{self.pb(aid)}">{self.sx(aid)}<span>{label}</span>'
                f'<span class="hint">{sub}</span></h2>')
        pairs = self.flow(nodes, chain + [label], kind == "unassigned")
        return h + f'<section class="area {kind}"{tog}>' + self.container(head, pairs) + "</section>"

    def board(self, b, multi, index):
        h = ""
        chain = []
        if multi:
            title = b.episode or "Film Beat Board"
            eid = self.atom([])
            eyebrow = '<span class="eb">Episode</span>' if b.episode else ""
            brk = " page-break-before" if (self.layout == "board" and index > 0) else ""
            self._prefix = f'<h2 class="ep-bar{brk}{self.pb(eid)}">{self.sx(eid)}{eyebrow}<span class="t">{esc(title)}</span></h2>'
            chain = [title]
        for a in b.acts:
            h += self.act(a, chain)
        if b.unassigned:
            h += self.area("unassigned", "Unassigned", "Not placed in an act", b.unassigned, chain)
        if b.parking:
            h += self.area("parking", "Parking Lot", "Held outside the board order · not numbered", b.parking, chain)
        return h


# ------------------------------------------------------------------------------------------
# document chrome
# ------------------------------------------------------------------------------------------

def metrics_html(boards, parking_on):
    on = dict(acts=0, seqs=0, cards=0, beats=0)
    pk = dict(acts=0, seqs=0, cards=0, beats=0)
    for b in boards:
        count(b.acts, on)
        count(b.unassigned, on)
        count(b.parking, pk)
    tiles = []
    for key, label in (("acts", "Acts"), ("seqs", "Sequences"), ("cards", "Scene cards"), ("beats", "Beats")):
        total = on[key]
        if total == 0 and pk[key] == 0:
            continue
        sub = ""
        if pk[key]:
            sub = f'<div class="sub" data-toggle="Parking Lot"{"" if parking_on else " hidden"}>+ {pk[key]} in Parking Lot</div>'
        tiles.append(f'<div class="metric"><div class="lbl">{label}</div><div class="val">{total}</div>{sub}</div>')
    return '<div class="metrics">' + "".join(tiles) + "</div>"


def keybox(r: R, boards):
    items = [
        '<span class="it"><span class="t-card sm">12</span> Scene Card, in board order (not the screenplay scene number)</span>',
        '<span class="it"><span class="t-beat sm">Beat</span> Story beat</span>',
        f'<span class="it"{r.t("Notes")}><b class="lbl">Note</b> the item\'s own note</span>',
        f'<span class="it"{r.t("Comments")}><span class="chip warn">Open</span> comment thread (status, author, replies)</span>',
    ]
    has_parking = any(b.parking for b in boards)
    inc = (f'<span class="chip"{r.t("Notes")}>Notes</span><span class="chip"{r.t("Comments")}>Comments</span>'
           f'<span class="chip"{r.t("Parking Lot")}>Parking Lot{"" if has_parking else " (none)"}</span>')
    anyon = any(r.on.values())
    lead = "Includes" if anyon else "Board only"
    return (f'<aside class="keybox"><div class="kb-row"><span class="kb-h">Key</span><div class="kb-items">{"".join(items)}</div></div>'
            f'<div class="kb-row"><span class="kb-h">{lead}</span><div class="kb-chips">{inc}</div><span class="priv">Private notes excluded.</span></div></aside>')


def body_html(r: R, boards, multi):
    h = metrics_html(boards, r.on["Parking Lot"]) + keybox(r, boards)
    h += '<main class="content">'
    for i, b in enumerate(boards):
        h += r.board(b, multi, i)
    return h + "</main>"


def masthead(layout, project, source_label, scope_label, scope_note=None, panels_extra=""):
    doc = "Beat Board" if layout == "board" else "Beat Board Outline"
    eyebrow = "Story · Beat Board" if layout == "board" else "Story · Outline"
    note = f"<span>{esc(scope_note)}</span>" if scope_note else ""
    return (f'<header class="masthead"><div><div class="eyebrow">{eyebrow}</div><h1>{doc}</h1>'
            f'<div class="proj">{esc(project)}</div><div class="meta-line">{esc(source_label)}</div></div>'
            f'<aside class="scope-panel" data-scope-panel="main"><b>{esc(scope_label)}</b><span>Exported {EXPORT_DATE}</span>{note}</aside>'
            f'{panels_extra}</header>')


CSS = r"""
  /* ===== Beat Board — document-specific CSS (shared tokens/components come from openframe-print.css) ===== */
  .masthead .proj { font-size: 13pt; color: var(--of-slate); margin-top: 1pt; }
  .masthead .meta-line { font-size: 8.4pt; }
  .scope-panel { background: var(--of-panel); border: var(--hair) solid var(--of-line); border-radius: var(--radius-lg);
    padding: 8pt 12pt 9pt; min-width: 132pt; max-width: 176pt; text-align: right; }
  .scope-panel b { display: block; font-size: 9.6pt; color: var(--of-navy); }
  .scope-panel span { display: block; font-size: 8.2pt; color: var(--of-slate); margin-top: 2pt; }
  .masthead + .metrics { margin-top: 6pt; }
  .metrics { margin: 6pt 0 5pt; }
  .metric { padding: 4pt 8pt 5pt; }
  .metric .val { font-size: 13pt; margin-top: 1pt; }
  .metric .sub { font-size: 7pt; }

  /* key + includes */
  .keybox { border: var(--hair) solid var(--of-line); border-radius: var(--radius); padding: 5pt 8pt; margin: 0 0 4pt;
    font-size: 7.6pt; color: var(--of-slate); break-inside: avoid; }
  .kb-row { display: flex; flex-wrap: wrap; align-items: center; gap: 3pt 12pt; padding: 2pt 0; }
  .kb-row + .kb-row { border-top: var(--hair) solid var(--of-line); }
  .kb-h { font-size: 6.6pt; font-weight: 700; letter-spacing: .12em; text-transform: uppercase; color: var(--of-navy); min-width: 44pt; flex: none; }
  .kb-row .it { display: inline-flex; align-items: center; gap: 4pt; }
  .kb-row .lbl { font-size: 6.6pt; font-weight: 700; letter-spacing: .09em; text-transform: uppercase; color: var(--of-slate); }
  .kb-items { display: flex; flex-wrap: wrap; gap: 3pt 14pt; flex: 1; min-width: 0; }
  .kb-chips { display: flex; flex-wrap: wrap; gap: 3pt 4pt; flex: 1; min-width: 0; align-items: center; }
  .kb-row .priv { margin-left: auto; }

  /* tiles */
  .t-card { display: inline-block; min-width: 26pt; text-align: center; background: var(--of-navy); color: #fff; border-radius: 3pt;
    font-weight: 700; font-size: 8.4pt; line-height: 1; padding: 3.6pt 3pt 3.8pt; font-variant-numeric: tabular-nums; }
  .t-card.sm { min-width: 20pt; font-size: 7.6pt; padding: 2.8pt 3pt 3pt; }
  .t-ghost, .t-beat { display: inline-block; min-width: 26pt; text-align: center; border-radius: 3pt; font-weight: 700; font-size: 6.6pt;
    letter-spacing: .08em; text-transform: uppercase; line-height: 1; padding: 4.2pt 2pt 4.4pt; }
  .t-ghost { background: #fff; color: var(--of-navy); border: var(--hair) solid var(--of-navy); }
  .t-beat { background: #fff; color: var(--of-orange-deep); border: var(--hair) solid var(--of-orange-line); }
  .t-beat.sm { min-width: 20pt; }

  /* notes + comments: a NOTE is plain (the item's own field); a COMMENT is a boxed thread with status + author */
  .xnote { display: grid; grid-template-columns: 24pt minmax(0, 1fr); column-gap: 5pt; margin-top: 2.6pt; font-size: 8pt; line-height: 1.3;
    color: var(--of-slate); border-left: 1.5pt solid var(--of-line-strong); padding-left: 5pt; }
  .xnote .lbl, .xc-head .lbl { font-size: 6.6pt; font-weight: 700; letter-spacing: .09em; text-transform: uppercase; color: var(--of-slate); padding-top: 1.2pt; }
  .xnote .txt, .xc-body { white-space: pre-line; }
  .xcomment { margin-top: 3.2pt; background: #fff; border: var(--hair) solid var(--of-line-strong); border-left: 2.5pt solid var(--of-teal);
    border-radius: 3pt; padding: 3.6pt 6pt 4.2pt; break-inside: avoid; }
  .xc-head { display: flex; flex-wrap: wrap; align-items: center; gap: 2pt 5pt; margin-bottom: 1.6pt; }
  .xc-head .who { font-size: 7.8pt; color: var(--of-navy); }
  .xc-body { font-size: 8pt; line-height: 1.3; color: var(--of-ink); }
  .xreply { margin-top: 3.4pt; padding-left: 7pt; border-left: var(--hair) solid var(--of-line-strong); }
  .ihead { font-size: 8.8pt; font-weight: 700; color: var(--of-navy); line-height: 1.25; }
  .idesc { font-size: 8.8pt; line-height: 1.3; color: var(--of-ink); }
  .ihead + .idesc { margin-top: 1pt; }
  .idesc.muted { color: var(--of-muted); }
  .ibeat { font-size: 9pt; font-weight: 700; color: var(--of-navy); line-height: 1.28; }

  /* act / sequence / area headings (identical in both layouts) */
  .act-bar { display: flex; justify-content: space-between; align-items: baseline; gap: 12pt; background: var(--of-navy); color: #fff;
    border-radius: 3pt; padding: 5.5pt 9pt 6pt; margin: 14pt 0 4pt; font-size: 10.5pt; font-weight: 700; letter-spacing: .06em;
    text-transform: uppercase; line-height: 1.2; break-after: avoid; break-inside: avoid; }
  .content > .act:first-child .act-bar { margin-top: 8pt; }
  .act-bar .t { min-width: 0; }
  .act-bar .hint, .seq-row .hint, .area-bar .hint { flex: none; text-align: right; }
  .act-bar .hint { font-size: 7.4pt; font-weight: 400; letter-spacing: 0; text-transform: none; color: var(--of-dark-muted); max-width: 40%; }
  .act-extra { break-after: avoid; margin: 0 0 3pt; }
  .act-extra .xnote:first-child, .act-extra .xcomment:first-child { margin-top: 1pt; }
  .seq-head { background: var(--of-tint); border-left: 3pt solid var(--of-orange); border-radius: 0 3pt 3pt 0; padding: 4.6pt 8pt 5pt;
    margin: 9pt 0 3pt; break-after: avoid; break-inside: avoid; }
  .seq-row { display: flex; justify-content: space-between; align-items: baseline; gap: 12pt; }
  .seq-row h3 { margin: 0; min-width: 0; font-size: 9.8pt; line-height: 1.22; }
  .seq-row .k { font-size: 6.6pt; font-weight: 700; letter-spacing: .12em; text-transform: uppercase; color: var(--of-slate); margin-right: 7pt; }
  .seq-row .t { font-weight: 700; color: var(--of-navy); }
  .seq-row .hint { font-size: 7.4pt; color: var(--of-slate); max-width: 40%; }
  .seq-head .xcomment { margin-top: 3pt; }
  .loose-label { display: flex; align-items: center; gap: 7pt; margin: 9pt 0 2pt; font-size: 6.6pt; font-weight: 700; letter-spacing: .12em;
    text-transform: uppercase; color: var(--of-slate); break-after: avoid; }
  .loose-label::after { content: ""; flex: 1; border-top: var(--hair) solid var(--of-line-strong); }
  .empty-state { padding: 6pt 0 6pt 3pt; font-size: 8.4pt; }
  .empty-state.seq-empty { margin-left: 10pt; }
  .area-bar { display: flex; justify-content: space-between; align-items: baseline; gap: 12pt; border-radius: 3pt; padding: 6pt 9pt 6.4pt;
    margin: 22pt 0 5pt; font-size: 10.5pt; font-weight: 700; letter-spacing: .06em; text-transform: uppercase; line-height: 1.2;
    break-after: avoid; break-inside: avoid; }
  .area-bar .hint { font-size: 7.4pt; font-weight: 400; letter-spacing: 0; text-transform: none; }
  .area-bar.parking { background: var(--of-orange-soft); border: 1pt solid var(--of-orange-line); border-left: 5pt solid var(--of-orange); color: var(--of-orange-deep); }
  .area-bar.unassigned { background: #fff; border: 1.2pt solid var(--of-navy); color: var(--of-navy); }
  .area-bar.unassigned .hint { color: var(--of-slate); }
  .ep-bar { margin: 20pt 0 6pt; padding: 7pt 0 6pt; border-top: 3pt solid var(--of-orange); border-bottom: var(--rule) solid var(--of-navy);
    font-size: 15pt; line-height: 1.15; color: var(--of-navy); break-after: avoid; break-inside: avoid; }
  .ep-bar .eb { display: block; font-size: 6.8pt; font-weight: 700; letter-spacing: .14em; text-transform: uppercase; color: var(--of-slate); margin-bottom: 2pt; }
  .ep-bar.page-break-before { margin-top: 0; }
  .content > .lead:first-child .ep-bar, .content > .act:first-child .ep-bar { margin-top: 10pt; }
  .lead > .ep-bar:first-child { margin-top: 18pt; }
  .content > .act:first-child .lead > .ep-bar:first-child { margin-top: 10pt; }
  .cont { font-size: 6.8pt; font-weight: 700; letter-spacing: .08em; text-transform: uppercase; color: var(--of-slate); border-bottom: var(--hair) solid var(--of-line-strong);
    padding: 0 0 3pt; margin: 0 0 6pt; break-after: avoid; break-inside: avoid; list-style: none; }
  .cont i { font-style: normal; font-weight: 400; letter-spacing: .03em; text-transform: none; }
  .sx { position: absolute; left: 0; top: 0; font-size: 2pt; line-height: 1; color: #fff; font-style: normal; letter-spacing: 0; text-transform: none; font-weight: 400; }
  .act-bar, .seq-head, .area-bar, .item, .crow, .loose-label, .ep-bar { position: relative; }
  .act, .area { break-inside: auto; }
  .pb:not(.pb-off) { break-before: page !important; }   /* pagination pin: page start of the captured state (see MD, section I) */

  /* variants / preview only */
  @media print { [data-body][hidden], [data-scope-panel][hidden] { display: none !important; } }
  .illus { display: inline-block; margin-left: 6pt; font-size: 6.6pt; font-weight: 700; letter-spacing: .08em; text-transform: uppercase; color: var(--of-amber-text);
    background: var(--of-amber-fill); border: var(--hair) solid var(--of-amber-line); border-radius: 3pt; padding: 2pt 4pt; vertical-align: middle; }
"""

CSS_OUTLINE = r"""
  /* ===== OUTLINE layout ===== */
  .items { list-style: none; margin: 0; padding: 0; }
  .items > li:first-child { break-before: avoid; }
  .item { display: grid; grid-template-columns: 28pt minmax(0, 1fr); column-gap: 8pt; padding: 5pt 0 5.6pt; border-bottom: var(--hair) solid var(--of-line); break-inside: avoid; align-items: start; }
  .item .tc { padding-top: .6pt; }
  .item.beat { background: var(--of-orange-soft); border: var(--hair) solid var(--of-orange-line); border-radius: 3pt; padding: 5pt 6pt 5.6pt; margin: 3pt 0; }
  .item.beat .xnote { border-left-color: var(--of-orange-line); }
  .seq-items.first { margin-bottom: 0; }
  .seq-items { margin: 0 0 4pt 1pt; padding-left: 9pt; border-left: 1.2pt solid var(--of-line-strong); }
  .loose-items { margin: 0 0 2pt; }
"""

CSS_BOARD = r"""
  /* ===== CARD BOARD layout ===== */
  .crow { display: grid; grid-template-columns: repeat(__COLS__, minmax(0, 1fr)); gap: __GAP__pt; margin: 0 0 __GAP__pt; align-items: stretch; break-inside: avoid; }
  .lead { break-inside: avoid; }
  .seq-head + .crow, .loose-label + .crow { margin-top: 6pt; }
  .bcard { display: flex; flex-direction: column; border: var(--hair) solid var(--of-line-strong); border-radius: 6pt; background: #fff; overflow: hidden; min-width: 0; }
  .bh { display: flex; align-items: center; gap: 5pt; background: var(--of-tint); border-bottom: var(--hair) solid var(--of-line-strong);
    padding: 3.2pt 6pt; font-size: 6.8pt; font-weight: 700; letter-spacing: .1em; text-transform: uppercase; color: var(--of-navy); }
  .bb { padding: 4.6pt 6.4pt 5.4pt; flex: 1; }
  .bcard .ihead, .bcard .idesc { font-size: 8.4pt; }
  .bcard .ibeat { font-size: 8.6pt; }
  .bcard .xnote { display: block; font-size: 7.6pt; margin-top: 3.4pt; }
  .bcard .xnote .lbl { margin-right: 4pt; }
  .bcard .xcomment { margin-top: 3.6pt; }
  .bcard .xc-body { font-size: 7.6pt; }
  .bcard.beat { background: var(--of-orange-soft); border-color: var(--of-orange-line); }
  .bcard.beat .bh { background: #ffe3c2; border-bottom-color: var(--of-orange-line); color: var(--of-orange-deep); }
  .bcard.beat .xnote { border-left-color: var(--of-orange-line); }
  .area.parking .bcard:not(.beat) { border-color: var(--of-orange-line); }
  .area.parking .bcard:not(.beat) .bh { background: var(--of-orange-soft); border-bottom-color: var(--of-orange-line); }
"""

JS_VARIANTS = r"""
(function () {
  var bar = document.querySelector(".preview-bar");
  if (!bar) return;
  var boxes = [].slice.call(bar.querySelectorAll("input[type=checkbox]"));
  var init = boxes.map(function (b) { return b.checked; });
  var conts = [].slice.call(document.querySelectorAll("[data-cont]"));
  function sync() { // pagination-dependent "continued" strips are only valid for the captured state
    var same = boxes.every(function (b, i) { return b.checked === init[i]; });
    conts.forEach(function (c) { c.hidden = !same; });
    [].forEach.call(document.querySelectorAll(".pb"), function (e) { e.classList.toggle("pb-off", !same); });
  }
  var VAR = { "Scope: Selected items only (example)": "selected", "Scope: Entire project, several boards (fictional example)": "project" };
  var vboxes = {};
  [].forEach.call(bar.querySelectorAll("label"), function (l) {
    var k = VAR[l.textContent.trim()];
    if (k) vboxes[k] = l.querySelector("input");
  });
  function show(which) {
    [].forEach.call(document.querySelectorAll("[data-body]"), function (b) { b.hidden = b.getAttribute("data-body") !== which; });
    [].forEach.call(document.querySelectorAll("[data-scope-panel]"), function (p) { p.hidden = p.getAttribute("data-scope-panel") !== which; });
  }
  Object.keys(vboxes).forEach(function (k) {
    vboxes[k].addEventListener("change", function () {
      if (vboxes[k].checked) {
        Object.keys(vboxes).forEach(function (o) { if (o !== k) vboxes[o].checked = false; });
        show(k);
      } else { show("main"); }
      sync();
    });
  });
  boxes.forEach(function (b) { b.addEventListener("change", sync); });
})();
"""


def page(layout, title, meta, project, relroot, body, header_text, panel_variants="", extra_css=""):
    landscape = False                      # v2: both layouts are portrait US Letter
    size = "8.5in 11in"
    css = CSS + (CSS_BOARD.replace("__COLS__", str(COLS)).replace("__GAP__", str(GAP)) if layout == "board" else CSS_OUTLINE) + extra_css
    logo = f'<meta name="of-logo" content="{relroot}00_SYSTEM/assets/openframe-logo.png">' if relroot != "../../" else ""
    return f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{esc(title)}</title>
<meta name="of-doc" content="{esc(meta)}">
{logo}
<link rel="stylesheet" href="{relroot}00_SYSTEM/assets/openframe-print.css">
<style>
  /* Running header: one compact line, top-right (project · document). Identifies loose printed pages of a long board. */
  @page {{ size: {size}; @top-right {{ content: "{esc(header_text)}"; }} }}
{css}
</style>
</head>
<body>
<div class="doc{' landscape' if landscape else ''}">
{body}
</div>
<script src="{relroot}00_SYSTEM/assets/of-preview.js"></script>
<script>{JS_VARIANTS}</script>
</body>
</html>
"""


# ------------------------------------------------------------------------------------------
# pagination probe loop
# ------------------------------------------------------------------------------------------

def read_page_starts(pdf):
    """first sentinel id on every page (in reading order)"""
    d = pymupdf.open(pdf)
    out = []
    for p in d:
        hits = []
        for w in p.get_text("words"):
            m = re.fullmatch(r"@@(\d+)@@", w[4])
            if m:
                hits.append((round(w[1]), w[0], int(m.group(1))))
        out.append(min(hits)[2] if hits else None)
    return out


def make_doc(layout, build_body, name, relroot, outdir, header_text, project, title, meta, extra_css=""):
    """Sequential pagination pinning.
    Each pass renders a probe (invisible sentinels at the start of every block), finds the first page whose
    first block is not yet pinned, pins that page start (forced break + 'continued' strip naming the act /
    sequence it belongs to) and renders again. Earlier page starts never move, so the loop always ends."""
    pins = {}                      # atom id -> chain (insertion ordered)
    pdf_probe = SCRATCH / f"{name}_probe.pdf"
    SCRATCH.mkdir(parents=True, exist_ok=True)
    for it in range(40):
        body, atoms = build_body(probe=True, cont=pins, pins=set(pins))
        tmp = outdir / f"_probe_{name}.html"
        tmp.write_text(page(layout, title, meta, project, relroot, body, header_text, extra_css=extra_css), encoding="utf-8")
        render_pdf.render(tmp, pdf_probe)
        tmp.unlink()
        starts = read_page_starts(pdf_probe)
        todo = [aid for pg, aid in enumerate(starts) if pg > 0 and aid is not None and aid not in pins]
        if not todo:
            print(f"   {name}: {len(starts)} pages, {len(pins)} pinned page starts after {it + 1} render(s)")
            break
        aid = todo[0]
        pins[aid] = atoms[aid]
    body, atoms = build_body(probe=False, cont=pins, pins=set(pins))
    return page(layout, title, meta, project, relroot, body, header_text, extra_css=extra_css), len(pins)


# ------------------------------------------------------------------------------------------
# documents
# ------------------------------------------------------------------------------------------

def hdr(project, layout):
    doc = "BEAT BOARD" if layout == "board" else "BEAT BOARD OUTLINE"
    return f"{project.upper()} · {doc}"


def build_primary(layout):
    project = "Low Tide Radio"
    doc_name = "Beat_Board_Card_Board" if layout == "board" else "Beat_Board_Outline"
    pdf_title = "Beat Board" if layout == "board" else "Beat Board Outline"

    def body_builder(probe, cont, pins):
        r = R(layout, probe=probe, cont=cont, pins=pins)
        b = low_tide()
        main = body_html(r, [b], False)
        atoms = r.atoms
        # ---- variant: selected items only (cards 24, 28 and the beat "Low point")
        sel_board = low_tide()
        sel = set()

        def pick(nodes):
            for n in nodes:
                if (n.kind == "card" and n.number in (24, 28)) or (n.kind == "beat" and n.title.startswith("Low point")):
                    sel.add(id(n))
                pick(n.children)
        pick(sel_board.acts)
        sel_board.acts = filter_selected(sel_board.acts, sel)
        sel_board.parking = []
        r2 = R(layout)
        v_sel = body_html(r2, [sel_board], False)
        # ---- variant: entire project, several boards (fictional)
        r3 = R(layout)
        film = low_tide()
        film.acts = film.acts[:1]
        film.parking = []
        film.unassigned = []
        number_board(film)
        ep = stress_episode()
        ep.acts[0].title = "Act One: Low Water"
        v_proj = body_html(r3, [film, ep], True)
        proj_panel = '<aside class="scope-panel" data-scope-panel="project" hidden><b>Entire project</b><span>Exported 22 Oct 2026</span></aside>'
        sel_panel = ('<aside class="scope-panel" data-scope-panel="selected" hidden><b>Selected items only</b>'
                     '<span>Exported 22 Oct 2026</span><span>Acts and sequences shown for context</span></aside>')
        panels = (f'<span data-toggle="Scope: Selected items only (example)" hidden></span>'
                  f'<span data-toggle="Scope: Entire project, several boards (fictional example)" hidden></span>')
        mast = masthead(layout, project, "Current Beat Board", "Current Beat Board", panels_extra=sel_panel + proj_panel + panels)
        illus = '<p class="note illus-note"><span class="illus">Illustrative (fictional)</span> two boards in one export; only the first act of Low Tide Radio is reused for the film board.</p>'
        body = (mast + f'<div data-body="main">{main}</div>'
                f'<div data-body="selected" hidden>{v_sel}</div>'
                f'<div data-body="project" hidden>{illus}{v_proj}</div>')
        return body, atoms

    html, n = make_doc(layout, body_builder, doc_name, "../../", OUT, hdr(project, layout), project, pdf_title,
                       f"Beat Board — {'Card Board' if layout == 'board' else 'Outline'} layout (story.export_board, layout={layout})")
    f = OUT / f"{doc_name}.html"
    f.write_text(html, encoding="utf-8")
    render_pdf.render(f, OUT / f"{doc_name}.pdf")
    print("   built", f.name, "continued strips:", n)


def build_stress(layout, variant):
    """variant 'A' = everything on (project scope, 3 boards); 'B' = options off, one tiny board"""
    doc_name = ("Beat_Board_Card_Board" if layout == "board" else "Beat_Board_Outline") + "_STRESS_TEST" + ("" if variant == "A" else "_B_options_off")
    pdf_title = ("Beat Board" if layout == "board" else "Beat Board Outline") + " (stress test)"
    project = "Harbour Lights"

    def body_builder(probe, cont, pins):
        if variant == "A":
            r = R(layout, probe=probe, cont=cont, pins=pins)
            boards = [stress_film(), stress_episode(), stress_episode_empty()]
            boards = [b for b in boards if b.acts or b.unassigned or b.parking]   # source drops empty boards
            multi = len(boards) > 1
            scope = "Entire project"
        else:
            r = R(layout, notes=False, comments=False, parking=False, probe=probe, cont=cont, pins=pins)
            b = stress_film()
            b.acts = b.acts[1:]                   # the long-title act + empty act
            b.acts[0].children = b.acts[0].children[:2]
            b.unassigned = []
            number_board(b)
            boards = [b]
            multi = False
            scope = "Current Beat Board"
        main = body_html(r, boards, multi)
        illus = '<span class="illus">Illustrative (fictional)</span>'
        mast = masthead(layout, project, "Current Beat Board", scope)
        mast = mast.replace('<div class="meta-line">', f'<div class="meta-line">{illus} ', 1)
        return mast + main, r.atoms

    html, n = make_doc(layout, body_builder, doc_name, "../../../", OUT / "stress-test", hdr(project, layout), project, pdf_title,
                       f"Beat Board — {layout} layout — STRESS TEST ({variant})")
    (OUT / "stress-test").mkdir(exist_ok=True)
    f = OUT / "stress-test" / f"{doc_name}.html"
    f.write_text(html, encoding="utf-8")
    render_pdf.render(f, OUT / "stress-test" / f"{doc_name}.pdf")
    print("   built", f.name, "continued strips:", n)


def stress_episode_empty():
    return stress_empty_ep()


if __name__ == "__main__":
    which = sys.argv[1:] or ["outline", "board"]
    (OUT / "stress-test").mkdir(parents=True, exist_ok=True)
    for lay in which:
        build_primary(lay)
        build_stress(lay, "A")
        build_stress(lay, "B")
