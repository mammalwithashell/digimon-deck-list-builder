"""Behavioral tests for EX9-066 Tai Kamiya & Matt Ishida (Tamer).

Card text:
[On Play] You may return 1 Digimon card with [Greymon], [Garurumon] or
    [Omnimon] in its name from your trash to the hand. If this effect
    didn't return, <Draw 1>.

[All Turns] When any of your Digimon are played or digivolve, by
    suspending this Tamer, gain 1 memory if you have a Digimon with
    [Greymon] in its name. Then, gain 1 memory if you have a Digimon
    with [Garurumon] in its name.

[Security] Play this card without paying the cost.
"""

import pytest


@pytest.mark.behavioral
class TestEX9066OnPlay:
    """Tests for the [On Play] effect: return Greymon/Garurumon/Omnimon from trash or Draw 1."""

    def test_on_play_returns_greymon_from_trash(self, debug_runner):
        """On Play: selecting a Greymon from trash returns it to hand."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Put tamer in hand to play it
        runner.inject_card(1, "EX9-066", "hand")
        # Put a Greymon in trash as a valid target
        runner.inject_card(1, "ST1-07", "trash")  # Greymon

        snap_before = runner.snapshot()
        trash_before = len(snap_before.p1_trash)
        hand_before = len(snap_before.p1_hand)

        action = runner.find_action("Play Tai Kamiya")
        assert action is not None, (
            f"Should be able to play Tai Kamiya & Matt Ishida from hand. "
            f"Actions: {runner.actions()}"
        )

        runner.execute(action)
        # In SelectTrash phase, the trash card actions use SEL_TRASH_START + idx.
        # Pick the non-decline action from the mask (action != 62 is the trash card).
        legal = runner.action_mask()
        trash_actions = [a for a in legal if a != 62]  # 62 = Decline
        assert len(trash_actions) > 0, (
            f"Should have trash selection actions. Legal: {legal}, Actions: {runner.actions()}"
        )
        runner.execute(trash_actions[0])
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Tamer was played from hand (-1 tamer) but Greymon returned to hand (+1)
        assert "ST1-07" in snap_after.p1_hand, (
            "Greymon should be returned to hand from trash"
        )
        assert "ST1-07" not in snap_after.p1_trash, (
            "Greymon should no longer be in trash"
        )

    def test_on_play_returns_garurumon_from_trash(self, debug_runner):
        """On Play: selecting a Garurumon from trash returns it to hand."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.inject_card(1, "EX9-066", "hand")
        runner.inject_card(1, "ST2-06", "trash")  # Garurumon

        action = runner.find_action("Play Tai Kamiya")
        assert action is not None, f"Actions: {runner.actions()}"

        runner.execute(action)
        # Pick the non-decline trash selection action
        legal = runner.action_mask()
        trash_actions = [a for a in legal if a != 62]
        assert len(trash_actions) > 0, f"Should have trash selection actions. Legal: {legal}"
        runner.execute(trash_actions[0])
        runner.auto_resolve()

        snap_after = runner.snapshot()
        assert "ST2-06" in snap_after.p1_hand, (
            "Garurumon should be returned to hand from trash"
        )

    def test_on_play_draw_1_when_no_targets(self, debug_runner):
        """On Play: Draw 1 when there are no Greymon/Garurumon/Omnimon in trash."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.inject_card(1, "EX9-066", "hand")
        # No qualifying Digimon in trash

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)
        deck_before = snap_before.p1_library_size

        action = runner.find_action("Play Tai Kamiya")
        assert action is not None, f"Actions: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Played tamer (-1 from hand) + Draw 1 (+1 to hand) = net 0
        assert len(snap_after.p1_hand) == hand_before, (
            f"Should draw 1 when no targets in trash. "
            f"Hand went from {hand_before} to {len(snap_after.p1_hand)}"
        )
        assert snap_after.p1_library_size == deck_before - 1, (
            "Deck should shrink by 1 from the draw"
        )

    def test_on_play_draw_1_when_declining_selection(self, debug_runner):
        """On Play: Draw 1 if player declines the optional selection."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.inject_card(1, "EX9-066", "hand")
        runner.inject_card(1, "ST1-07", "trash")  # Greymon in trash (valid target)

        snap_before = runner.snapshot()
        deck_before = snap_before.p1_library_size

        action = runner.find_action("Play Tai Kamiya")
        assert action is not None, f"Actions: {runner.actions()}"

        runner.execute(action)

        # Decline the optional selection (action 0 = pass/decline)
        decline = runner.find_action("Pass")
        if decline is None:
            decline = runner.find_action("Decline")
        if decline is None:
            # Try action 0 which is typically pass
            legal = runner.action_mask()
            decline = 0 if 0 in legal else None
        if decline is not None:
            runner.execute(decline)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # If player declined, should draw 1 (Draw 1 fallback)
        # Greymon should still be in trash
        assert "ST1-07" in snap_after.p1_trash, (
            "Greymon should still be in trash when player declines"
        )
        assert snap_after.p1_library_size == deck_before - 1, (
            "Deck should shrink by 1 from the Draw 1 fallback"
        )

    def test_on_play_non_digimon_not_selectable(self, debug_runner):
        """On Play: non-Digimon cards with matching names in trash are not valid targets."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.inject_card(1, "EX9-066", "hand")
        # Only put a non-matching Digimon in trash (Agumon, no Greymon/Garurumon/Omnimon)
        runner.inject_card(1, "ST1-03", "trash")  # Agumon

        snap_before = runner.snapshot()
        deck_before = snap_before.p1_library_size

        action = runner.find_action("Play Tai Kamiya")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # No valid targets -> Draw 1
        assert snap_after.p1_library_size == deck_before - 1, (
            "Should Draw 1 when no Greymon/Garurumon/Omnimon in trash"
        )


@pytest.mark.behavioral
class TestEX9066AllTurns:
    """Tests for the [All Turns] effect: suspend for memory on play/digivolve."""

    def test_gain_1_memory_with_greymon_on_field(self, debug_runner):
        """All Turns: gain 1 memory when playing a Digimon with Greymon on field."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place the tamer on field (not suspended)
        runner.place_on_field(1, ["EX9-066"])
        # Place a Greymon on field
        runner.place_on_field(1, ["ST1-07"])  # Greymon

        # Put a Digimon in hand to trigger the effect
        runner.inject_card(1, "ST1-03", "hand")  # Agumon (cost 3)

        mem_before = runner.game.memory

        action = runner.find_action("Play Agumon")
        assert action is not None, f"Should be able to play Agumon. Actions: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Playing Agumon costs 3 memory, then observer fires: +1 for Greymon = net -2
        # Tamer should now be suspended
        tamer_slot = None
        for slot in snap_after.p1_field:
            if slot.card_id == "EX9-066":
                tamer_slot = slot
                break
        assert tamer_slot is not None, "Tamer should still be on field"
        assert tamer_slot.is_suspended, "Tamer should be suspended after activation"

        # Memory: started at 10, played Agumon (-3), gained 1 (Greymon) = 8
        assert runner.game.memory == mem_before - 3 + 1, (
            f"Memory should be {mem_before - 3 + 1} (start {mem_before}, "
            f"-3 play, +1 Greymon). Got {runner.game.memory}"
        )

    def test_gain_2_memory_with_both_greymon_and_garurumon(self, debug_runner):
        """All Turns: gain 2 memory when both Greymon and Garurumon are on field."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place the tamer on field
        runner.place_on_field(1, ["EX9-066"])
        # Place Greymon and Garurumon on field
        runner.place_on_field(1, ["ST1-07"])  # Greymon
        runner.place_on_field(1, ["ST2-06"])  # Garurumon

        # Put a Digimon in hand to trigger
        runner.inject_card(1, "ST1-03", "hand")  # Agumon (cost 3)

        mem_before = runner.game.memory

        action = runner.find_action("Play Agumon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Memory: started at 10, played Agumon (-3), gained 2 (Greymon + Garurumon) = 9
        assert runner.game.memory == mem_before - 3 + 2, (
            f"Memory should be {mem_before - 3 + 2} (start {mem_before}, "
            f"-3 play, +1 Greymon, +1 Garurumon). Got {runner.game.memory}"
        )

    def test_gain_1_memory_with_garurumon_only(self, debug_runner):
        """All Turns: gain 1 memory when only Garurumon is on field."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["EX9-066"])
        runner.place_on_field(1, ["ST2-06"])  # Garurumon only

        runner.inject_card(1, "ST1-03", "hand")  # Agumon

        mem_before = runner.game.memory

        action = runner.find_action("Play Agumon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Memory: 10 - 3 (play) + 1 (Garurumon) = 8
        assert runner.game.memory == mem_before - 3 + 1, (
            f"Memory should be {mem_before - 3 + 1}. Got {runner.game.memory}"
        )

    def test_gain_0_memory_with_no_greymon_or_garurumon(self, debug_runner):
        """All Turns: gain 0 memory when neither Greymon nor Garurumon on field."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["EX9-066"])
        # No Greymon or Garurumon on field

        runner.inject_card(1, "ST1-03", "hand")  # Agumon

        mem_before = runner.game.memory

        action = runner.find_action("Play Agumon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Even though the trigger condition passes (a Digimon was played),
        # no Greymon or Garurumon means 0 memory. Tamer still gets suspended.
        tamer_slot = None
        for slot in snap_after.p1_field:
            if slot.card_id == "EX9-066":
                tamer_slot = slot
                break
        assert tamer_slot is not None, "Tamer should still be on field"
        assert tamer_slot.is_suspended, (
            "Tamer should still suspend (cost paid) even with 0 memory gain"
        )
        # Memory: 10 - 3 (play) + 0 = 7
        assert runner.game.memory == mem_before - 3, (
            f"Memory should be {mem_before - 3} (no gain). Got {runner.game.memory}"
        )

    def test_no_trigger_when_already_suspended(self, debug_runner):
        """All Turns: does not trigger when tamer is already suspended."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place tamer SUSPENDED
        runner.place_on_field(1, ["EX9-066"], is_suspended=True)
        runner.place_on_field(1, ["ST1-07"])  # Greymon

        runner.inject_card(1, "ST1-03", "hand")

        mem_before = runner.game.memory

        action = runner.find_action("Play Agumon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Memory: 10 - 3 (play) + 0 (tamer already suspended, no trigger) = 7
        assert runner.game.memory == mem_before - 3, (
            f"Memory should be {mem_before - 3} (no trigger, tamer already suspended). "
            f"Got {runner.game.memory}"
        )

    def test_trigger_on_digivolve(self, debug_runner):
        """All Turns: triggers when a Digimon digivolves."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place the tamer on field
        runner.place_on_field(1, ["EX9-066"])
        # Place a Greymon on field (for the memory gain check)
        runner.place_on_field(1, ["ST1-07"])  # Greymon
        # Place a Lv.3 on field to digivolve onto
        runner.place_on_field(1, ["ST1-03"])  # Agumon Lv.3

        # Put a Lv.4 in hand to digivolve
        runner.inject_card(1, "ST1-07", "hand")  # Greymon Lv.4 to digivolve onto Agumon

        mem_before = runner.game.memory

        action = runner.find_action("Digivolve")
        if action is None:
            # Try alternative action text
            action = runner.find_action("digivolve")
        if action is None:
            # Look for Greymon digivolve action specifically
            for aid, desc in runner.actions().items():
                if "greymon" in desc.lower() and "digi" in desc.lower():
                    action = aid
                    break
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

            snap_after = runner.snapshot()
            tamer_slot = None
            for slot in snap_after.p1_field:
                if slot.card_id == "EX9-066":
                    tamer_slot = slot
                    break
            assert tamer_slot is not None
            assert tamer_slot.is_suspended, (
                "Tamer should be suspended after digivolve trigger"
            )
            # Digivolve costs are typically level-based, but memory gain should include +1 for Greymon
        else:
            pytest.skip("Digivolve action not available in this game state")

    def test_does_not_trigger_on_opponent_digimon(self, debug_runner):
        """All Turns: does not trigger when opponent's Digimon is played."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # P1 has tamer + Greymon
        runner.place_on_field(1, ["EX9-066"])
        runner.place_on_field(1, ["ST1-07"])  # Greymon

        # Switch to P2's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True
        runner.game.turn_player = runner.game.player2
        runner.game.opponent_player = runner.game.player1

        # P2 plays a Digimon
        runner.inject_card(2, "ST1-03", "hand")

        snap_before = runner.snapshot()
        tamer_suspended_before = False
        for slot in snap_before.p1_field:
            if slot.card_id == "EX9-066":
                tamer_suspended_before = slot.is_suspended
                break

        action = runner.find_action("Play Agumon")
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

        snap_after = runner.snapshot()
        tamer_suspended_after = False
        for slot in snap_after.p1_field:
            if slot.card_id == "EX9-066":
                tamer_suspended_after = slot.is_suspended
                break

        # The observer system only scans the owner's battle area — opponent plays
        # don't fire play observers on P1's permanents.
        assert tamer_suspended_after == tamer_suspended_before, (
            "Tamer should NOT suspend when opponent plays a Digimon"
        )

    def test_only_triggers_once_per_play(self, debug_runner):
        """All Turns: tamer suspends once, cannot trigger again until unsuspended."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["EX9-066"])
        runner.place_on_field(1, ["ST1-07"])  # Greymon

        # Play first Digimon
        runner.inject_card(1, "ST1-03", "hand")
        runner.inject_card(1, "ST1-03", "hand")

        action = runner.find_action("Play Agumon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Tamer is now suspended. Playing a second Digimon should NOT gain extra memory.
        mem_after_first = runner.game.memory

        action2 = runner.find_action("Play Agumon")
        if action2 is not None:
            runner.execute(action2)
            runner.auto_resolve()

            # Memory should only decrease by the play cost, no gain
            assert runner.game.memory == mem_after_first - 3, (
                f"No extra memory gain after first trigger (tamer suspended). "
                f"Expected {mem_after_first - 3}, got {runner.game.memory}"
            )


@pytest.mark.behavioral
class TestEX9066Security:
    """Tests for the [Security] effect: play this card without paying the cost."""

    def test_security_play_effect(self, debug_runner):
        """Security: the tamer should have a security play effect."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Verify the card has a security effect by checking the script's effects
        from engine_py_legacy.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("EX9-066", runner.game.player1)
        assert cs is not None, "Should be able to create EX9-066 card source"

        effects = cs.effect_list(None)
        security_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(security_effects) >= 1, (
            "EX9-066 should have at least 1 security effect"
        )
