#!/usr/bin/env python3
"""Rasterize the Straft Studio brand files from packaging/brand/straft-logo-manrope.svg.

    python3 packaging/brand/build_assets.py logo     4K logo PNGs and the mark PNG
    python3 packaging/brand/build_assets.py print    logo for exports and the PDF package, on transparency
    python3 packaging/brand/build_assets.py verify   compare the 4K logo against the owner's image

Colour is never blended into edge pixels of a transparent file: a pixel holds the artwork's colour
and only its alpha varies, so no background can show a halo. Needs numpy, scipy, Pillow and OpenCV.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

import cv2
import numpy as np
from PIL import Image
from scipy.ndimage import distance_transform_edt

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
SVG = HERE / "straft-logo-manrope.svg"
SOURCE = HERE / "straft-logo-source.jpeg"
PRINT_TARGETS = [
    ROOT / "crates/straft-import-export/assets/straft-logo.png",
    ROOT / "docs/specs/pdf-design-package-v2/00_SYSTEM/assets/straft-logo.png",
]
PRINT_ORIGINAL = ROOT / "docs/specs/pdf-design-package-v2/00_SYSTEM/assets/straft-logo-original.png"

LOGO_WIDTH = 3840
LOGO_HEIGHT = 983
BG = (26, 29, 38)
PRINT_HEIGHT = 163  # pixel height the export footer logo has always had
PRINT_INK = "#1d1a2e"  # "Straft" on paper: the ink of the previous print logo, white would vanish

STOPS = ((0.0, (0xFF, 0x5D, 0x6C)), (0.55, (0xFF, 0x9A, 0x3D)), (1.0, (0xFF, 0xC2, 0x3D)))
MARK_LINES = ((7, 2, 7, 22), (17, 2, 17, 22), (2, 12, 22, 12), (2, 7, 7, 7), (2, 17, 7, 17), (22, 7, 17, 7), (22, 17, 17, 17))


# ---- reading the SVG ------------------------------------------------------------------------------


def read_svg():
    text = SVG.read_text(encoding="utf-8")
    m = re.search(r'id="mark" transform="translate\(([-\d.]+) ([-\d.]+)\) scale\(([-\d.]+)\)"', text)
    mark = tuple(float(x) for x in m.groups())
    words = []
    for wid in ("word-straft", "word-studio"):
        m = re.search(rf'id="{wid}" fill="(#[0-9a-f]{{6}})" fill-rule="evenodd" d="([^"]+)"', text)
        words.append((hex_rgb(m.group(1)), flatten(m.group(2))))
    return mark, words


def hex_rgb(h: str):
    return tuple(int(h[i : i + 2], 16) for i in (1, 3, 5))


def flatten(d: str) -> list[np.ndarray]:
    """Turn the M/L/C/Z path data this project writes into closed polygons, in viewBox units."""
    polys, cur = [], []
    for cmd, args in re.findall(r"([MLCZ])([^MLCZ]*)", d):
        v = [float(x) for x in args.split()]
        if cmd == "M":
            cur = [(v[0], v[1])]
        elif cmd == "L":
            cur.append((v[0], v[1]))
        elif cmd == "C":
            p0 = np.array(cur[-1])
            p1, p2, p3 = np.array(v[:2]), np.array(v[2:4]), np.array(v[4:6])
            steps = max(8, int(np.linalg.norm(p1 - p0) + np.linalg.norm(p2 - p1) + np.linalg.norm(p3 - p2)) * 4)
            for t in np.linspace(0, 1, steps + 1)[1:]:
                m = 1 - t
                cur.append(tuple(m**3 * p0 + 3 * m**2 * t * p1 + 3 * m * t**2 * p2 + t**3 * p3))
        elif cmd == "Z":
            polys.append(np.array(cur, dtype=np.float64))
            cur = []
    return polys


# ---- rendering -------------------------------------------------------------------------------------


def mark_distance(x, y):
    half = 0.8
    qx, qy = np.abs(x - 12) - 7.8, np.abs(y - 12) - 7.8
    outline = np.hypot(np.maximum(qx, 0), np.maximum(qy, 0)) + np.minimum(np.maximum(qx, qy), 0) - 2.2
    d = np.abs(outline) - half
    for ax, ay, bx, by in MARK_LINES:
        dx, dy = bx - ax, by - ay
        t = np.clip(((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy), 0, 1)
        d = np.minimum(d, np.hypot(x - ax - t * dx, y - ay - t * dy) - half)
    return d


def gradient(gx, gy):
    t = np.clip(((gx - 2) + (gy - 2)) / 40.0, 0, 1)
    offs = [o for o, _ in STOPS]
    return np.stack([np.interp(t, offs, [c[i] for _, c in STOPS]) for i in range(3)], axis=-1)


def render_mark(rgb, alpha, tx, ty, s, ss=4):
    """Paint the mark into rgb/alpha (float arrays). tx, ty, s map grid units to output pixels."""
    h, w = alpha.shape
    x0, x1 = max(0, int(tx + 0.5 * s)), min(w, int(tx + 23.5 * s) + 2)
    y0, y1 = max(0, int(ty + 0.5 * s)), min(h, int(ty + 23.5 * s) + 2)
    off = (np.arange(ss) + 0.5) / ss
    xs = (x0 + np.add.outer(np.arange(x1 - x0), off).ravel() - tx) / s
    ys = (y0 + np.add.outer(np.arange(y1 - y0), off).ravel() - ty) / s
    gx, gy = np.meshgrid(xs, ys)
    d = mark_distance(gx, gy) * s * ss  # in supersampled pixels
    cov = np.clip(0.5 - d, 0, 1)
    cov = cv2.resize(cov.astype(np.float32), (x1 - x0, y1 - y0), interpolation=cv2.INTER_AREA)
    cx = (np.arange(x0, x1) + 0.5 - tx) / s
    cy = (np.arange(y0, y1) + 0.5 - ty) / s
    colour = gradient(*np.meshgrid(cx, cy))
    rgb[y0:y1, x0:x1] = np.where(cov[..., None] > 0, colour, rgb[y0:y1, x0:x1])
    alpha[y0:y1, x0:x1] = np.maximum(alpha[y0:y1, x0:x1], cov)


def fill_polys(polys, scale, dx, dy, w, h, ss=8):
    """Even-odd coverage of polygons on a w by h grid. Returns a float coverage array."""
    all_pts = np.vstack(polys)
    x0 = max(0, int((all_pts[:, 0].min()) * scale + dx) - 2)
    x1 = min(w, int((all_pts[:, 0].max()) * scale + dx) + 3)
    y0 = max(0, int((all_pts[:, 1].min()) * scale + dy) - 2)
    y1 = min(h, int((all_pts[:, 1].max()) * scale + dy) + 3)
    big = np.zeros(((y1 - y0) * ss, (x1 - x0) * ss), np.uint8)
    shift = 6
    fixed = []
    for p in polys:
        q = (p * scale + [dx - x0, dy - y0]) * ss - 0.5  # OpenCV addresses pixel centres
        fixed.append(np.round(q * (1 << shift)).astype(np.int32))
    cv2.fillPoly(big, fixed, 255, lineType=cv2.LINE_8, shift=shift)
    small = cv2.resize(big, (x1 - x0, y1 - y0), interpolation=cv2.INTER_AREA).astype(np.float32) / 255.0
    out = np.zeros((h, w), np.float32)
    out[y0:y1, x0:x1] = small
    return out


def render_logo(width, height, straft_colour=None):
    """Transparent logo as straight-alpha RGBA uint8. Uniform scale; centred if rounding left a gap."""
    (tx, ty, s), words = read_svg()
    src_w, src_h = Image.open(SOURCE).size
    scale = width / src_w
    dy = (height - src_h * scale) / 2
    rgb = np.zeros((height, width, 3), np.float32)
    alpha = np.zeros((height, width), np.float32)
    for i, (colour, polys) in enumerate(words):
        if i == 0 and straft_colour is not None:
            colour = straft_colour
        cov = fill_polys(polys, scale, 0.0, dy, width, height)
        rgb[cov > 0] = colour
        alpha = np.maximum(alpha, cov)
    render_mark(rgb, alpha, tx * scale, ty * scale + dy, s * scale)
    return bleed(np.dstack([np.round(rgb), np.round(alpha * 255)]).astype(np.uint8))


def bleed(rgba):
    """Give fully transparent pixels the colour of the nearest artwork pixel.

    PDF viewers and simple image scalers resample colour and alpha separately. Black under the
    transparent area would then show as a dark fringe around the artwork.
    """
    empty = rgba[..., 3] == 0
    idx = distance_transform_edt(empty, return_distances=False, return_indices=True)
    out = rgba.copy()
    out[..., :3] = rgba[idx[0], idx[1]][..., :3]
    out[..., 3] = rgba[..., 3]
    return out


def over(rgba, bg):
    a = rgba[..., 3:4].astype(np.float32) / 255
    return np.round(rgba[..., :3] * a + np.array(bg, np.float32) * (1 - a)).astype(np.uint8)


def save_png(arr, path, mode=None):
    path.parent.mkdir(parents=True, exist_ok=True)
    Image.fromarray(arr, mode).save(path, optimize=True)


# ---- steps -------------------------------------------------------------------------------------------


def step_logo():
    rgba = render_logo(LOGO_WIDTH, LOGO_HEIGHT)
    save_png(rgba, HERE / "straft-logo-4k-transparent.png")
    save_png(over(rgba, BG), HERE / "straft-logo-4k.png")
    rgb = np.zeros((1024, 1024, 3), np.float32)
    alpha = np.zeros((1024, 1024), np.float32)
    render_mark(rgb, alpha, 0.0, 0.0, 1024 / 24, ss=2)
    save_png(bleed(np.dstack([np.round(rgb), np.round(alpha * 255)]).astype(np.uint8)), HERE / "straft-mark-1024.png")
    print("wrote straft-logo-4k.png, straft-logo-4k-transparent.png, straft-mark-1024.png")


def step_print():
    # Keep the artwork's own proportions: crop to its bounds at a height of PRINT_HEIGHT.
    (tx, ty, s), words = read_svg()
    pts = np.vstack([p for _, polys in words for p in polys])
    x1, x0 = float(pts[:, 0].max()), tx + 1.2 * s
    y0, y1 = ty + 1.2 * s, ty + 22.8 * s
    scale = PRINT_HEIGHT / (y1 - y0)
    width = int(np.ceil((x1 - x0) * scale))
    src_w, src_h = Image.open(SOURCE).size
    ink = hex_rgb(PRINT_INK)
    # Render on a 4 times grid, then reduce with the premultiplied area filter.
    ss = 4
    full_w = int(round(src_w * scale * ss))
    full_h = int(round(src_h * scale * ss))
    (tx_, ty_, s_) = (tx * scale * ss, ty * scale * ss, s * scale * ss)
    rgb = np.zeros((full_h, full_w, 3), np.float32)
    alpha = np.zeros((full_h, full_w), np.float32)
    for i, (colour, polys) in enumerate(words):
        cov = fill_polys(polys, scale * ss, 0.0, 0.0, full_w, full_h, ss=4)
        rgb[cov > 0] = ink if i == 0 else colour
        alpha = np.maximum(alpha, cov)
    render_mark(rgb, alpha, tx_, ty_, s_, ss=2)
    px0, py0 = int(round(x0 * scale * ss)), int(round(y0 * scale * ss))
    crop_rgb = rgb[py0 : py0 + PRINT_HEIGHT * ss, px0 : px0 + width * ss]
    crop_a = alpha[py0 : py0 + PRINT_HEIGHT * ss, px0 : px0 + width * ss]
    a = cv2.resize(crop_a, (width, PRINT_HEIGHT), interpolation=cv2.INTER_AREA)
    pm = cv2.resize(crop_rgb * crop_a[..., None], (width, PRINT_HEIGHT), interpolation=cv2.INTER_AREA)
    with np.errstate(divide="ignore", invalid="ignore"):
        straight = np.where(a[..., None] > 1e-4, pm / a[..., None], 0)
    out = bleed(np.dstack([np.round(np.clip(straight, 0, 255)), np.round(a * 255)]).astype(np.uint8))
    for target in PRINT_TARGETS:
        save_png(out, target)
        print(f"wrote {target.relative_to(ROOT)} {width}x{PRINT_HEIGHT}")
    # The "original" slot records the unmodified logo as supplied: the owner's image, pixel for pixel.
    Image.open(SOURCE).convert("RGB").save(PRINT_ORIGINAL, optimize=True)
    print(f"wrote {PRINT_ORIGINAL.relative_to(ROOT)} {src_w}x{src_h}")


def step_verify():
    ref = np.asarray(Image.open(SOURCE).convert("RGB"), dtype=np.float32)
    h, w = ref.shape[:2]
    big = np.asarray(Image.open(HERE / "straft-logo-4k.png").convert("RGB"))
    print("4K size", big.shape[1], big.shape[0])
    small = cv2.resize(big, (w, h), interpolation=cv2.INTER_AREA).astype(np.float32)
    diff = np.abs(small - ref)
    print(f"mean absolute difference, all channels, whole image: {diff.mean():.3f} of 255")
    for name, (x0, x1) in {"mark": (60, 385), "Straft": (425, 868), "Studio": (870, 1345)}.items():
        print(f"  {name}: mean {diff[:, x0:x1].mean():.3f}, luma-only mean {np.abs(lum(small) - lum(ref))[:, x0:x1].mean():.3f}")
    d = np.abs(lum(small) - lum(ref))
    print(f"max single-pixel luma difference: {d.max():.1f}")
    for k in (3, 5, 9):
        local = cv2.blur(d, (k, k))
        y, x = np.unravel_index(local.argmax(), local.shape)
        print(f"max local mean luma difference over {k}x{k}: {local.max():.2f} at x {x}, y {y}")
    print(f"share of pixels with luma difference above 16: {(d > 16).mean() * 100:.3f} percent")
    # Any global shift would show up as an improvement at a sub-pixel offset.
    a, b = lum(small), lum(ref)
    (sx, sy), resp = cv2.phaseCorrelate(a.astype(np.float64), b.astype(np.float64))
    print(f"phase-correlation shift of render against image: x {sx:.3f}, y {sy:.3f} px (response {resp:.3f})")
    # Transparent file over the same background must equal the opaque file.
    tr = np.asarray(Image.open(HERE / "straft-logo-4k-transparent.png").convert("RGBA"))
    comp = over(tr, BG)
    print("transparent over background vs opaque, max channel difference:", int(np.abs(comp.astype(int) - big.astype(int)).max()))
    edge = (tr[..., 3] > 0) & (tr[..., 3] < 255)
    print(f"edge pixels {edge.sum()}, darkest edge colour luma {lum(tr[..., :3].astype(np.float32))[edge].min():.1f} (a halo of the background would be near {0.299*26+0.587*29+0.114*38:.0f})")


def lum(a):
    return a[..., 0] * 0.299 + a[..., 1] * 0.587 + a[..., 2] * 0.114


STEPS = {"logo": step_logo, "print": step_print, "verify": step_verify}

if __name__ == "__main__":
    if len(sys.argv) < 2 or sys.argv[1] not in STEPS:
        sys.exit("usage: build_assets.py " + "|".join(STEPS))
    STEPS[sys.argv[1]]()
