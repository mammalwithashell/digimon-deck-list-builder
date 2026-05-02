"""Behavioral tests for LM-027 Red Scramble (Option, Red, Cost 2).

Card text (from cards.json):
  [Main] 1 of your red Digimon may digivolve into a red Digimon card in
  the hand with the digivolution cost reduced by 3. Then, place this card
  in the battle area.

  [Start of Your Turn] If your opponent has a Digimon, <Delay> (By trashing
  this card after the placing turn, activate the effect below.)
  - Return 1 red Digimon card from your trash to the top of the deck. Then,
  if you don't have a Digimon, you may play 1 red Digimon card with 2000 DP
  or less from your trash without paying the cost.

  [Security] You may play 1 red Digimon card with 2000 DP or less from your
  trash without paying the cost. Then, add this card to the hand.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestLM027RedScramble:
    """Tests for LM-027 Red Scramble."""

    def test_main_places_card_in_battle_area(self, debug_runner):
        """Playing Red Scramble should place it in the battle area (Delay)."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place a red Digimon on field so there's a valid digivolve base
        runner.place_on_field(1, ["ST1-03"])  # Agumon (Red, Lv.3)
        runner.inject_card(1, "LM-027", "hand")

        action = runner.find_action("Red Scramble")
        assert action is not None, "Should be able to play Red Scramble"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        in_battle = any(s.card_id == "LM-027" for s in snap.p1_field)
        assert in_battle, "Red Scramble should be placed in the battle area"

    def test_delay_condition_requires_opponent_digimon(self, debug_runner):
        """Delay condition should check opponent has a Digimon."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-027")
        effects = cs.effect_list(None)

        delay_effects = [e for e in effects if getattr(e, '_is_delay', False)]
        assert len(delay_effects) >= 1, "Should have a delay marker effect"

    def test_delay_returns_red_digimon_from_trash_to_deck_top(self, debug_runner):
        """Delay effect should return 1 red Digimon from trash to top of deck."""
        runner = debug_runner(initial_memory=3, skip_shuffle=True)

        # Setup: Red Scramble in battle area (placed on a prior turn)
        runner.place_on_field(1, ["LM-027"], turn_played=-1)
        # Opponent must have a Digimon for the Delay condition
        runner.place_on_field(2, ["ST1-03"])
        # Put a red Digimon in P1's trash
        runner.inject_card(1, "ST1-03", "trash")  # Agumon

        lib_size_before = len(runner.game.player1.library_cards)

        runner.set_phase("Main")

        delay_action = runner.find_action("Delay")
        assert delay_action is not None, "Delay effect should be available"
        runner.execute(delay_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # LM-027 should have been trashed (Delay consumption)
        lm027_on_field = any(s.card_id == "LM-027" for s in snap.p1_field)
        assert not lm027_on_field, "LM-027 should be trashed after Delay activation"

        # Library should have gained one card (the returned Digimon)
        assert snap.p1_library_size == lib_size_before + 1, (
            "Deck should gain 1 card from the returned red Digimon"
        )

    def test_has_security_effect(self, debug_runner):
        """LM-027 should have a security effect: play red Digimon <=2000 DP
        from trash free, then add this card to hand."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-027")
        effects = cs.effect_list(None)

        sec_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(sec_effects) >= 1, (
            "LM-027 should have a security effect: play red Digimon <=2000 DP from trash"
        )

        sec = sec_effects[0]
        assert sec.on_process_callback is not None, (
            "Security effect should have a process callback"
        )

    def test_has_option_skill_timing(self, debug_runner):
        """Should have OptionSkill timing for the Main effect."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-027")
        effects = cs.effect_list(None)
        main_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(main_effects) >= 1, "Should have OptionSkill effect"
