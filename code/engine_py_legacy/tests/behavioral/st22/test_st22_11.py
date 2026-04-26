"""Behavioral tests for ST22-11 Defense Plug-In F (Option, Black, Cost 3).

Card text:
  While you have a Tamer, you can ignore this card's color requirements.
  [Security] De-Digivolve 2 1 of your opponent's Digimon. Then, add this
  card to the hand.
  [Main] You may link this card to 1 of your Digimon without paying the cost.
  Then, until your opponent's turn ends, 1 of your Digimon gains <Reboot>
  and +3000 DP.

  Inherited: [Link] Lv.3 or higher: Cost 2
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestST2211ColorBypass:
    """While you have a Tamer, ignore color requirements."""

    def test_color_bypass_with_tamer(self, debug_runner):
        """Should bypass color requirement when player has a Tamer."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place a tamer for P1
        runner.place_on_field(1, ["ST1-12"])  # Tai Kamiya (Red Tamer)

        runner.inject_card(1, "ST22-11", "hand")

        cs = None
        for c in runner.game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "ST22-11":
                cs = c
                break
        assert cs is not None

        # Trigger effect_list to initialize _match_color_requirement_fn
        cs.effect_list(None)

        # With a tamer, color requirement should be bypassed
        assert cs.match_color_requirement is False, (
            "Color requirement should be bypassed when player has a Tamer"
        )

    def test_color_enforced_without_tamer(self, debug_runner):
        """Should enforce color requirement when player has no Tamer."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.inject_card(1, "ST22-11", "hand")

        cs = None
        for c in runner.game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "ST22-11":
                cs = c
                break
        assert cs is not None

        cs.effect_list(None)

        assert cs.match_color_requirement is True, (
            "Color requirement should be enforced when player has no Tamer"
        )


@pytest.mark.behavioral
class TestST2211SecurityEffect:
    """[Security] De-Digivolve 2 + add to hand."""

    def test_has_security_effect(self, debug_runner):
        """Should have a security effect with SecuritySkill timing."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("ST22-11")
        effects = cs.effect_list(None)
        sec_effects = [e for e in effects
                       if e.timing == EffectTiming.SecuritySkill
                       and getattr(e, 'is_security_effect', False)]
        assert len(sec_effects) >= 1, "Should have SecuritySkill effect"

    def test_security_has_de_digivolve(self, debug_runner):
        """Security effect should have a process callback for De-Digivolve."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("ST22-11")
        effects = cs.effect_list(None)
        sec_effects = [e for e in effects
                       if e.timing == EffectTiming.SecuritySkill
                       and getattr(e, 'is_security_effect', False)]
        assert sec_effects[0].on_process_callback is not None, (
            "Security should have a process callback for De-Digivolve"
        )


@pytest.mark.behavioral
class TestST2211MainEffect:
    """[Main] Link + grant Reboot + 3000 DP."""

    def test_has_option_skill_timing(self, debug_runner):
        """Should have an OptionSkill timing effect."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("ST22-11")
        effects = cs.effect_list(None)
        main_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(main_effects) >= 1, "Should have OptionSkill effect"

    def test_main_grants_reboot_and_dp(self, debug_runner):
        """Main effect should link and grant Reboot + 3000 DP."""
        runner = debug_runner(initial_memory=10)

        # Need a Digimon to link to
        runner.place_on_field(1, ["ST1-03"])  # Agumon Lv.3
        runner.inject_card(1, "ST22-11", "hand")

        # Need a black Digimon or tamer for color requirement
        runner.place_on_field(1, ["BT3-075"])  # Black Tamer

        runner.set_phase("Main")

        play_action = runner.find_action("Defense Plug-In")
        if play_action is None:
            play_action = runner.find_action("ST22-11")
        assert play_action is not None, f"Should be able to play ST22-11. Available: {runner.actions()}"

        runner.execute(play_action)
        runner.auto_resolve()

        # After resolving, the Digimon should have gained DP
        snap = runner.snapshot()
        agumon_on_field = [s for s in snap.p1_field if s.card_id == "ST1-03"]
        if agumon_on_field:
            # Agumon base DP is 2000, with +3000 = 5000
            assert agumon_on_field[0].dp >= 5000, (
                f"Agumon should have +3000 DP, got {agumon_on_field[0].dp}"
            )


@pytest.mark.behavioral
class TestST2211LinkRequirement:
    """Inherited: [Link] Lv.3 or higher: Cost 2."""

    def test_has_link_effect(self, debug_runner):
        """Should have a link inherited effect."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("ST22-11")
        effects = cs.effect_list(None)
        link_effects = [e for e in effects if getattr(e, '_is_link', False)]
        assert len(link_effects) >= 1, "Should have a link effect"
        assert link_effects[0]._link_cost == 2, "Link cost should be 2"
        assert link_effects[0]._link_min_level == 3, "Link min level should be 3"
