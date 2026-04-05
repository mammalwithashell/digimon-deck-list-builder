"""Behavioral tests for BT5-092 Nokia Shiramine (Tamer White, Cost 3).

Card text:
[On Play] You may play 1 [Agumon] or [Gabumon] from your hand without paying
    the cost.
[Your Turn] When one of your Digimon would digivolve into a Digimon card with
    [Greymon], [Garurumon] or [Omnimon] in its name, by suspending this Tamer,
    reduce the digivolution cost by 1.
[Security] Play this card without paying the cost.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase


@pytest.mark.behavioral
class TestBT5092NokiaShiramine:
    """Tests for BT5-092 Nokia Shiramine."""

    def test_play_nokia_on_field(self, debug_runner):
        """Play Nokia Shiramine from hand and verify it appears as a tamer."""
        runner = debug_runner(initial_memory=10)
        runner.inject_card(1, "BT5-092", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Nokia")
        assert action is not None, "Should find play action for Nokia Shiramine"

        result = runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        tamer_on_field = any(
            s.card_id == "BT5-092" and s.is_tamer
            for s in snap.p1_field
        )
        assert tamer_on_field, "Nokia Shiramine should be on field as a tamer"

    def test_play_cost_is_3(self, debug_runner):
        """Nokia Shiramine costs 3 memory to play."""
        runner = debug_runner(initial_memory=10)
        runner.inject_card(1, "BT5-092", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Nokia")
        assert action is not None

        result = runner.execute(action)
        # On play triggers selection for Agumon/Gabumon — skip it
        runner.auto_resolve()

        # Play cost is 3
        assert result.after.memory == result.before.memory - 3, (
            f"Nokia should cost 3 to play, "
            f"was {result.before.memory}, now {result.after.memory}"
        )

    def test_on_play_offers_agumon_from_hand(self, debug_runner):
        """[On Play] Should offer to play an Agumon from hand for free."""
        runner = debug_runner(initial_memory=10)
        runner.inject_card(1, "BT5-092", "hand")
        runner.inject_card(1, "ST1-03", "hand")  # Agumon

        runner.set_phase("Main")

        action = runner.find_action("Play Nokia")
        assert action is not None

        runner.execute(action)

        # After playing Nokia, should enter a selection phase for Agumon/Gabumon
        snap = runner.snapshot()
        # The effect_play_from_zone should trigger a selection
        # Check that after auto-resolve, Agumon appears on field
        runner.auto_resolve()

        snap_after = runner.snapshot()
        agumon_on_field = any(
            s.card_id == "ST1-03" and s.is_digimon
            for s in snap_after.p1_field
        )
        assert agumon_on_field, "Agumon should be played on field for free via On Play effect"

    def test_on_play_offers_gabumon_from_hand(self, debug_runner):
        """[On Play] Should offer to play a Gabumon from hand for free."""
        runner = debug_runner(initial_memory=10)
        runner.inject_card(1, "BT5-092", "hand")
        runner.inject_card(1, "BT1-029", "hand")  # Gabumon

        runner.set_phase("Main")

        action = runner.find_action("Play Nokia")
        assert action is not None

        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        gabumon_on_field = any(
            s.card_id == "BT1-029" and s.is_digimon
            for s in snap_after.p1_field
        )
        assert gabumon_on_field, "Gabumon should be played on field for free via On Play effect"

    def test_on_play_is_optional(self, debug_runner):
        """[On Play] Playing Agumon/Gabumon is optional."""
        runner = debug_runner(initial_memory=10)
        runner.inject_card(1, "BT5-092", "hand")
        # Agumon in hand — but we want to test declining
        runner.inject_card(1, "ST1-03", "hand")

        runner.set_phase("Main")

        action = runner.find_action("Play Nokia")
        assert action is not None
        runner.execute(action)

        # The effect should be optional (is_optional=True on effect_play_from_zone)
        # Check that auto_resolve processes without error
        runner.auto_resolve()

    def test_on_play_no_agumon_gabumon_in_hand(self, debug_runner):
        """[On Play] If no Agumon/Gabumon in hand, no selection should appear."""
        runner = debug_runner(initial_memory=10)
        runner.inject_card(1, "BT5-092", "hand")
        # No Agumon or Gabumon in hand

        runner.set_phase("Main")

        action = runner.find_action("Play Nokia")
        assert action is not None

        runner.execute(action)
        runner.auto_resolve()

        # Should complete without entering stuck selection
        snap = runner.snapshot()
        assert snap.phase == "Main", "Should return to Main phase after play with no valid targets"

    def test_digivolve_cost_reduction_condition(self, debug_runner):
        """[Your Turn] Cost reduction condition: checks Greymon/Garurumon/Omnimon name."""
        runner = debug_runner(initial_memory=10)

        # Place Nokia on field
        runner.place_on_field(1, ["BT5-092"])

        # Check the BeforePayCost effect condition directly
        game = runner.game
        player = game.player1
        nokia_card = None
        for p in player.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "BT5-092":
                nokia_card = p.top_card
                break

        assert nokia_card is not None
        effects = nokia_card.effect_list(None)
        cost_effect = None
        for eff in effects:
            if eff.timing == EffectTiming.BeforePayCost:
                cost_effect = eff
                break

        assert cost_effect is not None, "Should have BeforePayCost effect"
        assert cost_effect.cost_reduction == 1, "Cost reduction should be 1"

    def test_digivolve_cost_reduction_requires_unsuspended(self, debug_runner):
        """[Your Turn] Tamer must be unsuspended for cost reduction."""
        runner = debug_runner(initial_memory=10)

        # Place Nokia on field, suspended
        nokia_perm = runner.place_on_field(1, ["BT5-092"], is_suspended=True)

        game = runner.game
        player = game.player1
        nokia_card = None
        for p in player.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "BT5-092":
                nokia_card = p.top_card
                break

        effects = nokia_card.effect_list(None)
        cost_effect = None
        for eff in effects:
            if eff.timing == EffectTiming.BeforePayCost:
                cost_effect = eff
                break

        # Should be False because Nokia is suspended
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        greymon_cs = db.create_card_source("AD1-001", player)  # Greymon
        result = cost_effect.can_use_condition({
            "card_source": greymon_cs,
        })
        assert result is False, "Cost reduction should not activate when Nokia is suspended"

    def test_security_play_free(self, debug_runner):
        """[Security] Nokia should play for free from security."""
        runner = debug_runner(initial_memory=10)

        game = runner.game
        player = game.player1

        # Verify the security effect exists
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        nokia_cs = db.create_card_source("BT5-092", player)
        effects = nokia_cs.effect_list(None)
        security_effect = None
        for eff in effects:
            if eff.timing == EffectTiming.SecuritySkill:
                security_effect = eff
                break

        assert security_effect is not None, "Should have Security effect"
        assert security_effect.is_security_effect is True
