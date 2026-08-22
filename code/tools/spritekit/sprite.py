"""The ``.sprite.yaml`` source format: an indexed palette plus a character grid.

A sprite is stored as text so it diffs, reviews, and hand-edits like code:

.. code-block:: yaml

    card_id: EX12-015
    name: Gokuumon
    level: 5
    canvas: [80, 84]          # width, height
    palette:
      K: "#0d0a12"            # '.' is always transparent and is implicit
      f: "#f4f0e6"
    grid: |
      ......KKKK......
      ....KKffffKK....

Every grid row must be exactly ``canvas[0]`` characters wide and there must be
exactly ``canvas[1]`` rows, so a malformed edit fails loudly instead of silently
shifting the art.
"""
from __future__ import annotations

import re
from dataclasses import dataclass, field
from pathlib import Path

import yaml
from PIL import Image

TRANSPARENT = "."
_HEX_RE = re.compile(r"^#[0-9a-fA-F]{6}$")


class SpriteError(ValueError):
    """Raised when a sprite source is structurally invalid."""


def _hex_to_rgba(h: str) -> tuple[int, int, int, int]:
    h = h.lstrip("#")
    return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16), 255)


def _rgba_to_hex(c: tuple[int, int, int, int]) -> str:
    return "#{:02x}{:02x}{:02x}".format(*c[:3])


def luma(c: tuple[int, int, int, int]) -> float:
    return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]


@dataclass
class Sprite:
    """An indexed-palette pixel sprite."""

    name: str
    canvas: tuple[int, int]
    palette: dict[str, str]
    rows: list[str]
    card_id: str | None = None
    level: int | None = None
    notes: str | None = None
    donors: list[str] = field(default_factory=list)

    # ---------------------------------------------------------------- loading

    @classmethod
    def load(cls, path: str | Path) -> "Sprite":
        data = yaml.safe_load(Path(path).read_text(encoding="utf-8"))
        canvas = tuple(data["canvas"])
        rows = data["grid"].rstrip("\n").split("\n")
        sp = cls(
            name=data["name"],
            canvas=(int(canvas[0]), int(canvas[1])),
            palette={str(k): v for k, v in (data.get("palette") or {}).items()},
            rows=rows,
            card_id=data.get("card_id"),
            level=data.get("level"),
            notes=data.get("notes"),
            donors=list(data.get("donors") or []),
        )
        sp.validate()
        return sp

    @classmethod
    def blank(cls, name: str, w: int, h: int, **kw) -> "Sprite":
        return cls(name=name, canvas=(w, h), palette={}, rows=[TRANSPARENT * w] * h, **kw)

    # ------------------------------------------------------------- validation

    def validate(self) -> None:
        w, h = self.canvas
        if len(self.rows) != h:
            raise SpriteError(f"{self.name}: grid has {len(self.rows)} rows, canvas says {h}")
        for y, row in enumerate(self.rows):
            if len(row) != w:
                raise SpriteError(
                    f"{self.name}: row {y} is {len(row)} chars, canvas says {w}"
                )
        if TRANSPARENT in self.palette:
            raise SpriteError(f"{self.name}: '{TRANSPARENT}' is reserved for transparency")
        for key, val in self.palette.items():
            if len(key) != 1:
                raise SpriteError(f"{self.name}: palette key {key!r} must be one character")
            if not _HEX_RE.match(str(val)):
                raise SpriteError(f"{self.name}: palette {key!r} = {val!r} is not #rrggbb")
        used = {c for row in self.rows for c in row} - {TRANSPARENT}
        unknown = used - set(self.palette)
        if unknown:
            raise SpriteError(
                f"{self.name}: grid uses undeclared palette keys {sorted(unknown)}"
            )

    # ----------------------------------------------------------------- pixels

    def get(self, x: int, y: int) -> str:
        return self.rows[y][x]

    def set(self, x: int, y: int, key: str) -> None:
        row = self.rows[y]
        self.rows[y] = row[:x] + key + row[x + 1 :]

    def opaque_count(self) -> int:
        return sum(1 for row in self.rows for c in row if c != TRANSPARENT)

    def bbox(self) -> tuple[int, int, int, int] | None:
        """``(x0, y0, x1, y1)`` half-open bounds of non-transparent pixels."""
        xs, ys = [], []
        for y, row in enumerate(self.rows):
            for x, c in enumerate(row):
                if c != TRANSPARENT:
                    xs.append(x)
                    ys.append(y)
        if not xs:
            return None
        return min(xs), min(ys), max(xs) + 1, max(ys) + 1

    def used_keys(self) -> set[str]:
        return {c for row in self.rows for c in row} - {TRANSPARENT}

    # ------------------------------------------------------------------- I/O

    def to_image(self) -> Image.Image:
        w, h = self.canvas
        im = Image.new("RGBA", (w, h), (0, 0, 0, 0))
        px = im.load()
        lut = {k: _hex_to_rgba(v) for k, v in self.palette.items()}
        for y, row in enumerate(self.rows):
            for x, c in enumerate(row):
                if c != TRANSPARENT:
                    px[x, y] = lut[c]
        return im

    def save_png(self, path: str | Path, scale: int = 1) -> Path:
        im = self.to_image()
        if scale > 1:
            im = im.resize((im.width * scale, im.height * scale), Image.NEAREST)
        path = Path(path)
        path.parent.mkdir(parents=True, exist_ok=True)
        im.save(path)
        return path

    def dump(self) -> str:
        lines = []
        if self.card_id:
            lines.append(f"card_id: {self.card_id}")
        lines.append(f"name: {self.name}")
        if self.level is not None:
            lines.append(f"level: {self.level}")
        if self.donors:
            lines.append("donors:")
            lines += [f"  - {d}" for d in self.donors]
        if self.notes:
            lines.append(f"notes: |-")
            lines += [f"  {ln}" for ln in self.notes.strip().split("\n")]
        lines.append(f"canvas: [{self.canvas[0]}, {self.canvas[1]}]")
        lines.append("palette:")
        # Darkest first: outline, then shadows, then mids, then highlights.
        for key in sorted(self.palette, key=lambda k: luma(_hex_to_rgba(self.palette[k]))):
            lines.append(f'  "{key}": "{self.palette[key]}"')
        lines.append("grid: |")
        lines += [f"  {row}" for row in self.rows]
        return "\n".join(lines) + "\n"

    def save(self, path: str | Path) -> Path:
        self.validate()
        path = Path(path)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(self.dump(), encoding="utf-8")
        return path


# --------------------------------------------------------------------- import


# Distinct, visually separable glyphs; ordered so the first keys are the ones a
# human will type most (outline + main body tones).
KEY_ALPHABET = "KLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+*=~!@$%&<>?/|"


def from_image(
    im: Image.Image,
    name: str,
    colors: int = 24,
    alpha_threshold: int = 128,
    **kw,
) -> Sprite:
    """Quantise a PNG into an indexed :class:`Sprite`.

    Alpha is hard-thresholded (the reference sprites are effectively binary), then
    the opaque pixels are reduced to *colors* entries with median-cut.
    """
    im = im.convert("RGBA")
    w, h = im.size
    src = im.load()

    # Composite onto magenta so quantisation never blends with transparent black,
    # then rely on the mask rather than the composited colour.
    mask = [[src[x, y][3] >= alpha_threshold for x in range(w)] for y in range(h)]
    flat = Image.new("RGB", (w, h), (255, 0, 255))
    fp = flat.load()
    for y in range(h):
        for x in range(w):
            if mask[y][x]:
                fp[x, y] = src[x, y][:3]

    q = flat.quantize(colors=max(2, colors), method=Image.MEDIANCUT, dither=Image.NONE)
    pal = q.getpalette() or []
    qp = q.load()

    idx_to_key: dict[int, str] = {}
    palette: dict[str, str] = {}
    rows: list[str] = []
    for y in range(h):
        row = []
        for x in range(w):
            if not mask[y][x]:
                row.append(TRANSPARENT)
                continue
            i = qp[x, y]
            if i not in idx_to_key:
                rgb = (pal[i * 3], pal[i * 3 + 1], pal[i * 3 + 2])
                if len(idx_to_key) >= len(KEY_ALPHABET):
                    raise SpriteError("too many colours for the key alphabet")
                key = KEY_ALPHABET[len(idx_to_key)]
                idx_to_key[i] = key
                palette[key] = _rgba_to_hex((*rgb, 255))
            row.append(idx_to_key[i])
        rows.append("".join(row))

    sp = Sprite(name=name, canvas=(w, h), palette=palette, rows=rows, **kw)
    sp.relabel_by_luma()
    sp.validate()
    return sp


def _remap(sp: Sprite, mapping: dict[str, str]) -> None:
    trans = str.maketrans(mapping)
    sp.rows = [row.translate(trans) for row in sp.rows]
    sp.palette = {mapping.get(k, k): v for k, v in sp.palette.items()}


def _relabel(sp: Sprite, order: list[str]) -> None:
    """Assign ``KEY_ALPHABET`` glyphs to palette entries in the given order."""
    mapping = {old: KEY_ALPHABET[i] for i, old in enumerate(order)}
    # Two-step through a private range so overlapping old/new keys can't collide.
    tmp = {old: chr(0xE000 + i) for i, old in enumerate(order)}
    _remap(sp, tmp)
    _remap(sp, {tmp[old]: mapping[old] for old in order})


def _relabel_by_luma(self: Sprite) -> None:
    """Rename palette keys darkest-to-lightest so ``K`` is always the outline."""
    order = sorted(self.palette, key=lambda k: luma(_hex_to_rgba(self.palette[k])))
    _relabel(self, order)


Sprite.relabel_by_luma = _relabel_by_luma  # type: ignore[attr-defined]
