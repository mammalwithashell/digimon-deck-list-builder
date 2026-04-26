"""Behavioral tests for P-123 Ukkomon (Lv.3 White Digimon, Cost 3).

Card text:
[Your Turn] [Once Per Turn] When one of your Digimon moves from the breeding
area to the battle area, you may hatch in your breeding area. Then, gain 1
memory.

Key clauses:
  - Trigger: OnMove (your Digimon moves from breeding to battle area)
  - Your Turn only
  - Once Per Turn
  - Optional hatch ("you may hatch")
  - Unconditional memory gain ("Then, gain 1 memory" — happens regardless of hatch choice)
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase


@pytest.mark.behavioral
class TestP123Ukkomon:
    """Tests for P-123 Ukkomon."""

    def _inject_digitama(self, runner, player_id, card_id):
        """Helper to inject a digi-egg into the digitama library."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        player = runner.game.player1 if player_id == 1 else runner.game.player2
        db = CardDatabase()
        cs = db.create_card_source(card_id, player)
        player.digitama_library_cards.append(cs)
        return cs

    # ── Clause 1: Trigger fires on move from breeding ──────────────────

    def test_on_move_triggers_memory_gain(self, debug_runner):
        """When a Digimon moves from breeding, Ukkomon grants +1 memory."""
        runner = debug_runner(initial_memory=3)

        # Place Ukkomon on field (it observes other Digimon moving)
        runner.place_on_field(1, ["P-123"])

        # Place a Lv.3 Digimon in breeding area to move
        runner.place_in_breeding(1, ["ST1-01", "ST1-03"])

        runner.set_phase("Breeding")
        memory_before = runner.game.memory

        action = runner.find_action("Move")
        assert action is not None, "Should find Move action in Breeding phase"

        runner.execute(action)
        runner.auto_resolve()

        # Memory should increase by 1 from the Ukkomon effect
        assert runner.game.memory == memory_before + 1, (
            f"Memory should increase by exactly 1 from Ukkomon effect, "
            f"was {memory_before}, now {runner.game.memory}"
        )

    # ── Clause 2: Hatch is optional ────────────────────────────────────

    def test_hatch_offered_when_eggs_available(self, debug_runner):
        """When hatching is possible, a hatch choice should be offered."""
        runner = debug_runner(initial_memory=3)

        runner.place_on_field(1, ["P-123"])
        runner.place_in_breeding(1, ["ST1-01", "ST1-03"])
        # Add a digi-egg to digitama library so hatching is possible
        self._inject_digitama(runner, 1, "BT1-001")

        runner.set_phase("Breeding")

        action = runner.find_action("Move")
        assert action is not None

        runner.execute(action)

        # After the move, game should be in SelectEffectChoice phase
        # (the hatch choice from effect_choose_branch)
        assert runner.game.current_phase == GamePhase.SelectEffectChoice, (
            f"Expected SelectEffectChoice phase for hatch prompt, "
            f"got {runner.game.current_phase}"
        )

        # Auto-resolve picks first option ("Yes, hatch")
        runner.auto_resolve()

        # Verify hatch occurred — breeding area should now have a digi-egg
        snap = runner.snapshot()
        assert snap.p1_breeding is not None, (
            "After accepting hatch, breeding area should have a new digi-egg"
        )

    def test_decline_hatch_still_gains_memory(self, debug_runner):
        """Declining hatch should still gain 1 memory ('Then' is unconditional)."""
        runner = debug_runner(initial_memory=3)

        runner.place_on_field(1, ["P-123"])
        runner.place_in_breeding(1, ["ST1-01", "ST1-03"])
        self._inject_digitama(runner, 1, "BT1-001")

        runner.set_phase("Breeding")
        memory_before = runner.game.memory

        action = runner.find_action("Move")
        assert action is not None
        runner.execute(action)

        # Game should be in SelectEffectChoice for hatch prompt
        assert runner.game.current_phase == GamePhase.SelectEffectChoice

        # Pick second option ("No, skip") — action 1001
        from engine_py_legacy.engine.game.constants import SEL_EFFECT_CHOICE_START
        decline_action = SEL_EFFECT_CHOICE_START + 1
        runner.execute(decline_action)
        runner.auto_resolve()

        # Memory should still increase by 1 even though hatch was declined
        assert runner.game.memory == memory_before + 1, (
            f"Memory should increase by 1 even when hatch is declined, "
            f"was {memory_before}, now {runner.game.memory}"
        )

        # Breeding area should remain empty (hatch was declined)
        snap = runner.snapshot()
        assert snap.p1_breeding is None, (
            "After declining hatch, breeding area should remain empty"
        )

    # ── Clause 3: Memory gain is unconditional ─────────────────────────

    def test_memory_gain_when_no_eggs(self, debug_runner):
        """Memory +1 happens even when no digi-eggs are available to hatch."""
        runner = debug_runner(initial_memory=3)

        runner.place_on_field(1, ["P-123"])
        runner.place_in_breeding(1, ["ST1-01", "ST1-03"])
        # No digi-eggs injected — hatch is impossible

        runner.set_phase("Breeding")
        memory_before = runner.game.memory

        action = runner.find_action("Move")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Memory should still increase by 1
        assert runner.game.memory == memory_before + 1, (
            f"Memory should increase by 1 even when hatch is not possible, "
            f"was {memory_before}, now {runner.game.memory}"
        )

    def test_memory_gain_when_breeding_occupied(self, debug_runner):
        """Memory +1 happens even if breeding area is occupied (can't hatch).

        Edge case: another effect fills breeding before Ukkomon resolves,
        or breeding is occupied by another means. Since hatch requires
        empty breeding area, it's skipped, but memory still +1.
        """
        runner = debug_runner(initial_memory=3)

        runner.place_on_field(1, ["P-123"])
        runner.place_in_breeding(1, ["ST1-01", "ST1-03"])
        self._inject_digitama(runner, 1, "BT1-001")

        # Move from breeding
        runner.set_phase("Breeding")

        action = runner.find_action("Move")
        assert action is not None
        runner.execute(action)

        # Now manually place something in breeding before resolving
        # This simulates breeding being re-occupied
        runner.place_in_breeding(1, ["ST1-01"])

        # auto_resolve should still handle the effect
        runner.auto_resolve()

        # Memory should still have increased by 1
        snap = runner.snapshot()
        # Memory started at 3, should be 4 (the +1 from Ukkomon runs before
        # the selection phase, so it already happened)
        assert snap.memory >= 4, (
            f"Memory should be at least 4 after Ukkomon +1, got {snap.memory}"
        )

    # ── Clause 4: Once Per Turn ────────────────────────────────────────

    def test_once_per_turn_second_move_no_trigger(self, debug_runner):
        """The effect is Once Per Turn — second move should not trigger it again."""
        runner = debug_runner(initial_memory=3)

        runner.place_on_field(1, ["P-123"])
        runner.place_in_breeding(1, ["ST1-01", "ST1-03"])

        runner.set_phase("Breeding")

        # First move
        action = runner.find_action("Move")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        memory_after_first = runner.game.memory
        assert memory_after_first == 4, (
            f"After first move, memory should be 4, got {memory_after_first}"
        )

        # Set up another breeding area Digimon for a second move
        runner.place_in_breeding(1, ["ST1-01", "BT1-010"])
        runner.set_phase("Breeding")

        action2 = runner.find_action("Move")
        if action2 is not None:
            runner.execute(action2)
            runner.auto_resolve()

        # Memory should NOT increase again (once per turn)
        assert runner.game.memory == memory_after_first, (
            f"Once Per Turn: memory should not increase on second move, "
            f"was {memory_after_first}, now {runner.game.memory}"
        )

    # ── Clause 5: Your Turn only ───────────────────────────────────────

    def test_not_during_opponent_turn(self, debug_runner):
        """The effect should only trigger during your turn (condition check)."""
        runner = debug_runner(initial_memory=3)

        runner.place_on_field(1, ["P-123"])

        game = runner.game
        player = game.player1
        ukkomon_card = None
        for p in player.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "P-123":
                ukkomon_card = p.top_card
                break

        assert ukkomon_card is not None
        effects = ukkomon_card.effect_list(None)
        on_move_effect = None
        for eff in effects:
            if eff.timing == EffectTiming.OnMove:
                on_move_effect = eff
                break

        assert on_move_effect is not None, "Should have OnMove effect"

        # Simulate opponent's turn
        player.is_my_turn = False
        result = on_move_effect.can_use_condition({})
        assert result is False, "OnMove effect should not activate during opponent's turn"

        # Restore
        player.is_my_turn = True
        result = on_move_effect.can_use_condition({})
        assert result is True, "OnMove effect should activate during your turn"

    # ── Clause 6: Triggers on any of YOUR Digimon moving ───────────────

    def test_does_not_trigger_on_opponent_move(self, debug_runner):
        """Ukkomon on P1's field should NOT trigger when P2 moves from breeding."""
        runner = debug_runner(initial_memory=3)

        # Ukkomon on P1's field
        runner.place_on_field(1, ["P-123"])

        # P2 has a Digimon in breeding
        runner.place_in_breeding(2, ["ST1-01", "ST1-03"])

        # Give P2 the turn
        runner.game.memory = -3  # negative memory = P2's turn
        runner.game.turn_player = runner.game.player2
        runner.game.opponent_player = runner.game.player1
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True
        runner.game.active_player = runner.game.player2

        runner.set_phase("Breeding")
        memory_before = runner.game.memory

        action = runner.find_action("Move")
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

        # P1's Ukkomon should NOT have triggered (not P1's turn)
        assert runner.game.memory == memory_before, (
            f"P1's Ukkomon should not trigger on P2's move, "
            f"memory was {memory_before}, now {runner.game.memory}"
        )

    # ── Structural validation ──────────────────────────────────────────

    def test_effect_has_correct_timing(self, debug_runner):
        """P-123 should have exactly one OnMove effect."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-123")
        effects = cs.effect_list(None)

        on_move = [e for e in effects if e.timing == EffectTiming.OnMove]
        assert len(on_move) == 1, f"Expected exactly 1 OnMove effect, got {len(on_move)}"

    def test_effect_has_once_per_turn(self, debug_runner):
        """The OnMove effect should have max_count_per_turn = 1."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-123")
        effects = cs.effect_list(None)

        on_move = [e for e in effects if e.timing == EffectTiming.OnMove]
        assert len(on_move) == 1
        assert on_move[0].max_count_per_turn == 1, (
            f"OnMove effect should be Once Per Turn (max_count_per_turn=1), "
            f"got {on_move[0].max_count_per_turn}"
        )
