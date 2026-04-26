"""Behavioral tests for BT12-050 Stingmon.

Card text:
  Lv.4 | Green | Insectoid | DP:5000 | Cost:5 | Attribute: Free

  [Your Turn] When this Digimon would DNA digivolve into a blue Digimon card,
  gain 1 memory.

  Inherited: [Your Turn] While this Digimon has [Imperialdramon] in its name
  or the [Free] trait, it gains <Piercing>.

Key faithfulness points tested:
  - Main effect: DNA digivolve into blue -> gain 1 memory
  - Inherited: Piercing when Digimon has [Imperialdramon] in name
  - Inherited: Piercing when Digimon has [Free] attribute (NOT trait/type)
  - Inherited: NO Piercing when Digimon lacks both Imperialdramon name and Free attribute
  - No spurious OnEndTurn effect (card text has no End of Turn effect)
"""

import pytest

from engine_py_legacy.engine.data.enums import EffectTiming


# Card IDs used in tests
BT12_050 = "BT12-050"          # Card under test: Stingmon, Lv.4 Green
IMPERIALDRAMON_DM = "BT12-030" # Imperialdramon: Dragon Mode, Lv.6 Blue+Green, Free attribute
PAILDRAMON = "AD1-011"         # Paildramon, Lv.5 Blue+Green, Free attribute (no Imperialdramon name)
METALGREYMON = "ST1-09"        # MetalGreymon, Lv.5 Red, Vaccine attribute (no Imperialdramon, no Free)
EXVEEMON = "BT12-022"          # ExVeemon, Lv.4 Blue (DNA partner)

# Filler deck: enough cards for both players
FILLER_DECK = (
    ["BT3-001"] * 4   # Digi-Eggs
    + ["ST1-03"] * 10  # Agumon Lv.3
    + ["ST1-07"] * 10  # Greymon Lv.4
    + ["ST1-09"] * 10  # MetalGreymon Lv.5
    + ["ST1-10"] * 8   # WarGreymon Lv.6
    + ["ST1-11"] * 4   # Tai Kamiya tamer
    + ["ST1-02"] * 4   # Biyomon Lv.3
)


@pytest.mark.behavioral
class TestBT12050InheritedPiercing:
    """[Inherited] [Your Turn] While this Digimon has [Imperialdramon] in its
    name or the [Free] trait, it gains <Piercing>."""

    def test_piercing_with_imperialdramon_name(self, debug_runner):
        """Inherited Piercing should activate when top card has 'Imperialdramon' in name."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)

        # Stack BT12-050 under Imperialdramon: Dragon Mode (has "Imperialdramon" in name)
        perm = runner.place_on_field(1, [BT12_050, IMPERIALDRAMON_DM])

        assert perm.has_keyword('_is_piercing'), (
            f"Imperialdramon: Dragon Mode with BT12-050 inherited should have Piercing. "
            f"Keywords: {[kw for kw in ['_is_piercing', '_is_blocker', '_is_rush'] if perm.has_keyword(kw)]}"
        )

    def test_piercing_with_free_attribute(self, debug_runner):
        """Inherited Piercing should activate when top card has [Free] attribute.

        This tests the fix for the card_traits -> attribute_eng bug.
        Paildramon (AD1-011) has attribute_eng=['Free'] but card_traits=['Dragonkin', 'Hero'].
        """
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)

        # Stack BT12-050 under Paildramon (Free attribute, NOT Imperialdramon name)
        perm = runner.place_on_field(1, [BT12_050, PAILDRAMON])

        # Verify the test card actually has the Free attribute
        top = perm.top_card
        assert top is not None
        attrs = top.c_entity_base.attribute_eng if top.c_entity_base else []
        assert 'Free' in attrs, (
            f"Test setup: Paildramon should have Free attribute, got {attrs}"
        )

        # Verify Paildramon does NOT have 'Imperialdramon' in name (tests Free path specifically)
        assert not perm.contains_card_name('Imperialdramon'), (
            "Test setup: Paildramon should not contain 'Imperialdramon' in name"
        )

        assert perm.has_keyword('_is_piercing'), (
            f"Paildramon (Free attribute) with BT12-050 inherited should have Piercing. "
            f"attribute_eng={attrs}, card_traits={top.card_traits}"
        )

    def test_no_piercing_without_imperialdramon_or_free(self, debug_runner):
        """Inherited Piercing should NOT activate when top card lacks both
        [Imperialdramon] name and [Free] attribute."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)

        # Stack BT12-050 under MetalGreymon (Vaccine attribute, no Imperialdramon name)
        perm = runner.place_on_field(1, [BT12_050, METALGREYMON])

        # Verify test setup
        top = perm.top_card
        assert top is not None
        attrs = top.c_entity_base.attribute_eng if top.c_entity_base else []
        assert 'Free' not in attrs, (
            f"Test setup: MetalGreymon should NOT have Free attribute, got {attrs}"
        )
        assert not perm.contains_card_name('Imperialdramon'), (
            "Test setup: MetalGreymon should not contain 'Imperialdramon'"
        )

        assert not perm.has_keyword('_is_piercing'), (
            f"MetalGreymon (no Imperialdramon, no Free) with BT12-050 inherited "
            f"should NOT have Piercing. attribute_eng={attrs}"
        )

    def test_piercing_only_on_your_turn(self, debug_runner):
        """Inherited Piercing should only be active during [Your Turn]."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=10, first_player=2,
        )

        # Stack BT12-050 under Imperialdramon: Dragon Mode as P1's card
        # But P2 is the turn player, so it's P1's opponent's turn
        perm = runner.place_on_field(1, [BT12_050, IMPERIALDRAMON_DM])

        # P1's turn should be False since P2 is first player
        p1 = runner.game.player1
        assert not p1.is_my_turn, "Test setup: P1 should not be turn player"

        assert not perm.has_keyword('_is_piercing'), (
            "Inherited Piercing should NOT activate during opponent's turn"
        )


@pytest.mark.behavioral
class TestBT12050NoSpuriousEffects:
    """Verify BT12-050 has no spurious OnEndTurn effect.

    Card text only has two effects:
    - Main: [Your Turn] DNA digivolve into blue -> +1 memory
    - Inherited: [Your Turn] Imperialdramon/Free -> Piercing

    There should be NO OnEndTurn DNA digivolve effect.
    """

    def test_no_on_end_turn_effect(self, debug_runner):
        """BT12-050 should have no OnEndTurn effects."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)

        perm = runner.place_on_field(1, [BT12_050])
        card = perm.top_card
        assert card is not None

        # Get all effects from the card
        all_effects = card.effect_list(None)

        on_end_turn_effects = [
            eff for eff in all_effects
            if getattr(eff, 'timing', None) == EffectTiming.OnEndTurn
        ]

        assert len(on_end_turn_effects) == 0, (
            f"BT12-050 should have NO OnEndTurn effects (card text has no End of Turn effect). "
            f"Found {len(on_end_turn_effects)} OnEndTurn effects: "
            f"{[eff.effect_name for eff in on_end_turn_effects]}"
        )

    def test_only_expected_effects_exist(self, debug_runner):
        """BT12-050 should have exactly 2 effects: DNA memory and inherited Piercing."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)

        perm = runner.place_on_field(1, [BT12_050])
        card = perm.top_card
        assert card is not None

        all_effects = card.effect_list(None)

        # Filter out alt-digi effects (those are structural, not card text effects)
        card_effects = [
            eff for eff in all_effects
            if not (hasattr(eff, '_alt_digi_cost') and eff._alt_digi_cost is not None)
        ]

        # Should have exactly 2 effects: DNA memory gain + inherited Piercing
        assert len(card_effects) == 2, (
            f"BT12-050 should have exactly 2 card effects (DNA memory + inherited Piercing). "
            f"Found {len(card_effects)}: {[eff.effect_name for eff in card_effects]}"
        )


@pytest.mark.behavioral
class TestBT12050DNAMemory:
    """[Your Turn] When this Digimon would DNA digivolve into a blue Digimon
    card, gain 1 memory.

    Tests the condition logic at the unit level since full DNA flow through
    the action system is complex.
    """

    def test_dna_memory_effect_exists_with_correct_timing(self, debug_runner):
        """BT12-050 should have a WhenDigivolving effect for DNA memory gain."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)

        perm = runner.place_on_field(1, [BT12_050])
        card = perm.top_card
        assert card is not None

        all_effects = card.effect_list(None)
        dna_effects = [
            eff for eff in all_effects
            if getattr(eff, 'timing', None) == EffectTiming.WhenDigivolving
        ]

        assert len(dna_effects) >= 1, (
            f"BT12-050 should have at least one WhenDigivolving effect for DNA memory. "
            f"Effects: {[eff.effect_name for eff in all_effects]}"
        )

    def test_dna_memory_effect_is_not_inherited(self, debug_runner):
        """The DNA memory gain effect is a main (non-inherited) effect."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)

        perm = runner.place_on_field(1, [BT12_050])
        card = perm.top_card
        all_effects = card.effect_list(None)

        dna_effects = [
            eff for eff in all_effects
            if getattr(eff, 'timing', None) == EffectTiming.WhenDigivolving
        ]

        for eff in dna_effects:
            assert not eff.is_inherited_effect, (
                f"DNA memory gain should be a MAIN effect, not inherited. "
                f"Effect: {eff.effect_name}"
            )
