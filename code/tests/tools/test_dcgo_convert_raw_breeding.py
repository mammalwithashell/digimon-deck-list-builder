"""convert_raw.py: SetBreedingPhase(actor, doBreeding=true) is HATCH (60) or MOVE_FROM_BREEDING (61) by occupancy.

DCGO decides from its own breeding area (TurnStateMachine.SetBreedingPhase); the converter must know whether that
area holds a Digimon at the decision. A toggled flag (hatch -> occupied, move -> empty) misses everything an EFFECT
does to the breeding area - g6 (raw-20261113T025510Z): BT20-095's <Delay> moved Ryudamon (BT20-010) out of P0's
breeding area during P1's turn 7, so P0's next SetBreedingPhase(0, true) was a hatch, not a move.

Evidence the converter reads, strongest first:
  - Snapshot rows (`players[].breeding`): exact.
  - Field boards: `SelectPermanentEffect.SetTargetFrames` and the attack / activate packet rows carry
    GetFieldPermanents() - every permanent INCLUDING the breeding one (Player.cs:669-685, the breeding frame is the
    last FieldPermanents index, Player.cs:41-54). The breeding permanent is therefore the board's LAST entry: a known
    breeding top card that is not last has left the breeding area (wherever else its id appears), and a Digi-Egg on
    the board can only be the breeding card (no DP, never moved: CR 4.3 4-17-2).
  - Battle boards: the play packet, SetAttackingPermaent and PlayCardClass rows carry GetBattleAreaPermanents(). More
    copies of the breeding top card there than the battle area held is the breeding card having moved there OR a copy
    entering unseen: the area becomes UNKNOWN, and a doBreeding on an unknown area is a HATCH only when a later row shows
    a Digi-Egg in that breeding area; otherwise the conversion stops there (truncated) rather than guess.
  - Plays: a PlayCardClass row whose target is in the breeding area (`target_breeding`), or that plays into it
    (`breeding_area`, owner from the row's hand), leaves it occupied by that card; a non-turn player's target the mod
    could not place (TargetSlotJson searches the turn player only) that is absent from that seat's battle board is its
    breeding Digimon.
The decisions' own effect is the fallback only when no evidence speaks; when evidence overrides it the converter notes so.

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
    """An effect playing a card INTO the breeding area (PlayCardClass._isBreedingArea), from the owner's hand: the mod
    logs whose hand (g6 seq 1068: hand.player 0), which names whose breeding area it is."""
    return {"method": "PlayCardClass.PlayCard", "tp": actor, "turn": turn, "cards": [card], "pay_cost": False,
            "breeding_area": True, "target": None, "hand": {"player": actor, "slot": 0, "cards": [{"card_index": 40}]}}


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



# --- The field board's LAST permanent is the breeding one -------------------------------------------------------------
# GetFieldPermanents() walks FieldPermanents in FrameID order (Player.cs:669-684) and the breeding frame is the last
# FrameID (Player.cs:41-54, isBattleAreaFrameID Player.cs:1561-1563), so a breeding Digimon is always the board's last
# entry. Its card id still being somewhere on the board says nothing: BT20-095's <Delay> body can move it to the battle
# area without digivolving it, and another copy can be in play.

EGG_CARDS = {"P-176": {"card_kind": 3, "level": 2, "play_cost": 0}}


def _digivolve_breeding_with_boards(actor, turn, card, battle_p0, battle_p1, target="EGG"):
    """A digivolution onto the turn player's breeding Digimon; the row's `boards` are both battle areas before it
    (TargetSlotJson: GetBattleAreaPermanents, Core.cs)."""
    row = _effect_play_into_breeding_digimon(actor, turn, card)
    row["target"] = {"card_id": target, "card_index": 1}
    row["boards"] = [list(battle_p0), list(battle_p1)]
    return row


def _attack(tp, turn, battle_p0, battle_p1):
    """SetAttackingPermaent: its boards are GetBattleAreaPermanents() of both seats (battle area only, Core.cs)."""
    return {"method": "TurnStateMachine.SetAttackingPermaent", "tp": tp, "turn": turn, "args": [0, -1],
            "boards": [list(battle_p0), list(battle_p1)]}


def test_breeding_digimon_moved_below_another_battle_digimon_is_no_longer_last_on_the_field_board():
    # BT20-095's <Delay> "Do not digivolve": Ryudamon (BT20-010) moves to a battle frame below BT20-087's and stays
    # Ryudamon. Its id is still on P0's field, but not last, so the breeding area is empty: P0's next doBreeding hatches.
    out, inferred = convert_raw.convert(_game(
        _breed(P0, 2),
        _effect_play_into_breeding_digimon(P0, 2, "BT20-010"),
        _pass(P0, 2),
        _breed(P1, 3, do=False),
        _field_pick(P0, 3, ["BT20-010", "BT20-087"], ["ST24-04"]),
        _pass(P1, 3),
        _breed(P0, 4),
        _pass(P0, 4),
    ), cards={})
    assert _breeding_actions(out, P0) == [convert_raw.ACTION_HATCH, convert_raw.ACTION_HATCH]
    assert any("BT20-010" in note and "breeding" in note for note in inferred), inferred


def test_another_copy_on_the_field_does_not_hide_the_breeding_digimon_leaving():
    out, _ = convert_raw.convert(_game(
        _breed(P0, 2),
        _effect_play_into_breeding_digimon(P0, 2, "BT20-010"),
        _pass(P0, 2),
        _breed(P1, 3, do=False),
        _field_pick(P0, 3, ["BT20-010", "BT20-010", "BT20-087"], ["ST24-04"]),
        _pass(P1, 3),
        _breed(P0, 4),
        _pass(P0, 4),
    ), cards={})
    assert _breeding_actions(out, P0) == [convert_raw.ACTION_HATCH, convert_raw.ACTION_HATCH]


def test_another_copy_in_the_battle_area_keeps_the_area_occupied():
    # Regression guard: a second Ryudamon was already in P0's battle area when the breeding one became Ryudamon (the
    # digivolve row's battle board shows it); the battle board on P1's attack still shows exactly that one copy, and the
    # field board ends with Ryudamon: the breeding area still holds it, so the next doBreeding is a MOVE.
    out, inferred = convert_raw.convert(_game(
        _breed(P0, 2),
        _digivolve_breeding_with_boards(P0, 2, "BT20-010", ["BT20-010", "BT20-087"], []),
        _pass(P0, 2),
        _breed(P1, 3, do=False),
        _attack(P1, 3, ["BT20-010", "BT20-087"], ["ST24-04"]),
        _field_pick(P0, 3, ["BT20-010", "BT20-087", "BT20-010"], ["ST24-04"]),
        _pass(P1, 3),
        _breed(P0, 4),
        _pass(P0, 4),
    ), cards={})
    assert _breeding_actions(out, P0) == [convert_raw.ACTION_HATCH, convert_raw.ACTION_MOVE]
    assert not any("P0 breeding" in note for note in inferred), inferred


def _moved_to_the_last_frame(*after_decision):
    # P0's breeding Ryudamon (BT20-010) became Ryudamon with no copy in P0's battle area; during P1's turn the field
    # board still ends with BT20-010 (breeding OR the battle area's last frame) and P1's attack row's battle board shows
    # a Ryudamon in P0's battle area. Either the breeding one moved there (the area is empty) or another copy entered by a
    # path the converter did not see: the boards cannot tell which.
    return _game(
        _breed(P0, 2),
        _digivolve_breeding_with_boards(P0, 2, "BT20-010", ["BT20-087"], []),
        _pass(P0, 2),
        _breed(P1, 3, do=False),
        _field_pick(P0, 3, ["BT20-087", "BT20-010"], ["ST24-04"]),
        _attack(P1, 3, ["BT20-087", "BT20-010"], ["ST24-04"]),
        _pass(P1, 3),
        _breed(P0, 4),
        *after_decision,
        _pass(P0, 4),
    )


def test_breeding_digimon_that_may_have_moved_to_the_last_battle_frame_stops_the_conversion():
    out, _ = convert_raw.convert(_moved_to_the_last_frame(), cards=dict(EGG_CARDS), allow_truncated=True)
    assert convert_raw.LAST_TRUNCATION is not None and "breeding" in convert_raw.LAST_TRUNCATION, \
        convert_raw.LAST_TRUNCATION
    assert _breeding_actions(out, P0) == [convert_raw.ACTION_HATCH]  # the undecidable one is not written
    assert not any(r["type"] == "game_end" for r in out)


def test_unknown_occupancy_is_decided_by_the_egg_in_the_breeding_area_after_the_decision():
    # After the doBreeding, P0 digivolves onto a Digi-Egg (P-176) in its breeding area. A Digi-Egg has no DP and can
    # never be moved (CR 4.3 4-17-2), so it got there by THIS breeding decision: a HATCH.
    out, inferred = convert_raw.convert(_moved_to_the_last_frame(
        _digivolve_breeding_with_boards(P0, 4, "BT20-011", ["BT20-087", "BT20-010"], [], target="P-176"),
    ), cards=dict(EGG_CARDS))
    assert _breeding_actions(out, P0) == [convert_raw.ACTION_HATCH, convert_raw.ACTION_HATCH]
    assert any("P-176" in note for note in inferred), inferred


def test_non_turn_player_breeding_digimon_digivolved_by_an_effect_stays_in_the_breeding_area():
    # The mod's TargetSlotJson looks for the target among the TURN player's permanents only (Core.cs), so an effect
    # digivolving P0's breeding Ryudamon (BT20-010) during P1's turn logs target_slot -1 / target_breeding false. The
    # target is P0's (not found on P1's side) and not in P0's battle area (the row's battle boards): it is P0's breeding
    # Digimon, now EX13-055. The field board then ends with EX13-055 and P0's next doBreeding is a MOVE.
    out, _ = convert_raw.convert(_game(
        _breed(P0, 2),
        _effect_play_into_breeding_digimon(P0, 2, "BT20-010"),
        _pass(P0, 2),
        _breed(P1, 3, do=False),
        {"method": "PlayCardClass.PlayCard", "tp": P1, "turn": 3, "cards": ["EX13-055"], "pay_cost": False,
         "breeding_area": False, "target": {"card_id": "BT20-010", "card_index": 39}, "target_slot": -1,
         "target_breeding": False, "boards": [["BT20-087"], ["ST24-04"]], "hand": {"player": -1, "slot": -1}},
        _field_pick(P0, 3, ["BT20-087", "EX13-055"], ["ST24-04"]),
        _pass(P1, 3),
        _breed(P0, 4),
        _pass(P0, 4),
    ), cards={})
    assert _breeding_actions(out, P0) == [convert_raw.ACTION_HATCH, convert_raw.ACTION_MOVE]


def test_non_turn_player_digivolve_of_a_moved_breeding_digimon_in_the_battle_area():
    # Regression guard, the g6 shape (raw-20261113T025510Z#g6 seq 947-953): BT20-095's <Delay> moved Ryudamon into
    # P0's battle area during P1's turn and an effect digivolved it there into EX13-055; the field board then ends
    # with BT20-087, not a breeding Digimon: P0's next doBreeding hatches.
    out, _ = convert_raw.convert(_game(
        _breed(P0, 2),
        _effect_play_into_breeding_digimon(P0, 2, "BT20-010"),
        _pass(P0, 2),
        _breed(P1, 3, do=False),
        {"method": "PlayCardClass.PlayCard", "tp": P1, "turn": 3, "cards": ["EX13-055"], "pay_cost": False,
         "breeding_area": False, "target": {"card_id": "BT20-010", "card_index": 39}, "target_slot": -1,
         "target_breeding": False, "boards": [["BT20-010", "BT20-087"], ["ST24-04"]],
         "hand": {"player": -1, "slot": -1}},
        _field_pick(P0, 3, ["EX13-055", "BT20-087"], ["ST24-04"]),
        _pass(P1, 3),
        _breed(P0, 4),
        _pass(P0, 4),
    ), cards={})
    assert _breeding_actions(out, P0) == [convert_raw.ACTION_HATCH, convert_raw.ACTION_HATCH]


def test_breeding_area_play_of_an_unknown_owner_goes_to_the_seat_whose_field_board_shows_it():
    # An effect plays P-198 into a breeding area during P1's turn from outside a hand (the mod logs hand.player -1, so
    # the row does not say whose area). P1's field board does not hold it, P0's ends with it: it is P0's.
    out, _ = convert_raw.convert(_game(
        _breed(P0, 2, do=False),
        _pass(P0, 2),
        _breed(P1, 3, do=False),
        {"method": "PlayCardClass.PlayCard", "tp": P1, "turn": 3, "cards": ["P-198"], "pay_cost": False,
         "breeding_area": True, "target": None, "hand": {"player": -1, "slot": -1}},
        _field_pick(P1, 3, ["BT20-087", "P-198"], ["ST24-04"]),
        _pass(P1, 3),
        _breed(P0, 4),
        _pass(P0, 4),
        _breed(P1, 5),
        _pass(P1, 5),
    ), cards={})
    assert _breeding_actions(out, P0) == [convert_raw.ACTION_PASS, convert_raw.ACTION_MOVE]
    assert _breeding_actions(out, P1) == [convert_raw.ACTION_PASS, convert_raw.ACTION_HATCH]
