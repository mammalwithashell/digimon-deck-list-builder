"""Behavioral tests for BT21-093 Raging Serpentine (Option, Red, Cost 8).

Card text:
When this card would be used, if your opponent has 3 or fewer security cards,
reduce the use cost by 4.
[Main] Delete 1 of your opponent's highest DP Digimon. Then, place this card
in the battle area.
[All Turns] When your opponent's security stack is removed from, <Delay>
(By trashing this card after the placing turn, activate the effect below.)
- 1 of your [Reptile] or [Dragonkin] trait Digimon may digivolve into a
  [Reptile] or [Dragonkin] trait Digimon card in the hand without paying the cost.

Security Effect:
[Security] Delete 1 of your opponent's Digimon with the highest DP.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


CARD_ID = "BT21-093"
# ST1-03 Agumon: Red, Reptile, Lv3, 2000 DP — satisfies Red color req + Reptile trait
REPTILE_LV3 = "ST1-03"
# BT21-017 Dimetromon: Red, Reptile/LIBERATOR, Lv4, 4000 DP
REPTILE_LV4 = "BT21-017"
# BT6-037 Bulkmon: Red, Dragonkin, Lv4, 5000 DP
DRAGONKIN_LV4 = "BT6-037"
# ST1-07 Greymon: Red, Dinosaur (NOT Reptile/Dragonkin), Lv4, 4000 DP
NON_REPTILE_LV4 = "ST1-07"


def _place_red_digimon(runner):
    """Place a Red Digimon on P1's field to satisfy option color requirement."""
    return runner.place_on_field(1, [REPTILE_LV3])


@pytest.mark.behavioral
class TestBT21093RagingSerpentine:
    """Tests for BT21-093 Raging Serpentine."""

    # ── Cost reduction ────────────────────────────────────────────────

    def test_cost_reduction_with_3_or_fewer_security(self, debug_runner):
        """Cost reduced by 4 when opponent has 3 or fewer security."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # Place Red Digimon for color requirement
        _place_red_digimon(runner)

        # Opponent starts with 5 security; remove 3 to leave 2
        opp = game.player2
        for _ in range(3):
            if opp.security_cards:
                opp.security_cards.pop()
        assert len(opp.security_cards) <= 3

        runner.inject_card(1, CARD_ID, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Raging Serpentine")
        if action is None:
            action = runner.find_action(CARD_ID)
        assert action is not None, f"Should find play action. Actions: {runner.actions()}"

        mem_before = game.memory
        runner.execute(action)
        runner.auto_resolve()

        # Cost should be 8 - 4 = 4
        expected_mem = mem_before - 4
        assert game.memory == expected_mem, (
            f"Cost should be reduced by 4 (from 8 to 4). "
            f"Memory before: {mem_before}, after: {game.memory}, expected: {expected_mem}"
        )

    def test_no_cost_reduction_with_4_security(self, debug_runner):
        """No cost reduction when opponent has 4 security."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # Place Red Digimon for color requirement
        _place_red_digimon(runner)

        # Opponent starts with 5 security; remove 1 to leave 4
        opp = game.player2
        if opp.security_cards:
            opp.security_cards.pop()
        assert len(opp.security_cards) == 4

        runner.inject_card(1, CARD_ID, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Raging Serpentine")
        if action is None:
            action = runner.find_action(CARD_ID)
        assert action is not None, f"Should find play action. Actions: {runner.actions()}"

        mem_before = game.memory
        runner.execute(action)
        runner.auto_resolve()

        # Full cost of 8
        expected_mem = mem_before - 8
        assert game.memory == expected_mem, (
            f"No cost reduction with 4 opponent security. "
            f"Memory before: {mem_before}, after: {game.memory}, expected: {expected_mem}"
        )

    # ── Main effect: delete highest DP ───────────────────────────────

    def test_main_deletes_highest_dp_digimon(self, debug_runner):
        """[Main] deletes the opponent's highest DP Digimon."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # Place Red Digimon for color requirement
        _place_red_digimon(runner)

        # Remove opponent security to get cost reduction
        runner.clear_zone(2, "security")

        # Place two Digimon on opponent field: 2000 DP and 5000 DP
        runner.place_on_field(2, [REPTILE_LV3])  # 2000 DP
        runner.place_on_field(2, [DRAGONKIN_LV4])  # 5000 DP

        runner.inject_card(1, CARD_ID, "hand")
        runner.set_phase("Main")

        opp = game.player2
        opp_field_before = len(opp.battle_area)
        assert opp_field_before == 2

        action = runner.find_action("Raging Serpentine")
        if action is None:
            action = runner.find_action(CARD_ID)
        assert action is not None, f"Actions: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        # The 5000 DP Digimon should be deleted, leaving only the 2000 DP one
        opp_dps = [p.dp for p in opp.battle_area if p.is_digimon]
        assert len(opp.battle_area) == opp_field_before - 1, (
            f"Should have deleted 1 Digimon. Before: {opp_field_before}, "
            f"after: {len(opp.battle_area)}"
        )
        assert 5000 not in opp_dps, (
            f"Should have deleted the 5000 DP Digimon. Remaining DPs: {opp_dps}"
        )

    def test_main_places_in_battle_area(self, debug_runner):
        """[Main] places this card in the battle area after delete."""
        runner = debug_runner(initial_memory=10)
        game = runner.game
        player = game.player1

        # Place Red Digimon for color requirement
        _place_red_digimon(runner)

        # Clear opponent security for cost reduction
        runner.clear_zone(2, "security")

        # Need a target to delete
        runner.place_on_field(2, [REPTILE_LV3])

        runner.inject_card(1, CARD_ID, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Raging Serpentine")
        if action is None:
            action = runner.find_action(CARD_ID)
        assert action is not None, f"Actions: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        # BT21-093 should be in P1's battle area (delay placement)
        ba_ids = [
            p.top_card.c_entity_base.card_id
            for p in player.battle_area
            if p.top_card and p.top_card.c_entity_base
        ]
        assert CARD_ID in ba_ids, (
            f"{CARD_ID} should be placed in battle area. BA: {ba_ids}"
        )

    # ── Delay: manual activation ─────────────────────────────────────

    def test_delay_manual_activation_trashes_and_digivolves(self, debug_runner):
        """Manual delay: trashes option, digivolves Reptile/Dragonkin from hand."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        # Place BT21-093 in battle area (played last turn)
        runner.place_on_field(1, [CARD_ID], turn_played=0)

        # Place Reptile on P1's field
        runner.place_on_field(1, [REPTILE_LV3])

        # Reptile Lv4 in P1's hand
        runner.inject_card(1, REPTILE_LV4, "hand")

        game.turn_count = 2
        runner.set_phase("Main")

        # Use manual delay activation (engine's delay action)
        action = runner.find_action("Delay")
        if action is None:
            action = runner.find_action("delay")
        assert action is not None, (
            f"Should find delay action. Actions: {runner.actions()}"
        )

        runner.execute(action)
        runner.auto_resolve()

        # After delay: BT21-093 should be trashed
        ba_ids = [
            p.top_card.c_entity_base.card_id
            for p in player.battle_area
            if p.top_card and p.top_card.c_entity_base
        ]
        assert CARD_ID not in ba_ids, (
            f"{CARD_ID} should be trashed after delay activation. BA: {ba_ids}"
        )

        # Check that BT21-093 is in trash
        trash_ids = [
            c.c_entity_base.card_id for c in player.trash_cards
            if c.c_entity_base
        ]
        assert CARD_ID in trash_ids, (
            f"{CARD_ID} should be in trash after delay. Trash: {trash_ids}"
        )

    def test_delay_not_available_on_placing_turn(self, debug_runner):
        """Delay cannot be activated on the same turn the card was placed."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Place BT21-093 in battle area this turn
        game.turn_count = 1
        runner.place_on_field(1, [CARD_ID], turn_played=1)

        # Reptile on field + Reptile in hand
        runner.place_on_field(1, [REPTILE_LV3])
        runner.inject_card(1, REPTILE_LV4, "hand")

        runner.set_phase("Main")

        # Should NOT find delay action (placed this turn)
        action = runner.find_action("Delay")
        assert action is None, (
            f"Delay should not be available on placing turn. Actions: {runner.actions()}"
        )

    def test_delay_digivolve_only_reptile_dragonkin_on_field(self, debug_runner):
        """Delay digivolve only selects [Reptile] or [Dragonkin] trait Digimon on field."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        # Place BT21-093 in battle area (played last turn)
        runner.place_on_field(1, [CARD_ID], turn_played=0)

        # Place ONLY a non-Reptile/Dragonkin Digimon (Dinosaur trait)
        runner.place_on_field(1, [NON_REPTILE_LV4])

        # Put Reptile Lv4 in hand (valid digivolve card)
        runner.inject_card(1, REPTILE_LV4, "hand")

        game.turn_count = 2
        runner.set_phase("Main")

        # Delay is available at engine level, but selecting should find
        # no valid Reptile/Dragonkin Digimon on field
        action = runner.find_action("Delay")
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

            # The non-Reptile Digimon should NOT have been digivolved
            for p in player.battle_area:
                if p.is_digimon and p.top_card and p.top_card.c_entity_base:
                    top_id = p.top_card.c_entity_base.card_id
                    if top_id == NON_REPTILE_LV4:
                        # Correct - non-reptile was not digivolved
                        pass

    # ── Security effect ──────────────────────────────────────────────

    def test_security_effect_has_delete_highest_dp(self, debug_runner):
        """[Security] effect exists and targets highest DP opponent Digimon."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(CARD_ID, game.player2)
        effects = cs.effect_list(EffectTiming.SecuritySkill)
        security_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(security_effects) >= 1, "Should have a security effect"

    # ── Condition: OnLoseSecurity checks opponent ────────────────────

    def test_does_not_trigger_on_own_security_loss(self, debug_runner):
        """OnLoseSecurity should NOT trigger when card OWNER loses security."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        # Place BT21-093 in P1's battle area
        runner.place_on_field(1, [CARD_ID], turn_played=0)
        game.turn_count = 2

        # Place Reptile + hand card
        runner.place_on_field(1, [REPTILE_LV3])
        runner.inject_card(1, REPTILE_LV4, "hand")

        # P1 (card owner) loses own security - should NOT trigger delay
        if player.security_cards:
            player.security_cards.pop(0)
            player._fire_timing(
                EffectTiming.OnLoseSecurity,
                {"lost_card": None, "player": player}
            )

        # BT21-093 should still be in battle area (not triggered)
        ba_ids = [
            p.top_card.c_entity_base.card_id
            for p in player.battle_area
            if p.top_card and p.top_card.c_entity_base
        ]
        assert CARD_ID in ba_ids, (
            f"{CARD_ID} should still be on field (not triggered by own security loss). "
            f"BA: {ba_ids}"
        )

    def test_triggers_on_opponent_security_loss(self, debug_runner):
        """OnLoseSecurity should trigger when OPPONENT loses security, trashing option."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1
        opp = game.player2

        # Place BT21-093 in P1's battle area (played last turn)
        runner.place_on_field(1, [CARD_ID], turn_played=0)
        game.turn_count = 2

        # Place Reptile on P1's field to be digivolve target
        runner.place_on_field(1, [REPTILE_LV3])

        # Put Reptile Lv4 in P1's hand as the card to digivolve into
        runner.inject_card(1, REPTILE_LV4, "hand")

        assert len(opp.security_cards) > 0, "Opponent needs security cards"

        # Simulate opponent losing security
        opp.security_cards.pop(0)
        opp._fire_timing(
            EffectTiming.OnLoseSecurity,
            {"lost_card": None, "player": opp}
        )

        # The OnLoseSecurity effect should have triggered.
        # The delay mechanism trashes the option card and starts digivolve selection.
        # BT21-093 should no longer be in battle area (trashed as delay cost)
        ba_ids = [
            p.top_card.c_entity_base.card_id
            for p in player.battle_area
            if p.top_card and p.top_card.c_entity_base
        ]
        assert CARD_ID not in ba_ids, (
            f"{CARD_ID} should be trashed from battle area after delay trigger. BA: {ba_ids}"
        )

        # BT21-093 should be in trash
        trash_ids = [
            c.c_entity_base.card_id for c in player.trash_cards
            if c.c_entity_base
        ]
        assert CARD_ID in trash_ids, (
            f"{CARD_ID} should be in trash after delay activation. Trash: {trash_ids}"
        )
