"""Behavioral tests for BT16-088 Cody Hida & T.K. Takaishi (Tamer, Black/Yellow, Cost 4).

Card text:
  [Start of Your Main Phase] You may play 1 [Armadillomon] or [Patamon] from
  your hand without paying the cost. At the next end of your opponent's turn,
  return it to the hand.
  [Your Turn] When one of your Digimon digivolves into a black or yellow
  Digimon, by suspending this Tamer, gain 1 memory. If DNA digivolving,
  <De-Digivolve 1> 1 of your opponent's Digimon. (Trash the top card. You
  can't trash past level 3 cards.)
  Security: Play this card without paying the cost.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, CardColor


@pytest.mark.behavioral
class TestBT16088CodyTK:
    """Tests for BT16-088 Cody Hida & T.K. Takaishi."""

    # ── Script importability ─────────────────────────────────────────

    def test_script_importable(self):
        """Script module should import without error."""
        import importlib
        mod = importlib.import_module(
            "engine_py_legacy.engine.data.scripts.bt16.bt16_088")
        assert hasattr(mod, "BT16_088")

    # ── Security Effect ──────────────────────────────────────────────

    def test_has_security_effect(self, debug_runner):
        """Should have a Security effect to play for free."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "BT16-088", "hand")
        effects = card.effect_list(None)
        sec = [e for e in effects if e.is_security_effect]
        assert len(sec) >= 1, "Should have Security effect"

    # ── [Start of Main Phase] Play Armadillomon/Patamon free ─────────

    def test_start_main_phase_effect_exists(self, debug_runner):
        """Should have OnStartMainPhase effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-088"])
        card = perm.top_card
        effects = card.effect_list(None)
        main_phase = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase]
        assert len(main_phase) >= 1, "Should have OnStartMainPhase effect"

    def test_start_main_phase_is_optional(self, debug_runner):
        """Start of Main Phase effect should be optional ('You may')."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-088"])
        card = perm.top_card
        effects = card.effect_list(None)
        main_phase = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase][0]
        assert main_phase.is_optional, "Start of Main Phase effect should be optional"

    def test_start_main_phase_condition_requires_armadillomon_or_patamon(self, debug_runner):
        """Condition should require an Armadillomon or Patamon in hand."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-088"])
        card = perm.top_card
        effects = card.effect_list(None)
        main_phase = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase][0]

        # Clear hand and insert non-matching card
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT1-010", "hand")  # Agumon - not Armadillomon/Patamon
        assert not main_phase.can_use_condition({}), (
            "Should fail without Armadillomon/Patamon in hand")

        # Add Patamon to hand
        runner.inject_card(1, "BT1-048", "hand")  # Patamon
        assert main_phase.can_use_condition({}), (
            "Should pass with Patamon in hand")

    def test_start_main_phase_condition_accepts_armadillomon(self, debug_runner):
        """Condition should also accept Armadillomon in hand."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-088"])
        card = perm.top_card
        effects = card.effect_list(None)
        main_phase = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase][0]

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT1-027", "hand")  # Armadillomon
        assert main_phase.can_use_condition({}), (
            "Should pass with Armadillomon in hand")

    def test_start_main_phase_condition_requires_own_turn(self, debug_runner):
        """Condition should only pass on owner's turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-088"])
        card = perm.top_card
        effects = card.effect_list(None)
        main_phase = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase][0]

        runner.inject_card(1, "BT1-048", "hand")  # Patamon

        # P1's turn
        assert main_phase.can_use_condition({}), "Should pass on P1's turn"

        # Switch to P2's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True
        assert not main_phase.can_use_condition({}), "Should fail on P2's turn"

    def test_start_main_phase_requires_field_presence(self, debug_runner):
        """Condition should fail when the Tamer is not on the field (e.g., still in hand)."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "BT16-088", "hand")
        runner.inject_card(1, "BT1-048", "hand")  # Patamon
        effects = card.effect_list(None)
        main_phase = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase][0]
        assert not main_phase.can_use_condition({}), (
            "Should fail when the Tamer is not on the field")

    # ── End of Opponent's Turn bounce ────────────────────────────────

    def test_bounce_effect_exists(self, debug_runner):
        """Should have an OnEndTurn effect for bouncing."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-088"])
        card = perm.top_card
        effects = card.effect_list(None)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn]
        assert len(end_turn) >= 1, "Should have OnEndTurn bounce effect"

    def test_bounce_is_not_optional(self, debug_runner):
        """Bounce back is mandatory, not optional."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-088"])
        card = perm.top_card
        effects = card.effect_list(None)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]
        assert not end_turn.is_optional, "Bounce should be mandatory (not optional)"

    def test_bounce_condition_fails_without_pending(self, debug_runner):
        """Bounce condition should fail when no pending bounce is registered."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-088"])
        card = perm.top_card
        effects = card.effect_list(None)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]
        # No Patamon was played by the effect — bounce should not fire
        assert not end_turn.can_use_condition({}), (
            "Bounce should not fire with no pending bounce")

    def test_bounce_returns_played_digimon_to_hand_on_opponent_end_turn(self, debug_runner):
        """Running the start-of-main-phase process then opponent-end-turn should bounce Patamon back."""
        runner = debug_runner(initial_memory=5)
        tamer_perm = runner.place_on_field(1, ["BT16-088"])
        tamer_card = tamer_perm.top_card
        effects = tamer_card.effect_list(None)
        main_phase = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase][0]
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        # Put Patamon in hand
        runner.clear_zone(1, "hand")
        patamon_card = runner.inject_card(1, "BT1-048", "hand")  # Patamon

        # Run the start-of-main-phase process callback
        ctx = {
            'player': runner.game.player1,
            'game': runner.game,
            'permanent': tamer_perm,
        }
        main_phase.on_process_callback(ctx)

        # Patamon should have been placed in pending selection OR auto-selected.
        # Resolve any pending selection by invoking its callback with the
        # first valid index.
        if getattr(runner.game, 'pending_selection', None):
            ps = runner.game.pending_selection
            valid = getattr(ps, 'valid_indices', None) or []
            if valid and ps.callback:
                ps.callback(valid[0])
            runner.game.pending_selection = None

        # Now we expect Patamon to be on the field for P1
        p1_battle_names = [
            p.top_card.card_names[0] if p.top_card and p.top_card.card_names else ''
            for p in runner.game.player1.battle_area
        ]
        assert any('Patamon' in n for n in p1_battle_names), (
            f"Patamon should be on P1 battle area after play. Got: {p1_battle_names}")

        # Remember the played permanent to check bounce later
        played_perm = next(p for p in runner.game.player1.battle_area
                           if p.top_card and any('Patamon' in n for n in p.top_card.card_names))

        # Now simulate opponent's turn end: switch turn to P2 so it's P2's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        # Condition should now be True
        assert end_turn.can_use_condition({}), (
            "Bounce condition should fire on opponent turn end")

        # Run bounce process
        bounce_ctx = {
            'player': runner.game.player1,
            'game': runner.game,
            'permanent': tamer_perm,
        }
        end_turn.on_process_callback(bounce_ctx)

        # Played permanent should be gone from battle area and Patamon in hand
        assert played_perm not in runner.game.player1.battle_area, (
            "Played permanent should be removed from battle area")
        p1_hand_names = [
            c.card_names[0] if c.card_names else '' for c in runner.game.player1.hand_cards
        ]
        assert any('Patamon' in n for n in p1_hand_names), (
            f"Patamon should be back in P1 hand. Got: {p1_hand_names}")

    def test_bounce_does_not_fire_on_own_turn_end(self, debug_runner):
        """Bounce should NOT fire at the end of the owner's own turn."""
        runner = debug_runner(initial_memory=5)
        tamer_perm = runner.place_on_field(1, ["BT16-088"])
        tamer_card = tamer_perm.top_card
        effects = tamer_card.effect_list(None)
        main_phase = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase][0]
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT1-048", "hand")  # Patamon

        # Run start-of-main-phase
        main_phase.on_process_callback({
            'player': runner.game.player1,
            'game': runner.game,
            'permanent': tamer_perm,
        })
        # Resolve pending selection
        if getattr(runner.game, 'pending_selection', None):
            ps = runner.game.pending_selection
            valid = getattr(ps, 'valid_indices', None) or []
            if valid and ps.callback:
                ps.callback(valid[0])
            runner.game.pending_selection = None

        # On OWN turn (P1 still is_my_turn), bounce should NOT trigger
        assert not end_turn.can_use_condition({}), (
            "Bounce should not fire at own turn end")

    # ── [Your Turn] When digivolving trigger ─────────────────────────

    def test_digivolve_trigger_effect_exists(self, debug_runner):
        """Should have a When Digivolving trigger effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-088"])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving]
        assert len(when_digi) >= 1, "Should have When Digivolving effect"

    def test_digivolve_trigger_is_optional(self, debug_runner):
        """Digivolve trigger should be optional (suspension is cost)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-088"])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving][0]
        assert when_digi.is_optional, "Digivolve trigger should be optional"

    def test_digivolve_trigger_requires_unsuspended_tamer(self, debug_runner):
        """Condition should require this Tamer to be unsuspended (suspend is cost)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-088"])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving][0]

        # Place a Yellow Digimon to trigger the condition
        yellow_perm = runner.place_on_field(1, ["BT1-048"])  # Patamon (Yellow)

        ctx = {'digivolved_permanent': yellow_perm}
        assert when_digi.can_use_condition(ctx), (
            "Should pass when tamer is unsuspended")

        # Suspend the tamer
        perm.suspend()
        assert not when_digi.can_use_condition(ctx), (
            "Should fail when tamer is suspended")

    def test_digivolve_trigger_requires_black_or_yellow(self, debug_runner):
        """Condition should only trigger for black or yellow Digimon digivolving."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-088"])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving][0]

        # Red Digimon - should fail
        red_perm = runner.place_on_field(1, ["BT1-010"])  # Agumon (Red)
        ctx = {'digivolved_permanent': red_perm}
        assert not when_digi.can_use_condition(ctx), (
            "Should fail for Red Digimon digivolving")

        # Yellow Digimon - should pass
        yellow_perm = runner.place_on_field(1, ["BT1-048"])  # Patamon (Yellow)
        ctx = {'digivolved_permanent': yellow_perm}
        assert when_digi.can_use_condition(ctx), (
            "Should pass for Yellow Digimon digivolving")

    def test_digivolve_trigger_requires_black_accepts_black(self, debug_runner):
        """Condition should also pass for Black Digimon."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-088"])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving][0]

        black_perm = runner.place_on_field(1, ["BT2-052"])  # Hagurumon (Black)
        ctx = {'digivolved_permanent': black_perm}
        assert when_digi.can_use_condition(ctx), (
            "Should pass for Black Digimon digivolving")

    def test_digivolve_trigger_requires_own_digimon(self, debug_runner):
        """Condition should only trigger for player's own Digimon, not opponent's."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-088"])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving][0]

        # Opponent's yellow Digimon
        opp_yellow = runner.place_on_field(2, ["BT1-048"])
        ctx = {'digivolved_permanent': opp_yellow}
        assert not when_digi.can_use_condition(ctx), (
            "Should fail for opponent's Digimon digivolving")

    def test_digivolve_trigger_requires_own_turn(self, debug_runner):
        """Condition should require owner's turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-088"])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving][0]

        yellow_perm = runner.place_on_field(1, ["BT1-048"])
        ctx = {'digivolved_permanent': yellow_perm}
        assert when_digi.can_use_condition(ctx), "Should pass on P1's turn"

        # Opponent's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True
        assert not when_digi.can_use_condition(ctx), (
            "Should fail on opponent's turn")

    def test_digivolve_trigger_gains_1_memory_and_suspends(self, debug_runner):
        """Process should suspend this Tamer and gain 1 memory."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-088"])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving][0]

        memory_before = runner.game.memory
        assert not perm.is_suspended, "Tamer should start unsuspended"

        ctx = {
            'player': runner.game.player1,
            'game': runner.game,
            'is_dna_digivolve': False,
        }
        when_digi.on_process_callback(ctx)

        assert perm.is_suspended, "Tamer should be suspended after process"
        assert runner.game.memory == memory_before + 1, (
            f"Should gain 1 memory. Before: {memory_before}, After: {runner.game.memory}")

    def test_digivolve_trigger_non_dna_does_not_de_digivolve(self, debug_runner):
        """Non-DNA digivolve: process should NOT trigger a de-digivolve selection."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-088"])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving][0]

        # Opponent has a 3-card stack
        runner.game.player2.trash_cards.clear()
        opp_perm = runner.place_on_field(2, ["BT1-010", "BT1-010", "BT1-025"])
        stack_before = len(opp_perm.card_sources)

        ctx = {
            'player': runner.game.player1,
            'game': runner.game,
            'is_dna_digivolve': False,
        }
        when_digi.on_process_callback(ctx)

        # Opponent stack unchanged
        assert len(opp_perm.card_sources) == stack_before, (
            "Stack should be unchanged without DNA")
        assert len(runner.game.player2.trash_cards) == 0, (
            "Opponent trash should be empty without DNA")

    def test_digivolve_trigger_dna_de_digivolves_1(self, debug_runner):
        """If DNA digivolving, should de-digivolve 1 of opponent's Digimon."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-088"])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving][0]

        runner.game.player2.trash_cards.clear()
        # Give opponent a 3-card stack
        opp_perm = runner.place_on_field(2, ["BT1-010", "BT1-010", "BT1-025"])
        stack_before = len(opp_perm.card_sources)

        ctx = {
            'player': runner.game.player1,
            'game': runner.game,
            'is_dna_digivolve': True,
        }
        when_digi.on_process_callback(ctx)

        # Drive any pending selection for de-digivolve target.
        if getattr(runner.game, 'pending_selection', None):
            ps = runner.game.pending_selection
            valid = getattr(ps, 'valid_indices', None) or []
            if valid and ps.callback:
                ps.callback(valid[0])
            runner.game.pending_selection = None

        # The selected opponent perm should have lost its top card
        assert len(opp_perm.card_sources) == stack_before - 1, (
            f"Opponent stack should lose 1 card on DNA de-digivolve. "
            f"Before: {stack_before}, After: {len(opp_perm.card_sources)}")
        assert len(runner.game.player2.trash_cards) == 1, (
            f"Opponent trash should have 1 card. Got: {len(runner.game.player2.trash_cards)}")

    def test_digivolve_trigger_dna_on_no_opponent_digimon_still_gains_memory(self, debug_runner):
        """Even if there are no opponent Digimon to de-digivolve, memory/suspend still happen."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-088"])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving][0]

        # No opponent Digimon on field
        for p in list(runner.game.player2.battle_area):
            runner.game.player2.battle_area.remove(p)

        memory_before = runner.game.memory
        ctx = {
            'player': runner.game.player1,
            'game': runner.game,
            'is_dna_digivolve': True,
        }
        when_digi.on_process_callback(ctx)

        assert perm.is_suspended, "Tamer should suspend"
        assert runner.game.memory == memory_before + 1, "Memory should still increase"
