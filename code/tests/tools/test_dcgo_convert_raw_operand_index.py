"""convert_raw.py: a board operand among duplicate copies carries DCGO's own CardIndex.

An action row's board operand (the attacker of `SetAttackingPermaent`, the digivolve target, the host of an
activated permanent effect) is a position into the recorded board, and the board names permanents by card id only.
Two copies of one card therefore cannot be told apart from the position: the replay pairs duplicates in occurrence
order, which is a guess (raw-20261115T153822Z#g10 raw seq 821: two ST24-05, DCGO's attacker was the newer copy that
holds BT26-036, and the guess sent the older copy). A recorder 0.2.0 row logs `board_indexes` (the top card's
CardIndex per board entry, Core.cs BoardsJson), so the converter writes the operand's index on the action row as
`operand_card_index` (+ `operand_card_index_source: "recorder"`), only when the board holds more than one copy of the
operand's card.

Synthetic raw rows only: no recording (they carry private deck order) is read.
"""
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
_spec = importlib.util.spec_from_file_location(
    "dcgo_convert_raw_operand_index_under_test", ROOT / "code" / "dcgo-melon-recorder" / "convert_raw.py")
convert_raw = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(convert_raw)

YOU = 0


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


def _attack(position, boards, indexes=None):
    row = {"method": "TurnStateMachine.SetAttackingPermaent", "tp": YOU, "turn": 1, "args": [position, -1],
           "boards": boards}
    if indexes is not None:
        row["board_indexes"] = indexes
    return row


def _actions(out):
    """The attack / activated-effect actions (not the breeding / pass rows around them)."""
    return [r for r in out if r["type"] == "action"
            and convert_raw.ATTACK_START <= r["action_id"] < convert_raw.FIELD_EFFECT_START + 1000]


def test_attacker_among_duplicates_carries_the_recorded_card_index():
    boards = [["ST24-05", "ST24-05", "ST24-13"], ["EX11-027"]]
    out, _ = convert_raw.convert(_raw_game(_attack(0, boards, [[57, 85, 55], [36]])), cards={})
    attack = next(r for r in _actions(out) if r["action_id"] == convert_raw.ATTACK_START
                  + 0 * convert_raw.TARGETS_PER_ATTACKER + convert_raw.SECURITY_TARGET)
    assert attack["operand_card_index"] == 57
    assert attack["operand_card_index_source"] == "recorder"


def test_attacker_second_copy_names_its_own_index():
    boards = [["ST24-05", "ST24-05", "ST24-13"], ["EX11-027"]]
    out, _ = convert_raw.convert(_raw_game(_attack(1, boards, [[57, 85, 55], [36]])), cards={})
    attack = _actions(out)[0]
    assert attack["operand_card_index"] == 85


def test_unique_attacker_carries_no_operand_index():
    boards = [["ST24-05", "ST24-13"], ["EX11-027"]]
    out, _ = convert_raw.convert(_raw_game(_attack(0, boards, [[57, 55], [36]])), cards={})
    attack = _actions(out)[0]
    assert "operand_card_index" not in attack
    assert "operand_card_index_source" not in attack


def test_older_raw_row_without_board_indexes_omits_the_operand_index():
    boards = [["ST24-05", "ST24-05", "ST24-13"], ["EX11-027"]]
    out, _ = convert_raw.convert(_raw_game(_attack(0, boards)), cards={})
    attack = _actions(out)[0]
    assert "operand_card_index" not in attack


def test_board_indexes_that_do_not_line_up_with_the_board_are_not_trusted():
    boards = [["ST24-05", "ST24-05", "ST24-13"], ["EX11-027"]]
    out, _ = convert_raw.convert(_raw_game(_attack(0, boards, [[57, 85], [36]])), cards={})
    assert "operand_card_index" not in _actions(out)[0]


def test_activated_permanent_effect_host_among_duplicates_carries_the_index():
    boards = [["ST24-05", "ST24-05", "ST24-13"], []]
    packet = [0, 0, 0, 1, 0, 0, 0, 0]  # perm 1, skill 0
    row = {"method": "TurnStateMachine.QueueMainPhaseAction_Internal", "tp": YOU, "turn": 1,
           "args": [YOU, convert_raw.ACTIVATE_PERMANENT_PACKET_ID, packet + [0] * 4], "boards": boards,
           "board_indexes": [[57, 85, 55], []]}
    out, _ = convert_raw.convert(_raw_game(row), cards={})
    host = next(r for r in _actions(out) if convert_raw.FIELD_EFFECT_START <= r["action_id"] < convert_raw.FIELD_EFFECT_START + 1000)
    assert host["operand_card_index"] == 85
    assert host["operand_card_index_source"] == "recorder"


def _be32(n):
    n &= 0xFFFFFFFF
    return [(n >> 24) & 0xFF, (n >> 16) & 0xFF, (n >> 8) & 0xFF, n & 0xFF]


def _digivolve(target_slot, target, boards, indexes=None):
    """Hand card 65 (ST24-06) digivolved onto the board entry at `target_slot`."""
    packet = _be32(65) + _be32(3) + _be32(0) + _be32(-1) + _be32(0)
    queue = {"method": "TurnStateMachine.QueueMainPhaseAction_Internal", "tp": YOU, "turn": 1,
             "args": [YOU, convert_raw.PLAY_PACKET_ID, packet], "play": {"card_index": 65}}
    resolve = {"method": "PlayCardClass.PlayCard", "tp": YOU, "turn": 1, "cards": ["ST24-06"], "pay_cost": True,
               "target": target, "target_slot": target_slot, "target_breeding": False, "boards": boards,
               "hand": {"player": YOU, "slot": 0, "cards": [{"card_id": "ST24-06", "card_index": 65}]}}
    if indexes is not None:
        resolve["board_indexes"] = indexes
    return [queue, resolve]


def _digivolve_actions(out):
    return [r for r in out if r["type"] == "action"
            and convert_raw.DIGIVOLVE_START <= r["action_id"] < convert_raw.DIGIVOLVE_START + 15 * 15]


def test_digivolve_target_among_duplicates_carries_the_targets_card_index():
    boards = [["ST24-05", "ST24-05", "ST24-13"], []]
    out, _ = convert_raw.convert(_raw_game(*_digivolve(
        1, {"card_id": "ST24-05", "card_index": 85}, boards)), cards={})
    [dv] = _digivolve_actions(out)
    assert dv["action_id"] == convert_raw.DIGIVOLVE_START + 1
    assert dv["operand_card_index"] == 85
    assert dv["operand_card_index_source"] == "recorder"


def test_digivolve_target_without_duplicates_carries_no_operand_index():
    boards = [["ST24-05", "ST24-13"], []]
    out, _ = convert_raw.convert(_raw_game(*_digivolve(
        0, {"card_id": "ST24-05", "card_index": 85}, boards)), cards={})
    [dv] = _digivolve_actions(out)
    assert "operand_card_index" not in dv


def test_digivolve_target_naming_a_different_card_than_the_board_is_not_trusted():
    boards = [["ST24-05", "ST24-05", "ST24-13"], []]
    out, _ = convert_raw.convert(_raw_game(*_digivolve(
        2, {"card_id": "ST24-05", "card_index": 85}, boards)), cards={})
    [dv] = _digivolve_actions(out)
    assert "operand_card_index" not in dv
