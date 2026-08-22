"""Pick donor references for a target card.

A Digimon Up sprite of a *similar body plan* is worth far more than a verbal
style description: it carries the proportions, stance, keyline weight and
shading ramp for free, so authoring becomes re-skin-and-reshape rather than
draw-from-nothing.

Donors are scored on: the same Digimon (best), an evolution-family relative,
shared body-plan traits, attribute/colour agreement, and level proximity.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from functools import lru_cache
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import refs  # noqa: E402

# Card traits grouped into body plans. Two Digimon sharing a group usually share
# a silhouette skeleton, which is what a donor actually supplies.
BODY_PLANS: dict[str, set[str]] = {
    "ape": {"Beastkin", "Beast Man"},
    "beast4": {"Beast", "Holy Beast", "Ancient Beast", "Mammal", "Dark Animal",
               "Mythical Beast", "Mythical Animal", "Beastkin Dragon"},
    "reptile": {"Reptile", "Dinosaur", "Ceratopsian", "Lesser"},
    "dragon": {"Dragonkin", "Dragon Man", "Sky Dragon", "Light Dragon", "Dark Dragon",
               "Ancient Dragon", "Holy Dragon"},
    "bird": {"Bird", "Giant Bird", "Holy Bird", "Bird Man"},
    "insect": {"Insectoid", "Larva", "Crustacean", "Mollusk"},
    "aquatic": {"Aquatic", "Sea Animal", "Sea Beast", "Amphibian", "Fish", "Mollusk"},
    "machine": {"Machine", "Cyborg", "Android", "Armor", "Mutant"},
    "human": {"Wizard", "Monk", "Warrior", "Fairy", "Angel", "Archangel", "Demon Lord",
              "Undead", "Ghost", "Evil", "Shaman", "Tathāgata", "Deva"},
    "plant": {"Plant", "Vegetation", "Flower", "Fungus"},
    "rock": {"Rock", "Mineral", "Mud", "Icy"},
    "puppet": {"Puppet", "Toy"},
    "slime": {"Slime", "Baby", "Lesser"},
}
TRAIT_TO_PLAN = {t: plan for plan, ts in BODY_PLANS.items() for t in ts}

# Traits that describe an archetype/franchise rather than a shape — never a
# reason to consider two Digimon visually similar.
NON_VISUAL = {"SW", "TB", "VB", "ME", "DS", "NSp", "NSo", "TS", "Shambala", "ADVENTURE",
              "Hero", "X Antibody", "DigiPolice", "Xros Heart", "LIBERATOR"}


@lru_cache(maxsize=1)
def cards() -> dict:
    return json.loads((refs.repo_root() / "data" / "cards.json").read_text("utf-8"))


@lru_cache(maxsize=1)
def by_name() -> dict[str, dict]:
    """Lowercased Digimon name -> a representative printing."""
    out: dict[str, dict] = {}
    for c in cards().values():
        n = (c.get("card_name_eng") or "").lower()
        if not n or c.get("card_kind") != 0:
            continue
        # Prefer the printing with the richest trait list.
        if n not in out or len(c.get("type_eng") or []) > len(out[n].get("type_eng") or []):
            out[n] = c
    return out


def traits_of(name: str) -> set[str]:
    c = by_name().get(name.lower())
    return {t for t in (c.get("type_eng") or []) if t not in NON_VISUAL} if c else set()


def plans_of(name: str) -> set[str]:
    return {TRAIT_TO_PLAN[t] for t in traits_of(name) if t in TRAIT_TO_PLAN}


def _stem(name: str) -> str:
    """Strip evolution prefixes/suffixes so family members collide.

    ``SeitenGokuumon`` -> ``gokuu``, ``MetalGreymon`` -> ``grey``.
    """
    s = re.sub(r"mon$", "", name, flags=re.I)
    s = re.sub(
        r"^(Metal|War|Mega|Ultra|Super|Hi|Blue|Black|Red|Green|Gold|Silver|Dark|Holy|"
        r"Magna|Omni|Seiten|Cho|Chou|Great|Grand|Neo|Rise|Shine|Sky|Deep|Were|Skull|"
        r"Lady|Demi|Ko|Baby|Marine|Rapid|Crack|X)",
        "",
        s,
        flags=re.I,
    )
    return s.lower()[:6]


@lru_cache(maxsize=1)
def donors() -> list[dict]:
    """Every reference subject that has a single-pose ``ui_sprite``."""
    out = []
    seen = set()
    for r in refs.find(kind="ui_sprite"):
        if not refs.is_usable(r):
            continue  # blank placeholder or full-bleed UI chrome, not character art
        subj = r["subject"].split("#")[0].strip()
        base = subj.removeprefix("Child_").rstrip("_0123456789").strip("_")
        if base.lower() in seen:
            continue
        seen.add(base.lower())
        card = by_name().get(base.lower())
        out.append(
            {
                "subject": subj,
                "name": base,
                "record": r,
                "level": card.get("level") if card else None,
                "traits": traits_of(base),
                "plans": plans_of(base),
                "colors": set(card.get("card_colors") or []) if card else set(),
                "attribute": set(card.get("attribute_eng") or []) if card else set(),
            }
        )
    return out


def score(target: dict, d: dict) -> tuple[float, list[str]]:
    s, why = 0.0, []
    tname = target["name"].lower()

    if d["name"].lower() == tname:
        s += 100
        why.append("same Digimon")
    elif _stem(d["name"]) and _stem(d["name"]) == _stem(target["name"]):
        s += 45
        why.append("evolution family")

    shared_plans = target["plans"] & d["plans"]
    if shared_plans:
        s += 26 * len(shared_plans)
        why.append("body plan: " + "/".join(sorted(shared_plans)))
    shared_traits = target["traits"] & d["traits"]
    if shared_traits:
        s += 9 * len(shared_traits)
        why.append("traits: " + "/".join(sorted(shared_traits)))

    if d["level"] and target["level"]:
        gap = abs(d["level"] - target["level"])
        s += max(0, 12 - 6 * gap)
        if gap == 0:
            why.append(f"same level ({d['level']})")
    if target["colors"] & d["colors"]:
        s += 6
        why.append("shared colour")
    if target["attribute"] & d["attribute"]:
        s += 4
    return s, why


def suggest(card_id_or_name: str, top: int = 8) -> tuple[dict, list[tuple[float, dict, list[str]]]]:
    cid = card_id_or_name.upper()
    card = cards().get(cid)
    if card is None:
        card = by_name().get(card_id_or_name.lower())
    if card is None:
        raise SystemExit(f"no card found for {card_id_or_name!r}")

    name = card["card_name_eng"]
    target = {
        "name": name,
        "level": card.get("level"),
        "traits": traits_of(name),
        "plans": plans_of(name),
        "colors": set(card.get("card_colors") or []),
        "attribute": set(card.get("attribute_eng") or []),
    }
    ranked = []
    for d in donors():
        sc, why = score(target, d)
        if sc > 0:
            ranked.append((sc, d, why))
    ranked.sort(key=lambda r: -r[0])
    return {"card": card, **target}, ranked[:top]


def search(term: str) -> list[dict]:
    """Reference subjects whose name contains *term* (case-insensitive).

    Card traits are a lossy proxy for shape — Etemon, the roster's one true ape, is
    trait ``Puppet`` — so name knowledge is often the better lookup.
    """
    t = term.lower()
    return [d for d in donors() if t in d["name"].lower()]


def contact_sheet(entries: list[dict], out: Path, scale: int = 4, cols: int = 6) -> Path:
    """Render candidate donors side by side so they can be picked by eye."""
    from PIL import Image, ImageDraw

    tiles = []
    for d in entries:
        try:
            im = Image.open(refs.fetch(d["record"])).convert("RGBA")
        except Exception:
            continue
        im = im.resize((im.width * scale, im.height * scale), Image.NEAREST)
        tiles.append((f"{d['name']} L{d['level'] or '?'}", im))
    if not tiles:
        raise SystemExit("no donor images could be fetched")
    cw = max(im.width for _, im in tiles) + 16
    ch = max(im.height for _, im in tiles) + 28
    cols = min(cols, len(tiles))
    rows = (len(tiles) + cols - 1) // cols
    sheet = Image.new("RGBA", (cw * cols, ch * rows), (24, 24, 28, 255))
    d = ImageDraw.Draw(sheet)
    for i, (label, im) in enumerate(tiles):
        cx, cy = (i % cols) * cw, (i // cols) * ch
        # Bottom-align the art (the foot-anchored convention) and caption it
        # immediately underneath, so a short tile's label can't read as the
        # next row's.
        top = cy + ch - 20 - im.height
        sheet.alpha_composite(im, (cx + (cw - im.width) // 2, top))
        d.text((cx + 4, cy + ch - 16), label[:26], fill=(232, 232, 236, 255))
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("queries", nargs="+", help="card IDs or Digimon names")
    ap.add_argument("--top", type=int, default=8)
    ap.add_argument("--fetch", action="store_true", help="download the suggested donors")
    ap.add_argument("--search", action="store_true",
                    help="treat the queries as name substrings over the donor roster")
    ap.add_argument("--sheet", type=Path,
                    help="render the candidates to a labelled contact sheet PNG")
    args = ap.parse_args()

    if args.search:
        hits = [d for q in args.queries for d in search(q)]
        for d in hits:
            print(f"  {d['name']:24s} Lv{d['level'] or '?':<2} traits {sorted(d['traits']) or '-'}")
        print(f"({len(hits)} match)")
        if args.sheet and hits:
            print("sheet:", contact_sheet(hits, args.sheet))
        return 0

    sheet_pool: list[dict] = []
    for q in args.queries:
        target, ranked = suggest(q, args.top)
        c = target["card"]
        print(f"\n=== {c['card_id']} {target['name']} "
              f"(Lv{target['level']}, traits {sorted(target['traits']) or '-'}, "
              f"plans {sorted(target['plans']) or '-'}) ===")
        if not target["plans"]:
            print("  ! no body plan matched — donor ranking will lean on level/colour only")
        for sc, d, why in ranked:
            line = f"  {sc:6.1f}  {d['name']:24s} Lv{d['level'] or '?':<2}  {'; '.join(why)}"
            if args.fetch:
                p = refs.fetch(d["record"])
                line += f"\n          {p}"
            print(line)
            sheet_pool.append(d)
    if args.sheet and sheet_pool:
        print("\nsheet:", contact_sheet(sheet_pool, args.sheet))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
