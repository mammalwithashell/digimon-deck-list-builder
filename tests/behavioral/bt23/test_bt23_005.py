"""Behavioral tests for BT23-005 Elizamon (Lv.3 Red Digimon).

Card text:
  [Your Turn] When this Digimon would digivolve into a Digimon card with
  the [Reptile] or [Dragonkin] trait, reduce the digivolution cost by 1.

  Inherited: [Your Turn] This Digimon gets +2000 DP.
"""

import pytest


@pytest.mark.behavioral
class TestBT23005Elizamon:
    """Tests for BT23-005 Elizamon."""

    # ── Clause 1: Digivolution cost reduction ────────────────────────

    def test_digivolve_into_reptile_reduces_cost_by_1(self, debug_runner):
        """Digivolving Elizamon into a Reptile-trait Digimon costs 1 less.

        BT21-017 Dimetromon is Lv.4 Red with [Reptile, LIBERATOR], evo cost 2.
        With Elizamon's effect, it should cost 1.
        """
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")

        # Place Elizamon on field as a Lv.3 Digimon
        runner.place_on_field(1, ["BT23-005"])  # Elizamon, Lv.3 Red Reptile

        # Inject Dimetromon (Lv.4, Red, Reptile, evo cost 2 from Red Lv.3) into hand
        runner.inject_card(1, "BT21-017", "hand")

        before = runner.snapshot()
        action = runner.find_action("Digivolve Dimetromon")
        assert action is not None, (
            f"Should find digivolve action for Dimetromon onto Elizamon. "
            f"Actions: {runner.actions()}"
        )

        runner.execute(action)
        runner.auto_resolve()

        after = runner.snapshot()
        # Base evo cost is 2, reduced by 1 = 1 memory spent
        memory_spent = before.memory - after.memory
        assert memory_spent == 1, (
            f"Digivolve into Reptile should cost 1 (2 base - 1 reduction). "
            f"Spent {memory_spent}. Before={before.memory}, After={after.memory}"
        )

    def test_digivolve_into_dragonkin_reduces_cost_by_1(self, debug_runner):
        """Digivolving Elizamon into a Dragonkin-trait Digimon costs 1 less.

        BT8-011 Cyclonemon is Lv.4 Red with [Dragonkin], evo cost 2.
        With Elizamon's effect, it should cost 1.
        """
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")

        runner.place_on_field(1, ["BT23-005"])  # Elizamon

        runner.inject_card(1, "BT8-011", "hand")  # Cyclonemon, Dragonkin, evo cost 2

        before = runner.snapshot()
        action = runner.find_action("Digivolve Cyclonemon")
        assert action is not None, (
            f"Should find digivolve action for Cyclonemon. Actions: {runner.actions()}"
        )

        runner.execute(action)
        runner.auto_resolve()

        after = runner.snapshot()
        memory_spent = before.memory - after.memory
        assert memory_spent == 1, (
            f"Digivolve into Dragonkin should cost 1 (2 base - 1 reduction). "
            f"Spent {memory_spent}. Before={before.memory}, After={after.memory}"
        )

    def test_no_reduction_without_reptile_or_dragonkin(self, debug_runner):
        """Digivolving Elizamon into a Digimon WITHOUT Reptile/Dragonkin costs full.

        BT1-015 Greymon is Lv.4 Red with [Dinosaur], evo cost 2.
        No reduction should apply.
        """
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")

        runner.place_on_field(1, ["BT23-005"])  # Elizamon

        runner.inject_card(1, "BT1-015", "hand")  # Greymon, Dinosaur, evo cost 2

        before = runner.snapshot()
        action = runner.find_action("Digivolve Greymon")
        assert action is not None, (
            f"Should find digivolve action for Greymon. Actions: {runner.actions()}"
        )

        runner.execute(action)
        runner.auto_resolve()

        after = runner.snapshot()
        memory_spent = before.memory - after.memory
        assert memory_spent == 2, (
            f"Digivolve into Dinosaur (no Reptile/Dragonkin) should cost full 2. "
            f"Spent {memory_spent}. Before={before.memory}, After={after.memory}"
        )

    def test_no_reduction_on_opponent_turn(self, debug_runner):
        """Effect should not reduce cost during opponent's turn.

        This tests the [Your Turn] restriction. We set up P2's Elizamon,
        make it P1's turn, and verify P2's Elizamon doesn't reduce for P2.
        """
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")
        runner.clear_zone(2, "hand")

        # Place Elizamon on P2's field
        runner.place_on_field(2, ["BT23-005"])

        # Inject a Reptile Lv.4 into P2's hand
        runner.inject_card(2, "BT21-017", "hand")

        # It's P1's turn, so P2 cannot act. Verify via snapshot.
        snap = runner.snapshot()
        assert snap.active_player == 1, "Should be P1's turn"
        # P2's Elizamon effect shouldn't fire because [Your Turn] means P2's turn only

    # ── Clause 2: Inherited +2000 DP ���───────────────────────────────

    def test_inherited_dp_boost_on_your_turn(self, debug_runner):
        """When Elizamon is inherited (below top card), Digimon gets +2000 DP.

        Place Elizamon as base, digivolve into Greymon (4000 DP).
        On your turn, DP should be 4000 + 2000 = 6000.
        """
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")

        # Place a stack: Elizamon (bottom) + Greymon (top)
        runner.place_on_field(1, ["BT23-005", "BT1-015"])  # Elizamon -> Greymon

        snap = runner.snapshot()
        greymon_slot = snap.p1_field[0]
        assert greymon_slot.card_id == "BT1-015", "Top card should be Greymon"
        assert greymon_slot.dp == 6000, (
            f"Greymon (4000 base) + Elizamon inherited +2000 = 6000. Got {greymon_slot.dp}"
        )

    def test_inherited_dp_not_on_opponent_turn(self, debug_runner):
        """Inherited +2000 DP should NOT apply during opponent's turn.

        Place Elizamon as inherited under a Digimon on P2's field.
        It's P1's turn, so P2's [Your Turn] inherited effect is inactive.
        """
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place stack on P2's field: Elizamon (bottom) + Greymon (top)
        runner.place_on_field(2, ["BT23-005", "BT1-015"])

        snap = runner.snapshot()
        # During P1's turn, P2's inherited [Your Turn] +2000 DP should be inactive
        greymon_slot = snap.p2_field[0]
        assert greymon_slot.card_id == "BT1-015", "Top card should be Greymon"
        assert greymon_slot.dp == 4000, (
            f"Greymon should have base 4000 DP on opponent's turn (no inherited boost). "
            f"Got {greymon_slot.dp}"
        )

    # ── Edge case: no cost reduction leak to play cost ──────────────

    def test_no_leak_to_play_cost(self, debug_runner):
        """Elizamon's digivolution cost reduction must NOT reduce play costs.

        Place Elizamon on field, then play a Reptile Digimon from hand.
        The play cost should be the full amount, not reduced.
        """
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")

        # Place Elizamon on field
        runner.place_on_field(1, ["BT23-005"])

        # Inject a Reptile Digimon to play (not digivolve)
        # BT21-017 Dimetromon has play cost 4
        runner.inject_card(1, "BT21-017", "hand")

        before = runner.snapshot()
        play_action = runner.find_action("Play Dimetromon")
        assert play_action is not None, (
            f"Should be able to play Dimetromon. Actions: {runner.actions()}"
        )

        runner.execute(play_action)
        runner.auto_resolve()

        after = runner.snapshot()
        memory_spent = before.memory - after.memory
        assert memory_spent == 4, (
            f"Playing Dimetromon should cost full 4 (no leak from digi-cost reduction). "
            f"Spent {memory_spent}. Before={before.memory}, After={after.memory}"
        )
