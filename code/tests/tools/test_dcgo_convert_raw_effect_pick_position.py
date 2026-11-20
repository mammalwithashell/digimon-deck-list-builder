"""convert_raw.py: a SelectCardEffect row keeps the position DCGO's list held for the pick, and a Homeros effect pick
is no longer dropped by card id.

`SelectCardEffect.SetTargetCardAndIndicies(playerID, int[] cardIndexes, int[] positions)`: the third list is the
position of each pick in the candidate list the prompt showed. BT24-102 Homeros's "Select 1 effect to activate."
(BT24_102.cs:173-215) lists the chosen Digimon's [On Play] / [When Digivolving] effect objects; a printed
`[On Play] [When Digivolving] X` effect is one object per timing tag, so the list holds that card twice and the
recorder logs the picked card once, with its position. The converter writes the position as `indexes`
(docs/DCGO_RECORDING_SCHEMA.md), and the replay (`ReplayDriver::refire_duplicate_effect_pick_fold`) proves from the
ENGINE's state that the row is that duplicate prompt. The converter used to drop the row for ONE card id
(BT24-101) by inspecting its neighbours; that card-specific rule is gone, so every such row reaches the replay.

Synthetic raw rows only: no recording (they carry private deck order) is read.
"""
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
_spec = importlib.util.spec_from_file_location(
    "dcgo_convert_raw_effect_pick_position_under_test", ROOT / "code" / "dcgo-melon-recorder" / "convert_raw.py")
convert_raw = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(convert_raw)

YOU = 0
HOMEROS = "BT24-102"


def _snapshot():
    deck = ["ST1-03"] * 10
    return {"method": "Snapshot.SetRedraw", "t": "t", "you": YOU, "is_ai": False, "players": [
        {"id": 0, "hand": deck[:5], "library": deck[5:], "egg_library": []},
        {"id": 1, "hand": deck[:5], "library": deck[5:], "egg_library": []},
    ]}


def _raw_game(*main_phase):
    return [
        _snapshot(),
        {"method": "TurnStateMachine.SetRedraw", "args": [0, False]},
        {"method": "TurnStateMachine.SetRedraw", "args": [1, False]},
        {"method": "TurnStateMachine.SetBreedingPhase", "tp": YOU, "turn": 1, "args": [YOU, False]},
        *main_phase,
        {"method": "TurnStateMachine.QueueMainPhaseAction_Internal", "tp": YOU, "turn": 1,
         "args": [YOU, convert_raw.PASS_PACKET_ID, []]},
        {"method": "TurnStateMachine.Surrender", "tp": 1, "turn": 2, "args": [1]},
    ]


def _selections(out):
    return [r for r in out if r["type"] == "selection"]


def _homeros_pick(card, card_index, position):
    """Homeros's gate (yes), its [Olympos XII] pick of frame 0, then DCGO's effect pick naming `card`."""
    return [
        {"method": "OptionalSkill.SetUseOptional", "tp": YOU, "turn": 1, "args": [YOU, True]},
        {"method": "SelectPermanentEffect.SetTargetFrames", "tp": YOU, "turn": 1, "args": [YOU, [True], [0]],
         "boards": [[card, HOMEROS], []], "board_indexes": [[card_index, 55], []]},
        {"method": "SelectCardEffect.SetTargetCardAndIndicies", "tp": YOU, "turn": 1,
         "args": [YOU, [card_index], [position]], "card_ids": [card]},
    ]


def test_a_card_pick_keeps_its_position_in_dcgos_list():
    out, _ = convert_raw.convert(_raw_game(*_homeros_pick("BT24-041", 68, 1)), cards={})
    pick = next(r for r in _selections(out) if r["prompt"] == "SelectCardEffect")
    assert pick["card_ids"] == ["BT24-041"]
    assert pick["card_indexes"] == [68]
    assert pick["indexes"] == [1]


def test_a_pick_of_the_card_homeros_refired_is_no_longer_dropped_by_card_id():
    """The former BT24-101 rule consumed this row in the converter; the replay decides now, from the engine."""
    for card in ("BT24-101", "BT24-041", "BT26-081"):
        out, _ = convert_raw.convert(_raw_game(*_homeros_pick(card, 70, 0)), cards={})
        picks = [r for r in _selections(out) if r["prompt"] == "SelectCardEffect"]
        assert [p["card_ids"] for p in picks] == [[card]], card
        assert picks[0]["indexes"] == [0]


def test_a_malformed_position_list_is_not_written():
    for positions in ([], [-1], [True], ["1"]):
        raw = _homeros_pick("BT24-041", 68, 0)
        raw[-1]["args"] = [YOU, [68], positions]
        out, _ = convert_raw.convert(_raw_game(*raw), cards={})
        pick = next(r for r in _selections(out) if r["prompt"] == "SelectCardEffect")
        assert "indexes" not in pick, positions
        assert pick["card_indexes"] == [68]
