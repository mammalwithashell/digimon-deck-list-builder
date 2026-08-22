"""Inspect and surgically patch a sprite's character grid.

Most edits should go through ``ops.py``. This is for the pixels that make a
Digimon *that* Digimon — a staff, a headband, an ear — where you need to see
coordinates and rewrite a rectangle by hand.
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from sprite import TRANSPARENT, Sprite, _hex_to_rgba, luma  # noqa: E402


def show(sp: Sprite, x0=0, y0=0, x1=None, y1=None) -> str:
    """The grid with coordinate rulers, croppable to a region."""
    w, h = sp.canvas
    x1 = w if x1 is None else min(x1, w)
    y1 = h if y1 is None else min(y1, h)
    out = []
    # Column ruler: tens digit then units.
    out.append("     " + "".join(str((x // 10) % 10) if x % 5 == 0 else " " for x in range(x0, x1)))
    out.append("     " + "".join(str(x % 10) for x in range(x0, x1)))
    for y in range(y0, y1):
        out.append(f"{y:4d} {sp.rows[y][x0:x1]}")
    return "\n".join(out)


def legend(sp: Sprite) -> str:
    rows = []
    counts = {k: sum(r.count(k) for r in sp.rows) for k in sp.palette}
    for k in sorted(sp.palette, key=lambda k: luma(_hex_to_rgba(sp.palette[k]))):
        rows.append(f"  {k}  {sp.palette[k]}  luma {luma(_hex_to_rgba(sp.palette[k])):5.1f}"
                    f"  {counts[k]:5d}px")
    return "\n".join(rows)


def patch(sp: Sprite, x: int, y: int, block: list[str]) -> int:
    """Stamp *block* at (x, y). ``-`` in the block means "leave this pixel"."""
    w, h = sp.canvas
    n = 0
    for dy, line in enumerate(block):
        for dx, ch in enumerate(line):
            if ch == "-":
                continue
            px, py = x + dx, y + dy
            if not (0 <= px < w and 0 <= py < h):
                raise SystemExit(f"patch pixel ({px},{py}) is outside the {w}x{h} canvas")
            if ch != TRANSPARENT and ch not in sp.palette:
                raise SystemExit(f"patch uses undeclared palette key {ch!r}")
            sp.set(px, py, ch)
            n += 1
    return n


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    sub = ap.add_subparsers(dest="cmd", required=True)

    p = sub.add_parser("show", help="print the grid with rulers + palette legend")
    p.add_argument("sprite", type=Path)
    p.add_argument("--region", help="x0,y0,x1,y1")

    p = sub.add_parser("patch", help="stamp a block of rows at (x, y)")
    p.add_argument("sprite", type=Path)
    p.add_argument("--at", required=True, help="x,y")
    p.add_argument("--block", type=Path, required=True,
                   help="text file of grid rows; '-' keeps the existing pixel")

    args = ap.parse_args()
    sp = Sprite.load(args.sprite)

    if args.cmd == "show":
        box = [int(v) for v in args.region.split(",")] if args.region else [0, 0, None, None]
        print(show(sp, *box))
        print("\npalette:")
        print(legend(sp))
        return 0

    x, y = (int(v) for v in args.at.split(","))
    block = args.block.read_text(encoding="utf-8").rstrip("\n").split("\n")
    n = patch(sp, x, y, block)
    sp.save(args.sprite)
    print(f"patched {n} pixels at ({x},{y}) in {args.sprite.name}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
