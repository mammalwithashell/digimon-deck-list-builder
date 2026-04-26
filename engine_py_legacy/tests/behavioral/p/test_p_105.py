"""Behavioral tests for P-105 Physical Training (Option, Yellow, Cost 2).

Card text (from cards.json):
  [Main] Reveal the top 2 cards of your deck. Add 1 yellow card among them
  to your hand. Place the rest at the bottom of your deck in any order.
  Then, place this card in the battle area.
  [Main] <Delay> (By trashing this card after the placing turn, activate
  the effect below.)
  - 1 of your Digimon may digivolve into a yellow Digimon card in your hand
    for its digivolution cost. When it would digivolve by this effect,
    reduce the cost by 2.
  Security Effect [Security] Place this card in the battle area.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestP105PhysicalTraining:
    """Tests for P-105 Physical Training."""

    # ── Structure tests ──────────────────────────────────────────────

    def test_has_option_skill_timing(self, debug_runner):
        """Main effect should have OptionSkill timing."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-105")
        effects = cs.effect_list(None)
        main_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(main_effects) >= 1, "Should have OptionSkill effect"

    def test_has_delay_marker(self, debug_runner):
        """Should have a delay marker effect (_is_delay=True)."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-105")
        effects = cs.effect_list(None)
        delays = [e for e in effects if getattr(e, '_is_delay', False)]
        assert len(delays) >= 1, "Should have a delay marker effect"

    def test_has_security_effect(self, debug_runner):
        """Should have a security effect to place in battle area."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-105")
        effects = cs.effect_list(None)
        sec_effects = [e for e in effects
                       if e.timing == EffectTiming.SecuritySkill
                       or getattr(e, 'is_security_effect', False)]
        assert len(sec_effects) >= 1, "Should have a security effect"

    # ── Main effect (reveal + select) ────────────────────────────────

    def test_main_reveals_top_2_adds_yellow_card(self, debug_runner):
        """Playing Physical Training reveals top 2 and adds 1 yellow card to hand."""
        runner = debug_runner(initial_memory=10, skip_shuffle=True)
        runner.set_phase("Main")

        # Need a yellow Digimon/Tamer on field to play yellow option
        runner.place_on_field(1, ["ST3-04"])  # Patamon (Yellow Lv3)
        runner.inject_card(1, "P-105", "hand")

        # Stack deck: put a yellow card in top 2
        runner.inject_card(1, "ST3-04", "library_top")  # Yellow Digimon

        action = runner.find_action("Physical Training")
        assert action is not None, "Should be able to play Physical Training"
        runner.execute(action)
        runner.auto_resolve()

        # Card should be placed in battle area as delay
        snap = runner.snapshot()
        in_battle = any(s.card_id == "P-105" for s in snap.p1_field)
        assert in_battle, "Physical Training should be placed in the battle area"

    def test_main_filter_any_yellow_card(self, debug_runner):
        """Reveal filter should accept ANY yellow card (Digimon, Tamer, or Option)."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()

        # Yellow Digimon -- should pass
        yellow_digi = db.create_card_source("ST3-04")  # Patamon (Yellow Digimon)
        assert any(col.name == 'Yellow' for col in (yellow_digi.card_colors or []))

        # Yellow Option -- should ALSO pass (P-105 is "yellow card" not "yellow Digimon")
        yellow_opt = db.create_card_source("ST3-13")  # Heaven's Gate (Yellow Option)
        assert any(col.name == 'Yellow' for col in (yellow_opt.card_colors or []))

        # Yellow Tamer -- should ALSO pass
        yellow_tamer = db.create_card_source("ST3-12")  # T.K. Takaishi (Yellow Tamer)
        assert any(col.name == 'Yellow' for col in (yellow_tamer.card_colors or []))

        # Blue Digimon -- should NOT pass
        blue_digi = db.create_card_source("BT1-029")  # Gabumon (Blue)
        assert not any(col.name == 'Yellow' for col in (blue_digi.card_colors or []))

    def test_main_places_in_battle_area(self, debug_runner):
        """After resolving, P-105 should be placed in the battle area."""
        runner = debug_runner(initial_memory=10, skip_shuffle=True)
        runner.set_phase("Main")
        runner.place_on_field(1, ["ST3-04"])  # Yellow Digimon on field
        runner.inject_card(1, "P-105", "hand")

        action = runner.find_action("Physical Training")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        in_battle = any(s.card_id == "P-105" for s in snap.p1_field)
        assert in_battle, "P-105 should be in battle area after play"

    # ── Delay effect ─────────────────────────────────────────────────

    def test_delay_available_after_placement_turn(self, debug_runner):
        """Delay should be available on a subsequent turn."""
        runner = debug_runner(initial_memory=3, skip_shuffle=True)
        runner.place_on_field(1, ["P-105"], turn_played=-1)
        # Also need a Digimon target on field for the digivolve effect
        runner.place_on_field(1, ["ST3-04"])  # Patamon (Yellow Lv3)
        runner.set_phase("Main")

        delay_action = runner.find_action("Delay")
        assert delay_action is not None, "Delay effect should be available"

    def test_delay_trashes_card(self, debug_runner):
        """Delay activation should trash the card from the battle area."""
        runner = debug_runner(initial_memory=3, skip_shuffle=True)
        runner.place_on_field(1, ["P-105"], turn_played=-1)
        runner.place_on_field(1, ["ST3-04"])  # Digimon target
        # Put a yellow Digimon in hand for digivolve target
        runner.inject_card(1, "ST3-06", "hand")  # Gatomon (Yellow Lv4)
        runner.set_phase("Main")

        delay_action = runner.find_action("Delay")
        assert delay_action is not None
        runner.execute(delay_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert not any(s.card_id == "P-105" for s in snap.p1_field), (
            "P-105 should be trashed after Delay activation"
        )
        assert "P-105" in snap.p1_trash, (
            "P-105 should be in trash after Delay activation"
        )

    def test_delay_offers_digivolve_with_cost_reduction(self, debug_runner):
        """Delay should allow digivolving a Digimon into a yellow Digimon from hand with cost -2."""
        runner = debug_runner(initial_memory=5, skip_shuffle=True)
        runner.place_on_field(1, ["P-105"], turn_played=-1)
        # Place a yellow Lv3 Digimon on field (digivolve target)
        runner.place_on_field(1, ["ST3-04"])  # Patamon (Yellow Lv3)
        # Put a yellow Lv4 in hand (digivolve source)
        # ST3-06 Gatomon: Yellow Lv4, evolves from Yellow Lv3 for 2 memory
        # With cost reduction of 2, it should cost 0.
        runner.inject_card(1, "ST3-06", "hand")
        runner.set_phase("Main")

        memory_before = runner.game.memory

        delay_action = runner.find_action("Delay")
        assert delay_action is not None, "Delay should be available"
        runner.execute(delay_action)

        # After delay, should enter SelectTarget for picking which Digimon
        from digimon_gym.engine.data.enums import GamePhase
        assert runner.game.current_phase == GamePhase.SelectTarget, (
            "Should enter SelectTarget to pick a Digimon"
        )

        # Select the Patamon (the own Digimon to digivolve)
        select_action = runner.find_action("Patamon")
        assert select_action is not None, "Should be able to select Patamon as digivolve target"
        runner.execute(select_action)

        # Now should enter another SelectTarget for picking hand card to digivolve into
        # Find and select Gatomon from hand
        gatomon_action = runner.find_action("Gatomon")
        if gatomon_action is not None:
            runner.execute(gatomon_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # P-105 should be trashed
        assert "P-105" in snap.p1_trash

        # Gatomon should now be on the field (digivolved from Patamon)
        gatomon_on_field = any(
            s.card_id == "ST3-06" or "ST3-06" in s.stack_ids
            for s in snap.p1_field
        )
        assert gatomon_on_field, (
            "Delay should have offered digivolve; Gatomon should be on field"
        )

    def test_delay_digivolve_cost_reduced_by_2(self, debug_runner):
        """Digivolve via delay should cost 2 less than normal.

        NOTE: effect_digivolve_from_hand uses play cost as the base,
        not the evo cost from evolution requirements.  This matches
        all sibling Training cards (P-103/104/106/107/108).
        Gatomon play cost 4, minus reduction 2 = 2 memory.
        """
        runner = debug_runner(initial_memory=5, skip_shuffle=True)
        runner.place_on_field(1, ["P-105"], turn_played=-1)
        runner.place_on_field(1, ["ST3-04"])  # Patamon (Yellow Lv3)
        runner.inject_card(1, "ST3-06", "hand")  # Gatomon (Yellow Lv4, play cost 4)
        runner.set_phase("Main")

        # Clear hand of other cards so only Gatomon remains as digivolve source
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "ST3-06", "hand")

        memory_before = runner.game.memory

        delay_action = runner.find_action("Delay")
        assert delay_action is not None
        runner.execute(delay_action)

        # Select Patamon as the digivolve target
        select_action = runner.find_action("Patamon")
        assert select_action is not None
        runner.execute(select_action)

        # Select Gatomon from hand to digivolve into
        gatomon_action = runner.find_action("Gatomon")
        if gatomon_action is not None:
            runner.execute(gatomon_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # First confirm the digivolve actually happened
        gatomon_on_field = any(
            s.card_id == "ST3-06" or "ST3-06" in s.stack_ids
            for s in snap.p1_field
        )
        assert gatomon_on_field, (
            "Digivolve should have happened; Gatomon should be on field"
        )

        # Gatomon play cost 4 - reduction 2 = 2 memory spent
        expected_memory = memory_before - 2
        assert runner.game.memory == expected_memory, (
            f"Digivolve with cost -2 on play cost 4 should cost 2, "
            f"memory was {memory_before}, now {runner.game.memory}"
        )

    def test_delay_not_available_on_play_turn(self, debug_runner):
        """Delay should NOT be available on the turn the card enters play."""
        runner = debug_runner(initial_memory=10, skip_shuffle=True)
        runner.game.turn_count = 5
        runner.place_on_field(1, ["P-105"], turn_played=5)
        runner.set_phase("Main")

        delay_action = runner.find_action("Delay")
        assert delay_action is None, (
            "Delay should not be available on the turn card enters play"
        )

    def test_delay_digivolve_filter_only_yellow_digimon(self, debug_runner):
        """Delay digivolve should only allow digivolving into yellow Digimon cards."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()

        # Yellow Digimon -- should be valid digivolve target
        yellow_digi = db.create_card_source("ST3-06")
        assert yellow_digi.is_digimon
        assert any(col.name == 'Yellow' for col in (yellow_digi.card_colors or []))

        # Blue Digimon -- should NOT be valid
        blue_digi = db.create_card_source("BT1-029")
        assert blue_digi.is_digimon
        assert not any(col.name == 'Yellow' for col in (blue_digi.card_colors or []))

    # ── Security effect ──────────────────────────────────────────────

    def test_security_places_in_battle_area(self, debug_runner):
        """[Security] should place this card in the battle area (as delay)."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-105")
        effects = cs.effect_list(None)

        # Find the security effect
        sec_effects = [e for e in effects
                       if e.timing == EffectTiming.SecuritySkill
                       or getattr(e, 'is_security_effect', False)]
        assert len(sec_effects) >= 1, "Should have a security effect"
        sec = sec_effects[0]
        assert sec.on_process_callback is not None, "Security effect should have a callback"
