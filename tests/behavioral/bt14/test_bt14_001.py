"""Behavioral tests for BT14-001 Koromon (Lv.2 Digi-Egg, Red).

Card text:
Inherited Effect [Your Turn] [Once Per Turn] When a card is removed from
  your opponent's security stack, <Draw 1>.

Key behaviors:
- Inherited effect only (active under a Digimon)
- [Your Turn] restriction
- [Once Per Turn] OPT
- Triggers on opponent's security removal (not own security)
- Draw 1
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT14001Koromon:
    """Tests for BT14-001 Koromon inherited effect."""

    def test_inherited_effect_exists(self, debug_runner):
        """BT14-001 should have exactly 1 inherited OnLoseSecurity effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT14-001", "BT1-010"])  # Koromon + Agumon

        card = perm.card_sources[0]  # Koromon (bottom of stack)
        effects = card.effect_list(None)
        ols_effects = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity]
        assert len(ols_effects) == 1, "Should have exactly 1 OnLoseSecurity effect"

        ols = ols_effects[0]
        assert ols.is_inherited_effect, "Effect must be inherited"

    def test_draw_on_opponent_security_loss(self, debug_runner):
        """When opponent loses security on your turn, draw 1."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT14-001", "BT1-010"])

        game = runner.game
        # Ensure it's player 1's turn
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False

        hand_before = len(game.player1.hand_cards)

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        ols = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity][0]

        # Simulate condition context where the OPPONENT (player2) is losing security
        condition_ctx = {
            'game': game,
            'player': game.player1,  # owner of the permanent
            'permanent': perm,
            'event_player': game.player2,  # player losing security
        }

        assert ols.can_use_condition(condition_ctx), \
            "Condition should pass when opponent loses security on your turn"

        # Execute the draw
        ols.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert len(game.player1.hand_cards) == hand_before + 1, \
            "Should draw 1 card"

    def test_no_draw_on_own_security_loss(self, debug_runner):
        """When YOU lose security, this should NOT trigger."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT14-001", "BT1-010"])

        game = runner.game
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        ols = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity][0]

        # Context where player1 (the card's owner) is losing security
        condition_ctx = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'event_player': game.player1,  # OWN security loss
        }

        assert not ols.can_use_condition(condition_ctx), \
            "Condition should FAIL when own security is lost"

    def test_no_draw_on_opponent_turn(self, debug_runner):
        """Should NOT trigger on opponent's turn ([Your Turn] restriction)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT14-001", "BT1-010"])

        game = runner.game
        # Set it to player 2's turn (opponent's turn for card owner player1)
        game.turn_player = game.player2
        game.opponent_player = game.player1
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        ols = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity][0]

        condition_ctx = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'event_player': game.player2,  # opponent loses security
        }

        assert not ols.can_use_condition(condition_ctx), \
            "Condition should FAIL on opponent's turn"

    def test_once_per_turn(self, debug_runner):
        """Effect should only trigger once per turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT14-001", "BT1-010"])

        game = runner.game
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        ols = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity][0]

        # First activation should succeed
        assert ols.can_activate_this_turn(), "First activation should be allowed"

        # Record activation
        ols.record_activation()

        # Second activation should be blocked (OPT)
        assert not ols.can_activate_this_turn(), \
            "Second activation should be blocked by OPT"
