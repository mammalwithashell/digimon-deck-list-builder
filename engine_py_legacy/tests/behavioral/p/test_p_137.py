"""Behavioral tests for P-137 Flamedramon (Lv.4 Red/Blue Dragon Man, DP 5000, Cost 5).

Card text:
  Digivolve: from [Veemon] for 2 cost
  <Armor Purge>
  <Raid>
  [Your Turn][Once Per Turn] When this Digimon's attack target is switched,
  your opponent adds the top card of their security stack to the hand.

C# reference:
  - Alt digi from Veemon cost 2
  - Armor Purge, Raid keywords
  - OnAttackTargetChanged: opponent adds TOP security (index 0) to hand
  - NOTE: C# uses SecurityCards[0] = top of security
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestP137Flamedramon:
    """Tests for P-137 Flamedramon."""

    def test_has_alt_digi_from_veemon(self, debug_runner):
        """Should have alt digi from Veemon for cost 2."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["P-137"])

        card = perm.top_card
        effects = card.effect_list(EffectTiming.NoTiming)
        alt_digi = [e for e in effects if getattr(e, '_alt_digi_cost', None) is not None]
        assert len(alt_digi) >= 1
        found = any(
            getattr(e, '_alt_digi_name', None) == "Veemon"
            and getattr(e, '_alt_digi_cost', None) == 2
            for e in alt_digi
        )
        assert found, "Should have alt digi: from Veemon for cost 2"

    def test_has_armor_purge(self, debug_runner):
        """Should have <Armor Purge>."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["P-137"])

        card = perm.top_card
        effects = card.effect_list(None)
        ap = [e for e in effects if getattr(e, '_is_armor_purge', False)]
        assert len(ap) == 1, "Should have Armor Purge"

    def test_has_raid(self, debug_runner):
        """Should have <Raid>."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["P-137"])

        card = perm.top_card
        effects = card.effect_list(None)
        raid = [e for e in effects if getattr(e, '_is_raid', False)]
        assert len(raid) == 1, "Should have Raid"

    def test_attack_target_switch_effect_structure(self, debug_runner):
        """Should have OnAttackTargetChanged effect, once per turn, your turn only."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["P-137"])

        card = perm.top_card
        effects = card.effect_list(None)
        target_switch = [e for e in effects if e.timing == EffectTiming.OnAttackTargetChanged]
        assert len(target_switch) == 1
        eff = target_switch[0]
        assert eff.max_count_per_turn == 1, "Should be once per turn"

    def test_attack_target_switch_your_turn_only(self, debug_runner):
        """Effect should only activate on your turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["P-137"])
        game = runner.game

        card = perm.top_card
        effects = card.effect_list(None)
        eff = [e for e in effects if e.timing == EffectTiming.OnAttackTargetChanged][0]

        # P1's turn
        assert eff.can_use_condition({}), "Should pass on owner's turn"

        # Switch to opponent's turn
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True
        assert not eff.can_use_condition({}), "Should fail on opponent's turn"

    def test_attack_target_switch_adds_top_security_to_hand(self, debug_runner):
        """When triggered, opponent adds TOP security (index 0) to their hand."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["P-137"])
        game = runner.game

        # Give opponent security cards (security_top inserts at index 0)
        # Insert ST1-07 first, then ST1-03 on top
        runner.inject_card(2, "ST1-07", "security_top")
        runner.inject_card(2, "ST1-03", "security_top")

        # Security is [ST1-03, ST1-07], top = index 0 = ST1-03
        top_sec_id = game.player2.security_cards[0].c_entity_base.card_id
        opp_hand_before = len(game.player2.hand_cards)
        sec_before = len(game.player2.security_cards)

        card = perm.top_card
        effects = card.effect_list(None)
        eff = [e for e in effects if e.timing == EffectTiming.OnAttackTargetChanged][0]

        eff.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Opponent's hand should have +1 card
        assert len(game.player2.hand_cards) == opp_hand_before + 1, \
            "Opponent should have 1 more card in hand"
        # Security should have -1 card
        assert len(game.player2.security_cards) == sec_before - 1, \
            "Opponent should have 1 less security card"
        # The added card should be the TOP security card (index 0)
        added_card = game.player2.hand_cards[-1]
        assert added_card.c_entity_base.card_id == top_sec_id, \
            f"Should add TOP security card ({top_sec_id}), not bottom"

    def test_attack_target_switch_no_security_noop(self, debug_runner):
        """When opponent has no security, the effect should be a no-op."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["P-137"])
        game = runner.game

        # Clear opponent security
        runner.clear_zone(2, "security")

        opp_hand_before = len(game.player2.hand_cards)

        card = perm.top_card
        effects = card.effect_list(None)
        eff = [e for e in effects if e.timing == EffectTiming.OnAttackTargetChanged][0]

        eff.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert len(game.player2.hand_cards) == opp_hand_before, \
            "No cards should be added when security is empty"
