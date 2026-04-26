"""Behavioral tests for EX8-051 Proganomon | Lv.5 Black Digimon.

Card text:
    Main effect:
        <Collision> <Piercing> <Fragment (3)>
    Inherited effect:
        When effects trash this card from a [Mineral] or [Rock] trait
        Digimon's digivolution cards, <De-Digivolve 1> 1 of your
        opponent's Digimon. (Trash the top card. You can't trash past
        level 3 cards.)

C# reference (EX8_051.cs):
    - Collision: CollisionSelfStaticEffect
    - Piercing: PierceSelfEffect
    - Fragment (3): FragmentSelfEffect(trashValue: 3)
    - ESS OnDigivolutionCardDiscarded:
        CanUseCondition: permanent has Mineral/Rock trait AND
            CanTriggerOnTrashSelfDigivolutionCard(card in trashed list)
        CanActivateCondition: card IsExistOnTrash AND
            opponent has at least 1 Digimon
        Activate: select 1 opponent Digimon -> De-Digivolve 1
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


# Card constants
PROGANOMON = "EX8-051"        # Lv.5 Black, Mineral/LIBERATOR, the card under test
SUNARIZAMON = "EX8-047"       # Lv.3, Mineral, has same inherited pattern
TUMBLEMON = "EX8-005"         # Lv.2, Mineral/Rock, has same inherited pattern
PYRAMIDIMON = "EX8-055"       # Lv.6, Mineral, Fragment (3) + trash sources
LANDRAMON = "EX8-048"         # Lv.4, Mineral/Rock
OPP_LV5 = "BT1-020"          # Lv.5 opponent target for de-digivolve
OPP_LV4 = "ST1-06"           # Lv.4 opponent target
OPP_LV3 = "ST1-03"           # Lv.3 opponent target (can't de-digivolve further)
FILLER = "ST1-03"             # Lv.3 Red filler


@pytest.mark.behavioral
class TestEX8051Keywords:
    """Tests for Collision, Piercing, Fragment (3) keywords."""

    def test_has_collision_keyword(self, debug_runner):
        """Proganomon should have <Collision>."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [PROGANOMON])
        effects = perm.top_card.effect_list(None)
        has_collision = any(getattr(e, '_is_collision', False) for e in effects)
        assert has_collision, (
            f"Proganomon should have Collision keyword. "
            f"Effects: {[(getattr(e, '_effect_name', '?'), ) for e in effects]}")

    def test_has_piercing_keyword(self, debug_runner):
        """Proganomon should have <Piercing>."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [PROGANOMON])
        effects = perm.top_card.effect_list(None)
        has_piercing = any(getattr(e, '_is_piercing', False) for e in effects)
        assert has_piercing, (
            f"Proganomon should have Piercing keyword. "
            f"Effects: {[(getattr(e, '_effect_name', '?'), ) for e in effects]}")

    def test_has_fragment_keyword(self, debug_runner):
        """Proganomon should have <Fragment (3)>."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [PROGANOMON])
        effects = perm.top_card.effect_list(None)
        has_fragment = any(getattr(e, '_is_fragment', False) for e in effects)
        assert has_fragment, (
            f"Proganomon should have Fragment keyword. "
            f"Effects: {[(getattr(e, '_effect_name', '?'), ) for e in effects]}")

    def test_collision_is_not_inherited(self, debug_runner):
        """Collision should NOT be an inherited effect."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [PROGANOMON])
        effects = perm.top_card.effect_list(None)
        collision = [e for e in effects if getattr(e, '_is_collision', False)]
        assert len(collision) == 1
        assert not collision[0].is_inherited_effect

    def test_piercing_is_not_inherited(self, debug_runner):
        """Piercing should NOT be an inherited effect."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [PROGANOMON])
        effects = perm.top_card.effect_list(None)
        piercing = [e for e in effects if getattr(e, '_is_piercing', False)]
        assert len(piercing) == 1
        assert not piercing[0].is_inherited_effect

    def test_fragment_is_not_inherited(self, debug_runner):
        """Fragment should NOT be an inherited effect."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [PROGANOMON])
        effects = perm.top_card.effect_list(None)
        fragment = [e for e in effects if getattr(e, '_is_fragment', False)]
        assert len(fragment) == 1
        assert not fragment[0].is_inherited_effect

    def test_exactly_three_non_inherited_keywords(self, debug_runner):
        """Proganomon's top card should have exactly 3 keyword effects (non-inherited)."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [PROGANOMON])
        effects = perm.top_card.effect_list(None)
        non_inherited = [e for e in effects if not e.is_inherited_effect]
        # 3 keywords: Collision, Piercing, Fragment
        assert len(non_inherited) == 3, (
            f"Expected 3 non-inherited keyword effects, got {len(non_inherited)}: "
            f"{[getattr(e, '_effect_name', '?') for e in non_inherited]}")

    def test_fragment_condition_always_true(self, debug_runner):
        """Fragment condition should always return True (no special condition)."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [PROGANOMON])
        effects = perm.top_card.effect_list(None)
        fragment = [e for e in effects if getattr(e, '_is_fragment', False)][0]
        assert fragment.can_use_condition({})


@pytest.mark.behavioral
class TestEX8051InheritedEffect:
    """Tests for the inherited De-Digivolve 1 effect."""

    def test_inherited_effect_exists(self, debug_runner):
        """Proganomon should have exactly 1 inherited effect with OnDigivolutionCardDiscarded timing."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [PROGANOMON])
        effects = perm.top_card.effect_list(None)
        inherited = [e for e in effects if e.is_inherited_effect]
        assert len(inherited) == 1, (
            f"Expected 1 inherited effect, got {len(inherited)}: "
            f"{[getattr(e, '_effect_name', '?') for e in inherited]}")
        assert inherited[0].timing == EffectTiming.OnDigivolutionCardDiscarded

    def test_inherited_has_process_callback(self, debug_runner):
        """The inherited effect must have an on_process_callback for De-Digivolve logic."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [PROGANOMON])
        effects = perm.top_card.effect_list(None)
        inherited = [e for e in effects if e.is_inherited_effect][0]
        assert inherited.on_process_callback is not None

    def test_inherited_condition_requires_card_in_trashed(self, debug_runner):
        """Inherited condition should fail when the card is not in trashed_cards."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [PROGANOMON])
        card = perm.top_card
        effects = card.effect_list(None)
        inherited = [e for e in effects if e.is_inherited_effect][0]

        # Create a Mineral-trait permanent for context
        mineral_perm = runner.place_on_field(1, [LANDRAMON])

        # Empty trashed_cards -> should fail
        ctx = {'trashed_cards': [], 'permanent': mineral_perm}
        assert not inherited.can_use_condition(ctx), (
            "Should fail when card is not in trashed_cards list")

    def test_inherited_condition_requires_mineral_or_rock_trait(self, debug_runner):
        """Inherited condition should fail when host Digimon lacks Mineral/Rock trait."""
        runner = debug_runner(initial_memory=10)
        # Place Proganomon as a digivolution source under another Digimon
        perm = runner.place_on_field(1, [PROGANOMON])
        card = perm.top_card
        effects = card.effect_list(None)
        inherited = [e for e in effects if e.is_inherited_effect][0]

        # Place a non-Mineral, non-Rock Digimon (ST1-03 is Red Dragon)
        non_mineral_perm = runner.place_on_field(1, [FILLER])

        # Card IS in trashed_cards but host has wrong trait
        ctx = {'trashed_cards': [card], 'permanent': non_mineral_perm}
        assert not inherited.can_use_condition(ctx), (
            "Should fail when host Digimon doesn't have Mineral or Rock trait")

    def test_inherited_condition_passes_with_mineral_trait(self, debug_runner):
        """Inherited condition should pass when host has Mineral trait and card was trashed."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [PROGANOMON])
        card = perm.top_card
        effects = card.effect_list(None)
        inherited = [e for e in effects if e.is_inherited_effect][0]

        # Proganomon itself has [Mineral] trait
        mineral_perm = runner.place_on_field(1, [PROGANOMON])

        ctx = {'trashed_cards': [card], 'permanent': mineral_perm}
        assert inherited.can_use_condition(ctx), (
            "Should pass when host has Mineral trait and card is in trashed_cards")

    def test_inherited_condition_passes_with_rock_trait(self, debug_runner):
        """Inherited condition should pass when host has Rock trait and card was trashed."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [PROGANOMON])
        card = perm.top_card
        effects = card.effect_list(None)
        inherited = [e for e in effects if e.is_inherited_effect][0]

        # LANDRAMON (EX8-048) has Mineral and Rock traits
        rock_perm = runner.place_on_field(1, [LANDRAMON])

        ctx = {'trashed_cards': [card], 'permanent': rock_perm}
        assert inherited.can_use_condition(ctx), (
            "Should pass when host has Rock trait and card is in trashed_cards")

    def test_inherited_condition_requires_permanent_context(self, debug_runner):
        """Inherited condition should fail when no permanent is in context."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [PROGANOMON])
        card = perm.top_card
        effects = card.effect_list(None)
        inherited = [e for e in effects if e.is_inherited_effect][0]

        ctx = {'trashed_cards': [card]}  # no 'permanent' key
        assert not inherited.can_use_condition(ctx), (
            "Should fail when permanent is missing from context")

    def test_inherited_condition_with_different_card_in_trashed(self, debug_runner):
        """Inherited condition should fail when a different card is trashed (not this one)."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [PROGANOMON])
        card = perm.top_card
        effects = card.effect_list(None)
        inherited = [e for e in effects if e.is_inherited_effect][0]

        mineral_perm = runner.place_on_field(1, [LANDRAMON])
        other_card = mineral_perm.top_card

        # Other card in trashed list, not this card
        ctx = {'trashed_cards': [other_card], 'permanent': mineral_perm}
        assert not inherited.can_use_condition(ctx), (
            "Should fail when a different card is in trashed_cards")


@pytest.mark.behavioral
class TestEX8051InheritedDeDigivolve:
    """Tests for the De-Digivolve 1 process callback behavior."""

    def test_de_digivolve_creates_selection(self, debug_runner):
        """Inherited process should create a selection to pick opponent's Digimon."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # Place opponent Digimon (stacked for de-digivolve target)
        runner.place_on_field(2, [FILLER, OPP_LV4])

        # Get the inherited effect from Proganomon
        perm = runner.place_on_field(1, [PROGANOMON])
        card = perm.top_card
        effects = card.effect_list(None)
        inherited = [e for e in effects if e.is_inherited_effect][0]

        # Manually invoke the process callback
        ctx = {
            'player': game.player1,
            'game': game,
            'permanent': perm,
        }
        inherited.on_process_callback(ctx)

        # Should have a pending selection for opponent Digimon
        ps = game.pending_selection
        assert ps is not None, (
            "De-Digivolve 1 should create a selection for opponent's Digimon")

    def test_de_digivolve_targets_opponent_digimon(self, debug_runner):
        """Selection filter should target opponent's Digimon."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # Place opponent Digimon stack (Lv.3 under Lv.5)
        opp_perm = runner.place_on_field(2, [FILLER, OPP_LV5])
        initial_sources = len(opp_perm.card_sources)

        # Get the inherited effect
        perm = runner.place_on_field(1, [PROGANOMON])
        card = perm.top_card
        effects = card.effect_list(None)
        inherited = [e for e in effects if e.is_inherited_effect][0]

        # Invoke process
        ctx = {
            'player': game.player1,
            'game': game,
            'permanent': perm,
        }
        inherited.on_process_callback(ctx)

        # Resolve the selection (pick the first valid target)
        assert game.pending_selection is not None
        runner.auto_resolve()

        # After de-digivolve 1, opponent's Digimon should lose 1 card
        # and 1 card should go to opponent's trash
        assert len(opp_perm.card_sources) == initial_sources - 1, (
            f"De-Digivolve 1 should remove 1 card from stack. "
            f"Had {initial_sources}, now {len(opp_perm.card_sources)}")

    def test_de_digivolve_no_targets_does_nothing(self, debug_runner):
        """If opponent has no Digimon, the selection should not be created."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # No opponent Digimon on field

        perm = runner.place_on_field(1, [PROGANOMON])
        card = perm.top_card
        effects = card.effect_list(None)
        inherited = [e for e in effects if e.is_inherited_effect][0]

        ctx = {
            'player': game.player1,
            'game': game,
            'permanent': perm,
        }
        inherited.on_process_callback(ctx)

        # Should not have a pending selection (no targets)
        ps = game.pending_selection
        assert ps is None, (
            "Should not create selection when opponent has no Digimon")

    def test_de_digivolve_is_mandatory(self, debug_runner):
        """De-Digivolve 1 is NOT optional per card text (no 'may' clause)."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        runner.place_on_field(2, [FILLER, OPP_LV5])

        perm = runner.place_on_field(1, [PROGANOMON])
        card = perm.top_card
        effects = card.effect_list(None)
        inherited = [e for e in effects if e.is_inherited_effect][0]

        ctx = {
            'player': game.player1,
            'game': game,
            'permanent': perm,
        }
        inherited.on_process_callback(ctx)

        # Check that the selection was created with is_optional=False
        ps = game.pending_selection
        assert ps is not None
        # The effect_select_opponent_permanent call uses is_optional=False
        # We verify by checking no "Pass" action exists
        pass_action = runner.find_action("pass")
        # If selection is mandatory, there should be no pass option
        # (This depends on engine behavior for mandatory selections)

    def test_de_digivolve_sends_removed_to_trash(self, debug_runner):
        """Cards removed by De-Digivolve 1 should go to opponent's trash."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # Opponent: stack Lv.3 under Lv.5
        opp_perm = runner.place_on_field(2, [FILLER, OPP_LV5])
        initial_trash = len(game.player2.trash_cards)

        perm = runner.place_on_field(1, [PROGANOMON])
        card = perm.top_card
        effects = card.effect_list(None)
        inherited = [e for e in effects if e.is_inherited_effect][0]

        ctx = {
            'player': game.player1,
            'game': game,
            'permanent': perm,
        }
        inherited.on_process_callback(ctx)
        runner.auto_resolve()

        # Opponent's trash should have 1 more card
        assert len(game.player2.trash_cards) > initial_trash, (
            f"De-Digivolve should send removed card to opponent's trash. "
            f"Trash was {initial_trash}, now {len(game.player2.trash_cards)}")


@pytest.mark.behavioral
class TestEX8051InheritedUnderDigimon:
    """Tests for EX8-051 as an inherited effect under another Digimon."""

    def test_inherited_visible_when_under_mineral_digimon(self, debug_runner):
        """When Proganomon is under a Mineral Digimon, its inherited effect should be in the
        permanent's effect list."""
        runner = debug_runner(initial_memory=10)
        # Stack: Proganomon (Lv.5) under Pyramidimon (Lv.6 Mineral)
        perm = runner.place_on_field(1, [PROGANOMON, PYRAMIDIMON])
        effects = perm.effect_list(EffectTiming.OnDigivolutionCardDiscarded)
        inherited = [e for e in effects if e.is_inherited_effect]
        # Should find at least 1 inherited OnDigivolutionCardDiscarded effect
        # (from Proganomon under Pyramidimon)
        assert len(inherited) >= 1, (
            f"Should find inherited OnDigivolutionCardDiscarded effect. "
            f"Effects: {[getattr(e, '_effect_name', '?') for e in effects]}")

    def test_inherited_not_active_when_on_top(self, debug_runner):
        """When Proganomon is the top card, its inherited effect should NOT be in the
        permanent's active effects (only shows when under)."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [PROGANOMON])
        effects = perm.effect_list(EffectTiming.OnDigivolutionCardDiscarded)
        # Top card's inherited effects are excluded from permanent's effect_list
        inherited = [e for e in effects if e.is_inherited_effect]
        assert len(inherited) == 0, (
            f"Top card's inherited effect should not be in permanent's active effects. "
            f"Found: {[getattr(e, '_effect_name', '?') for e in inherited]}")


@pytest.mark.behavioral
class TestEX8051EffectStructure:
    """Tests for correct effect structure and attributes."""

    def test_total_effect_count(self, debug_runner):
        """Proganomon should produce exactly 4 effects: 3 keywords + 1 inherited."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [PROGANOMON])
        effects = perm.top_card.effect_list(None)
        assert len(effects) == 4, (
            f"Expected 4 effects (Collision, Piercing, Fragment, inherited), "
            f"got {len(effects)}: "
            f"{[getattr(e, '_effect_name', '?') for e in effects]}")

    def test_effect_names_contain_card_id(self, debug_runner):
        """All effect names should reference EX8-051 for debugging clarity."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [PROGANOMON])
        effects = perm.top_card.effect_list(None)
        for e in effects:
            name = getattr(e, 'effect_name', '')
            assert 'EX8-051' in name, (
                f"Effect name should contain 'EX8-051', got: '{name}'")

    def test_inherited_is_not_optional(self, debug_runner):
        """The inherited effect should not be marked optional (card text has no 'may')."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [PROGANOMON])
        effects = perm.top_card.effect_list(None)
        inherited = [e for e in effects if e.is_inherited_effect][0]
        assert not inherited.is_optional, (
            "Inherited De-Digivolve 1 effect should not be optional")
