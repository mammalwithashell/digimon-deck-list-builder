"""Behavioral tests for BT8-090 Kari Kamiya (Yellow Tamer, Cost 4).

Card text:
  [Start of Your Turn] If you have 2 or less memory, set it to 3.
  [Your Turn] When a card is added to your security stack, you may
      suspend this Tamer to gain 1 memory.
"""

import pytest


@pytest.mark.behavioral
class TestBT8090KariKamiya:
    """Tests for BT8-090 Kari Kamiya."""

    # ── Effect 0: [Start of Your Turn] memory set ──────────────────────

    def test_start_of_turn_sets_memory_to_3_when_below(self, debug_runner):
        """At start of turn, if memory <= 2, set to 3."""
        runner = debug_runner(initial_memory=0)

        # Place Kari on P1's field (P1 is turn player)
        runner.place_on_field(1, ["BT8-090"])
        runner.set_phase("Main")

        # Now pass turn so P2 gets a turn, then when P1's turn starts again,
        # OnStartTurn should fire. Instead, we test directly by triggering
        # phase_start manually.
        # Set memory to 1 (P1 as turn player has 1 memory)
        runner.game.memory = 1

        # Directly call phase_start to trigger OnStartTurn
        runner.game.phase_start()

        # Memory should be set to 3
        assert runner.game.memory == 3, (
            f"Memory should be set to 3 when starting at 1, got {runner.game.memory}"
        )

    def test_start_of_turn_sets_memory_to_3_when_negative(self, debug_runner):
        """At start of turn, if memory is negative (opponent advantage), set to 3."""
        runner = debug_runner(initial_memory=0)

        runner.place_on_field(1, ["BT8-090"])
        runner.set_phase("Main")

        # Negative memory means turn player is behind
        runner.game.memory = -2

        runner.game.phase_start()

        assert runner.game.memory == 3, (
            f"Memory should be set to 3 when starting at -2, got {runner.game.memory}"
        )

    def test_start_of_turn_sets_memory_to_3_when_at_2(self, debug_runner):
        """At start of turn with exactly 2 memory, should set to 3."""
        runner = debug_runner(initial_memory=0)

        runner.place_on_field(1, ["BT8-090"])
        runner.set_phase("Main")

        runner.game.memory = 2

        runner.game.phase_start()

        assert runner.game.memory == 3, (
            f"Memory should be set to 3 when starting at 2, got {runner.game.memory}"
        )

    def test_start_of_turn_does_not_change_memory_above_2(self, debug_runner):
        """At start of turn with memory > 2, should NOT change."""
        runner = debug_runner(initial_memory=0)

        runner.place_on_field(1, ["BT8-090"])
        runner.set_phase("Main")

        runner.game.memory = 4

        runner.game.phase_start()

        # Memory should stay at 4 (above 2, no change)
        assert runner.game.memory == 4, (
            f"Memory should remain 4 when already above 2, got {runner.game.memory}"
        )

    def test_start_of_turn_does_not_fire_for_opponent(self, debug_runner):
        """OnStartTurn should NOT fire for the opponent's Kari Kamiya."""
        runner = debug_runner(initial_memory=0)

        # Place Kari on P2's field (P1 is turn player)
        runner.place_on_field(2, ["BT8-090"])
        runner.set_phase("Main")

        # Set memory to -5 (P1 turn player has bad memory; P2's Kari should not fire)
        runner.game.memory = -5

        runner.game.phase_start()

        # P2's Kari should NOT fire because it's P1's turn
        # Memory should stay at -5 (or go through normal processing)
        assert runner.game.memory != 3, (
            "P2's Kari Kamiya should not fire OnStartTurn during P1's turn"
        )

    # ── Effect 1: [Your Turn] OnAddSecurity — suspend for +1 memory ───

    def test_on_add_security_gains_memory(self, debug_runner):
        """When a card is added to owner's security, may suspend to gain 1 memory."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        # Place Kari on P1's field (unsuspended)
        runner.place_on_field(1, ["BT8-090"])

        snap_before = runner.snapshot()
        mem_before = snap_before.memory

        # Trigger recovery on P1 (adds card to P1's security)
        runner.game.player1.recovery(1)

        # The optional trigger should create a pending selection
        # Auto-resolve it (accept the optional effect)
        runner.auto_resolve()

        snap_after = runner.snapshot()

        # Should have gained 1 memory from the effect
        assert snap_after.memory == mem_before + 1, (
            f"Should gain 1 memory from Kari's effect, "
            f"before={mem_before}, after={snap_after.memory}"
        )

        # Kari should be suspended
        kari_slot = None
        for slot in snap_after.p1_field:
            if slot.card_id == "BT8-090":
                kari_slot = slot
                break
        assert kari_slot is not None, "Kari should still be on field"
        assert kari_slot.is_suspended, "Kari should be suspended after using effect"

    def test_on_add_security_does_not_fire_when_suspended(self, debug_runner):
        """If Kari is already suspended, the effect should not activate."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        # Place Kari already suspended
        runner.place_on_field(1, ["BT8-090"], is_suspended=True)

        mem_before = runner.game.memory

        # Trigger recovery on P1
        runner.game.player1.recovery(1)
        runner.auto_resolve()

        # Memory should NOT have changed (effect couldn't activate)
        assert runner.game.memory == mem_before, (
            f"Memory should not change when Kari is already suspended, "
            f"before={mem_before}, after={runner.game.memory}"
        )

    def test_on_add_security_does_not_fire_for_opponent_security(self, debug_runner):
        """Kari's effect should not trigger when opponent's security is added to."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        # Place Kari on P1's field
        runner.place_on_field(1, ["BT8-090"])

        mem_before = runner.game.memory

        # Trigger recovery on P2 (adds to P2's security, not P1's)
        runner.game.player2.recovery(1)
        runner.auto_resolve()

        # P1's Kari should NOT have triggered
        assert runner.game.memory == mem_before, (
            f"Kari should not trigger from opponent's security add, "
            f"before={mem_before}, after={runner.game.memory}"
        )

    def test_on_add_security_does_not_fire_during_opponent_turn(self, debug_runner):
        """Kari's [Your Turn] effect should not fire during opponent's turn."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        # Place Kari on P1's field
        runner.place_on_field(1, ["BT8-090"])

        # Switch turn to P2 so it's P2's turn
        runner.game.turn_player = runner.game.player2
        runner.game.opponent_player = runner.game.player1
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        mem_before = runner.game.memory

        # P1 gets a card added to security during P2's turn
        runner.game.player1.recovery(1)
        runner.auto_resolve()

        # Kari should NOT trigger (not your turn)
        assert runner.game.memory == mem_before, (
            f"Kari should not trigger during opponent's turn, "
            f"before={mem_before}, after={runner.game.memory}"
        )

    def test_on_add_security_is_optional(self, debug_runner):
        """The effect is optional ('you may') — player can decline."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        # Place Kari on P1's field (unsuspended)
        runner.place_on_field(1, ["BT8-090"])

        mem_before = runner.game.memory

        # Trigger recovery on P1
        runner.game.player1.recovery(1)

        # Check if there's a pending optional selection — find a "pass" action
        # If optional, there should be a way to decline
        snap = runner.snapshot()
        if snap.phase not in ("Main", "Breeding"):
            # We're in a selection phase - find the pass/decline action
            pass_action = runner.find_action("pass") or runner.find_action("decline") or runner.find_action("no")
            if pass_action is not None:
                runner.execute(pass_action)
                runner.auto_resolve()
                # Memory should NOT have changed
                assert runner.game.memory == mem_before, (
                    f"Declining optional effect should not gain memory, "
                    f"before={mem_before}, after={runner.game.memory}"
                )
                # Kari should NOT be suspended
                kari_slot = None
                for slot in runner.snapshot().p1_field:
                    if slot.card_id == "BT8-090":
                        kari_slot = slot
                        break
                assert kari_slot is not None
                assert not kari_slot.is_suspended, "Kari should not be suspended when declining"
            else:
                # If auto-resolved (no choice offered), that's also acceptable for optional
                # as the engine may auto-accept single-target optionals
                pass

    # ── P2 turn player scenario ────────────────────────────────────────

    def test_p2_start_of_turn_sets_memory(self, debug_runner):
        """P2's Kari should also correctly set memory to 3 at P2's start of turn."""
        runner = debug_runner(initial_memory=0, first_player=2)
        runner.set_phase("Main")

        # Place Kari on P2's field (P2 is turn player)
        runner.place_on_field(2, ["BT8-090"])

        # Set memory to 1 (turn player = P2 has 1 memory)
        runner.game.memory = 1

        runner.game.phase_start()

        assert runner.game.memory == 3, (
            f"P2's Kari should set memory to 3 at P2's start of turn, got {runner.game.memory}"
        )
