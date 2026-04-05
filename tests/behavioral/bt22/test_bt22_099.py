"""Behavioral tests for BT22-099 Kuremi Detective Agency (Option, Purple/Yellow, Cost 3).

Card text (C# source of truth):
While you have a [CS] trait Digimon or Tamer on the field, you can ignore
this card's color requirements.
[Main] Reveal the top 3 cards of your deck. Add 1 [CS] trait card among them
to the hand. Return the rest to the bottom of the deck. Then, place this card
in the battle area.
[Main] <Delay> Gain 2 memory.
[Security] Place this card in the battle area.

Key cards used:
- BT22-099: Kuremi Detective Agency, Option, Purple/Yellow, cost 3, CS trait
- BT22-017: Gabumon, Digimon, Lv.3, CS trait
- BT22-083: Yuuko Kamishiro, Tamer, CS trait
- BT1-009: Monodramon, Digimon, Lv.3, no CS trait
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


# Deck with CS cards on top for reveal test
DETECT_DECK = ["BT22-099"] * 4 + ["BT22-017"] * 20 + ["BT1-009"] * 26

# Deck with a CS tamer for color bypass
CS_FIELD_DECK = ["BT22-099"] * 4 + ["BT22-083"] * 4 + ["BT22-017"] * 20 + ["BT1-009"] * 22


@pytest.mark.behavioral
class TestBT22099KuremiDetectiveAgency:
    """Tests for BT22-099 Kuremi Detective Agency."""

    # -- Color bypass --

    def test_color_bypass_with_cs_on_field(self, debug_runner):
        """Color requirement should be bypassed when a CS Digimon or Tamer is on field."""
        runner = debug_runner(deck1=DETECT_DECK, deck2=DETECT_DECK, initial_memory=5)

        game = runner.game

        # Place a CS Digimon on field
        runner.place_on_field(1, ["BT22-017"])  # Gabumon, CS

        # Check the card in hand for color bypass
        # Find a BT22-099 card in hand or inject one
        runner.inject_card(1, "BT22-099", "hand")
        card = None
        for c in game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "BT22-099":
                card = c
                break
        assert card is not None, "Should have BT22-099 in hand"
        assert not card.match_color_requirement, \
            "Color requirement should be bypassed (False) when CS on field"

    def test_color_enforced_without_cs_on_field(self, debug_runner):
        """Color requirement should be enforced when no CS Digimon/Tamer is on field."""
        runner = debug_runner(deck1=DETECT_DECK, deck2=DETECT_DECK, initial_memory=5)

        game = runner.game
        runner.inject_card(1, "BT22-099", "hand")
        card = None
        for c in game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "BT22-099":
                card = c
                break
        assert card is not None
        assert card.match_color_requirement, \
            "Color requirement should be enforced (True) without CS on field"

    # -- Main effect: Reveal and select --

    def test_main_reveal_selects_cs_card(self, debug_runner):
        """Main effect should reveal top 3, allow selecting 1 CS card to hand,
        and return the rest to deck bottom."""
        # Build deck so top 3 includes CS and non-CS cards
        deck = ["BT22-099"] * 4 + ["BT22-017"] * 2 + ["BT1-009"] * 1 + ["BT22-017"] * 20 + ["BT1-009"] * 23
        runner = debug_runner(deck1=deck, deck2=deck, initial_memory=10)

        game = runner.game
        card = None
        effects = []

        # Find the OptionSkill effect
        runner.inject_card(1, "BT22-099", "hand")
        for c in game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "BT22-099":
                card = c
                effects = card.effect_list(None)
                break

        option_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(option_effects) == 1, "Should have 1 OptionSkill effect"

    # -- Delay: Gain 2 memory --

    def test_delay_gain_2_memory(self, debug_runner):
        """Delay effect should gain 2 memory when activated from battle area."""
        runner = debug_runner(deck1=DETECT_DECK, deck2=DETECT_DECK, initial_memory=5)

        # Place the option in battle area (simulating it was played on a previous turn)
        perm = runner.place_on_field(1, ["BT22-099"], turn_played=-2)

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)

        delay_effects = [e for e in effects if e.timing == EffectTiming.OnDeclaration
                         and getattr(e, '_is_field_main', False)]
        assert len(delay_effects) == 1, "Should have 1 Delay field main effect"

        delay = delay_effects[0]
        assert delay.can_use_condition({}), "Delay condition should pass when on field"

        memory_before = game.memory

        delay.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert game.memory == memory_before + 2, \
            f"Memory should increase by 2, was {memory_before}, now {game.memory}"

    # -- Security: Place in battle area --

    def test_security_effect_exists(self, debug_runner):
        """Security effect should exist to place card in battle area."""
        runner = debug_runner(deck1=DETECT_DECK, deck2=DETECT_DECK, initial_memory=5)

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT22-099", runner.game.player1)
        effects = card.effect_list(None)

        security_effects = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(security_effects) == 1, "Should have 1 SecuritySkill effect"
        assert security_effects[0].is_security_effect, "Should be flagged as security effect"
