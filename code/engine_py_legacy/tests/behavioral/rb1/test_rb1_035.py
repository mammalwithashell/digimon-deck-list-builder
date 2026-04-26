"""Behavioral tests for RB1-035 Hokuto Amanokawa (White Tamer, cost 2).

Card text:
[Start of Your Turn] If your opponent has 3 or more Tamers in play, gain 1 memory.
[All Turns] When an opponent plays a Digimon, by suspending this Tamer,
    gain 1 memory if that Digimon is level 4 or higher, and <Draw 1> if it is level 3.
[Security] Play this card without paying the cost.

Clause breakdown:
  C1 (start_of_turn): Gain 1 memory if opponent has 3+ Tamers.
  C2 (all_turns_lv4): When opponent plays Lv.4+ Digimon, suspend -> gain 1 memory.
  C3 (all_turns_lv3): When opponent plays Lv.3 Digimon, suspend -> draw 1.
  C4 (suspend_cost): Effect requires suspending this Tamer (must be unsuspended).
  C5 (security): Play this card without paying the cost.
"""

import pytest
from engine_py_legacy.engine.data.enums import GamePhase


FILLER = ["ST1-02"] * 40
TAMER = "ST1-12"       # Tai Kamiya, play cost 2, Tamer
LV3_DIGI = "ST1-02"    # Biyomon, Lv.3, play cost 2
LV4_DIGI = "ST1-07"    # Greymon, Lv.4, play cost 5


@pytest.mark.behavioral
class TestRB1035StartOfTurn:
    """C1: [Start of Your Turn] Gain 1 memory if opponent has 3+ Tamers."""

    def test_gain_memory_with_3_opponent_tamers(self, debug_runner):
        """Hokuto gains 1 memory at start of turn when opponent has 3+ Tamers."""
        runner = debug_runner(
            deck1=["RB1-035"] * 4 + FILLER,
            deck2=FILLER + [LV3_DIGI] * 10,
            initial_memory=3,
        )
        # Place Hokuto on P1's field
        runner.place_on_field(1, ["RB1-035"])
        # Place 3 Tamers on P2's (opponent's) field
        runner.place_on_field(2, [TAMER])
        runner.place_on_field(2, [TAMER])
        runner.place_on_field(2, [TAMER])

        # Pass P1's turn to start P2's turn, then pass P2's turn
        # to get back to P1's start of turn where the effect fires
        runner.set_memory(-1)  # Force turn pass to P2
        runner.game.pass_turn()  # End P1 turn -> P2's turn starts
        runner.auto_resolve()

        # Now pass P2's turn back to P1
        runner.set_memory(1)  # Force turn back to P1
        mem_before = runner.snapshot().memory
        runner.game.pass_turn()  # End P2 turn -> P1's turn starts

        # After P1's start of turn, the OnStartTurn effect should fire
        # Memory should have increased by 1 from Hokuto's effect
        mem_after = runner.snapshot().memory
        assert mem_after > mem_before, (
            f"Hokuto should gain 1 memory at start of turn with 3 opponent Tamers. "
            f"Memory went from {mem_before} to {mem_after}"
        )

    def test_no_gain_with_2_opponent_tamers(self, debug_runner):
        """Hokuto does NOT gain memory when opponent has fewer than 3 Tamers."""
        runner = debug_runner(
            deck1=["RB1-035"] * 4 + FILLER,
            deck2=FILLER + [LV3_DIGI] * 10,
            initial_memory=3,
        )
        runner.place_on_field(1, ["RB1-035"])
        # Only 2 Tamers on opponent's field
        runner.place_on_field(2, [TAMER])
        runner.place_on_field(2, [TAMER])

        # Directly test the condition by checking the effect's condition callback
        hokuto_card = None
        for perm in runner.game.player1.battle_area:
            if perm.top_card and perm.top_card.c_entity_base and \
               perm.top_card.c_entity_base.card_id == "RB1-035":
                hokuto_card = perm.top_card
                break
        assert hokuto_card is not None, "Should find Hokuto on field"

        from engine_py_legacy.engine.data.enums import EffectTiming
        start_turn_effects = [
            eff for eff in hokuto_card.effect_list(EffectTiming.OnStartTurn)
            if eff.can_use_condition
        ]
        assert len(start_turn_effects) >= 1, "Should have a start-of-turn effect"

        # With only 2 tamers, condition should be False
        ctx = {
            'game': runner.game,
            'player': runner.game.player1,
            'permanent': runner.game.player1.battle_area[0],
        }
        # Make sure it's P1's turn
        runner.game.player1.is_my_turn = True
        result = start_turn_effects[0].can_use_condition(ctx)
        assert not result, (
            "Hokuto's start-of-turn condition should be False with only 2 opponent Tamers"
        )


@pytest.mark.behavioral
class TestRB1035OpponentPlaysLv4:
    """C2: [All Turns] When opponent plays Lv.4+ Digimon -> suspend -> gain 1 memory."""

    def test_gain_memory_when_opponent_plays_lv4(self, debug_runner):
        """When opponent plays a Lv.4 Digimon, Hokuto suspends and P1 gains 1 memory."""
        # P2 is turn player so they can play cards. P1 has Hokuto.
        runner = debug_runner(
            deck1=["RB1-035"] * 4 + FILLER,
            deck2=[LV4_DIGI] * 4 + FILLER,
            initial_memory=10,
            first_player=2,
        )
        # Place Hokuto on P1's field (unsuspended)
        hokuto_perm = runner.place_on_field(1, ["RB1-035"])
        assert not hokuto_perm.is_suspended, "Hokuto should start unsuspended"

        # Put Lv.4 Digimon in P2's hand
        runner.inject_card(2, LV4_DIGI, "hand")
        runner.set_phase("Main")

        # Record memory before P2 plays
        mem_before = runner.snapshot().memory

        # P2 plays the Lv.4 Digimon
        play_action = runner.find_action("play")
        assert play_action is not None, "P2 should be able to play Lv.4 Digimon"
        runner.execute(play_action)
        runner.auto_resolve()

        # After P2 plays Lv.4: Hokuto should suspend and P1 should gain 1 memory
        # From P2's perspective (turn player), P1 gaining memory means game.memory decreases
        mem_after = runner.snapshot().memory
        play_cost = 5  # ST1-07 play cost

        # Memory should decrease by play_cost (P2 pays) but also decrease by 1 more (P1 gains)
        # Expected: mem_before - play_cost - 1 (Hokuto gives P1 +1 which is -1 from turn player view)
        assert hokuto_perm.is_suspended, (
            "Hokuto should be suspended after opponent plays a Lv.4 Digimon"
        )
        expected_mem = mem_before - play_cost - 1
        assert mem_after == expected_mem, (
            f"P1 should gain 1 memory when opponent plays Lv.4. "
            f"Expected memory {expected_mem} (before={mem_before}, play_cost={play_cost}, "
            f"hokuto_gain=1), got {mem_after}"
        )


@pytest.mark.behavioral
class TestRB1035OpponentPlaysLv3:
    """C3: [All Turns] When opponent plays Lv.3 Digimon -> suspend -> draw 1."""

    def test_draw_when_opponent_plays_lv3(self, debug_runner):
        """When opponent plays a Lv.3 Digimon, Hokuto suspends and P1 draws 1."""
        runner = debug_runner(
            deck1=["RB1-035"] * 4 + FILLER,
            deck2=[LV3_DIGI] * 4 + FILLER,
            initial_memory=10,
            first_player=2,
        )
        hokuto_perm = runner.place_on_field(1, ["RB1-035"])
        runner.inject_card(2, LV3_DIGI, "hand")
        runner.set_phase("Main")

        hand_before = len(runner.game.player1.hand_cards)

        # P2 plays Lv.3 Digimon
        play_action = runner.find_action("play")
        assert play_action is not None, "P2 should be able to play Lv.3 Digimon"
        runner.execute(play_action)
        runner.auto_resolve()

        hand_after = len(runner.game.player1.hand_cards)
        assert hokuto_perm.is_suspended, (
            "Hokuto should be suspended after opponent plays a Lv.3 Digimon"
        )
        assert hand_after == hand_before + 1, (
            f"P1 should draw 1 card when opponent plays Lv.3. "
            f"Hand went from {hand_before} to {hand_after}"
        )

    def test_no_memory_gain_when_opponent_plays_lv3(self, debug_runner):
        """Playing Lv.3 should draw, NOT gain memory."""
        runner = debug_runner(
            deck1=["RB1-035"] * 4 + FILLER,
            deck2=[LV3_DIGI] * 4 + FILLER,
            initial_memory=10,
            first_player=2,
        )
        runner.place_on_field(1, ["RB1-035"])
        runner.inject_card(2, LV3_DIGI, "hand")
        runner.set_phase("Main")

        mem_before = runner.snapshot().memory
        play_cost = 2  # ST1-02 play cost

        play_action = runner.find_action("play")
        assert play_action is not None
        runner.execute(play_action)
        runner.auto_resolve()

        mem_after = runner.snapshot().memory
        # Lv.3 should NOT gain memory -- only draw 1
        # Memory should only decrease by play cost, no extra memory change
        expected_mem = mem_before - play_cost
        assert mem_after == expected_mem, (
            f"Lv.3 should NOT trigger memory gain, only draw. "
            f"Expected memory {expected_mem} (before={mem_before}, play_cost={play_cost}), "
            f"got {mem_after}"
        )


@pytest.mark.behavioral
class TestRB1035SuspendCost:
    """C4: Effect requires suspending Hokuto (must be unsuspended)."""

    def test_does_not_trigger_when_already_suspended(self, debug_runner):
        """If Hokuto is already suspended, the effect should not trigger."""
        runner = debug_runner(
            deck1=["RB1-035"] * 4 + FILLER,
            deck2=[LV4_DIGI] * 4 + FILLER,
            initial_memory=10,
            first_player=2,
        )
        # Place Hokuto suspended
        hokuto_perm = runner.place_on_field(1, ["RB1-035"], is_suspended=True)
        assert hokuto_perm.is_suspended, "Hokuto should be suspended"

        runner.inject_card(2, LV4_DIGI, "hand")
        runner.set_phase("Main")

        mem_before = runner.snapshot().memory
        play_cost = 5

        play_action = runner.find_action("play")
        assert play_action is not None
        runner.execute(play_action)
        runner.auto_resolve()

        mem_after = runner.snapshot().memory
        # Since Hokuto is already suspended, no extra memory gain
        expected_mem = mem_before - play_cost
        assert mem_after == expected_mem, (
            f"Suspended Hokuto should NOT trigger. "
            f"Expected memory {expected_mem}, got {mem_after}"
        )

    def test_does_not_trigger_twice_without_unsuspend(self, debug_runner):
        """After triggering once (suspending), a second opponent play should not trigger."""
        runner = debug_runner(
            deck1=["RB1-035"] * 4 + FILLER,
            deck2=[LV4_DIGI] * 8 + FILLER[:42],
            initial_memory=20,
            first_player=2,
        )
        hokuto_perm = runner.place_on_field(1, ["RB1-035"])
        runner.inject_card(2, LV4_DIGI, "hand")
        runner.inject_card(2, LV4_DIGI, "hand")
        runner.set_phase("Main")

        # First play should trigger
        play_action = runner.find_action("play")
        assert play_action is not None
        runner.execute(play_action)
        runner.auto_resolve()
        assert hokuto_perm.is_suspended, "Hokuto should be suspended after first trigger"

        mem_before_second = runner.snapshot().memory
        play_cost = 5

        # Second play should NOT trigger (already suspended)
        play_action2 = runner.find_action("play")
        if play_action2 is not None:
            runner.execute(play_action2)
            runner.auto_resolve()

            mem_after_second = runner.snapshot().memory
            expected = mem_before_second - play_cost
            assert mem_after_second == expected, (
                f"Second play should not trigger suspended Hokuto. "
                f"Expected {expected}, got {mem_after_second}"
            )

    def test_does_not_trigger_for_own_play(self, debug_runner):
        """Hokuto should NOT trigger when its own player plays a Digimon."""
        runner = debug_runner(
            deck1=["RB1-035"] * 4 + [LV4_DIGI] * 4 + FILLER[:42],
            deck2=FILLER + [LV3_DIGI] * 10,
            initial_memory=10,
        )
        hokuto_perm = runner.place_on_field(1, ["RB1-035"])
        runner.inject_card(1, LV4_DIGI, "hand")
        runner.set_phase("Main")

        mem_before = runner.snapshot().memory
        hand_before = len(runner.game.player1.hand_cards)

        # P1 plays a Lv.4 Digimon (their own)
        play_action = runner.find_action("play")
        assert play_action is not None
        runner.execute(play_action)
        runner.auto_resolve()

        # Hokuto should NOT suspend (it's own play, not opponent's)
        assert not hokuto_perm.is_suspended, (
            "Hokuto should NOT suspend when its own player plays a Digimon"
        )


@pytest.mark.behavioral
class TestRB1035OpponentPlaysNonDigimon:
    """Verify Hokuto does NOT trigger when opponent plays a non-Digimon."""

    def test_does_not_trigger_for_tamer_play(self, debug_runner):
        """Hokuto should not trigger when opponent plays a Tamer."""
        runner = debug_runner(
            deck1=["RB1-035"] * 4 + FILLER,
            deck2=[TAMER] * 4 + FILLER,
            initial_memory=10,
            first_player=2,
        )
        hokuto_perm = runner.place_on_field(1, ["RB1-035"])
        runner.inject_card(2, TAMER, "hand")
        runner.set_phase("Main")

        play_action = runner.find_action("play")
        assert play_action is not None
        runner.execute(play_action)
        runner.auto_resolve()

        assert not hokuto_perm.is_suspended, (
            "Hokuto should NOT suspend when opponent plays a Tamer (not a Digimon)"
        )
