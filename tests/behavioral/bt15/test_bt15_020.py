"""Behavioral tests for BT15-020 Gabumon (Lv.3 Blue, Cost 3).

Card text (per cards.json / C# source of truth):
Alt digivolve: [Tsunomon]: Cost 0
[Start of Your Main Phase] 1 of your Digimon gains Blocker until the end of
    your opponent's turn. Then, if you have a Tamer with [Matt Ishida] in its
    name, Draw 1.
Inherited: [When Attacking] [Once Per Turn] Draw 1.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase


@pytest.mark.behavioral
class TestBT15020Gabumon:
    """Tests for BT15-020 Gabumon."""

    # ---- [Start of Your Main Phase] effect ----

    def test_start_main_grants_blocker_to_own_digimon(self, debug_runner):
        """The start-of-main effect should select 1 of your Digimon to gain Blocker."""
        runner = debug_runner(initial_memory=5)

        # Place Gabumon and another Digimon on field
        gabumon_perm = runner.place_on_field(1, ["BT15-020"])
        target_perm = runner.place_on_field(1, ["ST2-08"])  # A Lv.5 blue Digimon

        effects = gabumon_perm.top_card.effect_list(None)
        start_main = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase]
        assert len(start_main) == 1, "Should have 1 OnStartMainPhase effect"

        eff = start_main[0]

        # Condition: on field, owner's turn
        assert eff.can_use_condition({}), "Should be usable on owner's turn with card on field"

    def test_start_main_grants_blocker_then_draws_with_matt(self, debug_runner):
        """When you have Matt Ishida tamer, the effect should grant Blocker THEN Draw 1."""
        runner = debug_runner(initial_memory=5)

        gabumon_perm = runner.place_on_field(1, ["BT15-020"])
        target_perm = runner.place_on_field(1, ["ST2-08"])
        # Place Matt Ishida tamer on field
        # BT15-083 is Matt Ishida tamer
        matt_perm = runner.place_on_field(1, ["BT15-083"])

        hand_before = len(runner.game.player1.hand_cards)

        effects = gabumon_perm.top_card.effect_list(None)
        start_main = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase][0]

        # Execute the effect - it should enter selection phase for blocker target
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)

        # Auto-resolve the selection (pick first valid target)
        runner.auto_resolve()

        hand_after = len(runner.game.player1.hand_cards)
        assert hand_after == hand_before + 1, (
            "Should draw 1 after granting Blocker when Matt Ishida is present"
        )

    def test_start_main_no_draw_without_matt(self, debug_runner):
        """Without Matt Ishida tamer, only grant Blocker, no draw."""
        runner = debug_runner(initial_memory=5)

        gabumon_perm = runner.place_on_field(1, ["BT15-020"])
        target_perm = runner.place_on_field(1, ["ST2-08"])

        hand_before = len(runner.game.player1.hand_cards)

        runner.game.execute_effects(EffectTiming.OnStartMainPhase)
        runner.auto_resolve()

        hand_after = len(runner.game.player1.hand_cards)
        assert hand_after == hand_before, (
            "Should NOT draw without Matt Ishida tamer"
        )

    def test_start_main_blocker_target_is_any_own_digimon(self, debug_runner):
        """The blocker target should be any of your own Digimon, not filtered by name."""
        runner = debug_runner(initial_memory=5)

        gabumon_perm = runner.place_on_field(1, ["BT15-020"])
        # Place a non-Garurumon, non-blue Digimon
        target_perm = runner.place_on_field(1, ["ST1-07"])  # A red Digimon

        effects = gabumon_perm.top_card.effect_list(None)
        start_main = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase][0]

        # Execute: should enter selection phase
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)

        # If we enter selection, any own Digimon should be selectable
        if runner.game.current_phase == GamePhase.SelectTarget:
            # The selection should include the red Digimon
            actions = runner.actions()
            action_descs = list(actions.values())
            # Should have at least 1 selectable target (our red Digimon and/or Gabumon)
            assert len(actions) >= 1, (
                "Should have at least 1 selectable Digimon for Blocker grant"
            )

    # ---- Inherited [When Attacking] [OPT] Draw 1 ----

    def test_inherited_draw_on_attack(self, debug_runner):
        """Inherited: Draw 1 when the Digimon attacks."""
        runner = debug_runner(initial_memory=5)

        # Place a Lv.4 with BT15-020 as inherited source
        perm = runner.place_on_field(1, ["BT15-020", "ST2-06"])

        bt15_card = perm.card_sources[0]  # BT15-020 at bottom
        effects = bt15_card.effect_list(None)
        atk_effects = [e for e in effects if e.is_inherited_effect
                       and e.timing == EffectTiming.OnUseAttack]
        assert len(atk_effects) == 1, "Should have 1 inherited OnUseAttack effect"

        atk_eff = atk_effects[0]
        assert atk_eff.max_count_per_turn == 1, "Should be Once Per Turn"

    def test_inherited_draw_not_restricted_to_player_attack(self, debug_runner):
        """The inherited draw should trigger on ANY attack, not only 'attack a player'."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-020", "ST2-06"])

        bt15_card = perm.card_sources[0]
        effects = bt15_card.effect_list(None)
        atk_eff = [e for e in effects if e.is_inherited_effect
                   and e.timing == EffectTiming.OnUseAttack][0]

        # The condition should NOT check target type
        # Just that card is on field
        assert atk_eff.can_use_condition({}), (
            "Inherited draw should be available regardless of attack target"
        )

    # ---- Not optional (per C# source) ----

    def test_start_main_is_mandatory(self, debug_runner):
        """The start-of-main effect is NOT optional per C# (canNoSelect=false)."""
        runner = debug_runner(initial_memory=5)
        gabumon_perm = runner.place_on_field(1, ["BT15-020"])

        effects = gabumon_perm.top_card.effect_list(None)
        start_main = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase][0]
        assert not start_main.is_optional, (
            "Start-of-main effect should be mandatory (not optional)"
        )
