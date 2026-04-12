"""Behavioral tests for BT15-102 Apocalymon | Lv.7 | White | DP:15000 | Cost:15.

Card text:
When this card would be played, by placing up to 3 [Dark Masters] trait cards
with different names from your battle area or trash under it, reduce the play
cost by 4 for each one.

[End of Your Turn] [Once Per Turn] By placing 1 level 6 or lower card from
your trash as this Digimon's bottom digivolution card, activate 1 [On Play]
effect on that card as an effect of this Digimon. Then, trash the top 2 cards
of your opponent's deck for each of this Digimon's level 6 digivolution cards.

Key implementation notes:
- Cost reduction: sources from battle area OR trash (not just trash)
- "cards" not "Digimon cards" — any card type with [Dark Masters] trait
- Cards placed under it become digivolution sources (matters for mill count)
- End of Turn: "level 6 or lower card" (any card type), placed as bottom digi
- Mill uses player.mill() for proper event flow (fires OnDiscardLibrary)
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase


# Dark Masters trait cards for testing:
# BT15-031: MetalSeadramon (Lv.6 Blue Digimon, Dark Masters)
# BT15-052: Puppetmon (Lv.6 Green Digimon, Dark Masters)
# BT15-066: Machinedramon (Lv.6 Black Digimon, Dark Masters)
# BT15-079: Piedmon (Lv.6 Purple Digimon, Dark Masters)
# EX10-072: Spiral Mountain (Option, Dark Masters trait)


def _pass_and_select_trash(runner, trash_action_id=130):
    """Helper: pass turn, then select a specific trash card in the EoT selection."""
    end_action = runner.find_action("Pass")
    assert end_action is not None, "Should find Pass action"
    runner.execute(end_action)
    # Now in SelectTrash phase for EoT effect
    assert runner.game.current_phase == GamePhase.SelectTrash, (
        f"Expected SelectTrash phase, got {runner.game.current_phase}"
    )
    runner.execute(trash_action_id)
    runner.auto_resolve()


@pytest.mark.behavioral
class TestBT15102Apocalymon:
    """Tests for BT15-102 Apocalymon."""

    # ---- Cost Reduction: Basic ------------------------------------------------

    def test_cost_reduction_with_1_dm_in_trash(self, debug_runner):
        """Playing with 1 DM card in trash reduces cost by 4."""
        runner = debug_runner(initial_memory=15)
        runner.clear_zone(1, "field")
        runner.clear_zone(1, "trash")

        runner.inject_card(1, "BT15-031", "trash")  # MetalSeadramon
        runner.inject_card(1, "BT15-102", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Apocalymon")
        assert action is not None, "Should be able to play Apocalymon"

        result = runner.execute(action)
        runner.auto_resolve()

        # Cost 15 - 4 = 11
        actual_cost = result.before.memory - result.after.memory
        assert actual_cost == 11, (
            f"With 1 DM card, cost should be 15-4=11, but spent {actual_cost}"
        )

    def test_cost_reduction_with_3_dm_in_trash(self, debug_runner):
        """Playing with 3 distinct DM cards in trash reduces cost by 12."""
        runner = debug_runner(initial_memory=15)
        runner.clear_zone(1, "field")
        runner.clear_zone(1, "trash")

        runner.inject_card(1, "BT15-031", "trash")  # MetalSeadramon
        runner.inject_card(1, "BT15-052", "trash")  # Puppetmon
        runner.inject_card(1, "BT15-066", "trash")  # Machinedramon
        runner.inject_card(1, "BT15-102", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Apocalymon")
        assert action is not None

        result = runner.execute(action)
        runner.auto_resolve()

        # Cost 15 - 12 = 3
        actual_cost = result.before.memory - result.after.memory
        assert actual_cost == 3, (
            f"With 3 DM cards, cost should be 15-12=3, but spent {actual_cost}"
        )

    def test_cost_reduction_duplicate_names_only_count_once(self, debug_runner):
        """Two cards with the same name only provide 1 reduction slot."""
        runner = debug_runner(initial_memory=15)
        runner.clear_zone(1, "field")
        runner.clear_zone(1, "trash")

        # Two copies of MetalSeadramon (same name)
        runner.inject_card(1, "BT15-031", "trash")
        runner.inject_card(1, "BT15-031", "trash")
        runner.inject_card(1, "BT15-102", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Apocalymon")
        assert action is not None

        result = runner.execute(action)
        runner.auto_resolve()

        # Only 1 distinct name -> reduction of 4
        actual_cost = result.before.memory - result.after.memory
        assert actual_cost == 11, (
            f"With 2 copies of same DM name, should only reduce by 4, but spent {actual_cost}"
        )

    # ---- Cost Reduction: Battle Area Source ----------------------------------

    def test_cost_reduction_from_battle_area(self, debug_runner):
        """DM cards on battle area count as a source for cost reduction."""
        runner = debug_runner(initial_memory=15)
        runner.clear_zone(1, "field")
        runner.clear_zone(1, "trash")

        # Place a DM Digimon on the battle area
        runner.place_on_field(1, ["BT15-031"])  # MetalSeadramon on field
        runner.inject_card(1, "BT15-102", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Apocalymon")
        assert action is not None, "Should be able to play Apocalymon"

        result = runner.execute(action)
        runner.auto_resolve()

        # Cost 15 - 4 = 11 (1 DM from battle area)
        actual_cost = result.before.memory - result.after.memory
        assert actual_cost == 11, (
            f"With 1 DM on field, cost should be 15-4=11, but spent {actual_cost}"
        )

    def test_cost_reduction_from_battle_area_and_trash_combined(self, debug_runner):
        """Can use DM cards from both battle area and trash."""
        runner = debug_runner(initial_memory=15)
        runner.clear_zone(1, "field")
        runner.clear_zone(1, "trash")

        # 1 on field, 2 in trash = 3 distinct names
        runner.place_on_field(1, ["BT15-031"])   # MetalSeadramon on field
        runner.inject_card(1, "BT15-052", "trash")  # Puppetmon in trash
        runner.inject_card(1, "BT15-066", "trash")  # Machinedramon in trash
        runner.inject_card(1, "BT15-102", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Apocalymon")
        assert action is not None

        result = runner.execute(action)
        runner.auto_resolve()

        # Cost 15 - 12 = 3 (3 distinct DM from field+trash)
        actual_cost = result.before.memory - result.after.memory
        assert actual_cost == 3, (
            f"With 3 DM (1 field + 2 trash), cost should be 15-12=3, but spent {actual_cost}"
        )

    # ---- Cost Reduction: Card Type Filter ------------------------------------

    def test_cost_reduction_accepts_non_digimon_dm_cards(self, debug_runner):
        """Option cards with [Dark Masters] trait also count for cost reduction.

        EX10-072 Spiral Mountain is an Option card with the Dark Masters trait.
        Card text says 'cards' not 'Digimon cards'.
        """
        runner = debug_runner(initial_memory=15)
        runner.clear_zone(1, "field")
        runner.clear_zone(1, "trash")

        runner.inject_card(1, "EX10-072", "trash")  # Spiral Mountain (Option, DM trait)
        runner.inject_card(1, "BT15-031", "trash")   # MetalSeadramon (Digimon, DM)
        runner.inject_card(1, "BT15-102", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Apocalymon")
        assert action is not None

        result = runner.execute(action)
        runner.auto_resolve()

        # 2 DM cards (1 Option + 1 Digimon) = reduction of 8
        actual_cost = result.before.memory - result.after.memory
        assert actual_cost == 7, (
            f"Option card with DM trait should count; cost should be 15-8=7, but spent {actual_cost}"
        )

    # ---- Cost Reduction: Cards Placed Under ----------------------------------

    def test_dm_cards_placed_as_digivolution_sources(self, debug_runner):
        """DM cards used for cost reduction should become digivolution sources."""
        runner = debug_runner(initial_memory=15)
        runner.clear_zone(1, "field")
        runner.clear_zone(1, "trash")

        runner.inject_card(1, "BT15-031", "trash")  # MetalSeadramon
        runner.inject_card(1, "BT15-052", "trash")  # Puppetmon
        runner.inject_card(1, "BT15-102", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Apocalymon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        apoc = [s for s in snap.p1_field if s.card_id == "BT15-102"]
        assert len(apoc) == 1, "Apocalymon should be on field"

        # The stack should contain Apocalymon + the 2 DM cards placed under
        assert len(apoc[0].stack_ids) == 3, (
            f"Stack should have 3 cards (Apocalymon + 2 DM), got {apoc[0].stack_ids}"
        )
        assert "BT15-031" in apoc[0].stack_ids, "MetalSeadramon should be in stack"
        assert "BT15-052" in apoc[0].stack_ids, "Puppetmon should be in stack"

    def test_dm_cards_removed_from_trash_after_placement(self, debug_runner):
        """DM cards placed under Apocalymon should no longer be in trash."""
        runner = debug_runner(initial_memory=15)
        runner.clear_zone(1, "field")
        runner.clear_zone(1, "trash")

        runner.inject_card(1, "BT15-031", "trash")
        runner.inject_card(1, "BT15-102", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Apocalymon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert "BT15-031" not in snap.p1_trash, (
            "MetalSeadramon should be removed from trash (placed under Apocalymon)"
        )

    def test_dm_from_field_removed_after_placement(self, debug_runner):
        """DM cards from battle area should be removed from field after placement."""
        runner = debug_runner(initial_memory=15)
        runner.clear_zone(1, "field")
        runner.clear_zone(1, "trash")

        runner.place_on_field(1, ["BT15-031"])  # MetalSeadramon on field
        runner.inject_card(1, "BT15-102", "hand")
        runner.set_phase("Main")

        snap_before = runner.snapshot()
        metal_on_field_before = [s for s in snap_before.p1_field if s.card_id == "BT15-031"]
        assert len(metal_on_field_before) == 1, "MetalSeadramon should be on field before play"

        action = runner.find_action("Play Apocalymon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        metal_on_field = [s for s in snap.p1_field if s.card_id == "BT15-031"]
        assert len(metal_on_field) == 0, (
            "MetalSeadramon should be removed from field (placed under Apocalymon)"
        )

    # ---- Cost Reduction: No Leak From Field ----------------------------------

    def test_cost_reduction_no_leak_from_field(self, debug_runner):
        """BT15-102 on field should NOT reduce play cost of other cards."""
        runner = debug_runner(initial_memory=15)

        # Place Apocalymon on field
        runner.place_on_field(1, ["BT15-102"])

        # Put some DM in trash
        runner.inject_card(1, "BT15-031", "trash")

        # Play a different card - Apocalymon on field should not reduce its cost
        runner.inject_card(1, "ST1-03", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Agumon")
        if action is not None:
            result = runner.execute(action)
            actual_cost = result.before.memory - result.after.memory
            assert actual_cost == 3, (
                f"Apocalymon on field should not reduce other cards' cost, "
                f"expected 3 but spent {actual_cost}"
            )

    # ---- End of Turn: Basic Flow ---------------------------------------------

    def test_end_of_turn_places_card_from_trash(self, debug_runner):
        """[End of Turn] Places 1 Lv.6- card from trash as bottom digi card."""
        runner = debug_runner(initial_memory=0)

        # Place Apocalymon on field
        runner.place_on_field(1, ["BT15-102"])

        # Put a Lv.3 card in trash
        runner.inject_card(1, "ST1-03", "trash")  # Agumon Lv.3

        # Move to End phase and select the trash card
        runner.set_phase("Main")
        _pass_and_select_trash(runner, 130)

        snap = runner.snapshot()
        apoc = [s for s in snap.p1_field if s.card_id == "BT15-102"]
        assert len(apoc) == 1, "Apocalymon should be on field"
        # Agumon should be in the stack now
        assert "ST1-03" in apoc[0].stack_ids, (
            f"Trash card should be placed as bottom digivolution card, got {apoc[0].stack_ids}"
        )

    def test_end_of_turn_once_per_turn(self, debug_runner):
        """[End of Turn] effect is Once Per Turn."""
        runner = debug_runner(initial_memory=0)

        perm = runner.place_on_field(1, ["BT15-102"])
        top = perm.top_card

        effects = top.effect_list(None)
        eot_effects = [e for e in effects
                       if e.timing == EffectTiming.OnEndTurn
                       and e.max_count_per_turn == 1]
        assert len(eot_effects) >= 1, "Should have a Once Per Turn End of Turn effect"

    def test_end_of_turn_optional(self, debug_runner):
        """[End of Turn] effect is optional (uses 'By placing')."""
        runner = debug_runner(initial_memory=0)

        perm = runner.place_on_field(1, ["BT15-102"])
        top = perm.top_card

        effects = top.effect_list(None)
        eot_effects = [e for e in effects if e.timing == EffectTiming.OnEndTurn]
        assert len(eot_effects) >= 1
        assert eot_effects[0].is_optional, "End of Turn effect should be optional"

    # ---- End of Turn: Mill ---------------------------------------------------

    def test_end_of_turn_mills_per_lv6_digi_cards(self, debug_runner):
        """After placing card, mill 2 per Lv.6 digivolution card."""
        runner = debug_runner(initial_memory=0)

        # Place Apocalymon with 2 Lv.6 DM cards as digi sources
        perm = runner.place_on_field(1, ["BT15-102"])
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs1 = db.create_card_source("BT15-031", runner.game.player1)  # Lv.6
        cs2 = db.create_card_source("BT15-052", runner.game.player1)  # Lv.6
        perm.add_card_source_bottom(cs1)
        perm.add_card_source_bottom(cs2)

        # Put a Lv.3 card in trash for the EoT placement
        runner.inject_card(1, "ST1-03", "trash")

        opp_deck_before = len(runner.game.player2.library_cards)

        # Move to End phase and select the trash card
        runner.set_phase("Main")
        _pass_and_select_trash(runner, 130)

        snap = runner.snapshot()
        # 2 Lv.6 digi cards * 2 = 4 cards milled from opponent deck
        opp_deck_after = snap.p2_library_size
        cards_milled = opp_deck_before - opp_deck_after

        assert cards_milled == 4, (
            f"Should mill 2*2=4 cards (2 Lv.6 digi cards), "
            f"but deck went from {opp_deck_before} to {opp_deck_after} (milled {cards_milled})"
        )

    def test_end_of_turn_mill_counts_only_lv6(self, debug_runner):
        """Mill count only counts exactly Lv.6 digi cards, not other levels."""
        runner = debug_runner(initial_memory=0)

        # Place Apocalymon with 1 Lv.6 and 1 Lv.4 card
        perm = runner.place_on_field(1, ["BT15-102"])
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs1 = db.create_card_source("BT15-031", runner.game.player1)  # Lv.6
        cs2 = db.create_card_source("ST1-07", runner.game.player1)    # Lv.4
        perm.add_card_source_bottom(cs1)
        perm.add_card_source_bottom(cs2)

        runner.inject_card(1, "ST1-03", "trash")

        opp_deck_before = len(runner.game.player2.library_cards)

        runner.set_phase("Main")
        _pass_and_select_trash(runner, 130)

        snap = runner.snapshot()
        cards_milled = opp_deck_before - snap.p2_library_size

        # Only 1 Lv.6 card -> mill 2
        assert cards_milled == 2, (
            f"Should mill 2*1=2 cards (only 1 Lv.6 digi card), but milled {cards_milled}"
        )

    def test_end_of_turn_no_lv6_no_mill(self, debug_runner):
        """If no Lv.6 digi cards, no milling occurs."""
        runner = debug_runner(initial_memory=0)

        # Place Apocalymon with no digi sources
        perm = runner.place_on_field(1, ["BT15-102"])

        runner.inject_card(1, "ST1-03", "trash")  # Lv.3

        opp_deck_before = len(runner.game.player2.library_cards)

        # Pass and select trash card
        runner.set_phase("Main")
        _pass_and_select_trash(runner, 130)

        snap = runner.snapshot()
        cards_milled = opp_deck_before - snap.p2_library_size

        # No Lv.6 digi cards -> 0 mill (Agumon placed is Lv.3)
        assert cards_milled == 0, (
            f"Should mill 0 cards (no Lv.6 digi cards), but milled {cards_milled}"
        )

    # ---- End of Turn: Card from trash only -----------------------------------

    def test_end_of_turn_no_valid_trash_card_no_trigger(self, debug_runner):
        """If no Lv.6- cards in trash, the effect cannot trigger."""
        runner = debug_runner(initial_memory=0)

        perm = runner.place_on_field(1, ["BT15-102"])
        runner.clear_zone(1, "trash")

        # Only a Lv.7 card in trash (not Lv.6 or lower)
        runner.inject_card(1, "BT15-102", "trash")  # Lv.7

        top = perm.top_card
        effects = top.effect_list(None)
        eot_effects = [e for e in effects if e.timing == EffectTiming.OnEndTurn]

        if eot_effects:
            ctx = {'game': runner.game, 'player': runner.game.player1,
                   'permanent': perm}
            # Condition should fail when no valid trash targets exist
            result = eot_effects[0].can_use_condition(ctx)
            assert not result, (
                "End of Turn condition should fail when no Lv.6- cards in trash"
            )

    # ---- Cost Reduction: Max 3 Cap -------------------------------------------

    def test_cost_reduction_capped_at_3(self, debug_runner):
        """Even with 4+ distinct DM cards, only up to 3 can be placed."""
        runner = debug_runner(initial_memory=15)
        runner.clear_zone(1, "field")
        runner.clear_zone(1, "trash")

        # 4 distinct DM names
        runner.inject_card(1, "BT15-031", "trash")  # MetalSeadramon
        runner.inject_card(1, "BT15-052", "trash")  # Puppetmon
        runner.inject_card(1, "BT15-066", "trash")  # Machinedramon
        runner.inject_card(1, "BT15-079", "trash")  # Piedmon
        runner.inject_card(1, "BT15-102", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Apocalymon")
        assert action is not None

        result = runner.execute(action)
        runner.auto_resolve()

        # Max 3 -> reduction of 12, cost = 15 - 12 = 3
        actual_cost = result.before.memory - result.after.memory
        assert actual_cost == 3, (
            f"Cost reduction capped at 3 placements (12), should be 15-12=3, but spent {actual_cost}"
        )

    # ---- No Cost Reduction Without DM ----------------------------------------

    def test_no_cost_reduction_without_dm_cards(self, debug_runner):
        """Full cost when no DM cards available."""
        runner = debug_runner(initial_memory=15)
        runner.clear_zone(1, "field")
        runner.clear_zone(1, "trash")

        runner.inject_card(1, "BT15-102", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Apocalymon")
        assert action is not None

        result = runner.execute(action)
        runner.auto_resolve()

        actual_cost = result.before.memory - result.after.memory
        assert actual_cost == 15, (
            f"With no DM cards, cost should be full 15, but spent {actual_cost}"
        )
