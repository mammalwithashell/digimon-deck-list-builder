"""The exam index is generated, never hand-edited.

A hand-edit must therefore be a failing build rather than silent drift, which
means rendering has to be deterministic: same ledger in, byte-identical
Markdown out.
"""

from tools.clause_coverage.exam_index import render_index


def _binding(total, confirmed, diverged, unreachable, unavailable, unmeasured):
    """Match the real shape `exam_binding.bind()` returns: `total_clauses`
    (and `total_cards`, `by_zone`) live nested under `denominator`, alongside
    `by_verdict` -- there is no top-level `total_clauses` key. `render_index`
    only reads `by_verdict` and `total_clauses`; `total_cards`/`by_zone` are
    included anyway so the fixture is honestly representative of a real
    `bind()` result."""
    return {
        "denominator": {
            "total_clauses": total,
            "total_cards": 1,
            "by_verdict": {
                "confirmed": confirmed,
                "diverged": diverged,
                "unreachable": unreachable,
                "unavailable": unavailable,
                "unmeasured": unmeasured,
            },
            "by_zone": {"effect": total},
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


def test_main_renders_archetypes_that_have_exam_activity(tmp_path, monkeypatch):
    """The index exists to answer 'what should I dispatch next', which it
    cannot do while it renders nothing."""
    import json

    from tools.clause_coverage import exam_index

    lib = tmp_path / "deck_library.json"
    lib.write_text(json.dumps({
        "version": 1,
        "archetypes": {
            "Fixtures": {"archetype_name": "Fixtures", "decklists": [
                {"deck_id": "1", "decklist": json.dumps(["EX12-004"])}]}
        },
    }), encoding="utf-8")

    cards = tmp_path / "cards"
    cards.mkdir()
    verdicts = tmp_path / "exam-verdicts"
    verdicts.mkdir()
    scenarios = tmp_path / "scenarios"
    scenarios.mkdir()
    out = tmp_path / "exam-index.md"

    rc = exam_index.main([
        "--out", str(out), "--library", str(lib), "--cards-dir", str(cards),
        "--scenarios-dir", str(scenarios), "--verdicts", str(verdicts),
    ])
    assert rc == 0
    text = out.read_text(encoding="utf-8")
    assert "Fixtures" in text, "an archetype with a pool must appear in the index"
