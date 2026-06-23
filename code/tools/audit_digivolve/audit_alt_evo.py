#!/usr/bin/env python3
"""Audit implemented cards' YAML alt-digivolve paths against the official Bandai DB.

Compares each implemented card's `alt_paths:` (cost + gate) against the official
`special_digivolution_condition` text (and standard cost circles) in
`data/card_official.json`, flagging authoring bugs that change gameplay:

  - GATE_MISMATCH: official prints "w/[X] trait: Cost N" but the YAML gates that cost on
    `color_is`/`level_eq` instead of `trait_has` (the P-215 class — over-permissive or
    mis-targeted cheaper evo).
  - COST_MISMATCH: the YAML alt cost differs from the printed alt cost.
  - MISSING_ALT: the official prints a special-digivolution alt the YAML doesn't encode.

Heuristic + conservative: it reports CANDIDATES for human/agent review, not auto-fixes.
App-Fusion / DNA / "in text" alts are noted but not deeply checked (different mechanics).

Usage:
    python code/tools/audit_digivolve/audit_alt_evo.py            # table to stdout
    python code/tools/audit_digivolve/audit_alt_evo.py --json out.json
"""
import argparse
import glob
import json
import os
import re
import sys

import yaml

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
OFFICIAL = os.path.join(ROOT, "data", "card_official.json")
CARDS_GLOB = os.path.join(ROOT, "code", "digimon-engine", "cards", "**", "*.yaml")

_COST = re.compile(r"Cost\s+(\d+)", re.I)
_TRAITS = re.compile(r"w/((?:\[[^\]]+\]/?)+)\s*trait", re.I)   # w/[A]/[B] trait
_BRACKET = re.compile(r"\[([^\]]+)\]")


def parse_official_alt(text):
    """Return a list of {kind, cost, gate, traits, raw} parsed from the special-digivolution text."""
    if not text:
        return []
    out = []
    # Split on the leading [Digivolve]/[App Fusion]/[DNA Digivolve] markers.
    for m in re.finditer(r"\[(Digivolve|App Fusion|DNA Digivolve|Blast Digivolve)\](.*?)(?=\[(?:Digivolve|App Fusion|DNA Digivolve|Blast Digivolve)\]|$)", text, re.S | re.I):
        kind = m.group(1).lower().replace(" ", "_")
        body = m.group(2)
        cost_m = _COST.search(body)
        cost = int(cost_m.group(1)) if cost_m else None
        rec = {"kind": kind, "cost": cost, "raw": (("[" + m.group(1) + "]") + body).strip()[:120]}
        tm = _TRAITS.search(body)
        if "in text" in body.lower():
            rec["gate"] = "name_in_text"
            rec["traits"] = _BRACKET.findall(body.split("Cost")[0])
        elif tm:
            rec["gate"] = "trait"
            rec["traits"] = _BRACKET.findall(tm.group(1))
        elif _BRACKET.search(body.split(":")[0] if ":" in body else body):
            rec["gate"] = "name"
            rec["traits"] = _BRACKET.findall(body.split(":")[0])
        else:
            rec["gate"] = "level_colour"
            rec["traits"] = []
        out.append(rec)
    return out


def parse_yaml_alts(path):
    """Return the card's alt_paths as [{kind, cost, gate_kinds:set, traits:set, from:{}}]."""
    try:
        doc = yaml.safe_load(open(path, encoding="utf-8"))
    except Exception:
        return None
    if not isinstance(doc, dict):
        return None
    out = []
    for ap in doc.get("alt_paths", []) or []:
        if not isinstance(ap, dict):
            continue
        frm = ap.get("from") or {}
        gate_kinds, traits = set(), set()
        def walk(node):
            if isinstance(node, dict):
                for k, v in node.items():
                    if k == "trait_has":
                        gate_kinds.add("trait"); traits.add(str(v))
                    elif k in ("color_is", "color_only"):
                        gate_kinds.add("colour")
                    elif k in ("name_is", "name_contains", "name_in"):
                        gate_kinds.add("name")
                    elif k in ("all_of", "any_of"):
                        for x in (v or []): walk(x)
            elif isinstance(node, list):
                for x in node: walk(x)
        walk(frm)
        out.append({"kind": ap.get("kind"), "cost": ap.get("cost"), "gate_kinds": gate_kinds, "traits": traits})
    return out


def audit(official, only=None):
    findings = []
    impl = {os.path.splitext(os.path.basename(p))[0]: p for p in glob.glob(CARDS_GLOB, recursive=True)}
    for cid, path in sorted(impl.items()):
        if only and cid not in only:
            continue
        oc = official.get(cid)
        if not oc:
            continue
        official_alts = parse_official_alt(oc.get("special_digivolution_condition"))
        # Only audit normal-digivolve printed alts (skip App Fusion / DNA — different mechanic).
        official_alts = [a for a in official_alts if a["kind"] == "digivolve" and a["cost"] is not None]
        if not official_alts:
            continue
        yaml_alts = parse_yaml_alts(path)
        if yaml_alts is None:
            continue
        yaml_digi = [a for a in yaml_alts if a.get("kind") == "digivolve"]
        for oa in official_alts:
            # Find YAML alt_paths at the same cost.
            same_cost = [a for a in yaml_digi if a.get("cost") == oa["cost"]]
            if not same_cost:
                # Maybe the cost differs → cost mismatch or missing.
                any_trait = [a for a in yaml_digi if "trait" in a["gate_kinds"]]
                findings.append({
                    "card": cid, "type": "MISSING_OR_COST",
                    "official": oa["raw"], "official_cost": oa["cost"], "official_gate": oa["gate"],
                    "yaml_costs": sorted({a["cost"] for a in yaml_digi if isinstance(a.get("cost"), int)}),
                    "detail": f"no YAML alt_path at cost {oa['cost']} for printed alt",
                })
                continue
            if oa["gate"] == "trait":
                # The printed alt is TRAIT-gated. The matching-cost YAML alt_path(s) must use trait_has.
                trait_gated = [a for a in same_cost if "trait" in a["gate_kinds"]]
                colour_gated = [a for a in same_cost if "colour" in a["gate_kinds"] and "trait" not in a["gate_kinds"]]
                if not trait_gated and colour_gated:
                    findings.append({
                        "card": cid, "type": "GATE_MISMATCH",
                        "official": oa["raw"], "official_cost": oa["cost"], "official_traits": oa["traits"],
                        "detail": f"printed alt is TRAIT-gated ({'/'.join(oa['traits'])}) but YAML gates cost {oa['cost']} on COLOUR/level — over-permissive (P-215 class)",
                    })
    return findings


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--official", default=OFFICIAL)
    ap.add_argument("--json", default=None)
    args = ap.parse_args()
    official = json.load(open(args.official, encoding="utf-8"))["cards"]
    findings = audit(official)
    gate = [f for f in findings if f["type"] == "GATE_MISMATCH"]
    miss = [f for f in findings if f["type"] == "MISSING_OR_COST"]
    print(f"# audited implemented cards vs {len(official)}-card official mirror", file=sys.stderr)
    print(f"# GATE_MISMATCH (trait-vs-colour, P-215 class): {len(gate)}", file=sys.stderr)
    print(f"# MISSING_OR_COST (no YAML alt at printed cost): {len(miss)}", file=sys.stderr)
    print("\n=== GATE_MISMATCH (highest priority — gameplay bug) ===")
    for f in gate:
        print(f"  {f['card']:10} {f['detail']}\n             official: {f['official']}")
    print("\n=== MISSING_OR_COST (review — may be DNA/text-alt/intentional) ===")
    for f in miss:
        print(f"  {f['card']:10} {f['detail']} | yaml costs {f['yaml_costs']} | {f['official'][:70]}")
    if args.json:
        json.dump(findings, open(args.json, "w", encoding="utf-8"), ensure_ascii=False, indent=2)


if __name__ == "__main__":
    main()
