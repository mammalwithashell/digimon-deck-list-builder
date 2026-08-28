"""Turn an archetype into a unit of dispatchable work.

Three kinds of "not done", kept apart because they cost different things:

- ``implement`` -- no YAML spec exists, so the card cannot be examined at all
  and the work is card authoring.
- ``exam`` -- a spec exists and clauses are outstanding; the work is scenario
  authoring against the oracle.
- ``skipped`` -- confirmed (with unchanged text) or ``unavailable`` because DCGO
  has no script for the card. **Reported with its reason, never hidden**: a plan
  that silently omits skipped work reads as though the work does not exist.

The denominator comes from `exam_binding.bind`, not from a second computation
here. One denominator, one producer.

`bind()`'s real return shape (verified against `exam_binding.py`, NOT the
top-level ``"clauses"`` list a draft of this module assumed) is::

    {"cards": {card_id: {"card_id": ..., "total_clauses": N,
                         "by_verdict": {...}, "clauses": [
        {"clause_id": ..., "zone": ..., "label": ..., "kind": ...,
         "text": ..., "source": ..., "verdict": ..., "invalidated": ...,
         "reason": ..., "scenarios": [...], ...},
        ...
     ]}},
     "denominator": {"total_clauses": N, "total_cards": M,
                     "by_verdict": {...}, "by_zone": {...}},
     ...}

A per-clause dict carries no ``card_id`` of its own -- the card id is the key
of the enclosing ``cards`` dict, so this module walks ``binding["cards"]``
rather than a flat clause list, and reconstructs each clause's ``clause_id``
that way.

Standard library only.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from tools.clause_coverage import archetype as archetype_mod
from tools.clause_coverage.exam_binding import bind


def _has_yaml(cards_dir: Path, card_id: str) -> bool:
    """Does a DSL spec exist for this card, in any set directory?"""
    return any(cards_dir.rglob(f"{card_id}.yaml"))


def build_plan(
    archetype: str,
    *,
    library_path: Path | str,
    cards_dir: Path | str,
    scenarios_dir: Path | str,
    verdicts_path: Path | str,
    core_fraction: float = archetype_mod.DEFAULT_CORE_FRACTION,
    limit: int | None = None,
) -> dict:
    """Build the work plan for one archetype."""
    library = archetype_mod.load_archetypes(library_path)
    canonical = archetype_mod.resolve(library, archetype)
    entry = library[canonical]

    cards_dir = Path(cards_dir)
    card_pool = archetype_mod.pool(entry)
    core = archetype_mod.core(entry, core_fraction)

    implement = [c for c in card_pool if not _has_yaml(cards_dir, c)]
    examinable = [c for c in card_pool if c not in implement]

    binding = bind(card_pool, scenarios_dir, verdicts_path)

    exam: list[dict] = []
    skipped: list[dict] = []
    cards_report = binding.get("cards", {})
    for card_id in examinable:
        card_entry = cards_report.get(card_id, {})
        for clause in card_entry.get("clauses", []):
            clause_id = clause["clause_id"]
            verdict = clause.get("verdict", "unmeasured")
            if verdict == "confirmed":
                skipped.append({
                    "clause_id": clause_id,
                    "card_id": card_id,
                    "reason": "confirmed",
                })
                continue
            if verdict == "unavailable":
                skipped.append({
                    "clause_id": clause_id,
                    "card_id": card_id,
                    "reason": clause.get("reason") or "DCGO has no script for this card",
                })
                continue
            exam.append({
                "clause_id": clause_id,
                "card_id": card_id,
                "label": clause.get("label", ""),
                "verdict": verdict,
                "is_core": card_id in core["cards"],
            })

    # Core clauses first: the campaign's done-criterion is defined on the core,
    # so working the tail first would delay the only gate that matters.
    exam.sort(key=lambda c: (not c["is_core"], c["clause_id"]))
    exam_total = len(exam)
    if limit is not None:
        exam = exam[:limit]

    return {
        "archetype": canonical,
        "pool": card_pool,
        "examinable": examinable,
        "core": core,
        "implement": implement,
        "exam": exam,
        "exam_total": exam_total,
        "elided": exam_total - len(exam),
        "skipped": skipped,
        "denominator": binding.get("denominator", {}),
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archetype", required=True)
    parser.add_argument("--library", type=Path, default=Path("data/deck_library.json"))
    parser.add_argument("--cards-dir", type=Path, default=Path("code/digimon-engine/cards"))
    parser.add_argument("--scenarios-dir", type=Path, default=Path("qa/dcgo-exams"))
    parser.add_argument("--verdicts", type=Path, default=Path("qa/qa-reports/exam-verdicts"))
    parser.add_argument("--core-fraction", type=float, default=archetype_mod.DEFAULT_CORE_FRACTION)
    parser.add_argument("--limit", type=int, default=None)
    parser.add_argument("--json", action="store_true", help="machine-readable output")
    args = parser.parse_args(argv)

    plan = build_plan(
        args.archetype,
        library_path=args.library,
        cards_dir=args.cards_dir,
        scenarios_dir=args.scenarios_dir,
        verdicts_path=args.verdicts,
        core_fraction=args.core_fraction,
        limit=args.limit,
    )

    if args.json:
        print(json.dumps(plan, indent=2, sort_keys=True))
        return 0

    core = plan["core"]
    d = plan["denominator"].get("by_verdict", {})
    print(f"{plan['archetype']} — {len(plan['pool'])} cards, "
          f"core {len(core['cards'])} (>={core['threshold']} of {core['list_count']} lists)")
    print(f"  implement : {len(plan['implement'])} cards with no YAML spec")
    print(f"  exam      : {plan['exam_total']} outstanding clauses "
          f"({sum(1 for c in plan['exam'] if c['is_core'])} shown are core)")
    print(f"  skipped   : {len(plan['skipped'])} (confirmed or unavailable)")
    if d:
        print("  denominator: " + ", ".join(f"{k} {v}" for k, v in sorted(d.items())))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
