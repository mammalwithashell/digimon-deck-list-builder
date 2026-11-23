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


def _bt20_099_paid_play(selection_method, selected_index=55, **play_overrides):
    # BT20-099's effect selects an ACCEL Digimon from hand, then plays it for its cost.
    card = "BT20-030"
    game = _raw_game(cards=[card], card_indexes=[55], root="Hand", **play_overrides)
    game[0]["players"][1]["hand"][1:] = [card, card]
    for entry in game[2]["hand"]["cards"][1:]:
        entry["card_id"] = card
    game.insert(2, {"method": selection_method, "args": [1, [selected_index], [0]]
                    if selection_method.startswith("SelectCardEffect.") else [1, [selected_index]],
                    "card_ids": [card], "zone": "Hand", "tp": 1, "turn": 2})
    return game


@pytest.mark.parametrize("selection_method", [
    "SelectHandEffect.SetTargetHandCards",
    "SelectCardEffect.SetTargetCardAndIndicies",
])
def test_paid_effect_play_is_not_emitted_after_the_selection(selection_method):
    raw = _bt20_099_paid_play(selection_method, play_provenance="effect")
    rows, _ = convert_raw.convert(raw, cards={})
    [pick] = [row for row in rows if row["type"] == "selection"]
    assert (pick["actor"], pick["card_ids"], pick["card_indexes"]) == (1, ["BT20-030"], [55])
    assert not [row for row in rows if row.get("card_id") == "BT20-030"]


@pytest.mark.parametrize("selection_method", [
    "SelectHandEffect.SetTargetHandCards",
    "SelectCardEffect.SetTargetCardAndIndicies",
])
def test_legacy_paid_effect_play_with_matching_hand_pick_is_unsupported(selection_method):
    raw = _bt20_099_paid_play(selection_method)
    with pytest.raises(SystemExit, match="unsupported decision .*ambiguous paid bot play.*CardIndex 55"):
        convert_raw.convert(raw, cards={})


def test_legacy_bot_reveal_then_play_is_not_silently_dropped():
    # BT13-046 can reveal a non-yellow card and leave it in hand to be played normally.
    raw = _raw_game()
    raw.insert(2, {"method": "SelectHandEffect.SetTargetHandCards", "args": [1, [55]],
                   "card_ids": ["ST1-03"], "tp": 1, "turn": 2})
    with pytest.raises(SystemExit, match="unsupported decision .*ambiguous paid bot play.*CardIndex 55"):
        convert_raw.convert(raw, cards={})
    rows, _ = convert_raw.convert(raw, cards={}, allow_truncated=True)
    assert rows[-1]["type"] == "selection"
    assert rows[-1]["card_indexes"] == [55]


@pytest.mark.parametrize("boundary", ["different_copy", "different_turn", "direct_set", "direct_provenance"])
def test_paid_bot_play_is_not_claimed_by_an_unrelated_selection(boundary):
    raw = _bt20_099_paid_play("SelectHandEffect.SetTargetHandCards")
    if boundary == "different_copy":
        raw[2]["args"][1] = [56]
    elif boundary == "different_turn":
        raw[2]["turn"] -= 1
    elif boundary == "direct_set":
        raw.insert(3, {"method": "TurnStateMachine.set_PlayCard", "args": [], "tp": 1, "turn": 2})
    else:
        raw[3]["play_provenance"] = "direct"
    rows, _ = convert_raw.convert(raw, cards={})
    [play] = [row for row in rows if row.get("card_id") == "BT20-030"]
    assert (play["actor"], play["action_id"], play["card_index"]) == (1, 1, 55)


def test_explicit_effect_play_is_not_a_decision_without_a_selection():
    rows, _ = convert_raw.convert(_raw_game(play_provenance="effect"), cards={})
    assert not [row for row in rows if row.get("card_id") == "ST1-03"]


@pytest.mark.parametrize("overrides", [{"is_ai": False}, {"pay_cost": False}, {"tp": 0}])
def test_unqueued_non_main_plays_are_not_decisions(overrides):
    rows, _ = convert_raw.convert(_raw_game(**overrides), cards={})
    assert not [row for row in rows if row.get("card_id") == "ST1-03"]


def test_unmappable_paid_bot_play_reports_unsupported_decision():
    with pytest.raises(SystemExit, match="digivolve of ST1-03 with an unlocatable target"):
        convert_raw.convert(_raw_game(target={"card_id": "ST1-01"}), cards={})
