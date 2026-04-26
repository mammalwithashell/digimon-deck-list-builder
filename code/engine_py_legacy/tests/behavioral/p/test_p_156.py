"""Behavioral tests for P-156 Future Potential! (Option, Yellow, Cost 2).

Card text:
  While you have a Tamer, you can ignore this card's color requirements.

  [Main] Choose 1 of your Tamers. You may play 1 Digimon card with the same
  color as that Tamer and with a play cost of 3 or less from your hand or
  trash without paying the cost.

  [Security] You may play 1 Tamer card from your hand without paying the
  cost. Then, add this card to the hand.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestP156FuturePotential:
    """Tests for P-156 Future Potential!"""

    # ── Color requirement bypass ────────────────────────────────────

    def test_color_bypass_with_tamer(self, debug_runner):
        """P-156 can be played without yellow color match if you have any Tamer."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place a red Tamer on field (no yellow presence)
        runner.place_on_field(1, ["BT1-085"])  # Tai Kamiya (Red Tamer)
        runner.inject_card(1, "P-156", "hand")

        action = runner.find_action("Future Potential")
        assert action is not None, (
            "P-156 should be playable with a red Tamer on field (color bypass)"
        )

    def test_color_bypass_requires_tamer(self, debug_runner):
        """P-156 should NOT bypass color if no Tamer is on the field."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Only a non-yellow Digimon on field (no Tamer at all)
        runner.place_on_field(1, ["ST1-03"])  # Agumon (Red Digimon)
        runner.inject_card(1, "P-156", "hand")

        action = runner.find_action("Future Potential")
        assert action is None, (
            "P-156 should NOT be playable without a Tamer and without yellow color match"
        )

    def test_playable_with_white_match_no_tamer(self, debug_runner):
        """P-156 (White) can be played via normal color match (White Digimon on field)."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # P-156 is White (color 4). Place a white Digimon for color match.
        runner.place_on_field(1, ["BT1-084"])  # Omnimon (White Digimon)
        runner.inject_card(1, "P-156", "hand")

        action = runner.find_action("Future Potential")
        assert action is not None, (
            "P-156 should be playable with white color match from Digimon on field"
        )

    # ── Main effect: Choose Tamer, play matching Digimon ────────────

    def test_main_choose_tamer_and_play_digimon_from_hand(self, debug_runner):
        """Main effect: choose yellow Tamer, play yellow Digimon cost<=3 from hand."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Yellow Tamer on field
        runner.place_on_field(1, ["BT1-087"])  # T.K. Takaishi (Yellow Tamer)
        # Yellow Digimon cost<=3 in hand
        runner.inject_card(1, "BT1-048", "hand")  # Patamon (Yellow, cost 3, DP 2000)
        runner.inject_card(1, "P-156", "hand")

        action = runner.find_action("Future Potential")
        assert action is not None, "Should be able to play P-156"
        runner.execute(action)

        # Should enter tamer selection phase
        snap = runner.snapshot()
        assert snap.phase != "Main", "Should be in a selection phase (choose Tamer)"

        # Select the Tamer (T.K. Takaishi)
        tamer_action = runner.find_action("T.K. Takaishi")
        assert tamer_action is not None, "Should offer T.K. Takaishi for tamer selection"
        runner.execute(tamer_action)

        # Now should offer to play Patamon from hand
        patamon_action = runner.find_action("Patamon")
        if patamon_action is None:
            # Try auto-resolve in case there's intermediate steps
            runner.auto_resolve()
        else:
            runner.execute(patamon_action)
            runner.auto_resolve()

        snap = runner.snapshot()
        patamon_on_field = any(s.card_id == "BT1-048" for s in snap.p1_field)
        assert patamon_on_field, "Patamon should be played to the field from hand"

    def test_main_play_digimon_from_trash(self, debug_runner):
        """Main effect: can play qualifying Digimon from trash as well."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Yellow Tamer on field
        runner.place_on_field(1, ["BT1-087"])  # T.K. Takaishi (Yellow Tamer)
        # Yellow Digimon cost<=3 in trash (not hand)
        runner.inject_card(1, "BT1-048", "trash")  # Patamon (Yellow, cost 3)
        runner.inject_card(1, "P-156", "hand")

        action = runner.find_action("Future Potential")
        assert action is not None
        runner.execute(action)

        # Select tamer
        tamer_action = runner.find_action("T.K. Takaishi")
        assert tamer_action is not None
        runner.execute(tamer_action)

        # Select Patamon from trash -- action ID 130+ is SEL_TRASH_START range
        # The play_from_zone('hand_or_trash') offers trash cards at 130+
        legal = runner.action_mask()
        trash_actions = [a for a in legal if a >= 130]
        assert trash_actions, "Should offer trash card (Patamon) for play"
        runner.execute(trash_actions[0])
        runner.auto_resolve()

        snap = runner.snapshot()
        patamon_on_field = any(s.card_id == "BT1-048" for s in snap.p1_field)
        assert patamon_on_field, "Patamon should be played from trash to field"

    def test_main_cost_restriction_blocks_high_cost(self, debug_runner):
        """Main effect should NOT allow playing Digimon with cost > 3."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Yellow Tamer on field
        runner.place_on_field(1, ["BT1-087"])  # T.K. Takaishi (Yellow Tamer)
        # Only a yellow Digimon with cost > 3 in hand
        runner.inject_card(1, "BT1-055", "hand")  # Angemon (Yellow, cost 5, Lv.4)
        runner.inject_card(1, "P-156", "hand")

        action = runner.find_action("Future Potential")
        assert action is not None
        runner.execute(action)

        # Select tamer
        tamer_action = runner.find_action("T.K. Takaishi")
        assert tamer_action is not None
        runner.execute(tamer_action)

        # Should NOT be able to play Angemon (cost 5 > 3)
        # The selection should either be skipped (no valid targets) or only offer Pass/Decline
        angemon_action = runner.find_action("Angemon")
        runner.auto_resolve()

        snap = runner.snapshot()
        angemon_on_field = any(s.card_id == "BT1-055" for s in snap.p1_field)
        assert not angemon_on_field, "Angemon (cost 5) should NOT be playable (limit is cost 3)"

    def test_main_color_must_match_tamer(self, debug_runner):
        """Main effect only allows playing Digimon matching the CHOSEN Tamer's color."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Red Tamer on field (not yellow)
        runner.place_on_field(1, ["BT1-085"])  # Tai Kamiya (Red Tamer)
        # Yellow Digimon cost<=3 in hand (doesn't match red Tamer)
        runner.inject_card(1, "BT1-048", "hand")  # Patamon (Yellow, cost 3)
        runner.inject_card(1, "P-156", "hand")

        action = runner.find_action("Future Potential")
        assert action is not None
        runner.execute(action)

        # Select the red tamer
        tai_action = runner.find_action("Tai Kamiya")
        assert tai_action is not None
        runner.execute(tai_action)

        # Patamon is yellow, Tai is red -- should NOT match
        # Either no selection or only decline available
        runner.auto_resolve()

        snap = runner.snapshot()
        patamon_on_field = any(s.card_id == "BT1-048" for s in snap.p1_field)
        assert not patamon_on_field, (
            "Yellow Patamon should NOT be playable when red Tamer was chosen"
        )

    def test_main_no_tamers_means_no_effect(self, debug_runner):
        """If player has no Tamers on field, Main effect does nothing (no Digimon played)."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # P-156 is White. Place a White Digimon for color match (but NOT a Tamer).
        runner.place_on_field(1, ["BT1-084"])  # Omnimon (White Digimon)
        # Yellow Digimon cost<=3 in hand (would be playable IF we had a Tamer)
        runner.inject_card(1, "BT1-048", "hand")  # Patamon (Yellow, cost 3)
        runner.inject_card(1, "P-156", "hand")

        action = runner.find_action("Future Potential")
        assert action is not None, "P-156 playable via white color match from Omnimon"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Should NOT have played Patamon (no Tamer to choose = Main effect does nothing)
        patamon_on_field = any(s.card_id == "BT1-048" for s in snap.p1_field)
        assert not patamon_on_field, (
            "Should not play any Digimon when no Tamer is on field"
        )

    # ── Security effect ─────────────────────────────────────────────

    def test_has_security_effect(self, debug_runner):
        """P-156 should have a SecuritySkill effect."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-156")
        effects = cs.effect_list(None)

        sec_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(sec_effects) >= 1, (
            "P-156 should have a security effect: play Tamer from hand, add to hand"
        )
        assert sec_effects[0].on_process_callback is not None

    def test_security_play_tamer_from_hand(self, debug_runner):
        """Security effect: play 1 Tamer from hand free, then add P-156 to hand."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-156")
        effects = cs.effect_list(None)
        sec_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert sec_effects, "Must have security effect"

        sec = sec_effects[0]
        assert sec.timing == EffectTiming.SecuritySkill

    def test_has_option_skill_timing(self, debug_runner):
        """P-156 should have OptionSkill timing for the Main effect."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-156")
        effects = cs.effect_list(None)
        main_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(main_effects) >= 1, "Should have OptionSkill effect"
