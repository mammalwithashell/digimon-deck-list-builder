"""Behavioral tests for BT22-017 Gabumon (Lv.3 Blue, CS trait).

Card text:
[On Play] Reveal the top 3 cards of your deck. Add 1 card with [Omnimon]
    in its text and 1 card with the [CS] trait among them to the hand.
    Return the rest to the bottom of the deck.
Inherited: [End of Your Turn] This Digimon and any of your other Digimon
    may DNA digivolve into a Digimon card in the hand.
Alt-digi: [Tsunomon] or Lv.2 w/[CS] trait: Cost 0
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase

# Use Dracomon (ST1-04) as filler to avoid "Gabumon" name collision
FILLER_DECK = ["ST1-04"] * 50


@pytest.mark.behavioral
class TestBT22017AltDigi:
    """Tests for BT22-017 alternate digivolution requirements.

    C# reference: EqualsCardName("Tsunomon") || (IsLevel2 && HasCSTraits)
    Two separate paths encoded as TWO alt-digi effects.
    """

    def test_alt_digi_from_tsunomon(self, debug_runner):
        """BT22-017 can digivolve onto Tsunomon (ST2-01) for cost 0."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["ST2-01"])

        runner.inject_card(1, "BT22-017", zone="hand")

        actions = runner.find_actions("Digivolve")
        gabumon_digi = [a for a, d in actions.items() if "Gabumon" in d]
        assert gabumon_digi, (
            "BT22-017 should be able to digivolve onto Tsunomon (exact name match)"
        )

    def test_alt_digi_from_lv2_cs_trait(self, debug_runner):
        """BT22-017 can digivolve onto any Lv.2 with CS trait for cost 0.

        BT22-005 Tsumemon is Lv.2 with CS trait but NOT named Tsunomon.
        """
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["BT22-005"])

        runner.inject_card(1, "BT22-017", zone="hand")

        actions = runner.find_actions("Digivolve")
        gabumon_digi = [a for a, d in actions.items() if "Gabumon" in d]
        assert gabumon_digi, (
            "BT22-017 should be able to digivolve onto Lv.2 with CS trait "
            "(BT22-005 Tsumemon)"
        )

    def test_alt_digi_rejects_lv2_without_cs(self, debug_runner):
        """BT22-017 cannot digivolve onto a Lv.2 WITHOUT CS trait
        (unless it's named Tsunomon) when standard evo also doesn't match.

        BT1-001 Yokomon is Lv.2 Red Bulb trait, no CS -- neither the
        name path (not Tsunomon) nor the CS path (no CS trait) match,
        AND standard evo doesn't match (BT22-017 is Blue, Yokomon is Red).
        """
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["BT1-001"])

        runner.inject_card(1, "BT22-017", zone="hand")

        actions = runner.find_actions("Digivolve")
        gabumon_digi = [a for a, d in actions.items() if "Gabumon" in d]
        assert not gabumon_digi, (
            "BT22-017 should NOT digivolve onto Lv.2 Red without CS trait "
            "(BT1-001 Yokomon)"
        )


@pytest.mark.behavioral
class TestBT22017OnPlay:
    """Tests for BT22-017 On Play effect.

    [On Play] Reveal the top 3 cards of your deck. Add 1 card with [Omnimon]
    in its text and 1 card with the [CS] trait among them to the hand.
    Return the rest to the bottom of the deck.

    Important: [Omnimon] in TEXT means card_text contains "Omnimon" (HasText),
    NOT card name check.
    """

    def test_on_play_reveals_and_selects(self, debug_runner):
        """Playing BT22-017 should reveal top 3 and allow selections."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        # Stack the deck: put known cards on top (last injected = top of deck)
        # BT22-013 WarGreymon: has "Omnimon" in text AND has CS trait
        # BT22-008 Agumon: has "Omnimon" in text AND has CS trait
        # ST1-07 Greymon: no Omnimon in text, no CS trait
        runner.inject_card(1, "BT22-013", zone="library_top")
        runner.inject_card(1, "BT22-008", zone="library_top")
        runner.inject_card(1, "ST1-07", zone="library_top")

        # Play BT22-017
        runner.inject_card(1, "BT22-017", zone="hand")
        play_action = runner.find_action("Play Gabumon")
        assert play_action is not None, "Should be able to play BT22-017"
        runner.execute(play_action)

        # Auto-resolve selection phases
        runner.auto_resolve()

        # Game state should be consistent -- back to Main
        assert game.current_phase == GamePhase.Main, (
            f"Should return to Main phase after reveal, got {game.current_phase}"
        )

    def test_on_play_omnimon_text_not_name(self, debug_runner):
        """The effect checks for 'Omnimon' in card TEXT, not card name.

        BT22-015 Omnimon has 'Omnimon' in its NAME but NOT in its card text.
        BT22-013 WarGreymon has 'Omnimon' in its card text.
        """
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        # Verify our card_text assumptions
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        wargreymon = db.create_card_source("BT22-013", None)
        omnimon = db.create_card_source("BT22-015", None)

        assert "Omnimon" in (wargreymon.card_text or ""), (
            "WarGreymon BT22-013 should have 'Omnimon' in card_text"
        )
        # BT22-015 Omnimon's own card text does NOT contain "Omnimon"
        assert "Omnimon" not in (omnimon.card_text or ""), (
            "Omnimon BT22-015 should NOT have 'Omnimon' in its card_text "
            "(the text check is about effect text, not the card name)"
        )

    def test_on_play_cs_trait_filter(self, debug_runner):
        """The second pass selects 1 card with CS trait."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        # Stack deck with cards on top (last inject = topmost)
        # BT22-004 Wanyamon: Lv.2, CS trait (but no Omnimon in text)
        runner.inject_card(1, "BT22-004", zone="library_top")
        runner.inject_card(1, "ST1-04", zone="library_top")  # Dracomon filler, no CS
        runner.inject_card(1, "ST1-04", zone="library_top")  # Dracomon filler

        runner.inject_card(1, "BT22-017", zone="hand")
        play_action = runner.find_action("Play Gabumon")
        assert play_action is not None, "Should be able to play BT22-017 Gabumon"
        runner.execute(play_action)
        runner.auto_resolve()

        # BT22-004 should have been added to hand via CS trait pass
        hand_ids = [c.c_entity_base.card_id for c in game.player1.hand_cards]
        assert "BT22-004" in hand_ids, (
            "BT22-004 (CS trait) should be added to hand in second pass"
        )

    def test_on_play_is_not_optional(self, debug_runner):
        """The On Play effect is NOT optional per C# (is_optional=false)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        # Check the effect's is_optional flag directly
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT22-017", game.player1)
        effects = cs.effect_list(None)
        on_play_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone and e.is_on_play
        ]
        assert len(on_play_effects) >= 1, "Should have On Play effect"
        assert not on_play_effects[0].is_optional, (
            "The On Play reveal effect should NOT be optional"
        )


@pytest.mark.behavioral
class TestBT22017Inherited:
    """Tests for BT22-017 inherited [End of Your Turn] DNA digivolve effect."""

    def test_inherited_is_end_of_turn(self, debug_runner):
        """The inherited effect should trigger at OnEndTurn timing."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        # Place a Digimon with BT22-017 as digivolution source
        perm = runner.place_on_field(1, ["BT22-017", "BT22-026"])

        card = perm.card_sources[0]  # BT22-017 (bottom of stack)
        effects = card.effect_list(None)
        eot_effects = [e for e in effects if e.timing == EffectTiming.OnEndTurn]
        assert len(eot_effects) >= 1, "Should have at least 1 OnEndTurn inherited effect"

        eot = eot_effects[0]
        assert eot.is_inherited_effect, "DNA digivolve effect must be inherited"
        assert eot.is_optional, "DNA digivolve effect must be optional"
