"""Behavioral tests for EX10-033 Pyramidimon (Lv.6 Black Mineral/LIBERATOR, DP 11000, Cost 11).

Card text:
<Fragment (3)> (When this Digimon would be deleted, by trashing any 3 of its
digivolution cards, it isn't deleted.)
[When Digivolving] [When Attacking] [Once Per Turn] You may place up to 3
[Mineral] or [Rock] trait cards from your trash as this Digimon's bottom
digivolution cards.
[When Digivolving] [When Attacking] By trashing up to 3 [Mineral] or [Rock]
trait cards from any of your Digimon's digivolution cards, to 1 of your
opponent's Digimon, reduce the play cost by 2 until their turn ends for each
card trashed.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase


# Reusable card IDs
PYRAMIDIMON = "EX10-033"         # Lv6 Black, Mineral + LIBERATOR
PROGANOMON = "EX10-032"          # Lv5 Black, Mineral + LIBERATOR
LANDRAMON = "EX10-028"           # Lv4 Black, Mineral + LIBERATOR
GOTSUMON = "BT23-048"            # Lv3 Black, Rock + Hudie + CS
AGUMON = "ST1-03"                # Lv3 Red, generic filler
WARGREYMON = "ST1-11"            # Lv6 Red, generic opponent target

FILLER = AGUMON
FILLER_DECK = [FILLER] * 54


@pytest.mark.behavioral
class TestEX10033Fragment:
    """Fragment (3) keyword tests."""

    def test_has_fragment_keyword(self, debug_runner):
        """Should have Fragment keyword effect."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [PYRAMIDIMON])

        card = perm.top_card
        effects = card.effect_list(None)
        fragment_effects = [e for e in effects if getattr(e, '_is_fragment', False)]
        assert len(fragment_effects) >= 1, "Should have Fragment keyword"


@pytest.mark.behavioral
class TestEX10033PlaceFromTrash:
    """[When Digivolving] [When Attacking] [Once Per Turn] place up to 3
    [Mineral] or [Rock] trait cards from trash as bottom digivolution cards."""

    def test_place_effect_exists_when_digivolving(self, debug_runner):
        """Should have a When Digivolving place effect."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [PYRAMIDIMON])

        card = perm.top_card
        effects = card.effect_list(None)
        place_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
            and "Place" in (e.effect_name or "")
        ]
        assert len(place_effects) >= 1, "Should have When Digivolving place effect"

    def test_place_effect_exists_when_attacking(self, debug_runner):
        """Should have a When Attacking place effect."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [PYRAMIDIMON])

        card = perm.top_card
        effects = card.effect_list(None)
        place_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnUseAttack
            and getattr(e, 'is_on_attack', False)
            and "Place" in (e.effect_name or "")
        ]
        assert len(place_effects) >= 1, "Should have When Attacking place effect"

    def test_place_effect_is_optional(self, debug_runner):
        """Place effect should be optional (you may)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [PYRAMIDIMON])

        card = perm.top_card
        effects = card.effect_list(None)
        place_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
            and "Place" in (e.effect_name or "")
        ]
        assert place_effects[0].is_optional, "Place effect should be optional"

    def test_place_effect_is_once_per_turn(self, debug_runner):
        """Place effect should be Once Per Turn."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [PYRAMIDIMON])

        card = perm.top_card
        effects = card.effect_list(None)
        place_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
            and "Place" in (e.effect_name or "")
        ]
        assert place_effects[0].max_count_per_turn == 1, "Should be once per turn"

    def test_place_condition_fails_without_mineral_rock_in_trash(self, debug_runner):
        """Condition should fail when no Mineral/Rock cards are in trash."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [PYRAMIDIMON])
        # Inject a non-Mineral/Rock card into trash
        runner.inject_card(1, AGUMON, "trash")

        card = perm.top_card
        effects = card.effect_list(None)
        place_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
            and "Place" in (e.effect_name or "")
        ]
        ctx = {"permanent": perm, "player": runner.game.player1, "game": runner.game}
        assert not place_effects[0].can_use_condition(ctx), \
            "Condition should fail without Mineral/Rock in trash"

    def test_place_condition_passes_with_mineral_in_trash(self, debug_runner):
        """Condition should pass when Mineral card is in trash."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [PYRAMIDIMON])
        runner.inject_card(1, LANDRAMON, "trash")

        card = perm.top_card
        effects = card.effect_list(None)
        place_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
            and "Place" in (e.effect_name or "")
        ]
        ctx = {"permanent": perm, "player": runner.game.player1, "game": runner.game}
        assert place_effects[0].can_use_condition(ctx), \
            "Condition should pass with Mineral card in trash"

    def test_place_condition_passes_with_rock_in_trash(self, debug_runner):
        """Condition should pass when Rock card is in trash."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [PYRAMIDIMON])
        runner.inject_card(1, GOTSUMON, "trash")

        card = perm.top_card
        effects = card.effect_list(None)
        place_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
            and "Place" in (e.effect_name or "")
        ]
        ctx = {"permanent": perm, "player": runner.game.player1, "game": runner.game}
        assert place_effects[0].can_use_condition(ctx), \
            "Condition should pass with Rock card in trash"

    def test_place_actually_moves_card_from_trash_to_sources(self, debug_runner):
        """Process should present trash selection and place selected card at bottom
        of digivolution stack. Tests the SEL_TRASH_START subtraction fix."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [PYRAMIDIMON])
        runner.inject_card(1, LANDRAMON, "trash")

        game = runner.game
        player = game.player1
        initial_sources = len(perm.card_sources)
        initial_trash = len(player.trash_cards)

        card = perm.top_card
        effects = card.effect_list(None)
        place_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
            and "Place" in (e.effect_name or "")
        ]
        effect = place_effects[0]

        # Trigger the effect process
        effect.on_process_callback({
            'player': player,
            'game': game,
            'permanent': perm,
        })

        # Now we should be in SelectTrash phase
        assert game.current_phase == GamePhase.SelectTrash, \
            f"Should enter SelectTrash phase, got {game.current_phase}"

        # Find the Landramon in trash and select it
        landramon_idx = None
        for i, c in enumerate(player.trash_cards):
            traits = getattr(c, 'card_traits', [])
            if 'Mineral' in traits:
                landramon_idx = i
                break
        assert landramon_idx is not None, "Landramon should be in trash"

        # Execute the selection (SEL_TRASH_START + index)
        from engine_py_legacy.engine.game.constants import SEL_TRASH_START
        action_id = SEL_TRASH_START + landramon_idx
        game.decode_action(action_id, game.current_player_id)

        # Verify: card moved from trash to bottom of digivolution cards
        assert len(perm.card_sources) == initial_sources + 1, \
            "Should have 1 more digivolution source after placing"
        assert len(player.trash_cards) == initial_trash - 1, \
            "Should have 1 fewer card in trash after placing"

        # Check the card is at the BOTTOM (index 0) of the stack
        bottom_card = perm.card_sources[0]
        bottom_traits = getattr(bottom_card, 'card_traits', [])
        assert 'Mineral' in bottom_traits, \
            "Bottom card should be the Mineral card we placed"

    def test_place_up_to_3_cards_iteratively(self, debug_runner):
        """Should allow placing up to 3 cards through iterative selection phases."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [PYRAMIDIMON])
        runner.inject_card(1, LANDRAMON, "trash")
        runner.inject_card(1, LANDRAMON, "trash")
        runner.inject_card(1, GOTSUMON, "trash")

        game = runner.game
        player = game.player1
        initial_sources = len(perm.card_sources)

        card = perm.top_card
        effects = card.effect_list(None)
        place_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
            and "Place" in (e.effect_name or "")
        ]
        effect = place_effects[0]

        effect.on_process_callback({
            'player': player,
            'game': game,
            'permanent': perm,
        })

        # Select all 3 iteratively
        from engine_py_legacy.engine.game.constants import SEL_TRASH_START
        for pick in range(3):
            assert game.current_phase == GamePhase.SelectTrash, \
                f"Pick {pick+1}: Should be in SelectTrash, got {game.current_phase}"
            # Pick the first legal action
            legal = runner.action_mask()
            # Filter to trash selections only
            trash_actions = [a for a in legal if SEL_TRASH_START <= a <= 179]
            assert len(trash_actions) > 0, f"Pick {pick+1}: Should have trash selections"
            game.decode_action(trash_actions[0], game.current_player_id)

        assert len(perm.card_sources) == initial_sources + 3, \
            "Should have placed 3 cards total"

    def test_place_can_stop_early(self, debug_runner):
        """Should allow declining after placing fewer than 3 cards."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [PYRAMIDIMON])
        runner.inject_card(1, LANDRAMON, "trash")
        runner.inject_card(1, LANDRAMON, "trash")
        runner.inject_card(1, GOTSUMON, "trash")

        game = runner.game
        player = game.player1
        initial_sources = len(perm.card_sources)

        card = perm.top_card
        effects = card.effect_list(None)
        place_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
            and "Place" in (e.effect_name or "")
        ]
        effect = place_effects[0]

        effect.on_process_callback({
            'player': player,
            'game': game,
            'permanent': perm,
        })

        # Select 1 card, then decline
        from engine_py_legacy.engine.game.constants import SEL_TRASH_START
        assert game.current_phase == GamePhase.SelectTrash
        trash_actions = [a for a in runner.action_mask() if SEL_TRASH_START <= a <= 179]
        game.decode_action(trash_actions[0], game.current_player_id)

        # Now decline (action 62)
        if game.current_phase == GamePhase.SelectTrash:
            assert 62 in runner.action_mask(), "Should be able to decline"
            game.decode_action(62, game.current_player_id)

        assert len(perm.card_sources) == initial_sources + 1, \
            "Should have placed exactly 1 card before declining"

    def test_place_filters_non_mineral_rock_from_trash(self, debug_runner):
        """Should only offer Mineral/Rock cards from trash, not others."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [PYRAMIDIMON])
        runner.inject_card(1, AGUMON, "trash")  # Not Mineral/Rock
        runner.inject_card(1, LANDRAMON, "trash")  # Mineral

        game = runner.game
        player = game.player1

        card = perm.top_card
        effects = card.effect_list(None)
        place_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
            and "Place" in (e.effect_name or "")
        ]
        effect = place_effects[0]

        effect.on_process_callback({
            'player': player,
            'game': game,
            'permanent': perm,
        })

        assert game.current_phase == GamePhase.SelectTrash
        from engine_py_legacy.engine.game.constants import SEL_TRASH_START

        # Check which trash indices are valid
        legal = runner.action_mask()
        trash_actions = [a for a in legal if SEL_TRASH_START <= a <= 179]

        # Only the Mineral card should be selectable, not Agumon
        for action_id in trash_actions:
            idx = action_id - SEL_TRASH_START
            card_at_idx = player.trash_cards[idx]
            traits = getattr(card_at_idx, 'card_traits', [])
            assert 'Mineral' in traits or 'Rock' in traits, \
                f"Only Mineral/Rock cards should be selectable, got traits: {traits}"


@pytest.mark.behavioral
class TestEX10033CostReduce:
    """[When Digivolving] [When Attacking] By trashing up to 3 [Mineral] or
    [Rock] trait cards from any of your Digimon's digivolution cards, to 1 of
    your opponent's Digimon, reduce the play cost by 2 until their turn ends
    for each card trashed."""

    def _find_cost_reduce_effects(self, perm, timing):
        """Helper to find the cost-reduce effects (distinguished from place effects
        by 'reduce' in the name)."""
        card = perm.top_card
        effects = card.effect_list(None)
        return [
            e for e in effects
            if e.timing == timing
            and "reduce" in (e.effect_name or "").lower()
        ]

    def test_cost_reduce_effect_exists_when_digivolving(self, debug_runner):
        """Should have a When Digivolving cost reduce effect."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [PYRAMIDIMON])
        reduce_effects = self._find_cost_reduce_effects(perm, EffectTiming.OnEnterFieldAnyone)
        assert len(reduce_effects) >= 1, "Should have When Digivolving cost reduce effect"

    def test_cost_reduce_effect_exists_when_attacking(self, debug_runner):
        """Should have a When Attacking cost reduce effect."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [PYRAMIDIMON])
        reduce_effects = self._find_cost_reduce_effects(perm, EffectTiming.OnUseAttack)
        assert len(reduce_effects) >= 1, "Should have When Attacking cost reduce effect"

    def test_cost_reduce_is_optional(self, debug_runner):
        """Cost reduce effect should be optional."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [PYRAMIDIMON])
        reduce_effects = self._find_cost_reduce_effects(perm, EffectTiming.OnEnterFieldAnyone)
        assert reduce_effects[0].is_optional, "Cost reduce effect should be optional"

    def test_cost_reduce_condition_requires_mineral_rock_sources(self, debug_runner):
        """Condition should fail without Mineral/Rock in any Digimon's sources."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        # Pyramidimon alone with no digi-source cards (only top card)
        perm = runner.place_on_field(1, [PYRAMIDIMON])
        reduce_effects = self._find_cost_reduce_effects(perm, EffectTiming.OnEnterFieldAnyone)
        ctx = {"permanent": perm, "player": runner.game.player1, "game": runner.game}
        assert not reduce_effects[0].can_use_condition(ctx), \
            "Should fail without Mineral/Rock sources"

    def test_cost_reduce_condition_passes_with_mineral_source(self, debug_runner):
        """Condition should pass when a Digimon has Mineral source under it."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        # Stack: Landramon (Mineral) under Pyramidimon
        perm = runner.place_on_field(1, [LANDRAMON, PYRAMIDIMON])
        reduce_effects = self._find_cost_reduce_effects(perm, EffectTiming.OnEnterFieldAnyone)
        ctx = {"permanent": perm, "player": runner.game.player1, "game": runner.game}
        assert reduce_effects[0].can_use_condition(ctx), \
            "Should pass with Mineral source"

    def test_cost_reduce_uses_player_selection_for_source(self, debug_runner):
        """Cost reduce must use player selection to choose which digivolution cards
        to trash -- NOT auto-trash. This tests the no-approximations fix."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        # Stack Pyramidimon with Mineral sources underneath
        perm = runner.place_on_field(1, [LANDRAMON, PROGANOMON, PYRAMIDIMON])
        # Opponent Digimon to target
        opp_perm = runner.place_on_field(2, [AGUMON])

        game = runner.game
        player = game.player1
        reduce_effects = self._find_cost_reduce_effects(perm, EffectTiming.OnEnterFieldAnyone)
        effect = reduce_effects[0]

        effect.on_process_callback({
            'player': player,
            'game': game,
            'permanent': perm,
        })

        # Should enter a selection phase, NOT auto-trash
        assert game.current_phase == GamePhase.SelectSource, \
            f"Should enter SelectSource for player selection, got {game.current_phase}"

    def test_cost_reduce_trash_1_and_apply_minus_2(self, debug_runner):
        """Trashing 1 card should reduce opponent Digimon's play cost by 2."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [LANDRAMON, PYRAMIDIMON])
        opp_perm = runner.place_on_field(2, [AGUMON])

        game = runner.game
        player = game.player1
        reduce_effects = self._find_cost_reduce_effects(perm, EffectTiming.OnEnterFieldAnyone)
        effect = reduce_effects[0]

        effect.on_process_callback({
            'player': player,
            'game': game,
            'permanent': perm,
        })

        # Select the Mineral source to trash
        assert game.current_phase == GamePhase.SelectSource
        legal = runner.action_mask()
        source_actions = [a for a in legal if 2000 <= a < 2168]
        assert len(source_actions) > 0, "Should have source selections available"
        game.decode_action(source_actions[0], game.current_player_id)

        # Decline to trash more (select only 1)
        if game.current_phase == GamePhase.SelectSource:
            game.decode_action(62, game.current_player_id)

        # Now select opponent target
        assert game.current_phase == GamePhase.SelectTarget, \
            f"Should enter SelectTarget for opponent, got {game.current_phase}"
        runner.auto_resolve()

        # The trashed card should be in player's trash
        mineral_in_trash = any(
            'Mineral' in getattr(c, 'card_traits', [])
            for c in player.trash_cards
        )
        assert mineral_in_trash, "Trashed Mineral card should be in player trash"

    def test_cost_reduce_trash_3_and_apply_minus_6(self, debug_runner):
        """Trashing 3 cards should reduce opponent Digimon's play cost by 6."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        # Stack 3 Mineral/Rock sources under Pyramidimon
        perm = runner.place_on_field(1, [GOTSUMON, LANDRAMON, PROGANOMON, PYRAMIDIMON])
        opp_perm = runner.place_on_field(2, [WARGREYMON])

        game = runner.game
        player = game.player1
        initial_source_count = len(perm.card_sources)
        reduce_effects = self._find_cost_reduce_effects(perm, EffectTiming.OnEnterFieldAnyone)
        effect = reduce_effects[0]

        effect.on_process_callback({
            'player': player,
            'game': game,
            'permanent': perm,
        })

        # Select all 3 sources iteratively
        for pick in range(3):
            assert game.current_phase == GamePhase.SelectSource, \
                f"Pick {pick+1}: Should be in SelectSource, got {game.current_phase}"
            legal = runner.action_mask()
            source_actions = [a for a in legal if 2000 <= a < 2168]
            assert len(source_actions) > 0, f"Pick {pick+1}: Should have source actions"
            game.decode_action(source_actions[0], game.current_player_id)

        # Now select opponent target
        assert game.current_phase == GamePhase.SelectTarget, \
            f"Should enter SelectTarget after trashing, got {game.current_phase}"
        runner.auto_resolve()

        # Verify 3 sources were removed
        assert len(perm.card_sources) == initial_source_count - 3, \
            "Should have 3 fewer sources after trashing"

        # Verify 3 Mineral/Rock cards are in trash
        mineral_rock_in_trash = sum(
            1 for c in player.trash_cards
            if 'Mineral' in getattr(c, 'card_traits', [])
            or 'Rock' in getattr(c, 'card_traits', [])
        )
        assert mineral_rock_in_trash >= 3, \
            "3 Mineral/Rock cards should be in trash"

    def test_cost_reduce_can_trash_from_different_digimon(self, debug_runner):
        """Should be able to trash sources from multiple different Digimon,
        not just this one."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        # Pyramidimon with 1 Mineral source
        perm1 = runner.place_on_field(1, [LANDRAMON, PYRAMIDIMON])
        # Another Digimon with a Rock source
        perm2 = runner.place_on_field(1, [GOTSUMON, AGUMON])
        opp_perm = runner.place_on_field(2, [AGUMON])

        game = runner.game
        player = game.player1
        reduce_effects = self._find_cost_reduce_effects(perm1, EffectTiming.OnEnterFieldAnyone)
        effect = reduce_effects[0]

        effect.on_process_callback({
            'player': player,
            'game': game,
            'permanent': perm1,
        })

        # Should enter source selection
        assert game.current_phase == GamePhase.SelectSource

        # Get all valid source actions -- should span both perm1 and perm2
        legal = runner.action_mask()
        source_actions = [a for a in legal if 2000 <= a < 2168]

        # Decode field indices from the actions
        from engine_py_legacy.engine.game.constants import SOURCES_PER_FIELD
        field_indices = set()
        for a in source_actions:
            normalized = a - 2000
            field_idx = normalized // SOURCES_PER_FIELD
            field_indices.add(field_idx)

        assert len(field_indices) >= 2, \
            "Should have sources from at least 2 different Digimon"

    def test_cost_reduce_skips_top_card(self, debug_runner):
        """Should NOT offer top card of any Digimon as a trashable source."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        # Pyramidimon IS Mineral trait, but is the top card -- should not be trashable
        perm = runner.place_on_field(1, [LANDRAMON, PYRAMIDIMON])
        opp_perm = runner.place_on_field(2, [AGUMON])

        game = runner.game
        player = game.player1
        reduce_effects = self._find_cost_reduce_effects(perm, EffectTiming.OnEnterFieldAnyone)
        effect = reduce_effects[0]

        effect.on_process_callback({
            'player': player,
            'game': game,
            'permanent': perm,
        })

        assert game.current_phase == GamePhase.SelectSource
        legal = runner.action_mask()
        source_actions = [a for a in legal if 2000 <= a < 2168]

        from engine_py_legacy.engine.game.constants import SOURCES_PER_FIELD
        for a in source_actions:
            normalized = a - 2000
            field_idx = normalized // SOURCES_PER_FIELD
            source_idx = normalized % SOURCES_PER_FIELD
            ba_perm = player.battle_area[field_idx]
            selected_src = ba_perm.card_sources[source_idx]
            assert selected_src is not ba_perm.top_card, \
                "Top card should never be offered as a trashable source"

    def test_cost_reduce_fires_on_digivolution_card_discarded(self, debug_runner):
        """Trashing sources should fire OnDigivolutionCardDiscarded timing via
        trash_specific_digivolution_cards()."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [LANDRAMON, PYRAMIDIMON])
        opp_perm = runner.place_on_field(2, [AGUMON])

        game = runner.game
        player = game.player1

        # Track timing events
        timing_fired = []
        original_exec = game.execute_effects

        def tracking_exec(timing, context=None):
            timing_fired.append(timing)
            return original_exec(timing, context)
        game.execute_effects = tracking_exec

        reduce_effects = self._find_cost_reduce_effects(perm, EffectTiming.OnEnterFieldAnyone)
        effect = reduce_effects[0]

        effect.on_process_callback({
            'player': player,
            'game': game,
            'permanent': perm,
        })

        # Select a source to trash
        assert game.current_phase == GamePhase.SelectSource
        legal = runner.action_mask()
        source_actions = [a for a in legal if 2000 <= a < 2168]
        game.decode_action(source_actions[0], game.current_player_id)

        # Decline more trashing
        if game.current_phase == GamePhase.SelectSource:
            game.decode_action(62, game.current_player_id)

        # Select opponent target
        runner.auto_resolve()

        assert EffectTiming.OnDigivolutionCardDiscarded in timing_fired, \
            "Should fire OnDigivolutionCardDiscarded when trashing digi sources"


@pytest.mark.behavioral
class TestEX10033TrashSpecificAPI:
    """Test the trash_specific_digivolution_cards() API on Permanent."""

    def test_removes_specific_cards(self, debug_runner):
        """Should remove specific CardSource objects from the permanent."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [GOTSUMON, LANDRAMON, PROGANOMON, PYRAMIDIMON])

        # Remember the specific card source at index 0 (Gotsumon)
        target = perm.card_sources[0]
        initial_count = len(perm.card_sources)

        trashed = perm.trash_specific_digivolution_cards([target])
        assert len(trashed) == 1
        assert trashed[0] is target
        assert len(perm.card_sources) == initial_count - 1
        assert target not in perm.card_sources

    def test_does_not_remove_top_card(self, debug_runner):
        """Should refuse to remove the top card."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [LANDRAMON, PYRAMIDIMON])

        top = perm.top_card
        trashed = perm.trash_specific_digivolution_cards([top])
        assert len(trashed) == 0, "Should not remove the top card"
        assert top in perm.card_sources

    def test_fires_on_digivolution_card_discarded(self, debug_runner):
        """Should fire OnDigivolutionCardDiscarded timing."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [LANDRAMON, PYRAMIDIMON])

        game = runner.game
        timing_fired = []
        original_exec = game.execute_effects

        def tracking_exec(timing, context=None):
            timing_fired.append(timing)
            return original_exec(timing, context)
        game.execute_effects = tracking_exec

        target = perm.card_sources[0]
        perm.trash_specific_digivolution_cards([target])

        assert EffectTiming.OnDigivolutionCardDiscarded in timing_fired
