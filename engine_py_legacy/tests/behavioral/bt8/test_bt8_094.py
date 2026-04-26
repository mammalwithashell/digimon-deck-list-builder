"""Behavioral tests for BT8-094 Digimon Emperor (Tamer, White, Cost 3).

Card text:
    [All Turns] When one of your opponent's level 5 or lower Digimon is deleted,
    you may suspend this Tamer to <Draw 1>.
    [Opponent's Turn] When one of your opponent's level 3 Digimon is moved
    from their breeding area to their battle area, gain 2 memory.
    [Security] Play this card without paying its memory cost.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


def _find_deletion_observer(perm):
    """Find the _is_deletion_observer effect on a permanent."""
    for source in perm.card_sources:
        for eff in source.effect_list(EffectTiming.NoTiming):
            if getattr(eff, '_is_deletion_observer', False):
                return eff
    return None


def _find_on_move_effect(perm):
    """Find the OnMove effect on a permanent."""
    for source in perm.card_sources:
        for eff in source.effect_list(EffectTiming.OnMove):
            if eff.timing == EffectTiming.OnMove:
                return eff
    return None


@pytest.mark.behavioral
class TestBT8094DigimonEmperor:
    """Tests for BT8-094 Digimon Emperor."""

    # ── Effect 0: Deletion observer — draw 1 when opp lv5- deleted ──

    def test_effect0_has_deletion_observer_flag(self, debug_runner):
        """Effect 0 should use NoTiming + _is_deletion_observer pattern."""
        runner = debug_runner(
            deck1=["BT8-094"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=3,
        )
        runner.set_phase("Main")
        tamer = runner.place_on_field(1, ["BT8-094"])

        observer = _find_deletion_observer(tamer)
        assert observer is not None, (
            "BT8-094 should have a _is_deletion_observer effect with NoTiming"
        )
        assert observer.is_optional is True, "Draw effect should be optional ('you may')"

    def test_effect0_draws_when_opp_lv5_or_lower_deleted(self, debug_runner):
        """When opponent's lv5- Digimon is deleted, tamer controller draws 1."""
        runner = debug_runner(
            deck1=["BT8-094"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=3,
        )
        runner.set_phase("Main")
        tamer = runner.place_on_field(1, ["BT8-094"])
        # Place a lv3 Digimon on opponent's field
        opp_digi = runner.place_on_field(2, ["ST1-03"])  # Agumon Lv3

        snap_before = runner.snapshot()
        p1_hand_before = len(snap_before.p1_hand)

        # Delete the opponent's Digimon
        runner.game.player2.delete_permanent(opp_digi, removal_cause='effect')
        runner.auto_resolve()

        snap_after = runner.snapshot()
        p1_hand_after = len(snap_after.p1_hand)

        assert p1_hand_after == p1_hand_before + 1, (
            f"Player 1 should draw 1 card. Hand before: {p1_hand_before}, after: {p1_hand_after}"
        )

    def test_effect0_suspends_tamer_as_cost(self, debug_runner):
        """The tamer should be suspended as a cost when the effect fires."""
        runner = debug_runner(
            deck1=["BT8-094"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=3,
        )
        runner.set_phase("Main")
        tamer = runner.place_on_field(1, ["BT8-094"])
        opp_digi = runner.place_on_field(2, ["ST1-03"])

        assert not tamer.is_suspended, "Tamer starts unsuspended"

        runner.game.player2.delete_permanent(opp_digi, removal_cause='effect')
        runner.auto_resolve()

        assert tamer.is_suspended, "Tamer should be suspended after effect fires"

    def test_effect0_does_not_fire_if_tamer_already_suspended(self, debug_runner):
        """If tamer is already suspended, should not draw."""
        runner = debug_runner(
            deck1=["BT8-094"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=3,
        )
        runner.set_phase("Main")
        tamer = runner.place_on_field(1, ["BT8-094"])
        tamer.suspend()  # Already suspended
        opp_digi = runner.place_on_field(2, ["ST1-03"])

        snap_before = runner.snapshot()
        p1_hand_before = len(snap_before.p1_hand)

        runner.game.player2.delete_permanent(opp_digi, removal_cause='effect')
        runner.auto_resolve()

        snap_after = runner.snapshot()
        p1_hand_after = len(snap_after.p1_hand)

        assert p1_hand_after == p1_hand_before, (
            "Should NOT draw when tamer is already suspended"
        )

    def test_effect0_does_not_fire_for_own_digimon_deleted(self, debug_runner):
        """Should NOT trigger when your own Digimon is deleted."""
        runner = debug_runner(
            deck1=["BT8-094"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=3,
        )
        runner.set_phase("Main")
        tamer = runner.place_on_field(1, ["BT8-094"])
        own_digi = runner.place_on_field(1, ["ST1-03"])

        snap_before = runner.snapshot()
        p1_hand_before = len(snap_before.p1_hand)

        runner.game.player1.delete_permanent(own_digi, removal_cause='effect')
        runner.auto_resolve()

        snap_after = runner.snapshot()
        p1_hand_after = len(snap_after.p1_hand)

        assert p1_hand_after == p1_hand_before, (
            "Should NOT draw when your own Digimon is deleted"
        )

    def test_effect0_does_not_fire_for_lv6_digimon(self, debug_runner):
        """Should NOT trigger for opponent's level 6+ Digimon."""
        runner = debug_runner(
            deck1=["BT8-094"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=3,
        )
        runner.set_phase("Main")
        tamer = runner.place_on_field(1, ["BT8-094"])
        # Place a Lv6 Digimon on opponent's field (WarGreymon)
        opp_digi = runner.place_on_field(2, ["ST1-11"])  # WarGreymon Lv6

        snap_before = runner.snapshot()
        p1_hand_before = len(snap_before.p1_hand)

        runner.game.player2.delete_permanent(opp_digi, removal_cause='effect')
        runner.auto_resolve()

        snap_after = runner.snapshot()
        p1_hand_after = len(snap_after.p1_hand)

        assert p1_hand_after == p1_hand_before, (
            "Should NOT draw for opponent's level 6 Digimon"
        )

    def test_effect0_fires_for_lv5_digimon(self, debug_runner):
        """Should trigger for opponent's level 5 Digimon (boundary case)."""
        runner = debug_runner(
            deck1=["BT8-094"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=3,
        )
        runner.set_phase("Main")
        tamer = runner.place_on_field(1, ["BT8-094"])
        # ST1-09 MetalGreymon is Lv5
        opp_digi = runner.place_on_field(2, ["ST1-09"])

        snap_before = runner.snapshot()
        p1_hand_before = len(snap_before.p1_hand)

        runner.game.player2.delete_permanent(opp_digi, removal_cause='effect')
        runner.auto_resolve()

        snap_after = runner.snapshot()
        p1_hand_after = len(snap_after.p1_hand)

        assert p1_hand_after == p1_hand_before + 1, (
            f"Should draw 1 for opponent's level 5 Digimon. Hand before: {p1_hand_before}, after: {p1_hand_after}"
        )

    def test_effect0_does_not_fire_for_tamer_deleted(self, debug_runner):
        """Should NOT trigger for opponent's Tamer being removed."""
        runner = debug_runner(
            deck1=["BT8-094"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=3,
        )
        runner.set_phase("Main")
        tamer = runner.place_on_field(1, ["BT8-094"])
        # Place opponent tamer
        opp_tamer = runner.place_on_field(2, ["BT8-094"])

        snap_before = runner.snapshot()
        p1_hand_before = len(snap_before.p1_hand)

        runner.game.player2.delete_permanent(opp_tamer, removal_cause='effect')
        runner.auto_resolve()

        snap_after = runner.snapshot()
        p1_hand_after = len(snap_after.p1_hand)

        assert p1_hand_after == p1_hand_before, (
            "Should NOT draw when opponent's Tamer is deleted (must be Digimon)"
        )

    # ── Effect 1: OnMove — gain 2 memory when opp moves lv3 from breeding ──

    def test_effect1_has_on_move_timing(self, debug_runner):
        """Effect 1 should have OnMove timing."""
        runner = debug_runner(
            deck1=["BT8-094"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=3,
        )
        runner.set_phase("Main")
        tamer = runner.place_on_field(1, ["BT8-094"])

        on_move = _find_on_move_effect(tamer)
        assert on_move is not None, "BT8-094 should have an OnMove effect"

    def test_effect1_gains_memory_when_opp_moves_lv3(self, debug_runner):
        """Gain 2 memory when opponent moves a lv3 Digimon from breeding."""
        runner = debug_runner(
            deck1=["BT8-094"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-01"] * 4 + ["ST1-03"] * 46,
            initial_memory=-3,  # Negative = opponent's turn
            first_player=2,
        )
        # Set up: it's P2's turn, P1 has Digimon Emperor
        tamer = runner.place_on_field(1, ["BT8-094"])
        # Place a lv3 in P2's breeding area (ST1-01 digitama + ST1-03 rookie)
        runner.place_in_breeding(2, ["ST1-01", "ST1-03"])

        # Verify it's opponent's turn for P1
        assert not runner.game.player1.is_my_turn, "Should be P2's turn (not P1's)"

        snap_before = runner.snapshot()
        memory_before = snap_before.memory

        # P2 moves from breeding to battle area
        runner.game.player2.move_from_breeding()
        # Fire OnMove effects
        moved = runner.game.player2.battle_area[-1]
        runner.game.execute_effects(EffectTiming.OnMove, {"moved_permanent": moved})
        runner.auto_resolve()

        snap_after = runner.snapshot()
        memory_after = snap_after.memory

        # Memory gain of 2 for P1 (non-turn-player) decreases game.memory by 2
        # (game.memory is from turn player's perspective; P2 is turn player)
        assert memory_after == memory_before - 2, (
            f"P1 should gain 2 memory (game.memory decreases by 2). Before: {memory_before}, After: {memory_after}"
        )

    def test_effect1_does_not_trigger_on_own_turn(self, debug_runner):
        """Should NOT trigger on your own turn (Opponent's Turn only)."""
        runner = debug_runner(
            deck1=["BT8-094"] * 4 + ["BT1-01"] * 4 + ["ST1-03"] * 42,
            deck2=["ST1-03"] * 50,
            initial_memory=3,
            first_player=1,
        )
        # P1's turn, P1 has tamer
        tamer = runner.place_on_field(1, ["BT8-094"])

        on_move = _find_on_move_effect(tamer)
        assert on_move is not None

        # Simulate a move context on P1's own turn
        dummy_perm = runner.place_on_field(2, ["ST1-03"])  # lv3
        ctx = {
            'game': runner.game,
            'player': runner.game.player1,
            'permanent': tamer,
            'moved_permanent': dummy_perm,
            'turn_player': runner.game.turn_player,
            'opponent_player': runner.game.opponent_player,
        }
        # P1's turn, so condition should fail
        assert runner.game.player1.is_my_turn is True
        result = on_move.can_use_condition(ctx)
        assert result is False, "OnMove should NOT trigger on your own turn"

    def test_effect1_does_not_trigger_for_lv4_or_higher(self, debug_runner):
        """Should NOT trigger for level 4+ Digimon moving from breeding."""
        runner = debug_runner(
            deck1=["BT8-094"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-01"] * 4 + ["ST1-06"] * 46,
            initial_memory=-3,
            first_player=2,
        )
        tamer = runner.place_on_field(1, ["BT8-094"])
        # Place a lv4 in P2's breeding area (ST1-01 digitama + ST1-03 rookie + ST1-06 Lv4)
        runner.place_in_breeding(2, ["ST1-01", "ST1-03", "ST1-06"])

        snap_before = runner.snapshot()
        memory_before = snap_before.memory

        runner.game.player2.move_from_breeding()
        moved = runner.game.player2.battle_area[-1]
        runner.game.execute_effects(EffectTiming.OnMove, {"moved_permanent": moved})
        runner.auto_resolve()

        snap_after = runner.snapshot()
        memory_after = snap_after.memory

        assert memory_after == memory_before, (
            f"Should NOT gain memory for lv4 Digimon. Before: {memory_before}, After: {memory_after}"
        )
