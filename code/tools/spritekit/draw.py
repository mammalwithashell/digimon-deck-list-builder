"""Drawing primitives for sprite surgery.

Big silhouette changes — a staff, a mane, a horn, a robe hem — are far easier to
control parametrically than by hand-typing grid rows, and they stay re-editable
because the call that produced them is the record of what was drawn.
"""
from __future__ import annotations

import math

from sprite import TRANSPARENT, Sprite


def _put(sp: Sprite, x: int, y: int, key: str, over: str | None = None) -> None:
    """Set a pixel if in bounds. *over* restricts writing to that existing key."""
    w, h = sp.canvas
    if not (0 <= x < w and 0 <= y < h):
        return
    if over is not None and sp.get(x, y) != over:
        return
    sp.set(x, y, key)


def line(sp: Sprite, x0, y0, x1, y1, key: str, width: int = 1, over=None) -> None:
    """Bresenham line, *width* px thick (grown horizontally)."""
    dx, dy = abs(x1 - x0), -abs(y1 - y0)
    sx, sy = (1 if x0 < x1 else -1), (1 if y0 < y1 else -1)
    err = dx + dy
    while True:
        for i in range(width):
            _put(sp, x0 + i - width // 2, y0, key, over)
        if x0 == x1 and y0 == y1:
            break
        e2 = 2 * err
        if e2 >= dy:
            err += dy
            x0 += sx
        if e2 <= dx:
            err += dx
            y0 += sy


def rect(sp: Sprite, x0, y0, x1, y1, key: str, fill=True, over=None) -> None:
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            if fill or y in (y0, y1) or x in (x0, x1):
                _put(sp, x, y, key, over)


def ellipse(sp: Sprite, cx, cy, rx, ry, key: str, fill=True, over=None) -> None:
    for y in range(cy - ry, cy + ry + 1):
        for x in range(cx - rx, cx + rx + 1):
            d = ((x - cx) / max(rx, 0.5)) ** 2 + ((y - cy) / max(ry, 0.5)) ** 2
            if d <= 1.0 and (fill or d > 0.45):
                _put(sp, x, y, key, over)


def spiky_ring(
    sp: Sprite,
    cx: int,
    cy: int,
    rx: int,
    ry: int,
    spikes: list[tuple[float, int]],
    key: str,
    taper: float = 0.45,
    tip: int = 1,
) -> None:
    """A mane/crest: tapered wedges radiating from an ellipse.

    *spikes* is ``[(angle_degrees, length_px), ...]`` measured clockwise from
    straight up, so the shape stays hand-tuned rather than procedurally regular
    — which is what keeps it from looking like a gear.

    *tip* is the wedge's half-width at its point. Leave it at 1 only for hair
    that should read as wispy: a 1px tip is entirely consumed by
    ``ops.outline`` and the spike renders as a black needle, so fur and crests
    want ``tip=2`` or more.
    """
    for ang, length in spikes:
        a = math.radians(ang)
        ux, uy = math.sin(a), -math.cos(a)     # outward unit vector
        px, py = math.cos(a), math.sin(a)      # perpendicular to it
        bx, by = cx + rx * ux, cy + ry * uy
        base_half = max(float(tip), length * taper)
        # Half-pixel steps along the axis: whole-pixel steps leave diagonal
        # gaps, which ops.outline then blackens into a checkerboard.
        for step in range(2 * length + 1):
            t = step / 2
            f = t / max(1, length)
            half = base_half * (1 - f) + tip * f
            ex, ey = bx + ux * t, by + uy * t
            for i in range(-round(half * 2), round(half * 2) + 1):
                _put(sp, round(ex + px * i / 2), round(ey + py * i / 2), key)


def fill_region(sp: Sprite, seeds: list[tuple[int, int]], key: str) -> int:
    """Flood-fill several seed points (a region split by interior lines)."""
    from ops import flood_fill

    return sum(flood_fill(sp, x, y, key) for x, y in seeds)


def swap_in_box(sp: Sprite, x0, y0, x1, y1, mapping: dict[str, str]) -> int:
    """Remap palette keys, but only inside a rectangle.

    The workhorse for "make the torso a robe without touching the arms"."""
    n = 0
    for y in range(max(0, y0), min(sp.canvas[1], y1 + 1)):
        row = list(sp.rows[y])
        for x in range(max(0, x0), min(sp.canvas[0], x1 + 1)):
            if row[x] in mapping:
                row[x] = mapping[row[x]]
                n += 1
        sp.rows[y] = "".join(row)
    return n


def erase_box(sp: Sprite, x0, y0, x1, y1, only: set[str] | None = None) -> int:
    """Clear a rectangle to transparency (optionally only certain keys)."""
    n = 0
    for y in range(max(0, y0), min(sp.canvas[1], y1 + 1)):
        row = list(sp.rows[y])
        for x in range(max(0, x0), min(sp.canvas[0], x1 + 1)):
            if row[x] != TRANSPARENT and (only is None or row[x] in only):
                row[x] = TRANSPARENT
                n += 1
        sp.rows[y] = "".join(row)
    return n
