"""Behavioral tests for EX4-006 Guilmon (Lv.3 Red Reptile, DP 2000, Cost 3).

Card text:
  Digivolve: from [Gigimon] for 0 cost
  [On Play] If the total number of cards in both players' trashes is 20 or more,
  this Digimon gains <Rush> for the turn.

C# reference:
  - Alt digi: from Gigimon, cost 0
  - On Play: conditional Rush (trash >= 20)
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestEX4006Guilmon:
    """Tests for EX4-006 Guilmon."""

    def test_has_alt_digi_from_gigimon(self, debug_runner):
        """Should have alt digi from Gigimon for cost 0."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-006"])

        card = perm.top_card
        effects = card.effect_list(EffectTiming.NoTiming)
        alt_digi = [e for e in effects if getattr(e, '_alt_digi_cost', None) is not None]
        assert len(alt_digi) >= 1, "Should have alt digi effect"
        found = any(
            getattr(e, '_alt_digi_name', None) == "Gigimon"
            and getattr(e, '_alt_digi_cost', None) == 0
            for e in alt_digi
        )
        assert found, "Should have alt digi: from Gigimon for cost 0"

    def test_on_play_grants_rush_when_trash_ge_20(self, debug_runner):
        """[On Play] Should grant Rush when both players' trashes total >= 20 cards."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-006"])
        game = runner.game

        # Fill trashes to total exactly 20
        for _ in range(10):
            runner.inject_card(1, "ST1-03", "trash")
            runner.inject_card(2, "ST1-03", "trash")

        total_trash = len(game.player1.trash_cards) + len(game.player2.trash_cards)
        assert total_trash >= 20

        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects
                   if e.timing == EffectTiming.OnEnterFieldAnyone and e.is_on_play]
        assert len(on_play) == 1

        ctx = {'player': game.player1, 'game': game, 'permanent': perm}
        on_play[0].on_process_callback(ctx)

        assert perm.has_keyword('_is_rush'), "Should have Rush when trash >= 20"

    def test_on_play_no_rush_when_trash_lt_20(self, debug_runner):
        """[On Play] Should NOT grant Rush when total trash < 20."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-006"])
        game = runner.game

        # Only put 5 cards in each trash = 10 total
        for _ in range(5):
            runner.inject_card(1, "ST1-03", "trash")
            runner.inject_card(2, "ST1-03", "trash")

        total_trash = len(game.player1.trash_cards) + len(game.player2.trash_cards)
        assert total_trash < 20

        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects
                   if e.timing == EffectTiming.OnEnterFieldAnyone and e.is_on_play]
        assert len(on_play) == 1

        ctx = {'player': game.player1, 'game': game, 'permanent': perm}
        on_play[0].on_process_callback(ctx)

        assert not perm.has_keyword('_is_rush'), "Should NOT have Rush when trash < 20"

    def test_on_play_rush_expires_after_turn(self, debug_runner):
        """[On Play] Rush granted 'for the turn' should expire when the next turn starts."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-006"])
        game = runner.game

        # Fill trashes to total >= 20
        for _ in range(10):
            runner.inject_card(1, "ST1-03", "trash")
            runner.inject_card(2, "ST1-03", "trash")

        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects
                   if e.timing == EffectTiming.OnEnterFieldAnyone and e.is_on_play]
        assert len(on_play) == 1

        ctx = {'player': game.player1, 'game': game, 'permanent': perm}
        on_play[0].on_process_callback(ctx)

        # Rush should be active during this turn
        assert perm.has_keyword('_is_rush'), "Should have Rush during the current turn"

        # Simulate turn change: increment turn_count and clear expired grants
        game.turn_count += 1
        perm.clear_expired_grants(game.turn_count)

        # Rush should have expired
        assert not perm.has_keyword('_is_rush'), \
            "Rush should expire after the turn (duration=turn_count)"

    def test_on_play_condition_requires_field_presence(self, debug_runner):
        """Condition should require the card to be on the field."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Create card in hand (not on field)
        runner.inject_card(1, "EX4-006", "hand")
        hand_card = game.player1.hand_cards[-1]

        effects = hand_card.effect_list(None)
        on_play = [e for e in effects
                   if e.timing == EffectTiming.OnEnterFieldAnyone and e.is_on_play]
        if on_play:
            # Card in hand -> permanent_of_this_card() is None -> condition fails
            assert not on_play[0].can_use_condition({}), \
                "Condition should fail when card is not on field"
