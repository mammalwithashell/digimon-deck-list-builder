"""Behavioral tests for BT9-112 DeathXmon (Lv.7 Purple/Black).

Card text:
When you would play this card, reduce its memory cost by 3 for each
    Digimon and Tamer your opponent has in play.
[On Play] [When Digivolving] <De-Digivolve 1> all of your opponent's Digimon.
    Then, delete all of your opponent's level 4 or lower Digimon.
[End of Opponent's Turn] [Once Per Turn] Delete all of your opponent's
    Digimon with the lowest play cost.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT9112DeathXmon:
    """Tests for BT9-112 DeathXmon."""

    # --- Cost reduction tests ---

    def test_cost_reduction_per_opponent_field(self, debug_runner):
        """Cost should be reduced by 3 per opponent Digimon/Tamer."""
        runner = debug_runner(initial_memory=5)

        # Place 3 opponent Digimon and 1 Tamer = 4 permanents -> -12 cost
        runner.place_on_field(2, ["ST1-03"])
        runner.place_on_field(2, ["ST1-06"])
        runner.place_on_field(2, ["ST1-07"])
        runner.place_on_field(2, ["ST1-12"])  # Tamer

        game = runner.game

        # Get the BeforePayCost effect from BT9-112
        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        db = CardDatabase()

        cs = db.create_card_source("BT9-112", game.player1)
        effects = cs.effect_list(None)
        cost_effects = [
            e for e in effects
            if e.timing == EffectTiming.BeforePayCost
        ]
        assert len(cost_effects) >= 1, "Should have BeforePayCost effect"

        # Check cost reduction value function
        cost_fn = getattr(cost_effects[0], '_cost_reduction_value_fn', None)
        assert cost_fn is not None, "Should have _cost_reduction_value_fn"
        reduction = cost_fn({})
        assert reduction == 12, f"Cost reduction should be 12 (4 * 3), got {reduction}"

    def test_cost_reduction_leak_guard(self, debug_runner):
        """BeforePayCost condition should only apply to THIS card (leak guard)."""
        runner = debug_runner(initial_memory=5)

        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        db = CardDatabase()

        cs = db.create_card_source("BT9-112", runner.game.player1)
        effects = cs.effect_list(None)
        cost_effects = [
            e for e in effects
            if e.timing == EffectTiming.BeforePayCost
        ]
        assert len(cost_effects) >= 1

        # Condition should check card_source identity
        other_cs = db.create_card_source("ST1-03", runner.game.player1)
        # Should return False for a different card
        result = cost_effects[0].can_use_condition({'card_source': other_cs})
        assert not result, "Cost reduction should NOT apply to other cards (leak guard)"

        # Should return True for THIS card
        result = cost_effects[0].can_use_condition({'card_source': cs})
        assert result, "Cost reduction should apply to THIS card"

    def test_process0_does_not_set_temp_play_cost_reduction(self, debug_runner):
        """process0 must NOT set _temp_play_cost_reduction (broken double-reduction pattern).

        The _cost_reduction_value_fn already handles cost reduction correctly.
        If process0 also sets _temp_play_cost_reduction, it creates a fragile
        double-reduction path that could leak across plays.
        """
        runner = debug_runner(initial_memory=20)

        # Set up opponent field so cost reduction is non-zero
        runner.place_on_field(2, ["ST1-03"])
        runner.place_on_field(2, ["ST1-06"])

        game = runner.game

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT9-112", game.player1)

        effects = cs.effect_list(None)
        cost_effects = [
            e for e in effects
            if e.timing == EffectTiming.BeforePayCost
        ]
        assert len(cost_effects) >= 1

        # Ensure _temp_play_cost_reduction starts at 0
        game.player1._temp_play_cost_reduction = 0

        # Fire the process callback (simulating commit phase)
        if cost_effects[0].on_process_callback:
            cost_effects[0].on_process_callback({
                'player': game.player1,
                'game': game,
            })

        # _temp_play_cost_reduction should NOT have been set by process0
        temp_reduction = getattr(game.player1, '_temp_play_cost_reduction', 0)
        assert temp_reduction == 0, (
            f"process0 should NOT set _temp_play_cost_reduction (got {temp_reduction}). "
            f"The _cost_reduction_value_fn already handles cost reduction."
        )

    def test_calculate_play_cost_integration(self, debug_runner):
        """Verify calculate_play_cost gives correct cost (no double-reduction).

        Base cost 20, 3 opponent Digimon = -9 reduction -> cost should be 11.
        """
        runner = debug_runner(initial_memory=20)

        runner.place_on_field(2, ["ST1-03"])
        runner.place_on_field(2, ["ST1-06"])
        runner.place_on_field(2, ["ST1-07"])

        game = runner.game

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT9-112", game.player1)
        game.player1.hand_cards.append(cs)

        # Calculate cost without commit (for display/mask)
        cost_display = game.calculate_play_cost(game.player1, cs)
        assert cost_display == 11, (
            f"Cost with 3 opponent Digimon should be 20 - 9 = 11, got {cost_display}"
        )

        # Calculate cost with commit (for actual play)
        cost_commit = game.calculate_play_cost(game.player1, cs, commit=True)
        assert cost_commit == 11, (
            f"Committed cost should also be 11, got {cost_commit}"
        )

    def test_cost_reduction_zero_opponents(self, debug_runner):
        """With no opponent Digimon/Tamer, cost should remain at base (20)."""
        runner = debug_runner(initial_memory=20)
        game = runner.game

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT9-112", game.player1)
        game.player1.hand_cards.append(cs)

        cost = game.calculate_play_cost(game.player1, cs)
        assert cost == 20, f"Cost with 0 opponents should be 20, got {cost}"

    # --- De-Digivolve + Delete tests ---

    def test_on_play_de_digivolve_all_then_delete_lv4(self, debug_runner):
        """[On Play] should De-Digivolve 1 all opponent Digimon, then delete Lv.4 or lower."""
        runner = debug_runner(initial_memory=10)

        # Opponent field:
        # - Lv.5 MetalGreymon (ST1-09) on top of Lv.4 Coredramon (ST1-06) on top of Lv.3 Agumon
        #   After de-digivolve: becomes Lv.4 Coredramon -> gets deleted (Lv.4 or lower)
        # - Lv.4 Greymon (ST1-07) on top of Lv.3 Agumon
        #   After de-digivolve: becomes Lv.3 Agumon -> gets deleted (Lv.3 <= 4)
        opp1 = runner.place_on_field(2, ["ST1-03", "ST1-06", "ST1-09"])  # Lv.5
        opp2 = runner.place_on_field(2, ["ST1-03", "ST1-07"])  # Lv.4

        game = runner.game
        perm = runner.place_on_field(1, ["BT9-112"])

        # Find the On Play effect
        top_card = perm.top_card
        effects = top_card.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
            and e.on_process_callback is not None
        ]
        assert len(on_play) >= 1, "Should have On Play effect"

        # Execute the effect
        on_play[0].on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Both opponent Digimon should be deleted:
        # opp1: was Lv.5, de-digivolve 1 -> Lv.4 (Coredramon) -> deleted (<=4)
        # opp2: was Lv.4, de-digivolve 1 -> Lv.3 (Agumon) -> deleted (<=4)
        snap = runner.snapshot()
        assert len(snap.p2_field) == 0, (
            f"Both opponent Digimon should be deleted, but {len(snap.p2_field)} remain"
        )

    def test_on_play_de_digivolve_preserves_high_level(self, debug_runner):
        """De-digivolve then delete should preserve Digimon that remain Lv.5+."""
        runner = debug_runner(initial_memory=10)

        # Opponent: Lv.6 on top of Lv.5 on top of Lv.4 on top of Lv.3
        # After de-digivolve 1: becomes Lv.5 -> NOT deleted (Lv.5 > 4)
        opp = runner.place_on_field(2, ["ST1-03", "ST1-06", "ST1-09", "ST1-11"])

        game = runner.game
        perm = runner.place_on_field(1, ["BT9-112"])

        top_card = perm.top_card
        effects = top_card.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
            and e.on_process_callback is not None
        ]
        on_play[0].on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Lv.6 -> de-digivolve 1 -> Lv.5 -> should survive
        snap = runner.snapshot()
        opp_field = [s for s in snap.p2_field if s.is_digimon]
        assert len(opp_field) == 1, (
            f"Lv.5 Digimon should survive de-digivolve+delete, but {len(opp_field)} remain"
        )
        assert opp_field[0].level == 5, f"Surviving Digimon should be Lv.5, got {opp_field[0].level}"

    def test_de_digivolve_no_cards_below_still_checks_level(self, debug_runner):
        """A bare Lv.3 Digimon (no digi cards) can't de-digivolve but is Lv.3 -> deleted."""
        runner = debug_runner(initial_memory=10)

        # Bare Lv.3 Agumon (no digivolution stack)
        opp = runner.place_on_field(2, ["ST1-03"])

        game = runner.game
        perm = runner.place_on_field(1, ["BT9-112"])

        top_card = perm.top_card
        effects = top_card.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
            and e.on_process_callback is not None
        ]
        on_play[0].on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Lv.3 -> no de-digivolve possible -> still Lv.3 -> deleted (<=4)
        snap = runner.snapshot()
        assert len(snap.p2_field) == 0, "Bare Lv.3 Digimon should be deleted"

    def test_when_digivolving_effect_same_as_on_play(self, debug_runner):
        """[When Digivolving] should perform the same de-digivolve + delete as On Play."""
        runner = debug_runner(initial_memory=10)

        # Opponent: bare Lv.4 Digimon -> should be deleted (Lv.4 <= 4)
        runner.place_on_field(2, ["ST1-07"])

        game = runner.game
        perm = runner.place_on_field(1, ["BT9-112"])

        top_card = perm.top_card
        effects = top_card.effect_list(None)
        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
            and e.on_process_callback is not None
        ]
        assert len(wd) >= 1, "Should have When Digivolving effect"

        wd[0].on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        snap = runner.snapshot()
        assert len(snap.p2_field) == 0, (
            "When Digivolving should delete Lv.4 Digimon same as On Play"
        )

    # --- End of Opponent's Turn tests ---

    def test_end_of_opponent_turn_deletes_lowest_cost(self, debug_runner):
        """[End of Opponent's Turn] should delete all Digimon with the lowest play cost."""
        runner = debug_runner(initial_memory=10)

        # DeathXmon on P1 field
        perm = runner.place_on_field(1, ["BT9-112"])

        # Opponent's field: ST1-03 (cost 3), ST1-06 (cost 5), ST1-03 (cost 3)
        runner.place_on_field(2, ["ST1-03"])
        runner.place_on_field(2, ["ST1-06"])
        runner.place_on_field(2, ["ST1-03"])

        game = runner.game

        # Find the End of Turn effect
        top_card = perm.top_card
        effects = top_card.effect_list(None)
        eot = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn
            and e.on_process_callback is not None
        ]
        assert len(eot) >= 1, "Should have OnEndTurn effect"

        # Execute on opponent's turn
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        eot[0].on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Both Agumon (cost 3, the lowest) should be deleted, Coredramon (cost 5) survives
        snap = runner.snapshot()
        p2_digimon = [s for s in snap.p2_field if s.is_digimon]
        assert len(p2_digimon) == 1, (
            f"Only 1 Digimon should survive (Coredramon), but {len(p2_digimon)} remain"
        )
        assert p2_digimon[0].card_id == "ST1-06", (
            f"Surviving Digimon should be Coredramon (ST1-06), got {p2_digimon[0].card_id}"
        )

    def test_end_of_turn_condition_only_opponent_turn(self, debug_runner):
        """End of turn effect condition should only pass on opponent's turn."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, ["BT9-112"])
        game = runner.game

        top_card = perm.top_card
        effects = top_card.effect_list(None)
        eot = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn
            and e.on_process_callback is not None
        ]
        assert len(eot) >= 1

        # On own turn, condition should fail
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False
        assert not eot[0].can_use_condition({}), (
            "End of turn effect should NOT activate on own turn"
        )

        # On opponent's turn, condition should pass
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True
        assert eot[0].can_use_condition({}), (
            "End of turn effect SHOULD activate on opponent's turn"
        )

    def test_end_of_turn_once_per_turn(self, debug_runner):
        """End of turn effect should be once per turn."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, ["BT9-112"])

        top_card = perm.top_card
        effects = top_card.effect_list(None)
        eot = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn
            and e.on_process_callback is not None
        ]
        assert len(eot) >= 1

        assert eot[0].max_count_per_turn == 1, (
            "End of turn effect should be once per turn"
        )

    def test_end_of_turn_no_opponent_digimon(self, debug_runner):
        """End of turn effect should do nothing if opponent has no Digimon."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, ["BT9-112"])
        game = runner.game

        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        top_card = perm.top_card
        effects = top_card.effect_list(None)
        eot = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn
            and e.on_process_callback is not None
        ]

        # Should not raise when no opponent Digimon exist
        eot[0].on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # No crash, no changes
        snap = runner.snapshot()
        assert len(snap.p2_field) == 0
