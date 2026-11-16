"""convert_raw.py round-6 review fixes (converter lane).

1. An unstamped "use this optional effect?" answer of a seat that is stamped elsewhere in the raw.
   Core.cs stamps an agent's decline as 62 and leaves only its "Use" answer unstamped (Core.cs:2184-2187 even stamps -1
   on an agent's click of "decline" for an effect the engine resolved without a choice, which IS divergence evidence),
   and ReadStamp returns -1 after its 0.5 s timeout while advancing the stamp index (Core.cs:718-736). So an unstamped
   FALSE on a stamped seat did not come from the agent: it came from DCGO's AI, a StallWatch timeout or a human. Only
   an unstamped TRUE on a stamped seat is dropped (the agent's own "Use"; the engine asked nothing), and each dropped
   answer is recorded in the conversion notes with its seq so it is auditable.
2. A card_index the frame model derived is not a recorder 0.2.0 `board_indexes` value: the target says which it is
   (`card_index_source`: "recorder" / "frame_model" / "hand_listing").
3. A played card's index is taken from the hand listing only when the listed card at the slot IS the played card.

Synthetic raw rows only: no recording (they carry private deck order) is read.
"""
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
_spec = importlib.util.spec_from_file_location(
    "dcgo_convert_raw_review_r6_under_test", ROOT / "code" / "dcgo-melon-recorder" / "convert_raw.py")
convert_raw = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(convert_raw)

YOU, REMOTE = 1, 0
TAMER = "ST24-13"
BURST = "BT25-104"


def _game(*rows):
    deck = ["ST1-03"] * 10
    return [
        {"method": "Snapshot.SetRedraw", "t": "t", "you": YOU, "is_ai": False, "players": [
            {"id": 0, "hand": deck[:5], "library": deck[5:], "egg_library": []},
            {"id": 1, "hand": deck[:5], "library": deck[5:], "egg_library": []},
        ]},
        {"method": "TurnStateMachine.SetRedraw", "args": [0, False]},
        {"method": "TurnStateMachine.SetRedraw", "args": [1, False]},
        {"method": "TurnStateMachine.SetBreedingPhase", "tp": YOU, "turn": 1, "args": [YOU, False]},
        *rows,
        {"method": "TurnStateMachine.Surrender", "tp": YOU, "turn": 1, "args": [YOU]},
    ]


def _optional(seat, use, seq, **extra):
    row = {"method": "OptionalSkill.SetUseOptional", "tp": YOU, "turn": 1, "seq": seq, "args": [seat, use]}
    row.update(extra)
    return row


def _bools(out):
    return [r for r in out if r.get("type") == "selection" and r.get("prompt") == "generic_bool"]


def test_trigger_order_preserves_recorded_engine_action():
    out, _ = convert_raw.convert(_game({
        "method": "MultipleSkills.SetTargetSkill", "tp": YOU, "turn": 1,
        "args": [YOU, 0], "engine_action": 31, "chosen_effect_card": TAMER,
    }), cards={})
    row = next(r for r in out if r.get("prompt") == "MultipleSkills")
    assert row["engine_action"] == 31
    assert row["chosen_effect_card"] == TAMER


# ---- 1. unstamped answers on a stamped seat -------------------------------------------------------------------

def test_an_agent_seats_unstamped_false_is_kept_for_judging():
    # YOU is stamped (its first answer carries engine_action 62), so it is an agent seat; its later unstamped FALSE
    # did not come from the agent (agent declines are stamped 62) and is kept as an unstamped row.
    out, _ = convert_raw.convert(_game(
        _optional(YOU, False, 10, engine_action=62),
        _optional(YOU, False, 11),
    ), cards={})
    rows = [(r["actor"], r["bool_value"], r.get("engine_action"), r.get("unstamped")) for r in _bools(out)]
    assert rows == [(YOU, False, 62, None), (YOU, False, None, True)]


def test_a_stamped_seats_unstamped_true_is_dropped_with_a_note_naming_its_seq():
    out, notes = convert_raw.convert(_game(
        _optional(YOU, False, 10, engine_action=62),
        _optional(YOU, True, 4711),
    ), cards={})
    assert [(r["bool_value"], r.get("engine_action")) for r in _bools(out)] == [(False, 62)]
    dropped = [n for n in notes if "unstamped" in n and "dropped" in n]
    assert len(dropped) == 1 and "seq 4711" in dropped[0] and "P1" in dropped[0], notes


def test_a_dropped_true_and_a_kept_false_in_one_game_are_each_accounted_for():
    out, notes = convert_raw.convert(_game(
        _optional(YOU, False, 10, engine_action=62),
        _optional(YOU, True, 20),
        _optional(YOU, False, 21),
        _optional(YOU, True, 22),
    ), cards={})
    assert [(r["bool_value"], r.get("unstamped")) for r in _bools(out)] == [(False, None), (False, True)]
    dropped = [n for n in notes if "unstamped" in n and "dropped" in n]
    assert len(dropped) == 2
    assert "seq 20" in dropped[0] and "seq 22" in dropped[1]
    assert not any("seq 21" in n for n in dropped)


def test_a_human_seat_with_no_stamps_keeps_both_answers():
    out, notes = convert_raw.convert(_game(_optional(YOU, True, 30), _optional(YOU, False, 31)), cards={})
    assert [(r["bool_value"], r.get("unstamped")) for r in _bools(out)] == [(True, True), (False, True)]
    assert not [n for n in notes if "dropped" in n]


# ---- 2. provenance of a derived card index --------------------------------------------------------------------

BOARD = [["BT20-056"], ["ST24-07", TAMER, TAMER]]  # seat 1 (the turn player) holds two copies


def _pick(frame, **extra):
    row = {"method": "SelectPermanentEffect.SetTargetFrames", "tp": YOU, "turn": 1, "args": [YOU, [True], [frame]],
           "boards": BOARD}
    row.update(extra)
    return row


def _target(out):
    pick = next(r for r in out if r.get("prompt") == "SelectPermanentEffect")
    return pick["targets"][0]


def test_a_recorder_board_index_is_marked_recorder(monkeypatch):
    monkeypatch.setattr(convert_raw.frame_model, "pick_identities", lambda rows, cards, you: {})
    out, _ = convert_raw.convert(_game(_pick(1, board_indexes=[[9], [5, 6, 7]])), cards={})
    assert _target(out)["card_index"] == 6
    assert _target(out)["card_index_source"] == "recorder"


def test_a_frame_model_index_is_marked_frame_model(monkeypatch):
    # the frame model names frame 2's occupant; the raw row has no board_indexes (a pre-0.2.0 recorder)
    def fake(rows, cards, you):
        return {i: {(YOU, 2): 77} for i, r in enumerate(rows) if r.get("method") == "SelectPermanentEffect.SetTargetFrames"}
    monkeypatch.setattr(convert_raw.frame_model, "pick_identities", fake)
    out, _ = convert_raw.convert(_game(_pick(2)), cards={})
    assert _target(out)["card_index"] == 77
    assert _target(out)["card_index_source"] == "frame_model"


def test_a_target_with_no_index_carries_no_source(monkeypatch):
    monkeypatch.setattr(convert_raw.frame_model, "pick_identities", lambda rows, cards, you: {})
    out, _ = convert_raw.convert(_game(_pick(2)), cards={})
    assert "card_index" not in _target(out) and "card_index_source" not in _target(out)


# ---- 3. the played card's index needs the hand slot to hold the played card -------------------------------------

def _be32(n):
    n &= 0xFFFFFFFF
    return [(n >> 24) & 0xFF, (n >> 16) & 0xFF, (n >> 8) & 0xFF, n & 0xFF]


def _queue_play(card_index, frame):
    return {"method": "TurnStateMachine.QueueMainPhaseAction_Internal", "tp": YOU, "turn": 1,
            "args": [YOU, convert_raw.PLAY_PACKET_ID, _be32(card_index) + _be32(frame) + _be32(0) + _be32(-1) + _be32(0)],
            "play": {"card_index": card_index}}


def _play(card, slot, cards):
    return {"method": "PlayCardClass.PlayCard", "tp": YOU, "turn": 1, "cards": [card], "pay_cost": True,
            "target": None, "seq": 99,
            "hand": {"player": YOU, "slot": slot, "cards": [{"card_id": c, "card_index": k} for c, k in cards]}}


def _plays(out):
    return [r for r in out if r.get("type") == "action" and r.get("card_id") == TAMER]


def test_the_hand_slot_holding_the_played_card_gives_its_index():
    out, _ = convert_raw.convert(_game(_queue_play(65, 10), _play(TAMER, 1, [("ST24-07", 58), (TAMER, 65)])), cards={})
    assert [p.get("card_index") for p in _plays(out)] == [65]


def test_a_hand_slot_holding_another_card_gives_no_index_and_a_note():
    # slot 0 lists ST24-07 (index 58) but the row says TAMER was played: that index is not the played card's
    out, notes = convert_raw.convert(_game(_queue_play(58, 10), _play(TAMER, 0, [("ST24-07", 58), (TAMER, 65)])),
                                     cards={})
    assert all("card_index" not in p for p in _plays(out)), _plays(out)
    assert any("seq 99" in n and "hand slot" in n for n in notes), notes
