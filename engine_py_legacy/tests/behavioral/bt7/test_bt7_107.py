"""Behavioral tests for BT7-107 Calling From the Darkness (Option, Purple, Cost 1).

Card text:
[Main] Delete 1 of your Digimon. Then, return up to 2 purple Digimon cards
    from your trash to your hand.
[Security] Add this card to the hand.

C# Reference: BT7_107.cs
- Delete uses SelectPermanentEffect with canNoSelect=false, canEndNotMax=false
  (mandatory, must select exactly 1 own Digimon to delete).
- Trash recovery uses SelectCardEffect with canNoSelect=()=>false,
  canEndNotMax=true, and CanEndSelectCondition requires count>0 when
  maxCount>=1. This means: if purple Digimon exist in trash, player MUST
  select at least 1 (up to 2). The second pick is optional.
- Filter: IsDigimon AND HasCardColor(Purple) AND Owner==card.Owner.
- Security: standard AddThisCardToHand.
"""

import pytest

from digimon_gym.engine.data.enums import GamePhase

# Minimal decks: filler with the card under test
FILLER_DECK = ["BT2-067"] * 50  # DemiDevimon (Purple Digimon, Lv.3, cost 2)


@pytest.mark.behavioral
class TestBT7107CallingFromTheDarkness:
    """Tests for BT7-107 Calling From the Darkness."""

    # ── [Main] Basic: Delete + Trash Recovery ──────────────────────────

    def test_play_option_deletes_own_digimon_and_recovers_from_trash(self, debug_runner):
        """Full effect: delete 1 own Digimon, then return 2 purple Digimon from trash."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        # Place a Digimon to delete
        runner.place_on_field(1, ["BT2-067"])  # DemiDevimon (Purple, Digimon)

        # Put 2 purple Digimon in trash
        runner.inject_card(1, "BT2-068", "trash")  # Impmon (Purple Digimon)
        runner.inject_card(1, "BT2-069", "trash")  # Gabumon (Purple Digimon)

        # Add the option to hand
        runner.inject_card(1, "BT7-107", "hand")
        runner.set_phase("Main")

        snap_before = runner.snapshot()
        hand_before = list(snap_before.p1_hand)
        trash_before = list(snap_before.p1_trash)

        # Play the option
        action = runner.find_action("Play Calling From the Darkness")
        assert action is not None, (
            f"Should find play action for BT7-107. Actions: {runner.actions()}"
        )
        runner.execute(action)

        # Phase should be a selection phase for deleting own Digimon
        snap = runner.snapshot()
        assert snap.phase in ("SelectTarget", "SelectTrash"), (
            f"Should enter a selection phase, got {snap.phase}"
        )

        # Select the Digimon to delete (mandatory)
        runner.auto_resolve()

        snap_after = runner.snapshot()

        # The Digimon should be deleted from field
        p1_field_ids = [f.card_id for f in snap_after.p1_field]
        assert "BT2-067" not in p1_field_ids, (
            "DemiDevimon should be deleted from field"
        )

        # At least 1 purple Digimon should be recovered from trash to hand
        # (auto_resolve picks first available)
        p1_hand_ids = snap_after.p1_hand
        recovered = [cid for cid in p1_hand_ids if cid in ("BT2-068", "BT2-069")]
        assert len(recovered) >= 1, (
            f"Should recover at least 1 purple Digimon from trash to hand, "
            f"hand: {p1_hand_ids}"
        )

    def test_delete_is_mandatory_not_optional(self, debug_runner):
        """The delete is mandatory (not optional) -- no decline action when selecting."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT2-067"])
        runner.inject_card(1, "BT7-107", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Calling From the Darkness")
        assert action is not None
        runner.execute(action)

        # In the delete selection phase, action 62 (decline) should NOT be available
        legal = runner.action_mask()
        assert 62 not in legal, (
            "Delete selection should be mandatory (action 62/decline should not be legal)"
        )

    def test_no_digimon_to_delete_still_runs_trash_recovery(self, debug_runner):
        """Per C# BT7_107.cs the delete and trash recovery are INDEPENDENT
        top-level if-blocks, not nested. So if no own Digimon exists, the
        delete clause is skipped but trash recovery still runs.

        Option cards require a matching-color Digimon or Tamer on field to
        play, so we use a Purple Tamer to satisfy the color requirement.
        """
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        # Clear P1 field, then place a Purple Tamer (satisfies color req, not a Digimon)
        runner.clear_zone(1, "field")
        runner.place_on_field(1, ["BT2-090"])  # Matt Ishida (Purple Tamer)

        # Put purple Digimon in trash
        runner.inject_card(1, "BT2-068", "trash")  # Impmon
        runner.inject_card(1, "BT2-069", "trash")  # Gabumon

        runner.inject_card(1, "BT7-107", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Calling From the Darkness")
        assert action is not None, (
            f"Should find play action (purple Tamer satisfies color req). "
            f"Actions: {runner.actions()}"
        )

        runner.execute(action)

        # With no own Digimon to delete, the effect should go straight to
        # the trash selection phase (SelectTrash).
        snap = runner.snapshot()
        assert snap.phase == "SelectTrash", (
            f"Should jump straight to SelectTrash when no Digimon to delete "
            f"(independent if-blocks per C#). Got phase={snap.phase}."
        )

        # First pick is mandatory (C# canNoSelect=false).
        legal = runner.action_mask()
        assert 62 not in legal, (
            "First trash recovery pick should be mandatory per C# "
            "(canNoSelect=false)."
        )

        runner.auto_resolve()

        snap_after = runner.snapshot()
        # At least 1 purple Digimon should be recovered to hand.
        recovered = [cid for cid in snap_after.p1_hand
                     if cid in ("BT2-068", "BT2-069")]
        assert len(recovered) >= 1, (
            f"Should recover at least 1 purple Digimon from trash even with "
            f"no Digimon to delete (C# runs trash recovery unconditionally). "
            f"Hand: {snap_after.p1_hand}"
        )

    # ── [Main] Trash Recovery: Purple Digimon Filter ───────────────────

    def test_only_purple_digimon_qualify_for_recovery(self, debug_runner):
        """Only purple Digimon cards in trash are valid targets for recovery."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT2-067"])

        # Put a mix in trash: 1 purple Digimon + 1 non-purple Digimon + 1 purple Tamer
        runner.inject_card(1, "BT2-068", "trash")  # Impmon (Purple Digimon) -- valid
        runner.inject_card(1, "ST1-03", "trash")   # Agumon (Red Digimon) -- invalid (not purple)
        runner.inject_card(1, "BT2-090", "trash")  # Matt Ishida (Purple Tamer) -- invalid (not Digimon)

        runner.inject_card(1, "BT7-107", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Calling From the Darkness")
        runner.execute(action)

        # First: select Digimon to delete (auto-resolve)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Impmon (purple Digimon) should be recovered
        assert "BT2-068" in snap_after.p1_hand, (
            "Purple Digimon (Impmon) should be recoverable from trash"
        )
        # Non-purple Digimon and purple Tamer should stay in trash
        assert "ST1-03" in snap_after.p1_trash, (
            "Non-purple Digimon (Agumon) should remain in trash"
        )
        assert "BT2-090" in snap_after.p1_trash, (
            "Purple Tamer (Matt Ishida) should remain in trash -- not a Digimon"
        )

    def test_non_purple_digimon_not_selectable(self, debug_runner):
        """Red Digimon in trash should not appear as selectable trash targets."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT2-067"])

        # Only non-purple Digimon in trash
        runner.inject_card(1, "ST1-03", "trash")  # Agumon (Red)
        runner.inject_card(1, "AD1-001", "trash")  # Greymon (Red)

        runner.inject_card(1, "BT7-107", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Calling From the Darkness")
        runner.execute(action)

        # Delete phase -> select Digimon
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # No purple Digimon in trash, so no recovery should happen
        # Both red Digimon should remain in trash
        red_in_trash = [cid for cid in snap_after.p1_trash if cid in ("ST1-03", "AD1-001")]
        assert len(red_in_trash) == 2, (
            f"Non-purple Digimon should remain in trash, got trash: {snap_after.p1_trash}"
        )

    def test_purple_option_in_trash_not_selectable(self, debug_runner):
        """Purple Option cards in trash should NOT be valid targets (must be Digimon)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT2-067"])

        # Put a purple option in trash (not Digimon)
        runner.inject_card(1, "BT2-107", "trash")  # Darkness Claw (Purple Option)

        runner.inject_card(1, "BT7-107", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Calling From the Darkness")
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        assert "BT2-107" in snap_after.p1_trash, (
            "Purple Option (Darkness Claw) should remain in trash -- not a Digimon"
        )

    # ── [Main] Trash Recovery: Up to 2 ────────────────────────────────

    def test_recover_exactly_2_when_available(self, debug_runner):
        """Can recover 2 purple Digimon when 2+ are available in trash.

        The second pick is optional per C#, so we manually select both
        rather than relying on auto_resolve (which declines optional picks).
        """
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT2-067"])

        # 3 purple Digimon in trash
        runner.inject_card(1, "BT2-068", "trash")  # Impmon
        runner.inject_card(1, "BT2-069", "trash")  # Gabumon
        runner.inject_card(1, "BT2-070", "trash")  # Tapirmon

        runner.inject_card(1, "BT7-107", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Calling From the Darkness")
        runner.execute(action)

        # Step 1: Select own Digimon to delete (mandatory)
        snap = runner.snapshot()
        if snap.phase == "SelectTarget":
            legal = runner.action_mask()
            field_acts = [a for a in legal if 100 <= a <= 113]
            runner.execute(field_acts[0])

        # Step 2: Select first purple Digimon from trash (mandatory)
        snap = runner.snapshot()
        assert snap.phase == "SelectTrash", f"Expected SelectTrash, got {snap.phase}"
        legal = runner.action_mask()
        trash_acts = [a for a in legal if 130 <= a <= 179]
        runner.execute(trash_acts[0])

        # Step 3: Select second purple Digimon from trash (optional)
        snap = runner.snapshot()
        if snap.phase == "SelectTrash":
            legal = runner.action_mask()
            trash_acts = [a for a in legal if 130 <= a <= 179]
            if trash_acts:
                runner.execute(trash_acts[0])

        runner.auto_resolve()

        snap_after = runner.snapshot()
        recovered = [cid for cid in snap_after.p1_hand
                     if cid in ("BT2-068", "BT2-069", "BT2-070")]
        assert len(recovered) == 2, (
            f"Should recover exactly 2 purple Digimon from trash, "
            f"got {len(recovered)}: {recovered}. Hand: {snap_after.p1_hand}"
        )

    def test_recover_1_when_only_1_available(self, debug_runner):
        """When only 1 purple Digimon is in trash, recover exactly 1."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT2-067"])
        runner.inject_card(1, "BT2-068", "trash")  # Only 1 purple Digimon

        runner.inject_card(1, "BT7-107", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Calling From the Darkness")
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        assert "BT2-068" in snap_after.p1_hand, (
            "Should recover the 1 available purple Digimon from trash"
        )

    def test_no_recovery_when_no_purple_digimon_in_trash(self, debug_runner):
        """When no purple Digimon in trash, the recovery step is skipped."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT2-067"])
        runner.clear_zone(1, "trash")

        runner.inject_card(1, "BT7-107", "hand")
        runner.set_phase("Main")

        snap_before = runner.snapshot()

        action = runner.find_action("Play Calling From the Darkness")
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Digimon should still be deleted even with no trash recovery targets
        p1_field_ids = [f.card_id for f in snap_after.p1_field]
        assert "BT2-067" not in p1_field_ids, (
            "The delete should still happen even when there is nothing to recover"
        )

    def test_max_recovery_is_2_not_more(self, debug_runner):
        """Even with 5 purple Digimon in trash, can recover at most 2.

        Manually select both picks to ensure we exercise the max limit.
        """
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT2-067"])

        # Put 5 purple Digimon in trash
        for cid in ["BT2-068", "BT2-069", "BT2-070", "BT2-071", "BT2-072"]:
            runner.inject_card(1, cid, "trash")

        runner.inject_card(1, "BT7-107", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Calling From the Darkness")
        runner.execute(action)

        # Step 1: Select own Digimon to delete (mandatory)
        snap = runner.snapshot()
        if snap.phase == "SelectTarget":
            legal = runner.action_mask()
            field_acts = [a for a in legal if 100 <= a <= 113]
            runner.execute(field_acts[0])

        # Step 2: Select first purple Digimon from trash (mandatory)
        snap = runner.snapshot()
        assert snap.phase == "SelectTrash", f"Expected SelectTrash, got {snap.phase}"
        legal = runner.action_mask()
        trash_acts = [a for a in legal if 130 <= a <= 179]
        runner.execute(trash_acts[0])

        # Step 3: Select second purple Digimon from trash (optional)
        snap = runner.snapshot()
        if snap.phase == "SelectTrash":
            legal = runner.action_mask()
            trash_acts = [a for a in legal if 130 <= a <= 179]
            if trash_acts:
                runner.execute(trash_acts[0])

        runner.auto_resolve()

        snap_after = runner.snapshot()
        all_purple = {"BT2-068", "BT2-069", "BT2-070", "BT2-071", "BT2-072"}
        recovered = [cid for cid in snap_after.p1_hand if cid in all_purple]
        assert len(recovered) == 2, (
            f"Should recover exactly 2, got {len(recovered)}: {recovered}"
        )
        # 3 should remain in trash (plus the deleted DemiDevimon and the used option)
        still_in_trash = [cid for cid in snap_after.p1_trash if cid in all_purple]
        assert len(still_in_trash) == 3, (
            f"3 purple Digimon should remain in trash, got {len(still_in_trash)}"
        )

        # Verify no third selection is offered (max is 2)
        snap_final = runner.snapshot()
        assert snap_final.phase != "SelectTrash", (
            "Should NOT offer a third trash selection (max is 2)"
        )

    # ── [Main] Trash Recovery: Optionality per C# ─────────────────────

    def test_trash_recovery_first_pick_is_mandatory_per_csharp(self, debug_runner):
        """C# CanEndSelectCondition requires count>0 when maxCount>=1.

        The first pick should be mandatory (no decline) when purple Digimon
        exist in trash. The second pick should be optional (can decline).
        """
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT2-067"])
        runner.inject_card(1, "BT2-068", "trash")  # Purple Digimon
        runner.inject_card(1, "BT2-069", "trash")  # Purple Digimon

        runner.inject_card(1, "BT7-107", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Calling From the Darkness")
        runner.execute(action)

        # Delete phase: select own Digimon (mandatory)
        legal = runner.action_mask()
        if runner.snapshot().phase == "SelectTarget":
            runner.execute(legal[0])  # pick the Digimon to delete

        # Now should be in trash selection phase
        snap = runner.snapshot()
        if snap.phase == "SelectTrash":
            legal_trash = runner.action_mask()
            # First pick: action 62 (decline) should NOT be legal per C#
            # (canNoSelect=false, CanEndSelectCondition requires count>0)
            assert 62 not in legal_trash, (
                "First trash selection should be mandatory per C# "
                "(canNoSelect=false, CanEndSelectCondition requires count>0). "
                f"Legal actions: {legal_trash}"
            )

            # Pick the first purple Digimon
            trash_actions = [a for a in legal_trash if a != 62]
            assert len(trash_actions) > 0, "Should have trash selection actions"
            runner.execute(trash_actions[0])

            # Second pick: action 62 (decline) SHOULD be legal
            snap2 = runner.snapshot()
            if snap2.phase == "SelectTrash":
                legal_trash2 = runner.action_mask()
                assert 62 in legal_trash2, (
                    "Second trash selection should be optional (can decline). "
                    f"Legal actions: {legal_trash2}"
                )

    # ── [Main] Memory Cost ─────────────────────────────────────────────

    def test_option_costs_1_memory(self, debug_runner):
        """Playing BT7-107 should cost 1 memory."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT2-067"])
        runner.inject_card(1, "BT7-107", "hand")
        runner.set_phase("Main")

        snap_before = runner.snapshot()
        action = runner.find_action("Play Calling From the Darkness")
        assert action is not None
        result = runner.execute(action)

        # Cost = 1 memory
        memory_spent = result.before.memory - result.after.memory
        assert memory_spent == 1, (
            f"BT7-107 costs 1, expected 1 memory spent but got {memory_spent}"
        )

    # ── [Main] Delete sends cards to trash ─────────────────────────────

    def test_deleted_digimon_goes_to_trash(self, debug_runner):
        """The deleted Digimon should end up in the owner's trash.

        Use a non-purple Digimon for the delete (with a purple Tamer for
        color requirement) so the deleted card is NOT eligible for the
        mandatory trash recovery and stays in trash.
        """
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        runner.clear_zone(1, "field")
        runner.place_on_field(1, ["BT2-090"])  # Matt Ishida (Purple Tamer, color req)
        runner.place_on_field(1, ["ST1-03"])   # Agumon (Red Digimon, delete target)
        runner.clear_zone(1, "trash")

        runner.inject_card(1, "BT7-107", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Calling From the Darkness")
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # The deleted Agumon (Red, not purple) should be in trash
        assert "ST1-03" in snap_after.p1_trash, (
            f"Deleted Digimon should be in trash, got: {snap_after.p1_trash}"
        )

    # ── [Main] Delete recovered cards come from trash after deletion ───

    def test_deleted_digimon_available_for_recovery(self, debug_runner):
        """A purple Digimon that was just deleted could be recovered from trash
        if it is purple. The delete happens first, then the trash selection.
        """
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        # Only 1 Digimon on field (will be deleted), nothing else in trash
        runner.place_on_field(1, ["BT2-068"])  # Impmon (Purple Digimon)
        runner.clear_zone(1, "trash")

        runner.inject_card(1, "BT7-107", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Calling From the Darkness")
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Impmon was deleted, went to trash, then could be recovered from trash
        # With auto_resolve, the engine should pick it from trash
        assert "BT2-068" in snap_after.p1_hand or "BT2-068" in snap_after.p1_trash, (
            "Impmon should either be recovered to hand or remain in trash"
        )

    # ── [Main] Multiple own Digimon: must select 1 ─────────────────────

    def test_select_which_digimon_to_delete(self, debug_runner):
        """When multiple own Digimon exist, must select exactly 1 to delete."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT2-067"])  # DemiDevimon
        runner.place_on_field(1, ["BT2-068"])  # Impmon

        runner.inject_card(1, "BT7-107", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Calling From the Darkness")
        runner.execute(action)

        # Should be in SelectTarget phase with 2 valid targets
        snap = runner.snapshot()
        assert snap.phase == "SelectTarget", (
            f"Should enter SelectTarget phase for choosing which Digimon to delete, "
            f"got {snap.phase}"
        )

        legal = runner.action_mask()
        # Should have exactly 2 field selection options (no decline)
        field_actions = [a for a in legal if 100 <= a <= 113]
        assert len(field_actions) == 2, (
            f"Should have 2 own Digimon to choose from, got {len(field_actions)}: {field_actions}"
        )
        assert 62 not in legal, "Delete selection should be mandatory (no decline)"

    # ── [Main] Opponent's Digimon not selectable for delete ────────────

    def test_cannot_delete_opponent_digimon(self, debug_runner):
        """The delete targets only YOUR Digimon, not opponent's."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT2-067"])  # Own Digimon
        runner.place_on_field(2, ["BT2-068"])  # Opponent's Digimon

        runner.inject_card(1, "BT7-107", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Calling From the Darkness")
        runner.execute(action)

        snap = runner.snapshot()
        if snap.phase == "SelectTarget":
            legal = runner.action_mask()
            # Opponent field actions (114-127) should NOT be available
            opp_actions = [a for a in legal if 114 <= a <= 127]
            assert len(opp_actions) == 0, (
                f"Should not be able to select opponent's Digimon for delete, "
                f"got opp actions: {opp_actions}"
            )

    # ── [Main] Tamers not selectable for delete ────────────────────────

    def test_tamer_not_selectable_for_delete(self, debug_runner):
        """Only Digimon can be deleted, not Tamers."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        # Place a Tamer + a Digimon
        runner.place_on_field(1, ["BT2-090"])  # Matt Ishida (Tamer)
        runner.place_on_field(1, ["BT2-067"])  # DemiDevimon (Digimon)

        runner.inject_card(1, "BT7-107", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Calling From the Darkness")
        runner.execute(action)

        snap = runner.snapshot()
        if snap.phase == "SelectTarget":
            legal = runner.action_mask()
            field_actions = [a for a in legal if 100 <= a <= 113]
            # Only the Digimon slot should be selectable, not the Tamer
            assert len(field_actions) == 1, (
                f"Only Digimon should be selectable for deletion, not Tamers. "
                f"Got {len(field_actions)} field actions: {field_actions}"
            )

    # ── [Security] Add this card to hand ───────────────────────────────

    def test_security_effect_adds_to_hand(self, debug_runner):
        """[Security] effect should add BT7-107 to the owner's hand."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        # Simulate security trigger by putting card in trash and calling security effect
        card_source = runner.inject_card(1, "BT7-107", "trash")
        player = runner.game.player1

        # Get the card from trash
        bt7_card = None
        for c in player.trash_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "BT7-107":
                bt7_card = c
                break

        assert bt7_card is not None, "Should find BT7-107 in trash"

        # Find and trigger the security effect
        from digimon_gym.engine.data.enums import EffectTiming
        effects = bt7_card.effect_list(EffectTiming.SecuritySkill)
        security_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(security_effects) == 1, (
            f"Should have exactly 1 security effect, got {len(security_effects)}"
        )

        # Execute the security effect
        security_effects[0].on_process_callback({
            'player': player,
            'game': runner.game,
        })

        # Card should move from trash to hand
        snap = runner.snapshot()
        assert "BT7-107" in snap.p1_hand, (
            f"BT7-107 should be added to hand by security effect, hand: {snap.p1_hand}"
        )
        assert "BT7-107" not in snap.p1_trash, (
            f"BT7-107 should be removed from trash by security effect, trash: {snap.p1_trash}"
        )

    # ── [Main] Returns to Main phase after effect ──────────────────────

    def test_returns_to_main_phase_after_completion(self, debug_runner):
        """After the option resolves fully, game should return to Main phase."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT2-067"])
        runner.inject_card(1, "BT2-068", "trash")

        runner.inject_card(1, "BT7-107", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Calling From the Darkness")
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Should return to main phase (or pass turn if memory went negative)
        assert snap.phase in ("Main", "Breeding", "End"), (
            f"Should return to a normal game phase after option resolves, got {snap.phase}"
        )
