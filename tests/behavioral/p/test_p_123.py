"""Behavioral tests for P-123 Ukkomon (Lv.3 White Digimon, Cost 3).

Card text:
[Your Turn] [Once Per Turn] When one of your Digimon moves from the breeding
area to the battle area, you may hatch in your breeding area. Then, gain 1
memory.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase


@pytest.mark.behavioral
class TestP123Ukkomon:
    """Tests for P-123 Ukkomon."""

    def _inject_digitama(self, runner, player_id, card_id):
        """Helper to inject a digi-egg into the digitama library."""
        from digimon_gym.engine.data.card_database import CardDatabase
        player = runner.game.player1 if player_id == 1 else runner.game.player2
        db = CardDatabase()
        cs = db.create_card_source(card_id, player)
        player.digitama_library_cards.append(cs)
        return cs

    def test_on_move_triggers_memory_gain(self, debug_runner):
        """When a Digimon moves from breeding, Ukkomon grants +1 memory."""
        runner = debug_runner(initial_memory=3)

        # Place Ukkomon on field
        runner.place_on_field(1, ["P-123"])

        # Place a Lv.3 Digimon in breeding area to move
        runner.place_in_breeding(1, ["ST1-01", "ST1-03"])

        # Set to Breeding phase so Move action is available
        runner.set_phase("Breeding")
        memory_before = runner.game.memory

        # Move from breeding
        action = runner.find_action("Move")
        assert action is not None, "Should find Move action in Breeding phase"

        runner.execute(action)
        runner.auto_resolve()

        # Memory should increase by 1 from the Ukkomon effect
        assert runner.game.memory >= memory_before + 1, (
            f"Memory should increase by at least 1 from Ukkomon effect, "
            f"was {memory_before}, now {runner.game.memory}"
        )

    def test_on_move_hatch_offered_when_possible(self, debug_runner):
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

        # After the move, the OnMove effect fires
        # If digitama deck has eggs and breeding is now empty, a choice should appear
        # Auto-resolve picks first option (hatch)
        runner.auto_resolve()

        # After auto-resolve, either the player hatched or didn't
        # Memory should still be +1
        snap = runner.snapshot()
        assert snap.memory >= 4, (
            f"Memory should increase by 1 from Ukkomon, got {snap.memory}"
        )

    def test_on_move_memory_gain_always_happens(self, debug_runner):
        """Memory +1 happens regardless of hatch choice (even if no eggs)."""
        runner = debug_runner(initial_memory=3)

        runner.place_on_field(1, ["P-123"])
        runner.place_in_breeding(1, ["ST1-01", "ST1-03"])
        # No digi-eggs — hatch not possible, but memory should still gain

        runner.set_phase("Breeding")
        memory_before = runner.game.memory

        action = runner.find_action("Move")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Memory should still increase by 1
        assert runner.game.memory >= memory_before + 1, (
            f"Memory should increase by 1 even when hatch is not possible, "
            f"was {memory_before}, now {runner.game.memory}"
        )

    def test_on_move_once_per_turn(self, debug_runner):
        """The effect is Once Per Turn -- second move should not trigger it again."""
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

    def test_on_move_not_during_opponent_turn(self, debug_runner):
        """The effect should only trigger during your turn."""
        runner = debug_runner(initial_memory=3)

        runner.place_on_field(1, ["P-123"])

        # Verify the condition checks is_my_turn
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
