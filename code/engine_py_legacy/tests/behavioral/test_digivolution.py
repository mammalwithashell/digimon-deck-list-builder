"""Behavioral tests for digivolution mechanics.

Tests cover:
- Standard digivolution deducts the digivolve cost (not play cost)
- Digivolving draws 1 card (core digivolution rule)
- Digivolving places the new card on top of the existing stack
- Digivolving onto breeding area works correctly
- Color restrictions prevent cross-color digivolution
"""

import pytest


class TestStandardDigivolve:
    """Tests for standard digivolution from hand onto field Digimon."""

    def test_digivolve_deducts_evo_cost(self, debug_runner):
        """Digivolving ST1-05 Birdramon (evo cost 2 from Red Lv.3) onto a Red Lv.3
        should deduct 2 memory, not the full play cost of 4."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")

        # Place a Red Lv.3 (ST1-02 Biyomon) on the field
        runner.place_on_field(1, ["ST1-02"])  # Biyomon, Red Lv.3

        # Inject a Lv.4 Red Digimon into hand
        runner.inject_card(1, "ST1-05", "hand")  # Birdramon, Red Lv.4, evo cost 2

        before = runner.snapshot()
        action = runner.find_action("Digivolve Birdramon")
        assert action is not None, (
            "Should find digivolve action for Birdramon onto Biyomon"
        )

        result = runner.execute(action)
        runner.auto_resolve()

        after = runner.snapshot()
        memory_spent = before.memory - after.memory
        assert memory_spent == 2, (
            f"Digivolve should cost 2 memory (evo cost), not {memory_spent}. "
            f"Before: {before.memory}, After: {after.memory}"
        )

    def test_digivolve_draws_one_card(self, debug_runner):
        """Digivolution should trigger a draw of 1 card (core game rule).
        The hand loses 1 card (placed on stack) and gains 1 (draw), so library
        should decrease by 1."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")

        # Place base Digimon on field
        runner.place_on_field(1, ["ST1-03"])  # Agumon, Red Lv.3

        # Inject digivolve target into hand
        runner.inject_card(1, "ST1-05", "hand")  # Birdramon, Red Lv.4, evo cost 2

        before = runner.snapshot()
        before_library_size = before.p1_library_size

        action = runner.find_action("Digivolve Birdramon")
        assert action is not None

        result = runner.execute(action)
        runner.auto_resolve()

        after = runner.snapshot()
        library_delta = before_library_size - after.p1_library_size
        assert library_delta >= 1, (
            f"Library should decrease by at least 1 (digivolve draw). "
            f"Before: {before_library_size}, After: {after.p1_library_size}"
        )

    def test_digivolve_places_on_stack(self, debug_runner):
        """Digivolving should place the new card on top of the existing stack."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")

        # Place base Digimon
        runner.place_on_field(1, ["ST1-02"])  # Biyomon, Lv.3

        # Inject digivolve card
        runner.inject_card(1, "ST1-05", "hand")  # Birdramon, Lv.4

        action = runner.find_action("Digivolve Birdramon")
        runner.execute(action)
        runner.auto_resolve()

        after = runner.snapshot()
        evolved = [s for s in after.p1_field if s.card_id == "ST1-05"]
        assert len(evolved) == 1, (
            "Should have one permanent with Birdramon (ST1-05) on top"
        )
        stack = evolved[0].stack_ids
        assert "ST1-02" in stack, "Biyomon (ST1-02) should be in the evo stack"
        assert "ST1-05" in stack, "Birdramon (ST1-05) should be in the evo stack"
        assert stack.index("ST1-05") > stack.index("ST1-02"), (
            "Birdramon should be above Biyomon in the stack (bottom-to-top order)"
        )


class TestDigivolveOntoBreeding:
    """Tests for digivolving onto a Digimon in the breeding area."""

    def test_digivolve_in_breeding_area(self, debug_runner):
        """A Lv.4 can digivolve onto a Lv.3 in the breeding area during Main phase."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")

        # Place a Lv.3 in breeding (egg -> Biyomon)
        runner.place_in_breeding(1, ["ST1-01", "ST1-02"])

        # Inject Lv.4 into hand
        runner.inject_card(1, "ST1-05", "hand")  # Birdramon, Red Lv.4

        action = runner.find_action("Digivolve Birdramon")
        assert action is not None, (
            "Should find digivolve action for Birdramon onto breeding area Biyomon"
        )

        runner.execute(action)
        runner.auto_resolve()

        after = runner.snapshot()
        assert after.p1_breeding == "ST1-05", (
            f"Breeding area should now show Birdramon (ST1-05) on top, "
            f"got {after.p1_breeding}"
        )


class TestDigivolveColorRestriction:
    """Tests that digivolving respects color requirements."""

    def test_cannot_digivolve_wrong_color(self, debug_runner):
        """A Blue Lv.4 should not be able to digivolve onto a Red Lv.3
        if the evo requirement specifies Blue."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")

        # Place a Red Lv.3 on field
        runner.place_on_field(1, ["ST1-02"])  # Biyomon, Red Lv.3

        # Inject Blue Lv.4 that requires Blue Lv.3
        runner.inject_card(1, "ST2-05", "hand")  # Ikkakumon, Blue Lv.4

        # Should NOT find a digivolve action for Ikkakumon onto Red Biyomon
        digivolve_actions = runner.find_actions("Digivolve Ikkakumon")
        # Filter to only actions that target Biyomon
        onto_biyomon = {k: v for k, v in digivolve_actions.items() if "Biyomon" in v}
        assert len(onto_biyomon) == 0, (
            "Should not be able to digivolve Blue Ikkakumon onto Red Biyomon"
        )
