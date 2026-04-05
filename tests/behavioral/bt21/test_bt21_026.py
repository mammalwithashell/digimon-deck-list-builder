"""Behavioral tests for BT21-026 WarGreymon (Lv.6 Red Dragonkin, DP 11000, Cost 11).

Card text:
  When this card would be played, reduce the play cost by 2 for each of your
  opponent's Digimon.
  <Rush> <Raid> <Blocker>
  [All Turns] [Once Per Turn] When any of your opponent's Digimon are deleted,
  this Digimon may unsuspend.

No inherited effect.
"""

import pytest

# Clean filler decks to avoid interference from other card effects
FILLER_DECK = ["ST1-03"] * 50


@pytest.mark.behavioral
class TestBT21026WarGreymon:
    """Tests for BT21-026 WarGreymon."""

    # ── BeforePayCost: cost reduction ────────────────────────────────

    def test_cost_reduction_scales_with_opponent_digimon(self, debug_runner):
        """Play cost should be reduced by 2 per opponent Digimon on field."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=15)

        # Place 2 Digimon on opponent's field
        runner.place_on_field(2, ["ST1-03"])
        runner.place_on_field(2, ["ST1-03"])

        # Inject WarGreymon into hand
        runner.inject_card(1, "BT21-026", "hand")

        runner.set_phase("Main")
        snap_before = runner.snapshot()

        # Play WarGreymon -- base cost 11, reduction = 2 * 2 = 4, so cost = 7
        play_action = runner.find_action("Play WarGreymon")
        assert play_action is not None, (
            f"Should be able to play WarGreymon. Actions: {runner.actions()}"
        )
        runner.execute(play_action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        memory_spent = snap_before.memory - snap_after.memory
        assert memory_spent == 7, (
            f"Should spend 7 memory (11 - 2*2 opponent Digimon), spent {memory_spent}"
        )

    def test_cost_reduction_zero_opponents(self, debug_runner):
        """With no opponent Digimon, play cost should be full (11)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=15)

        # No opponent Digimon
        runner.inject_card(1, "BT21-026", "hand")
        runner.set_phase("Main")
        snap_before = runner.snapshot()

        play_action = runner.find_action("Play WarGreymon")
        assert play_action is not None
        runner.execute(play_action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        memory_spent = snap_before.memory - snap_after.memory
        assert memory_spent == 11, (
            f"Should spend full 11 memory with 0 opponent Digimon, spent {memory_spent}"
        )

    def test_cost_reduction_leak_guard(self, debug_runner):
        """Cost reduction should NOT apply to other cards being played.

        The BeforePayCost effect should check card_source is card (leak guard).
        """
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=15)

        # Put WarGreymon on field (so its effects are active on field)
        runner.place_on_field(1, ["BT21-026"])
        # Put 3 opponent Digimon (would give -6 reduction if leaked)
        runner.place_on_field(2, ["ST1-03"])
        runner.place_on_field(2, ["ST1-03"])
        runner.place_on_field(2, ["ST1-03"])

        # Inject ST1-07 Greymon (cost 7) into hand
        runner.inject_card(1, "ST1-07", "hand")
        runner.set_phase("Main")
        snap_before = runner.snapshot()

        play_action = runner.find_action("Play Greymon")
        if play_action is not None:
            runner.execute(play_action)
            runner.auto_resolve()

            snap_after = runner.snapshot()
            memory_spent = snap_before.memory - snap_after.memory
            # ST1-07 Greymon costs 7 -- should NOT get WarGreymon's -6 reduction
            assert memory_spent >= 5, (
                f"WarGreymon's cost reduction should not leak to other cards. "
                f"Spent only {memory_spent}"
            )

    # ── Keywords: Rush, Raid, Blocker ─────────────────────────────────

    def test_has_rush(self, debug_runner):
        """WarGreymon should have Rush -- can attack the turn it comes into play."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=15)

        runner.place_on_field(1, ["BT21-026"])
        runner.set_phase("Main")

        snap = runner.snapshot()
        wg_slot = next(
            (s for s in snap.p1_field if s.card_id == "BT21-026"), None
        )
        assert wg_slot is not None, "WarGreymon should be on field"
        assert "rush" in wg_slot.keywords, (
            f"WarGreymon should have Rush. Keywords: {wg_slot.keywords}"
        )

    def test_has_blocker(self, debug_runner):
        """WarGreymon should have Blocker."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=15)

        runner.place_on_field(1, ["BT21-026"])

        snap = runner.snapshot()
        wg_slot = next(
            (s for s in snap.p1_field if s.card_id == "BT21-026"), None
        )
        assert wg_slot is not None, "WarGreymon should be on field"
        assert "blocker" in wg_slot.keywords, (
            f"WarGreymon should have Blocker. Keywords: {wg_slot.keywords}"
        )

    def test_has_raid(self, debug_runner):
        """WarGreymon should have Raid keyword."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=15)

        runner.place_on_field(1, ["BT21-026"])

        game = runner.game
        for perm in game.player1.battle_area:
            if perm.top_card and perm.top_card.c_entity_base.card_id == "BT21-026":
                assert perm.has_keyword('_is_raid'), "WarGreymon should have Raid"
                break
        else:
            pytest.fail("WarGreymon not found on field")

    # ── [All Turns] [OPT] Unsuspend on opponent deletion ─────────────

    def test_unsuspend_on_opponent_digimon_deleted(self, debug_runner):
        """WarGreymon should unsuspend when an opponent's Digimon is deleted."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=15)

        # Place WarGreymon suspended (simulating it attacked)
        runner.place_on_field(1, ["BT21-026"], is_suspended=True)

        # Place opponent Digimon that will be deleted
        runner.place_on_field(2, ["ST1-03"])

        runner.set_phase("Main")

        # Verify WarGreymon is suspended
        snap_before = runner.snapshot()
        wg_slot = next(
            (s for s in snap_before.p1_field if s.card_id == "BT21-026"), None
        )
        assert wg_slot is not None
        assert wg_slot.is_suspended, "WarGreymon should start suspended"

        # Delete the opponent's Digimon programmatically
        game = runner.game
        opp = game.player2
        target = opp.battle_area[0]
        opp.delete_permanent(target)
        runner.auto_resolve()

        # WarGreymon should now be unsuspended
        snap_after = runner.snapshot()
        wg_slot_after = next(
            (s for s in snap_after.p1_field if s.card_id == "BT21-026"), None
        )
        assert wg_slot_after is not None, "WarGreymon should still be on field"
        assert not wg_slot_after.is_suspended, (
            "WarGreymon should unsuspend when opponent's Digimon is deleted"
        )

    def test_no_unsuspend_on_own_digimon_deleted(self, debug_runner):
        """WarGreymon should NOT unsuspend when own Digimon is deleted."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=15)

        runner.place_on_field(1, ["BT21-026"], is_suspended=True)
        # Place own Digimon that will be deleted
        runner.place_on_field(1, ["ST1-03"])

        runner.set_phase("Main")

        # Delete own Digimon
        game = runner.game
        own = game.player1
        target = None
        for p in own.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "ST1-03":
                target = p
                break
        assert target is not None, "Should have an ST1-03 to delete"
        own.delete_permanent(target)
        runner.auto_resolve()

        # WarGreymon should remain suspended
        snap_after = runner.snapshot()
        wg_slot = next(
            (s for s in snap_after.p1_field if s.card_id == "BT21-026"), None
        )
        assert wg_slot is not None, "WarGreymon should still be on field"
        assert wg_slot.is_suspended, (
            "WarGreymon should remain suspended when own Digimon is deleted"
        )

    def test_unsuspend_once_per_turn(self, debug_runner):
        """The unsuspend effect should trigger only once per turn."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=15)

        runner.place_on_field(1, ["BT21-026"], is_suspended=True)
        runner.place_on_field(2, ["ST1-03"])
        runner.place_on_field(2, ["ST1-03"])

        runner.set_phase("Main")

        game = runner.game
        opp = game.player2

        # Delete first opponent Digimon -- should trigger unsuspend
        target1 = opp.battle_area[0]
        opp.delete_permanent(target1)
        runner.auto_resolve()

        snap1 = runner.snapshot()
        wg1 = next(
            (s for s in snap1.p1_field if s.card_id == "BT21-026"), None
        )
        assert wg1 is not None
        assert not wg1.is_suspended, "Should unsuspend after first deletion"

        # Re-suspend WarGreymon manually
        for perm in game.player1.battle_area:
            if perm.top_card and perm.top_card.c_entity_base.card_id == "BT21-026":
                perm.suspend()
                break

        # Delete second opponent Digimon -- should NOT trigger again (once per turn)
        if opp.battle_area:
            target2 = opp.battle_area[0]
            opp.delete_permanent(target2)
            runner.auto_resolve()

        snap2 = runner.snapshot()
        wg2 = next(
            (s for s in snap2.p1_field if s.card_id == "BT21-026"), None
        )
        assert wg2 is not None
        assert wg2.is_suspended, (
            "Should remain suspended -- once per turn limit should prevent second activation"
        )

    def test_unsuspend_not_required_when_already_unsuspended(self, debug_runner):
        """If WarGreymon is already unsuspended, effect is optional and does nothing harmful."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=15)

        # Place WarGreymon already unsuspended
        runner.place_on_field(1, ["BT21-026"])
        runner.place_on_field(2, ["ST1-03"])

        runner.set_phase("Main")

        game = runner.game
        opp = game.player2

        # Delete opponent Digimon -- effect triggers but WarGreymon is already unsuspended
        target = opp.battle_area[0]
        opp.delete_permanent(target)
        runner.auto_resolve()

        # Should not crash, WarGreymon should still be unsuspended
        snap = runner.snapshot()
        wg = next(
            (s for s in snap.p1_field if s.card_id == "BT21-026"), None
        )
        assert wg is not None
        assert not wg.is_suspended, "WarGreymon should remain unsuspended"
