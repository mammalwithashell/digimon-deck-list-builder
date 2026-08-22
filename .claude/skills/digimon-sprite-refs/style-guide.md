# Digimon Up sprite style — measured

Every number here was measured from the reference library itself
(`python code/tools/spritekit/analyze.py`, 338 cached `UI_<role>_<Name>.png`
sprites, 283 of which join to a printed card level in `data/cards.json`).
Re-run the analyzer after refreshing the index rather than trusting this file
if the library ever changes. The machine-readable copy lives in
`code/tools/spritekit/style.py` and is what `lint.py` enforces.

## 1. Canvas — size carries evolution stage

Tight bounding box of the art, by printed card level:

| Level | Stage | Height p10 | Height **median** | Height p90 | Width median | n |
|------:|-------|-----------:|------------------:|-----------:|-------------:|--:|
| 3 | Rookie   | 38 | **51** | 65 | 52 | 60 |
| 4 | Champion | 51 | **57** | 74 | 64 | 107 |
| 5 | Ultimate | 63 | **82** | 105 | 79 | 76 |
| 6 | Mega     | 76 | **90** | 109 | 87 | 38 |
| 7 | —        | 110 | **119** | 128 | 142 | 2 |

Size is the main way the reference set signals stage, so **a Rookie must read
smaller than its Champion**. When authoring an evolution line, set the canvases
first and keep them monotonically increasing; that single decision does more
for line coherence than any amount of shading work.

Framing: the art is **foot-anchored** — it touches the bottom edge and sits
horizontally centred. `ops.fit_canvas` applies this.

## 2. Alpha is binary

Median partial-alpha pixels per reference sprite: **1**. 152 of 338 are exactly
zero. There is no soft edge — the silhouette is hard, and anti-aliasing against
the background is never used. Authored sprites are binary by construction
(a grid cell is either a palette key or `.`).

## 3. The keyline is the style

A continuous near-black outline around the whole silhouette is the single most
recognisable feature. Measured: the darkest colour in a reference sprite has
luma **0** at the median (worst case 10.8), and ~35% of opaque pixels sit below
luma 60 once interior contour lines are counted.

Rules:
- Every silhouette-edge pixel is dark (luma ≤ 60). `lint.py` fails below 90%.
- Use a near-black that is *tinted toward the body colour*, not pure `#000000`
  — e.g. `#0d0a12` for cool subjects, `#140b08` for warm ones. It reads richer
  at 1:1 and is what the better references do.
- Interior separations (limb over torso, jaw line) use a dark shade of the
  local hue rather than the keyline black.

## 4. Palette — tight ramps, not gradients

The rips average **one distinct colour per four opaque pixels** (~400 colours
in a Rookie sprite) because they were downscaled from higher-resolution art.
**Do not imitate that.** Author with a tight indexed ramp — 8–32 entries,
target 14–22 — grouped as material ramps:

```
K   keyline            near-black, hue-tinted
a-c fur/skin           shadow / base / highlight   (3-4 steps)
d-f secondary material armour, robe, shell
g-h accent             gold trim, gems, weapon
i-j eye                dark socket, iris, 1px white specular
```

`ops.ramp(base, steps)` builds a cel ramp with the reference's behaviour:
shadows gain saturation and rotate slightly cool-dark, highlights desaturate.
A flat multiply/screen ramp looks plastic by comparison.

## 5. Silhouette density

Opaque pixels ÷ bbox area: median **0.59**, band 0.35–0.85. Below that the
pose is too spindly to read at 1:1; above it the shape is a blob. Limbs are
read by *negative space* — keep a transparent gap between an arm and the torso
rather than relying on an interior line.

## 6. Face

- Eyes are large and high-contrast: a dark socket block, a saturated iris, and
  a 1px white specular dot. At Rookie scale the eye is often 3×4 px total.
- The specular dot is what makes the sprite look alive — never omit it.
- Mouths are 1–2px dark strokes; teeth/fangs are 1px whites.

## 7. Pose

Three-quarter view, facing viewer-right, weight on both feet, arms clear of the
torso. Quadrupeds are shown in near-profile with the head turned toward the
viewer. Keep the pose neutral-idle: these are portrait/list sprites, not
action frames. (`Texture2D/<Name>.png` atlases hold the action poses if you
need to see how a shape moves.)

## 8. What "similar to Digimon Up" actually means

If a sprite has: stage-appropriate size, a continuous hue-tinted black keyline,
binary alpha, a tight cel ramp with saturated shadows, a big specular-dotted
eye, and a foot-anchored three-quarter idle pose — it reads as Digimon Up even
though it uses a fraction of the colours. Those seven properties are what
`lint.py` checks; everything else is subject matter.
