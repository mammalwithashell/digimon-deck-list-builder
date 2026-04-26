"""Behavioral tests for BT16-085 Davis Motomiya & Ken Ichijoji (Tamer, Blue/Green, Cost 4).

Card text:
  [Start of Your Main Phase] You may play 1 [Veemon] or [Wormmon] from your
  hand without paying the cost. At the next end of your opponent's turn,
  return it to the hand.
  [Your Turn] When one of your Digimon digivolves into a blue or green
  Digimon, by suspending this Tamer, gain 1 memory. If DNA digivolving,
  trash any 3 digivolution cards under your opponent's Digimon.
  Security: Play this card without paying the cost.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, CardColor


@pytest.mark.behavioral
class TestBT16085DavisKen:
    """Tests for BT16-085 Davis Motomiya & Ken Ichijoji."""

    # ── Security Effect ──────────────────────────────────────────────

    def test_has_security_effect(self, debug_runner):
        """Should have a Security effect to play for free."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "BT16-085", "hand")
        effects = card.effect_list(None)
        sec = [e for e in effects if e.is_security_effect]
        assert len(sec) >= 1, "Should have Security effect"

    # ── [Start of Main Phase] Play Veemon/Wormmon free ───────────────

    def test_start_main_phase_effect_exists(self, debug_runner):
        """Should have OnStartMainPhase effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-085"])
        card = perm.top_card
        effects = card.effect_list(None)
        main_phase = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase]
        assert len(main_phase) >= 1, "Should have OnStartMainPhase effect"

    def test_start_main_phase_is_optional(self, debug_runner):
        """Start of Main Phase effect should be optional."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-085"])
        card = perm.top_card
        effects = card.effect_list(None)
        main_phase = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase][0]
        assert main_phase.is_optional, "Start of Main Phase effect should be optional"

    def test_start_main_phase_condition_requires_veemon_or_wormmon_in_hand(self, debug_runner):
        """Condition should require a Veemon or Wormmon in hand."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-085"])
        card = perm.top_card
        effects = card.effect_list(None)
        main_phase = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase][0]

        # No Veemon/Wormmon in hand - should fail
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT1-010", "hand")  # Agumon, not Veemon/Wormmon
        assert not main_phase.can_use_condition({}), (
            "Should fail without Veemon/Wormmon in hand")

        # Add Veemon to hand - should pass
        runner.inject_card(1, "BT3-021", "hand")  # Veemon
        assert main_phase.can_use_condition({}), (
            "Should pass with Veemon in hand")

    def test_start_main_phase_condition_accepts_wormmon(self, debug_runner):
        """Condition should also accept Wormmon in hand."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-085"])
        card = perm.top_card
        effects = card.effect_list(None)
        main_phase = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase][0]

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT3-047", "hand")  # Wormmon
        assert main_phase.can_use_condition({}), (
            "Should pass with Wormmon in hand")

    def test_start_main_phase_condition_requires_own_turn(self, debug_runner):
        """Condition should only pass on owner's turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-085"])
        card = perm.top_card
        effects = card.effect_list(None)
        main_phase = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase][0]

        runner.inject_card(1, "BT3-021", "hand")  # Veemon

        # P1's turn - should pass
        assert main_phase.can_use_condition({}), "Should pass on P1's turn"

        # Switch to P2's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True
        assert not main_phase.can_use_condition({}), "Should fail on P2's turn"

    # ── End of Opponent's Turn bounce ────────────────────────────────

    def test_bounce_effect_exists(self, debug_runner):
        """Should have an OnEndTurn effect for bouncing."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-085"])
        card = perm.top_card
        effects = card.effect_list(None)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn]
        assert len(end_turn) >= 1, "Should have OnEndTurn bounce effect"

    def test_bounce_is_not_optional(self, debug_runner):
        """Bounce back is mandatory, not optional."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-085"])
        card = perm.top_card
        effects = card.effect_list(None)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]
        assert not end_turn.is_optional, "Bounce should be mandatory (not optional)"

    # ── [Your Turn] Digivolve trigger ────────────────────────────────

    def test_digivolve_trigger_effect_exists(self, debug_runner):
        """Should have a When Digivolving trigger effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-085"])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving]
        assert len(when_digi) >= 1, "Should have When Digivolving effect"

    def test_digivolve_trigger_is_optional(self, debug_runner):
        """Digivolve trigger should be optional (suspension is cost)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-085"])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving][0]
        assert when_digi.is_optional, "Digivolve trigger should be optional"

    def test_digivolve_trigger_requires_unsuspended_tamer(self, debug_runner):
        """Condition should require this Tamer to be unsuspended (suspend is cost)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-085"])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving][0]

        # Place a blue Digimon to be the "digivolved" permanent
        blue_perm = runner.place_on_field(1, ["BT3-021"])  # Veemon (Blue)

        ctx = {'digivolved_permanent': blue_perm}
        assert when_digi.can_use_condition(ctx), (
            "Should pass when tamer is unsuspended")

        # Suspend the tamer
        perm.suspend()
        assert not when_digi.can_use_condition(ctx), (
            "Should fail when tamer is suspended")

    def test_digivolve_trigger_requires_blue_or_green(self, debug_runner):
        """Condition should only trigger for blue or green Digimon digivolving."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-085"])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving][0]

        # Red Digimon - should fail
        red_perm = runner.place_on_field(1, ["BT1-010"])  # Agumon (Red)
        ctx = {'digivolved_permanent': red_perm}
        assert not when_digi.can_use_condition(ctx), (
            "Should fail for Red Digimon digivolving")

        # Blue Digimon - should pass
        blue_perm = runner.place_on_field(1, ["BT3-021"])  # Veemon (Blue)
        ctx = {'digivolved_permanent': blue_perm}
        assert when_digi.can_use_condition(ctx), (
            "Should pass for Blue Digimon digivolving")

    def test_digivolve_trigger_requires_own_digimon(self, debug_runner):
        """Condition should only trigger for player's own Digimon, not opponent's."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-085"])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving][0]

        # Opponent's blue Digimon - should fail
        opp_blue = runner.place_on_field(2, ["BT3-021"])
        ctx = {'digivolved_permanent': opp_blue}
        assert not when_digi.can_use_condition(ctx), (
            "Should fail for opponent's Digimon digivolving")

    def test_digivolve_trigger_gains_1_memory_and_suspends(self, debug_runner):
        """Process should suspend this Tamer and gain 1 memory."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-085"])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving][0]

        memory_before = runner.game.memory
        assert not perm.is_suspended, "Tamer should start unsuspended"

        ctx = {'player': runner.game.player1, 'game': runner.game, 'is_dna_digivolve': False}
        when_digi.on_process_callback(ctx)

        assert perm.is_suspended, "Tamer should be suspended after process"
        assert runner.game.memory == memory_before + 1, (
            f"Should gain 1 memory. Before: {memory_before}, After: {runner.game.memory}")

    def test_digivolve_trigger_dna_trashes_3_opp_digi_cards(self, debug_runner):
        """If DNA digivolving, should trash any 3 digivolution cards from opponent's Digimon."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-085"])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving][0]

        # Clear P2 trash
        runner.game.player2.trash_cards.clear()

        # Place opponent Digimon with 4 digi-cards (5 total in stack)
        opp_perm = runner.place_on_field(2, [
            "BT1-010", "BT1-010", "BT1-010", "BT1-010", "BT1-025"
        ])
        assert len(opp_perm.card_sources) == 5

        ctx = {
            'player': runner.game.player1,
            'game': runner.game,
            'is_dna_digivolve': True,
        }
        when_digi.on_process_callback(ctx)
        # The new selection flow requires resolving pending selections
        # (pick permanent → pick cards × 3)
        runner.auto_resolve()

        # Should trash exactly 3 digi-cards
        assert len(runner.game.player2.trash_cards) == 3, (
            f"Should trash 3 digi-cards on DNA. Got {len(runner.game.player2.trash_cards)}")
        # Opponent perm should still have 2 cards (5 - 3 = 2)
        assert len(opp_perm.card_sources) == 2, (
            f"Opponent perm should have 2 cards left. Got {len(opp_perm.card_sources)}")

    def test_digivolve_trigger_non_dna_does_not_trash(self, debug_runner):
        """Without DNA, should NOT trash opponent digi-cards."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-085"])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving][0]

        runner.game.player2.trash_cards.clear()
        opp_perm = runner.place_on_field(2, ["BT1-010", "BT1-025"])
        stack_before = len(opp_perm.card_sources)

        ctx = {
            'player': runner.game.player1,
            'game': runner.game,
            'is_dna_digivolve': False,
        }
        when_digi.on_process_callback(ctx)

        assert len(opp_perm.card_sources) == stack_before, (
            "Should NOT trash digi-cards without DNA digivolving")
        assert len(runner.game.player2.trash_cards) == 0, (
            "P2 trash should remain empty without DNA")

    def test_digivolve_trigger_dna_trashes_across_multiple_digimon(self, debug_runner):
        """DNA trash 3 should work across multiple opponent Digimon if needed."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-085"])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving][0]

        runner.game.player2.trash_cards.clear()

        # Opponent has 2 Digimon each with 2 digi-cards (3 in stack)
        opp_perm1 = runner.place_on_field(2, ["BT1-010", "BT1-010", "BT1-025"])
        opp_perm2 = runner.place_on_field(2, ["BT1-010", "BT1-010", "BT1-025"])

        ctx = {
            'player': runner.game.player1,
            'game': runner.game,
            'is_dna_digivolve': True,
        }
        when_digi.on_process_callback(ctx)
        runner.auto_resolve()

        total_trashed = len(runner.game.player2.trash_cards)
        assert total_trashed == 3, (
            f"Should trash exactly 3 total across all opponent Digimon. Got {total_trashed}")

    def test_digivolve_trigger_requires_own_turn(self, debug_runner):
        """Condition should require it to be owner's turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-085"])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving][0]

        blue_perm = runner.place_on_field(1, ["BT3-021"])

        # P1's turn - should pass
        ctx = {'digivolved_permanent': blue_perm}
        assert when_digi.can_use_condition(ctx), "Should pass on P1's turn"

        # Switch to P2's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True
        assert not when_digi.can_use_condition(ctx), (
            "Should fail on opponent's turn")
