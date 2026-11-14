"""convert_raw.py: DCGO's own card identity (`CardSource.CardIndex`) reaches the converted rows.

The recorded boards name a permanent by its card id only, so two copies of one card on a board cannot be told apart
from a pick's position (`duplicate_copy_target` in the replay stops there). The raw rows already carry DCGO's card
index wherever DCGO resolved a card:

* a play packet / play resolve names the played card's index (`PlayPacketJson` / `OwnerHandJson` in Core.cs: the
  hand listing with each card's `CardIndex`, and the played card's slot in it);
* `SelectHandEffect.SetTargetHandCards(pid, int[] cardIndexes)` and
  `SelectCardEffect.SetTargetCardAndIndicies(pid, int[] cardIndexes, int[] ...)` name the picked cards by index;
* a Burst Digivolve returns the picked Tamer to its owner's hand (CardController.cs:254-256; CR 4.3 8-3), so the
  returned card's index is the one of the picked card's id that is new in the owner's next hand listing, and that
  was played onto the field from the hand earlier (its last listing is the play that took it out of the hand).

The converter writes them as `card_index` (action rows: the played card), `card_indexes` (hand / card picks,
parallel to `card_ids`) and `targets[].card_index` (a burst Tamer pick, when exactly one card qualifies). The replay
pairs DCGO indexes with the engine cards its own applied rows moved, and resolves a target by that identity instead
of by board position. Field witness: raw-20261113T025510Z#g6 raw seq 991-1034 (Marcus Damon & Thomas H. Norstein
(ST24-13) copies; the burst of ShineGreymon: Burst Mode (BT25-104) returns one of three).

Synthetic raw rows only: no recording (they carry private deck order) is read.
"""
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
_spec = importlib.util.spec_from_file_location(
    "dcgo_convert_raw_card_index_under_test", ROOT / "code" / "dcgo-melon-recorder" / "convert_raw.py")
convert_raw = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(convert_raw)

YOU = 0
TAMER = "ST24-13"
BURST = "BT25-104"


def _be32(n):
    n &= 0xFFFFFFFF
    return [(n >> 24) & 0xFF, (n >> 16) & 0xFF, (n >> 8) & 0xFF, n & 0xFF]


def _play_bytes(card_index, frame, burst_frame):
    return _be32(card_index) + _be32(frame) + _be32(0) + _be32(burst_frame) + _be32(0)


def _snapshot():
    deck = ["ST1-03"] * 10
    return {"method": "Snapshot.SetRedraw", "t": "t", "you": YOU, "is_ai": False, "players": [
        {"id": 0, "hand": deck[:5], "library": deck[5:], "egg_library": []},
        {"id": 1, "hand": deck[:5], "library": deck[5:], "egg_library": []},
    ]}


def _hand(seat, slot, cards):
    return {"player": seat, "slot": slot, "cards": [{"card_id": c, "card_index": k} for c, k in cards]}


def _queue_play(card_index, frame, burst_frame=-1):
    return {"method": "TurnStateMachine.QueueMainPhaseAction_Internal", "tp": YOU, "turn": 1,
            "args": [YOU, convert_raw.PLAY_PACKET_ID, _play_bytes(card_index, frame, burst_frame)],
            "play": {"card_index": card_index}}


def _main_play(card, slot, cards):
    """A main-phase play of the hand card at `slot` (pay_cost true, no target)."""
    return {"method": "PlayCardClass.PlayCard", "tp": YOU, "turn": 1, "cards": [card], "pay_cost": True,
            "target": None, "hand": _hand(YOU, slot, cards)}


def _effect_play(card, slot, cards):
    """An effect playing the hand card at `slot` (pay_cost false): no main-phase decision."""
    return {"method": "PlayCardClass.PlayCard", "tp": YOU, "turn": 1, "cards": [card], "pay_cost": False,
            "target": None, "hand": _hand(YOU, slot, cards)}


BOARD = [["ST24-07", TAMER, TAMER], ["BT20-056"]]


def _tamer_pick(frame):
    return {"method": "SelectPermanentEffect.SetTargetFrames", "tp": YOU, "turn": 1, "args": [YOU, [True], [frame]],
            "boards": BOARD}


def _burst_resolve(slot, cards):
    return {"method": "PlayCardClass.PlayCard", "tp": YOU, "turn": 1, "cards": [BURST], "pay_cost": True,
            "target": "perm", "target_slot": 0, "hand": _hand(YOU, slot, cards), "boards": BOARD}


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


def _decisions(out):
    return [r for r in out if r["type"] in ("action", "selection")]


def test_main_phase_play_carries_the_played_card_index():
    out, _ = convert_raw.convert(_raw_game(
        _queue_play(65, frame=10),
        _main_play(TAMER, 1, [("ST24-07", 58), (TAMER, 65)]),
    ), cards={})
    play = next(r for r in _decisions(out) if r["type"] == "action" and r.get("card_id") == TAMER)
    assert play["action_id"] == convert_raw.PLAY_HAND_START + 1
    assert play["card_index"] == 65


def test_hand_and_card_picks_carry_dcgo_card_indexes():
    out, _ = convert_raw.convert(_raw_game(
        {"method": "SelectCardEffect.SetTargetCardAndIndicies", "tp": YOU, "turn": 1, "args": [YOU, [87], [2]],
         "card_ids": [TAMER]},
        {"method": "SelectHandEffect.SetTargetHandCards", "tp": YOU, "turn": 1, "args": [YOU, [87, 89]],
         "card_ids": [TAMER, TAMER]},
    ), cards={})
    picks = [r for r in _decisions(out) if r["type"] == "selection"]
    assert picks[0]["prompt"] == "SelectCardEffect" and picks[0]["card_indexes"] == [87]
    assert picks[1]["prompt"] == "SelectHandEffect" and picks[1]["card_indexes"] == [87, 89]


# The burst's Tamer pick must name a Tamer (convert_raw.is_tamer): the picked TAMER is card_kind 1.
TAMERS = {TAMER: {"card_kind": 1}}


def _burst_game(after_burst_listing):
    """Tamers 87 (effect play) and 89 (main play) go to the field; BT25-104 bursts, picking DCGO frame 2."""
    return _raw_game(
        _effect_play(TAMER, 0, [(TAMER, 87), (TAMER, 89), (BURST, 82)]),
        _queue_play(89, frame=12),
        _main_play(TAMER, 0, [(TAMER, 89), (BURST, 82)]),
        _tamer_pick(2),
        _queue_play(82, frame=4, burst_frame=11),
        _burst_resolve(0, [(BURST, 82), ("BT26-091", 94)]),
        *after_burst_listing,
    )


def _burst_target(out):
    pick = next(r for r in _decisions(out) if r.get("mechanic") == "burst")
    assert pick["targets"][0]["frame"] == 2
    return pick["targets"][0]


def test_burst_pick_names_the_returned_tamer_from_the_next_hand_listing():
    out, _ = convert_raw.convert(_burst_game([
        # the digivolve draw (BT26-091 94) and the returned Tamer (87) are both new in the hand; only 87 is a Tamer
        # of the picked id that had been played onto the field
        _effect_play("BT26-091", 1, [(BURST, 82), ("BT26-091", 94), (TAMER, 87)]),
    ]), cards=TAMERS)
    assert _burst_target(out)["card_index"] == 87


def test_burst_pick_without_a_later_listing_stays_positional():
    out, _ = convert_raw.convert(_burst_game([]), cards=TAMERS)
    assert "card_index" not in _burst_target(out)


def test_burst_pick_with_two_qualifying_cards_stays_positional():
    """Both Tamers come back before the next listing (another bounce): which one the burst returned is unknown."""
    out, _ = convert_raw.convert(_burst_game([
        _effect_play("BT26-091", 1, [(BURST, 82), ("BT26-091", 94), (TAMER, 87), (TAMER, 89)]),
    ]), cards=TAMERS)
    assert "card_index" not in _burst_target(out)


def test_a_drawn_copy_is_not_taken_for_the_returned_tamer():
    """A copy of the picked id new in the hand that was never played from it (a draw) does not qualify."""
    out, _ = convert_raw.convert(_burst_game([
        _effect_play("BT26-091", 1, [(BURST, 82), ("BT26-091", 94), (TAMER, 101)]),
    ]), cards=TAMERS)
    assert "card_index" not in _burst_target(out)
