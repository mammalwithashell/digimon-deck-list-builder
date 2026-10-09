"""Turn an archetype -- or any resolved card pool -- into a unit of dispatchable work.

`build_plan_for_pool(pool, core, ranking=...)` is the planner; `build_plan(archetype)`
is a thin wrapper that resolves an archetype to a pool + core first. The card
loop (`tools.card_loop.workset`) resolves cards / sets / decklists / archetypes
into a work set and plans through the same function (`--work-set plan.json`).

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
from typing import Sequence

from tools.clause_coverage import archetype as archetype_mod
from tools.clause_coverage.exam_binding import bind


def _has_yaml(cards_dir: Path, card_id: str) -> bool:
    """Does a DSL spec exist for this card, in any set directory?"""
    return any(cards_dir.rglob(f"{card_id}.yaml"))


def _yaml_ids(cards_dir: Path) -> set[str]:
    """Every card id with a DSL spec -- `_has_yaml` for a whole pool in one walk."""
    return {p.stem for p in cards_dir.rglob("*.yaml")} if cards_dir.is_dir() else set()


def _core_info(core: Sequence[str] | dict) -> dict:
    """Accept `archetype.core()`'s dict or a bare card list (a resolved work
    set's core). A bare list has no threshold to report, so those are None
    rather than invented."""
    if isinstance(core, dict):
        return core
    return {"cards": sorted(core), "threshold": None, "list_count": None, "fraction": None}


def build_plan_for_pool(
    pool: Sequence[str],
    core: Sequence[str] | dict,
    *,
    cards_dir: Path | str,
    scenarios_dir: Path | str,
    verdicts_path: Path | str,
    ranking: Sequence[str] | None = None,
    label: str | None = None,
    limit: int | None = None,
) -> dict:
    """Build the work plan for any resolved card pool.

    `pool`/`core`/`ranking` are a resolved work set's fields (e.g.
    `tools.card_loop.workset.WorkSet`, or a card-loop `plan.json`); this module
    does not import the card loop, so it takes the fields, not the type.
    Ordering: core before tail (the done-criterion is defined on the core),
    then `ranking` order when given, then clause id.
    """
    cards_dir = Path(cards_dir)
    card_pool = list(pool)
    core_info = _core_info(core)
    core_cards = set(core_info["cards"])
    rank = {c: i for i, c in enumerate(ranking)} if ranking else {}
    unranked = len(rank)

    with_spec = _yaml_ids(cards_dir)
    implement = [c for c in card_pool if c not in with_spec]
    if rank:
        implement.sort(key=lambda c: rank.get(c, unranked))
    examinable = [c for c in card_pool if c in with_spec]

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
                "is_core": card_id in core_cards,
            })

    # Core clauses first: the campaign's done-criterion is defined on the core,
    # so working the tail first would delay the only gate that matters.
    exam.sort(key=lambda c: (not c["is_core"], rank.get(c["card_id"], unranked), c["clause_id"]))
    exam_total = len(exam)
    if limit is not None:
        exam = exam[:limit]

    return {
        "archetype": None,
        "label": label,
        "pool": card_pool,
        "ranking": list(ranking) if ranking else None,
        "examinable": examinable,
        "core": core_info,
        "implement": implement,
        "exam": exam,
        "exam_total": exam_total,
        "elided": exam_total - len(exam),
        "skipped": skipped,
        "denominator": binding.get("denominator", {}),
    }


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
    """Build the work plan for one archetype -- a thin wrapper that resolves
    the archetype to a pool + core and hands them to `build_plan_for_pool`."""
    library = archetype_mod.load_archetypes(library_path)
    canonical = archetype_mod.resolve(library, archetype)
    entry = library[canonical]
    plan = build_plan_for_pool(
        archetype_mod.pool(entry),
        archetype_mod.core(entry, core_fraction),
        cards_dir=cards_dir,
        scenarios_dir=scenarios_dir,
        verdicts_path=verdicts_path,
        label=canonical,
        limit=limit,
    )
    plan["archetype"] = canonical
    return plan


def load_work_set(path: Path | str) -> dict:
    """A resolved work set from JSON: a card-loop `plan.json` (its
    `work_set`) or a bare `{pool, core, ranking}` object."""
    data = json.loads(Path(path).read_text(encoding="utf-8"))
    work_set = data.get("work_set", data)
    for key in ("pool", "core"):
        if not isinstance(work_set.get(key), list):
            raise ValueError(f"{path}: work set has no {key!r} list")
    label = data.get("run_id") or Path(path).stem
    return {"pool": work_set["pool"], "core": work_set["core"],
            "ranking": work_set.get("ranking"), "label": label}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--archetype")
    source.add_argument("--work-set", type=Path,
                        help="a resolved work set: card-loop plan.json or {pool, core, ranking}")
    parser.add_argument("--library", type=Path, default=Path("data/deck_library.json"))
    parser.add_argument("--cards-dir", type=Path, default=Path("code/digimon-engine/cards"))
    parser.add_argument("--scenarios-dir", type=Path, default=Path("qa/dcgo-exams"))
    parser.add_argument("--verdicts", type=Path, default=Path("qa/qa-reports/exam-verdicts"))
    parser.add_argument("--core-fraction", type=float, default=archetype_mod.DEFAULT_CORE_FRACTION)
    parser.add_argument("--limit", type=int, default=None)
    parser.add_argument("--json", action="store_true", help="machine-readable output")
    args = parser.parse_args(argv)

    if args.work_set is not None:
        ws = load_work_set(args.work_set)
        plan = build_plan_for_pool(
            ws["pool"],
            ws["core"],
            ranking=ws["ranking"],
            label=ws["label"],
            cards_dir=args.cards_dir,
            scenarios_dir=args.scenarios_dir,
            verdicts_path=args.verdicts,
            limit=args.limit,
        )
    else:
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
    rule = (f" (>={core['threshold']} of {core['list_count']} lists)"
            if core.get("threshold") is not None else "")
    print(f"{plan['archetype'] or plan['label']} — {len(plan['pool'])} cards, "
          f"core {len(core['cards'])}{rule}")
    print(f"  implement : {len(plan['implement'])} cards with no YAML spec")
    print(f"  exam      : {plan['exam_total']} outstanding clauses "
          f"({sum(1 for c in plan['exam'] if c['is_core'])} shown are core)")
    print(f"  skipped   : {len(plan['skipped'])} (confirmed or unavailable)")
    if d:
        print("  denominator: " + ", ".join(f"{k} {v}" for k, v in sorted(d.items())))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
