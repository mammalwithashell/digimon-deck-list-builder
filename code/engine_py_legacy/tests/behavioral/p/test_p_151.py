"""Behavioral tests for P-151 Digimon Liberator (Option, White, Cost 3).

Card text (from cards.json):
  While you have [LIBERATOR] trait Digimon or Tamer, you can ignore this
  card's color requirements.
  [Main] Reveal the top 3 cards of your deck. Add 1 card with the
  [LIBERATOR] trait among them to your hand. Return the rest to the bottom
  of the deck in any order. Then, you may play 1 card with the [LIBERATOR]
  trait and a play cost of 3 or less from your hand without paying the cost.
  [Security] Activate this card's [Main] effects.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestP151DigimonLiberator:
    """Tests for P-151 Digimon Liberator."""

    def test_has_option_skill_timing(self, debug_runner):
        """Should have OptionSkill timing for the Main reveal effect."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-151")
        effects = cs.effect_list(None)
        main_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(main_effects) >= 1, "Should have OptionSkill effect"

    def test_has_security_effect(self, debug_runner):
        """Should have a security effect that activates Main effects."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-151")
        effects = cs.effect_list(None)

        sec_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(sec_effects) >= 1, "Should have a security effect"
        sec = sec_effects[0]
        assert sec.on_process_callback is not None, (
            "Security effect should have a process callback to activate Main effects"
        )

    def test_color_bypass_conditional_on_liberator(self, debug_runner):
        """Color requirement bypass should be conditional on having LIBERATOR trait."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")
        runner.inject_card(1, "P-151", "hand")

        cs = None
        for c in runner.game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "P-151":
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
        runner.inject_card(1, "P-151", "hand")

        cs = None
        for c in runner.game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "P-151":
                cs = c
                break
        assert cs is not None

        # Trigger effect_list to initialize _match_color_requirement_fn
        cs.effect_list(None)

        # With LIBERATOR Digimon on field, color requirement should be bypassed
        assert cs.match_color_requirement is False, (
            "Color requirement should be bypassed when LIBERATOR Digimon is on field"
        )

    def test_security_effect_has_security_skill_timing(self, debug_runner):
        """Security effect should use SecuritySkill timing."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-151")
        effects = cs.effect_list(None)

        sec_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(sec_effects) >= 1
        sec = sec_effects[0]
        assert sec.timing == EffectTiming.SecuritySkill, (
            f"Security effect should have SecuritySkill timing, got {sec.timing}"
        )
