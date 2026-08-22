# Worked examples — the EX12 Seiyo Warriors Gokuumon line

Four complete builds, in the order they were authored. Read them for the
*shape* of a build, not as something to run.

| script | card | what it demonstrates |
|---|---|---|
| `kakamon_lv3.py` | EX12-006 Kakamon (Lv3) | the simple case: `map_to_ramp` a small-beast donor, then a face mask, oversized eyes, and a hat |
| `apemon_lv4.py` | EX12-012 Apemon (Lv4) | drawing a mane *behind* the body — snapshot the body, draw, then restore it in front |
| `gokuumon_lv5.py` | EX12-015 Gokuumon (Lv5) | the full case: erase the donor's head and author a new one; a held prop (staff tucked behind the fist) |
| `seitengokuumon_lv6.py` | EX12-048 SeitenGokuumon (Lv6) | **deriving a Mega from the line's own Ultimate** — how an evolution line stays one creature |

## These are scaffolding, not build infrastructure

Each script rebuilds its sprite **from scratch** and overwrites
`data/sprites/<ID>_<Name>.sprite.yaml`. The `.sprite.yaml` is the source of
truth; once a sprite is finished, hand-edit the YAML and let the script go
stale. Re-running one will silently discard any later hand edits.

Keep new build scripts in scratch. They are worth writing — an idempotent
script makes the render-look-tune loop fast — but they retire when the sprite
is done.

## Lessons these encode

- **Draw order matters more than anything.** A mane drawn after the body covers
  the torso; a head-erase box that overshoots leaves a hole where the mane used
  to hide it. Snapshot the body, draw, restore.
- **Rectangles cannot follow a silhouette.** Splitting a donor's single fur ramp
  into head-skin vs body-robe with `swap_in_box` leaves a visible straight seam.
  Inherit the body from the donor and author the head fresh instead — the head
  is the identity anyway.
- **Fewer, narrower strands read as fur; many strands merge into a block.** At
  17 strands on a radius-11 ring the bases overlap and the mane becomes a
  rounded lump. Wide gaps are the whole effect.
- **`ops.outline` eats 1px everywhere.** Anything thinner than ~5px across
  becomes solid keyline. That is why `spiky_ring` takes a `tip` width.
