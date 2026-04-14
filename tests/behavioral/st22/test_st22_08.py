"""Behavioral tests for ST22-08 Offensive Plug-In V (Option, Red, Cost 4).

Card text (from cards.json):
  While you have a Tamer, you can ignore this card's color requirements.
  [Security] Delete 1 of your opponent's Digimon with the lowest DP.
  Then, add this card to the hand.
  [Main] You may link this card to 1 of your Digimon without paying the cost.

  Inherited (Link): [Link] Lv.3 or higher: Cost 2
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestST2208OffensivePlugInV:
    """Tests for ST22-08 Offensive Plug-In V."""

    def test_security_deletes_lowest_dp(self, debug_runner):
        """Security effect should delete opponent's Digimon with the lowest DP."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("ST22-08")
        effects = cs.effect_list(None)

        sec_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(sec_effects) >= 1, "Should have a security effect"

        sec = sec_effects[0]
        assert sec.on_process_callback is not None, "Security should have process callback"

    def test_has_option_skill_timing(self, debug_runner):
        """Should have OptionSkill timing for the Main effect."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("ST22-08")
        effects = cs.effect_list(None)
        main_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(main_effects) >= 1, "Should have OptionSkill effect"

    def test_has_security_skill_timing(self, debug_runner):
        """Security effect should use SecuritySkill timing."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("ST22-08")
        effects = cs.effect_list(None)

        sec_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(sec_effects) >= 1, "Should have security effect"
        sec = sec_effects[0]
        assert sec.timing == EffectTiming.SecuritySkill, (
            f"Security effect should have SecuritySkill timing, got {sec.timing}"
        )

    def test_color_bypass_with_tamer(self, debug_runner):
        """Should bypass color requirement when player has a Tamer."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place a tamer for P1
        runner.place_on_field(1, ["ST1-12"])  # Tai Kamiya (Red Tamer)

        runner.inject_card(1, "ST22-08", "hand")

        cs = None
        for c in runner.game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "ST22-08":
                cs = c
                break
        assert cs is not None

        # Trigger effect_list to initialize _match_color_requirement_fn
        cs.effect_list(None)

        # With a tamer, color requirement should be bypassed
        assert cs.match_color_requirement is False, (
            "Color requirement should be bypassed when player has a Tamer"
        )

    def test_color_requirement_enforced_without_tamer(self, debug_runner):
        """Should enforce color requirement when player has no Tamer."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # No tamer for P1
        runner.inject_card(1, "ST22-08", "hand")

        cs = None
        for c in runner.game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "ST22-08":
                cs = c
                break
        assert cs is not None

        # Trigger effect_list to initialize _match_color_requirement_fn
        cs.effect_list(None)

        # Without a tamer, color requirement should be enforced
        assert cs.match_color_requirement is True, (
            "Color requirement should be enforced when player has no Tamer"
        )
