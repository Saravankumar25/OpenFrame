#!/usr/bin/env python3
"""Mechanical checks on a rendered design-reference PDF (requires PyMuPDF).

usage: python verify_pdf.py <file.pdf> [--size letter|landscape] [--thumbs DIR] [--gray]
                           [--header "TEXT"] [--no-header] [--dpi 70]

Per page it checks: page size; footer present (brand line, "Page X of Y" with the true
total, logo image in the footer band, logo aspect ratio undistorted); no content (text,
vector shapes, images) inside the footer band or outside the side margins; optional
running header text present / absent; blank pages; fill ratio (to spot near-empty pages).
Writes PNG thumbnails with --thumbs (--gray for a grayscale print-readability review).
Exit status 1 on any failure. These are mechanical checks: they do NOT replace looking
at the pages.
"""
import argparse
import os
import re
import sys

import pymupdf

PAPER = {"letter": (612, 792), "landscape": (792, 612)}
MARGIN_X, MARGIN_TOP, MARGIN_BOTTOM = 46, 46, 66
TOL = 1.5
LOGO_RATIO = 699 / 163


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("pdf")
    ap.add_argument("--size", default="letter", choices=list(PAPER))
    ap.add_argument("--thumbs")
    ap.add_argument("--gray", action="store_true")
    ap.add_argument("--header")
    ap.add_argument("--no-header", action="store_true")
    ap.add_argument("--dpi", type=int, default=70)
    a = ap.parse_args()
    d = pymupdf.open(a.pdf)
    W, H = PAPER[a.size]
    n = len(d)
    fails = []
    rows = []
    for i, p in enumerate(d, 1):
        w, h = p.rect.width, p.rect.height
        if abs(w - W) > 1 or abs(h - H) > 1:
            fails.append(f"p{i}: page size {w:.0f}x{h:.0f} != {W}x{H}")
        words = p.get_text("words")
        text = p.get_text()
        foot_top = h - MARGIN_BOTTOM
        if "Created using OpenFrame" not in text:
            fails.append(f"p{i}: brand line missing")
        m = re.search(r"Page\s+(\d+)\s+of\s+(\d+)", text)
        if not m:
            fails.append(f"p{i}: 'Page X of Y' missing")
        elif int(m.group(1)) != i or int(m.group(2)) != n:
            fails.append(f"p{i}: page label says {m.group(1)} of {m.group(2)}, expected {i} of {n}")
        imgs = p.get_image_info()
        logo = [im for im in imgs if im["bbox"][1] > foot_top - 4]
        if not logo:
            fails.append(f"p{i}: footer logo image not found in footer band")
        else:
            lb = logo[0]["bbox"]
            ratio = (lb[2] - lb[0]) / max(lb[3] - lb[1], 1e-6)
            if abs(ratio - LOGO_RATIO) > 0.12:
                fails.append(f"p{i}: logo aspect {ratio:.2f} != {LOGO_RATIO:.2f} (distorted)")
        content_bottom = 0.0
        minx, maxx = 1e9, 0.0
        for x0, y0, x1, y1, *_ in words:
            if y0 > foot_top - 2 or y1 < MARGIN_TOP - 2:
                continue
            content_bottom = max(content_bottom, y1)
            minx, maxx = min(minx, x0), max(maxx, x1)
        for dr in p.get_drawings():
            r = dr["rect"]
            if r.y0 > foot_top - 2 or r.y1 < MARGIN_TOP - 2 or r.width > w - 1 or r.height > h - 1:
                continue
            if dr.get("fill") == (1.0, 1.0, 1.0) and r.width * r.height > 0.5 * w * h:
                continue  # white page-area background, not content
            content_bottom = max(content_bottom, r.y1)
            minx, maxx = min(minx, r.x0), max(maxx, r.x1)
        for im in imgs:
            b = im["bbox"]
            if b[1] > foot_top - 4:
                continue
            content_bottom = max(content_bottom, b[3])
            minx, maxx = min(minx, b[0]), max(maxx, b[2])
        if content_bottom > foot_top + TOL:
            fails.append(f"p{i}: content reaches y={content_bottom:.1f} > footer top {foot_top:.0f}")
        if content_bottom and (minx < MARGIN_X - TOL or maxx > w - MARGIN_X + TOL):
            fails.append(f"p{i}: content x-range {minx:.1f}-{maxx:.1f} outside margins {MARGIN_X}-{w - MARGIN_X:.0f}")
        usable = h - MARGIN_TOP - MARGIN_BOTTOM
        fill = max(0.0, content_bottom - MARGIN_TOP) / usable if content_bottom else 0.0
        if a.header and a.header not in text:
            fails.append(f"p{i}: running header '{a.header}' missing")
        if a.no_header and p.get_text("words", clip=pymupdf.Rect(0, 0, w, MARGIN_TOP - 4)):
            fails.append(f"p{i}: unexpected text in top margin")
        if content_bottom == 0:
            fails.append(f"p{i}: BLANK page")
        rows.append((i, round(fill * 100)))
        if a.thumbs:
            os.makedirs(a.thumbs, exist_ok=True)
            pm = p.get_pixmap(dpi=a.dpi, colorspace=pymupdf.csGRAY if a.gray else pymupdf.csRGB)
            pm.save(os.path.join(a.thumbs, f"{os.path.splitext(os.path.basename(a.pdf))[0]}_p{i:02d}.png"))
    print(f"{os.path.basename(a.pdf)}: {n} page(s), {os.path.getsize(a.pdf) // 1024} KB, title={d.metadata.get('title')!r}")
    print("  content fill % per page:", " ".join(f"p{i}:{f}" for i, f in rows))
    low = [i for i, f in rows if f < 18 and i != n and n > 1]
    if low:
        print("  WARNING near-empty non-final pages:", low)
    if n > 1 and rows[-1][1] < 8:
        print(f"  WARNING final page nearly empty (fill {rows[-1][1]}%)")
    if fails:
        print("  FAIL")
        for f in fails:
            print("   -", f)
        sys.exit(1)
    print("  OK")


if __name__ == "__main__":
    main()
