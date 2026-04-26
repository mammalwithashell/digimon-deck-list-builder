"""Behavioral tests for ST13-08 Chikurimon (Lv.3, Black, Machine, Cost 3).

Card text:
  [All Turns] Players can't reduce play costs.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming
from digimon_gym.engine.interfaces.modifiers import ModifierType


@pytest.mark.behavioral
class TestST1308CannotReduceCost:
    """[All Turns] Players can't reduce play costs."""

    def test_has_declarative_effect(self, debug_runner):
        """Should have a declarative effect with NoTiming."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("ST13-08")
        effects = cs.effect_list(None)
        declarative = [e for e in effects if getattr(e, 'is_declarative', False)]
        assert len(declarative) >= 1, "Should have a declarative effect"
        assert declarative[0].timing == EffectTiming.NoTiming, (
            "Declarative effect should have NoTiming"
        )

    def test_blocks_own_cost_reduction(self, debug_runner):
        """Chikurimon on field should prevent own player's cost reduction."""
        runner = debug_runner(initial_memory=0)

        # Place Chikurimon
        runner.place_on_field(1, ["ST13-08"])

        # Place a card with cost reduction on P1 field
        # Use BT8-097 Crimson Blaze which has cost reduction per opponent Digimon
        runner.place_on_field(2, ["ST1-03"])  # Opponent Digimon
        runner.place_on_field(2, ["ST1-03"])  # 2 opponent Digimon
        runner.inject_card(1, "BT8-097", "hand")
        runner.set_phase("Main")

        # Check that cost is NOT reduced despite having opponent Digimon
        game = runner.game
        p1 = game.player1
        crimson = [c for c in p1.hand_cards
                   if c.c_entity_base and c.c_entity_base.card_id == "BT8-097"][0]
        cost = game.calculate_play_cost(p1, crimson)
        # BT8-097 base cost is 6. Without Chikurimon, cost would be 6-2=4.
        # With Chikurimon, cost should remain 6.
        assert cost == 6, (
            f"With Chikurimon blocking cost reduction, cost should be 6, got {cost}"
        )

    def test_blocks_opponent_cost_reduction(self, debug_runner):
        """Chikurimon on field should prevent opponent's cost reduction too."""
        runner = debug_runner(initial_memory=0)

        # Place Chikurimon on P1's field
        runner.place_on_field(1, ["ST13-08"])

        # P2 has BT8-097 and P1 has Digimon for cost reduction
        runner.place_on_field(1, ["ST1-03"])
        runner.place_on_field(1, ["ST1-03"])
        runner.inject_card(2, "BT8-097", "hand")

        game = runner.game
        p2 = game.player2
        crimson = [c for c in p2.hand_cards
                   if c.c_entity_base and c.c_entity_base.card_id == "BT8-097"][0]
        cost = game.calculate_play_cost(p2, crimson)
        # Should be full cost 6 since Chikurimon blocks reduction for all
        assert cost == 6, (
            f"Chikurimon should block opponent's cost reduction too, "
            f"expected 6, got {cost}"
        )

    def test_cost_reduction_works_without_chikurimon(self, debug_runner):
        """Baseline: cost reduction works normally without Chikurimon."""
        runner = debug_runner(initial_memory=0)

        # No Chikurimon
        runner.place_on_field(2, ["ST1-03"])
        runner.place_on_field(2, ["ST1-03"])
        runner.inject_card(1, "BT8-097", "hand")
        runner.set_phase("Main")

        game = runner.game
        p1 = game.player1
        crimson = [c for c in p1.hand_cards
                   if c.c_entity_base and c.c_entity_base.card_id == "BT8-097"][0]
        cost = game.calculate_play_cost(p1, crimson)
        # BT8-097 base cost is 6, reduced by 2 for 2 opponent Digimon
        assert cost == 4, (
            f"Without Chikurimon, cost should be reduced to 4, got {cost}"
        )

    def test_effect_removed_when_chikurimon_leaves_field(self, debug_runner):
        """When Chikurimon is deleted, cost reduction should work again."""
        runner = debug_runner(initial_memory=0)

        chiku_perm = runner.place_on_field(1, ["ST13-08"])
        runner.place_on_field(2, ["ST1-03"])
        runner.place_on_field(2, ["ST1-03"])
        runner.inject_card(1, "BT8-097", "hand")
        runner.set_phase("Main")

        game = runner.game
        p1 = game.player1

        # With Chikurimon: cost should be 6
        crimson = [c for c in p1.hand_cards
                   if c.c_entity_base and c.c_entity_base.card_id == "BT8-097"][0]
        cost_with = game.calculate_play_cost(p1, crimson)
        assert cost_with == 6, f"With Chikurimon: expected 6, got {cost_with}"

        # Delete Chikurimon
        p1.delete_permanent(chiku_perm)

        # Without Chikurimon: cost should be reduced
        cost_without = game.calculate_play_cost(p1, crimson)
        assert cost_without == 4, (
            f"After Chikurimon deleted, cost should be 4, got {cost_without}"
        )
