"""Style linter for authored sprites.

Checks a ``.sprite.yaml`` against the measured Digimon Up envelope in
``style.py``. ERRORs are things that will read as broken at 1:1 zoom; WARNs are
style drift worth a second look.
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import ops  # noqa: E402
import style  # noqa: E402
from sprite import TRANSPARENT, Sprite, SpriteError, _hex_to_rgba, luma  # noqa: E402


def check(sp: Sprite) -> list[tuple[str, str]]:
    """Return ``[(level, message)]`` where level is ``ERROR`` or ``WARN``."""
    out: list[tuple[str, str]] = []
    err = lambda m: out.append(("ERROR", m))  # noqa: E731
    warn = lambda m: out.append(("WARN", m))  # noqa: E731

    w, h = sp.canvas
    box = sp.bbox()
    if not box:
        err("sprite is entirely transparent")
        return out
    x0, y0, x1, y1 = box
    bw, bh = x1 - x0, y1 - y0

    # --- size envelope for the printed level -----------------------------
    lo, hi = style.height_bounds(sp.level)
    if not lo <= bh <= hi:
        warn(
            f"height {bh}px is outside the Lv{sp.level} reference band {lo}-{hi}px "
            f"(median {style.STAGE_ENVELOPE.get(sp.level, (0,0,0,0))[1]}px)"
        )

    # --- framing ---------------------------------------------------------
    if y1 != h:
        warn(f"art does not reach the bottom edge ({h - y1}px gap); refs are foot-anchored")
    left, right = x0, w - x1
    if abs(left - right) > max(2, w * 0.12):
        warn(f"art is off-centre horizontally (left={left}px right={right}px)")
    if bw > w or bh > h:
        err("content exceeds the canvas")

    # --- palette ---------------------------------------------------------
    n = len(sp.palette)
    plo, phi = style.PALETTE_RANGE
    if n < plo:
        warn(f"palette has only {n} colours; refs read as {plo}-{phi} tone steps")
    if n > phi:
        warn(f"palette has {n} colours; tighten the ramp (target <= {phi})")
    dead = set(sp.palette) - sp.used_keys()
    if dead:
        warn(f"palette declares unused keys {sorted(dead)}")
    lumas = {k: luma(_hex_to_rgba(v)) for k, v in sp.palette.items()}
    if lumas:
        darkest = min(lumas.values())
        if darkest > style.DARKEST_MAX_LUMA:
            err(
                f"darkest colour has luma {darkest:.0f}; the reference keyline is "
                f"near-black (<= {style.DARKEST_MAX_LUMA:.0f})"
            )
    for a in sp.palette:
        for b in sp.palette:
            if a < b:
                ca, cb = _hex_to_rgba(sp.palette[a]), _hex_to_rgba(sp.palette[b])
                d = sum((p - q) ** 2 for p, q in zip(ca[:3], cb[:3])) ** 0.5
                if d < 10:
                    warn(f"palette {a!r} and {b!r} are near-identical (dist {d:.0f})")

    # --- the black keyline -----------------------------------------------
    edge_total = edge_dark = 0
    for y in range(h):
        for x in range(w):
            k = sp.get(x, y)
            if k == TRANSPARENT:
                continue
            touches_air = False
            for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                nx, ny = x + dx, y + dy
                if not (0 <= nx < w and 0 <= ny < h) or sp.get(nx, ny) == TRANSPARENT:
                    touches_air = True
                    break
            if touches_air:
                edge_total += 1
                if lumas.get(k, 999) <= style.OUTLINE_MAX_LUMA:
                    edge_dark += 1
    if edge_total:
        frac = edge_dark / edge_total
        if frac < 0.90:
            err(
                f"only {frac:.0%} of silhouette-edge pixels are dark; the Digimon Up "
                f"look needs a continuous keyline (run ops.outline)"
            )
        elif frac < 0.99:
            warn(f"{edge_total - edge_dark} silhouette-edge pixels are not dark")

    # --- density + stray pixels -------------------------------------------
    fill = sp.opaque_count() / (bw * bh)
    flo, fhi = style.FILL_RATIO_RANGE
    if not flo <= fill <= fhi:
        warn(f"silhouette fill {fill:.2f} outside the reference band {flo}-{fhi}")
    comps = ops.components(sp)
    strays = [c for c in comps[1:] if c < 6]
    if strays:
        warn(f"{len(strays)} tiny disconnected island(s) of <6px: {strays}")
    if len(comps) > 4:
        warn(f"{len(comps)} disconnected pieces; refs are usually 1-3")

    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("paths", nargs="+", type=Path)
    ap.add_argument("--strict", action="store_true", help="treat WARN as failure")
    args = ap.parse_args()

    files: list[Path] = []
    for p in args.paths:
        files.extend(sorted(p.glob("*.sprite.yaml")) if p.is_dir() else [p])

    bad = 0
    for f in files:
        try:
            sp = Sprite.load(f)
        except (SpriteError, KeyError) as e:
            print(f"{f.name}: ERROR {e}")
            bad += 1
            continue
        issues = check(sp)
        errs = [i for i in issues if i[0] == "ERROR"]
        warns = [i for i in issues if i[0] == "WARN"]
        status = "FAIL" if errs else ("WARN" if warns else "OK")
        w, h = sp.canvas
        print(f"{status:4s} {f.name}  {w}x{h}  {len(sp.palette)} colours")
        for level, msg in issues:
            print(f"       {level}: {msg}")
        if errs or (args.strict and warns):
            bad += 1
    print(f"\n{len(files) - bad}/{len(files)} sprites clean")
    return 1 if bad else 0


if __name__ == "__main__":
    raise SystemExit(main())
