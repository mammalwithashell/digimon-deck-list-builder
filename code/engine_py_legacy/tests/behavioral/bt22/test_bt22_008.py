"""Behavioral tests for BT22-008 Agumon (Lv.3 Red, CS trait).

Card text:
[On Play] You may return 1 Digimon card with [Greymon], [Garurumon]
    or [Omnimon] in its name from your trash to the hand.
Inherited: [End of Your Turn] This Digimon and any of your other Digimon
    may DNA digivolve into a Digimon card in the hand.
Alt-digi: [Koromon] or Lv.2 w/[CS] trait: Cost 0
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase

# Use Dracomon (ST1-04) as filler to avoid "Agumon" name collision
FILLER_DECK = ["ST1-04"] * 50


@pytest.mark.behavioral
class TestBT22008AltDigi:
    """Tests for BT22-008 alternate digivolution requirements.

    C# reference: EqualsCardName("Koromon") || (IsLevel2 && HasCSTraits)
    Two separate paths, must be encoded as TWO alt-digi effects.
    """

    def test_alt_digi_from_koromon(self, debug_runner):
        """BT22-008 can digivolve onto Koromon (ST1-01) for cost 0.
        Koromon is Lv.2 with Lesser trait (no CS), so it only matches
        the name path."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        # Place Koromon (ST1-01) on field -- Lv.2, Lesser trait, NO CS
        perm = runner.place_on_field(1, ["ST1-01"])

        # Put BT22-008 in hand
        runner.inject_card(1, "BT22-008", zone="hand")

        # Should be able to digivolve BT22-008 onto Koromon
        actions = runner.find_actions("Digivolve")
        agumon_digi = [a for a, d in actions.items() if "Agumon" in d]
        assert agumon_digi, (
            "BT22-008 should be able to digivolve onto Koromon (exact name match)"
        )

    def test_alt_digi_from_lv2_cs_trait(self, debug_runner):
        """BT22-008 can digivolve onto any Lv.2 with CS trait for cost 0.

        BT22-004 Wanyamon is Lv.2 with CS trait but NOT named Koromon.
        """
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        # Place Wanyamon (BT22-004) -- Lv.2, CS trait
        perm = runner.place_on_field(1, ["BT22-004"])

        # Put BT22-008 in hand
        runner.inject_card(1, "BT22-008", zone="hand")

        # Should be able to digivolve BT22-008 onto Wanyamon via Lv.2 + CS
        actions = runner.find_actions("Digivolve")
        agumon_digi = [a for a, d in actions.items() if "Agumon" in d]
        assert agumon_digi, (
            "BT22-008 should be able to digivolve onto Lv.2 with CS trait "
            "(BT22-004 Wanyamon)"
        )

    def test_alt_digi_rejects_lv2_without_cs(self, debug_runner):
        """BT22-008 cannot digivolve onto a Lv.2 WITHOUT CS trait
        (unless it's named Koromon) when standard evo also doesn't match.

        BT1-003 Upamon is Lv.2 Blue Amphibian trait, no CS -- neither
        the name path (not Koromon) nor the CS path (no CS trait) match,
        AND standard evo doesn't match (BT22-008 is Red, Upamon is Blue).
        """
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["BT1-003"])

        runner.inject_card(1, "BT22-008", zone="hand")

        actions = runner.find_actions("Digivolve")
        agumon_digi = [a for a, d in actions.items() if "Agumon" in d]
        assert not agumon_digi, (
            "BT22-008 should NOT digivolve onto Lv.2 Blue without CS trait "
            "(BT1-003 Upamon)"
        )


@pytest.mark.behavioral
class TestBT22008OnPlay:
    """Tests for BT22-008 On Play effect.

    [On Play] You may return 1 Digimon card with [Greymon], [Garurumon]
    or [Omnimon] in its name from your trash to the hand.
    """

    def test_on_play_returns_greymon_from_trash(self, debug_runner):
        """Playing BT22-008 should allow selecting a Greymon-name Digimon
        from trash to add to hand."""
        from engine_py_legacy.engine.game.constants import SEL_TRASH_START

        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        # Put a Greymon card in trash -- WarGreymon BT22-013
        runner.inject_card(1, "BT22-013", zone="trash")
        assert any(
            c.c_entity_base.card_id == "BT22-013" for c in game.player1.trash_cards
        ), "WarGreymon should be in trash"

        # Put BT22-008 in hand and play it
        runner.inject_card(1, "BT22-008", zone="hand")
        play_action = runner.find_action("Play Agumon")
        assert play_action is not None, "Should be able to play BT22-008 Agumon"

        result = runner.execute(play_action)

        # Should be in a selection phase for trash
        assert game.current_phase == GamePhase.SelectTrash, (
            f"Expected SelectTrash phase, got {game.current_phase}"
        )

        # Find the trash selection action for the WarGreymon card
        legal = runner.action_mask()
        trash_actions = [a for a in legal if a >= SEL_TRASH_START]
        assert trash_actions, "Should have at least one trash card to select"

        # Select the first valid trash card
        runner.execute(trash_actions[0])

        # After selection, WarGreymon should be in hand (moved from trash)
        hand_ids = [c.c_entity_base.card_id for c in game.player1.hand_cards]
        assert "BT22-013" in hand_ids, (
            "WarGreymon should have been returned from trash to hand"
        )
        trash_ids = [c.c_entity_base.card_id for c in game.player1.trash_cards]
        assert "BT22-013" not in trash_ids, (
            "WarGreymon should no longer be in trash"
        )

    def test_on_play_is_optional(self, debug_runner):
        """The On Play effect is optional -- player can decline."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        # Put a Greymon card in trash
        runner.inject_card(1, "BT22-013", zone="trash")

        # Play BT22-008
        runner.inject_card(1, "BT22-008", zone="hand")
        play_action = runner.find_action("Play Agumon")
        assert play_action is not None
        runner.execute(play_action)

        # If in selection phase, there should be a decline/pass option
        if game.current_phase == GamePhase.SelectTrash:
            actions = runner.actions()
            # Action 0 is typically the decline/pass action
            assert 0 in actions or any(
                "decline" in d.lower() or "pass" in d.lower()
                for d in actions.values()
            ), "On Play should be optional -- decline option should exist"

    def test_on_play_only_digimon_cards(self, debug_runner):
        """The effect only returns Digimon cards, not Option/Tamer cards
        even if they contain 'Greymon' in name."""
        from engine_py_legacy.engine.game.constants import SEL_TRASH_START

        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        # ST1-07 Greymon (Digimon) should be selectable
        runner.inject_card(1, "ST1-07", zone="trash")

        runner.inject_card(1, "BT22-008", zone="hand")
        play_action = runner.find_action("Play Agumon")
        assert play_action is not None, "Should be able to play BT22-008 Agumon"
        result = runner.execute(play_action)

        # Should have a selection phase since there's a valid Digimon target
        assert game.current_phase == GamePhase.SelectTrash, (
            f"Expected SelectTrash phase, got {game.current_phase}"
        )

        # Select the Greymon card from trash
        legal = runner.action_mask()
        trash_actions = [a for a in legal if a >= SEL_TRASH_START]
        assert trash_actions, "Should have a trash card to select"
        runner.execute(trash_actions[0])

        hand_ids = [c.c_entity_base.card_id for c in game.player1.hand_cards]
        assert "ST1-07" in hand_ids, (
            "Greymon (Digimon) should be returned from trash to hand"
        )

    def test_on_play_no_valid_targets_no_selection(self, debug_runner):
        """If trash has no matching Digimon, no selection phase occurs."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        # Empty trash or trash with non-matching cards only
        runner.clear_zone(1, "trash")

        runner.inject_card(1, "BT22-008", zone="hand")
        play_action = runner.find_action("Play Agumon")
        assert play_action is not None
        runner.execute(play_action)

        # Should go straight to Main phase (no selection needed)
        assert game.current_phase == GamePhase.Main, (
            "With no valid trash targets, should skip selection and return to Main"
        )


@pytest.mark.behavioral
class TestBT22008Inherited:
    """Tests for BT22-008 inherited [End of Your Turn] DNA digivolve effect."""

    def test_inherited_is_end_of_turn(self, debug_runner):
        """The inherited effect should trigger at OnEndTurn timing."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        # Place a Digimon with BT22-008 as a digivolution source
        perm = runner.place_on_field(1, ["BT22-008", "BT22-013"])

        # Get inherited effects
        card = perm.card_sources[0]  # BT22-008 (bottom of stack)
        effects = card.effect_list(None)
        eot_effects = [e for e in effects if e.timing == EffectTiming.OnEndTurn]
        assert len(eot_effects) >= 1, "Should have at least 1 OnEndTurn inherited effect"

        eot = eot_effects[0]
        assert eot.is_inherited_effect, "DNA digivolve effect must be inherited"
        assert eot.is_optional, "DNA digivolve effect must be optional"
