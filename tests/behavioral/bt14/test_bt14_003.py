"""Behavioral tests for BT14-003 Tokomon (Lv.2 Digi-Egg, Yellow).

Card text:
Inherited Effect [Your Turn] [Once Per Turn] When a card is added to your
  security stack, <Draw 1> (Draw 1 card from your deck.)

Key behaviors:
- Inherited effect only (active under a Digimon)
- [Your Turn] restriction
- [Once Per Turn] OPT
- Triggers on OWN security stack addition (NOT opponent's)
- Draw 1

C# reference: DCGO/Assets/Scripts/CardEffect/BT14/Yellow/BT14_003.cs
  CanTriggerWhenAddSecurity(hashtable, player => player == card.Owner)
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT14003Tokomon:
    """Tests for BT14-003 Tokomon inherited effect."""

    def test_inherited_effect_exists(self, debug_runner):
        """BT14-003 should have exactly 1 inherited OnAddSecurity effect."""
        runner = debug_runner(initial_memory=5)
        # Yellow stack: Tokomon (egg) + Patamon (yellow Lv3)
        perm = runner.place_on_field(1, ["BT14-003", "BT1-048"])

        card = perm.card_sources[0]  # Tokomon (bottom of stack)
        effects = card.effect_list(None)
        oas_effects = [e for e in effects if e.timing == EffectTiming.OnAddSecurity]
        assert len(oas_effects) == 1, "Should have exactly 1 OnAddSecurity effect"

        oas = oas_effects[0]
        assert oas.is_inherited_effect, "Effect must be inherited"
        # OPT
        assert oas.max_count_per_turn == 1, "Should be once per turn"

    def test_draw_on_own_security_add(self, debug_runner):
        """When YOU add to your own security on your turn, draw 1."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT14-003", "BT1-048"])

        game = runner.game
        # Ensure it's player 1's turn
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False

        hand_before = len(game.player1.hand_cards)

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        oas = [e for e in effects if e.timing == EffectTiming.OnAddSecurity][0]

        # Condition context: player1 (the card's owner) is adding security
        condition_ctx = {
            'game': game,
            'player': game.player1,  # owner of the permanent
            'permanent': perm,
            'event_player': game.player1,  # player adding to their OWN security
        }

        assert oas.can_use_condition(condition_ctx), \
            "Condition should pass when own security is added on your turn"

        # Execute the draw
        oas.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert len(game.player1.hand_cards) == hand_before + 1, \
            "Should draw 1 card"

    def test_no_draw_on_opponent_security_add(self, debug_runner):
        """When OPPONENT adds to THEIR security, this should NOT trigger."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT14-003", "BT1-048"])

        game = runner.game
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        oas = [e for e in effects if e.timing == EffectTiming.OnAddSecurity][0]

        # Context where player2 (the opponent) is adding to their security
        condition_ctx = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'event_player': game.player2,  # OPPONENT adding security
        }

        assert not oas.can_use_condition(condition_ctx), \
            "Condition should FAIL when opponent adds to their own security"

    def test_no_draw_on_opponent_turn(self, debug_runner):
        """Should NOT trigger on opponent's turn ([Your Turn] restriction)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT14-003", "BT1-048"])

        game = runner.game
        # Set it to player 2's turn (opponent's turn for card owner player1)
        game.turn_player = game.player2
        game.opponent_player = game.player1
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        oas = [e for e in effects if e.timing == EffectTiming.OnAddSecurity][0]

        # Even if player1 somehow adds their own security on opponent's turn
        condition_ctx = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'event_player': game.player1,
        }

        assert not oas.can_use_condition(condition_ctx), \
            "Condition should FAIL on opponent's turn"

    def test_once_per_turn(self, debug_runner):
        """Effect should only trigger once per turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT14-003", "BT1-048"])

        game = runner.game
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        oas = [e for e in effects if e.timing == EffectTiming.OnAddSecurity][0]

        # First activation should succeed
        assert oas.can_activate_this_turn(), "First activation should be allowed"

        # Record activation
        oas.record_activation()

        # Second activation should be blocked (OPT)
        assert not oas.can_activate_this_turn(), \
            "Second activation should be blocked by OPT"

    def test_recovery_triggers_draw_end_to_end(self, debug_runner):
        """Integration: Player.recovery() fires OnAddSecurity and the egg draws 1.

        This exercises the full _fire_timing -> execute_effects -> condition ->
        process pipeline with the real engine event dispatch.
        """
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT14-003", "BT1-048"])

        game = runner.game
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False

        # Ensure there is a deck card to recover (added to security) AND a
        # deck card to draw from.
        # place_on_field consumes cards; top up the library so recovery/draw work.
        # We'll add dummy cards by giving the player extra library cards via inject.
        # Use an existing yellow card id as filler.
        filler_ids = ["BT1-048", "BT1-048", "BT1-048"]  # dupes are fine for filler
        for fid in filler_ids:
            runner.inject_card(1, fid, zone="library_bottom")

        hand_before = len(game.player1.hand_cards)
        sec_before = len(game.player1.security_cards)
        lib_before = len(game.player1.library_cards)

        # Recovery adds top of library to security stack and fires OnAddSecurity.
        game.player1.recovery(1)

        assert len(game.player1.security_cards) == sec_before + 1, \
            "Recovery should add 1 card to security"
        # Engine should draw 1 for the Tokomon inherited effect.
        assert len(game.player1.hand_cards) == hand_before + 1, \
            f"Should draw 1 card from Tokomon inherited effect (hand: {hand_before} -> {len(game.player1.hand_cards)})"
        # Library: -1 for recovery, -1 for the triggered draw
        assert len(game.player1.library_cards) == lib_before - 2, \
            "Library should decrease by 2 (1 recovery + 1 draw)"

    def test_opponent_recovery_does_not_trigger(self, debug_runner):
        """Integration: if opponent recovers, Tokomon must NOT draw."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT14-003", "BT1-048"])

        game = runner.game
        # It's player 1's turn (card owner's turn).
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False

        # Give player2 extra library cards so recovery can succeed.
        for fid in ["BT1-048", "BT1-048"]:
            runner.inject_card(2, fid, zone="library_bottom")

        hand_before_p1 = len(game.player1.hand_cards)

        # Opponent adds a card to their own security.
        game.player2.recovery(1)

        # Tokomon belongs to player1; opponent adding their own security
        # must NOT trigger the draw.
        assert len(game.player1.hand_cards) == hand_before_p1, \
            "Player1 should NOT draw when opponent adds to their own security"
