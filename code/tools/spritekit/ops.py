"""Editing operations on a :class:`~sprite.Sprite`.

Hand-typing an 80x84 character grid is slow and error-prone, so most authoring
edits should run through these primitives; only the pixels that make a Digimon
*that* Digimon (silhouette, gear, face) get touched by hand.
"""
from __future__ import annotations

import colorsys
from collections import deque

from sprite import TRANSPARENT, Sprite, SpriteError, _hex_to_rgba, _rgba_to_hex, luma


# ------------------------------------------------------------------- colour


def shift_hue(hex_color: str, hue_deg: float, sat_mul: float = 1.0, val_mul: float = 1.0) -> str:
    r, g, b, _ = _hex_to_rgba(hex_color)
    h, s, v = colorsys.rgb_to_hsv(r / 255, g / 255, b / 255)
    h = (h + hue_deg / 360.0) % 1.0
    s = min(1.0, s * sat_mul)
    v = min(1.0, v * val_mul)
    r, g, b = colorsys.hsv_to_rgb(h, s, v)
    return _rgba_to_hex((round(r * 255), round(g * 255), round(b * 255), 255))


def recolor(sp: Sprite, mapping: dict[str, str]) -> None:
    """Point palette keys at new hex colours in place (``{"a": "#ffffff"}``)."""
    for key, hexcol in mapping.items():
        if key not in sp.palette:
            raise SpriteError(f"{sp.name}: no palette key {key!r} to recolour")
        sp.palette[key] = hexcol


def restyle(sp: Sprite, keys: list[str], hue_deg: float, sat_mul=1.0, val_mul=1.0) -> None:
    """Hue-rotate a whole material ramp at once, preserving its tone steps."""
    recolor(sp, {k: shift_hue(sp.palette[k], hue_deg, sat_mul, val_mul) for k in keys})


def ramp(base: str, steps: int = 4, spread: float = 0.34) -> list[str]:
    """A cel-shading ramp around *base*, dark -> light.

    Shadows rotate slightly warm-to-dark and gain saturation, highlights lose it —
    the behaviour the Digimon Up references show, rather than a flat multiply.
    """
    r, g, b, _ = _hex_to_rgba(base)
    h, s, v = colorsys.rgb_to_hsv(r / 255, g / 255, b / 255)
    out = []
    for i in range(steps):
        t = (i / max(1, steps - 1)) - 0.5  # -0.5 (shadow) .. +0.5 (highlight)
        vv = max(0.05, min(1.0, v * (1 + spread * 2 * t)))
        ss = max(0.0, min(1.0, s * (1 - 0.45 * t)))
        hh = (h + (-0.02 if t < 0 else 0.015) * abs(t) * 2) % 1.0
        rr, gg, bb = colorsys.hsv_to_rgb(hh, ss, vv)
        out.append(_rgba_to_hex((round(rr * 255), round(gg * 255), round(bb * 255), 255)))
    return out


def merge_similar(sp: Sprite, threshold: int = 18) -> int:
    """Collapse palette entries closer than *threshold* in RGB distance.

    Ripped assets carry hundreds of near-identical tones; authored sprites want a
    tight ramp. Returns the number of entries removed.
    """
    keys = sorted(sp.palette, key=lambda k: luma(_hex_to_rgba(sp.palette[k])))
    canonical: dict[str, str] = {}
    kept: list[str] = []
    for k in keys:
        c = _hex_to_rgba(sp.palette[k])
        for kk in kept:
            cc = _hex_to_rgba(sp.palette[kk])
            if sum((a - b) ** 2 for a, b in zip(c[:3], cc[:3])) ** 0.5 < threshold:
                canonical[k] = kk
                break
        else:
            kept.append(k)
            canonical[k] = k
    removed = len(keys) - len(kept)
    if removed:
        sp.rows = [row.translate(str.maketrans(canonical)) for row in sp.rows]
        sp.palette = {k: sp.palette[k] for k in kept}
        sp.relabel_by_luma()
    return removed


def drop_unused(sp: Sprite) -> int:
    used = sp.used_keys()
    dead = set(sp.palette) - used
    for k in dead:
        del sp.palette[k]
    if dead:
        sp.relabel_by_luma()
    return len(dead)


# ---------------------------------------------------------------- geometry


def flip_h(sp: Sprite) -> None:
    sp.rows = [row[::-1] for row in sp.rows]


def pad(sp: Sprite, left=0, right=0, top=0, bottom=0) -> None:
    w, h = sp.canvas
    nw = w + left + right
    sp.rows = (
        [TRANSPARENT * nw] * top
        + [TRANSPARENT * left + row + TRANSPARENT * right for row in sp.rows]
        + [TRANSPARENT * nw] * bottom
    )
    sp.canvas = (nw, h + top + bottom)


def crop_to_content(sp: Sprite, margin: int = 0) -> None:
    box = sp.bbox()
    if not box:
        return
    x0, y0, x1, y1 = box
    w, h = sp.canvas
    x0, y0 = max(0, x0 - margin), max(0, y0 - margin)
    x1, y1 = min(w, x1 + margin), min(h, y1 + margin)
    sp.rows = [row[x0:x1] for row in sp.rows[y0:y1]]
    sp.canvas = (x1 - x0, y1 - y0)


def fit_canvas(sp: Sprite, w: int, h: int, anchor: str = "bottom-center") -> None:
    """Re-frame onto a ``w x h`` canvas, keeping the art anchored (default: feet
    on the bottom edge, horizontally centred) — the Digimon Up convention."""
    crop_to_content(sp)
    cw, ch = sp.canvas
    if cw > w or ch > h:
        raise SpriteError(f"{sp.name}: content {cw}x{ch} does not fit in {w}x{h}")
    left = (w - cw) // 2
    top = h - ch if anchor.startswith("bottom") else (h - ch) // 2
    pad(sp, left=left, right=w - cw - left, top=top, bottom=h - ch - top)


# ----------------------------------------------------------------- painting


def replace_key(sp: Sprite, old: str, new: str) -> int:
    n = sum(row.count(old) for row in sp.rows)
    sp.rows = [row.replace(old, new) for row in sp.rows]
    return n


def flood_fill(sp: Sprite, x: int, y: int, new: str) -> int:
    """4-connected fill of the contiguous same-key region containing (x, y)."""
    target = sp.get(x, y)
    if target == new:
        return 0
    w, h = sp.canvas
    q, seen, n = deque([(x, y)]), {(x, y)}, 0
    while q:
        cx, cy = q.popleft()
        if sp.get(cx, cy) != target:
            continue
        sp.set(cx, cy, new)
        n += 1
        for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            nx, ny = cx + dx, cy + dy
            if 0 <= nx < w and 0 <= ny < h and (nx, ny) not in seen:
                seen.add((nx, ny))
                q.append((nx, ny))
    return n


def outline(sp: Sprite, key: str = "K", color: str = "#0d0a12") -> int:
    """Force a 1px dark border on every silhouette edge pixel.

    Interior pixels that touch transparency are converted, which is exactly the
    reference convention: the black keyline reads the shape at 1:1 zoom.
    """
    sp.palette.setdefault(key, color)
    w, h = sp.canvas
    edge = []
    for y in range(h):
        for x in range(w):
            if sp.get(x, y) == TRANSPARENT:
                continue
            for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                nx, ny = x + dx, y + dy
                if not (0 <= nx < w and 0 <= ny < h) or sp.get(nx, ny) == TRANSPARENT:
                    edge.append((x, y))
                    break
    for x, y in edge:
        sp.set(x, y, key)
    return len(edge)


def components(sp: Sprite) -> list[int]:
    """Sizes of 8-connected opaque islands, largest first (loose-pixel check)."""
    w, h = sp.canvas
    seen: set[tuple[int, int]] = set()
    sizes = []
    for y in range(h):
        for x in range(w):
            if sp.get(x, y) == TRANSPARENT or (x, y) in seen:
                continue
            q, n = deque([(x, y)]), 0
            seen.add((x, y))
            while q:
                cx, cy = q.popleft()
                n += 1
                for dx in (-1, 0, 1):
                    for dy in (-1, 0, 1):
                        nx, ny = cx + dx, cy + dy
                        if (
                            0 <= nx < w
                            and 0 <= ny < h
                            and (nx, ny) not in seen
                            and sp.get(nx, ny) != TRANSPARENT
                        ):
                            seen.add((nx, ny))
                            q.append((nx, ny))
            sizes.append(n)
    return sorted(sizes, reverse=True)


def map_to_ramp(sp: Sprite, base: str, steps: int = 5, keys: list[str] | None = None,
                spread: float = 0.45) -> dict[str, str]:
    """Re-point a donor's whole tonal structure onto one new material ramp.

    Palette entries are ranked by luma and distributed across a fresh
    :func:`ramp`, so the donor's light-and-shadow reading survives while its hue
    is replaced. This is the core "make this Digimon a different creature"
    move; follow it with local repaints for face, gear and markings.

    Returns the ``{old_key: new_key}`` mapping actually applied.
    """
    src = keys if keys is not None else [k for k in sp.palette if k != "K"]
    src = sorted(src, key=lambda k: luma(_hex_to_rgba(sp.palette[k])))
    if not src:
        return {}
    cols = ramp(base, steps, spread=spread)
    free = [c for c in "abcdefghijnopqtuvwyz0123456789" if c not in sp.palette]
    new_keys = []
    for c in cols:
        k = free.pop(0)
        sp.palette[k] = c
        new_keys.append(k)
    mapping = {}
    for i, k in enumerate(src):
        # rank -> ramp step, preserving relative ordering
        mapping[k] = new_keys[min(steps - 1, i * steps // len(src))]
    sp.rows = [row.translate(str.maketrans(mapping)) for row in sp.rows]
    for k in src:
        sp.palette.pop(k, None)
    return mapping
