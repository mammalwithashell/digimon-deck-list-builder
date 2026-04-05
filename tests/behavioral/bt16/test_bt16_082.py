"""Behavioral tests for BT16-082 Ukkomon (Lv.3 Yellow Unidentified, DP 2000, Cost 3).

Card text:
  [Your Turn][Once Per Turn] When one of your Digimon moves from the breeding
  area to the battle area, reveal the top 3 cards of your deck. Add 1 Digimon
  card or Tamer card among them to the hand. Return the rest to the bottom of
  the deck. Then, you may hatch in your breeding area.

C# reference:
  - OnMove timing with Once Per Turn
  - PermanentCondition: IsPermanentExistsOnOwnerBattleAreaDigimon
  - RevealDeckTopCardsAndSelect(3, select Digimon/Tamer, rest to deck bottom)
  - Optional hatch if CanHatch
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT16082Ukkomon:
    """Tests for BT16-082 Ukkomon."""

    def test_has_on_move_effect(self, debug_runner):
        """Should have OnMove timing effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-082"])

        card = perm.top_card
        effects = card.effect_list(None)
        on_move = [e for e in effects if e.timing == EffectTiming.OnMove]
        assert len(on_move) == 1, "Should have exactly 1 OnMove effect"

    def test_on_move_is_optional(self, debug_runner):
        """Effect should be optional."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-082"])

        card = perm.top_card
        effects = card.effect_list(None)
        on_move = [e for e in effects if e.timing == EffectTiming.OnMove][0]
        assert on_move.is_optional, "OnMove effect should be optional"

    def test_on_move_once_per_turn(self, debug_runner):
        """Effect should be once per turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-082"])

        card = perm.top_card
        effects = card.effect_list(None)
        on_move = [e for e in effects if e.timing == EffectTiming.OnMove][0]
        assert on_move.max_count_per_turn == 1, "Should be once per turn"

    def test_condition_requires_your_turn(self, debug_runner):
        """Condition should only pass on owner's turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-082"])
        game = runner.game

        card = perm.top_card
        effects = card.effect_list(None)
        on_move = [e for e in effects if e.timing == EffectTiming.OnMove][0]

        # P1's turn - should pass with valid context
        event_perm = runner.place_on_field(1, ["ST1-03"])  # a Digimon on P1 field
        ctx = {
            'event_permanent': event_perm,
            'event_player': game.player1,
        }
        assert on_move.can_use_condition(ctx), "Should pass on owner's turn"

        # Switch to P2's turn
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True
        assert not on_move.can_use_condition(ctx), "Should fail on opponent's turn"

    def test_condition_requires_own_digimon_move(self, debug_runner):
        """Condition should only trigger when owner's Digimon moves, not opponent's."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-082"])
        game = runner.game

        card = perm.top_card
        effects = card.effect_list(None)
        on_move = [e for e in effects if e.timing == EffectTiming.OnMove][0]

        # Opponent's Digimon move - should NOT trigger
        opp_perm = runner.place_on_field(2, ["ST1-03"])
        ctx = {
            'event_permanent': opp_perm,
            'event_player': game.player2,
        }
        assert not on_move.can_use_condition(ctx), \
            "Should NOT trigger when opponent's Digimon moves"

    def test_condition_requires_digimon_not_tamer(self, debug_runner):
        """Condition should only trigger for Digimon moves, not Tamer moves."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-082"])
        game = runner.game

        card = perm.top_card
        effects = card.effect_list(None)
        on_move = [e for e in effects if e.timing == EffectTiming.OnMove][0]

        # Use a Tamer as the moved permanent
        tamer_perm = runner.place_on_field(1, ["ST1-12"])  # A tamer
        ctx = {
            'event_permanent': tamer_perm,
            'event_player': game.player1,
        }
        # Tamer should not match (is_digimon check)
        if tamer_perm.is_tamer and not tamer_perm.is_digimon:
            assert not on_move.can_use_condition(ctx), \
                "Should NOT trigger for Tamer moves"
