"""Measure the Digimon Up reference sprites so the style sheet is derived, not guessed.

Reports, per evolution level (joined from ``data/cards.json`` by Digimon name):
canvas footprint, palette size, alpha discipline, outline darkness/coverage.
"""
from __future__ import annotations

import argparse
import collections
import json
import statistics
import sys
from pathlib import Path

from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parent))
import refs  # noqa: E402

# The reference dump uses game-internal names; a handful differ from card names.
NAME_FIXUPS = {"Demiveemon": "DemiVeemon", "Exveemon": "ExVeemon"}


def card_levels() -> dict[str, int]:
    """Lowercased Digimon name -> the most common printed level for that name."""
    cards = json.loads((refs.repo_root() / "data" / "cards.json").read_text("utf-8"))
    per: dict[str, list[int]] = collections.defaultdict(list)
    for c in cards.values():
        lvl, name = c.get("level"), c.get("card_name_eng")
        if lvl and name and c.get("card_kind") == 0:
            per[name.lower()].append(int(lvl))
    return {k: collections.Counter(v).most_common(1)[0][0] for k, v in per.items()}


def measure(path: Path) -> dict:
    im = Image.open(path).convert("RGBA")
    w, h = im.size
    px = list(im.get_flattened_data())
    solid = [p for p in px if p[3] == 255]
    partial = [p for p in px if 0 < p[3] < 255]
    if not solid:
        return {}
    # Tight bounding box of visible pixels.
    bbox = im.getbbox() or (0, 0, w, h)
    bw, bh = bbox[2] - bbox[0], bbox[3] - bbox[1]
    lum = [0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2] for p in solid]
    dark = [l for l in lum if l < 60]
    return {
        "w": w,
        "h": h,
        "bw": bw,
        "bh": bh,
        "opaque": len(solid),
        "colors": len(set(solid)),
        "partial_alpha": len(partial),
        "fill": len(solid) / (bw * bh) if bw and bh else 0.0,
        "dark_frac": len(dark) / len(solid),
        "min_lum": min(lum),
    }


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--json-out", type=Path)
    args = ap.parse_args()

    levels = card_levels()
    rows = []
    for rec in refs.find(kind="ui_sprite"):
        p = refs.cache_dir() / f"{rec['folder']}__{rec['title']}"
        if not p.exists():
            continue
        subject = rec["subject"].split("#")[0].strip()
        base = subject.removeprefix("Child_").rstrip("_0123456789").strip("_")
        base = NAME_FIXUPS.get(base, base)
        m = measure(p)
        if not m:
            continue
        m.update(subject=subject, level=levels.get(base.lower()))
        rows.append(m)

    print(f"measured {len(rows)} reference sprites "
          f"({sum(1 for r in rows if r['level'])} matched to a card level)\n")

    print("=== canvas footprint by printed level ===")
    print(f"{'lvl':>4} {'n':>4} {'height p10/med/p90':>22} {'width med':>10} {'colors med':>11}")
    by = collections.defaultdict(list)
    for r in rows:
        if r["level"]:
            by[r["level"]].append(r)
    for lvl in sorted(by):
        g = by[lvl]
        hs = sorted(r["bh"] for r in g)
        ws = sorted(r["bw"] for r in g)
        cs = sorted(r["colors"] for r in g)
        q = lambda a, f: a[min(len(a) - 1, int(len(a) * f))]  # noqa: E731
        print(f"{lvl:>4} {len(g):>4} {q(hs,.1):>7}/{statistics.median(hs):>6.0f}/{q(hs,.9):>6} "
              f"{statistics.median(ws):>10.0f} {statistics.median(cs):>11.0f}")

    print("\n=== alpha + outline discipline (all sprites) ===")
    pa = [r["partial_alpha"] for r in rows]
    print(f"  partial-alpha pixels per sprite: median={statistics.median(pa):.0f} "
          f"max={max(pa)}  ({sum(1 for x in pa if x == 0)}/{len(pa)} are fully binary)")
    df = [r["dark_frac"] for r in rows]
    print(f"  fraction of opaque pixels with luma<60 (outline+lines): "
          f"median={statistics.median(df):.3f} p10={sorted(df)[len(df)//10]:.3f} "
          f"p90={sorted(df)[9*len(df)//10]:.3f}")
    ml = [r["min_lum"] for r in rows]
    print(f"  darkest pixel luma: median={statistics.median(ml):.1f} max={max(ml):.1f}")
    fl = [r["fill"] for r in rows]
    print(f"  silhouette fill (opaque / bbox area): median={statistics.median(fl):.2f}")
    cpp = [r["colors"] / r["opaque"] for r in rows]
    print(f"  distinct colors per opaque pixel: median={statistics.median(cpp):.3f}")

    if args.json_out:
        args.json_out.parent.mkdir(parents=True, exist_ok=True)
        args.json_out.write_text(json.dumps(rows, indent=1), encoding="utf-8")
        print(f"\nwrote {args.json_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
