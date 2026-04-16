"""Behavioral tests for BT19-093 Queen Device (Option, Yellow, Cost 3).

Card text:
  While you don't have [Queen Device] in the battle area, you may ignore
  this card's color requirements.
  When this card is trashed from the battle area, until the end of your
  opponent's turn, 1 of their Digimon can't activate [When Digivolving]
  effects and gets -3000 DP.
  [Main] Until the end of your opponent's turn, 1 of their Digimon can't
  activate [When Digivolving] effects and gets -3000 DP. Then, place this
  card in the battle area.
  [Security] Give 2 of your opponent's Digimon <Security A. -2> for the
  turn. Then, add this card to the hand.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming
from digimon_gym.engine.interfaces.modifiers import ModifierType


@pytest.mark.behavioral
class TestBT19093QueenDevice:
    """Tests for BT19-093 Queen Device."""

    # ================================================================
    # Color bypass clause
    # ================================================================

    def test_color_bypass_when_no_queen_device_on_field(self, debug_runner):
        """Should ignore color requirements when no Queen Device is in battle area."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "BT19-093", "hand")
        # Trigger effect loading (lazy)
        card.effect_list(None)
        # No Queen Device on field => bypass color requirement
        assert card.match_color_requirement is False, (
            "Should bypass color when no Queen Device in battle area")

    def test_color_bypass_disabled_when_queen_device_already_on_field(self, debug_runner):
        """Should NOT ignore color requirements when Queen Device is already in battle area."""
        runner = debug_runner(initial_memory=5)
        # Place a Queen Device on field first
        runner.place_on_field(1, ["BT19-093"])
        # Now inject another Queen Device to hand
        card2 = runner.inject_card(1, "BT19-093", "hand")
        # Trigger effect loading (lazy)
        card2.effect_list(None)
        # Queen Device already on field => must match color
        assert card2.match_color_requirement is True, (
            "Should require color match when Queen Device already in battle area")

    # ================================================================
    # [Main] effect — Effect 1 (OptionSkill)
    # ================================================================

    def test_main_effect_exists(self, debug_runner):
        """Should have an OptionSkill effect."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "BT19-093", "hand")
        effects = card.effect_list(None)
        option_skill = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(option_skill) >= 1, "Should have OptionSkill (Main) effect"

    def test_main_effect_applies_dp_reduction_via_modifier(self, debug_runner):
        """Main effect should apply -3000 DP via register_modifier (end_of_opponent_turn)."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "BT19-093", "hand")
        effects = card.effect_list(None)
        option_skill = [e for e in effects if e.timing == EffectTiming.OptionSkill][0]

        # Place opponent Digimon with known DP
        opp_perm = runner.place_on_field(2, ["BT1-025"])  # Greymon, Lv.4
        base_dp = opp_perm.dp

        ctx = {'player': runner.game.player1, 'game': runner.game}
        option_skill.on_process_callback(ctx)
        # Auto-resolve the selection (picks first legal target)
        runner.auto_resolve()

        # Check that DP was reduced via modifier system (not change_dp)
        effective = runner.game.modifiers.effective_dp(opp_perm, base_dp)
        assert effective == base_dp - 3000, (
            f"DP should be reduced by 3000 via modifier. Expected {base_dp - 3000}, got {effective}")

    def test_main_effect_applies_when_digivolving_disable(self, debug_runner):
        """Main effect should register DISABLE_EFFECT modifier for [When Digivolving]."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "BT19-093", "hand")
        effects = card.effect_list(None)
        option_skill = [e for e in effects if e.timing == EffectTiming.OptionSkill][0]

        opp_perm = runner.place_on_field(2, ["BT1-025"])

        ctx = {'player': runner.game.player1, 'game': runner.game}
        option_skill.on_process_callback(ctx)
        runner.auto_resolve()

        # Check that DISABLE_EFFECT modifier is registered targeting [When Digivolving]
        mock_wd_effect = type('MockEffect', (), {'is_when_digivolving': True})()
        has_disable = runner.game.modifiers.has_modifier(
            opp_perm, ModifierType.DISABLE_EFFECT, {'effect': mock_wd_effect})
        assert has_disable, "Should register DISABLE_EFFECT for [When Digivolving]"

    def test_main_effect_places_card_in_battle_area(self, debug_runner):
        """Main effect should keep the option in the battle area (_is_delay)."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "BT19-093", "hand")
        effects = card.effect_list(None)
        # Check that at least one effect has _is_delay = True
        has_delay = any(getattr(e, '_is_delay', False) for e in effects)
        assert has_delay, "Should have _is_delay to keep option in battle area"

    def test_main_effect_dp_modifier_expires_end_of_opponent_turn(self, debug_runner):
        """DP modifier from Main should expire at end of opponent's turn, not end of this turn."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "BT19-093", "hand")
        effects = card.effect_list(None)
        option_skill = [e for e in effects if e.timing == EffectTiming.OptionSkill][0]

        opp_perm = runner.place_on_field(2, ["BT1-025"])
        base_dp = opp_perm.dp

        ctx = {'player': runner.game.player1, 'game': runner.game}
        option_skill.on_process_callback(ctx)
        runner.auto_resolve()

        # DP should be reduced now
        effective_now = runner.game.modifiers.effective_dp(opp_perm, base_dp)
        assert effective_now == base_dp - 3000

        # Clear end_of_turn modifiers (simulates end of current player's turn)
        runner.game.modifiers.clear_expiry('end_of_turn')
        effective_after_eot = runner.game.modifiers.effective_dp(opp_perm, base_dp)
        assert effective_after_eot == base_dp - 3000, (
            "DP modifier should NOT expire at end_of_turn (should last until end of opponent's turn)")

        # Now clear end_of_opponent_turn (simulates end of opponent's turn)
        runner.game.modifiers.clear_expiry('end_of_opponent_turn')
        effective_after_opp = runner.game.modifiers.effective_dp(opp_perm, base_dp)
        assert effective_after_opp == base_dp, (
            "DP modifier should expire at end_of_opponent_turn")

    # ================================================================
    # On trashed from battle area — Effect 0 (OnDestroyedAnyone)
    # ================================================================

    def test_on_trashed_effect_exists(self, debug_runner):
        """Should have an OnDestroyedAnyone effect with is_on_deletion."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "BT19-093", "hand")
        effects = card.effect_list(None)
        destroyed = [e for e in effects
                     if e.timing == EffectTiming.OnDestroyedAnyone and e.is_on_deletion]
        assert len(destroyed) >= 1, "Should have OnDestroyedAnyone (on deletion) effect"

    def test_on_trashed_applies_dp_modifier(self, debug_runner):
        """When trashed from battle area, should apply -3000 DP via modifier."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "BT19-093", "hand")
        effects = card.effect_list(None)
        destroyed = [e for e in effects
                     if e.timing == EffectTiming.OnDestroyedAnyone and e.is_on_deletion][0]

        opp_perm = runner.place_on_field(2, ["BT1-025"])
        base_dp = opp_perm.dp

        ctx = {'player': runner.game.player1, 'game': runner.game}
        destroyed.on_process_callback(ctx)
        runner.auto_resolve()

        effective = runner.game.modifiers.effective_dp(opp_perm, base_dp)
        assert effective == base_dp - 3000, (
            f"On-trashed should apply -3000 DP modifier. Expected {base_dp - 3000}, got {effective}")

    def test_on_trashed_applies_when_digivolving_disable(self, debug_runner):
        """When trashed, should disable [When Digivolving] effects on target."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "BT19-093", "hand")
        effects = card.effect_list(None)
        destroyed = [e for e in effects
                     if e.timing == EffectTiming.OnDestroyedAnyone and e.is_on_deletion][0]

        opp_perm = runner.place_on_field(2, ["BT1-025"])

        ctx = {'player': runner.game.player1, 'game': runner.game}
        destroyed.on_process_callback(ctx)
        runner.auto_resolve()

        mock_wd_effect = type('MockEffect', (), {'is_when_digivolving': True})()
        has_disable = runner.game.modifiers.has_modifier(
            opp_perm, ModifierType.DISABLE_EFFECT, {'effect': mock_wd_effect})
        assert has_disable, "On-trashed should register DISABLE_EFFECT for [When Digivolving]"

    def test_on_trashed_dp_modifier_expires_end_of_opponent_turn(self, debug_runner):
        """On-trashed DP modifier should expire at end of opponent's turn."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "BT19-093", "hand")
        effects = card.effect_list(None)
        destroyed = [e for e in effects
                     if e.timing == EffectTiming.OnDestroyedAnyone and e.is_on_deletion][0]

        opp_perm = runner.place_on_field(2, ["BT1-025"])
        base_dp = opp_perm.dp

        ctx = {'player': runner.game.player1, 'game': runner.game}
        destroyed.on_process_callback(ctx)
        runner.auto_resolve()

        # Survives end_of_turn
        runner.game.modifiers.clear_expiry('end_of_turn')
        effective = runner.game.modifiers.effective_dp(opp_perm, base_dp)
        assert effective == base_dp - 3000, "Should survive end_of_turn"

        # Cleared at end_of_opponent_turn
        runner.game.modifiers.clear_expiry('end_of_opponent_turn')
        effective = runner.game.modifiers.effective_dp(opp_perm, base_dp)
        assert effective == base_dp, "Should expire at end_of_opponent_turn"

    # ================================================================
    # [Security] effect
    # ================================================================

    def test_security_effect_exists(self, debug_runner):
        """Should have a SecuritySkill effect."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "BT19-093", "hand")
        effects = card.effect_list(None)
        security = [e for e in effects if e.is_security_effect]
        assert len(security) >= 1, "Should have Security effect"

    def test_security_applies_sa_minus_2_via_modifier(self, debug_runner):
        """Security effect should apply SA-2 via register_modifier (end_of_turn) to 2 targets."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "BT19-093", "hand")
        effects = card.effect_list(None)
        security = [e for e in effects if e.is_security_effect][0]

        opp_perm1 = runner.place_on_field(2, ["BT1-025"])  # Greymon
        opp_perm2 = runner.place_on_field(2, ["BT1-010"])  # Agumon

        sa_before_1 = opp_perm1.security_attack_modifier()
        sa_before_2 = opp_perm2.security_attack_modifier()

        ctx = {'player': runner.game.player1, 'game': runner.game}
        security.on_process_callback(ctx)
        runner.auto_resolve()

        # Both should have exactly SA-2 (not SA-4 from double-application)
        total_sa_change = (
            (opp_perm1.security_attack_modifier() - sa_before_1) +
            (opp_perm2.security_attack_modifier() - sa_before_2)
        )
        assert total_sa_change == -4, (
            f"Total SA change across 2 Digimon should be -4 (2 * -2). Got {total_sa_change}")

    def test_security_sa_modifier_expires_end_of_turn(self, debug_runner):
        """Security SA-2 should expire at end of turn (not end_of_opponent_turn)."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "BT19-093", "hand")
        effects = card.effect_list(None)
        security = [e for e in effects if e.is_security_effect][0]

        opp_perm = runner.place_on_field(2, ["BT1-025"])
        sa_before = opp_perm.security_attack_modifier()

        ctx = {'player': runner.game.player1, 'game': runner.game}
        security.on_process_callback(ctx)
        runner.auto_resolve()

        assert opp_perm.security_attack_modifier() == sa_before - 2

        # Clear end_of_turn => SA should reset
        runner.game.modifiers.clear_expiry('end_of_turn')
        assert opp_perm.security_attack_modifier() == sa_before, (
            "SA modifier should expire at end_of_turn")

    def test_security_adds_card_to_hand(self, debug_runner):
        """Security effect should add this card to hand after applying SA-2."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "BT19-093", "trash")
        effects = card.effect_list(None)
        security = [e for e in effects if e.is_security_effect][0]

        p1 = runner.game.player1
        hand_before = len(p1.hand_cards)

        # No opponent Digimon — skip SA part, just add to hand
        ctx = {'player': p1, 'game': runner.game}
        security.on_process_callback(ctx)

        assert card in p1.hand_cards, "Card should be added to hand"
        assert card not in p1.trash_cards, "Card should no longer be in trash"
