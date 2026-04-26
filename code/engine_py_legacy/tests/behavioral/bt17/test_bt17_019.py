"""Behavioral tests for BT17-019 Gabumon (Lv.3 Blue Rookie).

Card text:
  Alt digivolve: from [Tsunomon] for cost 0.
  [Start of Your Main Phase] If you have a Tamer with [Matt Ishida] in its
      name, <Draw 1>.
  Inherited: [End of Your Turn] This Digimon and any of your other Digimon
      may DNA digivolve into a Digimon card in the hand.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT17019Gabumon:
    """Tests for BT17-019 Gabumon."""

    # ── Alt digivolve ────────────────────────────────────────────────

    def test_alt_digi_from_tsunomon(self, debug_runner):
        """Alt-digi effect should specify exact Tsunomon for cost 0."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT17-019"])
        top = perm.top_card
        effects = top.effect_list(None)

        alt_digi_effects = [
            e for e in effects
            if getattr(e, '_alt_digi_name', None) is not None
        ]
        assert len(alt_digi_effects) == 1, "Should have exactly 1 alt-digi effect"

        alt = alt_digi_effects[0]
        assert alt._alt_digi_name == "Tsunomon", "Alt-digi must be from Tsunomon"
        assert alt._alt_digi_cost == 0, "Alt-digi cost must be 0"
        assert alt._alt_digi_exact is True, "Alt-digi must be exact name match"

    # ── Start of Main Phase: Draw 1 ─────────────────────────────────

    def test_draw_with_matt_ishida_tamer(self, debug_runner):
        """With Matt Ishida tamer on field, the start-of-main effect should
        draw 1 card."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT17-019"])
        runner.place_on_field(1, ["BT1-086"])  # Matt Ishida

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)

        start_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnStartMainPhase
        ]
        assert len(start_effects) == 1, "Should have 1 OnStartMainPhase effect"

        eff = start_effects[0]
        assert eff.can_use_condition({}), (
            "Condition should pass with Matt Ishida tamer on field"
        )

        hand_before = len(game.player1.hand_cards)
        deck_before = len(game.player1.library_cards)

        eff.on_process_callback({
            'player': game.player1,
            'game': game,
        })

        assert len(game.player1.hand_cards) == hand_before + 1, (
            "Should draw 1 card"
        )
        assert len(game.player1.library_cards) == deck_before - 1, (
            "Should remove 1 card from deck"
        )

    def test_draw_without_matt_ishida(self, debug_runner):
        """Without Matt Ishida tamer, condition should fail."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT17-019"])
        # No Matt Ishida tamer

        top = perm.top_card
        effects = top.effect_list(None)
        start_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnStartMainPhase
        ]
        assert len(start_effects) == 1

        eff = start_effects[0]
        assert not eff.can_use_condition({}), (
            "Condition should fail without Matt Ishida tamer on field"
        )

    def test_draw_requires_deck_cards(self, debug_runner):
        """With Matt Ishida but empty deck, condition should fail
        (can't draw from empty deck)."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT17-019"])
        runner.place_on_field(1, ["BT1-086"])  # Matt Ishida

        game = runner.game
        # Empty the library
        game.player1.library_cards.clear()

        top = perm.top_card
        effects = top.effect_list(None)
        eff = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase][0]

        assert not eff.can_use_condition({}), (
            "Condition should fail with empty deck (no cards to draw)"
        )

    def test_draw_not_on_opponent_turn(self, debug_runner):
        """Effect should not activate on opponent's turn."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT17-019"])
        runner.place_on_field(1, ["BT1-086"])  # Matt Ishida

        game = runner.game
        # Switch turn ownership
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        top = perm.top_card
        effects = top.effect_list(None)
        eff = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase][0]

        assert not eff.can_use_condition({}), (
            "Condition should fail on opponent's turn"
        )

    # ── Inherited: End of Turn DNA digivolve ─────────────────────────

    def test_inherited_dna_effect_properties(self, debug_runner):
        """Inherited DNA digivolve effect should be optional, inherited,
        and trigger at End of Turn."""
        runner = debug_runner(initial_memory=5)

        # Place BT17-019 as a digivolution source under another Digimon
        perm = runner.place_on_field(1, ["BT17-019", "BT1-036"])  # Gabumon under Garurumon

        bottom_card = perm.card_sources[0]  # BT17-019 Gabumon
        effects = bottom_card.effect_list(None)
        eot_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn and e.is_inherited_effect
        ]
        assert len(eot_effects) == 1, "Should have exactly 1 inherited OnEndTurn effect"

        eot = eot_effects[0]
        assert eot.is_inherited_effect, "Must be inherited"
        assert eot.is_optional, "DNA digivolve from hand must be optional"

    def test_inherited_dna_condition_needs_other_digimon(self, debug_runner):
        """Condition should fail if there is no other Digimon on field."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT17-019", "BT1-036"])

        # Put a DNA Digimon in hand
        runner.inject_card(1, "BT17-078", "hand")  # Omnimon

        bottom_card = perm.card_sources[0]
        effects = bottom_card.effect_list(None)
        eot = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn and e.is_inherited_effect
        ][0]

        assert not eot.can_use_condition({}), (
            "Condition should fail with only 1 Digimon on field"
        )

    def test_inherited_dna_condition_needs_hand_cards(self, debug_runner):
        """Condition should fail if hand is empty."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT17-019", "BT1-036"])
        runner.place_on_field(1, ["ST2-06"])  # another Digimon

        runner.clear_zone(1, "hand")

        bottom_card = perm.card_sources[0]
        effects = bottom_card.effect_list(None)
        eot = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn and e.is_inherited_effect
        ][0]

        assert not eot.can_use_condition({}), (
            "Condition should fail with empty hand"
        )
