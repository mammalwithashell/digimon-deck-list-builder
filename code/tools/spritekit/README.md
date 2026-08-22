# spritekit

Tooling for authoring **Digimon Up-style pixel sprites** for Digimon TCG cards.

Driven by two skills — `/digimon-sprite-refs` (find style + donors) and
`/digimon-sprite-author` (build the sprite). Read those first; this file is the
module-level reference.

## The idea

There is no image-generation model in this pipeline. A sprite is **authored as
data** — an indexed palette plus a character grid — and rendered
deterministically. That makes the source diffable, reviewable and re-editable
like code, and it means every shape decision is recorded rather than baked into
pixels.

The leverage comes from **donors**: a real Digimon Up sprite with the right body
plan supplies proportions, keyline weight, stance and shading for free, so
authoring is re-skin-and-reshape rather than draw-from-nothing.

## Modules

| file | what it does |
|---|---|
| `drive_index.py` | Index the public "Digimon Up Assets" Drive folder → `data/sprite_refs/index.json` (6,311 assets) |
| `refs.py` | Query the index, lazily fetch PNGs into `.cache/sprite_refs/`, filter unusable records |
| `analyze.py` | Measure the reference library → the numbers behind `style.py` |
| `style.py` | The measured style envelope (canvas per level, outline/palette/fill bands) |
| `pick_refs.py` | Rank donors for a target card; `--search` by name; `--sheet` to compare by eye |
| `sprite.py` | The `.sprite.yaml` format: load/save/validate, PNG import (quantise) and export |
| `ops.py` | Palette + geometry ops: ramps, `map_to_ramp`, merge, fit/crop, flood fill, `outline` |
| `draw.py` | Drawing primitives: line, rect, ellipse, `spiky_ring` (manes/crests), box swap/erase |
| `grid.py` | Show the grid with coordinate rulers; stamp a hand-authored patch block |
| `lint.py` | Enforce the style envelope; ERROR = will read as broken at 1:1 |
| `render.py` | CLI: render PNG, import a donor, contact sheets, sprite-vs-reference strips |

## Quick start

```bash
SK=code/tools/spritekit

python $SK/drive_index.py                                  # once: build the index
python $SK/pick_refs.py EX12-015 --top 8 --sheet /tmp/d.png # pick a donor (then look at it)

python $SK/render.py import .cache/sprite_refs/Sprite__UI_Enemy_Etemon.png \
  --name Gokuumon --card-id EX12-015 --level 5 --colors 18 --merge 16 --fit 80x88 \
  -o data/sprites/EX12-015_Gokuumon.sprite.yaml

# ... edit via ops/draw/grid ...

python $SK/lint.py data/sprites/
python $SK/render.py png data/sprites/EX12-015_Gokuumon.sprite.yaml --scale 6 -o /tmp/look.png
```

## Output

- `data/sprites/<CARD_ID>_<Name>.sprite.yaml` — the editable source (committed)
- `data/sprites/<CARD_ID>_<Name>.png` — 1:1 render (committed)
- `.cache/sprite_refs/` — downloaded third-party reference art (gitignored)

## Notes

- The asset folder is link-shared, so the Drive API/connector **cannot** list its
  children. `drive_index.py` parses the public `embeddedfolderview` HTML, which
  returns the whole folder in one request. That is the fragile part; if indexing
  ever breaks, look there first.
- `merge_similar`, `drop_unused` and `map_to_ramp` all relabel palette keys
  darkest-to-lightest. Any key letters you were tracking change — re-run
  `grid.py show` after calling them.
- `ops.outline` consumes 1px of the body all round, so build shapes 1px larger
  than the intended read, and give `draw.spiky_ring` `tip>=2` or its spikes
  disappear into the keyline entirely.
