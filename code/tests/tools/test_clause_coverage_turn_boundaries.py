"""Turn-boundary timings observed on five actual official card printings."""
import json
from pathlib import Path

import pytest

from tools.clause_coverage.text_split import split_clauses

FIXTURE = Path(__file__).parents[1] / "fixtures/clause_coverage/turn_timing_observations.json"
OBSERVATIONS = {row["observation"]["extracted"]["card_id"]: row["observation"]
                for row in json.loads(FIXTURE.read_text(encoding="utf-8"))["rows"]}


@pytest.mark.parametrize("card,label,expected", [
    ("BT26-104", "Effect", [["Start of Your Main Phase"], ["On Play"], ["End of Your Turn"]]),
    ("EX5-065", "Effect", [["Your Turn"], ["Start of Opponent's Turn"]]),
    ("BT14-086", "Inherited Effect", [["All Turns"], ["End of All Turns"]]),
    ("BT26-027", "Effect", [["Start of Opponent's Main Phase", "On Play"]]),
    ("BT13-077", "Effect", [["On Play", "When Digivolving"], ["End of Opponent's Turn"]]),
])
def test_real_turn_timings_remain_separate_from_prior_effect(card, label, expected):
    text = next(section["text"] for section in OBSERVATIONS[card]["extracted"]["text_sections"]
                if section["label"] == label)
    spans = split_clauses(text)
    assert [span.timings for span in spans if span.kind == "timing"] == expected
    if card == "BT26-104":
        assert spans[1].text == "By trashing 1 [Shambala] trait card from your hand, ＜Draw 2＞."
        assert spans[2].text == (
            "If you have a Digimon with the [Tentei Hachibushu] trait, by suspending this Tamer, "
            "you may use 1 Option card with the [Shambala] trait from your hand without paying the cost.")


@pytest.mark.parametrize("timing", ["End of Your Turn", "End of Opponent's Turn",
    "End of All Turns", "Start of Opponent's Turn", "Start of Opponent's Main Phase"])
def test_inline_reference_to_turn_timing_does_not_start_another_effect(timing):
    text = f"[Main] Activate this card's [{timing}] effect."
    spans = split_clauses(text)
    assert len(spans) == 1
    assert spans[0].timings == ["Main"]
    assert spans[0].text == f"Activate this card's [{timing}] effect."
