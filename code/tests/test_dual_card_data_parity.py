"""Data-parity guard: DUAL cards in PRODUCTION card data match the official Bandai DB.

Why this exists. Every production surface -- the hosted API, the Python bindings, the
desktop app, the exam harness, the replay corpus -- builds `CardData` from
`data/cards.json`. The Rust behavioral suite does NOT: DebugRunner builds `CardData`
from each card's YAML (`card_data_from_compiled`). So a DUAL card whose YAML is right
and whose `cards.json` entry is wrong passes every behavioral test while being wrong
in every real game. That is not hypothetical. Until 2026-09-21:

  * ST23-09, ST24-07, BT25-043, BT25-057 and BT25-104 had NO `dual` block in
    cards.json at all -- in real games they were plain Digimon whose Option face
    could never be used;
  * EX12-018/033/052 and BT25-085 carried the wrong Option-face colours, which
    decide option-use colour legality (action/mask.rs
    `option_color_match_available`);
  * EX12-018/033/052's DUAL block lacked the second-colour digivolve circle the
    API drops, so those cards could not digivolve from that colour.

The source of truth is the official DB's "DUAL Color" / "DUAL Cost" fields
(world.digimoncard.com), mirrored as `dual_colors` / `dual_cost` in
`data/card_official.json` by `code/tools/build_card_bundles.py`.

What is checked (`find_violations` is pure so it can be run on any snapshot):
  1. every card the official mirror marks DUAL is `card_kind == 4` with a `dual` block;
  2. its Option colours and use cost equal the official ones;
  3. its DUAL-block digivolve circles equal the official printed circles;
  4. every YAML spec declaring a `dual:` block is a DUAL card in cards.json
     (the class of bug that hid ST23-09 et al. -- right YAML, wrong production data);
  5. `ingest_cards.DUAL_OPTION_COLOR_OVERRIDES` agrees with the official colours, so a
     re-ingest reproduces them instead of reverting them.

Colours are compared as SETS: legality is set-based, and the official DB's ordering is
presentation only.
"""
import glob
import json
import os
import re
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
_ROOT = os.path.abspath(os.path.join(_HERE, "..", ".."))

_COLOR = {"red": 0, "blue": 1, "yellow": 2, "green": 3, "white": 4, "black": 5, "purple": 6}


def _load_json(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def _circles(rows):
    return sorted((int(r["card_color"]), int(r["level"]), int(r["memory_cost"])) for r in rows)


def _norm_colors(names):
    return sorted(c.lower() for c in names)


def yaml_dual_option_colors(cards_dir):
    """{card_id: [colours]} for every YAML spec that declares a `dual:` block."""
    out = {}
    for path in glob.glob(os.path.join(cards_dir, "*", "*.yaml")):
        text = open(path, encoding="utf-8").read()
        if not re.search(r"(?m)^dual:\s*$", text):
            continue
        cid = os.path.splitext(os.path.basename(path))[0]
        m = re.search(r"(?ms)^  option:\s*$.*?^    colors:\s*\[([^\]]*)\]", text)
        out[cid] = [c.strip() for c in m.group(1).split(",")] if m else None
    return out


def find_violations(cards, official, yaml_duals, ingest_table):
    """Every DUAL data disagreement, as human-readable strings. Empty means clean."""
    v = []
    off_dual = {cid: e for cid, e in official.items() if e.get("dual_colors")}

    for cid, e in sorted(off_dual.items()):
        c = cards.get(cid)
        if c is None:
            v.append(f"{cid}: official DUAL card missing from cards.json")
            continue
        if c.get("card_kind") != 4 or not c.get("dual"):
            v.append(f"{cid}: official DUAL card is card_kind={c.get('card_kind')} "
                     f"with{'' if c.get('dual') else ' NO'} dual block in cards.json "
                     "-- production treats it as a plain card; its Option face is unusable")
            continue
        opt = c["dual"]["option"]
        want = _norm_colors(e["dual_colors"].split())
        if _norm_colors(opt.get("colors", [])) != want:
            v.append(f"{cid}: dual.option.colors {opt.get('colors')} != official {want}")
        if e.get("dual_cost") and opt.get("use_cost") != int(e["dual_cost"]):
            v.append(f"{cid}: dual.option.use_cost {opt.get('use_cost')} != official {e['dual_cost']}")
        if e.get("digivolve_costs") is not None:
            got = _circles(c["dual"]["digimon"].get("evo_costs", []))
            need = _circles(e["digivolve_costs"])
            if got != need:
                v.append(f"{cid}: dual.digimon.evo_costs {got} != official printed circles {need}")

    for cid, ycols in sorted(yaml_duals.items()):
        c = cards.get(cid)
        if c is None:
            continue
        if c.get("card_kind") != 4 or not c.get("dual"):
            v.append(f"{cid}: YAML declares a dual: block but cards.json has card_kind="
                     f"{c.get('card_kind')} and no dual block -- tests pass on the YAML while "
                     "every production surface loads a plain card")
            continue
        if ycols is not None and _norm_colors(ycols) != _norm_colors(c["dual"]["option"]["colors"]):
            v.append(f"{cid}: YAML dual.option.colors {ycols} != cards.json "
                     f"{c['dual']['option']['colors']}")

    for cid, cols in sorted(ingest_table.items()):
        e = off_dual.get(cid)
        if e and _norm_colors(cols) != _norm_colors(e["dual_colors"].split()):
            v.append(f"{cid}: ingest_cards.DUAL_OPTION_COLOR_OVERRIDES {cols} != official "
                     f"{e['dual_colors'].split()} -- a re-ingest would revert the fix")
    return v


def _ingest_table():
    sys.path.insert(0, os.path.join(_ROOT, "code", "tools"))
    try:
        import ingest_cards
    finally:
        sys.path.pop(0)
    return ingest_cards.DUAL_OPTION_COLOR_OVERRIDES


def _repo_inputs():
    cards = _load_json(os.path.join(_ROOT, "data", "cards.json"))
    cards = cards if isinstance(cards, dict) else {c["card_id"]: c for c in cards}
    official = _load_json(os.path.join(_ROOT, "data", "card_official.json"))["cards"]
    yaml_duals = yaml_dual_option_colors(os.path.join(_ROOT, "code", "digimon-engine", "cards"))
    return cards, official, yaml_duals, _ingest_table()


def test_dual_cards_match_the_official_db():
    violations = find_violations(*_repo_inputs())
    assert not violations, "DUAL card data disagrees with the official Bandai DB:\n  " + \
        "\n  ".join(violations)


def test_the_guard_covers_every_known_dual_card():
    # A vacuous pass is the failure mode to fear: if the official mirror stopped
    # capturing `dual_colors`, test_dual_cards_match_the_official_db would pass on
    # nothing. Pin the cards this guard was written against.
    cards, official, yaml_duals, _ = _repo_inputs()
    covered = {cid for cid, e in official.items() if e.get("dual_colors")}
    must = {"ST23-09", "ST24-07", "EX12-018", "EX12-033", "EX12-052",
            "BT25-043", "BT25-057", "BT25-085", "BT25-104"}
    assert must <= covered, f"official mirror lost dual_colors for {sorted(must - covered)}"
    assert set(yaml_duals) <= set(cards), "a YAML DUAL spec has no cards.json entry"
