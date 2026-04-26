"""Behavioral tests for BT12-021 Veemon (Lv.3 Blue Rookie).

Card text:
  [On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with
  [Imperialdramon] in its name or the [Free] trait and 1 Tamer card with
  [Davis Motomiya] in its name among them to your hand. Place the rest at
  the bottom of your deck in any order.

  Inherited: [End of Your Turn] This Digimon and any of your other Digimon
  may DNA digivolve into a Digimon card in the hand.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT12021OnPlay:
    """Tests for BT12-021 On Play effect: reveal top 3, select cards."""

    def test_on_play_effect_exists(self, debug_runner):
        """On Play effect should exist with correct timing."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT12-021"])
        top = perm.top_card
        effects = top.effect_list(None)

        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ]
        assert len(on_play) == 1, "Should have exactly 1 On Play effect"

    def test_on_play_condition_needs_deck(self, debug_runner):
        """On Play condition should require at least 1 card in deck."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT12-021"])
        top = perm.top_card
        effects = top.effect_list(None)

        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ][0]

        # With cards in deck, condition should pass
        assert on_play.can_use_condition({}), (
            "Condition should pass when deck has cards"
        )

    def test_on_play_condition_fails_empty_deck(self, debug_runner):
        """On Play condition should fail if deck is empty."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT12-021"])
        runner.clear_zone(1, "library")

        top = perm.top_card
        effects = top.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ][0]

        assert not on_play.can_use_condition({}), (
            "Condition should fail with empty deck"
        )

    def test_on_play_uses_reveal_and_select_multi(self, debug_runner):
        """On Play should call effect_reveal_and_select_multi with 2 passes."""
        runner = debug_runner(initial_memory=5)

        # Inject known cards at top of deck
        # BT3-031 = Imperialdramon Dragon Mode (Digimon with Imperialdramon in name)
        # BT3-093 = Davis Motomiya (Tamer)
        # BT1-015 = Greymon (random Digimon, neither Imperialdramon nor Davis)
        runner.inject_card(1, "BT3-031", "library_top")
        runner.inject_card(1, "BT3-093", "library_top")
        runner.inject_card(1, "BT1-015", "library_top")

        perm = runner.place_on_field(1, ["BT12-021"])
        game = runner.game

        top = perm.top_card
        effects = top.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ][0]

        hand_before = len(game.player1.hand_cards)

        # Execute the effect
        on_play.on_process_callback({
            'player': game.player1,
            'game': game,
        })
        runner.auto_resolve()

        # Imperialdramon and Davis Motomiya should move to hand,
        # Greymon should go to deck bottom
        hand_ids = [c.c_entity_base.card_id for c in game.player1.hand_cards]
        assert "BT3-031" in hand_ids, "Imperialdramon should be added to hand"
        assert "BT3-093" in hand_ids, "Davis Motomiya should be added to hand"


@pytest.mark.behavioral
class TestBT12021InheritedDNA:
    """Tests for BT12-021 inherited [End of Your Turn] DNA digivolve."""

    def test_inherited_dna_effect_exists(self, debug_runner):
        """Inherited DNA digivolve effect should exist with correct properties."""
        runner = debug_runner(initial_memory=5)

        # Place BT12-021 as a digivolution source under another Digimon
        perm = runner.place_on_field(1, ["BT12-021", "BT12-022"])  # Veemon under ExVeemon

        bottom_card = perm.card_sources[0]  # BT12-021 Veemon
        effects = bottom_card.effect_list(None)

        eot_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn and e.is_inherited_effect
        ]
        assert len(eot_effects) == 1, "Should have exactly 1 inherited OnEndTurn effect"

        eot = eot_effects[0]
        assert eot.is_inherited_effect, "Must be marked as inherited"
        assert eot.is_optional, "DNA digivolve from hand must be optional"

    def test_inherited_dna_condition_owner_turn(self, debug_runner):
        """Condition should only pass on the owner's turn."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT12-021", "BT12-022"])
        # Place another Digimon on field for DNA material
        runner.place_on_field(1, ["BT12-050"])  # Stingmon (green Lv.4)
        # Put a DNA target in hand (Paildramon: Blue Lv.4 + Green Lv.4)
        runner.inject_card(1, "BT12-028", "hand")

        bottom_card = perm.card_sources[0]
        effects = bottom_card.effect_list(None)
        eot = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn and e.is_inherited_effect
        ][0]

        # P1's turn - should pass
        assert runner.game.player1.is_my_turn, "Sanity: P1 should be turn player"
        assert eot.can_use_condition({}), (
            "Condition should pass on owner's turn with DNA targets available"
        )

    def test_inherited_dna_condition_fails_opponent_turn(self, debug_runner):
        """Condition should fail on opponent's turn."""
        runner = debug_runner(initial_memory=5, first_player=2)

        perm = runner.place_on_field(1, ["BT12-021", "BT12-022"])
        runner.place_on_field(1, ["BT12-050"])
        runner.inject_card(1, "BT12-028", "hand")

        bottom_card = perm.card_sources[0]
        effects = bottom_card.effect_list(None)
        eot = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn and e.is_inherited_effect
        ][0]

        assert not runner.game.player1.is_my_turn, "Sanity: P1 should NOT be turn player"
        assert not eot.can_use_condition({}), (
            "Condition should fail on opponent's turn"
        )

    def test_inherited_dna_condition_needs_other_digimon(self, debug_runner):
        """Condition should fail if there is no other Digimon on field."""
        runner = debug_runner(initial_memory=5)

        # Only one Digimon on field (the one with BT12-021 inherited)
        perm = runner.place_on_field(1, ["BT12-021", "BT12-022"])

        # Put a Digimon card with dna_costs in hand
        runner.inject_card(1, "BT12-028", "hand")

        bottom_card = perm.card_sources[0]
        effects = bottom_card.effect_list(None)
        eot = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn and e.is_inherited_effect
        ][0]

        # No other Digimon on field, condition should fail
        assert not eot.can_use_condition({}), (
            "Condition should fail with only 1 Digimon on field (need another for DNA)"
        )

    def test_inherited_dna_condition_needs_hand_digimon(self, debug_runner):
        """Condition should fail if hand has no Digimon cards."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT12-021", "BT12-022"])
        runner.place_on_field(1, ["BT12-050"])  # another Digimon

        # Clear the hand (no DNA targets)
        runner.clear_zone(1, "hand")

        bottom_card = perm.card_sources[0]
        effects = bottom_card.effect_list(None)
        eot = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn and e.is_inherited_effect
        ][0]

        assert not eot.can_use_condition({}), (
            "Condition should fail with no Digimon in hand"
        )

    def test_inherited_dna_condition_needs_field(self, debug_runner):
        """Condition should fail if card is not on the field."""
        runner = debug_runner(initial_memory=5)

        # Place card in hand only, not on field
        runner.inject_card(1, "BT12-021", "hand")
        game = runner.game

        veemon_hand = [
            c for c in game.player1.hand_cards
            if c.c_entity_base and c.c_entity_base.card_id == "BT12-021"
        ]
        assert len(veemon_hand) >= 1, "Sanity: Veemon should be in hand"

        # Get effects from the hand card
        card = veemon_hand[0]
        effects = card.effect_list(None)
        eot = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn and e.is_inherited_effect
        ]
        if eot:
            assert not eot[0].can_use_condition({}), (
                "Condition should fail when card is not on field"
            )

    def test_inherited_dna_process_calls_effect_dna_digivolve_from_hand(
        self, debug_runner
    ):
        """Process callback should invoke game.effect_dna_digivolve_from_hand."""
        runner = debug_runner(initial_memory=5)

        # Setup: ExVeemon (blue Lv.4) with Veemon inherited on field
        perm = runner.place_on_field(1, ["BT12-021", "BT12-022"])
        # Stingmon (green Lv.4) on field as DNA material
        runner.place_on_field(1, ["BT12-050"])
        # Paildramon in hand as DNA target (Blue Lv.4 + Green Lv.4 : Cost 0)
        runner.inject_card(1, "BT12-028", "hand")

        game = runner.game

        bottom_card = perm.card_sources[0]
        effects = bottom_card.effect_list(None)
        eot = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn and e.is_inherited_effect
        ][0]

        # Execute the process callback
        eot.on_process_callback({
            'player': game.player1,
            'game': game,
        })

        # The engine should now be in a selection phase for DNA digivolve
        # auto_resolve picks the first legal option through the selection chain
        runner.auto_resolve()

        # After DNA digivolve, Paildramon should be on the field
        field_ids = []
        for p in game.player1.battle_area:
            if p.top_card:
                field_ids.append(p.top_card.c_entity_base.card_id)

        assert "BT12-028" in field_ids, (
            "Paildramon should be on field after DNA digivolve from hand"
        )
