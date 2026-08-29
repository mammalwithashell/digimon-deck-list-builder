"""The work plan splits three kinds of 'not done' and hides none of them."""

import json

import pytest

from tools.clause_coverage.campaign import build_plan


@pytest.fixture
def workspace(tmp_path):
    """A miniature repo: one archetype, two cards, one with a YAML spec."""
    lib = tmp_path / "deck_library.json"
    lib.write_text(
        json.dumps(
            {
                "version": 1,
                "archetypes": {
                    "Fixtures": {
                        "archetype_name": "Fixtures",
                        "decklists": [
                            {"deck_id": "1", "decklist": json.dumps(["EX12-004", "BT8-084"])},
                            {"deck_id": "2", "decklist": json.dumps(["EX12-004"])},
                        ],
                    }
                },
            }
        ),
        encoding="utf-8",
    )

    cards = tmp_path / "cards" / "ex12"
    cards.mkdir(parents=True)
    (cards / "EX12-004.yaml").write_text("card: EX12-004\n", encoding="utf-8")
    # BT8-084 deliberately has NO yaml -> it must land in `implement`.

    verdicts = tmp_path / "exam-verdicts"
    verdicts.mkdir()

    scenarios = tmp_path / "scenarios"
    scenarios.mkdir()
    return {"library": lib, "cards": cards.parent, "verdicts": verdicts, "scenarios": scenarios}


def test_a_card_without_yaml_lands_in_implement(workspace):
    plan = build_plan(
        "Fixtures",
        library_path=workspace["library"],
        cards_dir=workspace["cards"],
        scenarios_dir=workspace["scenarios"],
        verdicts_path=workspace["verdicts"],
    )
    assert "BT8-084" in plan["implement"], "a card with no spec cannot be examined yet"
    assert "BT8-084" not in [c["card_id"] for c in plan["exam"]]


def test_the_plan_reports_its_denominator(workspace):
    plan = build_plan(
        "Fixtures",
        library_path=workspace["library"],
        cards_dir=workspace["cards"],
        scenarios_dir=workspace["scenarios"],
        verdicts_path=workspace["verdicts"],
    )
    d = plan["denominator"]
    for cls in ("confirmed", "diverged", "unreachable", "unavailable", "unmeasured"):
        assert cls in d["by_verdict"], f"missing {cls}: a card must never read as 'passed'"


def test_core_is_reported_with_its_threshold(workspace):
    plan = build_plan(
        "Fixtures",
        library_path=workspace["library"],
        cards_dir=workspace["cards"],
        scenarios_dir=workspace["scenarios"],
        verdicts_path=workspace["verdicts"],
    )
    assert plan["core"]["list_count"] == 2
    assert plan["core"]["threshold"] == 2  # ceil(2 * 0.7)
    assert plan["core"]["cards"] == ["EX12-004"]


def test_an_unknown_archetype_raises_with_suggestions(workspace):
    with pytest.raises(LookupError) as e:
        build_plan(
            "Fixture",  # near-miss
            library_path=workspace["library"],
            cards_dir=workspace["cards"],
            scenarios_dir=workspace["scenarios"],
            verdicts_path=workspace["verdicts"],
        )
    assert "Fixtures" in str(e.value)
