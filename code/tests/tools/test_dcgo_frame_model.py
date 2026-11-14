"""frame_model.py + convert_raw.py: which copy a board-position pick names, from DCGO's own frame placement.

A recorded board (`boards`, Core.cs BoardsJson) lists a seat's permanents by card id in `GetFieldPermanents()` order,
which walks `FieldPermanents` by frame index (Player.cs:669-684). A permanent keeps its frame while it stays on the
field: a fresh play goes to `CardSource.PreferredFrame()` (PlayCardClass -> PlayPermanentClass, CardController.cs:1221,
:1533-1541; CardSource.cs:2340-2410), which is the first empty battle-area frame in an order fixed by the scene layout
(BattleScene.unity YourPermanentFrame / OpponentPermanentFrame, GridLayoutGroup disabled) and the card kind; a
digivolve keeps its root's frame; only the hybrid re-row (CardController.cs:1811-1856) and the breeding move
(CardObjectController.cs:1149-1197) change a permanent's frame, both again to PreferredFrame.

So the converter can follow each permanent's frame from the plays it logged, check every recorded board against that
model, and name the copy a pick's position holds when every hypothesis agrees and the occupant's DCGO card index is
known. Anything the model does not explain (a board it cannot reproduce, too many hypotheses, a DNA packet) turns the
model off for that seat for the rest of the game: no guess is written. A recorder 0.2.0 row's `board_indexes` names
the copy directly. Field witness: raw-20261113T025510Z#g6 raw seq 1115 (Kudamon (EX13-026)'s [On Play] pick of one of
three Marcus Damon & Thomas H. Norstein (ST24-13)).

Synthetic raw rows only: no recording (they carry private deck order) is read.
"""
import importlib.util
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
REC = ROOT / "code" / "dcgo-melon-recorder"
sys.path.insert(0, str(REC))
_spec = importlib.util.spec_from_file_location("dcgo_convert_raw_frame_model_under_test", REC / "convert_raw.py")
convert_raw = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(convert_raw)
import frame_model  # noqa: E402

YOU = 0
TAMER = "ST24-13"
OPTION = "ST24-15"
DIGI = "ST24-12"
CARDS = {TAMER: {"card_kind": 1}, OPTION: {"card_kind": 2}, DIGI: {"card_kind": 0}}


def _be32(n):
    n &= 0xFFFFFFFF
    return [(n >> 24) & 0xFF, (n >> 16) & 0xFF, (n >> 8) & 0xFF, n & 0xFF]


def _snapshot():
    deck = ["ST1-03"] * 10
    return {"method": "Snapshot.SetRedraw", "t": "t", "you": YOU, "is_ai": False, "players": [
        {"id": 0, "hand": deck[:5], "library": deck[5:], "egg_library": []},
        {"id": 1, "hand": deck[:5], "library": deck[5:], "egg_library": []},
    ]}


def _hand(slot, cards):
    return {"player": YOU, "slot": slot, "cards": [{"card_id": c, "card_index": k} for c, k in cards]}


def _main_play(card, card_index, slot, cards):
    """A main-phase play packet (card index, a TargetFrameID DCGO does not use for a fresh play) and its resolve."""
    packet = {"method": "TurnStateMachine.QueueMainPhaseAction_Internal", "tp": YOU, "turn": 1,
              "args": [YOU, convert_raw.PLAY_PACKET_ID,
                       _be32(card_index) + _be32(0) + _be32(0) + _be32(-1) + _be32(0)],
              "play": {"card_index": card_index, "card_id": card, "frame": 0}}
    resolve = {"method": "PlayCardClass.PlayCard", "tp": YOU, "turn": 1, "cards": [card], "pay_cost": True,
               "target": None, "hand": _hand(slot, cards)}
    return [packet, resolve]


def _effect_play(card, hand=None):
    """An effect's play (pay_cost false); `hand` = (slot, listing) for a play from the hand, else none logged."""
    row = {"method": "PlayCardClass.PlayCard", "tp": YOU, "turn": 1, "cards": [card], "pay_cost": False,
           "target": None}
    row["hand"] = _hand(*hand) if hand else {"player": -1, "slot": -1}
    return row


def _pick(frame, board, **extra):
    row = {"method": "SelectPermanentEffect.SetTargetFrames", "tp": YOU, "turn": 1, "args": [YOU, [True], [frame]],
           "boards": [board, []]}
    row.update(extra)
    return row


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


def _picks(out):
    return [r for r in out if r.get("prompt") == "SelectPermanentEffect"]


def _three_tamers(*tail):
    """Option 54 takes frame 11, Tamer 65 frame 10 (11 is taken), Tamer 89 frame 12: board [T65, O, T89]."""
    return _raw_game(
        *_main_play(OPTION, 54, 0, [(OPTION, 54), (TAMER, 65), (TAMER, 89)]),
        *_main_play(TAMER, 65, 0, [(TAMER, 65), (TAMER, 89)]),
        _pick(1, [TAMER, OPTION]),
        _effect_play(TAMER, (0, [(TAMER, 89)])),
        *tail,
    )


def test_preferred_frame_orders_follow_the_scene_layout():
    # local seat: Tamer/Option back row centre-out (x ascending on a tie), Digimon front row centre-out
    assert frame_model.preferred_order(is_you=True, tamer_side=True) == [11, 10, 12, 9, 13, 14, 15, 4, 3, 5, 2, 6, 1,
                                                                          7, 0, 8]
    assert frame_model.preferred_order(is_you=True, tamer_side=False) == [4, 3, 5, 2, 6, 1, 7, 0, 8, 11, 10, 12, 9,
                                                                           13, 14, 15]
    # the other seat: Digimon in CardSource.cs's fixed correctOrder, Tamers back row (x descending on a tie)
    assert frame_model.preferred_order(is_you=False, tamer_side=False) == [4, 3, 5, 2, 6, 1, 7, 0, 8, 11, 10, 12, 9,
                                                                            13, 14, 15]
    assert frame_model.preferred_order(is_you=False, tamer_side=True) == [11, 10, 12, 9, 13, 14, 15, 4, 3, 5, 2, 6, 1,
                                                                           7, 0, 8]


def test_pick_names_each_copy_by_its_frame():
    out, _ = convert_raw.convert(_three_tamers(
        _pick(0, [TAMER, OPTION, TAMER]),
        _pick(2, [TAMER, OPTION, TAMER]),
    ), cards=CARDS)
    first, second = _picks(out)[1:]
    assert first["targets"] == [{"player": YOU, "frame": 0, "card_index": 65}]
    assert second["targets"] == [{"player": YOU, "frame": 2, "card_index": 89}]


def test_a_copy_with_no_logged_index_stays_positional():
    """A play from security logs no card index: the frame model still places it (frame 10, ahead of 65's frame 11),
    but cannot name it."""
    out, _ = convert_raw.convert(_raw_game(
        *_main_play(TAMER, 65, 0, [(TAMER, 65)]),
        _effect_play(TAMER),
        _pick(0, [TAMER, TAMER]),
        _pick(1, [TAMER, TAMER]),
    ), cards=CARDS)
    unnamed, named = _picks(out)
    assert unnamed["targets"] == [{"player": YOU, "frame": 0}]
    assert named["targets"] == [{"player": YOU, "frame": 1, "card_index": 65}]


def test_a_board_the_model_cannot_explain_turns_it_off():
    """Frame 12's Tamer listed ahead of frame 10's: the model is wrong somewhere, so nothing after is named."""
    out, _ = convert_raw.convert(_three_tamers(
        _pick(0, [TAMER, TAMER, OPTION]),
        _pick(0, [TAMER, OPTION, TAMER]),
    ), cards=CARDS)
    for row in _picks(out)[1:]:
        assert "card_index" not in row["targets"][0]


def test_an_unknown_departure_among_copies_leaves_the_pick_positional():
    """Three copies (frames 10, 11, 12), then one of them leaves: the two left could be any two of the three."""
    out, _ = convert_raw.convert(_raw_game(
        *_main_play(TAMER, 65, 0, [(TAMER, 65), (TAMER, 89)]),   # frame 11
        _effect_play(TAMER, (0, [(TAMER, 89)])),                 # frame 10
        _effect_play(TAMER),                                     # frame 12, no index
        _pick(0, [TAMER, TAMER, TAMER]),
        _pick(0, [TAMER, TAMER]),
    ), cards=CARDS)
    before, after = _picks(out)
    assert before["targets"] == [{"player": YOU, "frame": 0, "card_index": 89}]
    assert after["targets"] == [{"player": YOU, "frame": 0}]


def test_a_departure_the_boards_pin_down_keeps_the_names():
    """The Option leaves: both Tamers keep their frames, so both stay named."""
    out, _ = convert_raw.convert(_three_tamers(
        _pick(1, [TAMER, TAMER]),
    ), cards=CARDS)
    assert _picks(out)[1]["targets"] == [{"player": YOU, "frame": 1, "card_index": 89}]


def test_board_indexes_name_the_copy_directly():
    """A recorder 0.2.0 row logs each board entry's CardIndex (`board_indexes`): that is the copy, model or not."""
    out, _ = convert_raw.convert(_raw_game(
        _pick(1, [TAMER, TAMER], board_indexes=[[7, 9], []]),
    ), cards=CARDS)
    assert _picks(out)[0]["targets"] == [{"player": YOU, "frame": 1, "card_index": 9}]


def test_a_dna_packet_turns_the_model_off():
    """A DNA digivolve creates its permanent at a root's frame (TargetFrameID), not PreferredFrame: not modelled."""
    dna = {"method": "TurnStateMachine.QueueMainPhaseAction_Internal", "tp": YOU, "turn": 1,
           "args": [YOU, convert_raw.PLAY_PACKET_ID, _be32(5) + _be32(3) + _be32(2) + _be32(3) + _be32(4)],
           "play": {"card_index": 5, "card_id": DIGI, "frame": 3}}
    out, _ = convert_raw.convert(_three_tamers(
        dna,
        _pick(0, [TAMER, OPTION, TAMER]),
    ), cards=CARDS)
    assert "card_index" not in _picks(out)[1]["targets"][0]
