"""Build EX12-012 Apemon from the Leomon armature. Re-runnable from scratch."""
import subprocess, sys
sys.path.insert(0, '/home/user/digimon-deck-list-builder/code/tools/spritekit')
from sprite import Sprite
import ops, draw

OUT = 'data/sprites/EX12-012_Apemon.sprite.yaml'

subprocess.run([sys.executable, 'code/tools/spritekit/render.py', 'import',
                '.cache/sprite_refs/Sprite__UI_Enemy_Leomon.png',
                '--name', 'Apemon', '--card-id', 'EX12-012', '--level', '4',
                '--colors', '18', '--merge', '16', '--fit', '62x76',
                '-o', OUT], check=True, capture_output=True)
sp = Sprite.load(OUT)
W, H = sp.canvas

# Leomon's whole tonal structure -> Apemon's yellow fur. The donor's shading
# survives; only the hue moves.
ops.map_to_ramp(sp, '#f0c020', 5, spread=0.5)
FUR = sorted([k for k in sp.palette if k != 'K'],
             key=lambda k: ops.luma(ops._hex_to_rgba(sp.palette[k])))
sp.palette['K'] = '#150f06'

def add(prefix, base, steps, spread=0.4):
    free = [c for c in 'ABCDEFGHIJLMNOPQRSTUVWXYZ' if c not in sp.palette]
    out = []
    for c in ops.ramp(base, steps, spread=spread):
        k = free.pop(0)
        sp.palette[k] = c
        out.append(k)
    return out

FACE = add('f', '#4a6f9c', 4)      # Apemon's blue-grey face
RED  = add('r', '#d0342a', 3)      # mane streaks
GRN  = add('g', '#2e8b4f', 3)      # armbands
BONE = add('b', '#e6e0cc', 3)      # club

CX, CY = 33, 17                    # head centre after the 5,4 re-frame
BODY = list(sp.rows)               # so the torso can be restored in front

# ---- bigger, spikier mane (Apemon's is far larger than Leomon's) ---------
# Few, narrow strands: at 17 strands the bases merge and the mane reads as a
# rounded block. Wide gaps are what makes it read as hair.
STRANDS = [(-142, 4), (-115, 6), (-88, 5), (-61, 7), (-34, 6), (-7, 6),
           (20, 7), (47, 6), (74, 5), (101, 6), (128, 4)]
draw.spiky_ring(sp, CX, CY, 11, 9, STRANDS, FUR[3], taper=0.26, tip=2)
draw.ellipse(sp, CX, CY, 11, 9, FUR[3])
# mane shading, before the streaks so they stay saturated
for y in range(CY - 18, CY + 18):
    for x in range(CX - 22, CX + 22):
        if 0 <= x < W and 0 <= y < H and sp.get(x, y) == FUR[3]:
            if y > CY + 4 or x < CX - 8:
                sp.set(x, y, FUR[1])
# red streaks at the tips of a few strands
for ang, ln in STRANDS[1::3]:
    draw.spiky_ring(sp, CX, CY, 11 + ln - 3, 9 + ln - 3, [(ang, 2)], RED[1],
                    taper=0.30, tip=1)
# the torso and arms sit in front of the mane
for y in range(CY + 8, H):
    for x in range(W):
        if BODY[y][x] != '.':
            sp.set(x, y, BODY[y][x])

# ---- face ----------------------------------------------------------------
draw.ellipse(sp, CX - 8, CY + 1, 2, 3, FACE[0])         # ears
draw.ellipse(sp, CX + 10, CY + 1, 2, 3, FACE[0])
draw.ellipse(sp, CX + 1, CY + 1, 7, 6, FACE[1])
draw.ellipse(sp, CX + 1, CY + 4, 5, 3, FACE[2])         # muzzle
draw.ellipse(sp, CX + 1, CY + 5, 3, 2, FACE[3])
draw.rect(sp, CX - 4, CY - 2, CX + 6, CY - 2, FACE[0])  # brow
for ex in (CX - 2, CX + 4):
    draw.rect(sp, ex - 1, CY - 1, ex + 1, CY + 1, FACE[0])
    draw.rect(sp, ex - 1, CY, ex, CY + 1, FUR[4])       # bright yellow iris
    sp.set(ex - 1, CY, BONE[2])                         # specular
draw.rect(sp, CX - 1, CY + 6, CX + 3, CY + 6, FACE[0])  # mouth
sp.set(CX - 1, CY + 5, BONE[2])                         # fang
sp.set(CX + 3, CY + 5, BONE[2])

# ---- green armbands ------------------------------------------------------
for x0, x1, y0 in ((11, 20, 40), (42, 52, 42)):
    for src in FUR:
        draw.rect(sp, x0, y0, x1, y0 + 2, GRN[1], over=src)
        draw.rect(sp, x0, y0, x1, y0, GRN[2], over=GRN[1])

# ---- bone club in the lowered right hand ---------------------------------
draw.line(sp, 52, 46, 56, 68, BONE[1], width=3)
draw.ellipse(sp, 52, 45, 3, 3, BONE[2])
draw.ellipse(sp, 56, 69, 3, 3, BONE[2])

ops.outline(sp, 'K')
ops.merge_similar(sp, threshold=6)
ops.drop_unused(sp)
sp.donors = ['Sprite__UI_Enemy_Leomon.png (beastman stance/limb armature)']
sp.notes = ("Leomon armature restyled to Apemon: whole tonal structure mapped "
            "to yellow fur, mane enlarged with red streaks, blue face, green "
            "armbands, bone club.")
sp.save(OUT)
print(f'{OUT}  {sp.canvas[0]}x{sp.canvas[1]}  {len(sp.palette)} colours')
