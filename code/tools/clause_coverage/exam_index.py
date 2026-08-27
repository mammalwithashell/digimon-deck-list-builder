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
    parser.add_argument(
        "--out",
        type=Path,
        default=Path("qa/qa-reports/exam-index.md"),
        help="where to write the index",
    )
    parser.add_argument(
        "--verdicts",
        type=Path,
        default=Path("qa/qa-reports/exam-verdicts"),
        help="per-card verdict directory",
    )
    args = parser.parse_args(argv)

    # Archetype -> card list resolution lands with the campaign skill (plan 4).
    # Until then the index renders whatever rows a caller supplies; this
    # entrypoint writes an empty index rather than inventing an archetype map.
    text = render_index([], generated_from=str(args.verdicts))
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(text, encoding="utf-8")
    print(f"wrote {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
