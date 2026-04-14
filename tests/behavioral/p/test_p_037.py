"""Behavioral tests for P-037 Yellow Memory Boost! (Option, Yellow, Cost 3).

Card text (from cards.json):
  [Main] Reveal the top 4 cards of your deck. Add 1 yellow Digimon card
  among them to the hand. Place the remaining cards at the bottom of your
  deck in any order. Then, place this card in your battle area.
  [Main] <Delay> (By trashing this card after the placing turn, activate
  the effect below.)
  - Gain 2 memory.
  Security Effect [Security] Place this card in the battle area.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestP037YellowMemoryBoost:
    """Tests for P-037 Yellow Memory Boost!"""

    # ── Structure tests ──────────────────────────────────────────────

    def test_has_option_skill_timing(self, debug_runner):
        """Main effect should have OptionSkill timing."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-037")
        effects = cs.effect_list(None)
        main_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(main_effects) >= 1, "Should have OptionSkill effect"

    def test_has_delay_marker(self, debug_runner):
        """Should have a delay marker effect (_is_delay=True)."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-037")
        effects = cs.effect_list(None)
        delays = [e for e in effects if getattr(e, '_is_delay', False)]
        assert len(delays) >= 1, "Should have a delay marker effect"

    def test_has_security_effect(self, debug_runner):
        """Should have a security effect to place in battle area."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-037")
        effects = cs.effect_list(None)
        sec_effects = [e for e in effects
                       if e.timing == EffectTiming.SecuritySkill
                       or getattr(e, 'is_security_effect', False)]
        assert len(sec_effects) >= 1, "Should have a security effect"

    # ── Main effect (reveal + select) ────────────────────────────────

    def test_main_reveals_top_4_adds_yellow_digimon(self, debug_runner):
        """Playing Yellow Memory Boost reveals top 4 and adds 1 yellow Digimon to hand."""
        runner = debug_runner(initial_memory=10, skip_shuffle=True)
        runner.set_phase("Main")

        # Need a yellow Digimon/Tamer on field to play yellow option
        runner.place_on_field(1, ["ST3-04"])  # Patamon (Yellow Lv3)
        runner.inject_card(1, "P-037", "hand")

        # Stack deck: put a yellow Digimon in top 4
        # ST3-04 (Yellow Digimon) should be selectable
        runner.inject_card(1, "ST3-04", "library_top")

        hand_before = len(runner.game.player1.hand_cards)
        deck_before = len(runner.game.player1.library_cards)

        action = runner.find_action("Yellow Memory Boost")
        assert action is not None, "Should be able to play Yellow Memory Boost"
        runner.execute(action)
        runner.auto_resolve()

        # Card should be placed in battle area as delay
        snap = runner.snapshot()
        in_battle = any(s.card_id == "P-037" for s in snap.p1_field)
        assert in_battle, "Yellow Memory Boost should be placed in the battle area"

    def test_main_filter_only_yellow_digimon(self, debug_runner):
        """Reveal filter should only accept yellow Digimon (not any yellow card)."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()

        # Yellow Digimon -- should pass
        yellow_digi = db.create_card_source("ST3-04")  # Patamon (Yellow)
        assert yellow_digi.is_digimon
        assert any(col.name == 'Yellow' for col in (yellow_digi.card_colors or []))

        # Yellow Option -- should NOT pass (P-037 is "yellow Digimon" not "yellow card")
        yellow_opt = db.create_card_source("ST3-13")  # Heaven's Gate (Yellow Option)
        assert not yellow_opt.is_digimon

        # Blue Digimon -- should NOT pass
        blue_digi = db.create_card_source("BT1-029")  # Gabumon (Blue)
        assert not any(col.name == 'Yellow' for col in (blue_digi.card_colors or []))

    def test_main_places_in_battle_area(self, debug_runner):
        """After resolving, P-037 should be placed in the battle area."""
        runner = debug_runner(initial_memory=10, skip_shuffle=True)
        runner.set_phase("Main")
        runner.place_on_field(1, ["ST3-04"])  # Yellow Digimon on field
        runner.inject_card(1, "P-037", "hand")

        action = runner.find_action("Yellow Memory Boost")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        in_battle = any(s.card_id == "P-037" for s in snap.p1_field)
        assert in_battle, "P-037 should be in battle area after play"

    # ── Delay effect ─────────────────────────────────────────────────

    def test_delay_gains_2_memory(self, debug_runner):
        """Delay effect should gain 2 memory."""
        runner = debug_runner(initial_memory=3, skip_shuffle=True)

        # Place P-037 in battle area from a prior turn
        runner.place_on_field(1, ["P-037"], turn_played=-1)
        runner.set_phase("Main")

        memory_before = runner.game.memory

        delay_action = runner.find_action("Delay")
        assert delay_action is not None, "Delay effect should be available"
        runner.execute(delay_action)
        runner.auto_resolve()

        assert runner.game.memory >= memory_before + 2, (
            f"Should gain 2 memory from delay, was {memory_before}, now {runner.game.memory}"
        )

    def test_delay_trashes_card(self, debug_runner):
        """Delay activation should trash the card from the battle area."""
        runner = debug_runner(initial_memory=3, skip_shuffle=True)
        runner.place_on_field(1, ["P-037"], turn_played=-1)
        runner.set_phase("Main")

        delay_action = runner.find_action("Delay")
        assert delay_action is not None
        runner.execute(delay_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert not any(s.card_id == "P-037" for s in snap.p1_field), (
            "P-037 should be trashed after Delay activation"
        )
        assert "P-037" in snap.p1_trash, (
            "P-037 should be in trash after Delay activation"
        )

    def test_delay_not_available_on_play_turn(self, debug_runner):
        """Delay should NOT be available on the turn the card enters play."""
        runner = debug_runner(initial_memory=10, skip_shuffle=True)

        # Place on current turn (turn_played matches turn_count)
        runner.game.turn_count = 5
        runner.place_on_field(1, ["P-037"], turn_played=5)
        runner.set_phase("Main")

        delay_action = runner.find_action("Delay")
        assert delay_action is None, (
            "Delay should not be available on the turn card enters play"
        )

    # ── Security effect ──────────────────────────────────────────────

    def test_security_places_in_battle_area(self, debug_runner):
        """[Security] should place this card in the battle area (as delay)."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-037")
        effects = cs.effect_list(None)

        # Find the security effect
        sec_effects = [e for e in effects
                       if e.timing == EffectTiming.SecuritySkill
                       or getattr(e, 'is_security_effect', False)]
        assert len(sec_effects) >= 1, "Should have a security effect"
        # Security effect should have a callback that places the card
        sec = sec_effects[0]
        assert sec.on_process_callback is not None, "Security effect should have a callback"
