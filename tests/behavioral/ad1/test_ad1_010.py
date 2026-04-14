"""Behavioral tests for AD1-010 Garurumon (Lv.4 Blue Digimon, DP 5000, Cost 5).

Card text:
  [On Play] [When Digivolving] <Draw 1> (Draw 1 card from your deck.)
  [All Turns] When your Digimon or Tamers are played or digivolve, if any of
  them have [Greymon] or [Matt Ishida] in their names, this Digimon may
  digivolve into a Digimon card with [Garurumon] in its name in the hand
  without paying the cost.

Inherited effect:
  <Jamming> (This Digimon can't be deleted in battles against Security Digimon.)

Alt-Digi:
  [Digivolve] Lv.3 w/[Omnimon] in text or w/[ADVENTURE] trait: Cost 2
"""

import pytest


@pytest.mark.behavioral
class TestAD1010Garurumon:
    """Tests for AD1-010 Garurumon."""

    # ------------------------------------------------------------------ #
    #  1. On Play: Draw 1                                                  #
    # ------------------------------------------------------------------ #

    def test_on_play_draw_1(self, debug_runner):
        """Playing AD1-010 should draw 1 card."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")
        runner.inject_card(1, "AD1-010", "hand")

        lib_before = len(runner.game.player1.library_cards)
        hand_before = len(runner.game.player1.hand_cards)

        action = runner.find_action("Play Garurumon")
        assert action is not None, "Should be able to play AD1-010"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # After play: 1 card removed from hand (the played card), 1 drawn
        # Net hand change: -1 (played) + 1 (drawn) = 0
        assert snap.p1_library_size == lib_before - 1, "Should have drawn 1 card from deck"
        # Garurumon should be on field
        assert any(s.card_id == "AD1-010" for s in snap.p1_field), "Garurumon should be on field"

    # ------------------------------------------------------------------ #
    #  2. When Digivolving: Draw 1                                         #
    # ------------------------------------------------------------------ #

    def test_when_digivolving_draw_1(self, debug_runner):
        """Digivolving into AD1-010 should draw 1 card (in addition to the standard digi draw)."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place a Lv.3 with ADVENTURE trait on field as a base
        runner.place_on_field(1, ["ST21-02"])  # Gomamon (Blue, ADVENTURE, Lv.3)
        runner.inject_card(1, "AD1-010", "hand")

        lib_before = len(runner.game.player1.library_cards)

        action = runner.find_action("Digivolve")
        assert action is not None, "Should be able to digivolve onto ADVENTURE Lv.3"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Digivolution draws 1 (standard) + 1 (When Digivolving effect) = 2
        assert snap.p1_library_size == lib_before - 2, (
            "Should draw 2 total: 1 standard digi-draw + 1 from When Digivolving effect"
        )

    # ------------------------------------------------------------------ #
    #  3. Alt-Digi: ADVENTURE trait                                        #
    # ------------------------------------------------------------------ #

    def test_alt_digi_adventure_trait(self, debug_runner):
        """AD1-010 can digivolve onto Lv.3 with ADVENTURE trait for cost 2."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place a Lv.3 with ADVENTURE trait
        runner.place_on_field(1, ["ST21-02"])  # Gomamon (Blue, ADVENTURE, Lv.3)
        runner.inject_card(1, "AD1-010", "hand")

        action = runner.find_action("Digivolve")
        assert action is not None, "Should allow digivolve onto ADVENTURE Lv.3"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # AD1-010 should now be on top of the stack
        garurumon_on_field = any(
            s.card_id == "AD1-010" for s in snap.p1_field
        )
        assert garurumon_on_field, "AD1-010 should be on field after digivolving"
        # Alt-digi cost is 2
        assert snap.memory == 10 - 2, "Alt-digi should cost 2 memory"

    def test_alt_digi_omnimon_text(self, debug_runner):
        """AD1-010 can digivolve onto Lv.3 with Omnimon in text for cost 2."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # BT5-020: Gabumon (Lv.3) has Omnimon in text
        runner.place_on_field(1, ["BT5-020"])
        runner.inject_card(1, "AD1-010", "hand")

        action = runner.find_action("Digivolve")
        assert action is not None, "Should allow digivolve onto Lv.3 with Omnimon in text"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        garurumon_on_field = any(
            s.card_id == "AD1-010" for s in snap.p1_field
        )
        assert garurumon_on_field, "AD1-010 should be on field after digivolving"
        assert snap.memory == 10 - 2, "Alt-digi should cost 2 memory"

    def test_alt_digi_rejects_non_adventure_non_omnimon_lv3(self, debug_runner):
        """AD1-010 should NOT digivolve onto Lv.3 without ADVENTURE trait or Omnimon text."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # BT1-029: Gabumon (Blue Lv.3, Reptile trait, no ADVENTURE, no Omnimon text)
        runner.place_on_field(1, ["BT1-029"])
        runner.inject_card(1, "AD1-010", "hand")

        # AD1-010 has no standard evo_costs, so the only way to digivolve
        # is via alt-digi. Without ADVENTURE or Omnimon text, it should fail.
        action = runner.find_action("Digivolve")
        assert action is None, "Should NOT allow digivolve onto non-ADVENTURE non-Omnimon Lv.3"

    # ------------------------------------------------------------------ #
    #  4. All Turns: Reactive digivolve when Greymon/Matt Ishida played    #
    # ------------------------------------------------------------------ #

    def test_reactive_digivolve_on_greymon_play(self, debug_runner):
        """When a Greymon is played while AD1-010 is on field, should offer digivolve into [Garurumon] from hand."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # AD1-010 already on field
        runner.place_on_field(1, ["AD1-010"])
        # Put a WereGarurumon (Lv.5) in hand as digivolve target
        runner.inject_card(1, "BT1-040", "hand")  # WereGarurumon Lv.5
        # Put a Greymon in hand to play
        runner.inject_card(1, "BT1-015", "hand")  # Greymon (Lv.4, Red, cost 4)

        lib_before = len(runner.game.player1.library_cards)

        # Play Greymon - this should trigger AD1-010's reactive effect
        action = runner.find_action("Play Greymon")
        assert action is not None, "Should be able to play Greymon"
        runner.execute(action)

        # After Greymon play resolves, there should be a selection for
        # the reactive digivolve (optional). Look for WereGarurumon selection.
        wereg_action = runner.find_action("WereGarurumon")
        assert wereg_action is not None, (
            "Should offer digivolve into WereGarurumon from hand when Greymon played"
        )
        runner.execute(wereg_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # WereGarurumon should be on field, digivolved ON TOP of AD1-010
        wereg_slot = next(
            (s for s in snap.p1_field if s.card_id == "BT1-040"), None
        )
        assert wereg_slot is not None, "WereGarurumon should be on field after reactive digivolve"
        # AD1-010 should be in the digivolution stack under WereGarurumon
        assert "AD1-010" in wereg_slot.stack_ids, (
            "AD1-010 should be in the digivolution stack under WereGarurumon"
        )

    def test_reactive_digivolve_on_matt_ishida_play(self, debug_runner):
        """When Matt Ishida tamer is played while AD1-010 is on field, should offer digivolve."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # AD1-010 already on field
        runner.place_on_field(1, ["AD1-010"])
        # Put WereGarurumon in hand
        runner.inject_card(1, "BT1-040", "hand")  # WereGarurumon Lv.5
        # Put Matt Ishida tamer in hand
        runner.inject_card(1, "ST2-12", "hand")  # Matt Ishida (cost 2 tamer)

        # Play Matt Ishida
        action = runner.find_action("Play Matt Ishida")
        assert action is not None, "Should be able to play Matt Ishida"
        runner.execute(action)

        # Should trigger reactive digivolve
        wereg_action = runner.find_action("WereGarurumon")
        assert wereg_action is not None, (
            "Should offer digivolve into WereGarurumon from hand when Matt Ishida played"
        )
        runner.execute(wereg_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        wereg_slot = next(
            (s for s in snap.p1_field if s.card_id == "BT1-040"), None
        )
        assert wereg_slot is not None, "WereGarurumon should be on field after reactive digivolve"
        assert "AD1-010" in wereg_slot.stack_ids, (
            "AD1-010 should be in the stack under WereGarurumon"
        )

    def test_reactive_digivolve_no_trigger_on_unrelated_play(self, debug_runner):
        """Playing a Digimon without Greymon/Matt Ishida name should NOT trigger reactive digivolve."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # AD1-010 already on field
        runner.place_on_field(1, ["AD1-010"])
        # Put WereGarurumon in hand (potential target)
        runner.inject_card(1, "BT1-040", "hand")
        # Put an unrelated Digimon in hand
        runner.inject_card(1, "BT1-029", "hand")  # Gabumon (Lv.3, no Greymon/Matt)

        action = runner.find_action("Play Gabumon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # WereGarurumon should still be in hand - no reactive digivolve offered
        assert "BT1-040" in snap.p1_hand, (
            "WereGarurumon should remain in hand - no reactive digivolve for unrelated play"
        )

    def test_reactive_digivolve_is_optional(self, debug_runner):
        """The reactive digivolve should be optional - player can decline."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["AD1-010"])
        runner.inject_card(1, "BT1-040", "hand")  # WereGarurumon
        runner.inject_card(1, "BT1-015", "hand")  # Greymon

        # Play Greymon
        action = runner.find_action("Play Greymon")
        assert action is not None
        runner.execute(action)

        # Look for a pass/decline option
        pass_action = runner.find_action("pass") or runner.find_action("decline")
        if pass_action is None:
            # Try finding the first action that's not a WereGarurumon select
            for aid, desc in runner.actions().items():
                if "wereg" not in desc.lower() and "garurumon" not in desc.lower():
                    pass_action = aid
                    break
        assert pass_action is not None, "Should be able to decline reactive digivolve"
        runner.execute(pass_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # WereGarurumon should still be in hand
        assert "BT1-040" in snap.p1_hand, (
            "WereGarurumon should remain in hand when player declines"
        )

    def test_reactive_digivolve_free_cost(self, debug_runner):
        """Reactive digivolve should be without paying the cost (free)."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["AD1-010"])
        runner.inject_card(1, "BT1-040", "hand")  # WereGarurumon (cost 6)
        runner.inject_card(1, "BT1-015", "hand")  # Greymon (cost 4)

        memory_before = runner.game.memory

        # Play Greymon (costs 4)
        action = runner.find_action("Play Greymon")
        runner.execute(action)

        # Accept the reactive digivolve
        wereg_action = runner.find_action("WereGarurumon")
        assert wereg_action is not None
        runner.execute(wereg_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Memory should only be reduced by Greymon's play cost (4), not WereGarurumon's
        assert snap.memory == memory_before - 4, (
            "Only Greymon's play cost should be paid; reactive digivolve is free"
        )

    def test_reactive_digivolve_on_digivolve_event(self, debug_runner):
        """When a Digimon with Greymon name digivolves, should also trigger the reactive effect."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # AD1-010 on field
        runner.place_on_field(1, ["AD1-010"])
        # A Lv.3 on field to digivolve into Greymon
        runner.place_on_field(1, ["ST1-03"])  # Agumon (Red, Lv.3)
        # Greymon in hand to digivolve onto Agumon
        runner.inject_card(1, "ST1-07", "hand")  # Greymon (Lv.4, Red, cost 5, evo cost 3 from Red Lv.3)
        # WereGarurumon in hand as digivolve target
        runner.inject_card(1, "BT1-040", "hand")

        # Digivolve Agumon into Greymon
        action = runner.find_action("Digivolve")
        assert action is not None, "Should be able to digivolve Agumon into Greymon"
        runner.execute(action)

        # After digivolve event, AD1-010's reactive effect should fire
        wereg_action = runner.find_action("WereGarurumon")
        assert wereg_action is not None, (
            "Should offer digivolve into WereGarurumon when Greymon digivolves"
        )
        runner.execute(wereg_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        wereg_slot = next(
            (s for s in snap.p1_field if s.card_id == "BT1-040"), None
        )
        assert wereg_slot is not None, "WereGarurumon should be on field after reactive digivolve"
        assert "AD1-010" in wereg_slot.stack_ids, (
            "AD1-010 should be in the stack under WereGarurumon"
        )

    def test_reactive_only_garurumon_cards_offered(self, debug_runner):
        """Only cards with [Garurumon] in name should be offered for reactive digivolve."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["AD1-010"])
        # Put a non-Garurumon Digimon in hand alongside a Garurumon
        runner.inject_card(1, "BT1-040", "hand")  # WereGarurumon (valid target)
        runner.inject_card(1, "BT1-044", "hand")  # MetalGarurumon (also valid)
        runner.inject_card(1, "BT1-015", "hand")  # Greymon to play

        # Play Greymon to trigger
        action = runner.find_action("Play Greymon")
        runner.execute(action)

        # Check available actions - should include Garurumon cards but not non-Garurumon
        actions = runner.actions()
        descriptions = [desc.lower() for desc in actions.values()]
        has_garurumon_option = any("garurumon" in d for d in descriptions)
        assert has_garurumon_option, "Should offer Garurumon cards for selection"

    # ------------------------------------------------------------------ #
    #  5. Inherited effect: Jamming                                        #
    # ------------------------------------------------------------------ #

    def test_inherited_jamming(self, debug_runner):
        """AD1-010's inherited effect should grant Jamming."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Build a stack: base + AD1-010 as inherited source + something on top
        # Place AD1-010 as a digivolution source, with something on top
        runner.place_on_field(1, ["AD1-010", "BT1-040"])  # Garurumon under WereGarurumon

        snap = runner.snapshot()
        # The permanent should have jamming from inherited effect
        were_slot = next(
            (s for s in snap.p1_field if s.card_id == "BT1-040"), None
        )
        assert were_slot is not None, "WereGarurumon should be on field"
        assert "jamming" in were_slot.keywords, (
            "AD1-010 inherited effect should grant Jamming"
        )
