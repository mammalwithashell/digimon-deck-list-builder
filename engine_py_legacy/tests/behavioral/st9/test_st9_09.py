"""Behavioral tests for ST9-09 Stingmon (Lv.4 Green Digimon).

Card text:
Effect: When you would play this card from your hand, if you have a blue
Digimon in play, reduce its play cost by 1.
Inherited: [When Attacking] If you have a blue Digimon in play, <Draw 1>.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestST9_09Stingmon:
    """Tests for ST9-09 Stingmon."""

    # ---------------------------------------------------------------
    # Clause 1: BeforePayCost - play cost reduction with blue Digimon
    # ---------------------------------------------------------------

    def test_cost_reduction_with_blue_digimon_in_play(self, debug_runner):
        """Play cost should be reduced by 1 when a blue Digimon is on the field."""
        runner = debug_runner(initial_memory=10)
        # Place a blue Digimon (ST2-03 Gabumon is Blue)
        runner.place_on_field(1, ["ST2-03"])

        # Inject ST9-09 Stingmon into hand
        runner.inject_card(1, "ST9-09", zone="hand")

        game = runner.game
        # Find Stingmon in hand
        stingmon = None
        for c in game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "ST9-09":
                stingmon = c
                break
        assert stingmon is not None, "Stingmon should be in hand"

        # Calculate play cost - should be base 4 - 1 = 3
        cost = game.calculate_play_cost(game.player1, stingmon)
        assert cost == 3, f"Stingmon cost should be 3 with blue Digimon in play, got {cost}"

    def test_no_cost_reduction_without_blue_digimon(self, debug_runner):
        """Play cost should NOT be reduced when no blue Digimon is on the field."""
        runner = debug_runner(initial_memory=10)
        # Place a green Digimon only (ST4-03 Tentomon is Green)
        runner.place_on_field(1, ["ST4-03"])

        runner.inject_card(1, "ST9-09", zone="hand")

        game = runner.game
        stingmon = None
        for c in game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "ST9-09":
                stingmon = c
                break
        assert stingmon is not None

        cost = game.calculate_play_cost(game.player1, stingmon)
        assert cost == 4, f"Stingmon cost should be 4 without blue Digimon, got {cost}"

    def test_no_cost_reduction_with_empty_field(self, debug_runner):
        """Play cost should NOT be reduced with an empty field."""
        runner = debug_runner(initial_memory=10)

        runner.inject_card(1, "ST9-09", zone="hand")

        game = runner.game
        stingmon = None
        for c in game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "ST9-09":
                stingmon = c
                break
        assert stingmon is not None

        cost = game.calculate_play_cost(game.player1, stingmon)
        assert cost == 4, f"Stingmon cost should be 4 with empty field, got {cost}"

    def test_leak_guard_only_reduces_own_cost(self, debug_runner):
        """The cost reduction should only apply to ST9-09 itself, not other cards."""
        runner = debug_runner(initial_memory=10)
        # Place blue Digimon
        runner.place_on_field(1, ["ST2-03"])

        # Place Stingmon on field (simulates leaked effect from field)
        runner.place_on_field(1, ["ST9-09"])

        # Inject a different card into hand
        runner.inject_card(1, "ST1-03", zone="hand")  # Agumon, cost 3

        game = runner.game
        agumon = None
        for c in game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "ST1-03":
                agumon = c
                break
        assert agumon is not None

        # Agumon should NOT get Stingmon's cost reduction
        cost = game.calculate_play_cost(game.player1, agumon)
        assert cost == 3, f"Agumon cost should be 3 (no leak from Stingmon), got {cost}"

    # ---------------------------------------------------------------
    # Clause 2: Inherited [When Attacking] Draw 1 with blue Digimon
    # ---------------------------------------------------------------

    def test_inherited_effect_exists(self, debug_runner):
        """Stingmon should have an inherited OnUseAttack effect."""
        runner = debug_runner(initial_memory=10)
        # Place Stingmon under a higher-level Digimon for inherited effect
        perm = runner.place_on_field(1, ["ST9-09"])

        top = perm.top_card
        effects = top.effect_list(None)
        inherited_attack = [
            e for e in effects
            if e.timing == EffectTiming.OnUseAttack
            and getattr(e, 'is_inherited_effect', False)
        ]
        assert len(inherited_attack) == 1, (
            "Stingmon should have exactly 1 inherited OnUseAttack effect"
        )

    def test_inherited_condition_with_blue_digimon(self, debug_runner):
        """Inherited draw should trigger when a blue Digimon is in play."""
        runner = debug_runner(initial_memory=10)
        # Place a Lv.5 with Stingmon under it (inherited effect)
        perm = runner.place_on_field(1, ["ST9-09", "ST9-05"])

        # Also place a blue Digimon on field
        runner.place_on_field(1, ["ST2-03"])

        game = runner.game
        # Get the inherited effect from Stingmon (bottom card)
        stingmon_card = perm.card_sources[0]
        effects = stingmon_card.effect_list(None)
        inherited = [
            e for e in effects
            if e.timing == EffectTiming.OnUseAttack
            and getattr(e, 'is_inherited_effect', False)
        ][0]

        result = inherited.can_use_condition({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        assert result is True, "Inherited condition should pass with blue Digimon in play"

    def test_inherited_condition_without_blue_digimon(self, debug_runner):
        """Inherited draw should NOT trigger when no blue Digimon is in play."""
        runner = debug_runner(initial_memory=10)
        # Use a pure green Lv.5 (BT1-075 Digitamamon) so the stack itself is NOT blue
        perm = runner.place_on_field(1, ["ST9-09", "BT1-075"])

        game = runner.game
        stingmon_card = perm.card_sources[0]
        effects = stingmon_card.effect_list(None)
        inherited = [
            e for e in effects
            if e.timing == EffectTiming.OnUseAttack
            and getattr(e, 'is_inherited_effect', False)
        ][0]

        result = inherited.can_use_condition({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        assert result is False, "Inherited condition should fail without blue Digimon"

    def test_inherited_draw_one(self, debug_runner):
        """Inherited effect process should draw 1 card."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["ST9-09", "ST9-05"])
        runner.place_on_field(1, ["ST2-03"])

        game = runner.game
        stingmon_card = perm.card_sources[0]
        effects = stingmon_card.effect_list(None)
        inherited = [
            e for e in effects
            if e.timing == EffectTiming.OnUseAttack
            and getattr(e, 'is_inherited_effect', False)
        ][0]

        hand_before = len(game.player1.hand_cards)
        lib_before = len(game.player1.library_cards)

        inherited.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        hand_after = len(game.player1.hand_cards)
        lib_after = len(game.player1.library_cards)

        assert hand_after == hand_before + 1, (
            f"Hand should have 1 more card: {hand_before} -> {hand_after}"
        )
        assert lib_after == lib_before - 1, (
            f"Library should have 1 fewer card: {lib_before} -> {lib_after}"
        )

    def test_inherited_no_draw_when_deck_empty(self, debug_runner):
        """Inherited condition should fail when deck is empty (no cards to draw)."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["ST9-09", "ST9-05"])
        runner.place_on_field(1, ["ST2-03"])

        game = runner.game
        # Empty the deck
        game.player1.library_cards.clear()

        stingmon_card = perm.card_sources[0]
        effects = stingmon_card.effect_list(None)
        inherited = [
            e for e in effects
            if e.timing == EffectTiming.OnUseAttack
            and getattr(e, 'is_inherited_effect', False)
        ][0]

        result = inherited.can_use_condition({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        assert result is False, "Condition should fail when deck is empty"

    def test_inherited_requires_field_presence(self, debug_runner):
        """Inherited effect should require the card to be part of a permanent on field."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["ST9-09", "ST9-05"])
        runner.place_on_field(1, ["ST2-03"])

        game = runner.game
        stingmon_card = perm.card_sources[0]
        effects = stingmon_card.effect_list(None)
        inherited = [
            e for e in effects
            if e.timing == EffectTiming.OnUseAttack
            and getattr(e, 'is_inherited_effect', False)
        ][0]

        # Remove from field
        game.player1.battle_area.remove(perm)

        result = inherited.can_use_condition({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        assert result is False, "Condition should fail when not on field"
