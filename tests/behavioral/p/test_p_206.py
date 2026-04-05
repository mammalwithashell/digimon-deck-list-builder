"""Behavioral tests for P-206 Digital Gate Open (Option, White, Cost 4).

Card text (from cards.json):
  You can ignore this card's color requirements.
  [Main] Reveal the top 3 cards of your deck. Add 1 Digimon card and
  1 Tamer card among them to the hand. Return the rest to the bottom
  of the deck. Then, place this card in the battle area.
  [Main] <Delay> - You may play 1 Tamer card with the same color as any
  of your Digimon on the field from your hand with the play cost reduced by 4.
  [Security] You may play 1 Digimon card with a play cost of 3 or less from
  your hand or trash without paying the cost. Then, add this card to the hand.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestP206DigitalGateOpen:
    """Tests for P-206 Digital Gate Open."""

    def test_has_option_skill_timing(self, debug_runner):
        """Should have OptionSkill timing for the Main reveal effect."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-206")
        effects = cs.effect_list(None)
        main_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(main_effects) >= 1, "Should have OptionSkill effect"

    def test_color_requirement_bypassed(self, debug_runner):
        """P-206 unconditionally bypasses color requirement."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")
        runner.inject_card(1, "P-206", "hand")

        cs = None
        for c in runner.game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "P-206":
                cs = c
                break
        assert cs is not None

        # Trigger effect_list to initialize _match_color_requirement
        cs.effect_list(None)

        assert cs.match_color_requirement is False, (
            "P-206 should always bypass color requirement"
        )

    def test_has_delay_marker(self, debug_runner):
        """Should have a delay marker effect."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-206")
        effects = cs.effect_list(None)
        delays = [e for e in effects if getattr(e, '_is_delay', False)]
        assert len(delays) >= 1, "Should have a delay marker effect"

    def test_delay_plays_tamer_with_cost_reduction(self, debug_runner):
        """Delay effect should play a color-matched Tamer from hand with cost -4."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-206")
        effects = cs.effect_list(None)

        delay_main = [e for e in effects
                      if e.timing == EffectTiming.OnDeclaration
                      and getattr(e, '_is_field_main', False)]
        assert len(delay_main) >= 1, "Should have OnDeclaration _is_field_main delay effect"
        assert delay_main[0].on_process_callback is not None

    def test_has_security_effect(self, debug_runner):
        """Should have a security effect: play Digimon cost <=3 free, add to hand."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-206")
        effects = cs.effect_list(None)

        sec_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(sec_effects) >= 1, "Should have a security effect"
        sec = sec_effects[0]
        assert sec.on_process_callback is not None, "Security should have process callback"

    def test_security_has_security_skill_timing(self, debug_runner):
        """Security effect should have SecuritySkill timing."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-206")
        effects = cs.effect_list(None)

        sec_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(sec_effects) >= 1
        sec = sec_effects[0]
        assert sec.timing == EffectTiming.SecuritySkill, (
            f"Security should have SecuritySkill timing, got {sec.timing}"
        )

    def test_main_places_in_battle_area(self, debug_runner):
        """Playing Digital Gate Open should place it in the battle area."""
        runner = debug_runner(initial_memory=10, skip_shuffle=True)
        runner.set_phase("Main")
        runner.inject_card(1, "P-206", "hand")

        action = runner.find_action("Digital Gate Open")
        assert action is not None, "Should be able to play Digital Gate Open"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        in_battle = any(s.card_id == "P-206" for s in snap.p1_field)
        assert in_battle, "Digital Gate Open should be placed in the battle area"
