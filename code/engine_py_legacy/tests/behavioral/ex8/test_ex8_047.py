"""Behavioral tests for EX8-047 Sunarizamon (Lv.3 Black Digimon).

Card text:
  [On Play] Reveal the top 3 cards of your deck. Add 1 card with the [Mineral]
  or [Rock] trait and 1 card with the [LIBERATOR] trait among them to the hand.
  Return the rest to the bottom of the deck.

  Inherited Effect: When effects trash this card from a [Mineral] or [Rock] trait
  Digimon's digivolution cards, delete 1 of your opponent's Digimon with a play
  cost of 4 or less.

Key test cards:
  EX8-047: Sunarizamon (Reptile/LIBERATOR) -- the card itself
  EX8-048: Landramon  (Mineral/LIBERATOR, Lv.4, Cost 4)
  EX8-049: Golemon    (Mineral, Lv.4, Cost 5)
  EX8-050: Gogmamon   (Rock, Lv.5, Cost 7)
  BT2-054: Gotsumon   (Rock, Lv.3, Cost 3)
  BT1-010: Agumon     (Reptile, Lv.3, Cost 3) -- no relevant traits
  BT1-009: Monodramon (Mini Dragon, Lv.3, Cost 2) -- low cost, no relevant traits
  AD1-001: Greymon    (Dinosaur/ADVENTURE, Lv.4, Cost 5) -- cost > 4
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


# ---------------------------------------------------------------------------
# On Play Effect Tests
# ---------------------------------------------------------------------------

@pytest.mark.behavioral
class TestEX8047OnPlay:
    """Tests for EX8-047 [On Play] reveal 3, add Mineral/Rock + LIBERATOR."""

    def test_on_play_effect_exists(self, debug_runner):
        """On Play effect should exist with correct timing and is_on_play flag."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-047"])
        top = perm.top_card
        effects = top.effect_list(None)

        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ]
        assert len(on_play) == 1, "Should have exactly 1 On Play effect"

    def test_on_play_condition_passes_when_on_field(self, debug_runner):
        """On Play condition should pass when card is on the field."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-047"])
        top = perm.top_card
        effects = top.effect_list(None)

        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ][0]

        assert on_play.can_use_condition({}), (
            "Condition should pass when card is on the field"
        )

    def test_reveal_adds_mineral_and_liberator(self, debug_runner):
        """When revealed cards have Mineral/Rock and LIBERATOR matches, both are added."""
        runner = debug_runner(initial_memory=5)

        # Place Sunarizamon on field
        perm = runner.place_on_field(1, ["EX8-047"])

        # Inject known cards at top of deck:
        #   Top:    BT1-010 (Reptile, no match)
        #   2nd:    EX8-047 (LIBERATOR match)
        #   3rd:    BT2-054 (Rock match)
        # NOTE: library_top inserts at position 0, so last inject is on top
        runner.inject_card(1, "BT2-054", "library_top")   # will be at index 2
        runner.inject_card(1, "EX8-047", "library_top")   # will be at index 1
        runner.inject_card(1, "BT1-010", "library_top")   # will be at index 0 (top)

        game = runner.game
        hand_before = len(game.player1.hand_cards)
        deck_before = len(game.player1.library_cards)

        # Execute the on-play effect
        top = perm.top_card
        effects = top.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ][0]

        on_play.on_process_callback({
            'player': game.player1,
            'game': game,
        })
        runner.auto_resolve()

        hand_ids = [c.c_entity_base.card_id for c in game.player1.hand_cards]

        # BT2-054 (Rock) should be added to hand via pass 1
        assert "BT2-054" in hand_ids, (
            "Rock-trait card should be added to hand"
        )
        # EX8-047 (LIBERATOR) should be added to hand via pass 2
        assert "EX8-047" in hand_ids, (
            "LIBERATOR-trait card should be added to hand"
        )
        # BT1-010 (no match) should NOT be in hand (beyond what was already there)
        # It should go to deck bottom
        assert len(game.player1.hand_cards) == hand_before + 2, (
            "Exactly 2 cards should be added to hand"
        )

    def test_reveal_mineral_match_only(self, debug_runner):
        """When revealed has Mineral/Rock match but no LIBERATOR, only 1 goes to hand."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-047"])

        # Inject: 3 cards, only BT2-054 has Rock trait, none have LIBERATOR
        runner.inject_card(1, "BT1-010", "library_top")   # Reptile (no match)
        runner.inject_card(1, "BT1-009", "library_top")   # Mini Dragon (no match)
        runner.inject_card(1, "BT2-054", "library_top")   # Rock (match for pass 1)

        game = runner.game
        hand_before = len(game.player1.hand_cards)

        top = perm.top_card
        effects = top.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ][0]

        on_play.on_process_callback({
            'player': game.player1,
            'game': game,
        })
        runner.auto_resolve()

        hand_ids = [c.c_entity_base.card_id for c in game.player1.hand_cards]
        assert "BT2-054" in hand_ids, "Rock-trait card should be added to hand"
        assert len(game.player1.hand_cards) == hand_before + 1, (
            "Only 1 card added when only Mineral/Rock match exists"
        )

    def test_reveal_liberator_match_only(self, debug_runner):
        """When revealed has LIBERATOR match but no Mineral/Rock, only 1 goes to hand."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-047"])

        # EX8-047 has LIBERATOR trait but NOT Mineral or Rock
        runner.inject_card(1, "BT1-010", "library_top")   # no match
        runner.inject_card(1, "BT1-009", "library_top")   # no match
        runner.inject_card(1, "EX8-047", "library_top")   # LIBERATOR match for pass 2

        game = runner.game
        hand_before = len(game.player1.hand_cards)

        top = perm.top_card
        effects = top.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ][0]

        on_play.on_process_callback({
            'player': game.player1,
            'game': game,
        })
        runner.auto_resolve()

        hand_ids = [c.c_entity_base.card_id for c in game.player1.hand_cards]
        assert "EX8-047" in hand_ids, "LIBERATOR-trait card should be added to hand"
        assert len(game.player1.hand_cards) == hand_before + 1, (
            "Only 1 card added when only LIBERATOR match exists"
        )

    def test_reveal_no_matches(self, debug_runner):
        """When no revealed cards match either filter, all go to deck bottom."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-047"])

        # All cards have no Mineral/Rock/LIBERATOR traits
        runner.inject_card(1, "BT1-010", "library_top")   # Reptile
        runner.inject_card(1, "BT1-009", "library_top")   # Mini Dragon
        runner.inject_card(1, "BT1-010", "library_top")   # Reptile again

        game = runner.game
        hand_before = len(game.player1.hand_cards)
        deck_size_before = len(game.player1.library_cards)

        top = perm.top_card
        effects = top.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ][0]

        on_play.on_process_callback({
            'player': game.player1,
            'game': game,
        })
        runner.auto_resolve()

        assert len(game.player1.hand_cards) == hand_before, (
            "No cards should be added to hand when nothing matches"
        )
        # All 3 revealed cards should be returned to deck bottom
        assert len(game.player1.library_cards) == deck_size_before - 3 + 3, (
            "All revealed cards should return to deck bottom"
        )

    def test_reveal_mineral_card(self, debug_runner):
        """Mineral trait specifically matches the first pass filter."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-047"])

        # EX8-049 Golemon has Mineral trait
        runner.inject_card(1, "BT1-010", "library_top")
        runner.inject_card(1, "BT1-009", "library_top")
        runner.inject_card(1, "EX8-049", "library_top")   # Mineral

        game = runner.game

        top = perm.top_card
        effects = top.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ][0]

        on_play.on_process_callback({
            'player': game.player1,
            'game': game,
        })
        runner.auto_resolve()

        hand_ids = [c.c_entity_base.card_id for c in game.player1.hand_cards]
        assert "EX8-049" in hand_ids, (
            "Mineral-trait card should be added to hand via pass 1"
        )

    def test_dual_trait_card_picked_once(self, debug_runner):
        """A card with both Mineral and LIBERATOR can only be picked for one pass."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-047"])

        # EX8-048 Landramon has both Mineral and LIBERATOR
        # BT1-010 has neither
        runner.inject_card(1, "BT1-010", "library_top")
        runner.inject_card(1, "BT1-010", "library_top")
        runner.inject_card(1, "EX8-048", "library_top")   # Mineral + LIBERATOR

        game = runner.game
        hand_before = len(game.player1.hand_cards)

        top = perm.top_card
        effects = top.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ][0]

        on_play.on_process_callback({
            'player': game.player1,
            'game': game,
        })
        runner.auto_resolve()

        hand_ids = [c.c_entity_base.card_id for c in game.player1.hand_cards]
        assert "EX8-048" in hand_ids, (
            "Dual-trait card should be added to hand"
        )
        # The card is removed from pool after pass 1, so pass 2 can't pick it again
        assert len(game.player1.hand_cards) == hand_before + 1, (
            "Dual-trait card should only be added once (removed from pool after first pass)"
        )

    def test_remaining_cards_go_to_deck_bottom(self, debug_runner):
        """Non-selected revealed cards should go to deck bottom, not trash."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-047"])

        runner.inject_card(1, "BT1-010", "library_top")   # no match - should go to bottom
        runner.inject_card(1, "BT1-009", "library_top")   # no match - should go to bottom
        runner.inject_card(1, "BT2-054", "library_top")   # Rock - should go to hand

        game = runner.game
        trash_before = len(game.player1.trash_cards)

        top = perm.top_card
        effects = top.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ][0]

        on_play.on_process_callback({
            'player': game.player1,
            'game': game,
        })
        runner.auto_resolve()

        assert len(game.player1.trash_cards) == trash_before, (
            "Non-selected cards should NOT go to trash"
        )
        # The last 2 cards in deck should be the non-selected ones
        bottom_ids = [
            c.c_entity_base.card_id
            for c in game.player1.library_cards[-2:]
        ]
        assert "BT1-010" in bottom_ids or "BT1-009" in bottom_ids, (
            "Non-selected cards should be at deck bottom"
        )


# ---------------------------------------------------------------------------
# Inherited Effect Tests
# ---------------------------------------------------------------------------

@pytest.mark.behavioral
class TestEX8047Inherited:
    """Tests for EX8-047 inherited: trash from Mineral/Rock digi cards -> delete cost 4-."""

    def test_inherited_effect_exists(self, debug_runner):
        """Inherited effect should exist with correct timing and is_inherited_effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-047"])
        top = perm.top_card
        effects = top.effect_list(None)

        inherited = [
            e for e in effects
            if e.timing == EffectTiming.OnDigivolutionCardDiscarded
            and e.is_inherited_effect
        ]
        assert len(inherited) == 1, (
            "Should have exactly 1 inherited OnDigivolutionCardDiscarded effect"
        )

    def test_condition_passes_with_mineral_trait_permanent(self, debug_runner):
        """Condition should pass when permanent has Mineral trait and card is in trashed_cards."""
        runner = debug_runner(initial_memory=5)

        # EX8-049 Golemon (Mineral, Lv.4) on top, EX8-047 Sunarizamon underneath
        perm = runner.place_on_field(1, ["EX8-047", "EX8-049"])
        game = runner.game

        # Get EX8-047's inherited effect
        sunarizamon_card = perm.card_sources[0]  # bottom card
        effects = sunarizamon_card.effect_list(None)
        inherited = [
            e for e in effects
            if e.timing == EffectTiming.OnDigivolutionCardDiscarded
            and e.is_inherited_effect
        ][0]

        result = inherited.can_use_condition({
            'permanent': perm,
            'trashed_cards': [sunarizamon_card],
        })
        assert result is True, (
            "Condition should pass: permanent has Mineral trait and card is trashed"
        )

    def test_condition_passes_with_rock_trait_permanent(self, debug_runner):
        """Condition should pass when permanent has Rock trait."""
        runner = debug_runner(initial_memory=5)

        # EX8-050 Gogmamon (Rock, Lv.5) on top, EX8-047 underneath
        perm = runner.place_on_field(1, ["EX8-047", "EX8-050"])
        game = runner.game

        sunarizamon_card = perm.card_sources[0]
        effects = sunarizamon_card.effect_list(None)
        inherited = [
            e for e in effects
            if e.timing == EffectTiming.OnDigivolutionCardDiscarded
            and e.is_inherited_effect
        ][0]

        result = inherited.can_use_condition({
            'permanent': perm,
            'trashed_cards': [sunarizamon_card],
        })
        assert result is True, (
            "Condition should pass: permanent has Rock trait and card is trashed"
        )

    def test_condition_fails_without_mineral_or_rock(self, debug_runner):
        """Condition should fail when permanent lacks Mineral/Rock trait."""
        runner = debug_runner(initial_memory=5)

        # BT1-010 Agumon (Reptile, no Mineral/Rock) on top, EX8-047 underneath
        perm = runner.place_on_field(1, ["EX8-047", "BT1-010"])
        game = runner.game

        sunarizamon_card = perm.card_sources[0]
        effects = sunarizamon_card.effect_list(None)
        inherited = [
            e for e in effects
            if e.timing == EffectTiming.OnDigivolutionCardDiscarded
            and e.is_inherited_effect
        ][0]

        result = inherited.can_use_condition({
            'permanent': perm,
            'trashed_cards': [sunarizamon_card],
        })
        assert result is False, (
            "Condition should fail: permanent lacks Mineral/Rock trait"
        )

    def test_condition_fails_when_card_not_in_trashed(self, debug_runner):
        """Condition should fail when this card is not among the trashed cards."""
        runner = debug_runner(initial_memory=5)

        # EX8-049 Golemon (Mineral) on top, EX8-047 underneath
        perm = runner.place_on_field(1, ["EX8-047", "EX8-049"])
        game = runner.game

        sunarizamon_card = perm.card_sources[0]
        other_card = perm.card_sources[1]  # top card, not sunarizamon

        effects = sunarizamon_card.effect_list(None)
        inherited = [
            e for e in effects
            if e.timing == EffectTiming.OnDigivolutionCardDiscarded
            and e.is_inherited_effect
        ][0]

        result = inherited.can_use_condition({
            'permanent': perm,
            'trashed_cards': [other_card],  # different card trashed
        })
        assert result is False, (
            "Condition should fail: this card is not in trashed_cards"
        )

    def test_process_deletes_opponent_digimon_cost_4_or_less(self, debug_runner):
        """Process should delete an opponent's Digimon with play cost 4 or less."""
        runner = debug_runner(initial_memory=5)

        # Place a cheap Digimon on P2's field (BT1-009 Monodramon, cost 2)
        runner.place_on_field(2, ["BT1-009"])

        # Set up P1's permanent with EX8-047 under a Mineral Digimon
        perm = runner.place_on_field(1, ["EX8-047", "EX8-049"])
        game = runner.game

        sunarizamon_card = perm.card_sources[0]
        effects = sunarizamon_card.effect_list(None)
        inherited = [
            e for e in effects
            if e.timing == EffectTiming.OnDigivolutionCardDiscarded
            and e.is_inherited_effect
        ][0]

        p2_field_before = len(game.player2.battle_area)
        assert p2_field_before == 1, "Sanity: P2 should have 1 Digimon"

        inherited.on_process_callback({
            'player': game.player1,
            'game': game,
        })
        runner.auto_resolve()

        assert len(game.player2.battle_area) == 0, (
            "Opponent's Digimon (cost 2) should be deleted"
        )

    def test_process_does_not_delete_cost_5_digimon(self, debug_runner):
        """Process should NOT target opponent's Digimon with play cost > 4."""
        runner = debug_runner(initial_memory=5)

        # Place expensive Digimon on P2's field (AD1-001 Greymon, cost 5)
        runner.place_on_field(2, ["AD1-001"])

        perm = runner.place_on_field(1, ["EX8-047", "EX8-049"])
        game = runner.game

        sunarizamon_card = perm.card_sources[0]
        effects = sunarizamon_card.effect_list(None)
        inherited = [
            e for e in effects
            if e.timing == EffectTiming.OnDigivolutionCardDiscarded
            and e.is_inherited_effect
        ][0]

        p2_field_before = len(game.player2.battle_area)

        inherited.on_process_callback({
            'player': game.player1,
            'game': game,
        })
        runner.auto_resolve()

        assert len(game.player2.battle_area) == p2_field_before, (
            "Opponent's Digimon (cost 5) should NOT be deleted"
        )

    def test_process_deletes_cost_4_exactly(self, debug_runner):
        """Play cost of exactly 4 should be a valid target for deletion."""
        runner = debug_runner(initial_memory=5)

        # EX8-048 Landramon has play cost 4
        runner.place_on_field(2, ["EX8-048"])

        perm = runner.place_on_field(1, ["EX8-047", "EX8-049"])
        game = runner.game

        sunarizamon_card = perm.card_sources[0]
        effects = sunarizamon_card.effect_list(None)
        inherited = [
            e for e in effects
            if e.timing == EffectTiming.OnDigivolutionCardDiscarded
            and e.is_inherited_effect
        ][0]

        inherited.on_process_callback({
            'player': game.player1,
            'game': game,
        })
        runner.auto_resolve()

        assert len(game.player2.battle_area) == 0, (
            "Opponent's Digimon with exactly play cost 4 should be deleted"
        )

    def test_process_is_opponent_effect_delete(self, debug_runner):
        """Delete should be flagged as opponent effect for keyword interactions."""
        runner = debug_runner(initial_memory=5)

        runner.place_on_field(2, ["BT1-009"])
        perm = runner.place_on_field(1, ["EX8-047", "EX8-049"])
        game = runner.game

        # Track delete calls
        delete_calls = []
        original_delete = game.player2.delete_permanent

        def mock_delete(permanent, **kwargs):
            delete_calls.append(kwargs)
            return original_delete(permanent, **kwargs)

        game.player2.delete_permanent = mock_delete

        sunarizamon_card = perm.card_sources[0]
        effects = sunarizamon_card.effect_list(None)
        inherited = [
            e for e in effects
            if e.timing == EffectTiming.OnDigivolutionCardDiscarded
            and e.is_inherited_effect
        ][0]

        inherited.on_process_callback({
            'player': game.player1,
            'game': game,
        })
        runner.auto_resolve()

        assert len(delete_calls) == 1, "delete_permanent should be called once"
        assert delete_calls[0].get('is_opponent_effect') is True, (
            "Delete should pass is_opponent_effect=True since it's our effect on opponent's Digimon"
        )

    def test_no_opponent_digimon_means_no_selection(self, debug_runner):
        """When opponent has no valid targets, no selection phase occurs."""
        runner = debug_runner(initial_memory=5)

        # Opponent has no Digimon
        perm = runner.place_on_field(1, ["EX8-047", "EX8-049"])
        game = runner.game

        sunarizamon_card = perm.card_sources[0]
        effects = sunarizamon_card.effect_list(None)
        inherited = [
            e for e in effects
            if e.timing == EffectTiming.OnDigivolutionCardDiscarded
            and e.is_inherited_effect
        ][0]

        # Should not crash or enter selection when no targets
        inherited.on_process_callback({
            'player': game.player1,
            'game': game,
        })
        # No auto_resolve needed -- should complete immediately
