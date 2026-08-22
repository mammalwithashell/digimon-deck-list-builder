# Authored sprites

Digimon Up-style pixel sprites for Digimon TCG cards. Each card has two files:

- `<CARD_ID>_<Name>.sprite.yaml` — **the source**: an indexed palette plus a
  character grid, one character per pixel. Hand-editable, diffable, and what
  `lint.py` checks.
- `<CARD_ID>_<Name>.png` — the 1:1 render, regenerated from the source with
  `render.py png`.

Tooling lives in `code/tools/spritekit/` (see its README). Author with the
`/digimon-sprite-author` skill; find style references and donors with
`/digimon-sprite-refs`.

## Current coverage — EX12 Seiyo Warriors (`SW`)

The Gokuumon line, authored as the pilot for the pipeline:

| Card | Digimon | Lv | Canvas | Colours | Donor |
|---|---|---:|---|---:|---|
| EX12-006 | Kakamon | 3 | 58×58 | 16 | Gazimon (small-beast armature) |
| EX12-012 | Apemon | 4 | 62×76 | 15 | Leomon (beastman armature) |
| EX12-015 | Gokuumon | 5 | 80×88 | 19 | Etemon (ape stance/limbs) |
| EX12-048 | SeitenGokuumon | 6 | 90×96 | 15 | **this line's own Lv5 sprite** |

All four pass `python code/tools/spritekit/lint.py data/sprites/` with no
errors or warnings.

Canvas height increases monotonically along the line (58 → 76 → 88 → 96), which
is how the reference set signals evolution stage — see the style guide §1.

The Mega is derived from the line's own Ultimate rather than from an external
donor. That is deliberate: it is what makes Gokuumon and SeitenGokuumon read as
the same creature evolved instead of two unrelated apes.

## Regenerating

The `.sprite.yaml` is the source of truth and is fully self-contained — edit it
directly and re-render:

```bash
python code/tools/spritekit/render.py png data/sprites/EX12-015_Gokuumon.sprite.yaml
python code/tools/spritekit/lint.py data/sprites/
```

To review a set at a glance:

```bash
python code/tools/spritekit/render.py sheet data/sprites/*.sprite.yaml \
  -o /tmp/line.png --scale 4 --cols 5
```
