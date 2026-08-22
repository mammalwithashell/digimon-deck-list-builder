"""Build EX12-015 Gokuumon from the Etemon armature. Re-runnable from scratch."""
import subprocess, sys
sys.path.insert(0, '/home/user/digimon-deck-list-builder/code/tools/spritekit')
from sprite import Sprite
import ops, draw

OUT = 'data/sprites/EX12-015_Gokuumon.sprite.yaml'

# ---- 0. fresh donor import ------------------------------------------------
subprocess.run([sys.executable, 'code/tools/spritekit/render.py', 'import',
                '.cache/sprite_refs/Sprite__UI_Enemy_Etemon.png',
                '--name', 'Gokuumon', '--card-id', 'EX12-015', '--level', '5',
                '--colors', '18', '--merge', '16', '--fit', '80x88',
                '-o', OUT], check=True, capture_output=True)
sp = Sprite.load(OUT)

# ---- 1. palette: one ramp per material ------------------------------------
sp.palette['K'] = '#0d0a14'                       # cool-tinted keyline
RAMPS = {
    's': ops.ramp('#c98d5e', 4),                  # monkey skin
    'm': ops.ramp('#dbe3ef', 4),                  # silver-white mane
    'r': ops.ramp('#3d5a75', 5, spread=0.52),     # dark teal-navy gi
    'x': ops.ramp('#b8392f', 3),                  # red sash
    'g': ops.ramp('#e0ac36', 3),                  # gold circlet / trim
    'w': ops.ramp('#6b4a2e', 3),                  # staff timber
}
spare = list('abcdefhijnopqtuvwyz0123456789')
A = {}
for pre, cols in RAMPS.items():
    for i, c in enumerate(cols):
        k = spare.pop(0)
        A[f'{pre}{i}'] = k
        sp.palette[k] = c

# ---- 2. the whole donor becomes the robe ----------------------------------
# Keep the donor's tonal structure: its orange ramp and cream chest both map
# onto the gi, so all the inherited shading survives the restyle.
W, H = sp.canvas
whole = (0, 0, W - 1, H - 1)
draw.swap_in_box(sp, *whole, {
    'L': A['r0'], 'M': A['r1'], 'N': A['r1'], 'O': A['r2'],
    'P': A['r2'], 'Q': A['r3'], 'R': A['r3'], 'S': A['r4'],
    'T': A['r1'], 'U': A['r3'], 'V': A['r4'],
})

BODY = list(sp.rows)      # snapshot: used to put the grip back over the staff

# ---- 3. clear the donor's head; Gokuumon's is drawn fresh -----------------
draw.erase_box(sp, 30, 0, 60, 33)
draw.erase_box(sp, 33, 39, 55, 44)        # Etemon's open jaw
draw.erase_box(sp, 42, 51, 54, 70)        # Etemon's microphone

# ---- 4. mane (behind the face) -------------------------------------------
CX, CY = 45, 22
# A tight core plus directional wedges: round lobes read as an afro, and 1px
# spikes vanish once ops.outline runs, so the strands are short fat wedges.
draw.ellipse(sp, CX, CY, 13, 11, A['m2'])
draw.ellipse(sp, CX, CY + 7, 12, 9, A['m2'])         # flows onto the shoulders
draw.ellipse(sp, CX - 12, CY + 6, 5, 6, A['m2'])     # side falls
draw.ellipse(sp, CX + 12, CY + 6, 5, 6, A['m2'])
STRANDS = [(-158, 6), (-138, 9), (-118, 7), (-96, 10), (-74, 8), (-52, 11),
           (-30, 8), (-10, 10), (12, 8), (34, 11), (56, 8), (78, 10),
           (100, 7), (122, 9), (144, 7), (162, 6)]
draw.spiky_ring(sp, CX, CY, 12, 10, STRANDS, A['m2'], taper=0.42, tip=2)
# shade: lower half + left edge drop to the shadow tone
for y in range(CY - 2, CY + 22):
    for x in range(CX - 22, CX + 22):
        if 0 <= x < W and 0 <= y < H and sp.get(x, y) == A['m2']:
            if y > CY + 5 or x < CX - 12:
                sp.set(x, y, A['m1'])
draw.ellipse(sp, CX - 4, CY - 6, 6, 4, A['m3'])   # top-left highlight

# heal the chest where the head-erase overshot
for y in range(30, 46):
    for x in range(28, 62):
        if sp.get(x, y) == '.' and BODY[y][x] != '.':
            sp.set(x, y, BODY[y][x])

# ---- 5. face --------------------------------------------------------------
draw.ellipse(sp, CX + 1, CY + 2, 8, 8, A['s1'])       # head skin
draw.ellipse(sp, CX + 1, CY + 5, 6, 5, A['s2'])       # muzzle
draw.ellipse(sp, CX + 1, CY + 7, 4, 3, A['s3'])       # muzzle highlight
draw.ellipse(sp, CX - 9, CY + 2, 2, 3, A['s1'])       # ears
draw.ellipse(sp, CX + 11, CY + 2, 2, 3, A['s1'])

# brow + eyes
draw.rect(sp, CX - 6, CY - 1, CX + 8, CY - 1, A['s0'])
for ex in (CX - 4, CX + 5):
    draw.rect(sp, ex - 1, CY, ex + 1, CY + 2, 'K')     # socket
    draw.rect(sp, ex - 1, CY + 1, ex, CY + 2, A['g1']) # amber iris
    sp.set(ex - 1, CY + 1, A['m3'])                    # 1px specular
draw.rect(sp, CX - 1, CY + 8, CX + 3, CY + 8, A['s0'])  # mouth line

# ---- 6. gold circlet ------------------------------------------------------
draw.rect(sp, CX - 8, CY - 4, CX + 9, CY - 3, A['g1'])
draw.rect(sp, CX - 8, CY - 4, CX + 9, CY - 4, A['g2'])
sp.set(CX + 1, CY - 4, A['g0'])

# ---- 7. red sash across the chest ----------------------------------------
for src in (A['r0'], A['r1'], A['r2'], A['r3'], A['r4']):
    draw.line(sp, 29, 43, 50, 58, A['x1'], width=9, over=src)
draw.line(sp, 29, 42, 50, 56, A['x2'], width=2, over=A['x1'])
draw.line(sp, 29, 49, 50, 63, A['x0'], width=2, over=A['x1'])

# ---- 8. the staff, held in the raised right hand -------------------------
draw.line(sp, 70, 6, 70, 84, A['w1'], width=3)
draw.line(sp, 69, 6, 69, 84, A['w2'], width=1)
draw.rect(sp, 68, 10, 72, 12, A['g1'])
draw.rect(sp, 68, 78, 72, 80, A['g1'])
# put the raised fist back over the shaft so the staff reads as held
for y in range(24, 42):
    for x in range(63, 80):
        if BODY[y][x] != '.':
            sp.set(x, y, BODY[y][x])

# ---- 9. finish ------------------------------------------------------------
ops.outline(sp, 'K')
ops.merge_similar(sp, threshold=6)
ops.drop_unused(sp)
sp.donors = ['Sprite__UI_Enemy_Etemon.png (body/pose armature)']
sp.notes = ("Etemon supplies the ape stance and limb structure; head, mane, "
            "circlet, sash and staff are authored. Ramps: skin/mane/gi/sash/gold.")
sp.save(OUT)
print(f'{OUT}  {sp.canvas[0]}x{sp.canvas[1]}  {len(sp.palette)} colours')
