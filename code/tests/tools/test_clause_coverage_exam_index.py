"""The exam index is generated, never hand-edited.

A hand-edit must therefore be a failing build rather than silent drift, which
means rendering has to be deterministic: same ledger in, byte-identical
Markdown out.
"""

from tools.clause_coverage.exam_index import render_index


def _binding(total, confirmed, diverged, unreachable, unavailable, unmeasured):
    return {
        "total_clauses": total,
        "denominator": {
            "by_verdict": {
                "confirmed": confirmed,
                "diverged": diverged,
                "unreachable": unreachable,
                "unavailable": unavailable,
                "unmeasured": unmeasured,
            }
        },
    }


def test_render_is_deterministic():
    rows = [
        {"archetype": "Toho Braves", "cards": ["EX12-035"],
         "binding": _binding(166, 107, 0, 5, 0, 54)},
        {"archetype": "Hunters", "cards": ["BT12-042"],
         "binding": _binding(65, 0, 0, 0, 0, 65)},
    ]
    assert render_index(rows, "ledger") == render_index(rows, "ledger")


def test_every_row_prints_all_five_classes():
    """A card must never read as 'passed' on a partial denominator."""
    rows = [{"archetype": "Toho Braves", "cards": ["EX12-035"],
             "binding": _binding(166, 107, 0, 5, 0, 54)}]
    out = render_index(rows, "ledger")
    for column in ("confirmed", "diverged", "unreachable", "unavailable", "unmeasured"):
        assert column in out.lower()
    assert "107" in out and "54" in out


def test_counts_that_do_not_sum_to_the_denominator_are_rejected():
    """by_verdict summing to total is an invariant of bind(); if it ever
    breaks, the index must refuse rather than publish a lie."""
    rows = [{"archetype": "Broken", "cards": ["X-1"],
             "binding": _binding(10, 1, 0, 0, 0, 0)}]
    try:
        render_index(rows, "ledger")
    except ValueError as e:
        assert "sum" in str(e).lower()
    else:
        raise AssertionError("must reject counts that do not sum to the denominator")


def test_archetypes_sort_by_unmeasured_descending():
    """The index exists to answer 'what should I dispatch next'."""
    rows = [
        {"archetype": "Nearly Done", "cards": ["A-1"], "binding": _binding(10, 9, 0, 0, 0, 1)},
        {"archetype": "Untouched", "cards": ["B-1"], "binding": _binding(10, 0, 0, 0, 0, 10)},
    ]
    out = render_index(rows, "ledger")
    assert out.index("Untouched") < out.index("Nearly Done")
