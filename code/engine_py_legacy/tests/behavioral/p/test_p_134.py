"""Behavioral tests for P-134 Shoemon (Lv.3 Yellow Puppet/LIBERATOR, DP 1000, Cost 3).

Card text:
[On Play] 1 of your opponent's Digimon gains <Security A. -1> until the end of
    their turn.
Inherited: [When Attacking] [Once Per Turn] 1 of your opponent's Digimon gets
    -2000 DP for the turn.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase
from engine_py_legacy.engine.interfaces.modifiers import ModifierType


@pytest.mark.behavioral
class TestP134OnPlay:
    """Tests for [On Play] SA -1 effect."""

    def test_on_play_sa_minus_1_via_modifier(self, debug_runner):
        """On Play should register a CHANGE_SECURITY_ATTACK modifier on the
        selected opponent Digimon, giving SA -1."""
        runner = debug_runner(initial_memory=3)

        # Place an opponent Digimon as the target
        opp_perm = runner.place_on_field(2, ["ST1-03"])  # Agumon, Lv.3

        sa_before = opp_perm.security_attack_modifier()

        # Clear hand, then put P-134 into hand and play it
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "P-134", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Shoemon")
        assert action is not None, "Should find action to play Shoemon"

        result = runner.execute(action)
        runner.auto_resolve()  # resolve target selection

        sa_after = opp_perm.security_attack_modifier()
        assert sa_after == sa_before - 1, (
            f"Target should have SA -1. Before: {sa_before}, After: {sa_after}"
        )

    def test_on_play_sa_modifier_uses_register_modifier(self, debug_runner):
        """SA -1 should be implemented via game.register_modifier with
        CHANGE_SECURITY_ATTACK, not via _security_attack_modifier on effects."""
        runner = debug_runner(initial_memory=3)

        opp_perm = runner.place_on_field(2, ["ST1-03"])

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "P-134", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Shoemon")
        assert action is not None
        result = runner.execute(action)
        runner.auto_resolve()

        # Check that the modifier registry has CHANGE_SECURITY_ATTACK entries
        sa_modifiers = runner.game.modifiers._modifiers.get(
            ModifierType.CHANGE_SECURITY_ATTACK, [])
        assert len(sa_modifiers) > 0, (
            "SA -1 should be registered via ModifierType.CHANGE_SECURITY_ATTACK"
        )

    def test_on_play_sa_modifier_expires_end_of_opponent_turn(self, debug_runner):
        """SA -1 modifier should have 'end_of_opponent_turn' expiry (C# UntilOpponentTurnEnd)."""
        runner = debug_runner(initial_memory=3)

        opp_perm = runner.place_on_field(2, ["ST1-03"])

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "P-134", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Shoemon")
        assert action is not None
        result = runner.execute(action)
        runner.auto_resolve()

        sa_modifiers = runner.game.modifiers._modifiers.get(
            ModifierType.CHANGE_SECURITY_ATTACK, [])
        assert any(e.expiry == 'end_of_opponent_turn' for e in sa_modifiers), (
            "SA -1 modifier should expire at end_of_opponent_turn"
        )

    def test_on_play_requires_opponent_digimon(self, debug_runner):
        """On Play should not enter selection if opponent has no Digimon."""
        runner = debug_runner(initial_memory=3)

        # Clear opponent field
        runner.clear_zone(2, "field")

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "P-134", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Shoemon")
        assert action is not None
        result = runner.execute(action)

        # After playing, no selection should be pending (no targets)
        snap = runner.snapshot()
        # Should be back in Main phase, not stuck in SelectTarget
        assert snap.phase == "Main" or snap.phase == "End", (
            f"Without opponent Digimon, should not stay in SelectTarget. Phase: {snap.phase}"
        )

    def test_on_play_enters_selection_with_target(self, debug_runner):
        """On Play should enter target selection when opponent has Digimon."""
        runner = debug_runner(initial_memory=3)

        opp_perm = runner.place_on_field(2, ["ST1-03"])

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "P-134", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Shoemon")
        assert action is not None
        result = runner.execute(action)

        # Should be in selection phase (mandatory, not optional)
        snap = runner.snapshot()
        assert snap.phase == "SelectTarget", (
            f"Should enter SelectTarget to choose opponent's Digimon. Phase: {snap.phase}"
        )


@pytest.mark.behavioral
class TestP134InheritedWhenAttacking:
    """Tests for inherited [When Attacking] [Once Per Turn] -2000 DP."""

    def test_inherited_dp_minus_on_attack(self, debug_runner):
        """Inherited: when attacking, select 1 opponent Digimon for -2000 DP."""
        runner = debug_runner(initial_memory=5)

        # Clear hand so no interference
        runner.clear_zone(1, "hand")

        # Place Lv.4 with P-134 as inherited source (bottom of stack)
        perm = runner.place_on_field(1, ["P-134", "ST19-08"])  # Shoemon -> ShoeShoemon
        opp_perm = runner.place_on_field(2, ["ST1-03"])  # Agumon target

        dp_before = opp_perm.dp
        runner.set_phase("Main")

        # Find attack action for ShoeShoemon
        action = runner.find_action("attack")
        assert action is not None, (
            f"Should find attack action. Available: {runner.actions()}"
        )
        result = runner.execute(action)
        runner.auto_resolve()  # resolve target selection for -2000 DP

        dp_after = opp_perm.dp
        assert dp_after == dp_before - 2000, (
            f"Target should have -2000 DP. Before: {dp_before}, After: {dp_after}"
        )

    def test_inherited_once_per_turn(self, debug_runner):
        """Inherited DP debuff is once per turn — second attack should not trigger again."""
        runner = debug_runner(initial_memory=5)
        runner.clear_zone(1, "hand")

        perm = runner.place_on_field(1, ["P-134", "ST19-08"])
        opp_perm = runner.place_on_field(2, ["ST1-03"])

        runner.set_phase("Main")

        # First attack
        action = runner.find_action("attack")
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

        dp_after_first = opp_perm.dp

        # Unsuspend attacker for second attack
        perm.unsuspend()
        runner.set_phase("Main")

        # Second attack
        action = runner.find_action("attack")
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

        dp_after_second = opp_perm.dp
        assert dp_after_second == dp_after_first, (
            "Once per turn: second attack should not apply additional DP debuff. "
            f"After 1st: {dp_after_first}, After 2nd: {dp_after_second}"
        )

    def test_inherited_fires_on_own_attack(self, debug_runner):
        """Inherited WA should fire when the Digimon carrying the inherited attacks."""
        runner = debug_runner(initial_memory=5)
        runner.clear_zone(1, "hand")
        runner.clear_zone(2, "hand")

        # Place Digimon with P-134 inherited (bottom: P-134, top: ST19-08 ShoeShoemon)
        perm_with_inherited = runner.place_on_field(1, ["P-134", "ST19-08"])
        opp_perm = runner.place_on_field(2, ["ST1-03"])  # Agumon DP 2000

        runner.set_phase("Main")

        dp_before = opp_perm.dp

        # Attack with the Digimon that has P-134 inherited
        action = runner.find_action("attack")
        assert action is not None, "Should find attack action for ShoeShoemon"
        runner.execute(action)
        runner.auto_resolve()

        dp_after = opp_perm.dp
        assert dp_after == dp_before - 2000, (
            f"Inherited WA should trigger when own Digimon attacks. "
            f"Before: {dp_before}, After: {dp_after}"
        )
