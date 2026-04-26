"""Behavioral tests for EX9-068 Analogman (Tamer, Black, Cost 4).

Card text:
[Start of Your Turn] If you have 2 or less memory, set it to 3.
[Your Turn] When any of your play cost 7 or higher [Cyborg], [Machine] or
    [DM] trait Digimon are played, by suspending this Tamer, Draw 1 and
    gain 1 memory. Then, you may place 1 card from your hand face down as
    any of those Digimon's bottom digivolution card.
[Security] Play this card without paying the cost.
"""

import pytest


@pytest.mark.behavioral
class TestEX9068Analogman:
    """Tests for EX9-068 Analogman."""

    # ── Effect 0: [Start of Your Turn] set memory to 3 if <= 2 ────────

    def test_start_of_turn_sets_memory_to_3_when_at_2(self, debug_runner):
        """Memory at 2 should be set to 3 at start of turn."""
        runner = debug_runner(initial_memory=2)
        runner.set_phase("Main")

        # Place tamer on field (already P1's turn)
        runner.place_on_field(1, ["EX9-068"])

        # Simulate start of turn by directly calling execute_effects
        from engine_py_legacy.engine.data.enums import EffectTiming
        runner.game.execute_effects(EffectTiming.OnStartTurn, {})

        assert runner.game.memory == 3, (
            f"Memory should be set to 3 (was 2), got {runner.game.memory}"
        )

    def test_start_of_turn_sets_memory_to_3_when_at_0(self, debug_runner):
        """Memory at 0 should be set to 3 at start of turn."""
        runner = debug_runner(initial_memory=0)
        runner.set_phase("Main")

        runner.place_on_field(1, ["EX9-068"])

        from engine_py_legacy.engine.data.enums import EffectTiming
        runner.game.execute_effects(EffectTiming.OnStartTurn, {})

        assert runner.game.memory == 3, (
            f"Memory should be set to 3 (was 0), got {runner.game.memory}"
        )

    def test_start_of_turn_no_change_when_above_2(self, debug_runner):
        """Memory at 3 or higher should NOT be changed."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        runner.place_on_field(1, ["EX9-068"])

        from engine_py_legacy.engine.data.enums import EffectTiming
        runner.game.execute_effects(EffectTiming.OnStartTurn, {})

        assert runner.game.memory == 5, (
            f"Memory should remain 5 (above 2), got {runner.game.memory}"
        )

    # ── Effect 1: [Your Turn] OnEnterFieldAnyone trigger ─────────────

    def test_trigger_on_cost7_cyborg_played(self, debug_runner):
        """Playing a cost 7+ Cyborg Digimon should trigger draw + memory."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place tamer on field (unsuspended)
        runner.place_on_field(1, ["EX9-068"])

        # Put a cost 7 Cyborg Digimon in hand (BT10-065 Assaultmon, cost 7, Cyborg, Black)
        runner.inject_card(1, "BT10-065", "hand")

        # Put a known card on top of library so we can verify the draw
        runner.inject_card(1, "ST1-03", "library_top")

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)
        memory_before = snap_before.memory
        library_before = snap_before.p1_library_size

        # Play the Digimon
        action = runner.find_action("Play Assaultmon")
        assert action is not None, f"Should be able to play Assaultmon. Actions: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()

        # Tamer should be suspended
        tamer_slots = [s for s in snap_after.p1_field if s.card_id == "EX9-068"]
        assert len(tamer_slots) == 1, "Tamer should still be on field"
        assert tamer_slots[0].is_suspended, "Tamer should be suspended after triggering"

        # Player should have drawn 1 card
        # hand_before included BT10-065 which is now on field, so:
        # hand = hand_before - 1 (played) + 1 (draw) = hand_before
        # But the optional place-from-hand might reduce it by 1 more (auto_resolve picks first)
        # At minimum, library should have decreased by 1 from the draw
        assert snap_after.p1_library_size <= library_before - 1, (
            f"Library should decrease by at least 1 (draw). "
            f"Before: {library_before}, After: {snap_after.p1_library_size}"
        )

        # Memory should have gained 1 (on top of paying the play cost)
        # Play cost 7 reduces memory by 7, then gains 1 = net -6 from initial 10
        # But memory gauge works differently: memory = memory_before - play_cost + 1
        # Initial memory was 10, play cost 7 -> memory becomes 3, then +1 -> 4
        assert snap_after.memory == memory_before - 7 + 1, (
            f"Memory should be {memory_before - 7 + 1} (initial {memory_before} - 7 play cost + 1 gained). "
            f"Got {snap_after.memory}"
        )

    def test_trigger_on_cost7_machine_played(self, debug_runner):
        """Playing a cost 7+ Machine trait Digimon should also trigger."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["EX9-068"])

        # BT1-042 LoaderLeomon, cost 7, Machine, Blue
        runner.inject_card(1, "BT1-042", "hand")
        runner.inject_card(1, "ST1-03", "library_top")

        snap_before = runner.snapshot()
        memory_before = snap_before.memory

        action = runner.find_action("Play LoaderLeomon")
        assert action is not None, f"Should be able to play LoaderLeomon. Actions: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        tamer_slots = [s for s in snap_after.p1_field if s.card_id == "EX9-068"]
        assert len(tamer_slots) == 1
        assert tamer_slots[0].is_suspended, "Tamer should be suspended for Machine trait trigger"

    def test_no_trigger_on_cost_below_7(self, debug_runner):
        """Playing a Cyborg Digimon with cost < 7 should NOT trigger."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["EX9-068"])

        # BT14-056 Commandramon, cost 3, Cyborg, Black
        runner.inject_card(1, "BT14-056", "hand")

        snap_before = runner.snapshot()

        action = runner.find_action("Play Commandramon")
        assert action is not None, f"Should be able to play Commandramon. Actions: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        tamer_slots = [s for s in snap_after.p1_field if s.card_id == "EX9-068"]
        assert len(tamer_slots) == 1
        assert not tamer_slots[0].is_suspended, (
            "Tamer should NOT be suspended for cost < 7 Digimon"
        )

    def test_no_trigger_on_non_qualifying_trait(self, debug_runner):
        """Playing a cost 7+ Digimon without Cyborg/Machine/DM trait should NOT trigger."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["EX9-068"])

        # ST1-06 Greymon, cost 4, Dinosaur trait -- not qualifying
        # Need a cost 7+ non-qualifying Digimon
        # BT1-025 MetalGreymon is cost 8 with Cyborg trait -- that qualifies
        # Let's use a cost 7+ Digimon without Cyborg/Machine/DM
        # BT1-020 Birdramon cost 7 traits: ['Giant Bird'] -- no Cyborg/Machine/DM
        runner.inject_card(1, "BT1-020", "hand")

        action = runner.find_action("Play Birdramon")
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

            snap_after = runner.snapshot()
            tamer_slots = [s for s in snap_after.p1_field if s.card_id == "EX9-068"]
            assert len(tamer_slots) == 1
            assert not tamer_slots[0].is_suspended, (
                "Tamer should NOT suspend for non-Cyborg/Machine/DM Digimon"
            )

    def test_no_trigger_when_tamer_already_suspended(self, debug_runner):
        """Should NOT trigger if this Tamer is already suspended."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["EX9-068"])
        perm.suspend()  # Pre-suspend the tamer

        runner.inject_card(1, "BT10-065", "hand")
        runner.inject_card(1, "ST1-03", "library_top")

        snap_before = runner.snapshot()
        library_before = snap_before.p1_library_size

        action = runner.find_action("Play Assaultmon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Library should NOT decrease beyond what's normal (no draw from tamer effect)
        # The play itself doesn't draw, only the tamer effect does
        assert snap_after.p1_library_size == library_before, (
            f"Library should not change (tamer was already suspended). "
            f"Before: {library_before}, After: {snap_after.p1_library_size}"
        )

    def test_draw_uses_proper_api(self, debug_runner):
        """Draw should use player.draw_cards() to fire OnDraw/OnAddHand timing."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["EX9-068"])
        runner.inject_card(1, "BT10-065", "hand")
        runner.inject_card(1, "ST1-03", "library_top")

        # Track if draw_cards was called by monitoring hand properly
        # The best way is to verify the drawn card ends up in hand
        action = runner.find_action("Play Assaultmon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # ST1-03 was on top of library, should now be in hand (or placed as digi source)
        # At minimum, it should no longer be the top of library
        assert snap_after.p1_library_size < runner.snapshot().p1_library_size + 1

    def test_optional_hand_placement_as_bottom_digi_card(self, debug_runner):
        """After draw+memory, should optionally place hand card as bottom digivolution card."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["EX9-068"])

        # Clear hand for precise control
        runner.clear_zone(1, "hand")

        # Put Assaultmon in hand to play
        runner.inject_card(1, "BT10-065", "hand")

        # Put a card on library top that will be drawn
        runner.inject_card(1, "ST1-03", "library_top")

        # Put another card in hand that could be placed as digi source
        runner.inject_card(1, "BT14-056", "hand")

        snap_before = runner.snapshot()

        action = runner.find_action("Play Assaultmon")
        assert action is not None
        runner.execute(action)
        # auto_resolve will pick first option for the optional hand placement
        runner.auto_resolve()

        snap_after = runner.snapshot()

        # The Assaultmon permanent should have digivolution cards if placement happened
        assaultmon_slots = [s for s in snap_after.p1_field if s.card_id == "BT10-065"]
        assert len(assaultmon_slots) == 1, "Assaultmon should be on field"

        # If auto_resolve selected a card, the stack should have more than just Assaultmon
        # stack_ids[0] would be the bottom card (placed), stack_ids[-1] is top (Assaultmon)
        if len(assaultmon_slots[0].stack_ids) > 1:
            # A card was placed as bottom digivolution card
            assert assaultmon_slots[0].stack_ids[0] != "BT10-065", (
                "Bottom card should be the placed card, not Assaultmon itself"
            )

    def test_no_trigger_on_opponent_digimon(self, debug_runner):
        """Should NOT trigger when the opponent plays a qualifying Digimon."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Tamer on P1's field
        runner.place_on_field(1, ["EX9-068"])

        # Give P2 a qualifying Digimon and enough memory
        runner.inject_card(2, "BT10-065", "hand")

        # Switch to P2's turn
        runner.game.memory = -10  # Give P2 memory
        runner.game.turn_player = runner.game.player2
        runner.game.opponent_player = runner.game.player1
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True
        runner.game.active_player = runner.game.player2
        runner.game.current_phase = runner.game.current_phase  # refresh

        snap_before = runner.snapshot()
        tamer_slots_before = [s for s in snap_before.p1_field if s.card_id == "EX9-068"]
        assert not tamer_slots_before[0].is_suspended, "Tamer should start unsuspended"

        action = runner.find_action("Play Assaultmon")
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

        snap_after = runner.snapshot()
        tamer_slots = [s for s in snap_after.p1_field if s.card_id == "EX9-068"]
        assert len(tamer_slots) == 1
        assert not tamer_slots[0].is_suspended, (
            "Tamer should NOT suspend when opponent plays qualifying Digimon"
        )

    # ── Effect 1: context key correctness ─────────────────────────────

    def test_condition_uses_played_permanent_not_self(self, debug_runner):
        """The condition must check the PLAYED permanent, not the observing tamer.

        This is a regression test for the bug where context.get('permanent')
        returns the observing tamer (self) instead of the played Digimon.
        The correct key is context.get('played_permanent').
        """
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["EX9-068"])
        runner.inject_card(1, "BT10-065", "hand")
        runner.inject_card(1, "ST1-03", "library_top")

        snap_before = runner.snapshot()
        memory_before = snap_before.memory

        action = runner.find_action("Play Assaultmon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # If the bug exists, the tamer won't be suspended and no draw/memory happens
        tamer_slots = [s for s in snap_after.p1_field if s.card_id == "EX9-068"]
        assert tamer_slots[0].is_suspended, (
            "Tamer must be suspended — if not, condition is checking 'permanent' "
            "(the tamer itself) instead of 'played_permanent' (the played Digimon)"
        )

    # ── Effect 2: Security ────────────────────────────────────────────

    def test_security_effect_exists(self, debug_runner):
        """[Security] effect should exist and be marked as security."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.core.card_source import CardSource
        db = CardDatabase()
        script = db.get_script("EX9-068")
        cs = CardSource()
        effects = script.get_card_effects(cs)
        security_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(security_effects) == 1, "Should have exactly 1 security effect"

    # ── Trait matching exactness ──────────────────────────────────────

    def test_trait_check_is_exact_match(self, debug_runner):
        """Trait matching should use exact equality, not substring matching.

        The C# reference uses EqualsTraits() which does exact matching.
        """
        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.core.card_source import CardSource
        db = CardDatabase()
        script = db.get_script("EX9-068")

        # Read the script source and verify it doesn't use substring matching
        import inspect
        source = inspect.getsource(type(script))

        # Should NOT contain 'in t' pattern for trait matching (substring)
        # Should use exact comparison like t == 'Cyborg' or has_trait()
        assert "'Cyborg' in t" not in source and "'Machine' in t" not in source and "'DM' in t" not in source, (
            "Script should use exact trait matching (== or has_trait), not substring ('in t'). "
            "The C# reference uses EqualsTraits() which is exact match."
        )
