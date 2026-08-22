"""Render, preview and compare sprites.

Subcommands
-----------
``png``      ``.sprite.yaml`` -> PNG (1:1 for shipping, ``--scale`` for review)
``import``   PNG -> ``.sprite.yaml`` (quantise a donor reference into a grid)
``sheet``    contact sheet of several sprites/PNGs, labelled, for one-glance review
``strip``    one sprite beside its donors and the card art, for A/B review
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

from PIL import Image, ImageDraw

sys.path.insert(0, str(Path(__file__).resolve().parent))

import ops  # noqa: E402
from sprite import Sprite, from_image  # noqa: E402

BG = (24, 24, 28, 255)
FG = (232, 232, 236, 255)


def _load_any(path: Path, scale: int = 1) -> Image.Image:
    """Load a ``.sprite.yaml`` or an image file as RGBA, nearest-upscaled."""
    if path.name.endswith(".sprite.yaml"):
        im = Sprite.load(path).to_image()
    else:
        im = Image.open(path).convert("RGBA")
    if scale > 1:
        im = im.resize((im.width * scale, im.height * scale), Image.NEAREST)
    return im


def cmd_png(args) -> int:
    sp = Sprite.load(args.input)
    out = args.output or Path(str(args.input).replace(".sprite.yaml", ".png"))
    sp.save_png(out, scale=args.scale)
    print(f"{out}  {sp.canvas[0]*args.scale}x{sp.canvas[1]*args.scale}")
    return 0


def cmd_import(args) -> int:
    im = Image.open(args.input)
    sp = from_image(
        im,
        name=args.name or Path(args.input).stem,
        colors=args.colors,
        card_id=args.card_id,
        level=args.level,
        donors=[Path(args.input).name],
    )
    if args.merge:
        removed = ops.merge_similar(sp, threshold=args.merge)
        print(f"merged {removed} near-identical tones")
    ops.drop_unused(sp)
    if args.fit:
        w, h = (int(v) for v in args.fit.split("x"))
        ops.fit_canvas(sp, w, h)
    out = args.output or Path(f"{sp.name}.sprite.yaml")
    sp.save(out)
    print(f"{out}  {sp.canvas[0]}x{sp.canvas[1]}  {len(sp.palette)} colours")
    return 0


def _sheet(items: list[tuple[str, Image.Image]], cols: int, pad: int = 12) -> Image.Image:
    if not items:
        raise SystemExit("nothing to draw")
    cw = max(im.width for _, im in items) + pad * 2
    ch = max(im.height for _, im in items) + pad * 2 + 14
    rows = (len(items) + cols - 1) // cols
    sheet = Image.new("RGBA", (cw * min(cols, len(items)), ch * rows), BG)
    d = ImageDraw.Draw(sheet)
    for i, (label, im) in enumerate(items):
        cx, cy = (i % cols) * cw, (i // cols) * ch
        # Bottom-aligned so tiles of different heights share a ground line and
        # each caption sits under its own art.
        top = cy + ch - 16 - im.height
        sheet.alpha_composite(im, (cx + (cw - im.width) // 2, top))
        d.text((cx + 4, cy + ch - 13), label[:34], fill=FG)
    return sheet


def cmd_sheet(args) -> int:
    items = [(p.name.replace(".sprite.yaml", ""), _load_any(p, args.scale)) for p in args.inputs]
    sheet = _sheet(items, cols=args.cols)
    sheet.save(args.output)
    print(f"{args.output}  {sheet.width}x{sheet.height}  ({len(items)} tiles)")
    return 0


def cmd_strip(args) -> int:
    items = [("AUTHORED " + Path(args.sprite).stem, _load_any(Path(args.sprite), args.scale))]
    for r in args.refs:
        items.append(("ref " + Path(r).stem[:28], _load_any(Path(r), args.scale)))
    sheet = _sheet(items, cols=len(items))
    sheet.save(args.output)
    print(f"{args.output}  {sheet.width}x{sheet.height}")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)

    p = sub.add_parser("png", help="render a sprite to PNG")
    p.add_argument("input", type=Path)
    p.add_argument("-o", "--output", type=Path)
    p.add_argument("--scale", type=int, default=1)
    p.set_defaults(fn=cmd_png)

    p = sub.add_parser("import", help="quantise a PNG into a sprite grid")
    p.add_argument("input", type=Path)
    p.add_argument("-o", "--output", type=Path)
    p.add_argument("--name")
    p.add_argument("--card-id")
    p.add_argument("--level", type=int)
    p.add_argument("--colors", type=int, default=20)
    p.add_argument("--merge", type=int, default=0,
                   help="collapse tones closer than this RGB distance (try 18)")
    p.add_argument("--fit", help="re-frame onto WxH, foot-anchored (e.g. 79x82)")
    p.set_defaults(fn=cmd_import)

    p = sub.add_parser("sheet", help="labelled contact sheet")
    p.add_argument("inputs", nargs="+", type=Path)
    p.add_argument("-o", "--output", type=Path, required=True)
    p.add_argument("--scale", type=int, default=4)
    p.add_argument("--cols", type=int, default=5)
    p.set_defaults(fn=cmd_sheet)

    p = sub.add_parser("strip", help="one sprite beside its references")
    p.add_argument("sprite", type=Path)
    p.add_argument("refs", nargs="*", type=Path)
    p.add_argument("-o", "--output", type=Path, required=True)
    p.add_argument("--scale", type=int, default=5)
    p.set_defaults(fn=cmd_strip)

    args = ap.parse_args()
    return args.fn(args)


if __name__ == "__main__":
    raise SystemExit(main())
