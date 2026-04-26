"""Behavioral tests for EX7-024 Shoemon (Lv.3, Yellow, Puppet/LIBERATOR).

Card text:
[Your Turn] When this Digimon would digivolve into a Digimon card with the
[Puppet] trait, reduce the digivolution cost by 1.

[Inherited][Your Turn] All of your opponent's Security Digimon get -3000 DP.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


SHOEMON = "EX7-024"        # Card under test (Lv.3, Puppet/LIBERATOR)
PUPPET_LV4 = "ST19-04"    # PawnChessmon (Yellow) (Lv.4, Puppet)
NON_PUPPET_LV4 = "BT1-036"  # Gatomon (Lv.4, no Puppet trait)
FILLER = "ST19-05"         # Filler card


@pytest.mark.behavioral
class TestEX7024_Effect0_DigivolveCostReduction:
    """Effect 0: [Your Turn] Digivolve cost -1 into [Puppet] trait."""

    def test_effect0_has_when_would_digivolve_timing(self, debug_runner):
        """Effect 0 must use WhenWouldDigivolve timing so the engine finds it
        during digivolution cost calculation."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SHOEMON])

        card = perm.top_card
        effects = card.effect_list(EffectTiming.WhenWouldDigivolve)
        cost_effects = [e for e in effects if getattr(e, 'cost_reduction', 0) > 0]
        assert len(cost_effects) == 1, (
            f"Should have exactly 1 WhenWouldDigivolve cost reduction effect, "
            f"got {len(cost_effects)}"
        )
        assert cost_effects[0].cost_reduction == 1

    def test_effect0_is_not_inherited(self, debug_runner):
        """Effect 0 is NOT inherited — it only applies when Shoemon is the top card."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SHOEMON])

        card = perm.top_card
        effects = card.effect_list(EffectTiming.WhenWouldDigivolve)
        cost_effects = [e for e in effects if getattr(e, 'cost_reduction', 0) > 0]
        assert len(cost_effects) == 1
        assert not cost_effects[0].is_inherited_effect, \
            "Effect 0 should NOT be inherited"

    def test_effect0_condition_passes_for_puppet_target(self, debug_runner):
        """Condition should pass when the digivolving card has [Puppet] trait."""
        runner = debug_runner(initial_memory=5, first_player=1)
        perm = runner.place_on_field(1, [SHOEMON])

        card = perm.top_card
        effects = card.effect_list(EffectTiming.WhenWouldDigivolve)
        cost_effects = [e for e in effects if getattr(e, 'cost_reduction', 0) > 0]
        assert len(cost_effects) == 1

        # Create a mock card_source with Puppet trait
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        puppet_card = db.create_card_source(PUPPET_LV4, runner.game.player1)

        context = {
            "player": runner.game.player1,
            "permanent": perm,
            "card_source": puppet_card,
        }
        assert cost_effects[0].can_use_condition(context), \
            "Condition should pass for Puppet trait target"

    def test_effect0_condition_fails_for_non_puppet_target(self, debug_runner):
        """Condition should fail when the digivolving card lacks [Puppet] trait."""
        runner = debug_runner(initial_memory=5, first_player=1)
        perm = runner.place_on_field(1, [SHOEMON])

        card = perm.top_card
        effects = card.effect_list(EffectTiming.WhenWouldDigivolve)
        cost_effects = [e for e in effects if getattr(e, 'cost_reduction', 0) > 0]
        assert len(cost_effects) == 1

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        non_puppet_card = db.create_card_source(NON_PUPPET_LV4, runner.game.player1)

        context = {
            "player": runner.game.player1,
            "permanent": perm,
            "card_source": non_puppet_card,
        }
        assert not cost_effects[0].can_use_condition(context), \
            "Condition should fail for non-Puppet target"

    def test_effect0_condition_fails_on_opponent_turn(self, debug_runner):
        """Condition should fail on opponent's turn ([Your Turn] restriction)."""
        runner = debug_runner(initial_memory=5, first_player=2)
        perm = runner.place_on_field(1, [SHOEMON])

        card = perm.top_card
        effects = card.effect_list(EffectTiming.WhenWouldDigivolve)
        cost_effects = [e for e in effects if getattr(e, 'cost_reduction', 0) > 0]
        assert len(cost_effects) == 1

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        puppet_card = db.create_card_source(PUPPET_LV4, runner.game.player1)

        context = {
            "player": runner.game.player1,
            "permanent": perm,
            "card_source": puppet_card,
        }
        assert not cost_effects[0].can_use_condition(context), \
            "Condition should fail on opponent's turn"


@pytest.mark.behavioral
class TestEX7024_Effect1_InheritedSecurityDP:
    """Effect 1: [Inherited][Your Turn] Opponent's Security Digimon get -3000 DP."""

    def test_inherited_effect_structure(self, debug_runner):
        """Effect 1 must be inherited with dp_modifier=-3000 and
        _applies_to_opponent_security_digimon flag."""
        runner = debug_runner(initial_memory=5)
        # Place Shoemon under another Digimon so it becomes inherited
        perm = runner.place_on_field(1, [SHOEMON, PUPPET_LV4])

        # Shoemon is the bottom card source
        shoemon_src = perm.card_sources[0]
        assert shoemon_src.c_entity_base.card_id == SHOEMON

        effects = shoemon_src.effect_list(None)
        inherited = [e for e in effects if getattr(e, 'is_inherited_effect', False)]
        assert len(inherited) == 1, f"Should have 1 inherited effect, got {len(inherited)}"

        eff = inherited[0]
        assert eff.dp_modifier == -3000, \
            f"DP modifier should be -3000, got {eff.dp_modifier}"
        assert getattr(eff, '_applies_to_opponent_security_digimon', False), \
            "Must have _applies_to_opponent_security_digimon flag"

    def test_inherited_condition_passes_on_owner_turn(self, debug_runner):
        """Inherited DP reduction only active on owner's turn."""
        runner = debug_runner(initial_memory=5, first_player=1)
        perm = runner.place_on_field(1, [SHOEMON, PUPPET_LV4])

        shoemon_src = perm.card_sources[0]
        effects = shoemon_src.effect_list(None)
        inherited = [e for e in effects if getattr(e, 'is_inherited_effect', False)][0]

        assert runner.game.player1.is_my_turn
        assert inherited.can_use_condition({}), \
            "Condition should pass on owner's turn"

    def test_inherited_condition_fails_on_opponent_turn(self, debug_runner):
        """Inherited DP reduction inactive on opponent's turn."""
        runner = debug_runner(initial_memory=5, first_player=2)
        perm = runner.place_on_field(1, [SHOEMON, PUPPET_LV4])

        shoemon_src = perm.card_sources[0]
        effects = shoemon_src.effect_list(None)
        inherited = [e for e in effects if getattr(e, 'is_inherited_effect', False)][0]

        assert not runner.game.player1.is_my_turn
        assert not inherited.can_use_condition({}), \
            "Condition should fail on opponent's turn"
