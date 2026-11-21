"""convert_raw.py: a hand card's [Main] (`ActivateCardAction`, main-phase packet 1) is encoded as a hand effect.

DCGO's `ActivateCardAction` (GamePacketRegistration order: 0 attack, 1 activate card, 2 activate permanent, 3 cheat,
4 play, 5 pass) carries `(card index, skill index)` as two big-endian ints. The engine's action is
`HAND_EFFECT_START + <hand slot>` (action/space.rs), so the converter reads the card's CURRENT hand slot from the hand
listing of the PlayCard row that follows (a "[Hand] [Main] ... digivolves into this card" effect plays the card), and
writes the card id beside it. Before this the converter stopped at the packet ("main-phase packet 1"), truncating every
game with such an activation (raw-20261116T164448Z#g2, raw seq 454).

Synthetic raw rows only: no recording (they carry private deck order) is read.
"""
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
_spec = importlib.util.spec_from_file_location(
    "dcgo_convert_raw_activate_hand_under_test", ROOT / "code" / "dcgo-melon-recorder" / "convert_raw.py")
convert_raw = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(convert_raw)

YOU = 0


def _be32(n):
    n &= 0xFFFFFFFF
    return [(n >> 24) & 0xFF, (n >> 16) & 0xFF, (n >> 8) & 0xFF, n & 0xFF]


def _snapshot():
    deck = ["ST1-03"] * 10
    return {"method": "Snapshot.SetRedraw", "t": "t", "you": YOU, "is_ai": False, "players": [
        {"id": 0, "hand": deck[:5], "library": deck[5:], "egg_library": []},
        {"id": 1, "hand": deck[:5], "library": deck[5:], "egg_library": []},
    ]}


def _activate_card(card_index, skill=0):
    return {"method": "TurnStateMachine.QueueMainPhaseAction_Internal", "tp": YOU, "turn": 1,
            "args": [YOU, convert_raw.ACTIVATE_CARD_PACKET_ID, _be32(card_index) + _be32(skill)]}


def _hand(slot, cards):
    return {"player": YOU, "slot": slot, "cards": [{"card_id": c, "card_index": k} for c, k in cards]}


def _hand_effect_play(card, slot, cards):
    """The activated card playing itself (an effect's digivolve: pay_cost true, no queued play packet)."""
    return {"method": "PlayCardClass.PlayCard", "tp": YOU, "turn": 1, "cards": [card], "pay_cost": True,
            "target": None, "hand": _hand(slot, cards)}


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


def _actions(out):
    return [r for r in out if r["type"] == "action"]


HAND = [("ST1-03", 3), ("ST1-04", 41), ("BT18-081", 29), ("ST1-05", 52)]


def test_activated_hand_card_is_a_hand_effect_at_its_current_slot():
    out, _ = convert_raw.convert(_raw_game(
        _activate_card(29),
        {"method": "TurnStateMachine.SetActCardSkill", "tp": YOU, "turn": 1, "args": [29, 0]},
        _hand_effect_play("BT18-081", 2, HAND),
    ), cards={})
    act = next(r for r in _actions(out) if r["action_id"] == convert_raw.HAND_EFFECT_START + 2)
    assert act["card_id"] == "BT18-081"
    assert act["card_index"] == 29
    assert act["source"] == "main_phase"
    # The effect's own digivolve is not a second main-phase decision.
    assert [r["action_id"] for r in _actions(out) if r["actor"] == YOU][-1] == convert_raw.ACTION_PASS
    assert sum(1 for r in _actions(out) if r["actor"] == YOU and r["action_id"] != convert_raw.ACTION_PASS) >= 1
    assert convert_raw.LAST_TRUNCATION is None, convert_raw.LAST_TRUNCATION


def test_slot_is_the_hand_listing_slot_not_the_deck_order_index():
    # The packet names card index 29; its slot in the listing is 0 (the card index is NOT the slot).
    out, _ = convert_raw.convert(_raw_game(
        _activate_card(29),
        _hand_effect_play("BT18-081", 0, [("BT18-081", 29), ("ST1-03", 3)]),
    ), cards={})
    assert any(r["action_id"] == convert_raw.HAND_EFFECT_START for r in _actions(out))


def test_an_activation_with_no_listing_truncates_instead_of_guessing_a_slot():
    out, _ = convert_raw.convert(_raw_game(_activate_card(29)), cards={}, allow_truncated=True)
    assert "activate packet for hand card index 29" in (convert_raw.LAST_TRUNCATION or ""), convert_raw.LAST_TRUNCATION
    assert not any(r["action_id"] >= convert_raw.HAND_EFFECT_START and r["action_id"] < convert_raw.HAND_EFFECT_END
                   for r in _actions(out))
