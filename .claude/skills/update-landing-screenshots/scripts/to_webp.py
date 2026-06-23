"""Crop + resize a capture PNG and save as WebP for the landing-page gallery.

Usage: to_webp.py IN.png OUT.webp [--width 960] [--crop L T R B] [--quality 82]
--crop trims that many pixels off each edge (e.g. residual letterbox); omit for none.
"""
import argparse
from PIL import Image


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("src")
    ap.add_argument("dst")
    ap.add_argument("--width", type=int, default=960)
    ap.add_argument("--crop", type=int, nargs=4, metavar=("L", "T", "R", "B"))
    ap.add_argument("--quality", type=int, default=82)
    a = ap.parse_args()

    im = Image.open(a.src).convert("RGB")
    if a.crop:
        l, t, r, b = a.crop
        im = im.crop((l, t, im.width - r, im.height - b))
    if a.width and im.width != a.width:
        h = round(im.height * a.width / im.width)
        im = im.resize((a.width, h), Image.LANCZOS)
    im.save(a.dst, "WEBP", quality=a.quality, method=6)
    print(f"wrote {a.dst} {im.width}x{im.height}")


if __name__ == "__main__":
    main()
