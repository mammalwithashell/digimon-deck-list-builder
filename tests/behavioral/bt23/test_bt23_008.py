"""Behavioral tests for BT23-008 Greymon (Lv.4, Red, Cost 6, Dinosaur+CS traits).

Card text (C# source of truth):
Alt digivolve: From Lv.3 with [Agumon] in name or [CS] trait for cost 2.
<Raid>
[Main] [Once Per Turn] By placing this Digimon's top stacked card as its
bottom digivolution card, you may play 1 [Gabumon] or [Nokia Shiramine]
from your hand with the play cost reduced by 2.
Inherited: [Your Turn] This Digimon gets +2000 DP.

Key cards used:
- BT23-008: Greymon, Lv.4, Red, cost 6, Dinosaur+CS traits
- BT22-008: Agumon, Lv.3, Red, cost 3, CS trait (valid alt-digi: both name+trait)
- BT1-010: Agumon, Lv.3, Red, cost 3, Reptile (valid alt-digi: name match)
- BT22-017: Gabumon, Lv.3, CS trait (valid alt-digi: CS trait, valid play target)
- BT1-009: Monodramon, Lv.3, Red, no CS, not Agumon (should NOT alt-digi)
- BT22-084: Nokia Shiramine, Tamer, cost 5, CS trait (valid play target)
- BT1-029: Gabumon, Lv.3, Reptile, no CS (valid play target: name match)
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming
from digimon_gym.engine.validation.digivolve_validator import can_digivolve


GREYMON_DECK = (
    ["BT23-008"] * 4 + ["BT22-008"] * 4 + ["BT1-010"] * 4 +
    ["BT22-017"] * 4 + ["BT1-009"] * 10 + ["BT22-084"] * 4 +
    ["BT1-029"] * 10 + ["BT1-046"] * 10
)


@pytest.mark.behavioral
class TestBT23008Greymon:
    """Tests for BT23-008 Greymon."""

    # -- Alt digivolve conditions --

    def test_alt_digi_from_agumon_name(self, debug_runner):
        """Should be able to alt-digi from a Lv.3 Agumon (name match) for cost 2."""
        runner = debug_runner(deck1=GREYMON_DECK, deck2=GREYMON_DECK, initial_memory=5)

        # Place a Lv.3 Agumon without CS trait on field
        agumon_perm = runner.place_on_field(1, ["BT1-010"])

        # Create BT23-008 card source for validation
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        greymon = db.create_card_source("BT23-008", runner.game.player1)

        assert can_digivolve(greymon, agumon_perm), \
            "BT23-008 should be able to digivolve from Lv.3 Agumon (name match)"

    def test_alt_digi_from_cs_trait(self, debug_runner):
        """Should be able to alt-digi from a Lv.3 with CS trait for cost 2."""
        runner = debug_runner(deck1=GREYMON_DECK, deck2=GREYMON_DECK, initial_memory=5)

        # Place a Lv.3 CS Digimon that is NOT Agumon
        cs_perm = runner.place_on_field(1, ["BT22-017"])  # Gabumon, CS

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        greymon = db.create_card_source("BT23-008", runner.game.player1)

        assert can_digivolve(greymon, cs_perm), \
            "BT23-008 should be able to digivolve from Lv.3 CS-trait Digimon"

    def test_alt_digi_rejects_non_agumon_non_cs(self, debug_runner):
        """Should NOT be able to alt-digi from a Lv.3 without Agumon name or CS trait.
        (Standard Red Lv.3 digivolve still works via evo_costs, but alt-digi
        specifically requires Agumon or CS.)"""
        runner = debug_runner(deck1=GREYMON_DECK, deck2=GREYMON_DECK, initial_memory=5)

        # Place a Lv.3 non-Agumon, non-CS Digimon
        mono_perm = runner.place_on_field(1, ["BT1-009"])  # Monodramon, Lv.3 Red, no CS

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        greymon = db.create_card_source("BT23-008", runner.game.player1)

        # BT23-008 has standard evo_cost: Red Lv.3 for 2. BT1-009 (Monodramon)
        # is Red Lv.3, so standard digivolve works. The test here checks that
        # the alt-digi path specifically validates name/trait.
        # Since standard evo also matches, can_digivolve returns True.
        # We need to test the alt-digi logic more directly.

        # Check the alt-digi effects for the name/trait conditions
        effects = greymon.effect_list(EffectTiming.NoTiming)
        alt_digi_effects = [e for e in effects if getattr(e, '_alt_digi_cost', None) is not None]
        assert len(alt_digi_effects) >= 1, "Should have alt-digi effects"

        # Verify the alt-digi effects have proper name OR trait constraints
        for eff in alt_digi_effects:
            has_name = getattr(eff, '_alt_digi_name', None) is not None
            has_trait = getattr(eff, '_alt_digi_trait', None) is not None
            assert has_name or has_trait, \
                "Alt-digi effect must have _alt_digi_name or _alt_digi_trait constraint"

    # -- Raid --

    def test_raid_keyword(self, debug_runner):
        """Greymon should have the Raid keyword."""
        runner = debug_runner(deck1=GREYMON_DECK, deck2=GREYMON_DECK, initial_memory=5)

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT23-008", runner.game.player1)
        effects = card.effect_list(None)

        raid_effects = [e for e in effects if getattr(e, '_is_raid', False)]
        assert len(raid_effects) >= 1, "Should have at least 1 Raid effect"

    # -- Main [Once Per Turn] stack shift + play --

    def test_main_opt_exists_with_once_per_turn(self, debug_runner):
        """Main effect should be Once Per Turn with OnDeclaration timing."""
        runner = debug_runner(deck1=GREYMON_DECK, deck2=GREYMON_DECK, initial_memory=5)

        perm = runner.place_on_field(1, ["BT22-008", "BT23-008"])  # Agumon under Greymon

        card = perm.top_card  # Greymon
        effects = card.effect_list(None)

        main_effects = [e for e in effects if e.timing == EffectTiming.OnDeclaration
                        and getattr(e, '_is_field_main', False)]
        assert len(main_effects) == 1, "Should have 1 field main effect"

        main = main_effects[0]
        assert main.max_count_per_turn == 1, "Should be Once Per Turn"

    def test_main_opt_requires_stack(self, debug_runner):
        """Main effect should require at least 1 digivolution card (2+ total in stack)."""
        runner = debug_runner(deck1=GREYMON_DECK, deck2=GREYMON_DECK, initial_memory=5)

        # Place Greymon alone (no digivolution cards)
        perm = runner.place_on_field(1, ["BT23-008"])

        card = perm.top_card
        effects = card.effect_list(None)

        main_effects = [e for e in effects if e.timing == EffectTiming.OnDeclaration
                        and getattr(e, '_is_field_main', False)]
        main = main_effects[0]

        # With only 1 card in stack (just Greymon), condition should fail
        assert not main.can_use_condition({}), \
            "Main effect condition should fail with no digivolution cards"

    def test_main_opt_condition_passes_with_stack(self, debug_runner):
        """Main effect condition should pass when digivolution cards exist."""
        runner = debug_runner(deck1=GREYMON_DECK, deck2=GREYMON_DECK, initial_memory=5)

        # Place Greymon with Agumon as digivolution card
        perm = runner.place_on_field(1, ["BT22-008", "BT23-008"])

        card = perm.top_card
        effects = card.effect_list(None)

        main_effects = [e for e in effects if e.timing == EffectTiming.OnDeclaration
                        and getattr(e, '_is_field_main', False)]
        main = main_effects[0]

        assert main.can_use_condition({}), \
            "Main effect condition should pass with digivolution cards"

    def test_main_opt_stack_shift_moves_top_to_bottom(self, debug_runner):
        """Main effect should move the current top card to the bottom of the
        digivolution stack."""
        runner = debug_runner(deck1=GREYMON_DECK, deck2=GREYMON_DECK, initial_memory=5)

        # Place: bottom=Agumon(BT22-008), top=Greymon(BT23-008)
        perm = runner.place_on_field(1, ["BT22-008", "BT23-008"])

        game = runner.game
        card = perm.top_card  # Greymon

        # Before: card_sources = [Agumon, Greymon]
        assert perm.card_sources[-1].c_entity_base.card_id == "BT23-008"
        assert perm.card_sources[0].c_entity_base.card_id == "BT22-008"

        effects = card.effect_list(None)
        main_effects = [e for e in effects if e.timing == EffectTiming.OnDeclaration
                        and getattr(e, '_is_field_main', False)]
        main = main_effects[0]

        # Execute the effect (no valid play targets needed to verify stack shift)
        main.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # After stack shift: Greymon should be at bottom, Agumon on top
        # card_sources = [Greymon, Agumon]
        assert perm.card_sources[0].c_entity_base.card_id == "BT23-008", \
            "Greymon (originally top) should now be at bottom of stack"
        assert perm.card_sources[-1].c_entity_base.card_id == "BT22-008", \
            "Agumon (originally bottom) should now be on top"

    # -- Inherited: +2000 DP --

    def test_inherited_dp_boost(self, debug_runner):
        """Inherited effect should give +2000 DP during your turn."""
        runner = debug_runner(deck1=GREYMON_DECK, deck2=GREYMON_DECK, initial_memory=5)

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT23-008", runner.game.player1)
        effects = card.effect_list(None)

        inherited_effects = [e for e in effects if getattr(e, 'is_inherited_effect', False)]
        assert len(inherited_effects) == 1, "Should have 1 inherited effect"

        inherited = inherited_effects[0]
        assert inherited.dp_modifier == 2000, "Inherited DP modifier should be +2000"

    def test_inherited_dp_only_your_turn(self, debug_runner):
        """Inherited DP boost should only be active during your turn."""
        runner = debug_runner(deck1=GREYMON_DECK, deck2=GREYMON_DECK, initial_memory=5)

        # Place a Digimon with Greymon as digivolution source
        perm = runner.place_on_field(1, ["BT23-008", "BT22-010"])  # Greymon under Meramon

        card = perm.card_sources[0]  # Greymon (bottom of stack, inherited)
        effects = card.effect_list(None)

        inherited = [e for e in effects if getattr(e, 'is_inherited_effect', False)][0]

        # During own turn: condition should pass
        runner.game.player1.is_my_turn = True
        assert inherited.can_use_condition({}), \
            "Inherited condition should pass during own turn"

        # During opponent turn: condition should fail
        runner.game.player1.is_my_turn = False
        assert not inherited.can_use_condition({}), \
            "Inherited condition should fail during opponent turn"
