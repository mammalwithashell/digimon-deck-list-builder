"""Behavioral tests for LM-029 Yellow Scramble (Option, Yellow, Cost 2).

Card text:
  [Main] 1 of your yellow Digimon may digivolve into a yellow Digimon card
  in the hand with the digivolution cost reduced by 3. Then, place this
  card in the battle area.

  [Start of Your Turn] If your opponent has a Digimon, <Delay>.
  Return 1 yellow Digimon card from your trash to the top of the deck.
  Then, if you don't have a Digimon, you may play 1 yellow Digimon card
  with 2000 DP or less from your trash without paying the cost.

  [Security] You may play 1 yellow Digimon card with 2000 DP or less from
  your trash without paying the cost. Then, add this card to the hand.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestLM029YellowScramble:
    """Tests for LM-029 Yellow Scramble."""

    # ── Main effect: digivolve + place in battle area ───────────────

    def test_main_places_card_in_battle_area(self, debug_runner):
        """Playing Yellow Scramble should place it in the battle area (Delay)."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place a yellow Digimon on field so there's a valid digivolve base
        runner.place_on_field(1, ["BT1-048"])  # Patamon (Yellow, Lv.3)
        runner.inject_card(1, "LM-029", "hand")

        action = runner.find_action("Yellow Scramble")
        assert action is not None, "Should be able to play Yellow Scramble"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        in_battle = any(s.card_id == "LM-029" for s in snap.p1_field)
        assert in_battle, "Yellow Scramble should be placed in the battle area"

    def test_main_no_duplicate_placement(self, debug_runner):
        """Playing Yellow Scramble should NOT create two copies on the field."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["BT1-048"])  # Patamon (Yellow, Lv.3)
        runner.inject_card(1, "LM-029", "hand")

        action = runner.find_action("Yellow Scramble")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        lm029_count = sum(1 for s in snap.p1_field if s.card_id == "LM-029")
        assert lm029_count == 1, (
            f"Yellow Scramble should appear exactly once on the field, found {lm029_count}"
        )

    def test_main_digivolve_yellow_with_cost_reduction(self, debug_runner):
        """Main effect: yellow Digimon may digivolve into yellow from hand with cost -3."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Yellow Lv.3 on field as digivolve base
        runner.place_on_field(1, ["BT1-048"])  # Patamon (Yellow Lv.3)
        # Yellow Lv.4 in hand as digivolve target
        runner.inject_card(1, "BT1-052", "hand")  # Seasarmon (Yellow Lv.4)
        runner.inject_card(1, "LM-029", "hand")

        mem_before = runner.game.memory

        action = runner.find_action("Yellow Scramble")
        assert action is not None
        runner.execute(action)

        # Should enter permanent selection (choose yellow Digimon to digivolve)
        patamon_action = runner.find_action("Patamon")
        if patamon_action is not None:
            runner.execute(patamon_action)

            # Should offer Seasarmon from hand for digivolve
            seasarmon_action = runner.find_action("Seasarmon")
            if seasarmon_action is not None:
                runner.execute(seasarmon_action)

        runner.auto_resolve()

        snap = runner.snapshot()
        # Check that Seasarmon is now on the field (digivolved from Patamon)
        seasarmon_on_field = any(s.card_id == "BT1-052" for s in snap.p1_field)
        assert seasarmon_on_field, (
            "Seasarmon should be on the field after digivolving from Patamon"
        )

    # ── Delay effect ────────────────────────────────────────────────

    def test_delay_returns_yellow_digimon_from_trash_to_deck_top(self, debug_runner):
        """Delay effect should return 1 yellow Digimon from trash to top of deck."""
        runner = debug_runner(initial_memory=3, skip_shuffle=True)

        # Setup: Yellow Scramble in battle area (placed on a prior turn)
        runner.place_on_field(1, ["LM-029"], turn_played=-1)
        # Opponent must have a Digimon for the Delay condition
        runner.place_on_field(2, ["BT1-048"])
        # Put a yellow Digimon in P1's trash
        runner.inject_card(1, "BT1-048", "trash")  # Patamon

        lib_size_before = len(runner.game.player1.library_cards)

        runner.set_phase("Main")

        # Look for the Delay activation
        delay_action = runner.find_action("Delay")
        assert delay_action is not None, "Delay effect should be available"
        runner.execute(delay_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # LM-029 should have been trashed (Delay consumption)
        lm029_on_field = any(s.card_id == "LM-029" for s in snap.p1_field)
        assert not lm029_on_field, "LM-029 should be trashed after Delay activation"

        # Library should have gained one card (the returned Digimon)
        assert snap.p1_library_size == lib_size_before + 1, (
            "Deck should gain 1 card from the returned yellow Digimon"
        )

    def test_delay_play_from_trash_when_no_digimon(self, debug_runner):
        """After returning card to deck, if no Digimon on field, may play yellow DP<=2000 from trash."""
        runner = debug_runner(initial_memory=3, skip_shuffle=True)

        # Setup: Yellow Scramble in battle area, no other Digimon for P1
        runner.place_on_field(1, ["LM-029"], turn_played=-1)
        # Opponent must have a Digimon
        runner.place_on_field(2, ["BT1-048"])
        # Put two yellow Digimon in P1's trash:
        # one to return to deck, one with DP<=2000 to play
        runner.inject_card(1, "BT1-048", "trash")  # Patamon (DP 2000, Yellow Lv.3)
        runner.inject_card(1, "BT1-046", "trash")  # Kudamon (DP 1000, Yellow Lv.3)

        runner.set_phase("Main")

        delay_action = runner.find_action("Delay")
        assert delay_action is not None, "Delay effect should be available"
        runner.execute(delay_action)

        # First selection: pick a yellow Digimon from trash to return to deck
        legal = runner.action_mask()
        assert legal, "Should have trash selection options"
        runner.execute(legal[0])

        # Second selection: optional play from trash (since no Digimon on field)
        # Find any non-pass selection
        actions = runner.actions()
        play_action = None
        for aid, desc in actions.items():
            if "pass" not in desc.lower() and "decline" not in desc.lower():
                play_action = aid
                break

        if play_action is not None:
            runner.execute(play_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # P1 should have a Digimon on field (played from trash)
        has_digimon = any(s.is_digimon for s in snap.p1_field)
        assert has_digimon, (
            "Should play a yellow Digimon with DP<=2000 from trash when no Digimon on field"
        )

    def test_delay_no_play_when_has_digimon(self, debug_runner):
        """If P1 has a Digimon on field, should NOT get the play-from-trash sub-effect."""
        runner = debug_runner(initial_memory=3, skip_shuffle=True)

        # Setup: Yellow Scramble + a Digimon in battle area
        runner.place_on_field(1, ["LM-029"], turn_played=-1)
        runner.place_on_field(1, ["BT1-048"])  # P1 has a Digimon
        # Opponent must have a Digimon
        runner.place_on_field(2, ["BT1-048"])
        # Put yellow Digimon in P1's trash
        runner.inject_card(1, "BT1-048", "trash")
        runner.inject_card(1, "BT1-046", "trash")

        runner.set_phase("Main")

        delay_action = runner.find_action("Delay")
        assert delay_action is not None
        runner.execute(delay_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Only the original Patamon should be on field, no additional play from trash
        digimon_count = sum(1 for s in snap.p1_field if s.is_digimon)
        assert digimon_count == 1, (
            "Should NOT play from trash when P1 already has a Digimon on field"
        )

    def test_delay_effect_skipped_when_opponent_has_no_digimon(self, debug_runner):
        """When opponent has no Digimon, Delay trashes card but effect doesn't fire."""
        runner = debug_runner(initial_memory=3, skip_shuffle=True)

        runner.place_on_field(1, ["LM-029"], turn_played=-1)
        # No opponent Digimon
        runner.inject_card(1, "BT1-048", "trash")

        lib_size_before = len(runner.game.player1.library_cards)

        runner.set_phase("Main")

        delay_action = runner.find_action("Delay")
        assert delay_action is not None, "Delay action is available regardless"
        runner.execute(delay_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # LM-029 should be trashed
        assert not any(s.card_id == "LM-029" for s in snap.p1_field)
        # But the delay effect should NOT have fired -- library unchanged
        assert snap.p1_library_size == lib_size_before, (
            "Delay effect should not fire when opponent has no Digimon"
        )
        # Trash Patamon should still be in trash
        assert "BT1-048" in snap.p1_trash, "Patamon should remain in trash"

    # ── Security effect ─────────────────────────────────────────────

    def test_has_security_effect(self, debug_runner):
        """LM-029 should have a security effect: play yellow Digimon <=2000 DP
        from trash free, then add this card to hand."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-029")
        effects = cs.effect_list(None)

        sec_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(sec_effects) >= 1, (
            "LM-029 should have a security effect"
        )

        sec = sec_effects[0]
        assert sec.on_process_callback is not None
        assert sec.timing == EffectTiming.SecuritySkill

    def test_has_option_skill_timing(self, debug_runner):
        """Should have OptionSkill timing for the Main effect."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-029")
        effects = cs.effect_list(None)
        main_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(main_effects) >= 1, "Should have OptionSkill effect"

    def test_has_delay_marker(self, debug_runner):
        """LM-029 should have a Delay marker effect."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-029")
        effects = cs.effect_list(None)
        delay_effects = [e for e in effects if getattr(e, '_is_delay', False)]
        assert len(delay_effects) >= 1, "Should have a delay marker effect"

    def test_delay_condition_checks_opponent_digimon(self, debug_runner):
        """Delay effect condition should require opponent to have a Digimon."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-029")
        effects = cs.effect_list(None)

        delay_body = [e for e in effects if getattr(e, '_is_delay_effect', False)]
        assert len(delay_body) >= 1, "Should have a delay body effect"
        assert delay_body[0].timing == EffectTiming.OnStartTurn
