"""convert_raw.py: SetBreedingPhase(actor, doBreeding=true) is HATCH (60) or MOVE_FROM_BREEDING (61) by occupancy.

DCGO decides from its own breeding area (TurnStateMachine.SetBreedingPhase); the converter must know whether that
area holds a Digimon at the decision. A toggled flag (hatch -> occupied, move -> empty) misses everything an EFFECT
does to the breeding area - g6 (raw-20261113T025510Z): BT20-095's <Delay> moved Ryudamon (BT20-010) out of P0's
breeding area during P1's turn 7, so P0's next SetBreedingPhase(0, true) was a hatch, not a move.

Evidence the converter reads, strongest first:
  - Snapshot rows (`players[].breeding`): exact.
  - Field boards: `SelectPermanentEffect.SetTargetFrames` and the attack / activate packet rows carry
    GetFieldPermanents() - every permanent INCLUDING the breeding one (Player.cs:669-685, the breeding frame is the
    last FieldPermanents index, Player.cs:41-54). A known breeding top card absent from its owner's field board has
    left the breeding area.
  - Plays: a PlayCardClass row whose target is in the breeding area (`target_breeding`) or that plays into it
    (`breeding_area`) leaves it occupied by that card.
The flag is the fallback only when no evidence speaks; when evidence overrides it the converter notes so.

Synthetic raw rows only: no recording (they carry private deck order) is read.
"""
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
_spec = importlib.util.spec_from_file_location(
    "dcgo_convert_raw_breeding_under_test", ROOT / "code" / "dcgo-melon-recorder" / "convert_raw.py")
convert_raw = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(convert_raw)

P0, P1 = 0, 1


def _snapshot():
    deck = ["ST1-03"] * 10
    return {"method": "Snapshot.SetRedraw", "t": "t", "you": P1, "is_ai": False, "players": [
        {"id": 0, "hand": deck[:5], "library": deck[5:], "egg_library": []},
        {"id": 1, "hand": deck[:5], "library": deck[5:], "egg_library": []},
    ]}


def _breed(actor, turn, do=True):
    return {"method": "TurnStateMachine.SetBreedingPhase", "tp": actor, "turn": turn, "args": [actor, do]}


def _pass(actor, turn):
    return {"method": "TurnStateMachine.QueueMainPhaseAction_Internal", "tp": actor, "turn": turn,
            "args": [actor, convert_raw.PASS_PACKET_ID, []]}


def _effect_play_into_breeding_digimon(actor, turn, card):
    """An effect digivolving the breeding-area Digimon (pay_cost false: not a main-phase decision)."""
    return {"method": "PlayCardClass.PlayCard", "tp": actor, "turn": turn, "cards": [card], "pay_cost": False,
            "breeding_area": False, "target": {"card_id": "EGG", "card_index": 1}, "target_slot": -1,
            "target_breeding": True, "hand": {"player": actor, "slot": 0, "cards": [{"card_index": 9}]}}


def _effect_play_to_breeding_area(actor, turn, card):
    """An effect playing a card INTO the breeding area (PlayCardClass._isBreedingArea)."""
    return {"method": "PlayCardClass.PlayCard", "tp": actor, "turn": turn, "cards": [card], "pay_cost": False,
            "breeding_area": True, "target": None}


def _field_pick(actor, turn, board_p0, board_p1, frame=0):
    """A SelectPermanentEffect row: its boards are GetFieldPermanents() (battle area + breeding area)."""
    return {"method": "SelectPermanentEffect.SetTargetFrames", "tp": turn % 2, "turn": turn,
            "args": [actor, [actor == turn % 2], [frame]], "boards": [list(board_p0), list(board_p1)]}


def _game(*rows):
    return [
        _snapshot(),
        {"method": "TurnStateMachine.SetRedraw", "args": [0, False]},
        {"method": "TurnStateMachine.SetRedraw", "args": [1, False]},
        *rows,
        {"method": "TurnStateMachine.Surrender", "tp": P1, "turn": 9, "args": [P1]},
    ]


def _breeding_actions(out, actor):
    return [r["action_id"] for r in out if r["type"] == "action" and r.get("phase") == "Breeding"
            and r["actor"] == actor]


def test_breeding_digimon_moved_out_by_an_effect_makes_the_next_breeding_a_hatch():
    # g6 shape: P0 hatches (turn 2), its breeding Digimon digivolves into Ryudamon (BT20-010), P0 does not move it,
    # then during P1's turn an effect moves Ryudamon to the battle area (P0's field board no longer shows it in the
    # breeding area: a later pick's GetFieldPermanents lists the battle-area Digimon only, BT20-010 gone after a
    # digivolve onto it). P0's next doBreeding is a HATCH.
    out, inferred = convert_raw.convert(_game(
        _breed(P0, 2),                                   # hatch
        _effect_play_into_breeding_digimon(P0, 2, "BT20-010"),
        _pass(P0, 2),
        _breed(P1, 3, do=False),
        _field_pick(P0, 3, ["BT20-095", "BT20-087", "BT20-010"], ["ST24-04"]),  # still there (any area)
        _field_pick(P0, 3, ["EX13-055", "BT20-087"], ["ST24-04"]),  # BT20-010 is gone from P0's whole field
        _pass(P1, 3),
        _breed(P0, 4),                                   # must be HATCH
        _pass(P0, 4),
    ), cards={})
    assert _breeding_actions(out, P0) == [convert_raw.ACTION_HATCH, convert_raw.ACTION_HATCH]
    assert any("BT20-010" in note and "breeding" in note for note in inferred), inferred


def test_effect_play_into_the_breeding_area_makes_the_next_breeding_a_move():
    # g6 seq 1068: an effect played Ryudamon (BT20-010) INTO P0's empty breeding area (breeding_area true); the
    # end-of-game snapshot shows it there. A doBreeding after that is a MOVE.
    out, _ = convert_raw.convert(_game(
        _breed(P0, 2, do=False),
        _effect_play_to_breeding_area(P0, 2, "BT20-010"),
        _pass(P0, 2),
        _breed(P1, 3, do=False),
        _pass(P1, 3),
        _breed(P0, 4),                                   # must be MOVE
        _pass(P0, 4),
    ), cards={})
    assert _breeding_actions(out, P0) == [convert_raw.ACTION_PASS, convert_raw.ACTION_MOVE]


def test_breeding_digimon_still_on_the_field_board_keeps_the_area_occupied():
    # Regression guard: field boards that still list the breeding top card do not empty the area.
    out, inferred = convert_raw.convert(_game(
        _breed(P0, 2),
        _effect_play_into_breeding_digimon(P0, 2, "BT20-010"),
        _pass(P0, 2),
        _breed(P1, 3, do=False),
        _field_pick(P0, 3, ["BT20-087", "BT20-010"], ["ST24-04"]),
        _pass(P1, 3),
        _breed(P0, 4),
        _pass(P0, 4),
    ), cards={})
    assert _breeding_actions(out, P0) == [convert_raw.ACTION_HATCH, convert_raw.ACTION_MOVE]
    assert not any("breeding" in note and "BT20-010" in note for note in inferred), inferred


def test_without_any_board_evidence_the_toggle_still_decides():
    # Regression guard: hatch -> occupied -> move -> empty -> hatch, no boards anywhere.
    out, _ = convert_raw.convert(_game(
        _breed(P0, 2), _pass(P0, 2),
        _breed(P1, 3, do=False), _pass(P1, 3),
        _breed(P0, 4), _pass(P0, 4),
        _breed(P1, 5, do=False), _pass(P1, 5),
        _breed(P0, 6), _pass(P0, 6),
    ), cards={})
    assert _breeding_actions(out, P0) == [convert_raw.ACTION_HATCH, convert_raw.ACTION_MOVE,
                                          convert_raw.ACTION_HATCH]


def test_snapshot_breeding_zone_overrides_the_toggle():
    # A snapshot row's `breeding` list is exact; the toggle defers to it.
    snap = {"method": "Snapshot.Request", "turn_count": 3, "you": P1, "is_ai": False, "players": [
        {"id": 0, "hand": [], "library": [], "egg_library": [], "breeding": []},
        {"id": 1, "hand": [], "library": [], "egg_library": [], "breeding": []},
    ]}
    out, _ = convert_raw.convert(_game(
        _breed(P0, 2),
        _effect_play_into_breeding_digimon(P0, 2, "BT20-010"),
        _pass(P0, 2),
        snap,
        _breed(P0, 4),
        _pass(P0, 4),
    ), cards={})
    assert _breeding_actions(out, P0) == [convert_raw.ACTION_HATCH, convert_raw.ACTION_HATCH]
