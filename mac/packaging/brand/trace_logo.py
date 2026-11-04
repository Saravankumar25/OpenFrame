#!/usr/bin/env python3
"""Trace the owner's logo image into packaging/brand/straft-logo-manrope.svg.

The film-frame mark is not traced: its geometry is the 24 by 24 grid from
apps/desktop/src-tauri/icons/source.svg, only placed by a least-squares fit against the image.
The wordmark is traced from the image's luma channel: the 50 percent edge is found with marching
squares on an upscaled coverage map, split at corners, then fitted with straight lines and cubic
Beziers. Corners are rebuilt where the fitted sides meet, because the image softens them.

Needs numpy, Pillow, OpenCV and contourpy (shipped with matplotlib). Nothing is downloaded.

    python3 packaging/brand/trace_logo.py
"""

from __future__ import annotations

import math
from pathlib import Path

import cv2
import numpy as np
from contourpy import contour_generator
from PIL import Image
from scipy.ndimage import gaussian_filter, gaussian_filter1d
from scipy.optimize import minimize

HERE = Path(__file__).resolve().parent
SOURCE = HERE / "straft-logo-source.jpeg"
OUT = HERE / "straft-logo-manrope.svg"
MARK_OUT = HERE / "straft-mark.svg"

BG = "#1a1d26"
BG_RGB = (26, 29, 38)
STOPS = ((0.0, "#ff5d6c"), (0.55, "#ff9a3d"), (1.0, "#ffc23d"))
STRAFT_COLOUR = "#ffffff"
STUDIO_COLOUR = "#ff9a3d"

# Column ranges of the two words in the source image (measured, with margin).
WORDS = {"straft": (425, 868), "studio": (870, 1345)}
ROWS = (100, 262)

UPSCALE = 6  # marching squares grid per source pixel
PRE_BLUR = 0.45  # source pixels; calms JPEG ringing, the 50 percent edge of a straight side stays put
STEP = 0.2  # contour resampling, source pixels
SMOOTH = 0.5  # along the whole contour before corners are found, source pixels
SECTION_SMOOTH = 1.1  # along each side once corners are known, source pixels
CORNER_WINDOW = 2.2  # turning is measured across +/- this arc length
CORNER_DEG = 48.0
TRIM = 2.0  # points this near a corner are softened by the image and ignored
CORNER_REACH = 4.5  # a rebuilt corner may sit at most this far from the traced one
LINE_TOL = 0.09  # largest distance from a straight run to its line
LINE_BOW = 0.035  # largest parabolic bow of a straight run; gentle arcs fail this
LINE_MIN = 8.0  # shortest straight run
DIAGONAL_MIN = 24.0  # shortest straight run that is neither horizontal nor vertical
LINE_BACKOFF = 2.0  # a straight run gives this much back to the curve that follows it
WHOLE_LINE_TOL = 0.22  # a whole side between two corners within this is one line
CURVE_TOL = 0.20
AXIS_DEG = 1.1
AXIS_TAN = math.tan(math.radians(AXIS_DEG))
SNAP_ROW = 0.40  # horizontal edges closer than this share one height


def read_luma(path: Path) -> np.ndarray:
    im = Image.open(path)
    im.draft("YCbCr", im.size)  # the JPEG's own full-resolution luma, without a round trip
    im = im.convert("YCbCr")
    return np.asarray(im, dtype=np.float64)[:, :, 0]


def rgb_luma(rgb) -> float:
    return 0.299 * rgb[0] + 0.587 * rgb[1] + 0.114 * rgb[2]


def hex_rgb(h: str) -> tuple[int, int, int]:
    return tuple(int(h[i : i + 2], 16) for i in (1, 3, 5))


# ---- mark placement ----------------------------------------------------------------------------


def mark_distance(x: np.ndarray, y: np.ndarray) -> np.ndarray:
    """Signed distance in grid units to the union of the mark's round-capped strokes."""
    half = 0.8
    qx, qy = np.abs(x - 12) - (10 - 2.2), np.abs(y - 12) - (10 - 2.2)
    outline = np.hypot(np.maximum(qx, 0), np.maximum(qy, 0)) + np.minimum(np.maximum(qx, qy), 0) - 2.2
    d = np.abs(outline) - half
    for (ax, ay, bx, by) in MARK_LINES:
        dx, dy = bx - ax, by - ay
        t = np.clip(((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy), 0, 1)
        d = np.minimum(d, np.hypot(x - ax - t * dx, y - ay - t * dy) - half)
    return d


MARK_LINES = (
    (7, 2, 7, 22),
    (17, 2, 17, 22),
    (2, 12, 22, 12),
    (2, 7, 7, 7),
    (2, 17, 7, 17),
    (22, 7, 17, 7),
    (22, 17, 17, 17),
)


def fit_mark(rgb: np.ndarray) -> tuple[float, float, float, float]:
    """Place the 24 grid over the image by least squares. Returns tx, ty, scale, rms coverage error."""
    box = (60, 20, 385, 340)
    crop = rgb[box[1] : box[3], box[0] : box[2]]
    bg = np.array(BG_RGB, float)
    # Distance from the background colour works for every gradient position.
    dist = np.linalg.norm(crop - bg, axis=2)
    ys, xs = np.mgrid[box[1] : box[3], box[0] : box[2]]
    xc, yc = xs + 0.5, ys + 0.5

    def model(p):
        tx, ty, s = p
        d = mark_distance((xc - tx) / s, (yc - ty) / s) * s
        return np.clip(0.5 - d, 0, 1)

    # Colour at full coverage, per pixel, from the gradient.
    def colour(p):
        tx, ty, s = p
        gx, gy = (xc - tx) / s, (yc - ty) / s
        return gradient_rgb(gx, gy)

    def cost(p):
        pred = bg + model(p)[..., None] * (colour(p) - bg)
        return float(((pred - crop) ** 2).mean())

    # Start from the measured outer box: 21.6 grid units across.
    x0 = [84.0, 41.0, 278.0 / 21.6]
    x0[0] -= 1.2 * x0[2]
    x0[1] -= 1.2 * x0[2]
    res = minimize(cost, x0, method="Nelder-Mead", options={"xatol": 1e-3, "fatol": 1e-4, "maxiter": 800})
    tx, ty, s = res.x
    rms = float(np.sqrt(((model(res.x) - np.clip(dist / np.linalg.norm(colour(res.x) - bg, axis=2), 0, 1)) ** 2).mean()))
    return float(tx), float(ty), float(s), rms


def gradient_rgb(gx: np.ndarray, gy: np.ndarray) -> np.ndarray:
    t = np.clip(((gx - 2) + (gy - 2)) / 40.0, 0, 1)
    out = np.empty(t.shape + (3,))
    offs = [o for o, _ in STOPS]
    cols = [hex_rgb(c) for _, c in STOPS]
    for i in range(3):
        out[..., i] = np.interp(t, offs, [c[i] for c in cols])
    return out


# ---- contour extraction -------------------------------------------------------------------------


def word_contours(luma: np.ndarray, x0: int, x1: int, fg_luma: float) -> list[np.ndarray]:
    bg_luma = rgb_luma(BG_RGB)
    y0, y1 = ROWS
    cov = (luma[y0:y1, x0:x1] - bg_luma) / (fg_luma - bg_luma)
    cov = gaussian_filter(np.clip(cov, -0.3, 1.3), PRE_BLUR, mode="nearest")
    h, w = cov.shape
    big = cv2.resize(cov, (w * UPSCALE, h * UPSCALE), interpolation=cv2.INTER_CUBIC)
    gen = contour_generator(z=big, name="serial", line_type="Separate")
    out = []
    for line in gen.lines(0.5):
        if len(line) < 40 or np.linalg.norm(line[0] - line[-1]) > 1e-6:
            continue
        pts = (line[:-1] + 0.5) / UPSCALE + np.array([x0, y0])
        if polygon_area(pts) < 8.0:  # compression specks
            continue
        out.append(pts)
    return out


def polygon_area(p: np.ndarray) -> float:
    x, y = p[:, 0], p[:, 1]
    return abs(float(np.dot(x, np.roll(y, -1)) - np.dot(y, np.roll(x, -1)))) / 2


def resample_closed(p: np.ndarray, step: float) -> np.ndarray:
    q = np.vstack([p, p[:1]])
    seg = np.hypot(*np.diff(q, axis=0).T)
    s = np.concatenate([[0], np.cumsum(seg)])
    n = max(16, int(round(s[-1] / step)))
    t = np.linspace(0, s[-1], n, endpoint=False)
    return np.column_stack([np.interp(t, s, q[:, 0]), np.interp(t, s, q[:, 1])])


def turning(p: np.ndarray, window: int) -> np.ndarray:
    """Turning angle in degrees across +/- window samples."""
    fwd = np.roll(p, -window, axis=0) - p
    back = p - np.roll(p, window, axis=0)
    a = np.arctan2(fwd[:, 1], fwd[:, 0]) - np.arctan2(back[:, 1], back[:, 0])
    return np.degrees((a + np.pi) % (2 * np.pi) - np.pi)


def find_corners(p: np.ndarray) -> list[int]:
    window = int(round(CORNER_WINDOW / STEP))
    t = np.abs(turning(p, window))
    n = len(p)
    corners: list[int] = []
    order = np.argsort(-t)
    taken = np.zeros(n, bool)
    span = int(round(3.0 / STEP))
    for i in order:
        if t[i] < CORNER_DEG:
            break
        if taken[i]:
            continue
        corners.append(int(i))
        for k in range(-span, span + 1):
            taken[(i + k) % n] = True
    return sorted(corners)


# ---- fitting --------------------------------------------------------------------------------------


def fit_line(pts: np.ndarray):
    c = pts.mean(axis=0)
    u, s, vt = np.linalg.svd(pts - c, full_matrices=False)
    d = vt[0]
    if np.dot(pts[-1] - pts[0], d) < 0:
        d = -d
    n = np.array([-d[1], d[0]])
    dev = np.abs((pts - c) @ n).max()
    return c, d, dev


def bezier(c, t):
    t = t[:, None]
    m = 1 - t
    return m**3 * c[0] + 3 * m**2 * t * c[1] + 3 * m * t**2 * c[2] + t**3 * c[3]


def chord_params(p):
    d = np.concatenate([[0], np.cumsum(np.hypot(*np.diff(p, axis=0).T))])
    return d / d[-1]


def generate(p, u, t0, t1):
    """Schneider: least-squares handle lengths with fixed end points and end tangents."""
    b0, b1, b2, b3 = (1 - u) ** 3, 3 * u * (1 - u) ** 2, 3 * u**2 * (1 - u), u**3
    a0 = t0[None, :] * b1[:, None]
    a1 = t1[None, :] * b2[:, None]
    c00, c01, c11 = (a0 * a0).sum(), (a0 * a1).sum(), (a1 * a1).sum()
    tmp = p - (p[0] * (b0 + b1)[:, None] + p[-1] * (b2 + b3)[:, None])
    x0, x1 = (a0 * tmp).sum(), (a1 * tmp).sum()
    det = c00 * c11 - c01 * c01
    seg = np.linalg.norm(p[-1] - p[0])
    if abs(det) > 1e-9:
        al, ar = (x0 * c11 - x1 * c01) / det, (c00 * x1 - c01 * x0) / det
    else:
        al = ar = seg / 3
    if al < 1e-3 * seg or ar < 1e-3 * seg:
        al = ar = seg / 3
    return np.array([p[0], p[0] + t0 * al, p[-1] + t1 * ar, p[-1]])


def reparam(c, p, u):
    d1 = 3 * (c[1:] - c[:-1])
    d2 = 2 * (d1[1:] - d1[:-1])
    for _ in range(1):
        t = u[:, None]
        m = 1 - t
        q = bezier(c, u)
        q1 = m**2 * d1[0] + 2 * m * t * d1[1] + t**2 * d1[2]
        q2 = m * d2[0] + t * d2[1]
        num = ((q - p) * q1).sum(1)
        den = (q1 * q1).sum(1) + ((q - p) * q2).sum(1)
        with np.errstate(divide="ignore", invalid="ignore"):
            u = np.where(np.abs(den) > 1e-12, u - num / den, u)
    return np.clip(u, 0, 1)


def fit_cubic(p: np.ndarray, t0, t1, tol: float, tangent_at) -> list[np.ndarray]:
    """Recursive cubic fit; tangent_at(index) gives the unit tangent used at a split."""
    t0, t1 = np.array(t0, float), np.array(t1, float)
    if len(p) < 3:
        s = np.linalg.norm(p[-1] - p[0]) / 3
        return [np.array([p[0], p[0] + t0 * s, p[-1] + t1 * s, p[-1]])]
    u = chord_params(p)
    best = None
    for _ in range(6):
        c = generate(p, u, t0, t1)
        err = np.linalg.norm(bezier(c, u) - p, axis=1)
        best = (c, err.max(), int(err.argmax()))
        if best[1] < tol:
            return [c]
        u = reparam(c, p, u)
    c, _, split = best
    split = min(max(split, 2), len(p) - 3) if len(p) > 5 else len(p) // 2
    tc = np.array(tangent_at(p, split), float)
    return fit_cubic(p[: split + 1], t0, -tc, tol, tangent_at) + fit_cubic(p[split:], tc, t1, tol, tangent_at)


def local_tangent(p: np.ndarray, i: int, span: int = 15) -> np.ndarray:
    a, b = max(0, i - span), min(len(p), i + span + 1)
    _, d, _ = fit_line(p[a:b])
    return d


def is_straight(p: np.ndarray) -> bool:
    """Straight means close to its fitted line and without the parabolic bow of a gentle arc."""
    c, d, dev = fit_line(p)
    if dev > LINE_TOL:
        return False
    n = np.array([-d[1], d[0]])
    along, off = (p - c) @ d, (p - c) @ n
    length = along.max() - along.min()
    if length <= 0:
        return True
    bow = abs(np.polyfit(along, off, 2)[0]) * length * length / 8
    return bow <= LINE_BOW


def straight_runs(p: np.ndarray) -> list[tuple[int, int]]:
    """Greedy longest-first straight runs (index pairs) in an open point list."""
    min_n = int(round(LINE_MIN / STEP))
    step = max(1, int(round(1.0 / STEP)))
    runs: list[tuple[int, int]] = []

    def search(lo: int, hi: int):
        if hi - lo < min_n:
            return
        best = None
        for a in range(lo, hi - min_n + 1, step):
            b = a + min_n
            if not is_straight(p[a : b + 1]):
                continue
            while b + step <= hi and is_straight(p[a : b + step + 1]):
                b += step
            if hi - b < step and is_straight(p[a : hi + 1]):
                b = hi
            if a - lo < step and is_straight(p[lo : b + 1]):
                a = lo
            if best is None or b - a > best[1] - best[0]:
                best = (a, b)
        if best is None:
            return
        runs.append(best)
        search(lo, best[0])
        search(best[1], hi)

    search(0, len(p) - 1)
    return sorted(runs)


def extrema_splits(q: np.ndarray) -> list[tuple[int, np.ndarray]]:
    """Indices where the outline turns horizontal or vertical, as a type designer places nodes."""
    span = int(round(1.5 / STEP))
    margin = int(round(3.0 / STEP))
    if len(q) < 2 * margin + 3:
        return []
    d = q[2 * span :] - q[: -2 * span]  # central difference, index i + span
    out: list[tuple[int, np.ndarray]] = []
    for axis, (ux, uy) in ((0, (0.0, 1.0)), (1, (1.0, 0.0))):
        sign = np.sign(d[:, axis])
        change = np.nonzero(sign[1:] * sign[:-1] < 0)[0]
        for i in change:
            k = int(i) + span
            if margin <= k < len(q) - margin:
                other = d[i, 1 - axis]
                t = np.array([ux, uy]) * (1.0 if other >= 0 else -1.0)
                out.append((k, t))
    out.sort(key=lambda x: x[0])
    kept: list[tuple[int, np.ndarray]] = []
    for k, t in out:
        if not kept or k - kept[-1][0] > margin:
            kept.append((k, t))
    return kept


def fit_curve_run(q: np.ndarray, ta, tb) -> list[tuple]:
    """Cubics for one curved run, end directions of travel ta and tb, split at extrema."""
    if len(q) == 2:
        return [("L", q[0], q[1])]
    pieces = []
    cur, cur_t = 0, np.array(ta, float)
    for k, t in extrema_splits(q):
        pieces.append((cur, k, cur_t, t))
        cur, cur_t = k, t
    pieces.append((cur, len(q) - 1, cur_t, np.array(tb, float)))
    out: list[tuple] = []
    for i, j, t0, t1 in pieces:
        for c in fit_cubic(q[i : j + 1], t0, -t1, CURVE_TOL, local_tangent):
            out.append(("C", c[0], c[1], c[2], c[3]))
    return out


def fit_section(p: np.ndarray, start_t, end_t) -> list[tuple]:
    """Fit an open run of points. start_t/end_t are the directions of travel at the ends."""
    if np.linalg.norm(p[-1] - p[0]) > 1.0 and fit_line(p)[2] <= WHOLE_LINE_TOL:
        return [("L", p[0], p[-1])]
    lines = []
    back = int(round(LINE_BACKOFF / STEP))
    for a, b in straight_runs(p):
        _, d, _ = fit_line(p[a : b + 1])
        length = np.linalg.norm(p[b] - p[a])
        d = snap_direction(d, length)
        if abs(d[0]) != 1.0 and abs(d[1]) != 1.0 and length < DIAGONAL_MIN:
            continue  # a short slanted stretch is part of a curve
        # Leave the curve room to start bending before the run's greedy end.
        c = p[a : b + 1].mean(axis=0)
        if a > 0:
            a += back
        if b < len(p) - 1:
            b -= back
        if b <= a:
            continue
        lines.append((a, b, c + d * np.dot(p[a] - c, d), c + d * np.dot(p[b] - c, d), d))

    def curve(idx_a, idx_b, pa, pb, ta, tb):
        q = p[idx_a : idx_b + 1].copy()
        q[0], q[-1] = pa, pb
        if len(q) < 2 or np.hypot(*np.diff(q, axis=0).T).sum() < 1e-3:
            return []
        return fit_curve_run(q, ta, tb)

    segs: list[tuple] = []
    cur_i, cur_p, cur_t = 0, p[0], np.array(start_t, float)
    for a, b, pa, pb, d in lines:
        if a > cur_i:
            segs += curve(cur_i, a, cur_p, pa, cur_t, d)
        else:
            pa = cur_p
        if b == len(p) - 1:
            pb = p[-1]
        segs.append(("L", pa, pb))
        cur_i, cur_p, cur_t = b, pb, d
    if cur_i < len(p) - 1:
        segs += curve(cur_i, len(p) - 1, cur_p, p[-1], cur_t, np.array(end_t, float))
    return segs


def end_ray(run: np.ndarray) -> np.ndarray:
    """Direction of travel at the start of run, over the longest length that is still straight."""
    best = None
    for length in (3.0, 5.0, 8.0, 12.0, 18.0, 30.0, 60.0):
        n = int(round(length / STEP))
        if n >= len(run):
            n = len(run) - 1
        if n < 2:
            break
        q = run[: n + 1]
        if best is not None and not is_straight(q):
            break
        best = (fit_line(q)[1], np.linalg.norm(q[-1] - q[0]))
        if n == len(run) - 1:
            break
    if best is None:
        return unit_array(run[-1] - run[0])
    return snap_direction(best[0], best[1])


def unit_array(v: np.ndarray) -> np.ndarray:
    return v / max(np.linalg.norm(v), 1e-9)


def snap_direction(d: np.ndarray, length: float) -> np.ndarray:
    ang = math.degrees(math.atan2(d[1], d[0]))
    for axis in (0, 90, 180, -90, -180):
        delta = (ang - axis + 180) % 360 - 180
        if abs(delta) < AXIS_DEG and length * math.sin(math.radians(abs(delta))) < 0.35:
            r = math.radians(axis)
            return np.array([round(math.cos(r)), round(math.sin(r))], float)
    return d


def intersect(p0, d0, p1, d1):
    det = d0[0] * d1[1] - d0[1] * d1[0]
    if abs(det) < 1e-6:
        return None
    w = p1 - p0
    return p0 + d0 * ((w[0] * d1[1] - w[1] * d1[0]) / det)


def smooth_open(q: np.ndarray, sigma_px: float) -> np.ndarray:
    """Smooth along an open run with the ends held, so a run keeps its reach."""
    if len(q) < 8:
        return q
    sigma = sigma_px / STEP
    out = np.column_stack(
        [gaussian_filter1d(q[:, 0], sigma, mode="nearest"), gaussian_filter1d(q[:, 1], sigma, mode="nearest")]
    )
    # "nearest" pulls the first and last few samples inward; restore them on the original line.
    k = min(int(3 * sigma), len(q) // 2)
    w = np.linspace(0, 1, k)[:, None] if k else None
    if k:
        out[:k] = q[:k] * (1 - w) + out[:k] * w
        out[-k:] = out[-k:] * (1 - w) + q[-k:] * w
    return out


def trace_contour(raw: np.ndarray) -> list[tuple]:
    p = resample_closed(raw, STEP)
    sigma = SMOOTH / STEP
    p = np.column_stack(
        [gaussian_filter1d(p[:, 0], sigma, mode="wrap"), gaussian_filter1d(p[:, 1], sigma, mode="wrap")]
    )
    corners = find_corners(p)
    n = len(p)
    if not corners:
        # Smooth closed shape: start at the rightmost point, whose tangent is vertical in any glyph.
        wide = np.column_stack(
            [gaussian_filter1d(p[:, 0], SECTION_SMOOTH / STEP, mode="wrap"), gaussian_filter1d(p[:, 1], SECTION_SMOOTH / STEP, mode="wrap")]
        )
        i = int(wide[:, 0].argmax())
        q = np.vstack([np.roll(wide, -i, axis=0), wide[i : i + 1]])
        d = np.array([0.0, 1.0 if wide[(i + 5) % n, 1] > wide[i, 1] else -1.0])
        return fit_section(q, d, d)

    trim = int(round(TRIM / STEP))
    sections = []
    for k, c in enumerate(corners):
        nxt = corners[(k + 1) % len(corners)]
        length = (nxt - c) % n or n
        idx = (c + np.arange(trim, length - trim + 1)) % n
        if len(idx) < 5:
            # A very short side between two corners: keep what is there.
            idx = (c + np.arange(1, length)) % n
        sections.append(smooth_open(p[idx], SECTION_SMOOTH))
    # Rays leaving each section end, used to rebuild the corners.
    verts = []
    for k, c in enumerate(corners):
        before, after = sections[k - 1], sections[k]
        db = -end_ray(before[::-1])
        da = end_ray(after)
        pb, pa = before[-1], after[0]
        v = intersect(pb, db, pa, da)
        if v is None or np.linalg.norm(v - p[c]) > CORNER_REACH or np.dot(v - pb, db) < -0.5 or np.dot(pa - v, da) < -0.5:
            v = p[c]
        verts.append((np.array(v), db, da))

    segs: list[tuple] = []
    for k in range(len(corners)):
        v0, _, t0 = verts[k]
        v1, t1, _ = verts[(k + 1) % len(corners)]
        q = np.vstack([v0, sections[k], v1])
        segs += fit_section(q, t0, t1)
    return segs


# ---- clean-up ---------------------------------------------------------------------------------------


def weld(segs: list[tuple]) -> list[list]:
    """Mutable, gapless closed list of segments."""
    out = [[s[0], *[np.array(x, float) for x in s[1:]]] for s in segs]
    # Drop zero-length lines left by corner rebuilding.
    out = [s for s in out if np.linalg.norm(s[-1] - s[1]) > 1e-3]
    for i, s in enumerate(out):
        nxt = out[(i + 1) % len(out)]
        mid = (s[-1] + nxt[1]) / 2
        s[-1], nxt[1] = mid.copy(), mid.copy()
    return out


def move_vertex(segs, i, dx, dy):
    """Move the vertex between segment i and i+1, carrying curve handles so tangents survive."""
    d = np.array([dx, dy])
    a, b = segs[i], segs[(i + 1) % len(segs)]
    a[-1] = a[-1] + d
    b[1] = b[1] + d
    if a[0] == "C":
        a[3] = a[3] + d
    if b[0] == "C":
        b[2] = b[2] + d


def straighten_axis(segs: list[list]) -> None:
    """Lines that end up a hair off horizontal or vertical become exactly so."""
    for i, s in enumerate(segs):
        if s[0] != "L":
            continue
        dx, dy = s[2][0] - s[1][0], s[2][1] - s[1][1]
        if abs(dy) < 1e-9 or abs(dx) < 1e-9:
            continue
        if abs(dy) <= abs(dx) * AXIS_TAN and abs(dy) < 0.45:
            mid = (s[1][1] + s[2][1]) / 2
            move_vertex(segs, i - 1, 0.0, mid - s[1][1])
            move_vertex(segs, i, 0.0, mid - s[2][1])
            s[1][1] = s[2][1] = mid
        elif abs(dx) <= abs(dy) * AXIS_TAN and abs(dx) < 0.45:
            mid = (s[1][0] + s[2][0]) / 2
            move_vertex(segs, i - 1, mid - s[1][0], 0.0)
            move_vertex(segs, i, mid - s[2][0], 0.0)
            s[1][0] = s[2][0] = mid


def snap_rows(contours: list[list[list]]) -> None:
    """Make horizontal edges that belong to one font line share the same height."""
    rows = []
    for ci, segs in enumerate(contours):
        for si, s in enumerate(segs):
            if s[0] == "L" and abs(s[1][1] - s[2][1]) < 1e-6:
                rows.append((float(s[1][1]), abs(float(s[2][0] - s[1][0])), ci, si))
    rows.sort()
    groups: list[list] = []
    for r in rows:
        if groups and r[0] - groups[-1][-1][0] <= SNAP_ROW:
            groups[-1].append(r)
        else:
            groups.append([r])
    for g in groups:
        w = np.array([x[1] for x in g])
        target = float((np.array([x[0] for x in g]) * w).sum() / w.sum())
        for y, _, ci, si in g:
            segs = contours[ci]
            dy = target - y
            move_vertex(segs, si - 1, 0.0, dy)
            move_vertex(segs, si, 0.0, dy)
            s = segs[si]
            s[1][1] = target
            s[2][1] = target


def fmt(v: float) -> str:
    t = f"{v:.2f}".rstrip("0").rstrip(".")
    return "0" if t in ("-0", "") else t


def path_data(contours: list[list[list]]) -> str:
    parts = []
    for segs in contours:
        parts.append(f"M{fmt(segs[0][1][0])} {fmt(segs[0][1][1])}")
        for s in segs:
            if s[0] == "L":
                parts.append(f"L{fmt(s[2][0])} {fmt(s[2][1])}")
            else:
                parts.append("C" + " ".join(fmt(x) for pt in s[2:] for x in pt))
        parts.append("Z")
    return "".join(parts)


# ---- SVG writing ---------------------------------------------------------------------------------------


def mark_markup(indent: str) -> str:
    lines = [
        '<rect x="2" y="2" width="20" height="20" rx="2.2"/>',
        *[f'<line x1="{a}" y1="{b}" x2="{c}" y2="{d}"/>' for a, b, c, d in MARK_LINES],
    ]
    body = "\n".join(indent + "  " + x for x in lines)
    return (
        f'{indent}<g fill="none" stroke="url(#straft-mark-gradient)" stroke-width="1.6" '
        f'stroke-linecap="round" stroke-linejoin="round">\n{body}\n{indent}</g>'
    )


def gradient_markup(indent: str) -> str:
    stops = "\n".join(f'{indent}  <stop offset="{o:g}" stop-color="{c}"/>' for o, c in STOPS)
    return (
        f'{indent}<linearGradient id="straft-mark-gradient" gradientUnits="userSpaceOnUse" '
        f'x1="2" y1="2" x2="22" y2="22">\n{stops}\n{indent}</linearGradient>'
    )


def write_svg(tx, ty, s, straft_d, studio_d) -> None:
    w, h = Image.open(SOURCE).size
    OUT.write_text(
        f"""<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}" role="img" aria-label="Straft Studio">
  <!-- Straft Studio logo. The task named this file after the Manrope typeface, but no font file was
       supplied, so the wordmark outlines were traced from the owner's image
       (straft-logo-source.jpeg) by packaging/brand/trace_logo.py. They are paths only and need no
       font. The mark is the exact 24 by 24 grid geometry of the app icon, not a trace. Coordinates
       are source image pixels. Remove the rect with id "background" for a transparent logo. -->
  <defs>
{gradient_markup("    ")}
  </defs>
  <rect id="background" width="{w}" height="{h}" fill="{BG}"/>
  <g id="mark" transform="translate({tx:.3f} {ty:.3f}) scale({s:.5f})">
{mark_markup("    ")}
  </g>
  <path id="word-straft" fill="{STRAFT_COLOUR}" fill-rule="evenodd" d="{straft_d}"/>
  <path id="word-studio" fill="{STUDIO_COLOUR}" fill-rule="evenodd" d="{studio_d}"/>
</svg>
""",
        encoding="utf-8",
    )
    MARK_OUT.write_text(
        f"""<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 24 24" role="img" aria-label="Straft Studio mark">
  <!-- The Straft Studio film-frame mark alone, on a transparent background. Same geometry and
       gradient as apps/desktop/src-tauri/icons/source.svg. -->
  <defs>
{gradient_markup("    ")}
  </defs>
{mark_markup("  ")}
</svg>
""",
        encoding="utf-8",
    )


def main() -> None:
    rgb = np.asarray(Image.open(SOURCE).convert("RGB"), dtype=np.float64)
    luma = read_luma(SOURCE)
    tx, ty, s, rms = fit_mark(rgb)
    print(f"mark fit: translate({tx:.3f} {ty:.3f}) scale({s:.5f}), coverage rms {rms:.4f}")
    print(f"mark outer box in image px: x {tx + 1.2 * s:.2f}..{tx + 22.8 * s:.2f}, y {ty + 1.2 * s:.2f}..{ty + 22.8 * s:.2f}")

    data = {}
    for word, (x0, x1), colour in (
        ("straft", WORDS["straft"], STRAFT_COLOUR),
        ("studio", WORDS["studio"], STUDIO_COLOUR),
    ):
        raw = word_contours(luma, x0, x1, rgb_luma(hex_rgb(colour)))
        contours = [weld(trace_contour(c)) for c in raw]
        for c in contours:
            straighten_axis(c)
        snap_rows(contours)
        nl = sum(1 for c in contours for x in c if x[0] == "L")
        nc = sum(1 for c in contours for x in c if x[0] == "C")
        print(f"{word}: {len(contours)} contours, {nl} lines, {nc} cubics")
        data[word] = path_data(contours)
    write_svg(tx, ty, s, data["straft"], data["studio"])
    print(f"wrote {OUT.name}, {OUT.stat().st_size} bytes; {MARK_OUT.name}")


if __name__ == "__main__":
    main()
