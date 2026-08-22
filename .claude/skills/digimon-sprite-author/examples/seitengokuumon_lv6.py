"""Build EX12-048 SeitenGokuumon from the authored Gokuumon sprite.

The Mega is the same character evolved, so its donor is the line's own Lv5
sprite rather than a Digimon Up reference — which is what keeps an evolution
line reading as one creature.
"""
import sys
sys.path.insert(0, '/home/user/digimon-deck-list-builder/code/tools/spritekit')
from sprite import Sprite, _hex_to_rgba
import ops, draw

SRC = 'data/sprites/EX12-015_Gokuumon.sprite.yaml'
OUT = 'data/sprites/EX12-048_SeitenGokuumon.sprite.yaml'

sp = Sprite.load(SRC)
sp.name, sp.card_id, sp.level = 'SeitenGokuumon', 'EX12-048', 6


def nearest(hexc: str) -> str:
    """Palette key closest to *hexc* — the Lv5 build's ramps survive its final
    merge/relabel pass, so materials are recovered by colour, not by key."""
    t = _hex_to_rgba(hexc)
    return min(sp.palette,
               key=lambda k: sum((a - b) ** 2 for a, b in zip(_hex_to_rgba(sp.palette[k])[:3], t[:3])))


# Recover Gokuumon's material ramps (same bases as its build script).
MANE = [nearest(c) for c in ops.ramp('#dbe3ef', 4)]
GI   = [nearest(c) for c in ops.ramp('#3d5a75', 5, spread=0.52)]
SKIN = [nearest(c) for c in ops.ramp('#c98d5e', 4)]
GOLD = [nearest(c) for c in ops.ramp('#e0ac36', 3)]
SASH = [nearest(c) for c in ops.ramp('#b8392f', 3)]

# ---- 1. grow to Mega scale ------------------------------------------------
ops.fit_canvas(sp, 90, 96)
W, H = sp.canvas
DX, DY = (90 - 80) // 2, 96 - 88          # how the Lv5 art shifted

# ---- 2. restyle: silver mane -> gold, navy gi -> deep crimson ------------
ops.recolor(sp, dict(zip(dict.fromkeys(MANE), ops.ramp('#f2cf5a', len(dict.fromkeys(MANE))))))
ops.recolor(sp, dict(zip(dict.fromkeys(GI), ops.ramp('#7d1f22', len(dict.fromkeys(GI)), spread=0.52))))

CX, CY = 45 + DX, 22 + DY                 # head centre, carried from the Lv5

# ---- 3. heavier mane: the Mega's is longer and sweeps back ---------------
EXTRA = [(-150, 8), (-124, 11), (-98, 9), (-72, 12), (-46, 10), (-20, 12),
         (8, 12), (34, 10), (60, 12), (86, 9), (112, 11), (140, 8)]
draw.spiky_ring(sp, CX, CY, 13, 11, EXTRA, MANE[2], taper=0.34, tip=2)
for y in range(CY - 2, CY + 20):
    for x in range(CX - 26, CX + 26):
        if 0 <= x < W and 0 <= y < H and sp.get(x, y) == MANE[2]:
            if y > CY + 5 or x < CX - 12:
                sp.set(x, y, MANE[1])

# the face and circlet are re-stamped on top of the new mane
draw.ellipse(sp, CX + 1, CY + 2, 8, 8, SKIN[1])
draw.ellipse(sp, CX + 1, CY + 5, 6, 5, SKIN[2])
draw.ellipse(sp, CX + 1, CY + 7, 4, 3, SKIN[3])
draw.rect(sp, CX - 6, CY - 1, CX + 8, CY - 1, SKIN[0])
for ex in (CX - 4, CX + 5):
    draw.rect(sp, ex - 1, CY, ex + 1, CY + 2, 'K')
    draw.rect(sp, ex - 1, CY + 1, ex, CY + 2, GOLD[1])
    sp.set(ex - 1, CY + 1, MANE[3])
draw.rect(sp, CX - 1, CY + 8, CX + 3, CY + 8, SKIN[0])
draw.rect(sp, CX - 8, CY - 4, CX + 9, CY - 3, GOLD[1])
draw.rect(sp, CX - 8, CY - 4, CX + 9, CY - 4, GOLD[2])
# Mega crest horns on the circlet
for sx in (-7, 9):
    draw.spiky_ring(sp, CX + sx, CY - 4, 1, 1, [(sx * 4, 6)], GOLD[1], taper=0.5, tip=1)

# ---- 4. gold pauldrons + chest plate -------------------------------------
# sit them on the actual shoulder line, not floating on the chest
for px, py in ((CX - 16, CY + 24), (CX + 16, CY + 24)):
    draw.ellipse(sp, px, py, 9, 6, GOLD[1])
    draw.ellipse(sp, px, py - 2, 7, 3, GOLD[2])
    draw.ellipse(sp, px, py + 3, 9, 2, GOLD[0])
# chest plate: gold rim, dark crimson field, gold boss -- the inner tone has to
# differ from the body or the whole thing reads as a floating ring
draw.ellipse(sp, CX - 1, CY + 28, 11, 9, GOLD[1])
draw.ellipse(sp, CX - 1, CY + 28, 9, 7, SASH[0])
draw.ellipse(sp, CX - 1, CY + 27, 4, 3, GOLD[2])
draw.ellipse(sp, CX - 1, CY + 26, 2, 1, GOLD[0])

# ---- 5. a heavier staff ---------------------------------------------------
draw.line(sp, 70 + DX, 4, 70 + DX, 92, GOLD[0], width=4)
draw.line(sp, 69 + DX, 4, 69 + DX, 92, GOLD[1], width=2)
for cy in (8, 40, 70, 90):
    draw.rect(sp, 67 + DX, cy, 73 + DX, cy + 2, GOLD[2])

ops.outline(sp, 'K')
ops.merge_similar(sp, threshold=6)
ops.drop_unused(sp)
sp.donors = ['data/sprites/EX12-015_Gokuumon.sprite.yaml (same character, Lv5)']
sp.notes = ("Derived from the line's own Lv5 sprite so the Mega reads as the "
            "same creature: mane goes gold, gi goes crimson, plus pauldrons, "
            "chest plate, crest horns and a heavier staff.")
sp.save(OUT)
print(f'{OUT}  {sp.canvas[0]}x{sp.canvas[1]}  {len(sp.palette)} colours')
