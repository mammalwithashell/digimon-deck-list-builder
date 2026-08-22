"""Build EX12-006 Kakamon from the Gazimon armature. Re-runnable from scratch."""
import subprocess, sys
sys.path.insert(0, '/home/user/digimon-deck-list-builder/code/tools/spritekit')
from sprite import Sprite
import ops, draw

OUT = 'data/sprites/EX12-006_Kakamon.sprite.yaml'

subprocess.run([sys.executable, 'code/tools/spritekit/render.py', 'import',
                '.cache/sprite_refs/Sprite__UI_Enemy_Gazimon.png',
                '--name', 'Kakamon', '--card-id', 'EX12-006', '--level', '3',
                '--colors', '16', '--merge', '16', '--fit', '58x58',
                '-o', OUT], check=True, capture_output=True)
sp = Sprite.load(OUT)
W, H = sp.canvas

ops.map_to_ramp(sp, '#c89440', 5, spread=0.48)          # ochre-tan fur
FUR = sorted([k for k in sp.palette if k != 'K'],
             key=lambda k: ops.luma(ops._hex_to_rgba(sp.palette[k])))
sp.palette['K'] = '#170f08'

def add(base, steps, spread=0.4):
    free = [c for c in 'ABCDEFGHIJLMNOPQRSTUVWXYZ' if c not in sp.palette]
    out = []
    for c in ops.ramp(base, steps, spread=spread):
        k = free.pop(0)
        sp.palette[k] = c
        out.append(k)
    return out

MASK = add('#efe7dc', 3)     # white face mask
EAR  = add('#7b7f8c', 3)     # grey ears / paws
CAP  = add('#c0392f', 3)     # red cap
EYE  = add('#2f7fd0', 3)     # big blue eyes

CX, CY = 36, 25              # head centre after the 0,4 re-frame

# ---- grey ears -----------------------------------------------------------
# Gazimon's ears are already the right silhouette; only the hue moves, and
# only above the skull line so the head fur stays tan.
for src in FUR:
    draw.swap_in_box(sp, 10, 4, 32, 18, {src: EAR[1]})
    draw.swap_in_box(sp, 40, 4, 57, 18, {src: EAR[1]})
draw.swap_in_box(sp, 10, 4, 32, 10, {EAR[1]: EAR[2]})
draw.swap_in_box(sp, 44, 4, 57, 10, {EAR[1]: EAR[2]})

# ---- white face mask -----------------------------------------------------
draw.ellipse(sp, CX, CY + 2, 10, 8, MASK[1])
draw.ellipse(sp, CX, CY + 4, 8, 5, MASK[2])
draw.ellipse(sp, CX - 11, CY + 2, 2, 3, FUR[2])        # cheek fur tufts
draw.ellipse(sp, CX + 11, CY + 2, 2, 3, FUR[2])

# ---- big blue eyes (the Rookie read: eyes dominate the face) -------------
for ex in (CX - 5, CX + 5):
    draw.ellipse(sp, ex, CY + 2, 4, 4, 'K')
    draw.ellipse(sp, ex, CY + 2, 3, 3, EYE[1])
    draw.ellipse(sp, ex, CY + 3, 2, 2, EYE[2])
    sp.set(ex - 1, CY + 1, MASK[2])                     # specular
    sp.set(ex - 2, CY + 1, MASK[2])
draw.rect(sp, CX - 1, CY + 7, CX + 1, CY + 7, FUR[0])   # small muzzle line
sp.set(CX, CY + 6, FUR[1])

# ---- red cap -------------------------------------------------------------
draw.ellipse(sp, CX, CY - 7, 11, 6, CAP[1])
draw.ellipse(sp, CX - 3, CY - 9, 5, 3, CAP[2])          # highlight
draw.rect(sp, CX - 12, CY - 4, CX + 12, CY - 3, CAP[0]) # brim
sp.set(CX, CY - 13, CAP[2])
draw.ellipse(sp, CX, CY - 12, 2, 2, CAP[2])             # pom

# ---- grey paws -----------------------------------------------------------
for x0, y0, x1, y1 in ((15, 45, 25, 57), (30, 45, 42, 57)):
    for src in FUR[:4]:
        draw.rect(sp, x0, y0, x1, y1, EAR[1], over=src)
    draw.rect(sp, x0, y0, x1, y0 + 1, EAR[0], over=EAR[1])

ops.outline(sp, 'K')
ops.merge_similar(sp, threshold=6)
ops.drop_unused(sp)
sp.donors = ['Sprite__UI_Enemy_Gazimon.png (small-beast stance armature)']
sp.notes = ("Gazimon armature restyled to Kakamon: ochre fur, grey ears/paws, "
            "white face mask, oversized blue eyes, red cap.")
sp.save(OUT)
print(f'{OUT}  {sp.canvas[0]}x{sp.canvas[1]}  {len(sp.palette)} colours')
