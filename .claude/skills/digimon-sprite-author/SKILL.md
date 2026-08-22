---
name: digimon-sprite-author
description: Author Digimon-Up-style pixel sprites for Digimon TCG cards — one card, an evolution line, or a whole archetype. Produces a diffable .sprite.yaml source plus a rendered PNG, built from a donor reference and checked against the measured style envelope. Use when asked to make/draw/generate sprite art, pixel art, or board avatars for Digimon cards, to restyle an existing sprite, or to give an archetype sprite coverage.
---

# Authoring Digimon Up sprites

Output per card: `data/sprites/<CARD_ID>_<Name>.sprite.yaml` (indexed palette +
character grid, the editable source) and a 1:1 `.png` rendered from it.

There is **no image-generation model in this loop.** Sprites are authored as
data and rendered deterministically, which is why the source is a text grid:
it diffs, reviews, and re-edits like code. The leverage comes from starting on
a *donor* — a real Digimon Up sprite with the right body plan — so proportions,
keyline weight and shading ramp are inherited rather than invented.

## Worked examples

`examples/` next to this file holds the four builds of the EX12 Seiyo Warriors
Gokuumon line (Lv3 → Lv6), with a README of the mistakes they encode — draw
order, why rectangles can't follow a silhouette, why manes need wide gaps, and
what `ops.outline` eats. **Read `examples/README.md` before your first build.**

## Before you start

1. Load **`digimon-sprite-refs`** and read its `style-guide.md`. Do not skip
   this; the whole style is seven measurable properties and lint checks them.
2. Know the subject. Read the card art
   (`python .claude/skills/digimon-card-lookup/resolve_cards.py <ID>`) and, for
   a Digimon that already exists in the game, its atlas `Texture2D/<Name>.png`.
3. If you were given an **evolution line or archetype, plan it as a set first**
   (see §Lines below) before authoring any single card.

## The loop

Everything runs from the repo root. `SK=code/tools/spritekit`.

### A. Armature — import a donor

```bash
python $SK/pick_refs.py EX12-015 --top 8 --sheet /tmp/donors.png   # then Read it
python $SK/render.py import .cache/sprite_refs/Sprite__UI_Enemy_Etemon.png \
  --name Gokuumon --card-id EX12-015 --level 5 \
  --colors 20 --merge 18 --fit 79x82 \
  -o data/sprites/EX12-015_Gokuumon.sprite.yaml
```

`--merge` collapses the rip's near-identical tones into real ramp steps;
`--fit` re-frames onto the level's canvas, foot-anchored.

### B. Restyle — move the palette to the subject

Palette work is cheap and high-yield, so do it before touching geometry.
Group keys into material ramps and move each ramp as a unit:

```python
import sys; sys.path.insert(0, "code/tools/spritekit")
from sprite import Sprite
import ops
sp = Sprite.load("data/sprites/EX12-015_Gokuumon.sprite.yaml")
fur = ["L", "M", "N", "O"]                    # the donor's orange body ramp
ops.recolor(sp, dict(zip(fur, ops.ramp("#e8e4d8", steps=4))))   # -> white fur
sp.save("data/sprites/EX12-015_Gokuumon.sprite.yaml")
```

`ops.ramp` reproduces the reference behaviour (shadows gain saturation, high
lights desaturate); a flat multiply looks plastic. `ops.restyle` hue-rotates a
ramp in place when you want to keep its tone steps exactly.

### C. Silhouette surgery — make it the right Digimon

**This is the step that decides whether the output is real work or a recolour.**
Change the outline: add the staff, ears, tail, robe, crest, horns; remove what
belongs to the donor and not the subject. Use `grid.py` to see coordinates:

```bash
python $SK/grid.py show data/sprites/EX12-015_Gokuumon.sprite.yaml --region 20,0,60,30
```

Edit the `grid:` block directly, or stamp a rectangle:

```bash
python $SK/grid.py patch data/sprites/EX12-015_Gokuumon.sprite.yaml --at 44,8 --block /tmp/staff.txt
```

`-` in a patch block means "leave this pixel". Rows must keep their exact
width — the loader rejects a ragged grid rather than silently shifting the art.

### D. Face

Big eye, dark socket, saturated iris, **1px white specular dot**. The specular
is what makes it look alive; the style guide calls it out because it is the
detail most often dropped.

### E. Repair, lint, look

```bash
python -c "import sys;sys.path.insert(0,'code/tools/spritekit');\
from sprite import Sprite;import ops;\
sp=Sprite.load('data/sprites/EX12-015_Gokuumon.sprite.yaml');\
ops.outline(sp);ops.drop_unused(sp);sp.save('data/sprites/EX12-015_Gokuumon.sprite.yaml')"

python $SK/lint.py data/sprites/EX12-015_Gokuumon.sprite.yaml
python $SK/render.py strip data/sprites/EX12-015_Gokuumon.sprite.yaml \
  .cache/sprite_refs/Sprite__UI_Enemy_Etemon.png -o /tmp/ab.png --scale 5
```

**Then `Read` `/tmp/ab.png`.** Never ship a sprite you have not looked at
beside its donor. Lint catches structure, not whether it reads as the Digimon.
Iterate C–E until it does.

## Lines and archetypes

Author an evolution line **together**, not card by card:

- Set every canvas first, from `style.canvas_for(level)`, strictly increasing
  along the line. Stage reads through size before anything else.
- Share one palette across the line, extending it for stage-specific gear. A
  Rookie and its Mega should be recognisably the same creature's colours.
- Carry a signature motif up the line (a crest, a marking, a weapon).
- Review the line as a set before declaring it done:

```bash
python $SK/render.py sheet data/sprites/*.sprite.yaml -o /tmp/line.png --scale 4 --cols 5
```

Read that sheet. Silhouettes should be distinguishable at 1:1 with the colour
stripped away — if two sprites differ only in palette, the line has failed.

## The bar

A sprite is done when all of these hold:

1. `lint.py` reports no ERROR (WARNs need a stated reason).
2. Read side by side with the card art, it is recognisably **that Digimon** —
   its distinctive gear and silhouette are present, not just its colours.
3. Read beside its donor, the silhouette has genuinely changed. A palette swap
   of a donor is **not** an acceptable deliverable; say so and keep working.
4. In a line, it reads as the right stage relative to its neighbours.

## Gotchas

- The card image has a `SAMPLE` watermark across the middle. Read the
  silhouette and gear from the clear areas; do not invent detail under it.
- `ops.outline` converts every silhouette-edge pixel to the keyline. Run it
  *after* geometry edits and *before* lint — but note it eats 1px of the body,
  so build shapes 1px larger than the final read.
- Keep the palette at 14–22 entries. More does not look better at this scale;
  it just makes the ramp muddy.
- `merge_similar` and `drop_unused` relabel keys darkest-to-lightest, so any
  key letters you memorised change. Re-run `grid.py show` after either.
