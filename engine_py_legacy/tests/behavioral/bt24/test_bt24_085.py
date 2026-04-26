"""Behavioral tests for BT24-085 Dan Yuki & Kanan Yuki (Tamer).

Card text:
  [Start of Your Main Phase] If you have 4 or less memory, gain 1 memory.
  [End of Your Turn] By suspending this Tamer, you may use 1 [TS] trait Option
  card with as high or lower a use cost as your opponent's memory from your hand
  without paying the cost. Then, 1 of your Digimon with the [TS] trait may attack.
  [Security] Play this card without paying the cost.
"""

import pytest


@pytest.mark.behavioral
class TestBT24085DanYukiKananYuki:
    """Tests for BT24-085 Dan Yuki & Kanan Yuki."""

    # ── Effect 0: Start of Main Phase memory gain ────────────────────

    def test_start_main_phase_memory_gain_at_4(self, debug_runner):
        """If memory <= 4 at start of main phase, gain 1 memory."""
        runner = debug_runner(initial_memory=4)
        runner.place_on_field(1, ["BT24-085"])
        runner.set_phase("Main")
        # Manually fire start-of-main effects
        from engine_py_legacy.engine.data.enums import EffectTiming
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)
        snap = runner.snapshot()
        assert snap.memory == 5, f"Expected memory 5, got {snap.memory}"

    def test_start_main_phase_no_gain_above_4(self, debug_runner):
        """If memory > 4, no gain."""
        runner = debug_runner(initial_memory=5)
        runner.place_on_field(1, ["BT24-085"])
        runner.set_phase("Main")
        from engine_py_legacy.engine.data.enums import EffectTiming
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)
        snap = runner.snapshot()
        assert snap.memory == 5, f"Expected memory 5 (unchanged), got {snap.memory}"

    # ── Effect 1: End of Turn ────────────────────────────────────────

    def test_eot_condition_blocks_when_positive_memory(self, debug_runner):
        """EOT effect should not activate when turn player has positive memory."""
        runner = debug_runner(initial_memory=3)
        runner.place_on_field(1, ["BT24-085"])
        runner.place_on_field(1, ["BT24-009"])  # TS Digimon
        runner.inject_card(1, "BT24-090", "hand")  # TS Option cost 3
        runner.set_phase("Main")
        # Memory is 3 (positive) — opponent has -3 — gate should block
        from engine_py_legacy.engine.data.enums import EffectTiming
        runner.game.execute_effects(EffectTiming.OnEndTurn)
        # Tamer should NOT be suspended (effect didn't fire)
        snap = runner.snapshot()
        tamer = next((s for s in snap.p1_field if s.card_id == "BT24-085"), None)
        assert tamer is not None
        assert not tamer.is_suspended, "Tamer should NOT suspend when memory > 0"

    def test_eot_suspends_tamer_and_creates_option_selection(self, debug_runner):
        """EOT effect should suspend tamer and offer option selection."""
        runner = debug_runner(initial_memory=-3)
        runner.place_on_field(1, ["BT24-085"])
        runner.place_on_field(1, ["BT24-009"])  # TS Digimon (Shamanmon)
        runner.inject_card(1, "BT24-090", "hand")  # TS Option cost 3

        runner.set_phase("End")
        runner.game.phase_end()

        # Tamer should be suspended
        snap = runner.snapshot()
        tamer = next((s for s in snap.p1_field if s.card_id == "BT24-085"), None)
        assert tamer is not None
        assert tamer.is_suspended, (
            f"Tamer should be suspended after EOT effect. "
            f"Phase: {snap.phase}, actions: {runner.actions()}")

        # Should be in a selection phase for the option
        ps = runner.game.pending_selection
        assert ps is not None, "Should have a pending selection for TS Option"

    def test_eot_option_play_chains_to_attack_selection(self, debug_runner):
        """After selecting an option, should chain to TS Digimon attack selection."""
        runner = debug_runner(initial_memory=-3)
        runner.place_on_field(1, ["BT24-085"])
        runner.place_on_field(1, ["BT24-009"])  # TS Digimon (Shamanmon)
        runner.inject_card(1, "BT24-090", "hand")  # TS Option cost 3

        runner.set_phase("End")
        runner.game.phase_end()

        # First selection: pick the option (or decline)
        # Decline the option to test chain to attacker selection
        ps = runner.game.pending_selection
        assert ps is not None, "Should have pending selection"
        runner.execute(62)  # Decline option

        # Should now have selection for TS Digimon attacker
        ps2 = runner.game.pending_selection
        assert ps2 is not None, (
            "After declining option, should have attack selection. "
            f"Phase: {runner.snapshot().phase}")
        assert 'digimon' in ps2.prompt.lower() or 'attack' in ps2.prompt.lower(), (
            f"Expected attack selection, got: {ps2.prompt}")

    def test_eot_may_attack_creates_eot_action_phase(self, debug_runner):
        """After selecting a TS Digimon, MAY_ATTACK should create EndOfTurnAction phase."""
        runner = debug_runner(initial_memory=-3)
        runner.place_on_field(1, ["BT24-085"])
        ts_perm = runner.place_on_field(1, ["BT24-009"])  # TS Digimon (Shamanmon)
        # No TS option in hand — skip straight to attacker selection

        runner.set_phase("End")
        runner.game.phase_end()

        # Should be in selection for the attacker (no option to select)
        ps = runner.game.pending_selection
        assert ps is not None, "Should have pending selection for attacker"

        # Select the TS Digimon
        legal = runner.action_mask()
        non_pass = [a for a in legal if a != 62]
        assert non_pass, f"Should have a non-pass action for TS Digimon. Legal: {legal}"
        runner.execute(non_pass[0])

        # Now should be in EndOfTurnAction phase with attack options
        snap = runner.snapshot()
        assert snap.phase == "EndOfTurnAction", (
            f"Expected EndOfTurnAction phase, got {snap.phase}. "
            f"Actions: {runner.actions()}")

        # The selected Digimon should be unsuspended and able to attack
        assert not ts_perm.is_suspended, "Selected Digimon should be unsuspended"

    def test_eot_no_ts_digimon_skips_attack(self, debug_runner):
        """If no TS Digimon on field, attack selection is skipped."""
        runner = debug_runner(initial_memory=-3)
        runner.place_on_field(1, ["BT24-085"])
        runner.place_on_field(1, ["BT1-010"])  # Non-TS Digimon

        runner.set_phase("End")
        runner.game.phase_end()

        # Tamer should still be suspended (cost was paid)
        snap = runner.snapshot()
        tamer = next((s for s in snap.p1_field if s.card_id == "BT24-085"), None)
        assert tamer is not None
        assert tamer.is_suspended, "Tamer should be suspended"

    def test_eot_option_cost_exceeds_opp_memory_not_selectable(self, debug_runner):
        """Options costing more than opponent's memory should not be selectable."""
        # Memory = -1 → opponent has 1 memory. BT24-091 costs 5, should not be valid.
        runner = debug_runner(initial_memory=-1)
        runner.place_on_field(1, ["BT24-085"])
        runner.place_on_field(1, ["BT24-009"])
        runner.inject_card(1, "BT24-091", "hand")  # TS Option cost 5

        runner.set_phase("End")
        runner.game.phase_end()

        # The selection should be for the attacker (option too expensive)
        ps = runner.game.pending_selection
        assert ps is not None
        # Should be the attacker selection, not option selection
        assert 'digimon' in ps.prompt.lower() or 'attack' in ps.prompt.lower(), (
            f"Expected attacker selection (option too expensive), got: {ps.prompt}")
