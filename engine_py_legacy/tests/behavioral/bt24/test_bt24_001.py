"""Behavioral tests for BT24-001 Gigimon (Lv.2 Digitama).

Card text:
Inherited Effect [Your Turn] [Once Per Turn] When your opponent's security stack
is removed from, you may delete 1 of their Digimon with 3000 DP or less.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase
from digimon_gym.engine.game.constants import SEL_OPP_FIELD_START


@pytest.mark.behavioral
class TestBT24001Gigimon:
    """Tests for BT24-001 Gigimon inherited effect."""

    def _get_inherited_effect(self, perm):
        """Get the OnLoseSecurity inherited effect from Gigimon (bottom of stack)."""
        card = perm.card_sources[0]  # Gigimon is bottom of digivolution stack
        effects = card.effect_list(None)
        ols_effects = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity]
        assert len(ols_effects) == 1, "Gigimon should have exactly 1 OnLoseSecurity inherited effect"
        return ols_effects[0]

    def test_effect_metadata(self, debug_runner):
        """Verify effect metadata: inherited, optional, once per turn."""
        runner = debug_runner(initial_memory=5)
        # Stack Gigimon (Lv.2) under Elizamon (Lv.3 Red, 2000 DP)
        perm = runner.place_on_field(1, ["BT24-001", "BT24-008"])
        effect = self._get_inherited_effect(perm)

        assert effect.is_inherited_effect, "Effect must be inherited"
        assert effect.is_optional, "Effect must be optional"
        assert effect.max_count_per_turn == 1, "Effect must be once per turn"

    def test_condition_passes_opponent_loses_security_on_your_turn(self, debug_runner):
        """Condition should pass when opponent loses security on your turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT24-001", "BT24-008"])
        game = runner.game

        effect = self._get_inherited_effect(perm)

        # Context simulating opponent (player2) losing security on player1's turn
        context = {
            'game': game,
            'player': game.player1,       # effect owner
            'permanent': perm,
            'event_player': game.player2,  # opponent lost security
        }
        assert effect.can_use_condition(context), \
            "Condition should pass when opponent loses security on your turn"

    def test_condition_fails_own_security_loss(self, debug_runner):
        """Condition should FAIL when the card owner's own security is removed.

        This is the critical bug: the effect says 'opponent's security stack',
        so losing your own security should NOT trigger it.
        """
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT24-001", "BT24-008"])
        game = runner.game

        effect = self._get_inherited_effect(perm)

        # Context simulating owner (player1) losing their own security
        context = {
            'game': game,
            'player': game.player1,       # effect owner
            'permanent': perm,
            'event_player': game.player1,  # owner lost security (NOT opponent)
        }
        assert not effect.can_use_condition(context), \
            "Condition must FAIL when owner's own security is removed"

    def test_condition_fails_on_opponent_turn(self, debug_runner):
        """Condition should fail when it's not your turn (opponent's turn)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT24-001", "BT24-008"])
        game = runner.game

        effect = self._get_inherited_effect(perm)

        # Switch to opponent's turn
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        context = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'event_player': game.player2,
        }
        assert not effect.can_use_condition(context), \
            "Condition should fail on opponent's turn"

    def test_condition_fails_not_on_field(self, debug_runner):
        """Condition should fail when the card is not on field (permanent is None)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT24-001", "BT24-008"])
        game = runner.game

        effect = self._get_inherited_effect(perm)

        # Remove from field to simulate not being on the field
        game.player1.battle_area.remove(perm)

        context = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'event_player': game.player2,
        }
        assert not effect.can_use_condition(context), \
            "Condition should fail when card is not on field"

    def test_delete_opponent_digimon_3000dp_or_less(self, debug_runner):
        """Process callback should create selection phase targeting
        opponent's Digimon with 3000 DP or less, and resolving deletes it."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT24-001", "BT24-008"])
        game = runner.game

        # Place a real Digimon (BT24-008 Elizamon, 2000 DP) on opponent's field
        target = runner.place_on_field(2, ["BT24-008"], is_suspended=False)
        assert target.dp == 2000, f"Target should have 2000 DP, got {target.dp}"

        effect = self._get_inherited_effect(perm)

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Should have created a pending selection for opponent's field
        ps = game.pending_selection
        assert ps is not None, "Should create a pending selection for delete target"
        assert game.current_phase == GamePhase.SelectTarget

        # The valid indices should include the opponent's first field slot
        assert SEL_OPP_FIELD_START in ps.valid_indices, \
            f"Target at index {SEL_OPP_FIELD_START} should be valid, got {ps.valid_indices}"

        # Resolve the selection
        p2_field_before = len(game.player2.battle_area)
        ps.callback(SEL_OPP_FIELD_START)

        # Opponent should have lost a Digimon
        assert len(game.player2.battle_area) < p2_field_before, \
            "Opponent's Digimon with <=3000 DP should be deleted"

    def test_no_delete_digimon_above_3000dp(self, debug_runner):
        """Process callback should NOT offer target above 3000 DP."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT24-001", "BT24-008"])
        game = runner.game

        # Place a Digimon on opponent's field and force its DP to 4000
        target = runner.place_on_field(2, ["BT24-008"], is_suspended=False)
        target.card_sources[0].base_dp = 4000

        assert target.dp == 4000, f"Target should have 4000 DP, got {target.dp}"

        effect = self._get_inherited_effect(perm)

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # No valid target (>3000 DP) -- no selection phase, nothing deleted
        ps = game.pending_selection
        assert ps is None, \
            "No selection should be created when no valid target exists"
        assert len(game.player2.battle_area) == 1, \
            "Opponent's Digimon with >3000 DP should NOT be deleted"

    def test_integration_opponent_security_removal_triggers(self, debug_runner):
        """Integration: removing opponent's security fires the effect via execute_effects."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT24-001", "BT24-008"])
        game = runner.game

        # Place a 2000 DP Digimon on opponent's field as delete target
        target = runner.place_on_field(2, ["BT24-008"], is_suspended=False)
        assert target.dp == 2000

        # Give opponent a security card to remove
        sec_card = runner.inject_card(2, "BT24-008", zone="security_top")

        p2_sec_before = len(game.player2.security_cards)

        # Trash opponent's security -- fires OnLoseSecurity with player=player2
        game.player2.trash_security_card(sec_card)

        assert len(game.player2.security_cards) == p2_sec_before - 1, \
            "Security card should be removed"

        # The inherited effect should have triggered and created a pending selection
        ps = game.pending_selection
        assert ps is not None, \
            "OnLoseSecurity should trigger Gigimon inherited effect selection"
        assert game.current_phase == GamePhase.SelectTarget

        # Resolve the selection to delete the 2000 DP Digimon
        p2_field_before = len(game.player2.battle_area)
        ps.callback(SEL_OPP_FIELD_START)

        assert len(game.player2.battle_area) < p2_field_before, \
            "Opponent's low-DP Digimon should be deleted by inherited effect"
