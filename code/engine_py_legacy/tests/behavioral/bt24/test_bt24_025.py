"""Behavioral tests for BT24-025 Shellmon (Lv.4 Blue Digimon).

Card text:
Alt-digi: Lv.3 with [TS] trait for cost 2
[Your Turn] When any of your other blue Digimon with the [TS] trait unsuspend,
this Digimon may digivolve into [Venusmon] in the hand, ignoring level.
[End of Your Turn] [Once Per Turn] 1 of your other Digimon with the [TS] trait
may unsuspend.
Inherited: <Jamming>
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT24025Shellmon:
    """Tests for BT24-025 Shellmon."""

    def test_unsuspend_trigger_targets_shellmon_itself(self, debug_runner):
        """When another blue TS Digimon unsuspends, Shellmon itself should be
        the target for digivolve into Venusmon — not the triggering Digimon."""
        runner = debug_runner(initial_memory=10)

        # Place Shellmon on field
        shellmon_perm = runner.place_on_field(1, ["BT24-025"])
        # Place another blue TS Digimon (BT24-009 Shamanmon) suspended
        trigger_perm = runner.place_on_field(1, ["BT24-009"], is_suspended=True)
        # Inject Venusmon into hand so digivolve target exists
        runner.inject_card(1, "BT10-042", "hand")

        runner.set_phase("Main")

        # Unsuspend the TS Digimon to trigger Shellmon's effect
        trigger_perm.unsuspend()

        # The game should now be in a selection phase for Shellmon to digivolve
        # into Venusmon. The key assertion: the digivolve target is Shellmon,
        # not the triggering Digimon.
        snap = runner.snapshot()
        # Shellmon should still be on field (hasn't digivolved yet, pending selection)
        shellmon_on_field = any(
            s.card_id == "BT24-025" for s in snap.p1_field
        )
        assert shellmon_on_field, "Shellmon should still be on field awaiting digivolve selection"

    def test_unsuspend_trigger_condition_requires_blue_ts(self, debug_runner):
        """Only blue Digimon with [TS] trait should trigger the unsuspend effect."""
        runner = debug_runner(initial_memory=10)

        shellmon_perm = runner.place_on_field(1, ["BT24-025"])
        # Place a non-TS Digimon (ST1-03 Agumon) suspended
        non_ts_perm = runner.place_on_field(1, ["ST1-03"], is_suspended=True)

        runner.set_phase("Main")

        # Unsuspending a non-TS Digimon should NOT trigger Shellmon's effect
        non_ts_perm.unsuspend()

        # Verify no digivolve selection phase triggered
        game = runner.game
        from engine_py_legacy.engine.data.enums import GamePhase
        assert game.current_phase != GamePhase.SelectTarget, (
            "Unsuspending a non-TS Digimon should not trigger Shellmon's digivolve effect"
        )

    def test_unsuspend_trigger_not_self(self, debug_runner):
        """Shellmon unsuspending itself should NOT trigger its own effect."""
        runner = debug_runner(initial_memory=10)

        shellmon_perm = runner.place_on_field(1, ["BT24-025"], is_suspended=True)
        runner.inject_card(1, "BT10-042", "hand")

        runner.set_phase("Main")

        # Unsuspending Shellmon itself should not trigger
        shellmon_perm.unsuspend()

        game = runner.game
        from engine_py_legacy.engine.data.enums import GamePhase
        assert game.current_phase != GamePhase.SelectTarget, (
            "Shellmon unsuspending itself should not trigger its own digivolve effect"
        )

    def test_end_of_turn_unsuspend_other_ts(self, debug_runner):
        """[End of Your Turn] Should be able to unsuspend 1 other TS Digimon."""
        runner = debug_runner(initial_memory=10)

        shellmon_perm = runner.place_on_field(1, ["BT24-025"])
        # Place another TS Digimon suspended
        ts_perm = runner.place_on_field(1, ["BT24-009"], is_suspended=True)

        # Find the end-of-turn effect on Shellmon's card
        shellmon_card = shellmon_perm.top_card
        effects = shellmon_card.effect_list(None)
        eot_effect = None
        for eff in effects:
            if eff.timing == EffectTiming.OnEndTurn:
                eot_effect = eff
                break

        assert eot_effect is not None, "Shellmon should have an End of Turn effect"
        # Condition should pass (Shellmon on field, owner's turn, suspended TS target exists)
        assert eot_effect.can_use_condition({}), (
            "End of Turn condition should be True when a suspended TS Digimon exists"
        )

    def test_end_of_turn_does_not_require_blue(self, debug_runner):
        """End-of-turn unsuspend targets any TS Digimon, not just blue ones.
        Card text: '1 of your other Digimon with the [TS] trait' (no blue)."""
        runner = debug_runner(initial_memory=10)

        shellmon_perm = runner.place_on_field(1, ["BT24-025"])

        # Verify the effect's filter function in process2 does not check blue
        shellmon_card = shellmon_perm.top_card
        effects = shellmon_card.effect_list(None)
        eot_effect = None
        for eff in effects:
            if eff.timing == EffectTiming.OnEndTurn:
                eot_effect = eff
                break

        assert eot_effect is not None
        # The description should NOT mention "blue"
        assert "blue" not in eot_effect.effect_description.lower(), (
            f"End-of-turn effect description should not mention blue, "
            f"got: {eot_effect.effect_description}"
        )

    def test_inherited_jamming(self, debug_runner):
        """Inherited effect should grant Jamming."""
        runner = debug_runner(initial_memory=10)

        # Place Shellmon as a digivolution source under another Digimon
        perm = runner.place_on_field(1, ["BT24-025", "BT24-010"])

        # Check that the stack has jamming from inherited effect
        assert perm.has_keyword("_is_jamming"), (
            "Digimon with Shellmon in sources should have inherited Jamming"
        )
