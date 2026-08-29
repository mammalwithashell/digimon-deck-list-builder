"""Render the human-facing exam index from the ledger.

This file is **generated**. `qa/qa-reports/exam-index.md` is regenerated from
the per-card verdict files, the scenario corpus and the clause denominator, and
a test asserts the rendering is deterministic -- so a hand-edit shows up as a
failing build rather than as silent drift.

It answers one question: *what should I dispatch next?* Hence the sort by
`unmeasured` descending -- the archetypes with the most unproven clauses first.

Standard library only, matching the rest of `tools/clause_coverage/`.
"""

from __future__ import annotations

import argparse
from pathlib import Path

VERDICT_COLUMNS = (
    "confirmed",
    "diverged",
    "unreachable",
    "unavailable",
    "unmeasured",
)


def render_index(rows: list[dict], generated_from: str) -> str:
    """Render the index.

    `rows` is ``[{"archetype": str, "cards": [card_id], "binding": bind_result}]``.

    Raises ``ValueError`` if a row's five verdict counts do not sum to its
    denominator. That sum is an invariant of ``exam_binding.bind()`` (one class
    is appended per clause in a single loop); if it is ever violated the index
    must refuse rather than publish a total nobody can trust.
    """
    for row in rows:
        binding = row["binding"]
        by_verdict = binding["denominator"]["by_verdict"]
        total = binding["denominator"]["total_clauses"]
        got = sum(by_verdict.get(k, 0) for k in VERDICT_COLUMNS)
        if got != total:
            raise ValueError(
                f"{row['archetype']}: verdict counts sum to {got}, "
                f"denominator is {total} -- refusing to render"
            )

    ordered = sorted(
        rows,
        key=lambda r: (
            -r["binding"]["denominator"]["by_verdict"].get("unmeasured", 0),
            r["archetype"],
        ),
    )

    out: list[str] = []
    out.append("# DCGO exam index")
    out.append("")
    out.append(
        "**Generated — do not hand-edit.** Regenerate with "
        "`python -m tools.clause_coverage.exam_index`."
    )
    out.append("")
    out.append(
        "Every row prints the full denominator. An archetype is never "
        '"passed"; it is a count per verdict class, and `unmeasured` is as '
        "real an outcome as `confirmed`."
    )
    out.append("")
    out.append(f"Source: {generated_from}")
    out.append("")
    out.append(
        "| Archetype | Cards | Clauses | "
        + " | ".join(c.capitalize() for c in VERDICT_COLUMNS)
        + " | Measured |"
    )
    out.append("|---|---|---|" + "---|" * (len(VERDICT_COLUMNS) + 1))

    for row in ordered:
        binding = row["binding"]
        by_verdict = binding["denominator"]["by_verdict"]
        total = binding["denominator"]["total_clauses"]
        measured = total - by_verdict.get("unmeasured", 0)
        pct = f"{(100 * measured / total):.0f}%" if total else "n/a"
        cells = " | ".join(str(by_verdict.get(c, 0)) for c in VERDICT_COLUMNS)
        out.append(
            f"| {row['archetype']} | {len(row['cards'])} | {total} | {cells} | {pct} |"
        )

    out.append("")
    return "\n".join(out)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, default=Path("qa/qa-reports/exam-index.md"))
    parser.add_argument("--verdicts", type=Path, default=Path("qa/qa-reports/exam-verdicts"))
    parser.add_argument("--library", type=Path, default=Path("data/deck_library.json"))
    parser.add_argument("--cards-dir", type=Path, default=Path("code/digimon-engine/cards"))
    parser.add_argument("--scenarios-dir", type=Path, default=Path("qa/dcgo-exams"))
    parser.add_argument(
        "--min-clauses",
        type=int,
        default=1,
        help="skip archetypes whose pool yields fewer clauses than this",
    )
    parser.add_argument(
        "--only",
        action="append",
        default=None,
        help="restrict to these archetypes (repeatable, or comma-separated); "
        "for scoping a slow run, never for silently sampling the full index",
    )
    args = parser.parse_args(argv)

    from tools.clause_coverage import archetype as archetype_mod
    from tools.clause_coverage.campaign import build_plan

    library = archetype_mod.load_archetypes(args.library)
    names = sorted(library)
    if args.only:
        wanted: list[str] = []
        for entry in args.only:
            wanted.extend(part.strip() for part in entry.split(",") if part.strip())
        names = sorted({archetype_mod.resolve(library, w) for w in wanted})

    rows = []
    for name in names:
        try:
            plan = build_plan(
                name,
                library_path=args.library,
                cards_dir=args.cards_dir,
                scenarios_dir=args.scenarios_dir,
                verdicts_path=args.verdicts,
            )
        except (LookupError, ValueError):
            # An archetype the library lists but cannot be resolved into a pool
            # is skipped rather than rendered as zeros -- a row of zeros reads
            # as "measured and empty", which is the opposite of the truth.
            continue
        total = plan["denominator"].get("total_clauses", 0)
        if total < args.min_clauses:
            continue
        rows.append({"archetype": plan["archetype"], "cards": plan["pool"],
                     "binding": {"denominator": plan["denominator"],
                                 "total_clauses": total}})

    text = render_index(rows, generated_from=str(args.verdicts))
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(text, encoding="utf-8")
    print(f"wrote {args.out} ({len(rows)} archetypes)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
