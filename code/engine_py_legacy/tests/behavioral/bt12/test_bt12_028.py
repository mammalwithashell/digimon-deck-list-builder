"""Behavioral tests for BT12-028 Paildramon.

Card text:
  Lv.5 | Blue/Green | Dragonkin | DP:8000 | Cost:8 | Attribute: Free
  [When Digivolving] Trash the top 3 digivolution cards of all of your
  opponent's Digimon. Then, if DNA digivolving, 2 of your opponent's Digimon
  with no digivolution cards can't attack until the end of your opponent's turn.

  Inherited: [End of Attack] If this Digimon has [Imperialdramon] in its name
  or [Free] trait, gain 1 memory.

Key faithfulness points tested:
  - When Digivolving: trashes top 3 digi-cards from ALL opponent Digimon
  - Does NOT trash from opponent Digimon with 0 digi-cards
  - DNA clause: selects exactly 2 opponent Digimon with no digi-cards,
    applies CANNOT_ATTACK until end of opponent's turn
  - Selection must use chained callbacks (not loop), since engine
    request_selection overwrites pending_selection
  - Inherited: triggers on End of Attack
  - Inherited: activates if name contains "Imperialdramon"
  - Inherited: activates if attribute is "Free" (NOT trait/type_eng)
  - Inherited: does NOT activate if neither condition met
"""

import pytest
from engine_py_legacy.engine.interfaces.modifiers import ModifierType


# Card IDs used in tests
BT12_028 = "BT12-028"          # Card under test (Paildramon, Lv.5 Blue/Green)
IMPERIALDRAMON_DM = "BT12-030" # Imperialdramon: Dragon Mode (Lv.6)
AGUMON_ST1 = "ST1-03"          # Agumon (Lv.3, Red, name doesn't contain Imperialdramon)
GREYMON_ST1 = "ST1-07"         # Greymon (Lv.4, Red)
METALGREYMON_ST1 = "ST1-09"    # MetalGreymon (Lv.5, Red)


@pytest.mark.behavioral
class TestBT12028WhenDigivolving:
    """[When Digivolving] Trash top 3 digi-cards from ALL opponent Digimon."""

    def test_trashes_top_3_digi_cards_from_opponent_digimon(self, debug_runner):
        """When digivolving, should trash top 3 digivolution cards from each
        opponent Digimon that has digivolution cards."""
        runner = debug_runner(initial_memory=10)

        # Place opponent Digimon with a 4-card stack: Lv.3 -> Lv.4 -> Lv.5 -> top
        # That's 3 digi-cards below the top card
        opp_perm = runner.place_on_field(2, [AGUMON_ST1, GREYMON_ST1, METALGREYMON_ST1, AGUMON_ST1])
        assert len(opp_perm.card_sources) == 4, "Opponent should have 4-card stack"

        # Place BT12-028 on P1 field and invoke the When Digivolving effect directly
        perm = runner.place_on_field(1, [BT12_028])
        game = runner.game

        # Find the When Digivolving effect
        top = perm.top_card
        all_effects = top.effect_list(None)
        wd_effects = [
            e for e in all_effects
            if getattr(e, 'is_when_digivolving', False)
        ]
        assert len(wd_effects) >= 1, "Should have a When Digivolving effect"

        wd_effect = wd_effects[0]
        trash_before = len(game.player2.trash_cards)

        wd_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'is_dna_digivolve': False,
        })

        # All 3 digi-cards should be trashed (the top card remains)
        assert len(opp_perm.card_sources) == 1, (
            f"Opponent Digimon should have only top card remaining, "
            f"got {len(opp_perm.card_sources)} cards"
        )
        assert len(game.player2.trash_cards) >= trash_before + 3, (
            f"3 cards should have been trashed to opponent's trash"
        )

    def test_trashes_from_multiple_opponent_digimon(self, debug_runner):
        """Should trash from ALL opponent Digimon, not just one."""
        runner = debug_runner(initial_memory=10)

        # Place two opponent Digimon with digi-cards
        opp1 = runner.place_on_field(2, [AGUMON_ST1, GREYMON_ST1])  # 1 digi-card
        opp2 = runner.place_on_field(2, [AGUMON_ST1, GREYMON_ST1, METALGREYMON_ST1])  # 2 digi-cards

        perm = runner.place_on_field(1, [BT12_028])
        game = runner.game

        top = perm.top_card
        wd_effect = [e for e in top.effect_list(None) if getattr(e, 'is_when_digivolving', False)][0]

        wd_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'is_dna_digivolve': False,
        })

        # opp1 had 1 digi-card -> trashed 1 (can't trash more than available)
        assert len(opp1.card_sources) == 1, (
            f"Opp Digimon 1 should have 1 card left, got {len(opp1.card_sources)}"
        )
        # opp2 had 2 digi-cards -> trashed 2
        assert len(opp2.card_sources) == 1, (
            f"Opp Digimon 2 should have 1 card left, got {len(opp2.card_sources)}"
        )

    def test_skips_opponent_digimon_with_no_digi_cards(self, debug_runner):
        """Should not try to trash from Digimon with no digivolution cards."""
        runner = debug_runner(initial_memory=10)

        # Place opponent Digimon with NO digi-cards (just a single card = top card only)
        opp_perm = runner.place_on_field(2, [AGUMON_ST1])
        assert opp_perm.has_no_digivolution_cards is True

        perm = runner.place_on_field(1, [BT12_028])
        game = runner.game
        trash_before = len(game.player2.trash_cards)

        top = perm.top_card
        wd_effect = [e for e in top.effect_list(None) if getattr(e, 'is_when_digivolving', False)][0]

        wd_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'is_dna_digivolve': False,
        })

        # Nothing should be trashed
        assert len(opp_perm.card_sources) == 1
        assert len(game.player2.trash_cards) == trash_before

    def test_dna_selects_2_targets_with_no_digi_cards(self, debug_runner):
        """When DNA digivolving, should select 2 opponent Digimon with no
        digivolution cards and apply CANNOT_ATTACK modifier."""
        runner = debug_runner(initial_memory=10)

        # Place 3 opponent Digimon with no digi-cards (just top card)
        opp1 = runner.place_on_field(2, [AGUMON_ST1])
        opp2 = runner.place_on_field(2, [GREYMON_ST1])
        opp3 = runner.place_on_field(2, [METALGREYMON_ST1])

        perm = runner.place_on_field(1, [BT12_028])
        game = runner.game

        top = perm.top_card
        wd_effect = [e for e in top.effect_list(None) if getattr(e, 'is_when_digivolving', False)][0]

        wd_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'is_dna_digivolve': True,
        })

        # Auto-resolve the selection phases (should select 2)
        runner.auto_resolve()

        # At least 2 opponent Digimon should have CANNOT_ATTACK modifier
        cannot_attack_count = 0
        for opp_perm in game.player2.battle_area:
            if game.modifiers.has_modifier(opp_perm, ModifierType.CANNOT_ATTACK):
                cannot_attack_count += 1

        assert cannot_attack_count == 2, (
            f"Exactly 2 opponent Digimon should have CANNOT_ATTACK, "
            f"got {cannot_attack_count}"
        )

    def test_dna_no_effect_when_not_dna(self, debug_runner):
        """When NOT DNA digivolving, should not apply CANNOT_ATTACK."""
        runner = debug_runner(initial_memory=10)

        opp1 = runner.place_on_field(2, [AGUMON_ST1])
        opp2 = runner.place_on_field(2, [GREYMON_ST1])

        perm = runner.place_on_field(1, [BT12_028])
        game = runner.game

        top = perm.top_card
        wd_effect = [e for e in top.effect_list(None) if getattr(e, 'is_when_digivolving', False)][0]

        wd_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'is_dna_digivolve': False,
        })
        runner.auto_resolve()

        # No CANNOT_ATTACK modifiers should exist
        for opp_perm in game.player2.battle_area:
            assert not game.modifiers.has_modifier(opp_perm, ModifierType.CANNOT_ATTACK), (
                "Without DNA, no opponent Digimon should have CANNOT_ATTACK"
            )

    def test_dna_only_targets_digimon_with_no_digi_cards_after_trash(self, debug_runner):
        """DNA selection should only target Digimon with no digi-cards AFTER
        the trash step. A Digimon that had digi-cards but lost them all from
        the trash step should be eligible."""
        runner = debug_runner(initial_memory=10)

        # Place opponent Digimon with exactly 3 digi-cards -> will have 0 after trash
        opp_had_cards = runner.place_on_field(2, [AGUMON_ST1, GREYMON_ST1, METALGREYMON_ST1, AGUMON_ST1])
        # Place opponent Digimon already with 0 digi-cards
        opp_no_cards = runner.place_on_field(2, [GREYMON_ST1])

        perm = runner.place_on_field(1, [BT12_028])
        game = runner.game

        top = perm.top_card
        wd_effect = [e for e in top.effect_list(None) if getattr(e, 'is_when_digivolving', False)][0]

        wd_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'is_dna_digivolve': True,
        })
        runner.auto_resolve()

        # Both should now be eligible (both have 0 digi-cards after trash)
        # Both should get CANNOT_ATTACK
        cannot_attack_count = 0
        for opp_perm in game.player2.battle_area:
            if game.modifiers.has_modifier(opp_perm, ModifierType.CANNOT_ATTACK):
                cannot_attack_count += 1

        assert cannot_attack_count == 2, (
            f"Both opponent Digimon (now with 0 digi-cards) should have "
            f"CANNOT_ATTACK, got {cannot_attack_count}"
        )


@pytest.mark.behavioral
class TestBT12028Inherited:
    """Inherited: [End of Attack] If Imperialdramon name or Free attribute, gain 1 memory."""

    def test_inherited_triggers_for_imperialdramon_name(self, debug_runner):
        """Inherited should activate when the Digimon has Imperialdramon in its name."""
        runner = debug_runner(initial_memory=3)

        # Place BT12-028 under Imperialdramon: Dragon Mode
        perm = runner.place_on_field(1, [BT12_028, IMPERIALDRAMON_DM])
        game = runner.game

        # Find the inherited End of Attack effect
        source_card = perm.card_sources[0]  # BT12-028 is under
        all_effects = source_card.effect_list(None)
        inherited = [
            e for e in all_effects
            if getattr(e, 'is_inherited_effect', False)
        ]
        assert len(inherited) >= 1, "Should have inherited effect"

        eoa_effect = inherited[0]

        # Check condition passes
        result = eoa_effect.can_use_condition({})
        assert result is True, (
            "Inherited condition should pass for Imperialdramon name"
        )

        # Process and check memory gain
        mem_before = game.memory
        eoa_effect.on_process_callback({'player': game.player1})
        assert game.memory == mem_before + 1, (
            f"Should gain 1 memory. Before: {mem_before}, after: {game.memory}"
        )

    def test_inherited_triggers_for_free_attribute(self, debug_runner):
        """Inherited should activate when the Digimon has [Free] attribute.

        BUG: The script uses perm.has_trait('Free') which checks type_eng
        (e.g. 'Dragonkin'). 'Free' is in attribute_eng, not type_eng.
        This test will FAIL until the bug is fixed.
        """
        runner = debug_runner(initial_memory=3)

        # Place BT12-028 under itself (Paildramon has attribute_eng = ["Free"])
        # Paildramon name does NOT contain "Imperialdramon"
        perm = runner.place_on_field(1, [BT12_028, BT12_028])
        game = runner.game

        # Verify that BT12-028 (Paildramon) has Free attribute, not in name Imperialdramon
        top = perm.top_card
        assert top.c_entity_base is not None
        assert 'Free' in top.c_entity_base.attribute_eng, (
            "BT12-028 should have 'Free' in attribute_eng"
        )
        assert not perm.contains_card_name('Imperialdramon'), (
            "Paildramon should NOT contain 'Imperialdramon' in name"
        )

        # Find the inherited effect
        source_card = perm.card_sources[0]
        all_effects = source_card.effect_list(None)
        inherited = [
            e for e in all_effects
            if getattr(e, 'is_inherited_effect', False)
        ]
        assert len(inherited) >= 1

        eoa_effect = inherited[0]
        result = eoa_effect.can_use_condition({})
        assert result is True, (
            "Inherited condition should pass for [Free] attribute. "
            "If this fails, the script is using has_trait('Free') which checks "
            "type_eng instead of attribute_eng."
        )

    def test_inherited_does_not_trigger_without_imperialdramon_or_free(self, debug_runner):
        """Inherited should NOT activate if Digimon lacks both Imperialdramon
        name and Free attribute."""
        runner = debug_runner(initial_memory=3)

        # Place BT12-028 under Greymon (no Imperialdramon name, attribute is Vaccine not Free)
        perm = runner.place_on_field(1, [BT12_028, GREYMON_ST1])
        game = runner.game

        # Verify Greymon has neither Imperialdramon in name nor Free attribute
        top = perm.top_card
        assert not perm.contains_card_name('Imperialdramon')
        attrs = top.c_entity_base.attribute_eng if top.c_entity_base else []
        assert 'Free' not in attrs, "Greymon should not have Free attribute"

        source_card = perm.card_sources[0]
        all_effects = source_card.effect_list(None)
        inherited = [
            e for e in all_effects
            if getattr(e, 'is_inherited_effect', False)
        ]
        assert len(inherited) >= 1

        eoa_effect = inherited[0]
        result = eoa_effect.can_use_condition({})
        assert result is False, (
            "Inherited condition should fail when neither Imperialdramon "
            "name nor Free attribute is present"
        )
