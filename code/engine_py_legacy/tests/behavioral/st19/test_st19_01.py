"""Behavioral tests for ST19-01 Kyaromon (Lv.2 Digi-Egg).

Card text:
Inherited Effect [When Attacking] [Once Per Turn] If you have another Digimon,
<Draw 1> (Draw 1 card from your deck.)
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestST19_01Kyaromon:
    """Tests for ST19-01 Kyaromon inherited [When Attacking] Draw 1."""

    def _get_attack_effect(self, perm):
        """Find the inherited OnAllyAttack effect from Kyaromon in the stack."""
        for cs in perm.card_sources:
            if cs.c_entity_base and cs.c_entity_base.card_id == "ST19-01":
                effects = cs.effect_list(None)
                attack_effs = [
                    e for e in effects if e.timing == EffectTiming.OnAllyAttack
                ]
                assert len(attack_effs) == 1, (
                    "ST19-01 should have exactly 1 OnAllyAttack effect"
                )
                return attack_effs[0]
        raise AssertionError("ST19-01 not found in permanent stack")

    def test_effect_is_inherited(self, debug_runner):
        """The effect must be an inherited effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["ST19-01", "ST19-02"])
        eff = self._get_attack_effect(perm)
        assert eff.is_inherited_effect, "Effect must be inherited"

    def test_effect_is_once_per_turn(self, debug_runner):
        """The effect must be once per turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["ST19-01", "ST19-02"])
        eff = self._get_attack_effect(perm)
        assert eff.max_count_per_turn == 1, "Effect must be once per turn"

    def test_effect_is_not_optional(self, debug_runner):
        """Per C# reference, this effect is NOT optional (isOptional=false)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["ST19-01", "ST19-02"])
        eff = self._get_attack_effect(perm)
        assert not eff.is_optional, "Effect must not be optional"

    def test_draw_1_when_another_digimon_exists(self, debug_runner):
        """With another Digimon on the field, attacking should draw 1."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Place the Digimon with Kyaromon in sources
        perm = runner.place_on_field(1, ["ST19-01", "ST19-02"])
        # Place another Digimon on the field
        runner.place_on_field(1, ["ST19-02"])

        hand_before = len(game.player1.hand_cards)
        deck_before = len(game.player1.library_cards)

        eff = self._get_attack_effect(perm)

        # Condition should pass
        assert eff.can_use_condition({}), (
            "Condition should pass when another Digimon exists"
        )

        # Process the effect
        eff.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert len(game.player1.hand_cards) == hand_before + 1, (
            "Should draw 1 card"
        )
        assert len(game.player1.library_cards) == deck_before - 1, (
            "Deck should shrink by 1"
        )

    def test_no_draw_without_another_digimon(self, debug_runner):
        """With no other Digimon, condition should fail (no draw)."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Only one Digimon on the field (the one with Kyaromon)
        perm = runner.place_on_field(1, ["ST19-01", "ST19-02"])

        eff = self._get_attack_effect(perm)

        # Condition should fail - no other Digimon
        assert not eff.can_use_condition({}), (
            "Condition should fail when no other Digimon exists"
        )

    def test_tamer_does_not_count_as_another_digimon(self, debug_runner):
        """A Tamer on the field should not satisfy 'another Digimon'."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        perm = runner.place_on_field(1, ["ST19-01", "ST19-02"])
        # Place a tamer (ST1-12 is Tai Kamiya, a red Tamer)
        runner.place_on_field(1, ["ST1-12"])

        eff = self._get_attack_effect(perm)

        assert not eff.can_use_condition({}), (
            "Tamer should not count as another Digimon"
        )

    def test_condition_fails_when_not_on_field(self, debug_runner):
        """If the card is not on the field (no permanent), condition fails."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Place the Digimon then remove it
        perm = runner.place_on_field(1, ["ST19-01", "ST19-02"])
        eff = self._get_attack_effect(perm)

        # Remove from field
        game.player1.battle_area.remove(perm)

        assert not eff.can_use_condition({}), (
            "Condition should fail when permanent is not on field"
        )

    def test_multiple_other_digimon_still_draws_one(self, debug_runner):
        """With multiple other Digimon, still only draws 1."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        perm = runner.place_on_field(1, ["ST19-01", "ST19-02"])
        runner.place_on_field(1, ["ST19-02"])
        runner.place_on_field(1, ["ST19-02"])

        hand_before = len(game.player1.hand_cards)

        eff = self._get_attack_effect(perm)
        eff.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert len(game.player1.hand_cards) == hand_before + 1, (
            "Should draw exactly 1 card regardless of how many other Digimon"
        )
