"""Regression reduced from the offline raw-20261123T213314Z.jsonl capture."""
import importlib.util
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[3]
_spec = importlib.util.spec_from_file_location(
    "dcgo_convert_raw_bot_play_under_test", ROOT / "code" / "dcgo-melon-recorder" / "convert_raw.py")
convert_raw = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(convert_raw)


def _raw_game(is_ai=True, **play_overrides):
    # The unrelated deck order is omitted. Keep the observed bot hand/slot, with two copies of Agumon.
    return [
        {"method": "Snapshot.PostMulligan", "t": "t", "turn_count": 0, "is_ai": is_ai, "you": 0, "players": [
            {"id": 0, "library": [], "hand": ["ST1-12"], "security": [], "egg_library": [], "breeding": []},
            {"id": 1, "library": [], "hand": ["ST1-15", "ST1-03", "ST1-03"], "security": [],
             "egg_library": [], "breeding": []},
        ]},
        {"method": "TurnStateMachine.SetBreedingPhase", "args": [0, True], "tp": 0, "turn": 1},
        {"method": "PlayCardClass.PlayCard", "seq": 11, "cards": ["ST1-03"], "pay_cost": True,
         "breeding_area": False, "target": None, "tp": 1, "turn": 2,
         "hand": {"player": 1, "slot": 1, "cards": [
             {"card_id": "ST1-15", "card_index": 54},
             {"card_id": "ST1-03", "card_index": 55},
             {"card_id": "ST1-03", "card_index": 56},
         ]}, **play_overrides},
        {"method": "TurnStateMachine.Surrender", "args": [0], "tp": 0, "turn": 3},
    ]


def test_unqueued_paid_bot_play_preserves_the_recorded_copy():
    rows, _ = convert_raw.convert(_raw_game(), cards={})
    [play] = [row for row in rows if row.get("card_id") == "ST1-03"]
    assert (play["actor"], play["action_id"], play["card_index"]) == (1, 1, 55)


@pytest.mark.parametrize("overrides", [{"is_ai": False}, {"pay_cost": False}, {"tp": 0}])
def test_unqueued_non_main_plays_are_not_decisions(overrides):
    rows, _ = convert_raw.convert(_raw_game(**overrides), cards={})
    assert not [row for row in rows if row.get("card_id") == "ST1-03"]


def test_unmappable_paid_bot_play_reports_unsupported_decision():
    with pytest.raises(SystemExit, match="digivolve of ST1-03 with an unlocatable target"):
        convert_raw.convert(_raw_game(target={"card_id": "ST1-01"}), cards={})
