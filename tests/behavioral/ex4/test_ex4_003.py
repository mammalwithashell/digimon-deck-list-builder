"""Behavioral tests for EX4-003 Tsunomon (Digi-Egg, Lv.2, Purple).

Card text:
Inherited Effect [Your Turn] [Once Per Turn] When one of your other Digimon
digivolves, <Draw 1> (Draw 1 card from your deck.)

Key behaviors:
- Fires as an inherited effect (only when in a Digimon's digivolution stack)
- Triggers when a DIFFERENT friendly Digimon digivolves (not the host)
- Once Per Turn limit
- Only on your turn
- Draws 1 card
"""

import pytest


# ST1-03 Agumon = Red Lv.3 (used as host for Tsunomon — cannot be target of Purple digivolve)
# ST6-02 DemiDevimon = Purple Lv.3 (digivolve target)
# ST6-06 Garurumon = Purple Lv.4 (evo cost 2 from Purple Lv.3)
FILLER_DECK = ["ST1-03"] * 50


@pytest.mark.behavioral
class TestEX4003Tsunomon:
    """Tests for EX4-003 Tsunomon inherited effect."""

    def test_draw_when_other_digimon_digivolves(self, debug_runner):
        """When another of your Digimon digivolves, Tsunomon's inherited effect
        should trigger and draw 1 card."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")

        # Place Digimon A: Red Lv.3 with EX4-003 Tsunomon in its stack
        # (Red host ensures Garurumon can't digivolve onto it — Purple only)
        runner.place_on_field(1, ["EX4-003", "ST1-03"], turn_played=-1)

        # Place Digimon B: Purple Lv.3 that will digivolve
        runner.place_on_field(1, ["ST6-02"], turn_played=-1)

        # Inject Lv.4 into hand for digivolving onto Digimon B
        runner.inject_card(1, "ST6-06", "hand")

        before = runner.snapshot()
        before_lib = before.p1_library_size

        # Digivolve DemiDevimon into Garurumon (only valid target is Digimon B)
        action = runner.find_action("Digivolve Garurumon")
        assert action is not None, "Should find digivolve action for Garurumon"
        runner.execute(action)
        runner.auto_resolve()

        after = runner.snapshot()

        # Digivolve itself draws 1 card (core rule).
        # Tsunomon inherited should draw 1 additional card.
        # Library: lost 2 (core draw + Tsunomon draw)
        lib_delta = before_lib - after.p1_library_size
        assert lib_delta == 2, (
            f"Library should decrease by 2 (1 core digivolve draw + 1 Tsunomon draw), "
            f"got delta={lib_delta} (before={before_lib}, after={after.p1_library_size})"
        )

    def test_does_not_trigger_when_host_digivolves(self, debug_runner):
        """Tsunomon should NOT trigger when its own host Digimon digivolves,
        since the text says 'one of your OTHER Digimon'."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")

        # Only one Digimon: Purple Lv.3 with Tsunomon in its stack
        runner.place_on_field(1, ["EX4-003", "ST6-02"], turn_played=-1)

        # Inject Lv.4 to digivolve the HOST (the permanent with Tsunomon)
        runner.inject_card(1, "ST6-06", "hand")

        before = runner.snapshot()
        before_lib = before.p1_library_size

        # Digivolve Tsunomon's host into Garurumon
        action = runner.find_action("Digivolve Garurumon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        after = runner.snapshot()

        # Only the core digivolve draw should happen (1 card), NOT Tsunomon's draw.
        # _fire_digivolve_observers skips the digivolving permanent,
        # so Tsunomon's effect won't be checked.
        lib_delta = before_lib - after.p1_library_size
        assert lib_delta == 1, (
            f"Library should decrease by 1 (core digivolve draw only, Tsunomon should NOT trigger "
            f"on self-digivolve). Got delta={lib_delta}"
        )

    def test_once_per_turn_limit(self, debug_runner):
        """Tsunomon's draw should only happen once per turn even if multiple
        other Digimon digivolve."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")

        # Digimon A: Red Lv.3 with Tsunomon in stack (host, won't be digivolved)
        runner.place_on_field(1, ["EX4-003", "ST1-03"], turn_played=-1)

        # Digimon B and C: both Purple Lv.3, will digivolve
        runner.place_on_field(1, ["ST6-02"], turn_played=-1)
        runner.place_on_field(1, ["ST6-02"], turn_played=-1)

        # Inject two Lv.4 cards
        runner.inject_card(1, "ST6-06", "hand")
        runner.inject_card(1, "ST6-06", "hand")

        before = runner.snapshot()
        before_lib = before.p1_library_size

        # First digivolve: should trigger Tsunomon
        action = runner.find_action("Digivolve Garurumon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        mid = runner.snapshot()
        mid_lib = mid.p1_library_size
        first_delta = before_lib - mid_lib
        # First digivolve: 1 core draw + 1 Tsunomon draw = 2 total from library
        assert first_delta == 2, (
            f"First digivolve should draw 2 (core + Tsunomon), got delta={first_delta}"
        )

        # Second digivolve: Tsunomon should NOT trigger again (OPT)
        action2 = runner.find_action("Digivolve Garurumon")
        assert action2 is not None
        runner.execute(action2)
        runner.auto_resolve()

        final = runner.snapshot()
        second_delta = mid_lib - final.p1_library_size
        # Second digivolve: only 1 core draw (Tsunomon OPT already used)
        assert second_delta == 1, (
            f"Second digivolve should draw 1 (core only, OPT used), got delta={second_delta}"
        )

    def test_does_not_trigger_on_opponent_turn(self, debug_runner):
        """Tsunomon should not trigger on the opponent's turn."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")
        runner.clear_zone(2, "hand")

        # P1 has Digimon with Tsunomon in stack
        runner.place_on_field(1, ["EX4-003", "ST1-03"], turn_played=-1)

        # P1 also has another Digimon (Purple Lv.3) on field
        runner.place_on_field(1, ["ST6-02"], turn_played=-1)

        # Pass turn to P2 by setting memory to give P2 the turn
        game = runner.game
        game.memory = -10  # Give P2 a lot of memory

        # Now set up P2 with a digivolve target
        runner.place_on_field(2, ["ST6-02"], turn_played=-1)
        runner.inject_card(2, "ST6-06", "hand")

        before = runner.snapshot()
        p1_lib_before = before.p1_library_size

        # P2 digivolves their Digimon - Tsunomon (on P1 field) should NOT trigger
        # because it's not P1's turn
        action = runner.find_action("Digivolve Garurumon")
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

        after = runner.snapshot()
        p1_lib_delta = p1_lib_before - after.p1_library_size
        assert p1_lib_delta == 0, (
            f"P1's library should not change when P2 digivolves (not P1's turn). "
            f"Got delta={p1_lib_delta}"
        )
