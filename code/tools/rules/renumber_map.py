"""Build a rule-number migration map between two Comprehensive Rules versions.

Pairs old and new rule units by text (cross references and rule numbers are
already neutralised in ``RuleUnit.text_norm``), then classifies every unit:

* ``renumbered`` -- same text, different id (``{"<old>": "<new>"}``)
* ``reworded``   -- paired but the wording changed (similarity < 0.97),
                    ``{"old", "new", "similarity"}`` (old == new when the id held)
* ``added``      -- new ids with no old counterpart
* ``removed``    -- old ids with no new counterpart
* ``unchanged``  -- count of same-id, same-text pairs (not listed)

Pairing passes, each iterating in document order with ties going to the first
candidate: (1) same id + identical text, (2) identical text elsewhere,
(3) fuzzy ``SequenceMatcher`` ratio >= 0.6, preferring the same id when its
ratio is >= 0.9, else the best unused old unit, (4) any old unit still unpaired
whose id also exists as a still-unpaired new unit is paired when the ratio is
>= 0.4 and reported ``reworded`` -- a rule that kept its number but was
rewritten (e.g. a reversed ruling) is the most important migration case.

An id may legitimately appear in both ``added`` and ``removed``: the number was
reused for unrelated text (similarity below 0.4, 15 such ids today). That is a
reuse of the number, not a collision -- the old rule is gone and the new one is
new.
"""
from __future__ import annotations

import argparse
import difflib
import json
from pathlib import Path

from tools.rules.parse_rules import RuleUnit, parse_rules

FUZZY_MIN = 0.6
SAME_ID_REWRITE_MIN = 0.4
SAME_ID_ACCEPT = 0.9
RENUMBER_MIN = 0.97


def _ratio(a: str, b: str) -> float:
    return difflib.SequenceMatcher(None, a, b).ratio()


def _pair(old_units: list[RuleUnit], new_units: list[RuleUnit]) -> dict[str, tuple[str, float]]:
    on = {u.id: u.text_norm for u in old_units}
    nn = {u.id: u.text_norm for u in new_units}
    old_order = [u.id for u in old_units]
    new_order = [u.id for u in new_units]

    pairs: dict[str, tuple[str, float]] = {}  # new_id -> (old_id, ratio)
    used_old: set[str] = set()

    # Pass 1: same id, identical text.
    for r in new_order:
        if r in on and on[r] == nn[r]:
            pairs[r] = (r, 1.0)
            used_old.add(r)

    # Pass 2: identical text elsewhere (renumbered).
    by_text: dict[str, list[str]] = {}
    for o in old_order:
        if o not in used_old:
            by_text.setdefault(on[o], []).append(o)
    for r in new_order:
        if r in pairs:
            continue
        cands = by_text.get(nn[r])
        if cands:
            o = cands.pop(0)
            pairs[r] = (o, 1.0)
            used_old.add(o)

    # Pass 3: fuzzy.
    for r in new_order:
        if r in pairs:
            continue
        best: str | None = None
        best_ratio = 0.0
        if r in on and r not in used_old:
            best, best_ratio = r, _ratio(on[r], nn[r])
        if best_ratio < SAME_ID_ACCEPT:
            for o in old_order:
                if o in used_old:
                    continue
                if abs(len(on[o]) - len(nn[r])) > max(60, 0.6 * len(nn[r])):
                    continue  # cheap length prefilter
                ratio = _ratio(on[o], nn[r])
                if ratio > best_ratio:
                    best, best_ratio = o, ratio
        if best is not None and best_ratio >= FUZZY_MIN:
            pairs[r] = (best, best_ratio)
            used_old.add(best)

    # Pass 4: a rule that kept its number but was rewritten.
    for r in new_order:
        if r in pairs or r not in on or r in used_old:
            continue
        ratio = _ratio(on[r], nn[r])
        if ratio >= SAME_ID_REWRITE_MIN:
            pairs[r] = (r, ratio)
            used_old.add(r)
    return pairs


def build_map(old_units: list[RuleUnit], new_units: list[RuleUnit]) -> dict:
    pairs = _pair(old_units, new_units)
    used_old = {o for o, _ in pairs.values()}

    renumbered: dict[str, str] = {}
    reworded: list[dict] = []
    unchanged = 0
    for u in new_units:
        if u.id not in pairs:
            continue
        old_id, ratio = pairs[u.id]
        if ratio >= RENUMBER_MIN:
            if old_id != u.id:
                renumbered[old_id] = u.id
            else:
                unchanged += 1
        else:
            reworded.append({"old": old_id, "new": u.id, "similarity": round(ratio, 2)})

    return {
        "renumbered": renumbered,
        "reworded": reworded,
        "added": [u.id for u in new_units if u.id not in pairs],
        "removed": [u.id for u in old_units if u.id not in used_old],
        "unchanged": unchanged,
        "stats": {"old": len(old_units), "new": len(new_units), "paired": len(pairs)},
    }


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--old", type=Path, required=True)
    ap.add_argument("--new", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args(argv)
    m = build_map(parse_rules(args.old), parse_rules(args.new))
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open("w", encoding="utf-8", newline="\n") as f:
        f.write(json.dumps(m, sort_keys=True, indent=2, ensure_ascii=False) + "\n")
    s = m["stats"]
    print(
        f"old {s['old']} new {s['new']} paired {s['paired']}: "
        f"{len(m['renumbered'])} renumbered, {len(m['reworded'])} reworded, "
        f"{m['unchanged']} unchanged, {len(m['added'])} added, {len(m['removed'])} removed"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
