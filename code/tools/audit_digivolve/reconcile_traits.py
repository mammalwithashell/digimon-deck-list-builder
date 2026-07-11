#!/usr/bin/env python3
"""Reconcile card traits from the official Bandai DB into cards.json (via overrides).

The digimoncard.io API behind `data/cards.json` drops printed trait data the official
Bandai DB (`world.digimoncard.com`, scraped into `data/card_official.json`) preserves:

  1. `(Rule) Trait: Has [X] Type/attribute.` grants — entirely absent from cards.json
     (e.g. the Ice-Snow dragons, Mineral/Rock LIBERATOR, Iliad/TS, `[Free]` Royal Knights).
  2. The `form` field — frequently emptied or partial, so traits that live in form (most
     importantly the **Appmon** mechanic trait, printed as `<grade>/Appmon` and required
     by ~37 cards) never reach production `CardData.traits`
     (`card_data.rs` builds traits = form_eng + attribute_eng + type_eng).
  3. `(App Name)` suffixes on app-type traits (`Search (App Name)` vs official `Search`).

The official DB is authoritative for printed data (CLAUDE.md "Printed card data"). This
tool reconciles the trait fields of **in-scope** cards toward the official DB:

  in-scope = has a Rule grant  OR  carries `Appmon` in its official `form`
             OR  its authored DSL `traits:` exceed current production `CardData` traits.

For each in-scope card it proposes, per field (`type_eng` / `form_eng` / `attribute_eng`),
the order-preserving union of (cards.json current ∪ official split ∪ Rule grants for that
field). `apply_overrides` does a wholesale per-field `dict.update`, so each emitted field
carries the FULL value; the union guarantees no existing trait is dropped.

It also reports **(b)-bucket** cards: a DSL spec declares a trait the official DB does NOT
list (an authoring error, e.g. BT10-029 YAML `Lesser` vs official `Major`) — those get a
YAML fix, not a cards.json injection.

Usage:
    python code/tools/audit_digivolve/reconcile_traits.py            # report only
    python code/tools/audit_digivolve/reconcile_traits.py --apply    # write overrides
"""
import argparse
import glob
import json
import os
import re
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
OFFICIAL = os.path.join(ROOT, "data", "card_official.json")
CARDS = os.path.join(ROOT, "data", "cards.json")
OVERRIDES = os.path.join(ROOT, "data", "card_overrides.json")
CARDS_YAML_GLOB = os.path.join(ROOT, "code", "digimon-engine", "cards", "**", "*.yaml")

# Matches every observed printed form of the rule box (see the guard test
# `official_rule_grants.rs` for the same list in Rust):
#   (Rule) Trait: Has [Ice-Snow] Type.        [Rule] Trait: Has [Dragonkin] Type.
#   (Rule) Trait: Has [Free] attribute.       (Rule) Trait: Has [Hybrid] Form.
#   (Rule) Trait: Has [Olympos XII].          (Rule) Trait: Has [Boss] and [D-Brigade].
_RULE = re.compile(
    r"[\(\[]Rule[\)\]]\s*Trait:\s*Has\s*((?:\[[^\]]+\]\s*(?:and\s*)?)+)\s*(Types?|attribute|Form)?",
    re.IGNORECASE,
)
_RULE_NAME = re.compile(r"\[([^\]]+)\]")
_YAML_TRAITS = re.compile(r"^traits:\s*\[([^\]]*)\]", re.M)

# Engine maps each field into the single merged `traits` list.
FIELDS = ("type_eng", "form_eng", "attribute_eng")
OFFICIAL_FIELD = {"type_eng": "type", "form_eng": "form", "attribute_eng": "attribute"}


def load_json(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def split_line(value):
    """Official `type`/`form`/`attribute` is a '/'-joined trait line (or empty)."""
    return [t.strip() for t in str(value or "").split("/") if t.strip()]


def union_preserve(*lists):
    out, seen = [], set()
    for lst in lists:
        for x in lst:
            k = x.strip().lower()
            if k and k not in seen:
                seen.add(k)
                out.append(x.strip())
    return out


def authored_yaml_traits():
    """{card_id: [traits]} parsed from each card YAML's single-line `traits:` field."""
    out = {}
    for p in glob.glob(CARDS_YAML_GLOB, recursive=True):
        cid = os.path.splitext(os.path.basename(p))[0]
        m = _YAML_TRAITS.search(open(p, encoding="utf-8").read())
        if not m:
            continue
        traits = [t.strip().strip('"').strip("'").strip() for t in m.group(1).split(",")]
        out[cid] = [t for t in traits if t]
    return out


def rule_grants(official_card):
    """(granted_types, granted_attributes, granted_forms) parsed from official text.

    A grant with no field word (`Has [Olympos XII].`) is a type grant — the
    engine merges form/attribute/type into one `traits` list, so the bucket
    only decides which cards.json field carries the correction.
    """
    text = " ".join(s.get("text", "") for s in official_card.get("text_sections", []))
    types, attrs, forms = [], [], []
    for m in _RULE.finditer(text):
        suffix = (m.group(2) or "type").lower()
        bucket = attrs if suffix.startswith("attr") else (forms if suffix.startswith("form") else types)
        bucket.extend(name.strip() for name in _RULE_NAME.findall(m.group(1)))
    return types, attrs, forms


def production_traits(cj):
    """How card_data.rs builds CardData.traits: form + attribute + type."""
    out = []
    for f in ("form_eng", "attribute_eng", "type_eng"):
        out += [t for t in (cj.get(f) or [])]
    return out


def reconcile(official_cards, cards, authored):
    """Return (overrides_by_card, yaml_errors_by_card)."""
    overrides, yaml_errors = {}, {}
    for cid, oc in official_cards.items():
        cj = cards.get(cid)
        if not cj:
            continue
        g_types, g_attrs, g_forms = rule_grants(oc)
        off = {f: split_line(oc.get(OFFICIAL_FIELD[f])) for f in FIELDS}
        prod = {t.lower() for t in production_traits(cj)}
        ytr = authored.get(cid, [])
        ytr_missing = [t for t in ytr if t.lower() not in prod]

        is_appmon = any(t.lower() == "appmon" for t in off["form_eng"])
        in_scope = bool(g_types or g_attrs or g_forms) or is_appmon or bool(ytr_missing)
        if not in_scope:
            continue

        # Reconcile the gameplay-trait fields (type/attribute) toward the official DB.
        # `form_eng` carries the evolutionary STAGE (Rookie/Champion/...) which is not a
        # match-on trait — only touch it for Appmon cards (where it also carries the
        # gameplay-relevant `Appmon` mechanic trait + grade) or when a rule box grants a
        # Form trait outright (`(Rule) Trait: Has [Hybrid] Form.`); in the latter case
        # inject ONLY the granted name, not the official stage line.
        reconcile_form = is_appmon or bool(g_forms)
        reconcile_fields = ("type_eng", "attribute_eng") + (("form_eng",) if reconcile_form else ())
        proposed = {}
        for f in reconcile_fields:
            cur = list(cj.get(f) or [])
            if f == "type_eng":
                new = union_preserve(cur, off[f], g_types)
            elif f == "attribute_eng":
                new = union_preserve(cur, off[f], g_attrs)
            elif is_appmon:
                new = union_preserve(cur, off[f], g_forms)
            else:
                new = union_preserve(cur, g_forms)
            if [x.lower() for x in new] != [x.lower() for x in cur]:
                proposed[f] = new
        if proposed:
            overrides[cid] = proposed

        # (b)-bucket: any authored trait that will NOT be in production after the
        # reconciliation above — the guard would fail on these, so each needs a YAML fix
        # (or, if the official DB confirms it, a wider reconciliation). Annotate official
        # status so the two cases are distinguishable.
        new_prod = {t.lower() for t in production_traits({**cj, **proposed})}
        official_all = {t.lower() for fld in off.values() for t in fld} | \
                       {t.lower() for t in g_types + g_attrs + g_forms}
        unresolved = [t for t in ytr if t.lower() not in new_prod]
        if unresolved:
            yaml_errors[cid] = {
                "authored": ytr,
                "unresolved": unresolved,
                "in_official": [t for t in unresolved if t.lower() in official_all],
                "official_type": oc.get("type"),
                "official_form": oc.get("form"),
                "official_attribute": oc.get("attribute"),
            }
    return overrides, yaml_errors


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--official", default=OFFICIAL)
    ap.add_argument("--apply", action="store_true")
    ap.add_argument("--out-report", default=None)
    args = ap.parse_args()

    official_cards = load_json(args.official).get("cards", {})
    cards = load_json(CARDS)
    authored = authored_yaml_traits()
    overrides, yaml_errors = reconcile(official_cards, cards, authored)

    print(f"# scanned {len(official_cards)} official cards", file=sys.stderr)
    print(f"# {len(overrides)} cards get a cards.json trait correction; "
          f"{len(yaml_errors)} flagged for a DSL-spec fix (official DB lacks the trait)",
          file=sys.stderr)
    for cid in sorted(overrides):
        bits = "; ".join(f"{f} -> {overrides[cid][f]}" for f in FIELDS if f in overrides[cid])
        print(f"  {cid:10} {bits}")
    if yaml_errors:
        print("\n# (b)-bucket — DSL spec declares a trait the official DB lacks (fix the YAML):",
              file=sys.stderr)
        for cid in sorted(yaml_errors):
            e = yaml_errors[cid]
            print(f"  {cid:10} authored={e['authored']} unresolved={e['unresolved']} "
                  f"| official type/form/attr = {e['official_type']!r}/{e['official_form']!r}/{e['official_attribute']!r}")

    if args.out_report:
        json.dump({"overrides": overrides, "yaml_errors": yaml_errors},
                  open(args.out_report, "w", encoding="utf-8"), ensure_ascii=False, indent=2)

    if args.apply:
        ov = load_json(OVERRIDES)
        for cid, fields in overrides.items():
            ov.setdefault(cid, {}).update(fields)
        with open(OVERRIDES, "w", encoding="utf-8") as f:
            json.dump(ov, f, ensure_ascii=False, indent=2)
            f.write("\n")
        print(f"# applied {len(overrides)} trait corrections to {OVERRIDES}", file=sys.stderr)


if __name__ == "__main__":
    main()
