#!/usr/bin/env python3
"""Render an OpenFrame design-reference HTML file to PDF with headless Chrome/Edge.

usage: python render_pdf.py <input.html> <output.pdf>

Deterministic for a given browser build. The page size, margins, running footer and
"Page X of Y" come from the CSS @page rules in the HTML; no browser header/footer is
added (--no-pdf-header-footer). Page numbers and the total are computed by the browser
from the real rendered page count.
"""
import os
import pathlib
import shutil
import subprocess
import sys

CANDIDATES = [
    r"C:\Program Files\Google\Chrome\Application\chrome.exe",
    r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    "/usr/bin/google-chrome",
    "/usr/bin/chromium",
    "/usr/bin/chromium-browser",
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
]


def browser():
    for c in CANDIDATES:
        if os.path.exists(c):
            return c
    for n in ("google-chrome", "chromium", "chrome", "msedge"):
        p = shutil.which(n)
        if p:
            return p
    sys.exit("No Chrome/Chromium/Edge found")


def render(html, pdf):
    html = pathlib.Path(html).resolve()
    pdf = pathlib.Path(pdf).resolve()
    pdf.parent.mkdir(parents=True, exist_ok=True)
    if pdf.exists():
        pdf.unlink()
    cmd = [
        browser(), "--headless=new", "--disable-gpu", "--no-pdf-header-footer",
        "--hide-scrollbars", "--allow-file-access-from-files",
        "--virtual-time-budget=4000",
        f"--print-to-pdf={pdf}", html.as_uri(),
    ]
    r = subprocess.run(cmd, capture_output=True, text=True, timeout=240)
    if not pdf.exists() or pdf.stat().st_size < 1000:
        sys.exit(f"render failed for {html}\n{r.stderr[-800:]}")
    return pdf


if __name__ == "__main__":
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    out = render(sys.argv[1], sys.argv[2])
    print("wrote", out, out.stat().st_size, "bytes")
