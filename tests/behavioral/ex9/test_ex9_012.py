"""Behavioral tests for EX9-012 MetalGreymon: Alterous Mode (Lv.5 Red/Purple).

Card text:
  Alt-digi 1: [Digivolve] [MetalGreymon]: Cost 1 (exact name, any level)
  Alt-digi 2: [Digivolve] Lv.4 w/[Greymon] in name or w/[ADVENTURE] trait: Cost 3
  [On Play] [When Digivolving] Delete 1 of your opponent's Digimon with 8000 DP or less.
  [Your Turn] When any of your Digimon or Tamers with [Garurumon] or [Tai Kamiya]
    in their names are played or any of your Digimon digivolve into a Digimon
    with [Garurumon] in its name, this Digimon may digivolve into a Digimon card
    with [Greymon] in its name in the hand without paying the cost.
  Inherited: [Your Turn] This Digimon gets +4000 DP.
"""

import pytest


@pytest.mark.behavioral
class TestEX9012OnPlayDelete:
    """[On Play] Delete 1 of your opponent's Digimon with 8000 DP or less."""

    def test_on_play_deletes_opponent_digimon_le_8000(self, debug_runner):
        """Playing EX9-012 should delete an opponent Digimon with DP <= 8000."""
        runner = debug_runner(initial_memory=10)

        # Place an opponent Digimon with DP <= 8000 (AD1-001 Greymon, 5000 DP)
        runner.place_on_field(2, ["AD1-001"])
        # Inject EX9-012 into hand
        runner.inject_card(1, "EX9-012", "hand")

        runner.set_phase("Main")

        play = runner.find_action("Play MetalGreymon: Alterous Mode")
        assert play is not None, "Should be able to play EX9-012 from hand"
        runner.execute(play)
        results = runner.auto_resolve()

        snap = runner.snapshot()
        # Opponent's Digimon should have been deleted
        opp_digimon = [s for s in snap.p2_field if s.card_id == "AD1-001"]
        assert len(opp_digimon) == 0, "Opponent's 5000 DP Digimon should be deleted"
        # EX9-012 should be on our field
        our_card = [s for s in snap.p1_field if s.card_id == "EX9-012"]
        assert len(our_card) == 1, "EX9-012 should be on our field"

    def test_on_play_does_not_delete_digimon_over_8000(self, debug_runner):
        """Opponent Digimon with DP > 8000 should not be deletable."""
        runner = debug_runner(initial_memory=10)

        # Place an opponent Digimon with DP > 8000
        runner.place_on_field(2, ["BT1-025"])  # WarGreymon, 11000 DP
        runner.inject_card(1, "EX9-012", "hand")

        runner.set_phase("Main")

        play = runner.find_action("Play MetalGreymon: Alterous Mode")
        assert play is not None
        runner.execute(play)
        results = runner.auto_resolve()

        snap = runner.snapshot()
        # Opponent Digimon should still be on field (DP > 8000)
        opp_digimon = [s for s in snap.p2_field if s.card_id == "BT1-025"]
        assert len(opp_digimon) == 1, "Opponent's 11000 DP Digimon should NOT be deleted"


@pytest.mark.behavioral
class TestEX9012WhenDigivolvingDelete:
    """[When Digivolving] Delete 1 of your opponent's Digimon with 8000 DP or less."""

    def test_when_digivolving_deletes_opponent_digimon(self, debug_runner):
        """Digivolving into EX9-012 should delete an opponent Digimon with DP <= 8000."""
        runner = debug_runner(initial_memory=10)

        # Place a Lv.4 Greymon on our field (for alt-digi cost 3)
        runner.place_on_field(1, ["ST1-07"])  # Greymon, Lv.4
        # Place an opponent Digimon with DP <= 8000
        runner.place_on_field(2, ["AD1-001"])  # Greymon 5000 DP
        # Inject EX9-012 into hand
        runner.inject_card(1, "EX9-012", "hand")

        runner.set_phase("Main")

        # Find the digivolve action
        digivolve = runner.find_action("Digivolve")
        assert digivolve is not None, "Should be able to digivolve into EX9-012"
        runner.execute(digivolve)
        results = runner.auto_resolve()

        snap = runner.snapshot()
        # Opponent Digimon should be deleted
        opp_digimon = [s for s in snap.p2_field if s.card_id == "AD1-001"]
        assert len(opp_digimon) == 0, "Opponent's 5000 DP Digimon should be deleted on digivolve"


@pytest.mark.behavioral
class TestEX9012ReactiveDigivolve:
    """[Your Turn] Reactive digivolve into [Greymon] from hand for free."""

    def test_play_garurumon_triggers_reactive_digivolve(self, debug_runner):
        """Playing a Garurumon should let this Digimon digivolve into a Greymon from hand free."""
        runner = debug_runner(initial_memory=10)

        # Place EX9-012 on field
        runner.place_on_field(1, ["EX9-012"])
        # Inject a Garurumon into hand to be played
        runner.inject_card(1, "BT1-036", "hand")  # Garurumon Lv.4
        # Inject a Greymon card for the free digivolve
        runner.inject_card(1, "BT1-021", "hand")  # MetalGreymon Lv.5

        runner.set_phase("Main")

        play = runner.find_action("Play Garurumon")
        assert play is not None, "Should be able to play Garurumon"
        runner.execute(play)
        results = runner.auto_resolve()

        snap = runner.snapshot()
        # EX9-012 should have digivolved into the MetalGreymon from hand
        # Look for a stack that has BT1-021 on top with EX9-012 underneath
        found_digivolved = False
        for s in snap.p1_field:
            if s.card_id == "BT1-021" and "EX9-012" in s.stack_ids:
                found_digivolved = True
                break
        assert found_digivolved, (
            "EX9-012 should have digivolved into BT1-021 (MetalGreymon) from hand. "
            f"Field: {[(s.card_id, s.stack_ids) for s in snap.p1_field]}"
        )

    def test_play_tai_kamiya_triggers_reactive_digivolve(self, debug_runner):
        """Playing a Tai Kamiya tamer should trigger the reactive digivolve."""
        runner = debug_runner(initial_memory=10)

        # Place EX9-012 on field
        runner.place_on_field(1, ["EX9-012"])
        # Inject a Tai Kamiya tamer into hand
        runner.inject_card(1, "ST1-12", "hand")  # Tai Kamiya
        # Inject a Greymon card for the free digivolve
        runner.inject_card(1, "BT1-021", "hand")  # MetalGreymon Lv.5

        runner.set_phase("Main")

        play = runner.find_action("Play Tai Kamiya")
        assert play is not None, "Should be able to play Tai Kamiya"
        runner.execute(play)
        results = runner.auto_resolve()

        snap = runner.snapshot()
        # EX9-012 should have digivolved into MetalGreymon from hand
        found_digivolved = False
        for s in snap.p1_field:
            if s.card_id == "BT1-021" and "EX9-012" in s.stack_ids:
                found_digivolved = True
                break
        assert found_digivolved, (
            "Playing Tai Kamiya should trigger reactive digivolve. "
            f"Field: {[(s.card_id, s.stack_ids) for s in snap.p1_field]}"
        )

    def test_digivolve_into_garurumon_triggers_reactive(self, debug_runner):
        """Digivolving into a Garurumon should trigger the reactive digivolve."""
        runner = debug_runner(initial_memory=10)

        # Place EX9-012 on field
        runner.place_on_field(1, ["EX9-012"])
        # Place a Lv.3 for digivolving into Garurumon
        runner.place_on_field(1, ["BT1-029"])  # Gabumon Lv.3
        # Inject Garurumon into hand (for the digivolve source)
        runner.inject_card(1, "BT1-036", "hand")  # Garurumon Lv.4
        # Inject a Greymon card for the free digivolve target
        runner.inject_card(1, "BT1-021", "hand")  # MetalGreymon Lv.5

        runner.set_phase("Main")

        # Digivolve Gabumon into Garurumon
        digivolve = runner.find_action("Digivolve")
        assert digivolve is not None, "Should be able to digivolve Gabumon into Garurumon"
        runner.execute(digivolve)
        results = runner.auto_resolve()

        snap = runner.snapshot()
        # EX9-012 should have digivolved into BT1-021 from hand
        found_digivolved = False
        for s in snap.p1_field:
            if s.card_id == "BT1-021" and "EX9-012" in s.stack_ids:
                found_digivolved = True
                break
        assert found_digivolved, (
            "Digivolving into Garurumon should trigger reactive digivolve. "
            f"Field: {[(s.card_id, s.stack_ids) for s in snap.p1_field]}"
        )

    def test_no_trigger_on_opponent_garurumon_play(self, debug_runner):
        """Opponent playing Garurumon should NOT trigger the reactive digivolve."""
        runner = debug_runner(initial_memory=10)

        # Place EX9-012 on our field
        runner.place_on_field(1, ["EX9-012"])
        # Inject a Greymon card for the (should-not-happen) digivolve
        runner.inject_card(1, "BT1-021", "hand")  # MetalGreymon Lv.5

        # Place Garurumon on opponent's field (simulates opponent playing it)
        runner.place_on_field(2, ["BT1-036"])  # Garurumon on opponent field

        snap = runner.snapshot()
        # EX9-012 should still be on our field untouched
        our_card = [s for s in snap.p1_field if s.card_id == "EX9-012"]
        assert len(our_card) == 1, "EX9-012 should remain unchanged (opponent's play)"

    def test_no_trigger_without_greymon_in_hand(self, debug_runner):
        """If there's no Greymon card in hand, the reactive digivolve should not be offered."""
        runner = debug_runner(initial_memory=10)

        # Place EX9-012 on field
        runner.place_on_field(1, ["EX9-012"])
        # Inject a Garurumon (trigger) but no Greymon target
        runner.inject_card(1, "BT1-036", "hand")  # Garurumon Lv.4

        runner.set_phase("Main")

        play = runner.find_action("Play Garurumon")
        assert play is not None
        runner.execute(play)
        results = runner.auto_resolve()

        snap = runner.snapshot()
        # EX9-012 should remain as-is (no Greymon in hand to digivolve into)
        our_card = [s for s in snap.p1_field if s.card_id == "EX9-012"]
        assert len(our_card) == 1, "EX9-012 should remain when no Greymon target in hand"

    def test_reactive_digivolve_is_free(self, debug_runner):
        """The reactive digivolve should cost 0 memory."""
        runner = debug_runner(initial_memory=5)

        # Place EX9-012 on field
        runner.place_on_field(1, ["EX9-012"])
        # Inject Tai Kamiya (trigger) and Greymon (target)
        runner.inject_card(1, "ST1-12", "hand")  # Tai Kamiya cost 2
        runner.inject_card(1, "BT1-021", "hand")  # MetalGreymon Lv.5

        runner.set_phase("Main")
        snap_before = runner.snapshot()
        mem_before = snap_before.memory

        play = runner.find_action("Play Tai Kamiya")
        assert play is not None
        runner.execute(play)
        results = runner.auto_resolve()

        snap = runner.snapshot()
        # Memory should only have decreased by Tai Kamiya's cost (2), not extra for the digivolve
        # The digivolve from hand draws a card too
        memory_spent = mem_before - snap.memory
        assert memory_spent == 2, (
            f"Only Tai Kamiya's cost (2) should be paid, not the digivolve. "
            f"Memory spent: {memory_spent}"
        )


@pytest.mark.behavioral
class TestEX9012InheritedDP:
    """Inherited: [Your Turn] This Digimon gets +4000 DP."""

    def test_inherited_dp_boost_your_turn(self, debug_runner):
        """EX9-012 as inherited source should give +4000 DP during your turn."""
        runner = debug_runner(initial_memory=5)

        # Place a Digimon with EX9-012 as an inherited card
        perm = runner.place_on_field(1, ["EX9-012", "BT1-025"])  # Stack: EX9-012 bottom, WarGreymon top

        snap = runner.snapshot()
        # WarGreymon base DP is 11000, should be 15000 on our turn
        our_digimon = [s for s in snap.p1_field if s.card_id == "BT1-025"]
        assert len(our_digimon) == 1
        assert our_digimon[0].dp == 15000, (
            f"WarGreymon should have 11000 + 4000 = 15000 DP on your turn, got {our_digimon[0].dp}"
        )


@pytest.mark.behavioral
class TestEX9012AltDigi:
    """Alt-digi requirements."""

    def test_alt_digi_from_metalgreymon_cost_1(self, debug_runner):
        """Should be able to digivolve from MetalGreymon (exact name) for cost 1."""
        runner = debug_runner(initial_memory=5)

        # Place MetalGreymon (exact name, Lv.5) on field
        runner.place_on_field(1, ["ST1-09"])  # MetalGreymon Lv.5
        # Place an opponent Digimon for the WD delete effect
        runner.place_on_field(2, ["AD1-001"])  # low DP target
        # Inject EX9-012 into hand
        runner.inject_card(1, "EX9-012", "hand")

        runner.set_phase("Main")
        snap_before = runner.snapshot()

        digivolve = runner.find_action("Digivolve")
        assert digivolve is not None, "Should be able to digivolve from MetalGreymon"
        runner.execute(digivolve)
        results = runner.auto_resolve()

        snap = runner.snapshot()
        # Check the digivolve happened
        digivolved = [s for s in snap.p1_field if s.card_id == "EX9-012"]
        assert len(digivolved) == 1, "Should have digivolved into EX9-012"
        # Check cost: only 1 memory should have been spent
        memory_spent = snap_before.memory - snap.memory
        # Digivolve costs 1 memory (alt-digi)
        assert memory_spent == 1, f"Alt-digi from MetalGreymon should cost 1, spent {memory_spent}"

    def test_alt_digi_from_greymon_lv4_cost_3(self, debug_runner):
        """Should be able to digivolve from Lv.4 Greymon for cost 3."""
        runner = debug_runner(initial_memory=5)

        # Place a Lv.4 Greymon on field
        runner.place_on_field(1, ["ST1-07"])  # Greymon Lv.4
        # Inject EX9-012 into hand
        runner.inject_card(1, "EX9-012", "hand")

        runner.set_phase("Main")
        snap_before = runner.snapshot()

        digivolve = runner.find_action("Digivolve")
        assert digivolve is not None, "Should be able to digivolve from Lv.4 Greymon"
        runner.execute(digivolve)
        results = runner.auto_resolve()

        snap = runner.snapshot()
        digivolved = [s for s in snap.p1_field if s.card_id == "EX9-012"]
        assert len(digivolved) == 1, "Should have digivolved into EX9-012"
        memory_spent = snap_before.memory - snap.memory
        assert memory_spent == 3, f"Alt-digi from Lv.4 Greymon should cost 3, spent {memory_spent}"

    def test_alt_digi_from_adventure_trait_lv4_cost_3(self, debug_runner):
        """Should be able to digivolve from Lv.4 with ADVENTURE trait for cost 3."""
        runner = debug_runner(initial_memory=5)

        # Place a Lv.4 with ADVENTURE trait (ST20-03 Birdramon: Giant Bird, ADVENTURE)
        runner.place_on_field(1, ["ST20-03"])
        # Inject EX9-012 into hand
        runner.inject_card(1, "EX9-012", "hand")

        runner.set_phase("Main")

        digivolve = runner.find_action("Digivolve")
        assert digivolve is not None, (
            "Should be able to digivolve from Lv.4 ADVENTURE trait Digimon"
        )
