"""convert_raw.py: a REMOTE seat's Burst Digivolve Tamer, named by the frame model from the play packet's FrameID.

A burst digivolver's own Tamer pick (`SelectBurstDigivolutionEffect.SelectTamer`, `isLocal: true`,
SelectBurstDigivolutionEffect.cs:107-190) runs on the digivolver's own client only: a recorder on the OTHER client sees
no pick row. What reaches every client is the play packet, whose `BurstTamerFrameID` is the picked permanent's
`PermanentFrame.FrameID` (TurnStateMachine.cs:3016-3030; packet layout: card index, TargetFrameID, int[] jogress roots,
BurstTamerFrameID, int[] app fusion; ActionEncoder.cs). `frame_model` follows every permanent's frame from the plays the
raw logged and checks each recorded board against it, so a FrameID names a permanent exactly when every hypothesis
agrees. The converter then writes the pick the remote seat never logged (`synthesized: "burst_tamer_frame"`,
`mechanic: "burst"`) right after the digivolve action, and notes the provenance. When the model is not certain
(a board it cannot explain, a Tamer it never saw played, several copies it cannot tell apart) it halts, exactly as
before.

Field witness for the packet shape (local seat, so the pick exists to cross-check): raw-20261113T025510Z#g6 raw seq
1026-1029 (BT25-104 onto ShineGreymon (ST24-07), returning a Marcus Damon & Thomas H. Norstein (ST24-13)).

Synthetic raw rows only: no recording (they carry private deck order) is read.
"""
import importlib.util
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
REC = ROOT / "code" / "dcgo-melon-recorder"
sys.path.insert(0, str(REC))
_spec = importlib.util.spec_from_file_location("dcgo_convert_raw_burst_remote_under_test", REC / "convert_raw.py")
convert_raw = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(convert_raw)

YOU = 0
REMOTE = 1
TAMER = "ST24-13"
HOST = "ST24-07"
BURST = "BT25-104"
FILLER = "ST1-03"
CARDS = {TAMER: {"card_kind": 1}, HOST: {"card_kind": 0}, BURST: {"card_kind": 0}, FILLER: {"card_kind": 0}}


def _be32(n):
    n &= 0xFFFFFFFF
    return [(n >> 24) & 0xFF, (n >> 16) & 0xFF, (n >> 8) & 0xFF, n & 0xFF]


def _snapshot():
    deck = [FILLER] * 10
    return {"method": "Snapshot.SetRedraw", "t": "t", "you": YOU, "is_ai": False, "players": [
        {"id": 0, "hand": deck[:5], "library": deck[5:], "egg_library": []},
        {"id": 1, "hand": deck[:5], "library": deck[5:], "egg_library": []},
    ]}


def _hand(seat, slot, listing):
    return {"player": seat, "slot": slot, "cards": [{"card_id": c, "card_index": k} for c, k in listing]}


def _boards(remote_board):
    return [[], list(remote_board)]


def _play_packet(card_index, target_frame, burst_frame, *, boards):
    return {"method": "TurnStateMachine.QueueMainPhaseAction_Internal", "tp": REMOTE, "turn": 1,
            "args": [REMOTE, convert_raw.PLAY_PACKET_ID,
                     _be32(card_index) + _be32(target_frame) + _be32(0) + _be32(burst_frame) + _be32(0)],
            "play": {"card_index": card_index}, "boards": boards}


def _fresh_play(card, card_index, listing, *, boards, slot=0):
    """A main-phase play of `card` from the remote seat's hand slot `slot` onto an empty frame (PreferredFrame)."""
    return [
        _play_packet(card_index, 0, -1, boards=boards),
        {"method": "PlayCardClass.PlayCard", "tp": REMOTE, "turn": 1, "cards": [card], "pay_cost": True,
         "target": None, "hand": _hand(REMOTE, slot, listing), "boards": boards},
    ]


def _burst_digivolve(burst_frame, *, boards):
    """BT25-104 (card index 82, hand slot 0 of [BT25-104]) onto HOST, wherever HOST sits on the remote board."""
    host_slot = boards[REMOTE].index(HOST)
    return [
        _play_packet(82, 4, burst_frame, boards=boards),
        {"method": "PlayCardClass.PlayCard", "tp": REMOTE, "turn": 1, "cards": [BURST], "pay_cost": True,
         "hand": _hand(REMOTE, 0, [(BURST, 82)]), "target": "perm", "target_slot": host_slot, "boards": boards},
    ]


def _raw_game(*main_phase):
    return [
        _snapshot(),
        {"method": "TurnStateMachine.SetRedraw", "args": [0, False]},
        {"method": "TurnStateMachine.SetRedraw", "args": [1, False]},
        {"method": "TurnStateMachine.SetBreedingPhase", "tp": REMOTE, "turn": 1, "args": [REMOTE, False]},
        *main_phase,
        {"method": "TurnStateMachine.QueueMainPhaseAction_Internal", "tp": REMOTE, "turn": 1,
         "args": [REMOTE, convert_raw.PASS_PACKET_ID, []]},
        {"method": "TurnStateMachine.Surrender", "tp": YOU, "turn": 2, "args": [YOU]},
    ]


def _decisions(out):
    return [r for r in out if r["type"] in ("action", "selection")]


DIGIVOLVE = convert_raw.DIGIVOLVE_START + 0 * convert_raw.FIELDS_PER_HAND


def _setup():
    """The remote seat plays HOST (frame 4), a Tamer (index 65, frame 11) and another Tamer (index 89, frame 10):
    its battle area, in frame order, is [HOST, TAMER(89), TAMER(65)]."""
    return [
        *_fresh_play(HOST, 21, [(HOST, 21), (TAMER, 65), (TAMER, 89), (BURST, 82)], boards=_boards([])),
        *_fresh_play(TAMER, 65, [(TAMER, 65), (TAMER, 89), (BURST, 82)], boards=_boards([HOST])),
        *_fresh_play(TAMER, 89, [(TAMER, 89), (BURST, 82)], boards=_boards([HOST, TAMER])),
    ]


BOARD = [HOST, TAMER, TAMER]  # frames 4, 10, 11


def _halts(raw):
    out, _ = convert_raw.convert(raw, cards=CARDS, allow_truncated=True)
    assert not any(r.get("action_id") == DIGIVOLVE for r in _decisions(out)), "wrote the burst digivolve anyway"
    assert "Burst" in (convert_raw.LAST_TRUNCATION or ""), convert_raw.LAST_TRUNCATION


def test_remote_burst_tamer_is_named_from_the_packet_frame_and_noted():
    out, notes = convert_raw.convert(_raw_game(*_setup(), *_burst_digivolve(11, boards=_boards(BOARD))), cards=CARDS)
    rows = _decisions(out)
    kinds = [(r["type"], r.get("action_id", r.get("prompt"))) for r in rows]
    at = kinds.index(("action", DIGIVOLVE))
    assert kinds[at + 1] == ("selection", "SelectPermanentEffect"), kinds
    pick = rows[at + 1]
    # frame 11 is the third battle frame (HOST 4, TAMER 10, TAMER 11): position 2, the copy played with index 65
    assert pick["targets"] == [{"player": REMOTE, "frame": 2, "card_index": 65, "card_index_source": "frame_model"}]
    assert pick["mechanic"] == "burst"
    assert pick["actor"] == REMOTE
    assert pick["synthesized"] == "burst_tamer_frame"
    assert pick["board_p1"] == BOARD
    assert any("burst" in n.lower() and "BurstTamerFrameID 11" in n and "frame model" in n for n in notes), notes
    assert [r["step"] for r in rows] == list(range(len(rows)))


def test_remote_burst_names_the_other_copy_for_the_other_frame():
    out, _ = convert_raw.convert(_raw_game(*_setup(), *_burst_digivolve(10, boards=_boards(BOARD))), cards=CARDS)
    pick = [r for r in _decisions(out) if r["type"] == "selection"][-1]
    assert pick["targets"] == [{"player": REMOTE, "frame": 1, "card_index": 89, "card_index_source": "frame_model"}]


def test_remote_burst_with_a_frame_holding_no_tamer_halts():
    """FrameID 4 is HOST, a Digimon: not a Tamer pick, so nothing is written."""
    _halts(_raw_game(*_setup(), *_burst_digivolve(4, boards=_boards(BOARD))))


def test_remote_burst_with_an_empty_frame_halts():
    _halts(_raw_game(*_setup(), *_burst_digivolve(12, boards=_boards(BOARD))))


def test_remote_burst_when_the_model_never_saw_the_tamer_played_halts():
    """No plays logged for the remote seat: the board cannot be explained, the model is off, the FrameID names nothing."""
    _halts(_raw_game(*_burst_digivolve(11, boards=_boards(BOARD))))


def test_remote_burst_among_copies_with_no_card_index_halts():
    """A copy with no logged index (played by an effect, no hand listing) cannot be told from its twin by the engine:
    the pick would resolve by position onto the wrong permanent, so it halts rather than guess."""
    setup = [
        *_fresh_play(HOST, 21, [(HOST, 21), (TAMER, 65), (BURST, 82)], boards=_boards([])),
        *_fresh_play(TAMER, 65, [(TAMER, 65), (BURST, 82)], boards=_boards([HOST])),
        # an effect plays the second copy: no card index is logged for it
        {"method": "PlayCardClass.PlayCard", "tp": REMOTE, "turn": 1, "cards": [TAMER], "pay_cost": False,
         "target": None, "hand": {"player": -1, "slot": -1}},
    ]
    _halts(_raw_game(*setup, *_burst_digivolve(10, boards=_boards(BOARD))))


def test_remote_burst_of_a_lone_copy_needs_no_card_index():
    """One copy of the Tamer on the board: the card id alone names it, the frame model supplies the position."""
    one = [
        *_fresh_play(HOST, 21, [(HOST, 21), (TAMER, 65), (BURST, 82)], boards=_boards([])),
        {"method": "PlayCardClass.PlayCard", "tp": REMOTE, "turn": 1, "cards": [TAMER], "pay_cost": False,
         "target": None, "hand": {"player": -1, "slot": -1}},
    ]
    board = [HOST, TAMER]  # frames 4, 11
    out, _ = convert_raw.convert(_raw_game(*one, *_burst_digivolve(11, boards=_boards(board))), cards=CARDS)
    pick = [r for r in _decisions(out) if r["type"] == "selection"][-1]
    assert pick["targets"] == [{"player": REMOTE, "frame": 1}]
    assert pick["mechanic"] == "burst"


def test_local_burst_still_uses_its_own_logged_pick():
    """Regression guard: a LOCAL seat's pick is a real row; the model is not consulted and nothing is synthesized."""
    local_pick = {"method": "SelectPermanentEffect.SetTargetFrames", "tp": YOU, "turn": 1, "args": [YOU, [True], [1]],
                  "boards": [[HOST, TAMER], []]}
    packet = {"method": "TurnStateMachine.QueueMainPhaseAction_Internal", "tp": YOU, "turn": 1,
              "args": [YOU, convert_raw.PLAY_PACKET_ID,
                       _be32(82) + _be32(4) + _be32(0) + _be32(11) + _be32(0)], "play": {"card_index": 82}}
    resolve = {"method": "PlayCardClass.PlayCard", "tp": YOU, "turn": 1, "cards": [BURST], "pay_cost": True,
               "hand": _hand(YOU, 0, [(BURST, 82)]), "target": "perm", "target_slot": 0,
               "boards": [[HOST, TAMER], []]}
    raw = [
        _snapshot(),
        {"method": "TurnStateMachine.SetRedraw", "args": [0, False]},
        {"method": "TurnStateMachine.SetRedraw", "args": [1, False]},
        {"method": "TurnStateMachine.SetBreedingPhase", "tp": YOU, "turn": 1, "args": [YOU, False]},
        local_pick, packet, resolve,
        {"method": "TurnStateMachine.QueueMainPhaseAction_Internal", "tp": YOU, "turn": 1,
         "args": [YOU, convert_raw.PASS_PACKET_ID, []]},
        {"method": "TurnStateMachine.Surrender", "tp": REMOTE, "turn": 2, "args": [REMOTE]},
    ]
    out, _ = convert_raw.convert(raw, cards=CARDS)
    picks = [r for r in out if r.get("prompt") == "SelectPermanentEffect"]
    assert len(picks) == 1 and not picks[0].get("synthesized") and picks[0]["mechanic"] == "burst"
