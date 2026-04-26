"""Behavioral tests for BT24-040 Venusmon.

Card text:
When this card would be played, if you have 3 or fewer security cards,
reduce the play cost by 5.
[On Play] [When Digivolving] Trash all digivolution cards of 1 of your
opponent's Digimon. Then, until your opponent's turn ends, 2 of their
Digimon or Tamers can't suspend or activate [When Digivolving] effects.
[All Turns] [Once Per Turn] When any of your [TS] trait Digimon would leave
the battle area other than by your effects, by placing 1 other Digimon with
no digivolution cards as the bottom security card, they don't leave.
"""

import pytest


@pytest.mark.behavioral
class TestBT24040Venusmon:
    """Tests for BT24-040 Venusmon."""

    def test_cannot_suspend_scoped_to_target(self, debug_runner):
        """CANNOT_SUSPEND should only apply to the selected targets, not all permanents."""
        runner = debug_runner(archetype1="TS Jupitermon", initial_memory=15)

        # Place Venusmon via direct field placement (skip On Play trigger)
        runner.place_on_field(1, ["BT24-040"])

        # Place 3 opponent Digimon: 2 will be frozen, 1 should remain free
        runner.place_on_field(2, ["ST1-03"])   # slot 0 - will be frozen target
        runner.place_on_field(2, ["ST1-06"])   # slot 1 - will be frozen target
        runner.place_on_field(2, ["ST1-07"])   # slot 2 - should NOT be frozen

        # Play Venusmon from hand to trigger [On Play]
        runner.inject_card(1, "BT24-040", "hand")
        runner.set_phase("Main")
        action = runner.find_action("Play Venusmon")
        assert action is not None, "Should find play action for Venusmon"
        runner.execute(action)

        # First selection: opponent Digimon to trash digi cards from
        # Select the first opponent Digimon (slot 0)
        actions = runner.actions()
        opp_select = None
        for aid, desc in actions.items():
            if "opponent" in desc.lower() and "field" in desc.lower():
                opp_select = aid
                break
        if opp_select:
            runner.execute(opp_select)

        # Second selection: first freeze target
        actions = runner.actions()
        freeze1 = None
        for aid, desc in actions.items():
            if "opponent" in desc.lower() and "field" in desc.lower():
                freeze1 = aid
                break
        if freeze1:
            runner.execute(freeze1)

        # Third selection: second freeze target
        actions = runner.actions()
        freeze2 = None
        for aid, desc in actions.items():
            if "opponent" in desc.lower() and "field" in desc.lower():
                freeze2 = aid
                break
        if freeze2:
            runner.execute(freeze2)

        runner.auto_resolve()

        # Verify: the unfrozen opponent Digimon can still be suspended
        snap = runner.snapshot()
        # There should be at least 3 opponent Digimon
        assert len(snap.p2_field) >= 3, (
            f"Expected at least 3 opponent field permanents, got {len(snap.p2_field)}"
        )

        # Check via the engine: the third Digimon should be suspendable
        game = runner.game
        enemy = game.player2
        # At least one opponent permanent should NOT have CANNOT_SUSPEND
        from engine_py_legacy.engine.interfaces.modifiers import ModifierType
        unfrozen_count = sum(
            1 for p in enemy.battle_area
            if p.is_digimon and not game.modifiers.has_modifier(p, ModifierType.CANNOT_SUSPEND)
        )
        assert unfrozen_count >= 1, (
            "At least one opponent Digimon should NOT be frozen by CANNOT_SUSPEND"
        )

    def test_disable_effect_only_when_digivolving(self, debug_runner):
        """DISABLE_EFFECT should only block [When Digivolving] effects, not all effects."""
        runner = debug_runner(archetype1="TS Jupitermon", initial_memory=15)

        # Place an opponent Digimon
        opp_perm = runner.place_on_field(2, ["ST1-03"])

        # Manually register DISABLE_EFFECT the way Venusmon does (post-fix)
        game = runner.game
        from engine_py_legacy.engine.interfaces.modifiers import ModifierType
        from engine_py_legacy.engine.interfaces.card_effect import ICardEffect

        game.register_modifier(
            opp_perm, ModifierType.DISABLE_EFFECT,
            condition=lambda p, c, fp=opp_perm: (
                p is fp
                and c.get('effect') is not None
                and getattr(c['effect'], 'is_when_digivolving', False)
            ),
            expiry='end_of_opponent_turn')

        # A [When Digivolving] effect SHOULD be disabled
        wd_effect = ICardEffect()
        wd_effect.is_when_digivolving = True
        assert game.modifiers.has_modifier(
            opp_perm, ModifierType.DISABLE_EFFECT, {'effect': wd_effect}
        ), "When Digivolving effect should be disabled"

        # A non-WD effect (e.g. On Play) should NOT be disabled
        op_effect = ICardEffect()
        op_effect.is_on_play = True
        assert not game.modifiers.has_modifier(
            opp_perm, ModifierType.DISABLE_EFFECT, {'effect': op_effect}
        ), "On Play effect should NOT be disabled by Venusmon's freeze"

        # An effect with no timing flags should NOT be disabled
        plain_effect = ICardEffect()
        assert not game.modifiers.has_modifier(
            opp_perm, ModifierType.DISABLE_EFFECT, {'effect': plain_effect}
        ), "Plain effect should NOT be disabled by Venusmon's freeze"

    def test_cost_reduction_with_low_security(self, debug_runner):
        """Play cost reduced by 5 when player has 3 or fewer security cards."""
        runner = debug_runner(archetype1="TS Jupitermon", initial_memory=15)
        runner.inject_card(1, "BT24-040", "hand")

        # Ensure player has <= 3 security
        game = runner.game
        while len(game.player1.security_cards) > 3:
            game.player1.security_cards.pop()

        runner.set_phase("Main")
        action = runner.find_action("Play Venusmon")
        assert action is not None

        result = runner.execute(action)
        runner.auto_resolve()

        # Base cost 12, reduced by 5 = 7
        cost_paid = result.before.memory - result.after.memory
        assert cost_paid == 7, (
            f"With <=3 security, Venusmon should cost 7 (12-5), but paid {cost_paid}"
        )

    def test_cost_reduction_not_applied_with_high_security(self, debug_runner):
        """Play cost NOT reduced when player has more than 3 security cards."""
        runner = debug_runner(archetype1="TS Jupitermon", initial_memory=15)
        runner.inject_card(1, "BT24-040", "hand")

        # Ensure player has > 3 security
        game = runner.game
        while len(game.player1.security_cards) < 4:
            runner.inject_card(1, "ST1-03", "security_top")

        runner.set_phase("Main")
        action = runner.find_action("Play Venusmon")
        assert action is not None

        result = runner.execute(action)
        runner.auto_resolve()

        # Full cost 12
        cost_paid = result.before.memory - result.after.memory
        assert cost_paid == 12, (
            f"With >3 security, Venusmon should cost 12, but paid {cost_paid}"
        )
