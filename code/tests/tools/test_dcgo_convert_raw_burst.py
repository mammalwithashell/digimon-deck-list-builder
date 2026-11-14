"""convert_raw.py: a Burst Digivolve's returned Tamer is written AFTER the digivolve action.

DCGO's human UI asks the Tamer to return (`SelectBurstDigivolutionEffect.SelectTamer`, a local `SelectPermanentEffect`
over the seat's own Tamers, SelectBurstDigivolutionEffect.cs:107-190) BEFORE it queues the play, and queues the
`PlayCardAction` from that pick's end coroutine with the Tamer's FrameID as `BurstTamerFrameID`
(TurnStateMachine.cs:3016-3030). The packet bytes are: card index, TargetFrameID, int[] JogressEvoRootsFrameIDs
(count + ids), BurstTamerFrameID, int[] AppFusionFrameIDs (count + ids), big-endian. The engine asks the Tamer AFTER
the digivolve action, as one of its "which digivolution cost?" routes. So the converter writes the action ahead of
the pick, which answers the engine's route choice. A remote seat's pick ran on its own client only (SetIsLocal), so
its Tamer is known only as a raw FrameID: the converter stops there rather than replay the burst as an ordinary
digivolve. raw-20261113T025510Z#g6 raw seq 1026-1029 (BT25-104 onto ShineGreymon (ST24-07), returning a Marcus
Damon & Thomas H. Norstein (ST24-13)).

Synthetic raw rows only: no recording (they carry private deck order) is read.
"""
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
_spec = importlib.util.spec_from_file_location(
    "dcgo_convert_raw_burst_under_test", ROOT / "code" / "dcgo-melon-recorder" / "convert_raw.py")
convert_raw = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(convert_raw)

YOU = 0


# The burst's Tamer pick must name a Tamer (convert_raw.is_tamer): Marcus Damon & Thomas H. Norstein (ST24-13).
TAMERS = {"ST24-13": {"card_kind": 1}}


def _be32(n):
    n &= 0xFFFFFFFF
    return [(n >> 24) & 0xFF, (n >> 16) & 0xFF, (n >> 8) & 0xFF, n & 0xFF]


def _play_bytes(card_index, frame, burst_frame):
    """A full PlayCardAction payload: no jogress roots, `burst_frame` (-1 = none), no app fusion."""
    return _be32(card_index) + _be32(frame) + _be32(0) + _be32(burst_frame) + _be32(0)


def _snapshot():
    deck = ["ST1-03"] * 10
    return {"method": "Snapshot.SetRedraw", "t": "t", "you": YOU, "is_ai": False, "players": [
        {"id": 0, "hand": deck[:5], "library": deck[5:], "egg_library": []},
        {"id": 1, "hand": deck[:5], "library": deck[5:], "egg_library": []},
    ]}


BOARDS = [["ST24-07", "ST24-12", "ST24-13", "ST24-13"], ["BT20-056"]]


def _tamer_pick(seat, frame):
    return {"method": "SelectPermanentEffect.SetTargetFrames", "tp": seat, "turn": 1, "args": [seat, [True], [frame]],
            "boards": BOARDS if seat == 0 else BOARDS[::-1]}


def _queue_play(seat, card_index, frame, burst_frame):
    return {"method": "TurnStateMachine.QueueMainPhaseAction_Internal", "tp": seat, "turn": 1,
            "args": [seat, convert_raw.PLAY_PACKET_ID, _play_bytes(card_index, frame, burst_frame)],
            "play": {"card_index": card_index}}


def _resolve_digivolve(seat, slot, played_index, card, target_slot):
    cards = [{"card_index": 100 + i} for i in range(5)]
    cards[slot] = {"card_index": played_index}
    return {"method": "PlayCardClass.PlayCard", "tp": seat, "turn": 1, "cards": [card],
            "hand": {"player": seat, "slot": slot, "cards": cards}, "target": "perm", "target_slot": target_slot,
            "boards": BOARDS if seat == 0 else BOARDS[::-1]}


def _raw_game(seat, *main_phase):
    return [
        _snapshot(),
        {"method": "TurnStateMachine.SetRedraw", "args": [0, False]},
        {"method": "TurnStateMachine.SetRedraw", "args": [1, False]},
        {"method": "TurnStateMachine.SetBreedingPhase", "tp": seat, "turn": 1, "args": [seat, False]},
        *main_phase,
        {"method": "TurnStateMachine.QueueMainPhaseAction_Internal", "tp": seat, "turn": 1,
         "args": [seat, convert_raw.PASS_PACKET_ID, []]},
        {"method": "TurnStateMachine.Surrender", "tp": 1 - seat, "turn": 2, "args": [1 - seat]},
    ]


def _decisions(out):
    return [r for r in out if r["type"] in ("action", "selection")]


DIGIVOLVE = convert_raw.DIGIVOLVE_START + 2 * convert_raw.FIELDS_PER_HAND + 0  # hand slot 2 onto battle slot 0


def test_local_burst_tamer_pick_is_written_after_the_digivolve_action():
    out, _ = convert_raw.convert(_raw_game(
        YOU,
        _tamer_pick(YOU, 3),
        _queue_play(YOU, card_index=82, frame=4, burst_frame=11),
        _resolve_digivolve(YOU, slot=2, played_index=82, card="BT25-104", target_slot=0),
    ), cards=TAMERS)
    rows = _decisions(out)
    kinds = [(r["type"], r.get("action_id", r.get("prompt"))) for r in rows]
    at = kinds.index(("action", DIGIVOLVE))
    assert kinds[at + 1] == ("selection", "SelectPermanentEffect"), kinds
    assert rows[at + 1]["targets"] == [{"player": YOU, "frame": 3}]
    assert rows[at + 1]["board_p0"] == BOARDS[0]
    assert rows[at + 1]["mechanic"] == "burst"
    assert sum(1 for k in kinds if k == ("selection", "SelectPermanentEffect")) == 1, "moved, not copied"
    assert [r["step"] for r in rows] == list(range(len(rows)))


def test_ordinary_digivolve_keeps_an_earlier_own_pick_in_front():
    """Regression guard: BurstTamerFrameID -1 is an ordinary digivolve; an own-side pick before it stays put."""
    out, _ = convert_raw.convert(_raw_game(
        YOU,
        _tamer_pick(YOU, 3),
        _queue_play(YOU, card_index=82, frame=4, burst_frame=-1),
        _resolve_digivolve(YOU, slot=2, played_index=82, card="BT25-104", target_slot=0),
    ), cards=TAMERS)
    rows = _decisions(out)
    kinds = [(r["type"], r.get("action_id", r.get("prompt"))) for r in rows]
    assert kinds.index(("selection", "SelectPermanentEffect")) < kinds.index(("action", DIGIVOLVE)), kinds
    assert all("mechanic" not in r for r in rows)


def test_local_burst_with_no_tamer_pick_logged_stops():
    out, _ = convert_raw.convert(_raw_game(
        YOU,
        _queue_play(YOU, card_index=82, frame=4, burst_frame=11),
        _resolve_digivolve(YOU, slot=2, played_index=82, card="BT25-104", target_slot=0),
    ), cards=TAMERS, allow_truncated=True)
    assert not any(r.get("action_id") == DIGIVOLVE for r in _decisions(out))
    assert "Burst" in (convert_raw.LAST_TRUNCATION or ""), convert_raw.LAST_TRUNCATION


def test_remote_burst_stops_instead_of_replaying_an_ordinary_digivolve():
    seat = 1 - YOU
    out, _ = convert_raw.convert(_raw_game(
        seat,
        _queue_play(seat, card_index=82, frame=4, burst_frame=11),
        _resolve_digivolve(seat, slot=2, played_index=82, card="BT25-104", target_slot=0),
    ), cards=TAMERS, allow_truncated=True)
    assert not any(r.get("action_id") == DIGIVOLVE and r.get("actor") == seat for r in _decisions(out))
    assert "Burst" in (convert_raw.LAST_TRUNCATION or ""), convert_raw.LAST_TRUNCATION
