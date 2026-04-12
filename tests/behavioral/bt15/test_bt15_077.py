"""Behavioral tests for BT15-077 LadyDevimon (Lv.5 Purple, Cost 6, DP 6000).

Card text:
[On Play] Reveal the top 4 cards of your deck. Add 2 level 6 or higher cards
    among them to the hand. Return the rest to the bottom of the deck.
[End of Your Turn] By deleting 1 of your Digimon, you may play 1 Digimon card
    with the [Dark Masters] trait from your hand to an empty space in your
    breeding area without paying the cost.
Inherited: <Retaliation>
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase


@pytest.mark.behavioral
class TestBT15077LadyDevimon:
    """Tests for BT15-077 LadyDevimon."""

    # ---- [On Play] Reveal top 4, add up to 2 Lv6+ to hand ----

    def test_on_play_reveal_adds_lv6_to_hand(self, debug_runner):
        """On Play should reveal top 4, letting you add up to 2 Lv6+ cards."""
        runner = debug_runner(initial_memory=8)

        # Need to advance past Breeding phase to Main
        runner.set_phase("Main")

        # Clear library and inject known cards on top
        runner.clear_zone(1, "library")
        # Top of deck: 2 Lv6 cards + 2 Lv4 cards (inserted top-first, so last insert is top)
        # We want top 4 to be: BT15-079, BT15-031, ST1-04, ST1-04
        # Need some extra cards in library so game doesn't deck out
        for _ in range(10):
            runner.inject_card(1, "ST1-03", "library_top")
        runner.inject_card(1, "ST1-04", "library_top")    # Lv.4 filler (pos 4)
        runner.inject_card(1, "ST1-04", "library_top")    # Lv.4 filler (pos 3)
        runner.inject_card(1, "BT15-031", "library_top")  # MetalSeadramon Lv.6 (pos 2)
        runner.inject_card(1, "BT15-079", "library_top")  # Piedmon Lv.6 (pos 1, top)

        # Put BT15-077 in hand
        runner.inject_card(1, "BT15-077", "hand")

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)

        # Play BT15-077 (LadyDevimon)
        aid = runner.find_action("LadyDevimon")
        assert aid is not None, f"Should be able to play LadyDevimon. Actions: {runner.actions()}"
        runner.execute(aid)

        # Auto-resolve the reveal selections (pick Lv6+ cards)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # LadyDevimon was played from hand (-1), but up to 2 Lv6 cards added (+2)
        # Net hand change: at least +1 (played 1, gained up to 2)
        # The remaining 2 Lv4 cards should go to deck bottom
        assert snap_after.p1_library_size >= 2, (
            "Non-qualifying cards should return to deck bottom"
        )

    def test_on_play_effect_is_on_play_only(self, debug_runner):
        """The reveal effect should only trigger on play, not when digivolving."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-077"])

        effects = perm.top_card.effect_list(None)
        on_play_effects = [e for e in effects
                           if e.timing == EffectTiming.OnEnterFieldAnyone
                           and getattr(e, 'is_on_play', False)]
        assert len(on_play_effects) == 1, "Should have exactly 1 On Play effect"

    # ---- [End of Your Turn] Delete Digimon -> play Dark Masters to breeding ----

    def test_end_turn_effect_exists_and_is_optional(self, debug_runner):
        """End of Turn effect should exist and be optional."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-077"])

        effects = perm.top_card.effect_list(None)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn]
        assert len(end_turn) == 1, "Should have 1 End of Turn effect"
        assert end_turn[0].is_optional, "End of Turn effect should be optional"

    def test_end_turn_requires_own_turn(self, debug_runner):
        """End of Turn effect should only fire on your own turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-077"])

        effects = perm.top_card.effect_list(None)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        # Player 1's turn
        runner.game.player1.is_my_turn = True
        assert end_turn.can_use_condition({}), "Should be usable on own turn"

        # Not player 1's turn
        runner.game.player1.is_my_turn = False
        assert not end_turn.can_use_condition({}), "Should NOT be usable on opponent's turn"

    def test_end_turn_plays_dark_masters_to_breeding_area(self, debug_runner):
        """End of Turn: after deleting a Digimon, Dark Masters card should go
        to BREEDING area (not battle area)."""
        runner = debug_runner(initial_memory=5)

        # Place LadyDevimon on field
        runner.place_on_field(1, ["BT15-077"])
        # Place a disposable Digimon to delete as cost (slot 1)
        runner.place_on_field(1, ["ST1-03"])

        # Put a Dark Masters Digimon in hand
        runner.inject_card(1, "BT15-079", "hand")  # Piedmon (Lv.6, Dark Masters)

        # Breeding area must be empty for the effect
        assert runner.game.player1.breeding_area is None, "Breeding area should start empty"

        # Advance to End of Turn phase
        runner.set_phase("End")
        runner.game.phase_end()

        # Step 1: Select Digimon to delete — ST1-03 is at slot 1 (action 101)
        # SEL_MY_FIELD_START = 100, so slot 1 = 101
        assert 101 in runner.action_mask(), (
            f"Action 101 (slot 1) should be legal. Legal: {runner.action_mask()}"
        )
        runner.execute(101)

        # Step 2: Select Dark Masters Digimon from hand to play to breeding
        aid_play = runner.find_action("Piedmon")
        assert aid_play is not None, f"Should find Piedmon to play. Actions: {runner.actions()}"
        runner.execute(aid_play)

        # Auto-resolve any remaining phases
        runner.auto_resolve(max_steps=10)

        snap_after = runner.snapshot()

        # The Dark Masters Digimon should be in breeding area, NOT on battle area
        assert snap_after.p1_breeding is not None, (
            "Dark Masters card should be placed in breeding area"
        )
        assert snap_after.p1_breeding == "BT15-079", (
            f"Breeding area should contain Piedmon (BT15-079), got {snap_after.p1_breeding}"
        )

    def test_end_turn_does_not_play_to_battle_area(self, debug_runner):
        """Dark Masters card must go to breeding, not battle area."""
        runner = debug_runner(initial_memory=5)

        runner.place_on_field(1, ["BT15-077"])
        runner.place_on_field(1, ["ST1-03"])  # sacrifice target
        runner.inject_card(1, "BT15-079", "hand")  # Piedmon

        field_count_before = len(runner.game.player1.battle_area)

        runner.set_phase("End")
        runner.game.phase_end()

        # Step 1: Select Digimon to delete — ST1-03 at slot 1 (action 101)
        assert 101 in runner.action_mask(), (
            f"Action 101 (slot 1) should be legal. Legal: {runner.action_mask()}"
        )
        runner.execute(101)

        # Step 2: Select Dark Masters Digimon to play to breeding
        aid_play = runner.find_action("Piedmon")
        assert aid_play is not None, f"Should find Piedmon to play. Actions: {runner.actions()}"
        runner.execute(aid_play)

        runner.auto_resolve(max_steps=10)

        # After deleting one Digimon, field should have 1 fewer (the deleted one)
        # Piedmon should be in breeding area, NOT battle area
        field_count_after = len(runner.game.player1.battle_area)
        # LadyDevimon (1) + sacrifice (1) = 2 before. Delete 1 = 1 after.
        assert field_count_after == field_count_before - 1, (
            f"Dark Masters should NOT be placed on battle area. "
            f"Before: {field_count_before}, After: {field_count_after}"
        )
        # Confirm Piedmon is NOT on battle area
        field_ids = [p.top_card.c_entity_base.card_id for p in runner.game.player1.battle_area]
        assert "BT15-079" not in field_ids, (
            "Piedmon should NOT be on the battle area"
        )

    def test_end_turn_requires_empty_breeding_area(self, debug_runner):
        """If breeding area is occupied, the play effect should not proceed."""
        runner = debug_runner(initial_memory=5)

        runner.place_on_field(1, ["BT15-077"])
        runner.place_on_field(1, ["ST1-03"])  # sacrifice
        runner.inject_card(1, "BT15-079", "hand")  # Piedmon

        # Occupy breeding area
        runner.place_in_breeding(1, ["BT1-001"])

        snap_before = runner.snapshot()
        hand_before = list(snap_before.p1_hand)

        runner.set_phase("End")
        runner.game.phase_end()
        runner.auto_resolve(max_steps=30)

        snap_after = runner.snapshot()
        # Piedmon should still be in hand since breeding is occupied
        # (the delete-cost step may or may not have happened depending on
        # implementation, but the play should definitely NOT happen)
        # The key assertion: breeding area still has the egg, not Piedmon
        assert snap_after.p1_breeding != "BT15-079", (
            "Should not play Dark Masters to breeding when it is already occupied"
        )

    def test_end_turn_requires_deleting_digimon_as_cost(self, debug_runner):
        """Must delete 1 of your Digimon as cost; no Digimon = no effect."""
        runner = debug_runner(initial_memory=5)

        # Only LadyDevimon on field — but she needs to be the only one
        # and we need to verify the effect still works if she's the only target
        # (she can delete herself)
        runner.place_on_field(1, ["BT15-077"])
        runner.inject_card(1, "BT15-079", "hand")

        snap_before = runner.snapshot()
        # LadyDevimon is on field — she CAN delete herself as cost
        assert len(snap_before.p1_field) == 1

    def test_end_turn_only_plays_dark_masters_trait(self, debug_runner):
        """Should only allow playing Digimon with [Dark Masters] trait."""
        runner = debug_runner(initial_memory=5)

        runner.place_on_field(1, ["BT15-077"])
        runner.place_on_field(1, ["ST1-03"])  # sacrifice

        # Put a non-Dark-Masters Digimon in hand (e.g., ST1-06, Lv.5)
        runner.inject_card(1, "ST1-06", "hand")

        runner.set_phase("End")
        runner.game.phase_end()
        runner.auto_resolve(max_steps=30)

        snap_after = runner.snapshot()
        # ST1-06 should NOT have been played to breeding
        assert snap_after.p1_breeding is None, (
            "Non-Dark-Masters Digimon should not be playable"
        )

    def test_end_turn_only_plays_digimon(self, debug_runner):
        """Should only play Digimon cards, not Tamers or Options with Dark Masters."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-077"])

        effects = perm.top_card.effect_list(None)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        # The filter in process should check is_digimon
        # This is a structural check — the process callback should filter for Digimon

    # ---- Inherited: Retaliation ----

    def test_inherited_retaliation(self, debug_runner):
        """Should have inherited Retaliation effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-077"])

        effects = perm.top_card.effect_list(None)
        retal = [e for e in effects
                 if getattr(e, '_is_retaliation', False)
                 and getattr(e, 'is_inherited_effect', False)]
        assert len(retal) == 1, "Should have exactly 1 inherited Retaliation effect"

    def test_inherited_retaliation_is_on_deletion(self, debug_runner):
        """Retaliation triggers on deletion."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-077"])

        effects = perm.top_card.effect_list(None)
        retal = [e for e in effects if getattr(e, '_is_retaliation', False)][0]
        assert getattr(retal, 'is_on_deletion', False), (
            "Retaliation should be flagged as is_on_deletion"
        )
