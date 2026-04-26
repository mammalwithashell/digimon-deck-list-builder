"""Behavioral tests for BT8-071 Psychemon (Lv.3, Purple, Reptile, Cost 3).

Card text:
  [All Turns] Players can't reduce play costs.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT8071CannotReduceCost:
    """[All Turns] Players can't reduce play costs."""

    def test_has_declarative_blocks_cost_reduction_effect(self, debug_runner):
        """Script should expose a declarative NoTiming effect that blocks cost reduction."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT8-071")
        effects = cs.effect_list(None)
        declarative = [e for e in effects if getattr(e, 'is_declarative', False)]
        assert len(declarative) >= 1, "Should have a declarative effect"
        blockers = [e for e in effects if getattr(e, '_blocks_cost_reduction', False)]
        assert len(blockers) >= 1, "Should have a _blocks_cost_reduction effect"
        assert blockers[0].timing == EffectTiming.NoTiming, (
            "Declarative cost-reduction blocker should use NoTiming"
        )

    def test_blocks_own_cost_reduction(self, debug_runner):
        """Psychemon on P1 field must prevent P1's cost reduction (BT8-097)."""
        runner = debug_runner(initial_memory=0)

        # Psychemon on P1 field
        runner.place_on_field(1, ["BT8-071"])

        # Opponent has 2 Digimon, so BT8-097 Crimson Blaze (cost 6) would
        # normally be reduced by 2 to cost 4.
        runner.place_on_field(2, ["ST1-03"])
        runner.place_on_field(2, ["ST1-03"])
        runner.inject_card(1, "BT8-097", "hand")
        runner.set_phase("Main")

        game = runner.game
        p1 = game.player1
        crimson = [c for c in p1.hand_cards
                   if c.c_entity_base and c.c_entity_base.card_id == "BT8-097"][0]
        cost = game.calculate_play_cost(p1, crimson)
        # With Psychemon on field, reduction is blocked; cost must remain 6.
        assert cost == 6, (
            f"With Psychemon blocking cost reduction, expected cost 6, got {cost}"
        )

    def test_blocks_opponent_cost_reduction(self, debug_runner):
        """Psychemon on P1 field must also block P2 cost reduction."""
        runner = debug_runner(initial_memory=0)

        # Psychemon on P1's field
        runner.place_on_field(1, ["BT8-071"])

        # P1 has 2 Digimon -> P2's BT8-097 would normally be reduced to 4
        runner.place_on_field(1, ["ST1-03"])
        runner.place_on_field(1, ["ST1-03"])
        runner.inject_card(2, "BT8-097", "hand")

        game = runner.game
        p2 = game.player2
        crimson = [c for c in p2.hand_cards
                   if c.c_entity_base and c.c_entity_base.card_id == "BT8-097"][0]
        cost = game.calculate_play_cost(p2, crimson)
        assert cost == 6, (
            f"Psychemon must block opponent's cost reduction too, "
            f"expected 6, got {cost}"
        )

    def test_cost_reduction_works_without_psychemon(self, debug_runner):
        """Baseline sanity: without Psychemon, BT8-097 cost is reduced normally."""
        runner = debug_runner(initial_memory=0)

        # No Psychemon
        runner.place_on_field(2, ["ST1-03"])
        runner.place_on_field(2, ["ST1-03"])
        runner.inject_card(1, "BT8-097", "hand")
        runner.set_phase("Main")

        game = runner.game
        p1 = game.player1
        crimson = [c for c in p1.hand_cards
                   if c.c_entity_base and c.c_entity_base.card_id == "BT8-097"][0]
        cost = game.calculate_play_cost(p1, crimson)
        # BT8-097 base cost 6, reduced by 2 for 2 opponent Digimon -> 4
        assert cost == 4, (
            f"Without Psychemon, BT8-097 cost should reduce to 4, got {cost}"
        )

    def test_effect_removed_when_psychemon_leaves_field(self, debug_runner):
        """Once Psychemon is deleted, cost reduction resumes."""
        runner = debug_runner(initial_memory=0)

        psyche_perm = runner.place_on_field(1, ["BT8-071"])
        runner.place_on_field(2, ["ST1-03"])
        runner.place_on_field(2, ["ST1-03"])
        runner.inject_card(1, "BT8-097", "hand")
        runner.set_phase("Main")

        game = runner.game
        p1 = game.player1
        crimson = [c for c in p1.hand_cards
                   if c.c_entity_base and c.c_entity_base.card_id == "BT8-097"][0]

        cost_with = game.calculate_play_cost(p1, crimson)
        assert cost_with == 6, f"With Psychemon: expected 6, got {cost_with}"

        # Delete Psychemon
        p1.delete_permanent(psyche_perm)

        cost_without = game.calculate_play_cost(p1, crimson)
        assert cost_without == 4, (
            f"After Psychemon deleted, cost should be 4, got {cost_without}"
        )

    def test_condition_false_when_not_on_field(self, debug_runner):
        """Field-presence gate: effect condition returns False from hand."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT8-071")
        effects = cs.effect_list(None)
        blockers = [e for e in effects if getattr(e, '_blocks_cost_reduction', False)]
        assert len(blockers) >= 1
        # CardSource with no permanent_of_this_card -> condition False
        assert blockers[0].can_use_condition({}) is False, (
            "Blocker condition should be False while card is not on field"
        )

    def test_applies_on_both_turns(self, debug_runner):
        """[All Turns] -> effect works regardless of whose turn it is."""
        runner = debug_runner(initial_memory=0, first_player=2)

        # Psychemon on P1 (non-turn-player)
        runner.place_on_field(1, ["BT8-071"])
        runner.place_on_field(1, ["ST1-03"])
        runner.place_on_field(1, ["ST1-03"])
        runner.inject_card(2, "BT8-097", "hand")

        game = runner.game
        p2 = game.player2
        crimson = [c for c in p2.hand_cards
                   if c.c_entity_base and c.c_entity_base.card_id == "BT8-097"][0]
        # P2 is turn player; P1 owns Psychemon. Effect must still apply.
        cost = game.calculate_play_cost(p2, crimson)
        assert cost == 6, (
            f"[All Turns] effect should apply on opponent's turn too, got {cost}"
        )
