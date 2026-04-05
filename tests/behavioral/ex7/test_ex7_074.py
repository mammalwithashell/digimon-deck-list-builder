"""Behavioral tests for EX7-074 Vortex Resonance (Option, Green/Yellow, Cost 3).

Card text (from cards.json):
  While you have [LIBERATOR] trait Digimon or Tamer, you can ignore this
  card's color requirements.
  [Main] Reveal the top 3 cards of your deck. Add 1 card with the
  [LIBERATOR] trait among them to the hand. Return the rest to the bottom
  of the deck. Then, 1 of your Digimon may digivolve into a Digimon card
  in your hand with the digivolution cost reduced by 4.
  [Security] You may play 1 card with the [LIBERATOR] trait with a play
  cost of 4 or less from your hand or trash without paying the cost. Then,
  add this card to the hand.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestEX7074VortexResonance:
    """Tests for EX7-074 Vortex Resonance."""

    def test_has_option_skill_timing(self, debug_runner):
        """Should have OptionSkill timing for the Main reveal+digivolve effect."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("EX7-074")
        effects = cs.effect_list(None)
        main_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(main_effects) >= 1, "Should have OptionSkill effect"

    def test_has_security_effect(self, debug_runner):
        """Should have a security effect: play LIBERATOR cost <=4 free, add to hand."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("EX7-074")
        effects = cs.effect_list(None)

        sec_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(sec_effects) >= 1, "Should have a security effect"
        sec = sec_effects[0]
        assert sec.on_process_callback is not None, "Security should have process callback"

    def test_color_bypass_conditional_on_liberator(self, debug_runner):
        """Color requirement bypass should be conditional on having LIBERATOR trait."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")
        runner.inject_card(1, "EX7-074", "hand")

        cs = None
        for c in runner.game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "EX7-074":
                cs = c
                break
        assert cs is not None

        # Trigger effect_list to initialize _match_color_requirement_fn
        cs.effect_list(None)

        # Without LIBERATOR trait on field, color requirement should be enforced
        assert cs.match_color_requirement is True, (
            "Color requirement should be enforced when no LIBERATOR on field"
        )

    def test_color_bypass_with_liberator_digimon(self, debug_runner):
        """With a LIBERATOR trait Digimon, should bypass color requirement."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place a LIBERATOR trait Digimon
        runner.place_on_field(1, ["BT18-060"])  # Vemmon (LIBERATOR trait Lv.3)
        runner.inject_card(1, "EX7-074", "hand")

        cs = None
        for c in runner.game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "EX7-074":
                cs = c
                break
        assert cs is not None

        # Trigger effect_list to initialize _match_color_requirement_fn
        cs.effect_list(None)

        # With LIBERATOR Digimon on field, color requirement should be bypassed
        assert cs.match_color_requirement is False, (
            "Color requirement should be bypassed when LIBERATOR Digimon is on field"
        )

    def test_color_requirement_enforced_without_liberator(self, debug_runner):
        """With a non-LIBERATOR Digimon, color requirement should still be enforced."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place a non-LIBERATOR Digimon
        runner.place_on_field(1, ["ST1-03"])  # Agumon (Reptile, no LIBERATOR)
        runner.inject_card(1, "EX7-074", "hand")

        cs = None
        for c in runner.game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "EX7-074":
                cs = c
                break
        assert cs is not None

        # Trigger effect_list to initialize _match_color_requirement_fn
        cs.effect_list(None)

        assert cs.match_color_requirement is True, (
            "Color requirement should be enforced with non-LIBERATOR Digimon"
        )
