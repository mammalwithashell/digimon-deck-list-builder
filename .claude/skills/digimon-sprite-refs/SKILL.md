---
name: digimon-sprite-refs
description: Resolve a Digimon card, name, or archetype to Digimon Up reference sprite art — the measured style envelope plus the best donor sprites to build from. Use when authoring, reviewing, or restyling pixel sprites for Digimon cards; when asking "what should <Digimon> look like as a sprite", "what size should a Lv5 sprite be", "what can I use as a base for <Digimon>"; or before running /digimon-sprite-author. Also use to refresh or repair the reference index.
---

# Digimon Up reference sprites

The style target is the **Digimon Up** mobile-game sprite set: ~33–130px
hard-edged pixel art with a continuous black keyline, cel shading and big
specular-dotted eyes. A public Google Drive folder holds the ripped assets and
this skill turns it into something you can actually author against.

**Read `style-guide.md` (next to this file) before authoring anything.** It is
measured from the library, not remembered, and it is what `lint.py` enforces.

## What is available

`data/sprite_refs/index.json` indexes **6,311 assets** across three folders:

| kind | n | what it is | use it for |
|---|---:|---|---|
| `ui_sprite` | 339 | `Sprite/UI_<role>_<Name>.png` — one hard-edged idle pose | **the primary donor + style reference** |
| `atlas` | 691 | `Texture2D/<Name>.png` — packed animation sheet | seeing how a body plan moves / alternate poses |
| `fx` | 489 | attack and hit effect sheets | attack VFX, not characters |
| `other` | 4,792 | UI chrome, avatar parts, backgrounds, audio | ignore |

**248 distinct Digimon** have a usable single-pose sprite. Notable donors whose
card traits do *not* advertise their shape: **Etemon** (the roster's only real
ape — trait `Puppet`), **Unimon** (winged horse — `Mythical Beast`),
**Tortomon** (turtle), **Gekomon`/`ShogunGekomon** (frog/kappa), **Taomon**
(robed monk), **WaruMonzaemon** (bear).

## Workflow

### 1. Make sure the index exists

```bash
python code/tools/spritekit/drive_index.py          # ~15s, writes data/sprite_refs/index.json
```

Only needed once, or when the Drive folder changes. The folder is link-shared,
so the Drive connector **cannot** list it (`parentId = ...` returns nothing) —
the indexer parses the public `embeddedfolderview` HTML instead, which returns
the whole folder in one request. If that ever breaks, that is the thing to fix.

### 2. Get donor candidates for the target

```bash
python code/tools/spritekit/pick_refs.py EX12-015 --top 8 --sheet /tmp/donors.png
```

Ranks the 248 donors by: same Digimon → evolution family → shared body plan →
shared traits → level proximity → colour. Accepts card IDs or Digimon names.

**Then `Read` the contact sheet and pick by eye.** The ranking is a shortlist,
not an answer — card traits are a lossy proxy for shape (Etemon is `Puppet`,
Sistermon is `Puppet`, Unimon is `Mythical Beast`). When you know the subject
is a monkey/turtle/horse, search the roster by name and look:

```bash
python code/tools/spritekit/pick_refs.py --search etemon leomon vikemon --sheet /tmp/c.png
```

Choose a donor for its **body plan and stance**, not its colour — colour is one
`ops.restyle` call away, but a wrong skeleton costs the whole sprite.

### 3. See the subject

Donors give you style; the card art gives you the subject. Resolve and read it:

```bash
python .claude/skills/digimon-card-lookup/resolve_cards.py EX12-015
```

Card images carry a `SAMPLE` watermark across the middle — read the silhouette,
palette and gear from the clear areas, and consult the atlas
(`Texture2D/<Name>.png`) for any Digimon that already exists in the game.

### 4. Fetch what you chose

```python
import sys; sys.path.insert(0, "code/tools/spritekit")
import refs
rec = refs.find(kind="ui_sprite", subject="Etemon")[0]
print(refs.fetch(rec))     # -> .cache/sprite_refs/Sprite__UI_Enemy_Etemon.png
```

PNGs land in the gitignored `.cache/sprite_refs/`; the repo carries only the
index, never the third-party art.

## Sizing

Set the canvas from the printed level before drawing anything:

```python
import sys; sys.path.insert(0, "code/tools/spritekit")
import style
style.canvas_for(5)        # -> (79, 82)   median Ultimate footprint
style.height_bounds(5)     # -> (51, 123)  what lint.py will accept
```

Across an evolution line, keep canvases strictly increasing — stage reads
through size more than through detail.

## Gotchas

- Some `UI_*` records are not character art (`UI_Pet_Monzaemon.png` is blank,
  a few are full-bleed banners). `refs.is_usable()` filters them by silhouette
  fill and `pick_refs` applies it; if you bypass the picker, apply it yourself.
- `Child_<Name>`, `<Name>_0` and `<Name>_2` are alternate printings of the same
  subject — deduplicated by the picker, still worth a look for pose variety.
- The reference sprites average one colour per four pixels because they were
  downscaled from larger art. **Do not copy that.** See style-guide §4.
