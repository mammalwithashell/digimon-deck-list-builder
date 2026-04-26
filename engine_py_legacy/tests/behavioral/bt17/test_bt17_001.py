"""Behavioral tests for BT17-001 Gigimon (Lv.2 Red Digi-Egg).

Card text:
Inherited [When Attacking] By paying 1 cost, delete 1 of your opponent's
Digimon with 3000 DP or less.
"""

import pytest
from engine_py_legacy.engine.data.enums import GamePhase
from engine_py_legacy.engine.game.constants import SEL_OPP_FIELD_START


@pytest.mark.behavioral
class TestBT17001Gigimon:
    """Tests for BT17-001 Gigimon inherited effect."""

    def _get_inherited_effect(self, perm):
        """Get the [When Attacking] inherited effect from Gigimon (bottom of stack)."""
        card = perm.card_sources[0]  # Gigimon is bottom of digivolution stack
        effects = card.effect_list(None)
        on_attack_effects = [e for e in effects if e.is_on_attack and e.is_inherited_effect]
        assert len(on_attack_effects) == 1, (
            "Gigimon should have exactly 1 inherited [When Attacking] effect"
        )
        return on_attack_effects[0]

    # ── Effect metadata ──────────────────────────────────────────────

    def test_effect_is_inherited(self, debug_runner):
        """Effect must be flagged as inherited (only active as digivolution source)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT17-001", "ST1-03"])
        effect = self._get_inherited_effect(perm)
        assert effect.is_inherited_effect, "Effect must be inherited"

    def test_effect_is_on_attack(self, debug_runner):
        """Effect must be flagged as [When Attacking]."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT17-001", "ST1-03"])
        effect = self._get_inherited_effect(perm)
        assert effect.is_on_attack, "Effect must trigger [When Attacking]"

    def test_effect_is_optional(self, debug_runner):
        """'By paying 1 cost' implies the player can choose not to pay, so optional."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT17-001", "ST1-03"])
        effect = self._get_inherited_effect(perm)
        assert effect.is_optional, "Effect must be optional (pay cost is a choice)"

    # ── Condition checks ─────────────────────────────────────────────

    def test_condition_passes_with_valid_target(self, debug_runner):
        """Condition should pass when the permanent is attacking and opponent
        has a Digimon with 3000 DP or less."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT17-001", "ST1-03"])
        # Place a low-DP Digimon on opponent's field
        runner.place_on_field(2, ["ST1-03"])  # Agumon, 2000 DP

        effect = self._get_inherited_effect(perm)

        context = {
            'attacking_permanent': perm,
            'permanent': perm,
        }
        assert effect.can_use_condition(context), (
            "Condition should pass: attacking permanent matches, "
            "opponent has Digimon with <= 3000 DP"
        )

    def test_condition_fails_no_valid_target(self, debug_runner):
        """Condition should fail when opponent has no Digimon with 3000 DP or less."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT17-001", "ST1-03"])
        # Place a high-DP Digimon on opponent's field
        high_dp = runner.place_on_field(2, ["ST1-03"])
        high_dp.card_sources[0].base_dp = 5000

        effect = self._get_inherited_effect(perm)

        context = {
            'attacking_permanent': perm,
            'permanent': perm,
        }
        assert not effect.can_use_condition(context), (
            "Condition should fail: no opponent Digimon with <= 3000 DP"
        )

    def test_condition_fails_empty_opponent_field(self, debug_runner):
        """Condition should fail when opponent has no Digimon at all."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT17-001", "ST1-03"])
        # Opponent field is empty
        runner.clear_zone(2, "field")

        effect = self._get_inherited_effect(perm)

        context = {
            'attacking_permanent': perm,
            'permanent': perm,
        }
        assert not effect.can_use_condition(context), (
            "Condition should fail: opponent has no Digimon"
        )

    def test_condition_fails_different_attacking_permanent(self, debug_runner):
        """Condition should fail if a different permanent is attacking."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT17-001", "ST1-03"])
        other_perm = runner.place_on_field(1, ["ST1-03"])
        runner.place_on_field(2, ["ST1-03"])  # valid target

        effect = self._get_inherited_effect(perm)

        context = {
            'attacking_permanent': other_perm,  # different permanent is attacking
            'permanent': other_perm,
        }
        assert not effect.can_use_condition(context), (
            "Condition should fail: a different permanent is attacking"
        )

    def test_condition_fails_not_on_field(self, debug_runner):
        """Condition should fail when card is not on field (permanent_of_this_card returns None)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT17-001", "ST1-03"])
        game = runner.game

        effect = self._get_inherited_effect(perm)

        # Remove from field
        game.player1.battle_area.remove(perm)

        context = {
            'attacking_permanent': perm,
            'permanent': perm,
        }
        assert not effect.can_use_condition(context), (
            "Condition should fail: card is not on field"
        )

    # ── DP threshold boundary ────────────────────────────────────────

    def test_condition_passes_exactly_3000dp(self, debug_runner):
        """Boundary: opponent Digimon with exactly 3000 DP should be a valid target."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT17-001", "ST1-03"])
        target = runner.place_on_field(2, ["ST1-03"])
        target.card_sources[0].base_dp = 3000

        effect = self._get_inherited_effect(perm)

        context = {
            'attacking_permanent': perm,
            'permanent': perm,
        }
        assert effect.can_use_condition(context), (
            "Condition should pass: opponent has Digimon with exactly 3000 DP"
        )

    def test_condition_fails_3001dp(self, debug_runner):
        """Boundary: opponent Digimon with 3001 DP should NOT be a valid target."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT17-001", "ST1-03"])
        target = runner.place_on_field(2, ["ST1-03"])
        target.card_sources[0].base_dp = 3001

        effect = self._get_inherited_effect(perm)

        context = {
            'attacking_permanent': perm,
            'permanent': perm,
        }
        assert not effect.can_use_condition(context), (
            "Condition should fail: 3001 DP is above the 3000 DP threshold"
        )

    # ── Process callback: cost payment + deletion ────────────────────

    def test_process_pays_1_memory(self, debug_runner):
        """Process callback should pay 1 memory cost."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT17-001", "ST1-03"])
        runner.place_on_field(2, ["ST1-03"])  # 2000 DP target
        game = runner.game

        effect = self._get_inherited_effect(perm)

        memory_before = game.memory

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Memory should decrease by 1 (cost payment)
        assert game.memory == memory_before - 1, (
            f"Should pay 1 memory; was {memory_before}, now {game.memory}"
        )

    def test_process_deletes_valid_target(self, debug_runner):
        """Process callback should delete opponent Digimon with <= 3000 DP."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT17-001", "ST1-03"])
        target = runner.place_on_field(2, ["ST1-03"])  # 2000 DP
        game = runner.game

        effect = self._get_inherited_effect(perm)

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Should create a pending selection for opponent's field
        ps = game.pending_selection
        assert ps is not None, "Should create a pending selection for delete target"
        assert game.current_phase == GamePhase.SelectTarget

        assert SEL_OPP_FIELD_START in ps.valid_indices, (
            f"Target at index {SEL_OPP_FIELD_START} should be valid, got {ps.valid_indices}"
        )

        p2_field_before = len(game.player2.battle_area)
        ps.callback(SEL_OPP_FIELD_START)

        assert len(game.player2.battle_area) < p2_field_before, (
            "Opponent's Digimon with <= 3000 DP should be deleted"
        )

    def test_process_selection_is_mandatory(self, debug_runner):
        """Once the cost is paid, the deletion selection should be mandatory
        (is_optional=False on effect_select_opponent_permanent)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT17-001", "ST1-03"])
        target = runner.place_on_field(2, ["ST1-03"])  # 2000 DP
        game = runner.game

        effect = self._get_inherited_effect(perm)

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        ps = game.pending_selection
        assert ps is not None, "Should create a pending selection"
        # Mandatory selection means no "skip" or "pass" index in valid_indices
        # The pass/skip index is typically 0 in the action space
        # For mandatory selections, valid_indices should not include the pass action
        assert not ps.is_optional, (
            "Selection should be mandatory after paying cost"
        )

    def test_process_excludes_high_dp_target(self, debug_runner):
        """Process callback filter should exclude opponent Digimon with > 3000 DP."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT17-001", "ST1-03"])
        # Place two targets: one valid (2000 DP), one too high (5000 DP)
        runner.place_on_field(2, ["ST1-03"])  # 2000 DP -- slot 0
        high_dp = runner.place_on_field(2, ["ST1-03"])  # will be forced to 5000 DP -- slot 1
        high_dp.card_sources[0].base_dp = 5000
        game = runner.game

        effect = self._get_inherited_effect(perm)

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        ps = game.pending_selection
        assert ps is not None, "Should create a pending selection"
        # Only the first target (slot 0, 2000 DP) should be valid
        assert SEL_OPP_FIELD_START in ps.valid_indices, (
            "2000 DP target should be a valid selection"
        )
        assert (SEL_OPP_FIELD_START + 1) not in ps.valid_indices, (
            "5000 DP target should NOT be a valid selection"
        )
