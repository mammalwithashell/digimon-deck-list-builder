#!/usr/bin/env python3
"""Scout pass for the DSL digivolution-requirement audit.

For every DSL-implemented Digimon (a YAML spec under
`code/digimon-engine/cards/<set>/<ID>.yaml` whose cards.json kind is Digimon and
level >= 3), emit a structured audit record:

  - cards.json: card_colors, level, evo_costs, xros_req, dna_costs, effect text
  - YAML: declared color, and the normalized `alt_paths` (digivolve / dna kinds)
  - image path (authoritative printed text) + DCGO C# path (alt-condition oracle)
  - "smell" flags that PRIORITIZE which cards a vision agent should scrutinize

The smells are heuristics — they OVER- and UNDER-flag (proven during the BG
Imperial fix: a mono-colour card can have a dual-colour circle, and a 2-colour
card can legitimately have a mono-colour circle). They are NOT verdicts; the
card image is authoritative. They exist only to focus and structure the
vision audit.

Output: JSON list to stdout (or --out FILE), plus a summary to stderr.
"""
import argparse
import glob
import json
import os
import re
import sys

CMAP = {0: "Red", 1: "Blue", 2: "Yellow", 3: "Green", 4: "White", 5: "Black", 6: "Purple"}

try:
    import yaml
except ImportError:
    print("pyyaml required", file=sys.stderr)
    sys.exit(1)


def repo_root():
    # this file: code/tools/audit_digivolve/extract_records.py
    return os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))


IMG_DIR = os.environ.get(
    "DIGIMON_CARD_IMAGE_DIR",
    r"C:\Users\james\Documents\DCGO_Application\Assets\Textures\Card",
)


def color_name(code):
    return CMAP.get(code, f"?{code}")


def dcgo_cs_path(base_dcgo, cid, color_codes):
    """Best-effort DCGO C# path: CardEffect/<SET>/<COLOR>/<ID_with_underscore>.cs."""
    set_id = cid.split("-")[0]
    fname = cid.replace("-", "_") + ".cs"
    # try each of the card's colours (C# files live under one colour dir)
    for cc in color_codes or []:
        cdir = color_name(cc)
        p = os.path.join(base_dcgo, "Assets", "Scripts", "CardEffect", set_id, cdir, fname)
        if os.path.exists(p):
            return p
    # fallback: glob any colour dir
    hits = glob.glob(
        os.path.join(base_dcgo, "Assets", "Scripts", "CardEffect", set_id, "*", fname)
    )
    return hits[0] if hits else None


def normalize_alt_paths(spec):
    """Pull digivolve/dna alt_paths out of a parsed YAML spec into a compact form."""
    out = []
    for p in spec.get("alt_paths", []) or []:
        if not isinstance(p, dict):
            continue
        kind = p.get("kind")
        if kind not in ("digivolve", "dna_digivolve"):
            continue
        entry = {
            "kind": kind,
            "from": p.get("from"),
            "cost": p.get("cost"),
            "materials": p.get("materials"),
            "ignore_requirements": p.get("ignore_requirements", False),
            "condition": "condition" in p,
            "source_treated_as": p.get("source_treated_as"),
        }
        out.append(entry)
    return out


def yaml_digivolve_colors_levels(alt_paths):
    """Collect (color, level, cost) tuples a YAML digivolve alt_path pins explicitly."""
    tuples = []
    for p in alt_paths:
        if p["kind"] != "digivolve":
            continue
        frm = p.get("from")
        if not isinstance(frm, dict):
            tuples.append((None, None, p.get("cost")))
            continue
        col = frm.get("color_is")
        lvl = frm.get("level_eq")
        tuples.append((col, lvl, p.get("cost")))
    return tuples


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=None)
    args = ap.parse_args()

    root = repo_root()
    cards = json.loads(open(os.path.join(root, "data", "cards.json"), encoding="utf-8").read())
    base_dcgo = os.path.join(root, "DCGO")

    yamls = sorted(glob.glob(os.path.join(root, "code", "digimon-engine", "cards", "*", "*.yaml")))
    records = []
    smell_counts = {}

    for ypath in yamls:
        cid = os.path.splitext(os.path.basename(ypath))[0]
        c = cards.get(cid)
        if not c:
            continue
        if c.get("card_kind") != 0:  # 0 = Digimon
            continue
        level = c.get("level")
        if not level or level < 3:
            continue

        try:
            spec = yaml.safe_load(open(ypath, encoding="utf-8").read()) or {}
        except Exception as e:
            spec = {}
        alt = normalize_alt_paths(spec)

        colors = c.get("card_colors") or []
        evo = c.get("evo_costs") or []
        evo_colors = sorted({e["card_color"] for e in evo})
        xros = (c.get("xros_req") or "").replace("\r", " ").replace("\xa0", " ").strip()
        dna = c.get("dna_costs") or []
        eff = (c.get("effect_description_eng") or "").replace("\r", " ").strip()

        smells = []
        # S1 — xros_req declares extra digivolve/DNA path(s) but YAML has none of that kind.
        has_dig_alt = any(p["kind"] == "digivolve" for p in alt)
        has_dna_alt = any(p["kind"] == "dna_digivolve" for p in alt)
        if xros:
            if re.search(r"\[?Digivolve\]?", xros) and not has_dig_alt:
                smells.append("S1_xros_digivolve_no_yaml_altpath")
            if re.search(r"DNA\s*Digivolve", xros, re.I) and not has_dna_alt:
                smells.append("S1_xros_dna_no_yaml_altpath")
        if dna and not has_dna_alt:
            smells.append("S1_dna_costs_no_yaml_dna_altpath")

        # S2 — an evo_cost colour not present on the card's own colour identity.
        for e in evo:
            if e["card_color"] not in colors:
                smells.append("S2_evo_color_not_in_card_colors")
                break

        # S3 — multi-colour card but every evo_cost shares one colour (maybe a
        #      dropped off-colour circle; needs vision).
        if len(colors) >= 2 and len(evo_colors) == 1 and evo:
            smells.append("S3_multicolor_card_single_color_evo")

        # S4 — a YAML digivolve alt_path that pins colour+level but whose cost
        #      disagrees with cards.json evo_costs for that colour+level, OR
        #      drops a colour the printed evo_costs carry.
        for (col, lvl, cost) in yaml_digivolve_colors_levels(alt):
            if col is not None and lvl is not None:
                match = [e for e in evo if color_name(e["card_color"]).lower() == str(col).lower() and e["level"] == lvl]
                if match and cost is not None and isinstance(cost, int) and match[0]["memory_cost"] != cost:
                    smells.append("S4_yaml_altpath_cost_mismatch")
                    break
            if col is None and lvl is not None and evo:
                # YAML allows ANY colour at this level but the printed circle is colour-specific
                if len({e["card_color"] for e in evo if e["level"] == lvl}) >= 1 and any(
                    e["level"] == lvl for e in evo
                ):
                    smells.append("S4_yaml_altpath_drops_color")
                    break

        # S5 — Lv>=3 Digimon with NO standard evo_costs and NO YAML digivolve
        #      alt_path: literally no normal digivolution route.
        if not evo and not has_dig_alt and not alt:
            smells.append("S5_no_digivolve_route")

        for s in smells:
            smell_counts[s] = smell_counts.get(s, 0) + 1

        img = os.path.join(IMG_DIR, cid + ".webp")
        records.append(
            {
                "card_id": cid,
                "name": c.get("card_name_eng", ""),
                "set": cid.split("-")[0],
                "level": level,
                "card_colors": colors,
                "card_colors_named": [color_name(x) for x in colors],
                "cards_json_evo_costs": [
                    {"color": color_name(e["card_color"]), "color_code": e["card_color"], "level": e["level"], "cost": e["memory_cost"]}
                    for e in evo
                ],
                "xros_req": xros,
                "dna_costs": dna,
                "effect_text": eff[:400],
                "yaml_path": os.path.relpath(ypath, root).replace("\\", "/"),
                "yaml_alt_paths": alt,
                "image_path": img if os.path.exists(img) else None,
                "dcgo_cs_path": dcgo_cs_path(base_dcgo, cid, colors),
                "smells": smells,
            }
        )

    # Priority sort: cards with more smells first; S5/S1 weigh highest.
    weight = {"S5_no_digivolve_route": 100, "S1_xros_digivolve_no_yaml_altpath": 50,
              "S1_xros_dna_no_yaml_altpath": 50, "S1_dna_costs_no_yaml_dna_altpath": 40,
              "S2_evo_color_not_in_card_colors": 30, "S4_yaml_altpath_cost_mismatch": 25,
              "S4_yaml_altpath_drops_color": 20, "S3_multicolor_card_single_color_evo": 10}
    records.sort(key=lambda r: -sum(weight.get(s, 1) for s in r["smells"]))

    payload = {"count": len(records), "smell_counts": smell_counts, "records": records}
    out = json.dumps(payload, ensure_ascii=False, indent=2)
    if args.out:
        open(args.out, "w", encoding="utf-8").write(out)
    else:
        sys.stdout.write(out)

    print(f"\n=== {len(records)} DSL Digimon (Lv>=3) audited ===", file=sys.stderr)
    flagged = sum(1 for r in records if r["smells"])
    print(f"flagged by >=1 smell: {flagged}  (clean: {len(records) - flagged})", file=sys.stderr)
    for s, n in sorted(smell_counts.items(), key=lambda kv: -kv[1]):
        print(f"  {s}: {n}", file=sys.stderr)


if __name__ == "__main__":
    main()
